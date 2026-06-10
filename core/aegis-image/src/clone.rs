//! 디스크 이미징 및 파일 백업

use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct BackupStats {
    pub files: u64,
    pub bytes: u64,
}

pub async fn create_image(
    source: &Path,
    dest: &Path,
    compress: bool,
) -> anyhow::Result<PathBuf> {
    let image_path = if compress {
        dest.with_extension("img.zst")
    } else {
        dest.with_extension("img")
    };

    if let Some(parent) = image_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tracing::info!(
        source = %source.display(),
        dest = %image_path.display(),
        compress = compress,
        "creating disk image"
    );

    if compress {
        // zstd 압축 파이프라인
        let pipeline = format!(
            "dd if={} bs=1M conv=noerror,sync status=progress | zstd -T0 -3 -o {}",
            source.display(), image_path.display()
        );
        let status = tokio::process::Command::new("sh")
            .args([
                "-c",
                pipeline.as_str(),
            ])
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow::anyhow!("Image creation failed"));
        }
    } else {
        let input = format!("if={}", source.display());
        let output = format!("of={}", image_path.display());
        let status = tokio::process::Command::new("dd")
            .args([
                input.as_str(),
                output.as_str(),
                "bs=1M",
                "conv=noerror,sync",
                "status=progress",
            ])
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow::anyhow!("dd image creation failed"));
        }
    }

    // 이미지 해시 저장
    let hash = aegis_common::hash::sha256_file(&image_path)?;
    let hash_file = image_path.with_extension("sha256");
    std::fs::write(&hash_file, format!("{}  {}\n", hash,
        image_path.file_name().unwrap_or_default().to_string_lossy()))?;

    tracing::info!(sha256 = %&hash[..16], "image hashed");
    Ok(image_path)
}

pub async fn clone_disk(source: &Path, dest: &Path) -> anyhow::Result<()> {
    tracing::info!(source = %source.display(), dest = %dest.display(), "cloning disk");

    // partclone 우선 사용 (파일시스템 인식 + 빠른 복사)
    let fs_type = detect_fs_type(source);

    let status = if let Some(fs) = fs_type {
        let tool = format!("partclone.{}", fs);
        let source_arg = source.to_string_lossy().into_owned();
        let dest_arg = dest.to_string_lossy().into_owned();
        tokio::process::Command::new(&tool)
            .args([
                "-c",  // clone
                "-s", source_arg.as_str(),
                "-o", dest_arg.as_str(),
            ])
            .status()
            .await
    } else {
        // 폴백: dd
        let input = format!("if={}", source.display());
        let output = format!("of={}", dest.display());
        tokio::process::Command::new("dd")
            .args([
                input.as_str(),
                output.as_str(),
                "bs=1M", "conv=noerror,sync", "status=progress",
            ])
            .status()
            .await
    }?;

    if !status.success() {
        return Err(anyhow::anyhow!("Disk clone failed"));
    }

    Ok(())
}

pub async fn backup_files(
    source: &Path,
    dest: &Path,
    incremental: bool,
    passphrase: Option<&str>,
) -> anyhow::Result<BackupStats> {
    tokio::fs::create_dir_all(dest).await?;

    // restic 우선 사용 (중복제거 + 암호화 + 증분)
    if is_tool_available("restic") {
        return backup_with_restic(source, dest, incremental, passphrase).await;
    }

    // borg 폴백
    if is_tool_available("borg") {
        return backup_with_borg(source, dest, passphrase).await;
    }

    // 최후 폴백: tar + zstd
    backup_with_tar(source, dest).await
}

async fn backup_with_restic(
    source: &Path,
    dest: &Path,
    incremental: bool,
    passphrase: Option<&str>,
) -> anyhow::Result<BackupStats> {
    let repo = dest.to_string_lossy().into_owned();
    let source_arg = source.to_string_lossy().into_owned();
    let pass = passphrase.unwrap_or("aegis_backup_default");

    // 초기화 (처음 실행 시)
    let _ = tokio::process::Command::new("restic")
        .env("RESTIC_PASSWORD", pass)
        .args(["init", "-r", repo.as_str()])
        .status()
        .await;

    // 백업
    let output = tokio::process::Command::new("restic")
        .env("RESTIC_PASSWORD", pass)
        .args(["backup", "-r", repo.as_str(), source_arg.as_str()])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    tracing::info!("restic: {}", stdout.trim());

    Ok(BackupStats { files: 0, bytes: 0 }) // 실제 파싱 필요
}

async fn backup_with_borg(
    source: &Path,
    dest: &Path,
    passphrase: Option<&str>,
) -> anyhow::Result<BackupStats> {
    let repo = dest.to_string_lossy().into_owned();
    let source_arg = source.to_string_lossy().into_owned();
    let pass = passphrase.unwrap_or("aegis_backup_default");
    let archive = format!("{}::backup-{}", repo, chrono::Utc::now().format("%Y%m%d-%H%M%S"));

    let _ = tokio::process::Command::new("borg")
        .env("BORG_PASSPHRASE", pass)
        .args(["init", "--encryption=repokey", repo.as_str()])
        .status()
        .await;

    let status = tokio::process::Command::new("borg")
        .env("BORG_PASSPHRASE", pass)
        .args(["create", "--stats", archive.as_str(), source_arg.as_str()])
        .status()
        .await?;

    Ok(BackupStats { files: 0, bytes: 0 })
}

async fn backup_with_tar(source: &Path, dest: &Path) -> anyhow::Result<BackupStats> {
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let archive = dest.join(format!("backup_{}.tar.zst", ts));
    let pipeline = format!(
        "tar -czf - {} | zstd -T0 -3 -o {}",
        source.display(), archive.display()
    );

    let status = tokio::process::Command::new("sh")
        .args(["-c", pipeline.as_str()])
        .status()
        .await?;

    let size = tokio::fs::metadata(&archive).await?.len();
    Ok(BackupStats { files: 1, bytes: size })
}

fn detect_fs_type(device: &Path) -> Option<String> {
    let device_arg = device.to_string_lossy().into_owned();
    let output = std::process::Command::new("blkid")
        .args(["-o", "value", "-s", "TYPE", device_arg.as_str()])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
    if s.is_empty() { None } else { Some(s) }
}

fn is_tool_available(tool: &str) -> bool {
    std::process::Command::new(tool)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
