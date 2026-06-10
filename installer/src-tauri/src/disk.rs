//! 설치 대상 디스크 열거 및 정보 조회

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetDisk {
    pub device: String,
    pub model: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub removable: bool,
    pub partition_table: String,
    pub partitions: Vec<PartitionSummary>,
    pub has_os: bool,
    pub os_detected: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionSummary {
    pub device: String,
    pub size_bytes: u64,
    pub filesystem: String,
    pub label: Option<String>,
    pub mountpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub disk: TargetDisk,
    pub free_space_bytes: u64,
    pub recommended_layout: Vec<PartitionPlan>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionPlan {
    pub label: String,
    pub size_bytes: u64,
    pub filesystem: String,
    pub mountpoint: String,
    pub flags: Vec<String>,
}

pub fn list_disks() -> anyhow::Result<Vec<TargetDisk>> {
    let output = std::process::Command::new("lsblk")
        .args(["-J", "-b", "-o",
               "NAME,MODEL,SIZE,RM,TYPE,FSTYPE,LABEL,MOUNTPOINT,PKNAME"])
        .output()?;

    if !output.status.success() {
        return Ok(sample_disks());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_default();

    let mut disks = vec![];
    if let Some(devices) = json["blockdevices"].as_array() {
        for dev in devices {
            if dev["type"].as_str() != Some("disk") { continue; }

            let name = dev["name"].as_str().unwrap_or("").to_string();
            let device = format!("/dev/{}", name);
            let model = dev["model"].as_str().unwrap_or("Unknown").trim().to_string();
            let size_bytes = dev["size"].as_u64().unwrap_or(0);
            let removable = dev["rm"].as_bool().unwrap_or(false);

            let partitions = parse_partitions(dev);
            let os_detected = detect_os_on_disk(&partitions);
            let has_os = os_detected.is_some();

            disks.push(TargetDisk {
                device,
                model,
                size_bytes,
                size_human: human_bytes(size_bytes),
                removable,
                partition_table: detect_partition_table(&name),
                partitions,
                has_os,
                os_detected,
            });
        }
    }

    if disks.is_empty() {
        return Ok(sample_disks());
    }

    Ok(disks)
}

pub fn get_disk_info(device: &str) -> anyhow::Result<DiskInfo> {
    let disks = list_disks()?;
    let disk = disks.into_iter()
        .find(|d| d.device == device)
        .ok_or_else(|| anyhow::anyhow!("Disk not found: {}", device))?;

    let free = disk.size_bytes.saturating_sub(
        disk.partitions.iter().map(|p| p.size_bytes).sum::<u64>()
    );

    let mut warnings = vec![];
    if disk.size_bytes < 4 * 1024 * 1024 * 1024 {
        warnings.push("디스크 용량이 4GB 미만입니다 (최소 4GB 권장)".into());
    }
    if disk.has_os {
        warnings.push(format!(
            "⚠️ {} 설치됨 — 기존 OS가 삭제될 수 있습니다",
            disk.os_detected.as_deref().unwrap_or("OS")
        ));
    }

    let recommended_layout = recommended_partition_layout(disk.size_bytes, free);

    Ok(DiskInfo { disk, free_space_bytes: free, recommended_layout, warnings })
}

fn parse_partitions(dev: &serde_json::Value) -> Vec<PartitionSummary> {
    let mut parts = vec![];
    if let Some(children) = dev["children"].as_array() {
        for child in children {
            let name = child["name"].as_str().unwrap_or("").to_string();
            parts.push(PartitionSummary {
                device: format!("/dev/{}", name),
                size_bytes: child["size"].as_u64().unwrap_or(0),
                filesystem: child["fstype"].as_str().unwrap_or("").to_string(),
                label: child["label"].as_str().map(|s| s.to_string()),
                mountpoint: child["mountpoint"].as_str().map(|s| s.to_string()),
            });
        }
    }
    parts
}

fn detect_os_on_disk(partitions: &[PartitionSummary]) -> Option<String> {
    for p in partitions {
        if let Some(mp) = &p.mountpoint {
            if mp == "/boot/efi" || mp == "/boot" {
                if p.filesystem.contains("vfat") { return Some("Windows (EFI)".into()); }
                return Some("Linux".into());
            }
        }
        match p.filesystem.as_str() {
            "ntfs" => return Some("Windows".into()),
            "apfs" => return Some("macOS".into()),
            _ => {}
        }
    }
    None
}

fn detect_partition_table(disk_name: &str) -> String {
    let output = std::process::Command::new("blkid")
        .args(["-p", "-o", "value", "-s", "PTTYPE",
               &format!("/dev/{}", disk_name)])
        .output();
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_uppercase(),
        Err(_) => "UNKNOWN".into(),
    }
}

fn recommended_partition_layout(total: u64, free: u64) -> Vec<PartitionPlan> {
    let gib = 1024 * 1024 * 1024u64;
    vec![
        PartitionPlan {
            label: "EFI System".into(),
            size_bytes: 512 * 1024 * 1024,
            filesystem: "vfat".into(),
            mountpoint: "/boot/efi".into(),
            flags: vec!["esp".into(), "boot".into()],
        },
        PartitionPlan {
            label: "AegisRescue System".into(),
            size_bytes: 2 * gib,
            filesystem: "ext4".into(),
            mountpoint: "/".into(),
            flags: vec![],
        },
        PartitionPlan {
            label: "AegisRescue Data".into(),
            size_bytes: free.saturating_sub(3 * gib),
            filesystem: "ext4".into(),
            mountpoint: "/var/lib/aegis".into(),
            flags: vec![],
        },
    ]
}

fn sample_disks() -> Vec<TargetDisk> {
    vec![
        TargetDisk {
            device: "/dev/sda".into(),
            model: "Sample Disk (non-Linux)".into(),
            size_bytes: 32 * 1024 * 1024 * 1024,
            size_human: "32.0 GiB".into(),
            removable: true,
            partition_table: "GPT".into(),
            partitions: vec![],
            has_os: false,
            os_detected: None,
        }
    ]
}

fn human_bytes(bytes: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 { return "0 B".into(); }
    let exp = (bytes as f64).log(1024.0).floor() as usize;
    let exp = exp.min(units.len() - 1);
    let val = bytes as f64 / 1024_f64.powi(exp as i32);
    if exp == 0 { format!("{} B", bytes) } else { format!("{:.1} {}", val, units[exp]) }
}
