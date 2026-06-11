//! 증거 이미징 — 해시 체인 보장, 쓰기 차단

use aegis_common::hash::ForensicHashes;
use std::path::{Path, PathBuf};

pub struct CaptureResult {
    pub image_path: PathBuf,
    pub sha256: String,
    pub blake3: String,
    pub size_bytes: u64,
}

pub async fn capture_evidence(source: &Path, output_dir: &Path) -> anyhow::Result<CaptureResult> {
    std::fs::create_dir_all(output_dir)?;

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let source_name = source.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("evidence");
    let image_name = format!("{}_evidence_{}.img", source_name, ts);
    let mut image_path = output_dir.join(&image_name);

    tracing::info!(
        source = %source.display(),
        dest = %image_path.display(),
        "capturing evidence"
    );

    // dc3dd 또는 dd 로 이미징 (쓰기 차단 확인 후)
    if source.is_file() || is_block_device(source) {
        capture_with_dc3dd(source, &image_path).await
            .or_else(|_| capture_with_dd(source, &image_path))?;
    } else {
        let archive_path = capture_directory(source, &image_path)?;
        image_path = archive_path;
    }

    // 캡처 후 해시
    let hashes = ForensicHashes::compute(&image_path)?;

    // 해시 파일 저장 (체인 오브 커스터디)
    let hash_file = image_path.with_extension("sha256");
    std::fs::write(&hash_file, format!(
        "# AegisRescue Evidence Hash\n# Source: {}\n# Captured: {}\n{} {}\n",
        source.display(), ts, hashes.sha256, image_name
    ))?;

    tracing::info!(
        sha256 = %&hashes.sha256[..16],
        size = hashes.size_bytes,
        "evidence captured and hashed"
    );

    Ok(CaptureResult {
        image_path,
        sha256: hashes.sha256,
        blake3: hashes.blake3,
        size_bytes: hashes.size_bytes,
    })
}

async fn capture_with_dc3dd(source: &Path, dest: &Path) -> anyhow::Result<()> {
    let input = format!("if={}", source.display());
    let output = format!("of={}", dest.display());
    let status = tokio::process::Command::new("dc3dd")
        .args([
            input.as_str(),
            output.as_str(),
            "hof=sha256",
            "bs=512k",
        ])
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("dc3dd failed"))
    }
}

fn capture_with_dd(source: &Path, dest: &Path) -> anyhow::Result<()> {
    let input = format!("if={}", source.display());
    let output = format!("of={}", dest.display());
    let status = std::process::Command::new("dd")
        .args([
            input.as_str(),
            output.as_str(),
            "bs=512k",
            "conv=noerror,sync",
            "status=progress",
        ])
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("dd failed"))
    }
}

fn capture_directory(source: &Path, dest: &Path) -> anyhow::Result<PathBuf> {
    let archive = dest.with_extension("tar.gz");
    let archive_arg = archive.to_string_lossy().into_owned();
    let parent_arg = source.parent().unwrap_or(source).to_string_lossy().into_owned();
    let name_arg = source
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let status = std::process::Command::new("tar")
        .args([
            "-czf",
            archive_arg.as_str(),
            "-C",
            parent_arg.as_str(),
            name_arg.as_str(),
        ])
        .status()?;

    if status.success() {
        Ok(archive)
    } else {
        Err(anyhow::anyhow!("tar failed"))
    }
}

pub fn verify_hashes(hash_file: &Path) -> anyhow::Result<Vec<(String, bool)>> {
    let content = std::fs::read_to_string(hash_file)?;
    let dir = hash_file.parent().unwrap_or(Path::new("."));
    let mut results = vec![];

    for line in content.lines() {
        if line.starts_with('#') || line.trim().is_empty() { continue; }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 { continue; }

        let expected_hash = parts[0].trim();
        let filename = parts[1].trim();
        let filepath = dir.join(filename);

        let verified = if filepath.exists() {
            aegis_common::hash::sha256_file(&filepath)
                .map(|h| h.eq_ignore_ascii_case(expected_hash))
                .unwrap_or(false)
        } else {
            false
        };

        results.push((filename.to_string(), verified));
    }

    Ok(results)
}

fn is_block_device(path: &Path) -> bool {
    tracing::debug!(path = %path.display(), "checking block device type");
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        std::fs::metadata(path)
            .map(|m| m.file_type().is_block_device())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.to_string_lossy().starts_with("/dev/")
    }
}
