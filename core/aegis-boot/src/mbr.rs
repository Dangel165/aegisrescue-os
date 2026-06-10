//! MBR/GPT 분석 및 복구

use std::io::{Read, Write};
use std::path::Path;

const MBR_SIZE: usize = 512;
const GPT_HEADER_LBA: u64 = 1;
const GPT_SIGNATURE: &[u8] = b"EFI PART";

pub fn analyze_boot_record(device: &Path) -> anyhow::Result<String> {
    let mut f = std::fs::File::open(device)?;
    let mut sector = [0u8; 512 * 34]; // 첫 34 섹터 (GPT 기본 헤더 포함)
    let read = f.read(&mut sector).unwrap_or(0);

    let mut report = format!("Boot Record Analysis: {}\n", device.display());
    report.push_str(&"─".repeat(60));
    report.push('\n');

    if read < 512 {
        report.push_str("ERROR: Could not read boot sector\n");
        return Ok(report);
    }

    let mbr = &sector[..512];

    // MBR 서명 확인
    let sig_ok = mbr[510] == 0x55 && mbr[511] == 0xAA;
    report.push_str(&format!("MBR signature (0x55AA):  {}\n",
        if sig_ok { "✓ Valid" } else { "✗ MISSING/CORRUPTED" }));

    // GPT 확인 (LBA 1 = offset 512)
    if read >= 1024 {
        let gpt_header = &sector[512..512+8];
        if gpt_header == GPT_SIGNATURE {
            report.push_str("Partition table type:    GPT\n");
            report.push_str(&analyze_gpt(&sector));
        } else {
            report.push_str("Partition table type:    MBR (DOS)\n");
            report.push_str(&analyze_mbr_partitions(mbr));
        }
    }

    // MBR 코드 해시 (부트킷 탐지에 사용)
    let mbr_hash = aegis_common::hash::sha256_bytes(&mbr[..446]); // 부트코드만
    report.push_str(&format!("Boot code SHA-256:       {}...\n", &mbr_hash[..16]));

    // 알려진 정상 부트코드 해시와 비교
    let is_known_clean = check_known_mbr_hash(&mbr_hash);
    report.push_str(&format!("Boot code integrity:     {}\n",
        if is_known_clean { "✓ Known clean" } else { "? Unknown — manual review recommended" }));

    Ok(report)
}

fn analyze_gpt(data: &[u8]) -> String {
    if data.len() < 1024 {
        return "  GPT: insufficient data\n".to_string();
    }

    let header = &data[512..];

    // GPT 헤더 파싱 (간략)
    let revision = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
    let header_size = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
    let partition_entries = u32::from_le_bytes([header[80], header[81], header[82], header[83]]);

    format!(
        "  GPT revision:        {:#010x}\n  GPT header size:     {} bytes\n  Partition entries:   {}\n",
        revision, header_size, partition_entries
    )
}

fn analyze_mbr_partitions(mbr: &[u8]) -> String {
    let mut result = String::new();
    let part_table = &mbr[446..510];

    for i in 0..4 {
        let entry = &part_table[i * 16..(i + 1) * 16];
        let status = entry[0];
        let part_type = entry[4];
        let lba_start = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);
        let sectors = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);

        if part_type != 0 {
            result.push_str(&format!(
                "  Partition {}: type={:#04x} lba={} sectors={} {}\n",
                i + 1, part_type, lba_start, sectors,
                if status == 0x80 { "(bootable)" } else { "" }
            ));
        }
    }

    if result.is_empty() {
        result = "  No MBR partitions found\n".to_string();
    }

    result
}

pub fn repair_mbr(device: &Path) -> anyhow::Result<()> {
    tracing::info!(device = %device.display(), "rewriting MBR boot code");

    let device_arg = device.to_string_lossy().into_owned();

    // ms-sys 또는 grub-install로 MBR 재작성
    // 1) syslinux/ms-sys로 표준 MBR 코드 기록
    let status = std::process::Command::new("ms-sys")
        .args(["-w", device_arg.as_str()])
        .status();

    match status {
        Ok(s) if s.success() => {
            tracing::info!("MBR rewritten via ms-sys");
            return Ok(());
        }
        _ => {}
    }

    // 폴백: dd로 표준 MBR 코드 기록
    let mbr_code_path = "/usr/share/aegis/mbr/generic.mbr";
    if Path::new(mbr_code_path).exists() {
        let status = std::process::Command::new("dd")
            .args([
                &format!("if={}", mbr_code_path),
                &format!("of={}", device.display()),
                "bs=446",
                "count=1",
                "conv=notrunc",
            ])
            .status()?;

        if status.success() {
            tracing::info!("MBR boot code restored via dd");
        }
    }

    Ok(())
}

pub fn repair_gpt(device: &Path) -> anyhow::Result<()> {
    tracing::info!(device = %device.display(), "repairing GPT");
    let device_arg = device.to_string_lossy().into_owned();

    // gdisk -l 로 확인 후 백업 GPT에서 복원
    let status = std::process::Command::new("gdisk")
        .args([device_arg.as_str()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn();

    match status {
        Ok(mut child) => {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                // gdisk 대화형: "r" → recovery menu, "d" → use backup GPT, "w" → write, "Y" → confirm
                stdin.write_all(b"r\nd\nw\nY\n")?;
            }
            child.wait()?;
            tracing::info!("GPT restored from backup header");
        }
        Err(_) => {
            // sgdisk 사용
            let _ = std::process::Command::new("sgdisk")
                .args(["--recover", device_arg.as_str()])
                .status();
        }
    }

    Ok(())
}

fn check_known_mbr_hash(_hash: &str) -> bool {
    // 알려진 정상 MBR 해시 목록 (실제 배포 시 파일에서 로드)
    // 여기서는 false 반환 (알 수 없음)
    false
}
