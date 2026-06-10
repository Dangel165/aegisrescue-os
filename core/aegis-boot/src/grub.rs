//! GRUB 복구

use std::path::Path;

pub async fn repair_grub(device: &Path, linux_root: &Path) -> anyhow::Result<()> {
    tracing::info!(device = %device.display(), root = %linux_root.display(), "repairing GRUB");

    // chroot 환경 구성
    mount_bind_filesystems(linux_root).await;

    let root_arg = linux_root.to_string_lossy().into_owned();
    let device_arg = device.to_string_lossy().into_owned();

    // grub-install 실행 (chroot)
    let status = tokio::process::Command::new("chroot")
        .args([
            root_arg.as_str(),
            "grub-install",
            device_arg.as_str(),
        ])
        .status()
        .await?;

    if status.success() {
        // update-grub 실행
        let _ = tokio::process::Command::new("chroot")
            .args([
                root_arg.as_str(),
                "update-grub",
            ])
            .status()
            .await;

        tracing::info!("GRUB installed and updated");
    } else {
        // grub2-install 폴백 (Fedora/RHEL 계열)
        let _ = tokio::process::Command::new("chroot")
            .args([
                root_arg.as_str(),
                "grub2-install",
                device_arg.as_str(),
            ])
            .status()
            .await;
    }

    // 바인드 마운트 해제
    umount_bind_filesystems(linux_root).await;

    Ok(())
}

async fn mount_bind_filesystems(root: &Path) {
    let binds = [
        ("/dev",     "dev"),
        ("/dev/pts", "dev/pts"),
        ("/proc",    "proc"),
        ("/sys",     "sys"),
    ];

    for (src, rel_dst) in &binds {
        let dst = root.join(rel_dst);
        std::fs::create_dir_all(&dst).ok();
        let dst_arg = dst.to_string_lossy().into_owned();
        let _ = tokio::process::Command::new("mount")
            .args(["--bind", src, dst_arg.as_str()])
            .status()
            .await;
    }
}

async fn umount_bind_filesystems(root: &Path) {
    let binds = ["dev/pts", "dev", "proc", "sys"];
    for rel in binds {
        let dst = root.join(rel);
        let dst_arg = dst.to_string_lossy().into_owned();
        let _ = tokio::process::Command::new("umount")
            .arg(dst_arg)
            .status()
            .await;
    }
}
