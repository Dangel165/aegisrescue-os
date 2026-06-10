//! Windows Volume Shadow Copy (VSS) 복원

use std::path::{Path, PathBuf};

pub struct VssStats {
    pub total: u64,
    pub recovered: u64,
}

pub async fn recover_from_vss(target: &Path, output: &Path) -> anyhow::Result<VssStats> {
    std::fs::create_dir_all(output)?;

    // VSS 스냅샷 목록 탐색
    let shadows = list_vss_snapshots(target);

    if shadows.is_empty() {
        return Err(anyhow::anyhow!("No VSS snapshots found on {}", target.display()));
    }

    tracing::info!(count = shadows.len(), "VSS snapshots found");

    let mut total = 0u64;
    let mut recovered = 0u64;

    for shadow_path in &shadows {
        tracing::info!(shadow = %shadow_path.display(), "restoring from VSS snapshot");

        // vsshadow / diskshadow 마운트 후 파일 복사
        match restore_from_shadow(shadow_path, output) {
            Ok(count) => {
                recovered += count;
                total += count;
            }
            Err(e) => {
                tracing::warn!("VSS restore from {} failed: {}", shadow_path.display(), e);
            }
        }
    }

    Ok(VssStats { total, recovered })
}

/// VSS 스냅샷 경로 탐색
/// 마운트된 Windows 볼륨의 System Volume Information 하위에서 탐색
fn list_vss_snapshots(root: &Path) -> Vec<PathBuf> {
    let mut shadows = vec![];

    // 직접 마운트된 VSS 경로 (vssadmin/diskshadow으로 미리 마운트된 경우)
    let vss_mount_base = Path::new("/tmp/aegis_vss");
    if vss_mount_base.exists() {
        if let Ok(entries) = std::fs::read_dir(vss_mount_base) {
            for entry in entries.filter_map(|e| e.ok()) {
                shadows.push(entry.path());
            }
        }
        return shadows;
    }

    // Windows System Volume Information에서 VSS 탐색
    let svi = root.join("System Volume Information");
    if svi.exists() {
        // vsshadow 도구로 마운트 시도
        let _ = mount_vss_snapshots(&svi, vss_mount_base);

        if let Ok(entries) = std::fs::read_dir(vss_mount_base) {
            for entry in entries.filter_map(|e| e.ok()) {
                shadows.push(entry.path());
            }
        }
    }

    shadows
}

fn mount_vss_snapshots(svi_path: &Path, mount_base: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(mount_base)?;

    // vsshadow -p (libvshadow 기반) 도구 사용
    let output = std::process::Command::new("vsshadow")
        .args(["-p", &svi_path.parent().unwrap_or(svi_path).to_string_lossy()])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            tracing::info!("VSS snapshots mounted via vsshadow");
        }
        _ => {
            tracing::warn!("vsshadow not available, trying wmic method");
        }
    }

    Ok(())
}

fn restore_from_shadow(shadow: &Path, output: &Path) -> anyhow::Result<u64> {
    let mut count = 0u64;

    for entry in walkdir::WalkDir::new(shadow)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let rel = entry.path().strip_prefix(shadow).unwrap_or(entry.path());
        let dest = output.join("vss_restore").join(rel);

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        if std::fs::copy(entry.path(), &dest).is_ok() {
            count += 1;
        }
    }

    Ok(count)
}
