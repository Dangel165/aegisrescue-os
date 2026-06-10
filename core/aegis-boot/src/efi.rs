//! EFI 부트로더 복구

use std::path::Path;

pub async fn repair_efi(esp: &Path) -> anyhow::Result<()> {
    tracing::info!(esp = %esp.display(), "repairing EFI boot");

    // ESP 마운트 확인
    if !esp.exists() {
        return Err(anyhow::anyhow!("ESP path does not exist: {}", esp.display()));
    }

    // efibootmgr로 부트 항목 조회
    let output = tokio::process::Command::new("efibootmgr")
        .arg("-v")
        .output()
        .await;

    match output {
        Ok(o) => {
            let entries = String::from_utf8_lossy(&o.stdout);
            tracing::info!("Current EFI boot entries:\n{}", entries);
        }
        Err(_) => tracing::warn!("efibootmgr not available"),
    }

    Ok(())
}

pub async fn install_bootloader(esp: &Path, os: &str) -> anyhow::Result<()> {
    match os.to_lowercase().as_str() {
        "windows" => install_windows_efi(esp).await,
        "linux" | "grub" => install_grub_efi(esp).await,
        "systemd-boot" => install_systemd_boot(esp).await,
        _ => Err(anyhow::anyhow!("Unknown OS: {}", os)),
    }
}

async fn install_windows_efi(esp: &Path) -> anyhow::Result<()> {
    // Windows 부트로더 파일 확인 및 복원
    let windows_efi = esp.join("EFI/Microsoft/Boot/bootmgfw.efi");

    if !windows_efi.exists() {
        // Windows PE 이미지에서 bootmgfw.efi 복사
        let source = Path::new("/usr/share/aegis/efi/bootmgfw.efi");
        if source.exists() {
            std::fs::create_dir_all(windows_efi.parent().unwrap())?;
            std::fs::copy(source, &windows_efi)?;
            tracing::info!("Windows EFI bootloader restored");
        } else {
            tracing::warn!("Windows EFI bootloader template not found");
        }
    }

    // efibootmgr로 부트 항목 추가
    let _ = tokio::process::Command::new("efibootmgr")
        .args([
            "--create",
            "--label", "Windows Boot Manager",
            "--loader", "\\EFI\\Microsoft\\Boot\\bootmgfw.efi",
        ])
        .status()
        .await;

    Ok(())
}

async fn install_grub_efi(esp: &Path) -> anyhow::Result<()> {
    let status = tokio::process::Command::new("grub-install")
        .args([
            "--target=x86_64-efi",
            &format!("--efi-directory={}", esp.display()),
            "--bootloader-id=AegisRescue",
            "--removable",
        ])
        .status()
        .await?;

    if status.success() {
        tracing::info!("GRUB EFI installed");
    }

    Ok(())
}

async fn install_systemd_boot(esp: &Path) -> anyhow::Result<()> {
    let status = tokio::process::Command::new("bootctl")
        .args(["install", &format!("--esp-path={}", esp.display())])
        .status()
        .await?;

    if status.success() {
        tracing::info!("systemd-boot installed");
    }

    Ok(())
}
