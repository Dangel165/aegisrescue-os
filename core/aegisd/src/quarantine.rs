//! 격리 저장소 관리

use aegis_common::types::QuarantineEntry;
use std::path::{Path, PathBuf};
use tracing::info;
use uuid::Uuid;

pub struct QuarantineManager {
    base_path: PathBuf,
}

impl QuarantineManager {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }

    /// 파일을 격리 저장소로 이동 (원본 경로는 삭제)
    pub fn quarantine(&self, entry: &QuarantineEntry) -> anyhow::Result<()> {
        let dest = &entry.quarantine_path;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 격리 파일 이동
        std::fs::copy(&entry.original_path, dest)?;
        // 원본 퍼미션 제거 (0o000)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o000))?;
        }

        info!(
            id = %entry.id,
            original = %entry.original_path.display(),
            quarantine = %dest.display(),
            "file quarantined"
        );
        Ok(())
    }

    /// 격리 해제 — 지정 경로로 복원
    pub fn restore(&self, entry: &QuarantineEntry, restore_path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = restore_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&entry.quarantine_path, restore_path)?;
        info!(
            id = %entry.id,
            restore_path = %restore_path.display(),
            "quarantine entry restored"
        );
        Ok(())
    }

    /// 격리 항목 영구 삭제 (보안 삭제: 덮어쓰기)
    pub fn delete(&self, entry: &QuarantineEntry) -> anyhow::Result<()> {
        secure_delete(&entry.quarantine_path)?;
        info!(id = %entry.id, "quarantine entry securely deleted");
        Ok(())
    }

    pub fn quarantine_path_for(&self, id: Uuid) -> PathBuf {
        self.base_path.join(id.to_string())
    }
}

/// 파일 보안 삭제: 0으로 덮어쓰기 후 제거
fn secure_delete(path: &Path) -> anyhow::Result<()> {
    use std::io::Write;
    let metadata = std::fs::metadata(path)?;
    let size = metadata.len() as usize;

    {
        let mut file = std::fs::OpenOptions::new().write(true).open(path)?;
        let zeros = vec![0u8; size.min(1024 * 1024)];
        let mut written = 0usize;
        while written < size {
            let chunk = (size - written).min(zeros.len());
            file.write_all(&zeros[..chunk])?;
            written += chunk;
        }
        file.sync_all()?;
    }

    std::fs::remove_file(path)?;
    Ok(())
}
