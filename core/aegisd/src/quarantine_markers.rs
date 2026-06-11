//! *.aegis-quarantine-request 마커 파일 처리 (aegis-scan → aegisd)

use crate::state::AppState;
use aegis_common::types::{EventSeverity, EventType, QuarantineEntry, ThreatInfo, ThreatVerdict};
use chrono::Utc;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

const MARKER_SUFFIX: &str = "aegis-quarantine-request";

pub fn spawn_marker_watcher(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            if let Err(e) = process_markers(&state).await {
                warn!(error = %e, "quarantine marker scan failed");
            }
        }
    });
}

pub async fn process_markers(state: &AppState) -> anyhow::Result<()> {
    let roots = marker_search_roots();
    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some(MARKER_SUFFIX) {
                continue;
            }
            if let Err(e) = ingest_marker(state, path).await {
                warn!(marker = %path.display(), error = %e, "marker ingest failed");
            }
        }
    }
    Ok(())
}

async fn ingest_marker(state: &AppState, marker: &Path) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(marker)?;
    let target_path = parse_marker_path(&content, marker)?;
    if !target_path.exists() {
        let _ = std::fs::remove_file(marker);
        return Ok(());
    }

    let threat = ThreatInfo {
        id: Uuid::new_v4(),
        path: target_path.clone(),
        verdict: ThreatVerdict::Malicious,
        risk_score: 0.9,
        malware_family: Some("marker-quarantine".into()),
        malware_type: None,
        detections: vec![],
        hash_sha256: aegis_common::hash::sha256_file(&target_path).unwrap_or_default(),
        size_bytes: std::fs::metadata(&target_path).map(|m| m.len()).unwrap_or(0),
        detected_at: Utc::now(),
    };

    let id = Uuid::new_v4();
    let mgr = state.quarantine_manager();
    let entry = QuarantineEntry {
        id,
        original_path: target_path.clone(),
        quarantine_path: mgr.quarantine_path_for(id),
        threat: threat.clone(),
        quarantined_at: Utc::now(),
        can_restore: true,
    };

    mgr.quarantine(&entry)?;
    state.add_quarantine_entry(entry).await?;
    state.emit_threat(&threat).await;
    state
        .emit_event(
            EventType::QuarantineAdded,
            EventSeverity::Warning,
            format!("Quarantined from marker: {}", target_path.display()),
            serde_json::json!({ "entry_id": id, "marker": marker }),
        )
        .await;

    let _ = std::fs::remove_file(marker);
    info!(path = %target_path.display(), "quarantine marker processed");
    Ok(())
}

fn parse_marker_path(content: &str, marker: &Path) -> anyhow::Result<PathBuf> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("path=") {
            return Ok(PathBuf::from(rest.trim()));
        }
    }
    Ok(marker.with_extension(""))
}

fn marker_search_roots() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/mnt/targets"),
        PathBuf::from("/home"),
        PathBuf::from("/tmp"),
        PathBuf::from("/root"),
    ]
}
