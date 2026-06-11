//! 설치 전 시스템 사전 점검

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightResult {
    pub passed: bool,
    pub checks: Vec<Check>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Skip,
}

pub fn run_preflight() -> anyhow::Result<PreflightResult> {
    let mut checks = vec![];

    // 1. RAM 용량
    checks.push(check_ram());
    // 2. 디스크 공간
    checks.push(check_disk_space());
    // 3. CPU 아키텍처
    checks.push(check_architecture());
    // 4. UEFI 지원
    checks.push(check_uefi());
    // 5. Secure Boot 상태
    checks.push(check_secure_boot());
    // 6. TPM
    checks.push(check_tpm());
    // 7. 인터넷 연결 (선택)
    checks.push(check_internet());
    // 8. 루트 권한
    checks.push(check_root());
    // 9. 필수 도구
    checks.push(check_required_tools());

    let errors: Vec<String> = checks.iter()
        .filter(|c| c.status == CheckStatus::Fail && c.required)
        .map(|c| c.name.clone())
        .collect();
    let warnings: Vec<String> = checks.iter()
        .filter(|c| c.status == CheckStatus::Warn)
        .map(|c| c.detail.clone())
        .collect();

    let passed = errors.is_empty();

    Ok(PreflightResult { passed, checks, warnings, errors })
}

fn check_ram() -> Check {
    let ram_mb = ram_mb();
    let (status, detail) = if ram_mb >= 2048 {
        (CheckStatus::Pass, format!("{} MiB RAM 감지", ram_mb))
    } else if ram_mb >= 1024 {
        (CheckStatus::Warn, format!("{} MiB RAM — 최소 2 GiB 권장", ram_mb))
    } else {
        (CheckStatus::Fail, format!("{} MiB RAM — 최소 1 GiB 필요", ram_mb))
    };
    Check { name: "RAM 용량".into(), status, detail, required: true }
}

fn check_disk_space() -> Check {
    let free_gb = free_space_gb("/");
    let (status, detail) = if free_gb >= 4.0 {
        (CheckStatus::Pass, format!("{:.1} GiB 여유 공간", free_gb))
    } else if free_gb >= 2.0 {
        (CheckStatus::Warn, format!("{:.1} GiB — 최소 4 GiB 권장", free_gb))
    } else {
        (CheckStatus::Fail, format!("{:.1} GiB — 설치 공간 부족", free_gb))
    };
    Check { name: "디스크 공간".into(), status, detail, required: true }
}

fn check_architecture() -> Check {
    let arch = std::env::consts::ARCH;
    let supported = matches!(arch, "x86_64" | "aarch64");
    Check {
        name: "CPU 아키텍처".into(),
        status: if supported { CheckStatus::Pass } else { CheckStatus::Fail },
        detail: format!("{} (지원: x86_64, aarch64)", arch),
        required: true,
    }
}

fn check_uefi() -> Check {
    let uefi = std::path::Path::new("/sys/firmware/efi").exists();
    Check {
        name: "UEFI 펌웨어".into(),
        status: if uefi { CheckStatus::Pass } else { CheckStatus::Warn },
        detail: if uefi { "UEFI 모드".into() } else { "Legacy BIOS — Secure Boot 불가".into() },
        required: false,
    }
}

fn check_secure_boot() -> Check {
    let sb = std::fs::read(
        "/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c"
    ).map(|d| d.get(4).copied() == Some(1)).unwrap_or(false);

    Check {
        name: "Secure Boot".into(),
        status: if sb { CheckStatus::Pass } else { CheckStatus::Warn },
        detail: if sb {
            "Secure Boot 활성화됨".into()
        } else {
            "Secure Boot 비활성 — 설치 후 MOK 등록 권장".into()
        },
        required: false,
    }
}

fn check_tpm() -> Check {
    let tpm = std::path::Path::new("/dev/tpm0").exists()
        || std::path::Path::new("/dev/tpmrm0").exists();
    Check {
        name: "TPM 2.0".into(),
        status: if tpm { CheckStatus::Pass } else { CheckStatus::Warn },
        detail: if tpm { "TPM 2.0 감지됨".into() } else { "TPM 없음 — 측정 부팅 비활성".into() },
        required: false,
    }
}

fn check_internet() -> Check {
    // 인터넷 연결 확인 (ping 대신 파일 존재로 간단히)
    let connected = std::process::Command::new("ping")
        .args(["-c", "1", "-W", "2", "8.8.8.8"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    Check {
        name: "인터넷 연결".into(),
        status: if connected { CheckStatus::Pass } else { CheckStatus::Warn },
        detail: if connected {
            "인터넷 연결됨 (서명 DB 업데이트 가능)".into()
        } else {
            "오프라인 — 내장 DB만 사용".into()
        },
        required: false,
    }
}

fn check_root() -> Check {
    let (has_priv, detail) = root_privilege_status();
    Check {
        name: "관리자 권한".into(),
        status: if has_priv { CheckStatus::Pass } else { CheckStatus::Fail },
        detail,
        required: true,
    }
}

fn root_privilege_status() -> (bool, String) {
    #[cfg(unix)]
    {
        if unsafe { libc::getuid() == 0 } {
            return (true, "root 권한으로 실행 중".into());
        }
        let sudo_ok = std::process::Command::new("sudo")
            .args(["-n", "true"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if sudo_ok {
            return (true, "sudo로 관리자 권한 사용 가능".into());
        }
        (false, "root 권한 필요 — Live ISO에서는 sudo aegis-installer 실행".into())
    }
    #[cfg(not(unix))]
    {
        (true, "root 권한으로 실행 중".into())
    }
}

fn check_required_tools() -> Check {
    let tools = ["dd", "mkfs.ext4", "mount", "sgdisk"];
    let missing: Vec<&str> = tools.iter()
        .filter(|&&t| !is_tool_available(t))
        .copied()
        .collect();

    Check {
        name: "필수 도구".into(),
        status: if missing.is_empty() { CheckStatus::Pass } else { CheckStatus::Warn },
        detail: if missing.is_empty() {
            "모든 필수 도구 확인".into()
        } else {
            format!("없는 도구: {}", missing.join(", "))
        },
        required: false,
    }
}

fn ram_mb() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("MemTotal:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u64>().ok())
        .map(|kb| kb / 1024)
        .unwrap_or(4096) // 비-Linux 환경 기본값
}

fn free_space_gb(path: &str) -> f64 {
    #[cfg(unix)]
    {
        use std::mem::MaybeUninit;
        let path = std::ffi::CString::new(path).unwrap();
        let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
        unsafe {
            if libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) == 0 {
                let stat = stat.assume_init();
                return (stat.f_bavail as f64 * stat.f_frsize as f64) / (1024.0 * 1024.0 * 1024.0);
            }
        }
    }
    32.0 // 비-Linux 기본값
}

fn is_tool_available(tool: &str) -> bool {
    std::process::Command::new("which")
        .arg(tool)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
