//! 파일시스템 타임라인 생성

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub timestamp: String,
    pub event_type: String,
    pub path: String,
    pub size: u64,
    pub permissions: Option<String>,
}

pub async fn build_timeline(root: &Path, output: &Path) -> anyhow::Result<()> {
    let mut entries: Vec<TimelineEntry> = vec![];

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        // mtime (수정 시간)
        if let Ok(modified) = metadata.modified() {
            let dt: chrono::DateTime<chrono::Utc> = modified.into();
            entries.push(TimelineEntry {
                timestamp: dt.to_rfc3339(),
                event_type: "mtime".into(),
                path: path.to_string_lossy().to_string(),
                size: metadata.len(),
                permissions: get_permissions(&metadata),
            });
        }

        // atime (접근 시간)
        if let Ok(accessed) = metadata.accessed() {
            let dt: chrono::DateTime<chrono::Utc> = accessed.into();
            entries.push(TimelineEntry {
                timestamp: dt.to_rfc3339(),
                event_type: "atime".into(),
                path: path.to_string_lossy().to_string(),
                size: metadata.len(),
                permissions: None,
            });
        }

        // ctime (변경 시간) - Unix only
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let ctime_secs = metadata.ctime();
            if ctime_secs > 0 {
                let dt = chrono::DateTime::from_timestamp(ctime_secs, 0)
                    .unwrap_or_default();
                entries.push(TimelineEntry {
                    timestamp: dt.to_rfc3339(),
                    event_type: "ctime".into(),
                    path: path.to_string_lossy().to_string(),
                    size: metadata.len(),
                    permissions: None,
                });
            }
        }
    }

    // 시간순 정렬
    entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    // JSONL 출력
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = std::fs::File::create(output)?;
    for entry in &entries {
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{}", line)?;
    }

    tracing::info!(entries = entries.len(), "timeline built");
    Ok(())
}

fn get_permissions(metadata: &std::fs::Metadata) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Some(format!("{:o}", metadata.permissions().mode()))
    }
    #[cfg(not(unix))]
    { None }
}
