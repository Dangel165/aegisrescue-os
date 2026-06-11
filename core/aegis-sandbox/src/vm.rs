//! microVM 기반 L2 격리 (QEMU/Firecracker)

use crate::fc_api::{firecracker_available, FirecrackerSession};
use std::path::Path;

const VM_DIR: &str = "/usr/share/aegis/vm";

pub async fn run_in_vm(
    sample: &Path,
    timeout_secs: u64,
) -> anyhow::Result<serde_json::Value> {
    tracing::info!("Starting microVM sandbox (L2)");

    if firecracker_available() && vm_artifacts_ready() {
        run_with_firecracker(sample, timeout_secs).await
    } else if is_qemu_available() && Path::new(VM_DIR).join("analysis.qcow2").exists() {
        run_with_qemu(sample, timeout_secs).await
    } else {
        Err(anyhow::anyhow!(
            "L2 sandbox unavailable: need firecracker+{}/vmlinux.bin+rootfs.ext4 or qemu+analysis.qcow2 (run tools/build-vm-artifacts.sh)",
            VM_DIR
        ))
    }
}

async fn run_with_firecracker(
    sample: &Path,
    timeout_secs: u64,
) -> anyhow::Result<serde_json::Value> {
    let mut session = FirecrackerSession::start().await?;
    session.configure(sample).await?;
    session.start_instance().await?;

    let wait = timeout_secs.min(300);
    tokio::time::sleep(tokio::time::Duration::from_secs(wait)).await;

    session.shutdown().await?;

    Ok(serde_json::json!({
        "status": "completed",
        "backend": "firecracker",
        "timeout_secs": wait,
        "sample": sample.display().to_string(),
        "vm_dir": VM_DIR,
    }))
}

async fn run_with_qemu(
    sample: &Path,
    timeout_secs: u64,
) -> anyhow::Result<serde_json::Value> {
    let qcow = Path::new(VM_DIR).join("analysis.qcow2");
    let socket = format!("/tmp/aegis_qemu_{}.sock", std::process::id());

    let mut child = tokio::process::Command::new("qemu-system-x86_64")
        .args([
            "-m",
            "256",
            "-nographic",
            "-nodefaults",
            "-no-reboot",
            "-enable-kvm",
            "-drive",
            &format!("file={},if=virtio,snapshot=on", qcow.display()),
            "-drive",
            &format!(
                "file={},if=virtio,readonly=on,format=raw",
                sample.display()
            ),
            "-monitor",
            &format!("unix:{socket},server,nowait"),
            "-serial",
            "stdio",
        ])
        .kill_on_drop(true)
        .spawn()?;

    let wait = timeout_secs.min(300);
    tokio::time::sleep(tokio::time::Duration::from_secs(wait)).await;
    let _ = child.kill().await;

    Ok(serde_json::json!({
        "status": "completed",
        "backend": "qemu",
        "timeout_secs": wait,
        "sample": sample.display().to_string(),
    }))
}

fn vm_artifacts_ready() -> bool {
    let base = Path::new(VM_DIR);
    base.join("vmlinux.bin").exists() && base.join("rootfs.ext4").exists()
}

fn is_qemu_available() -> bool {
    std::process::Command::new("qemu-system-x86_64")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
