//! Windows/Linux 포렌식 아티팩트 수집

use std::path::{Path, PathBuf};

pub async fn collect_all(root: &Path, output: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(output)?;

    // Windows 아티팩트
    collect_windows_artifacts(root, output).await;

    // Linux 아티팩트
    collect_linux_artifacts(root, output).await;

    tracing::info!(output = %output.display(), "artifact collection complete");
    Ok(())
}

async fn collect_windows_artifacts(root: &Path, output: &Path) {
    let windows_artifacts: &[(&str, &str)] = &[
        // 레지스트리 하이브
        ("Windows/System32/config/SAM",      "registry/SAM"),
        ("Windows/System32/config/SYSTEM",   "registry/SYSTEM"),
        ("Windows/System32/config/SOFTWARE", "registry/SOFTWARE"),
        ("Windows/System32/config/SECURITY", "registry/SECURITY"),
        // 이벤트 로그
        ("Windows/System32/winevt/Logs",     "eventlogs/"),
        // Prefetch
        ("Windows/Prefetch",                 "prefetch/"),
        // MFT (파일 시스템 마스터 파일 테이블)
        ("$MFT",                             "mft/$MFT"),
        // 최근 문서
        ("Users",                            "users/"),
    ];

    let out_win = output.join("windows");
    std::fs::create_dir_all(&out_win).ok();

    for (src_rel, dst_rel) in windows_artifacts {
        let src = root.join(src_rel);
        if !src.exists() { continue; }

        let dst = out_win.join(dst_rel);

        if src.is_dir() {
            copy_dir(&src, &dst);
        } else {
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::copy(&src, &dst).ok();
            tracing::info!(artifact = src_rel, "collected");
        }
    }

    // USN 저널 수집
    collect_usn_journal(root, &out_win).await;

    // Amcache, Shimcache
    collect_shimcache(root, &out_win);
}

async fn collect_usn_journal(root: &Path, output: &Path) {
    // USN 저널: $Extend/$UsnJrnl:$J
    let usn_path = root.join("$Extend/$UsnJrnl");
    if usn_path.exists() {
        let dst = output.join("journal/usn_journal.bin");
        if let Some(p) = dst.parent() { std::fs::create_dir_all(p).ok(); }
        std::fs::copy(&usn_path, &dst).ok();
        tracing::info!("USN journal collected");
    }
}

fn collect_shimcache(root: &Path, output: &Path) {
    // AppCompatCache (shimcache)는 SYSTEM 하이브에 포함됨 (별도 파싱 필요)
    // 여기서는 SYSTEM 하이브가 이미 수집되어 있으면 shimcache는 별도 파싱 도구로 처리
    let system_hive = output.join("registry/SYSTEM");
    if system_hive.exists() {
        // AppCompatCacheParser (외부 도구) 또는 자체 파서
        let _ = std::process::Command::new("appcompatcacheparser")
            .args(["-f", &system_hive.to_string_lossy(), "--csv", &output.to_string_lossy()])
            .status();
    }
}

async fn collect_linux_artifacts(root: &Path, output: &Path) {
    let linux_artifacts: &[(&str, &str)] = &[
        ("etc/passwd",           "linux/passwd"),
        ("etc/shadow",           "linux/shadow"),
        ("etc/sudoers",          "linux/sudoers"),
        ("etc/hosts",            "linux/hosts"),
        ("etc/crontab",          "linux/crontab"),
        ("var/log",              "linux/logs/"),
        ("etc/systemd/system",   "linux/systemd/"),
        ("root/.bash_history",   "linux/bash_history_root"),
    ];

    let out_linux = output.join("linux");
    std::fs::create_dir_all(&out_linux).ok();

    for (src_rel, dst_rel) in linux_artifacts {
        let src = root.join(src_rel);
        if !src.exists() { continue; }

        let dst = output.join(dst_rel);

        if src.is_dir() {
            copy_dir(&src, &dst);
        } else {
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::copy(&src, &dst).ok();
            tracing::info!(artifact = src_rel, "collected");
        }
    }

    // /proc/kcore (메모리 스냅샷, 루트 필요)
    if root == Path::new("/") {
        collect_memory_snapshot(output).await;
    }
}

async fn collect_memory_snapshot(output: &Path) {
    let mem_output = output.join("memory/mem.dump");
    if let Some(p) = mem_output.parent() { std::fs::create_dir_all(p).ok(); }

    // 가벼운 메모리 덤프 (LiME 또는 /dev/mem)
    let _ = tokio::process::Command::new("insmod")
        .args(["lime.ko", &format!("path={}", mem_output.display()), "format=lime"])
        .status()
        .await;
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).ok();
    for entry in walkdir::WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let rel = entry.path().strip_prefix(src).unwrap_or(entry.path());
        let dest = dst.join(rel);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest).ok();
        } else {
            std::fs::copy(entry.path(), &dest).ok();
        }
    }
}
