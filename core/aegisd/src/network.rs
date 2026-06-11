//! 네트워크 격리 정책 적용

use aegis_common::policy::{NetworkMode, NetworkPolicy};
use tracing::{info, warn};

const NFT_AIRGAP: &str = "/usr/share/aegis/nftables/airgap.nft";
const NFT_WHITELIST: &str = "/usr/share/aegis/nftables/whitelist.nft";

pub async fn apply_policy(policy: &NetworkPolicy) {
    match &policy.default {
        NetworkMode::AirGap => {
            info!("Applying Air-Gap network policy (full block)");
            if !apply_nft_file(NFT_AIRGAP).await {
                run_nftables_airgap_inline().await;
            }
        }
        NetworkMode::Whitelist => {
            info!("Applying Whitelist network policy");
            if !apply_nft_file(NFT_WHITELIST).await {
                run_nftables_whitelist(&policy.allowed_hosts).await;
            }
        }
        NetworkMode::Restricted => {
            info!("Applying Restricted network policy (outbound only)");
            run_nftables_restricted().await;
        }
        NetworkMode::Full => {
            warn!("Network policy set to FULL — all traffic allowed (not recommended)");
        }
    }
}

async fn apply_nft_file(path: &str) -> bool {
    if !std::path::Path::new(path).exists() {
        return false;
    }
    let status = tokio::process::Command::new("nft")
        .args(["-f", path])
        .status()
        .await;
    match status {
        Ok(s) if s.success() => {
            info!(rules = path, "nftables rules applied from file");
            true
        }
        Ok(s) => {
            warn!(rules = path, code = ?s.code(), "nft file apply failed");
            false
        }
        Err(e) => {
            warn!(rules = path, error = %e, "nft not available");
            false
        }
    }
}

async fn run_nftables_airgap_inline() {
    let rules = include_str!("../../../security/nftables/airgap.nft");
    apply_nft_rules(rules).await;
}

async fn run_nftables_whitelist(allowed_hosts: &[String]) {
    if apply_nft_file(NFT_WHITELIST).await && !allowed_hosts.is_empty() {
        let mut extra = String::new();
        for host in allowed_hosts {
            extra.push_str(&format!(
                "add rule inet aegis_whitelist output ip daddr {} accept\n",
                host
            ));
        }
        let tmp = format!("/tmp/aegis_whitelist_extra_{}.nft", std::process::id());
        if std::fs::write(&tmp, &extra).is_ok() {
            let _ = tokio::process::Command::new("nft")
                .args(["-f", &tmp])
                .status()
                .await;
            let _ = std::fs::remove_file(&tmp);
        }
        return;
    }

    let host_rules: String = allowed_hosts
        .iter()
        .map(|h| format!("        ip daddr {} accept;\n", h))
        .collect();

    let rules = format!(
        r#"
        table inet aegis_whitelist {{
            chain input  {{ type filter hook input  priority 0; policy drop;
                ct state established,related accept;
            }}
            chain output {{ type filter hook output priority 0; policy drop;
                ct state established,related accept;
{host_rules}
            }}
            chain forward {{ type filter hook forward priority 0; policy drop; }}
        }}
    "#
    );
    apply_nft_rules(&rules).await;
}

async fn run_nftables_restricted() {
    let rules = r#"
        table inet aegis_restricted {
            chain input  { type filter hook input  priority 0; policy drop;
                ct state established,related accept;
            }
            chain output { type filter hook output priority 0; policy accept; }
            chain forward{ type filter hook forward priority 0; policy drop; }
        }
    "#;
    apply_nft_rules(rules).await;
}

async fn apply_nft_rules(rules: &str) {
    let tmp = tempfile_path();
    if let Err(e) = std::fs::write(&tmp, rules) {
        warn!("Failed to write nft rules: {}", e);
        return;
    }

    let status = tokio::process::Command::new("nft")
        .args(["-f", &tmp])
        .status()
        .await;

    match status {
        Ok(s) if s.success() => info!("nftables rules applied"),
        Ok(s) => warn!("nft exited with code {:?}", s.code()),
        Err(e) => warn!("Failed to run nft: {} (non-Linux or nft not installed)", e),
    }

    let _ = std::fs::remove_file(&tmp);
}

fn tempfile_path() -> String {
    format!("/tmp/aegis_nft_{}.rules", std::process::id())
}
