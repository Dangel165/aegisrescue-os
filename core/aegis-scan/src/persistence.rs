//! 영속성 메커니즘 탐지 (오프라인)
//!
//! Windows: 레지스트리 자동실행 키, 서비스, 스케줄 작업, WMI
//! Linux:   systemd unit, cron, rc.local, LD_PRELOAD, init.d

use std::path::Path;

#[derive(Debug)]
pub struct PersistenceItem {
    pub os: &'static str,
    pub mechanism: String,
    pub location: String,
    pub entry: String,
    pub severity: &'static str,
}

pub struct PersistenceChecker;

impl PersistenceChecker {
    pub fn check(root: &Path) -> Vec<PersistenceItem> {
        let mut items = vec![];

        // Windows 영속성 점검
        items.extend(check_windows_registry(root));
        items.extend(check_windows_services(root));
        items.extend(check_windows_scheduled_tasks(root));
        items.extend(check_windows_startup_folder(root));

        // Linux 영속성 점검
        items.extend(check_linux_systemd(root));
        items.extend(check_linux_cron(root));
        items.extend(check_linux_rc_local(root));
        items.extend(check_linux_ld_preload(root));

        items
    }
}

fn check_windows_registry(root: &Path) -> Vec<PersistenceItem> {
    let mut items = vec![];

    // 오프라인 레지스트리 파싱 (nt_hive_parser 또는 외부 도구)
    let autorun_keys = [
        "Windows/System32/config/SOFTWARE",
        "Windows/System32/config/SYSTEM",
    ];

    for hive_path in &autorun_keys {
        let full_path = root.join(hive_path);
        if full_path.exists() {
            // 실제 구현에서는 nt-hive-parser crate 사용
            // 여기서는 외부 hivexget 도구 활용
            if let Some(results) = parse_hive_autorun(&full_path) {
                items.extend(results);
            }
        }
    }

    items
}

fn parse_hive_autorun(hive_path: &Path) -> Option<Vec<PersistenceItem>> {
    let autorun_keys = [
        "Microsoft\\Windows\\CurrentVersion\\Run",
        "Microsoft\\Windows\\CurrentVersion\\RunOnce",
        "Microsoft\\Windows NT\\CurrentVersion\\Winlogon",
    ];

    let mut items = vec![];

    for key in &autorun_keys {
        let hive_arg = hive_path.to_string_lossy().into_owned();
        let output = std::process::Command::new("hivexget")
            .args([hive_arg.as_str(), key])
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if !line.trim().is_empty() {
                    items.push(PersistenceItem {
                        os: "Windows",
                        mechanism: "Registry AutoRun".into(),
                        location: format!("HKLM\\{}", key),
                        entry: line.to_string(),
                        severity: "HIGH",
                    });
                }
            }
        }
    }

    Some(items)
}

fn check_windows_services(root: &Path) -> Vec<PersistenceItem> {
    let services_path = root.join("Windows/System32/config/SYSTEM");
    if !services_path.exists() { return vec![]; }

    // SYSTEM 하이브의 Services 키 파싱
    // 실제 구현에서는 nt-hive-parser 사용
    vec![]
}

fn check_windows_scheduled_tasks(root: &Path) -> Vec<PersistenceItem> {
    let tasks_path = root.join("Windows/System32/Tasks");
    if !tasks_path.exists() { return vec![]; }

    let mut items = vec![];

    for entry in walkdir::WalkDir::new(&tasks_path).max_depth(3)
        .into_iter().filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "xml" || x == "").unwrap_or(false))
    {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            if content.contains("<Actions>") {
                items.push(PersistenceItem {
                    os: "Windows",
                    mechanism: "Scheduled Task".into(),
                    location: entry.path().to_string_lossy().to_string(),
                    entry: "Scheduled task found — review manually".into(),
                    severity: "MEDIUM",
                });
            }
        }
    }

    items
}

fn check_windows_startup_folder(root: &Path) -> Vec<PersistenceItem> {
    let startup_paths = [
        "ProgramData/Microsoft/Windows/Start Menu/Programs/Startup",
        "Users/*/AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Startup",
    ];

    let mut items = vec![];

    for path_pattern in &startup_paths {
        let full = root.join(path_pattern);
        if full.exists() {
            if let Ok(entries) = std::fs::read_dir(&full) {
                for entry in entries.filter_map(|e| e.ok()) {
                    items.push(PersistenceItem {
                        os: "Windows",
                        mechanism: "Startup Folder".into(),
                        location: full.to_string_lossy().to_string(),
                        entry: entry.file_name().to_string_lossy().to_string(),
                        severity: "MEDIUM",
                    });
                }
            }
        }
    }

    items
}

fn check_linux_systemd(root: &Path) -> Vec<PersistenceItem> {
    let unit_dirs = [
        "etc/systemd/system",
        "lib/systemd/system",
        "usr/lib/systemd/system",
    ];

    let mut items = vec![];

    for dir in &unit_dirs {
        let path = root.join(dir);
        if !path.exists() { continue; }

        for entry in walkdir::WalkDir::new(&path).max_depth(1)
            .into_iter().filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "service")
                    .unwrap_or(false)
            })
        {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                // 의심 패턴: ExecStart 가 /tmp 또는 /dev/shm 경로
                if content.contains("/tmp/") || content.contains("/dev/shm/") {
                    items.push(PersistenceItem {
                        os: "Linux",
                        mechanism: "systemd service".into(),
                        location: entry.path().to_string_lossy().to_string(),
                        entry: "Service using temp path (suspicious)".into(),
                        severity: "HIGH",
                    });
                }
            }
        }
    }

    items
}

fn check_linux_cron(root: &Path) -> Vec<PersistenceItem> {
    let cron_paths = [
        "etc/cron.d", "etc/crontab",
        "var/spool/cron/crontabs",
    ];

    let mut items = vec![];

    for path_str in &cron_paths {
        let path = root.join(path_str);
        if !path.exists() { continue; }

        items.push(PersistenceItem {
            os: "Linux",
            mechanism: "cron".into(),
            location: path.to_string_lossy().to_string(),
            entry: "Cron entry present — review manually".into(),
            severity: "LOW",
        });
    }

    items
}

fn check_linux_rc_local(root: &Path) -> Vec<PersistenceItem> {
    let rc_local = root.join("etc/rc.local");
    if !rc_local.exists() { return vec![]; }

    if let Ok(content) = std::fs::read_to_string(&rc_local) {
        if content.trim() != "#!/bin/sh -e\nexit 0" && !content.trim().is_empty() {
            return vec![PersistenceItem {
                os: "Linux",
                mechanism: "rc.local".into(),
                location: rc_local.to_string_lossy().to_string(),
                entry: "Non-default rc.local — review manually".into(),
                severity: "MEDIUM",
            }];
        }
    }

    vec![]
}

fn check_linux_ld_preload(root: &Path) -> Vec<PersistenceItem> {
    let ld_preload = root.join("etc/ld.so.preload");
    if !ld_preload.exists() { return vec![]; }

    if let Ok(content) = std::fs::read_to_string(&ld_preload) {
        if !content.trim().is_empty() {
            return vec![PersistenceItem {
                os: "Linux",
                mechanism: "LD_PRELOAD".into(),
                location: ld_preload.to_string_lossy().to_string(),
                entry: content.trim().to_string(),
                severity: "CRITICAL",
            }];
        }
    }

    vec![]
}
