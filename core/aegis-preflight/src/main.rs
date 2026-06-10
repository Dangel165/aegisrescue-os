//! aegis-preflight — AegisRescue 시스템 사전 점검 도구
//!
//! 부팅 직후 또는 설치 전 하드웨어 호환성, 보안 기능, 도구 가용성을 점검합니다.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Parser)]
#[command(name = "aegis-preflight", about = "AegisRescue system pre-flight check")]
struct Cli {
    /// JSON 형식으로 출력
    #[arg(long)]
    json: bool,

    /// 특정 점검 범주만 실행 (hardware, security, tools, network)
    #[arg(long, value_delimiter = ',')]
    only: Vec<String>,

    /// 실패 시 오류 코드 반환
    #[arg(long)]
    strict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreflightReport {
    pub system: SystemInfo,
    pub checks: Vec<CheckResult>,
    pub summary: Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemInfo {
    pub hostname: String,
    pub kernel: String,
    pub arch: String,
    pub ram_mb: u64,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub uefi: bool,
    pub secure_boot: bool,
    pub tpm: bool,
    pub virtualized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckResult {
    pub category: String,
    pub name: String,
    pub status: Status,
    pub detail: String,
    pub required: bool,
    pub fix_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum Status { Pass, Warn, Fail, Skip }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Summary {
    pub pass: usize,
    pub warn: usize,
    pub fail: usize,
    pub skip: usize,
    pub overall: OverallStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum OverallStatus { Ready, Degraded, Blocked }

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .init();

    let system = gather_system_info();
    let mut checks = vec![];

    let run_all = cli.only.is_empty();
    let run = |cat: &str| run_all || cli.only.iter().any(|c| c == cat);

    if run("hardware") {
        checks.extend(check_hardware(&system));
    }
    if run("security") {
        checks.extend(check_security(&system));
    }
    if run("tools") {
        checks.extend(check_tools());
    }
    if run("network") {
        checks.extend(check_network());
    }
    if run("integrity") {
        checks.extend(check_integrity());
    }

    let pass  = checks.iter().filter(|c| c.status == Status::Pass).count();
    let warn  = checks.iter().filter(|c| c.status == Status::Warn).count();
    let fail  = checks.iter().filter(|c| c.status == Status::Fail && c.required).count();
    let skip  = checks.iter().filter(|c| c.status == Status::Skip).count();

    let overall = if fail > 0 {
        OverallStatus::Blocked
    } else if warn > 0 {
        OverallStatus::Degraded
    } else {
        OverallStatus::Ready
    };

    let report = PreflightReport {
        system,
        checks,
        summary: Summary { pass, warn, fail, skip, overall: overall.clone() },
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }

    if cli.strict {
        if matches!(report.summary.overall, OverallStatus::Blocked) {
            std::process::exit(1);
        }
    }

    Ok(())
}

// ─── 시스템 정보 수집 ─────────────────────────────────────────

fn gather_system_info() -> SystemInfo {
    SystemInfo {
        hostname: read_file("/etc/hostname").unwrap_or_else(|| "unknown".into()).trim().to_string(),
        kernel: run_cmd("uname", &["-r"]).unwrap_or_else(|| "unknown".into()),
        arch: std::env::consts::ARCH.to_string(),
        ram_mb: total_ram_mb(),
        cpu_model: cpu_model(),
        cpu_cores: num_cpus(),
        uefi: Path::new("/sys/firmware/efi").exists(),
        secure_boot: check_secure_boot_enabled(),
        tpm: Path::new("/dev/tpm0").exists() || Path::new("/dev/tpmrm0").exists(),
        virtualized: detect_virtualization(),
    }
}

// ─── 하드웨어 점검 ────────────────────────────────────────────

fn check_hardware(sys: &SystemInfo) -> Vec<CheckResult> {
    vec![
        // RAM
        CheckResult {
            category: "hardware".into(),
            name: "RAM".into(),
            status: if sys.ram_mb >= 2048 { Status::Pass }
                    else if sys.ram_mb >= 1024 { Status::Warn }
                    else { Status::Fail },
            detail: format!("{} MiB 총 RAM", sys.ram_mb),
            required: true,
            fix_hint: if sys.ram_mb < 2048 {
                Some("최소 2 GiB RAM이 필요합니다".into())
            } else { None },
        },
        // CPU
        CheckResult {
            category: "hardware".into(),
            name: "CPU 아키텍처".into(),
            status: if matches!(sys.arch.as_str(), "x86_64" | "aarch64") {
                Status::Pass
            } else {
                Status::Fail
            },
            detail: format!("{} ({} 코어) — {}", sys.arch, sys.cpu_cores, sys.cpu_model),
            required: true,
            fix_hint: None,
        },
        // 스토리지
        check_disk_space(),
        // 가상화
        CheckResult {
            category: "hardware".into(),
            name: "가상화 환경".into(),
            status: if sys.virtualized { Status::Warn } else { Status::Pass },
            detail: if sys.virtualized {
                "가상 머신 감지 — 일부 기능 제한".into()
            } else {
                "물리 하드웨어".into()
            },
            required: false,
            fix_hint: None,
        },
    ]
}

fn check_disk_space() -> CheckResult {
    #[cfg(unix)]
    {
        use std::mem::MaybeUninit;
        let path = std::ffi::CString::new("/").unwrap();
        let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
        unsafe {
            if libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) == 0 {
                let stat = stat.assume_init();
                let free_gb = (stat.f_bavail as f64 * stat.f_frsize as f64)
                    / (1024.0_f64.powi(3));
                return CheckResult {
                    category: "hardware".into(),
                    name: "디스크 여유 공간".into(),
                    status: if free_gb >= 4.0 { Status::Pass }
                            else if free_gb >= 2.0 { Status::Warn }
                            else { Status::Fail },
                    detail: format!("{:.1} GiB 여유 공간", free_gb),
                    required: true,
                    fix_hint: if free_gb < 4.0 {
                        Some("최소 4 GiB 여유 공간이 필요합니다".into())
                    } else { None },
                };
            }
        }
    }
    CheckResult {
        category: "hardware".into(),
        name: "디스크 여유 공간".into(),
        status: Status::Skip,
        detail: "비-Linux 환경 — 점검 건너뜀".into(),
        required: false,
        fix_hint: None,
    }
}

// ─── 보안 점검 ────────────────────────────────────────────────

fn check_security(sys: &SystemInfo) -> Vec<CheckResult> {
    vec![
        CheckResult {
            category: "security".into(),
            name: "UEFI 부팅".into(),
            status: if sys.uefi { Status::Pass } else { Status::Warn },
            detail: if sys.uefi { "UEFI 모드".into() } else { "Legacy BIOS — Secure Boot 불가".into() },
            required: false,
            fix_hint: if !sys.uefi {
                Some("BIOS 설정에서 UEFI 모드를 활성화하세요".into())
            } else { None },
        },
        CheckResult {
            category: "security".into(),
            name: "Secure Boot".into(),
            status: if sys.secure_boot { Status::Pass } else { Status::Warn },
            detail: if sys.secure_boot {
                "Secure Boot 활성화됨".into()
            } else {
                "Secure Boot 비활성 — MOK 등록 후 활성화 권장".into()
            },
            required: false,
            fix_hint: if !sys.secure_boot {
                Some("BIOS에서 Secure Boot를 활성화하고 MOK를 등록하세요".into())
            } else { None },
        },
        CheckResult {
            category: "security".into(),
            name: "TPM 2.0".into(),
            status: if sys.tpm { Status::Pass } else { Status::Warn },
            detail: if sys.tpm { "TPM 2.0 감지됨".into() } else { "TPM 없음 — 측정 부팅 비활성".into() },
            required: false,
            fix_hint: None,
        },
        check_apparmor(),
        check_aslr(),
        check_nx(),
    ]
}

fn check_apparmor() -> CheckResult {
    let enabled = Path::new("/sys/kernel/security/apparmor").exists();
    CheckResult {
        category: "security".into(),
        name: "AppArmor".into(),
        status: if enabled { Status::Pass } else { Status::Warn },
        detail: if enabled { "AppArmor 활성화됨".into() } else { "AppArmor 비활성 — 프로세스 격리 약화".into() },
        required: false,
        fix_hint: if !enabled {
            Some("커널 파라미터에 apparmor=1 security=apparmor 추가".into())
        } else { None },
    }
}

fn check_aslr() -> CheckResult {
    let level = read_file("/proc/sys/kernel/randomize_va_space")
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(0);
    CheckResult {
        category: "security".into(),
        name: "ASLR".into(),
        status: if level == 2 { Status::Pass } else if level == 1 { Status::Warn } else { Status::Fail },
        detail: format!("randomize_va_space = {}", level),
        required: true,
        fix_hint: if level < 2 {
            Some("echo 2 > /proc/sys/kernel/randomize_va_space".into())
        } else { None },
    }
}

fn check_nx() -> CheckResult {
    let nx = cpu_flags_contain("nx") || cpu_flags_contain("xd");
    CheckResult {
        category: "security".into(),
        name: "NX/XD 비트".into(),
        status: if nx { Status::Pass } else { Status::Warn },
        detail: if nx { "NX/XD 비트 지원".into() } else { "NX/XD 미지원 — DEP 불가".into() },
        required: false,
        fix_hint: None,
    }
}

// ─── 도구 점검 ────────────────────────────────────────────────

fn check_tools() -> Vec<CheckResult> {
    let required_tools = [
        ("aegisd",          true,  "aegis-daemon"),
        ("aegis-mount",     true,  "안전 마운트"),
        ("aegis-scan",      true,  "악성코드 스캐너"),
        ("aegis-recover",   false, "랜섬웨어 복구"),
        ("aegis-boot",      false, "부트 복구"),
        ("aegis-forensic",  false, "포렌식"),
        ("aegis-image",     false, "디스크 이미징"),
        ("aegis-ai",        false, "AI 분류기"),
        ("clamav",          false, "ClamAV 스캐너"),
        ("yara",            false, "YARA 룰 엔진"),
        ("photorec",        false, "파일 카빙"),
        ("ddrescue",        false, "손상 디스크 복구"),
        ("restic",          false, "증분 백업"),
        ("dd",              true,  "디스크 I/O"),
        ("mkfs.ext4",       false, "ext4 포맷"),
        ("sgdisk",          false, "GPT 파티셔닝"),
        ("grub-install",    false, "GRUB 부트로더"),
        ("nft",             false, "nftables 방화벽"),
        ("bubblewrap",      false, "네임스페이스 격리"),
    ];

    required_tools.iter().map(|(tool, required, desc)| {
        let found = is_available(tool);
        CheckResult {
            category: "tools".into(),
            name: tool.to_string(),
            status: if found { Status::Pass }
                    else if *required { Status::Fail }
                    else { Status::Warn },
            detail: if found {
                format!("{} — 사용 가능", desc)
            } else {
                format!("{} — 설치되지 않음", desc)
            },
            required: *required,
            fix_hint: if !found {
                Some(format!("apt install {} 또는 패키지 매니저로 설치", tool))
            } else { None },
        }
    }).collect()
}

// ─── 네트워크 점검 ────────────────────────────────────────────

fn check_network() -> Vec<CheckResult> {
    let airgap_active = check_airgap_mode();
    vec![
        CheckResult {
            category: "network".into(),
            name: "Air-Gap 모드".into(),
            status: if airgap_active { Status::Pass } else { Status::Warn },
            detail: if airgap_active {
                "네트워크 격리 활성화됨 — 안전 상태".into()
            } else {
                "네트워크 활성화됨 — 격리 권장".into()
            },
            required: false,
            fix_hint: if !airgap_active {
                Some("aegis network airgap 명령으로 격리 모드를 활성화하세요".into())
            } else { None },
        },
        check_dns(),
    ]
}

fn check_airgap_mode() -> bool {
    let output = std::process::Command::new("nft")
        .args(["list", "ruleset"])
        .output();
    match output {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            s.contains("drop") && s.contains("aegis-airgap")
        }
        Err(_) => false,
    }
}

fn check_dns() -> CheckResult {
    let resolv = read_file("/etc/resolv.conf").unwrap_or_default();
    let has_dns = resolv.contains("nameserver");
    CheckResult {
        category: "network".into(),
        name: "DNS 설정".into(),
        status: if has_dns { Status::Pass } else { Status::Warn },
        detail: if has_dns { "/etc/resolv.conf 설정됨".into() } else { "DNS 없음".into() },
        required: false,
        fix_hint: None,
    }
}

// ─── 무결성 점검 ─────────────────────────────────────────────

fn check_integrity() -> Vec<CheckResult> {
    vec![
        check_dm_verity(),
        check_ima(),
        check_aegis_policy(),
    ]
}

fn check_dm_verity() -> CheckResult {
    let active = read_file("/proc/mounts")
        .map(|m| m.contains("dm-verity") || m.contains("verity"))
        .unwrap_or(false);
    CheckResult {
        category: "integrity".into(),
        name: "dm-verity".into(),
        status: if active { Status::Pass } else { Status::Warn },
        detail: if active { "루트 파일시스템 검증됨".into() } else { "dm-verity 비활성 — rootfs 무결성 미보장".into() },
        required: false,
        fix_hint: None,
    }
}

fn check_ima() -> CheckResult {
    let ima_active = Path::new("/sys/kernel/security/ima").exists();
    CheckResult {
        category: "integrity".into(),
        name: "IMA (무결성 측정)".into(),
        status: if ima_active { Status::Pass } else { Status::Warn },
        detail: if ima_active { "IMA 서브시스템 활성화됨".into() } else { "IMA 비활성".into() },
        required: false,
        fix_hint: if !ima_active {
            Some("커널 파라미터에 ima=1 추가".into())
        } else { None },
    }
}

fn check_aegis_policy() -> CheckResult {
    let policy_path = "/etc/aegis/policy.toml";
    let exists = Path::new(policy_path).exists();
    CheckResult {
        category: "integrity".into(),
        name: "Aegis 보안 정책".into(),
        status: if exists { Status::Pass } else { Status::Warn },
        detail: if exists {
            format!("{} 존재", policy_path)
        } else {
            "정책 파일 없음 — 기본값 사용".into()
        },
        required: false,
        fix_hint: if !exists {
            Some("aegisd --init-policy 실행".into())
        } else { None },
    }
}

// ─── 출력 ─────────────────────────────────────────────────────

fn print_report(report: &PreflightReport) {
    let green  = "\x1b[32m";
    let yellow = "\x1b[33m";
    let red    = "\x1b[31m";
    let dim    = "\x1b[2m";
    let bold   = "\x1b[1m";
    let reset  = "\x1b[0m";

    println!("\n{bold}🛡 AegisRescue 사전 점검 보고서{reset}");
    println!("─────────────────────────────────────────────────────");
    println!(" 호스트명: {}  커널: {}  아키텍처: {}",
             report.system.hostname, report.system.kernel, report.system.arch);
    println!(" RAM: {} MiB  CPU 코어: {}  UEFI: {}  Secure Boot: {}  TPM: {}",
             report.system.ram_mb,
             report.system.cpu_cores,
             if report.system.uefi { "예" } else { "아니오" },
             if report.system.secure_boot { "예" } else { "아니오" },
             if report.system.tpm { "예" } else { "아니오" });
    println!("─────────────────────────────────────────────────────\n");

    let mut last_cat = "";
    for check in &report.checks {
        if check.category != last_cat {
            println!("\n{bold}[{}]{reset}", check.category.to_uppercase());
            last_cat = &check.category;
        }

        let (icon, color) = match check.status {
            Status::Pass => ("✓", green),
            Status::Warn => ("⚠", yellow),
            Status::Fail => ("✗", red),
            Status::Skip => ("–", dim),
        };

        let req = if check.required { " {필수}" } else { "" };
        println!("  {color}{icon}{reset} {:<28} {dim}{}{dim}{reset}{}",
                 check.name, check.detail, req);

        if let Some(hint) = &check.fix_hint {
            println!("      {dim}→ 해결 방법: {}{reset}", hint);
        }
    }

    println!("\n─────────────────────────────────────────────────────");
    let (status_color, status_text) = match report.summary.overall {
        OverallStatus::Ready    => (green,  "✓ 준비됨"),
        OverallStatus::Degraded => (yellow, "⚠ 일부 경고"),
        OverallStatus::Blocked  => (red,    "✗ 차단됨"),
    };
    println!(" {status_color}{bold}{status_text}{reset}  통과: {}  경고: {}  실패: {}  건너뜀: {}",
             report.summary.pass,
             report.summary.warn,
             report.summary.fail,
             report.summary.skip);
    println!("─────────────────────────────────────────────────────\n");
}

// ─── 헬퍼 ─────────────────────────────────────────────────────

fn read_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn run_cmd(cmd: &str, args: &[&str]) -> Option<String> {
    std::process::Command::new(cmd).args(args).output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}

fn is_available(tool: &str) -> bool {
    std::process::Command::new("which").arg(tool)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || Path::new(&format!("/usr/bin/{}", tool)).exists()
        || Path::new(&format!("/usr/sbin/{}", tool)).exists()
        || Path::new(&format!("/opt/aegis/bin/{}", tool)).exists()
}

fn total_ram_mb() -> u64 {
    read_file("/proc/meminfo")
        .and_then(|c| c.lines().find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u64>().ok()))
        .map(|kb| kb / 1024)
        .unwrap_or(4096)
}

fn cpu_model() -> String {
    read_file("/proc/cpuinfo")
        .and_then(|c| c.lines()
            .find(|l| l.starts_with("model name"))
            .map(|l| l.splitn(2, ':').nth(1).unwrap_or("").trim().to_string()))
        .unwrap_or_else(|| "Unknown CPU".into())
}

fn num_cpus() -> u32 {
    read_file("/proc/cpuinfo")
        .map(|c| c.lines().filter(|l| l.starts_with("processor")).count() as u32)
        .unwrap_or(1)
}

fn check_secure_boot_enabled() -> bool {
    std::fs::read(
        "/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c"
    ).map(|d| d.get(4).copied() == Some(1)).unwrap_or(false)
}

fn detect_virtualization() -> bool {
    read_file("/proc/cpuinfo")
        .map(|c| c.contains("hypervisor"))
        .unwrap_or(false)
        || Path::new("/proc/xen").exists()
        || std::process::Command::new("systemd-detect-virt")
            .arg("--quiet")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

fn cpu_flags_contain(flag: &str) -> bool {
    read_file("/proc/cpuinfo")
        .map(|c| c.lines()
            .filter(|l| l.starts_with("flags") || l.starts_with("Features"))
            .any(|l| l.contains(flag)))
        .unwrap_or(false)
}

#[cfg(not(unix))]
mod libc {
    pub unsafe fn getuid() -> u32 { 0 }
    #[repr(C)]
    pub struct statvfs {
        pub f_bavail: u64,
        pub f_frsize: u64,
    }
    pub unsafe fn statvfs(_: *const i8, _: *mut statvfs) -> i32 { -1 }
}
