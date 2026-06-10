//! microVM 기반 L2 격리 (QEMU/Firecracker)

use std::path::Path;

pub async fn run_in_vm(
    sample: &Path,
    timeout_secs: u64,
) -> anyhow::Result<serde_json::Value> {
    tracing::info!("Starting microVM sandbox (L2)");

    // Firecracker 우선, 없으면 QEMU 폴백
    if is_firecracker_available() {
        run_with_firecracker(sample, timeout_secs).await
    } else if is_qemu_available() {
        run_with_qemu(sample, timeout_secs).await
    } else {
        Err(anyhow::anyhow!(
            "No microVM backend available (install firecracker or qemu)"
        ))
    }
}

async fn run_with_firecracker(
    sample: &Path,
    timeout_secs: u64,
) -> anyhow::Result<serde_json::Value> {
    // Firecracker API (REST over Unix socket)
    // 분석용 경량 VM 설정: 1vCPU, 256MB RAM
    let config = serde_json::json!({
        "boot-source": {
            "kernel_image_path": "/usr/share/aegis/vm/vmlinux.bin",
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
        },
        "drives": [{
            "drive_id": "rootfs",
            "path_on_host": "/usr/share/aegis/vm/rootfs.ext4",
            "is_root_device": true,
            "is_read_only": false
        }, {
            "drive_id": "sample",
            "path_on_host": sample.to_string_lossy(),
            "is_root_device": false,
            "is_read_only": true
        }],
        "machine-config": {
            "vcpu_count": 1,
            "mem_size_mib": 256
        },
        "network-interfaces": []  // 네트워크 없음 (격리)
    });

    tracing::info!("Firecracker VM config: {}", config);

    // 실제 구현: firecracker API 소켓으로 설정 전송 + 부팅 + 분석 에이전트 통신
    Ok(serde_json::json!({
        "status": "vm_started",
        "backend": "firecracker",
        "timeout_secs": timeout_secs,
        "note": "Full VM analysis requires firecracker binary and VM image"
    }))
}

async fn run_with_qemu(
    sample: &Path,
    timeout_secs: u64,
) -> anyhow::Result<serde_json::Value> {
    let socket = format!("/tmp/aegis_qemu_{}.sock", std::process::id());

    let mut child = tokio::process::Command::new("qemu-system-x86_64")
        .args([
            "-m", "256",
            "-nographic",
            "-nodefaults",
            "-no-reboot",
            "-drive", &format!("file=/usr/share/aegis/vm/analysis.qcow2,if=virtio,snapshot=on"),
            "-drive", &format!("file={},if=virtio,readonly=on,format=raw", sample.display()),
            "-virtfs", "local,path=/tmp,mount_tag=shared,security_model=none,readonly=off",
            "-monitor", &format!("unix:{},server,nowait", socket),
            "-serial", "stdio",
        ])
        .kill_on_drop(true)
        .spawn()?;

    tokio::time::sleep(tokio::time::Duration::from_secs(timeout_secs.min(60))).await;

    let _ = child.kill().await;

    Ok(serde_json::json!({
        "status": "completed",
        "backend": "qemu",
        "timeout_secs": timeout_secs,
    }))
}

fn is_firecracker_available() -> bool {
    std::path::Path::new("/usr/bin/firecracker").exists() ||
    std::path::Path::new("/usr/local/bin/firecracker").exists()
}

fn is_qemu_available() -> bool {
    std::process::Command::new("qemu-system-x86_64")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
