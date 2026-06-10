//! aegisd 격리 요청 클라이언트

use std::path::Path;

pub fn request_quarantine(path: &Path) -> anyhow::Result<()> {
    // The scanner may run in an offline/live context before aegisd is ready.
    // Leave a durable marker next to the target so aegisd/GUI can pick it up
    // later without mutating suspicious evidence immediately.
    let marker = path.with_extension("aegis-quarantine-request");
    std::fs::write(
        &marker,
        format!(
            "path={}\nrequested_at={}\n",
            path.display(),
            chrono::Utc::now().to_rfc3339()
        ),
    )?;
    tracing::info!(
        path = %path.display(),
        marker = %marker.display(),
        "quarantine request recorded"
    );
    Ok(())
}
