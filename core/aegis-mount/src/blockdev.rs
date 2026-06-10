//! 블록 장치 열거

use aegis_common::types::{DiskInfo, DiskHealth, HealthStatus, PartitionTableType};
use std::path::PathBuf;

/// 시스템의 모든 블록 장치 목록 반환
pub fn list_block_devices() -> anyhow::Result<Vec<DiskInfo>> {
    let mut disks = vec![];

    // /sys/block 에서 디스크 열거
    let block_dir = std::path::Path::new("/sys/block");
    if !block_dir.exists() {
        // 비-Linux 환경: 샘플 반환
        return Ok(sample_disks());
    }

    for entry in std::fs::read_dir(block_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        // 루프/램/가상 장치 제외 (복구 대상만)
        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("zram") {
            continue;
        }

        let dev_path = PathBuf::from(format!("/dev/{}", name));
        let size_path = entry.path().join("size");
        let size_sectors: u64 = std::fs::read_to_string(&size_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);

        let model = std::fs::read_to_string(entry.path().join("device/model"))
            .ok()
            .map(|s| s.trim().to_string());

        disks.push(DiskInfo {
            device: dev_path,
            model,
            serial: None,
            size_bytes: size_sectors * 512,
            sector_size: 512,
            partition_table: detect_partition_table(&name),
            partitions: vec![],
            health: DiskHealth {
                smart_available: false,
                overall_health: HealthStatus::Unknown,
                reallocated_sectors: 0,
                pending_sectors: 0,
                uncorrectable_errors: 0,
            },
        });
    }

    Ok(disks)
}

fn detect_partition_table(disk_name: &str) -> PartitionTableType {
    let path = format!("/dev/{}", disk_name);
    let output = std::process::Command::new("blkid")
        .args(["-p", "-o", "value", "-s", "PTTYPE", &path])
        .output();

    match output {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_lowercase();
            match s.as_str() {
                "gpt"  => PartitionTableType::Gpt,
                "dos"  => PartitionTableType::Mbr,
                _      => PartitionTableType::Unknown,
            }
        }
        Err(_) => PartitionTableType::Unknown,
    }
}

fn sample_disks() -> Vec<DiskInfo> {
    vec![
        DiskInfo {
            device: PathBuf::from("/dev/sda"),
            model: Some("Sample Disk (non-Linux env)".into()),
            serial: None,
            size_bytes: 500 * 1024 * 1024 * 1024,
            sector_size: 512,
            partition_table: PartitionTableType::Gpt,
            partitions: vec![],
            health: DiskHealth {
                smart_available: false,
                overall_health: HealthStatus::Unknown,
                reallocated_sectors: 0,
                pending_sectors: 0,
                uncorrectable_errors: 0,
            },
        }
    ]
}
