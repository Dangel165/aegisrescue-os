//! 상태 표시 유틸리티

pub fn print_system_status() {
    println!("System Status:");
    println!("  aegisd:       {}", check_daemon());
    println!("  Secure Boot:  {}", check_secure_boot());
    println!("  Network mode: Air-Gap (default)");
    println!("  dm-verity:    {}", check_dm_verity());
}

pub fn print_disk_list() {
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

pub fn print_jobs() {
    println!("Jobs: (connect to aegisd for job list)");
    println!("  Run 'aegisd' daemon first, then use this command.");
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
