//! 네트워크 격리 정책 적용

use aegis_common::policy::{NetworkMode, NetworkPolicy};
use tracing::{info, warn};

pub async fn apply_policy(policy: &NetworkPolicy) {
    match &policy.default {
        NetworkMode::AirGap => {
            info!("Applying Air-Gap network policy (full block)");
            run_nftables_airgap().await;
        }
        NetworkMode::Whitelist => {
            info!("Applying Whitelist network policy");
            run_nftables_whitelist(&policy.allowed_hosts).await;
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

async fn run_nftables_airgap() {
    // nftables air-gap 룰 적용
    // 실제 배포 시 nft 명령으로 실행
    let rules = r#"
        table inet aegis_airgap {
            chain input  { type filter hook input  priority 0; policy drop; }
            chain output { type filter hook output priority 0; policy drop; }
            chain forward{ type filter hook forward priority 0; policy drop; }
        }
    "#;
    apply_nft_rules(rules).await;
}

async fn run_nftables_whitelist(allowed_hosts: &[String]) {
    let host_rules: String = allowed_hosts
        .iter()
        .map(|h| format!("        ip daddr {} accept;\n", h))
        .collect();

    let rules = format!(r#"
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
    "#);
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
    // 임시 파일에 룰 작성 후 nft 실행
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
