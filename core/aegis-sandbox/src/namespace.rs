//! 리눅스 네임스페이스 기반 L1 격리

use std::path::Path;
use uuid::Uuid;

const PACKAGED_SECCOMP: &str = "/usr/share/aegis/seccomp/aegis-sandbox.json";

pub async fn run_in_namespace(
    sample: &Path,
    timeout_secs: u64,
) -> anyhow::Result<serde_json::Value> {
    let session_id = Uuid::new_v4();
    let sandbox_root = format!("/tmp/aegis_sandbox_{}", session_id);

    tracing::info!(session = %session_id, "starting namespace sandbox");

    setup_sandbox_rootfs(&sandbox_root)?;

    let sample_dest = format!("{}/sample", sandbox_root);
    std::fs::copy(sample, &sample_dest)?;

    let seccomp_json = format!("{}/seccomp.json", sandbox_root);
    write_seccomp_profile(&seccomp_json)?;

    let seccomp_bpf = format!("{}/seccomp.bpf", sandbox_root);
    let seccomp_enforced = compile_seccomp_bpf(&seccomp_json, &seccomp_bpf);

    let output = tokio::time::timeout(
        tokio::time::Duration::from_secs(timeout_secs),
        run_with_bwrap(&sandbox_root, &sample_dest, &seccomp_bpf, seccomp_enforced),
    )
    .await;

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

fn write_seccomp_profile(dest: &str) -> anyhow::Result<()> {
    if std::path::Path::new(PACKAGED_SECCOMP).exists() {
        std::fs::copy(PACKAGED_SECCOMP, dest)?;
    } else {
        std::fs::write(dest, serde_json::to_string_pretty(&crate::seccomp::seccomp_profile())?)?;
    }
    Ok(())
}

fn compile_seccomp_bpf(json_path: &str, bpf_path: &str) -> bool {
    for tool in ["scmp_compile", "pfc"] {
        let ok = std::process::Command::new(tool)
            .args(["-o", bpf_path, json_path])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok && std::path::Path::new(bpf_path).exists() {
            tracing::info!(tool, bpf = bpf_path, "seccomp BPF compiled");
            return true;
        }
    }
    tracing::warn!("seccomp BPF compile skipped — install libseccomp-utils (scmp_compile)");
    false
}

fn setup_sandbox_rootfs(root: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(root)?;
    std::fs::create_dir_all(format!("{}/tmp", root))?;
    std::fs::create_dir_all(format!("{}/proc", root))?;
    Ok(())
}

async fn run_with_bwrap(
    sandbox_root: &str,
    sample: &str,
    seccomp_bpf: &str,
    seccomp_enforced: bool,
) -> anyhow::Result<serde_json::Value> {
    let strace_log = format!("{}/strace.log", sandbox_root);

    if seccomp_enforced {
        let script = format!(
            r#"#!/bin/sh
set -e
exec 3<"{seccomp_bpf}"
exec bwrap \
  --ro-bind /usr /usr \
  --ro-bind /lib /lib \
  --ro-bind /lib64 /lib64 \
  --ro-bind {sandbox_root} {sandbox_root} \
  --proc /proc \
  --dev /dev \
  --tmpfs /tmp \
  --unshare-all \
  --new-session \
  --die-with-parent \
  --seccomp 3 \
  -- strace -f -o {strace_log} {sample}
"#
        );
        let script_path = format!("{}/run.sh", sandbox_root);
        std::fs::write(&script_path, script)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
        }
        let output = tokio::process::Command::new(&script_path).output().await?;
        return build_result(sandbox_root, sample, &strace_log, seccomp_bpf, true, &output);
    }

    let output = tokio::process::Command::new("bwrap")
        .args([
            "--ro-bind",
            "/usr",
            "/usr",
            "--ro-bind",
            "/lib",
            "/lib",
            "--ro-bind",
            "/lib64",
            "/lib64",
            "--ro-bind",
            sandbox_root,
            sandbox_root,
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--unshare-all",
            "--new-session",
            "--die-with-parent",
            "--",
            "strace",
            "-f",
            "-o",
            strace_log.as_str(),
            sample,
        ])
        .output()
        .await?;

    build_result(
        sandbox_root,
        sample,
        &strace_log,
        seccomp_bpf,
        false,
        &output,
    )
}

fn build_result(
    sandbox_root: &str,
    sample: &str,
    strace_log: &str,
    seccomp_bpf: &str,
    seccomp_enforced: bool,
    output: &std::process::Output,
) -> anyhow::Result<serde_json::Value> {
    let strace_log_content = std::fs::read_to_string(strace_log).unwrap_or_default();
    Ok(serde_json::json!({
        "status": "completed",
        "sandbox_root": sandbox_root,
        "sample": sample,
        "exit_code": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        "seccomp_enforced": seccomp_enforced,
        "seccomp_rules": crate::seccomp::ALLOWED_SYSCALLS.len(),
        "seccomp_bpf_bytes": std::fs::metadata(seccomp_bpf).map(|m| m.len()).unwrap_or(0),
        "syscalls_traced": strace_log_content.lines().count(),
        "suspicious_syscalls": analyze_strace(&strace_log_content),
    }))
}

fn cleanup_sandbox(root: &str) {
    let _ = std::fs::remove_dir_all(root);
}

fn analyze_strace(log: &str) -> Vec<String> {
    let suspicious = [
        "execve",
        "ptrace",
        "process_vm_write",
        "memfd_create",
        "socket",
        "connect",
        "sendto",
    ];

    suspicious
        .iter()
        .filter(|s| log.contains(*s))
        .map(|s| s.to_string())
        .collect()
}
