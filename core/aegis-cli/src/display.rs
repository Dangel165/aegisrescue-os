//! 상태 표시 유틸리티

use aegis_common::ipc::{DaemonRequest, DaemonResponse};
use crate::client;

pub fn print_system_status() {
    match client::send_request_sync(&DaemonRequest::GetStatus) {
        Ok(DaemonResponse::Status(s)) => {
            println!("System Status:");
            println!("  Version:      {}", s.version);
            println!("  Uptime:       {}s", s.uptime_secs);
            println!("  Active jobs:  {}", s.active_jobs);
            println!("  Queued jobs:  {}", s.queued_jobs);
            println!("  Network mode: {}", s.network_mode);
            println!("  Secure Boot:  {}", if s.secure_boot_active { "✓ enabled" } else { "⚠ disabled" });
            println!("  dm-verity:    {}", if s.dm_verity_active { "✓ active" } else { "? not detected" });
            println!("  Quarantine:   {}", s.quarantine_count);
            println!("  Threats:      {}", s.total_threats_detected);
        }
        Ok(DaemonResponse::Error { message, .. }) => {
            println!("aegisd error: {}", message);
            print_local_status_fallback();
        }
        _ => print_local_status_fallback(),
    }
}

pub fn print_disk_list() {
    match client::send_request_sync(&DaemonRequest::ListDisks) {
        Ok(DaemonResponse::Disks(disks)) => {
            println!("{:<20} {:<12} {:<10}", "DEVICE", "SIZE", "PARTITIONS");
            for disk in disks {
                println!(
                    "{:<20} {:<12} {:<10}",
                    disk.device.display(),
                    aegis_common::utils::human_bytes(disk.size_bytes),
                    disk.partitions.len()
                );
            }
        }
        _ => print_disk_list_local(),
    }
}

pub fn print_jobs() {
    match client::send_request_sync(&DaemonRequest::ListJobs) {
        Ok(DaemonResponse::Jobs(jobs)) => {
            if jobs.is_empty() {
                println!("No jobs queued.");
                return;
            }
            println!("{:<36} {:<12} {:?}", "JOB ID", "STATUS", "TYPE");
            for job in jobs {
                println!("{:<36} {:<12?} {:?}", job.id, job.status, job.job_type);
            }
        }
        Ok(DaemonResponse::Error { message, .. }) => {
            println!("aegisd error: {}", message);
        }
        _ => {
            println!("Jobs: (connect to aegisd for job list)");
            println!("  Run 'aegisd' daemon first, then use this command.");
        }
    }
}

fn print_local_status_fallback() {
    println!("System Status:");
    println!("  aegisd:       {}", check_daemon());
    println!("  Secure Boot:  {}", check_secure_boot());
    println!("  Network mode: Air-Gap (default)");
    println!("  dm-verity:    {}", check_dm_verity());
}

fn print_disk_list_local() {
    println!("Scanning block devices...");
    let output = std::process::Command::new("lsblk")
        .args(["-o", "NAME,SIZE,TYPE,FSTYPE,LABEL,MOUNTPOINT", "--json"])
        .output();

    match output {
        Ok(o) => {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                if let Some(devs) = json["blockdevices"].as_array() {
                    println!("{:<15} {:<10} {:<8} {:<12} {}",
                        "NAME", "SIZE", "TYPE", "FSTYPE", "MOUNTPOINT");
                    for dev in devs {
                        println!("{:<15} {:<10} {:<8} {:<12} {}",
                            dev["name"].as_str().unwrap_or("?"),
                            dev["size"].as_str().unwrap_or("?"),
                            dev["type"].as_str().unwrap_or("?"),
                            dev["fstype"].as_str().unwrap_or(""),
                            dev["mountpoint"].as_str().unwrap_or(""),
                        );
                    }
                }
            }
        }
        Err(_) => println!("lsblk not available"),
    }
}

fn check_daemon() -> &'static str {
    if std::path::Path::new(aegis_common::ipc::AEGIS_SOCKET_PATH).exists() {
        "✓ running"
    } else {
        "✗ not running"
    }
}

fn check_secure_boot() -> &'static str {
    let path = "/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c";
    if let Ok(data) = std::fs::read(path) {
        if data.get(4).copied() == Some(1) { "✓ enabled" } else { "⚠ disabled" }
    } else {
        "? (UEFI vars not accessible)"
    }
}

fn check_dm_verity() -> &'static str {
    if std::fs::read_to_string("/proc/mounts")
        .map(|m| m.contains("dm-verity"))
        .unwrap_or(false)
    {
        "✓ active"
    } else {
        "? not detected"
    }
}
