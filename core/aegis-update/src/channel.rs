//! 온라인 업데이트 채널 (네트워크 기능)

use crate::{manifest::{ComponentRecord, Manifest}, UpdateTarget};
use std::path::Path;

/// 업데이트 가용 여부 확인 (네트워크)
pub async fn check_updates(data_dir: &Path, channel: &str) -> anyhow::Result<()> {
    println!("🔍 업데이트 확인 중 (채널: {})", channel);

    let manifest = Manifest::load(data_dir)?;

    // 네트워크 피처가 없거나 에어갭 환경에서는 오프라인 모드
    #[cfg(not(feature = "network"))]
    {
        println!("\n⚠ 네트워크 업데이트 비활성화됨 (오프라인/에어갭 모드)");
        println!("  오프라인 번들을 사용하세요:");
        println!("  aegis-update apply --bundle <bundle.tar.zst>");
        println!("\n현재 설치된 버전:");
        crate::manifest::print_status(data_dir)?;
        return Ok(());
    }

    #[cfg(feature = "network")]
    {
        check_online(data_dir, channel, &manifest).await?;
    }

    Ok(())
}

#[cfg(feature = "network")]
async fn check_online(
    data_dir: &Path,
    channel: &str,
    current: &Manifest,
) -> anyhow::Result<()> {
    let base_url = update_server_url(channel);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let remote_manifest_url = format!("{}/manifest.json", base_url);
    let response = client.get(&remote_manifest_url).send().await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            let remote: serde_json::Value = resp.json().await?;
            print_update_comparison(current, &remote);
        }
        Ok(resp) => {
            tracing::warn!("서버 응답 오류: {}", resp.status());
            println!("⚠ 업데이트 서버 응답 없음 — 오프라인 모드");
        }
        Err(e) => {
            tracing::warn!("네트워크 오류: {}", e);
            println!("⚠ 네트워크 연결 실패 — 오프라인 번들을 사용하세요");
        }
    }

    Ok(())
}

pub async fn apply_online(
    targets: &[UpdateTarget],
    data_dir: &Path,
    channel: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    #[cfg(not(feature = "network"))]
    {
        println!("⚠ 네트워크 업데이트 비활성화됨");
        println!("  오프라인 번들을 사용하세요: aegis-update apply --bundle <file>");
        return Ok(());
    }

    #[cfg(feature = "network")]
    {
        apply_online_impl(targets, data_dir, channel, dry_run).await?;
    }

    Ok(())
}

#[cfg(feature = "network")]
async fn apply_online_impl(
    targets: &[UpdateTarget],
    data_dir: &Path,
    channel: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;

    let base_url = update_server_url(channel);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let mut manifest = Manifest::load(data_dir)?;

    for target in targets {
        let target_name = format!("{:?}", target).to_lowercase()
            .replace("signatures", "sigs")
            .replace("yararules", "yara")
            .replace("iocdatabase", "ioc")
            .replace("aimodel", "ai-model")
            .replace("corebinaries", "core")
            .replace("securitypolicy", "policy");

        let url = format!("{}/{}.tar.zst", base_url, target_name);

        println!("  ↓ {} 다운로드 중...", target.display_name());

        if dry_run {
            println!("    [dry-run] {}", url);
            continue;
        }

        let tmp_path = std::path::PathBuf::from("/tmp")
            .join(format!("aegis_update_{}.tar.zst", target_name));

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let bytes = resp.bytes().await?;
                let mut f = tokio::fs::File::create(&tmp_path).await?;
                f.write_all(&bytes).await?;

                // 번들 적용
                crate::bundle::apply_bundle(
                    &tmp_path,
                    &[target.clone()],
                    data_dir,
                    std::path::Path::new("/etc/aegis/keys/update.pub"),
                    false,
                )?;

                let _ = tokio::fs::remove_file(&tmp_path).await;
            }
            Ok(resp) => {
                tracing::warn!("{} 다운로드 실패: {}", target.display_name(), resp.status());
            }
            Err(e) => {
                tracing::warn!("{} 네트워크 오류: {}", target.display_name(), e);
            }
        }
    }

    manifest.save(data_dir)?;
    Ok(())
}

#[cfg(feature = "network")]
fn print_update_comparison(current: &Manifest, remote: &serde_json::Value) {
    println!("\n{:<22} {:<16} {}", "컴포넌트", "현재 버전", "최신 버전");
    println!("─────────────────────────────────────────────────");

    let targets = [
        "signatures", "yara", "ioc", "ai-model", "decryptors", "core", "policy"
    ];

    for t in &targets {
        let current_ver = "unknown";
        let remote_ver = remote[t]["version"].as_str().unwrap_or("unknown");
        let update = if current_ver != remote_ver { " ← 업데이트 가능" } else { "" };
        println!("{:<22} {:<16} {}{}", t, current_ver, remote_ver, update);
    }
    println!();
}

fn update_server_url(channel: &str) -> String {
    match channel {
        "testing" => "https://updates.aegisrescue.io/testing".into(),
        _         => "https://updates.aegisrescue.io/stable".into(),
    }
}
