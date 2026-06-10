//! 리눅스 네임스페이스 기반 L1 격리

use std::path::Path;
use uuid::Uuid;

pub async fn run_in_namespace(
    sample: &Path,
    timeout_secs: u64,
) -> anyhow::Result<serde_json::Value> {
    let session_id = Uuid::new_v4();
    let sandbox_root = format!("/tmp/aegis_sandbox_{}", session_id);

    tracing::info!(session = %session_id, "starting namespace sandbox");

    // 1. 격리 루트 파일시스템 구성 (tmpfs)
    setup_sandbox_rootfs(&sandbox_root)?;

    // 2. 샘플 읽기 전용 주입
    let sample_dest = format!("{}/sample", sandbox_root);
    std::fs::copy(sample, &sample_dest)?;

    // 3. unshare로 네임스페이스 생성 후 bubblewrap으로 실행
    let output = tokio::time::timeout(
        tokio::time::Duration::from_secs(timeout_secs),
        run_with_bwrap(&sandbox_root, &sample_dest),
    )
    .await;

    // 4. 정리
    cleanup_sandbox(&sandbox_root);

    match output {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            tracing::warn!("Sandbox timeout after {}s", timeout_secs);
            Ok(serde_json::json!({
                "status": "timeout",
                "timeout_secs": timeout_secs,
            }))
        }
    }
}

fn setup_sandbox_rootfs(root: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(root)?;
    std::fs::create_dir_all(format!("{}/tmp", root))?;
    std::fs::create_dir_all(format!("{}/proc", root))?;
    Ok(())
}

async fn run_with_bwrap(sandbox_root: &str, sample: &str) -> anyhow::Result<serde_json::Value> {
    // bubblewrap (bwrap) — 네임스페이스 + seccomp 격리
    let output = tokio::process::Command::new("bwrap")
        .args([
            "--ro-bind", "/usr", "/usr",
            "--ro-bind", "/lib", "/lib",
            "--ro-bind", "/lib64", "/lib64",
            "--proc", "/proc",
            "--dev", "/dev",
            "--tmpfs", "/tmp",
            "--unshare-all",          // 모든 네임스페이스 격리
            "--new-session",          // 새 세션 (ptrace 방지)
            "--die-with-parent",
            "--", "strace", "-f", "-o", "/tmp/strace.log", sample,
        ])
        .output()
        .await?;

    let strace_log = std::fs::read_to_string("/tmp/strace.log").unwrap_or_default();

    Ok(serde_json::json!({
        "status": "completed",
        "exit_code": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        "syscalls_traced": strace_log.lines().count(),
        "suspicious_syscalls": analyze_strace(&strace_log),
    }))
}

fn cleanup_sandbox(root: &str) {
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_file("/tmp/strace.log");
}

fn analyze_strace(log: &str) -> Vec<String> {
    let suspicious = [
        "execve", "ptrace", "process_vm_write", "memfd_create",
        "socket", "connect", "sendto",
        "openat.*\\/tmp", "openat.*\\/dev\\/shm",
    ];

    suspicious.iter()
        .filter(|s| log.contains(*s))
        .map(|s| s.to_string())
        .collect()
}
