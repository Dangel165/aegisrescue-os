use std::path::Path;

pub async fn restore_image(image: &Path, dest: &Path) -> anyhow::Result<()> {
    tracing::info!(image = %image.display(), dest = %dest.display(), "restoring image");

    let ext = image.extension().and_then(|s| s.to_str()).unwrap_or("");

    let status = if ext == "zst" {
        tokio::process::Command::new("sh")
            .args(["-c", &format!(
                "zstd -d {} -c | dd of={} bs=1M status=progress",
                image.display(), dest.display()
            )])
            .status()
            .await?
    } else {
        tokio::process::Command::new("dd")
            .args([
                &format!("if={}", image.display()),
                &format!("of={}", dest.display()),
                "bs=1M", "status=progress",
            ])
            .status()
            .await?
    };

    if status.success() {
        tracing::info!("restore complete");
        Ok(())
    } else {
        Err(anyhow::anyhow!("restore failed"))
    }
}
