//! 컴포넌트 버전 매니페스트 관리

use crate::UpdateTarget;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub components: Vec<ComponentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRecord {
    pub target: UpdateTarget,
    pub version: String,
    pub updated_at: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub previous_versions: Vec<String>,
}

impl Manifest {
    pub fn load(base: &Path) -> anyhow::Result<Self> {
        let path = manifest_path(base);
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&content).unwrap_or_default())
    }

    pub fn save(&self, base: &Path) -> anyhow::Result<()> {
        let path = manifest_path(base);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn get(&self, target: &UpdateTarget) -> Option<&ComponentRecord> {
        let name = format!("{:?}", target);
        self.components.iter().find(|c| format!("{:?}", c.target) == name)
    }

    pub fn upsert(&mut self, record: ComponentRecord) {
        let name = format!("{:?}", record.target);
        if let Some(pos) = self.components.iter().position(|c| format!("{:?}", c.target) == name) {
            let prev = self.components[pos].version.clone();
            self.components[pos] = record;
            if !self.components[pos].previous_versions.contains(&prev) {
                self.components[pos].previous_versions.push(prev);
                // 최대 5개 이전 버전만 보관
                if self.components[pos].previous_versions.len() > 5 {
                    self.components[pos].previous_versions.remove(0);
                }
            }
        } else {
            self.components.push(record);
        }
    }
}

fn manifest_path(base: &Path) -> PathBuf {
    base.join("update-manifest.toml")
}

pub fn print_status(base: &Path) -> anyhow::Result<()> {
    let manifest = Manifest::load(base)?;

    println!("\n🛡 AegisRescue 컴포넌트 버전");
    println!("─────────────────────────────────────────────────────────────");
    println!("{:<22} {:<16} {:<26} {}", "컴포넌트", "버전", "업데이트", "체크섬(SHA256)");
    println!("─────────────────────────────────────────────────────────────");

    let all_targets = [
        UpdateTarget::Signatures,
        UpdateTarget::YaraRules,
        UpdateTarget::IocDatabase,
        UpdateTarget::AiModel,
        UpdateTarget::Decryptors,
        UpdateTarget::CoreBinaries,
        UpdateTarget::SecurityPolicy,
    ];

    for target in &all_targets {
        if let Some(rec) = manifest.get(target) {
            let short_hash = &rec.sha256[..16];
            println!("{:<22} {:<16} {:<26} {}...",
                     target.display_name(),
                     rec.version,
                     rec.updated_at,
                     short_hash);
        } else {
            println!("{:<22} {}", target.display_name(), "(설치되지 않음)");
        }
    }
    println!("─────────────────────────────────────────────────────────────\n");

    Ok(())
}

pub fn rollback(
    target: &UpdateTarget,
    version: Option<&str>,
    base: &Path,
) -> anyhow::Result<()> {
    let mut manifest = Manifest::load(base)?;

    let rec = manifest.get(target)
        .ok_or_else(|| anyhow::anyhow!("{} 기록 없음", target.display_name()))?
        .clone();

    let rollback_version = version.unwrap_or_else(|| {
        rec.previous_versions.last().map(|s| s.as_str()).unwrap_or("none")
    });

    if rollback_version == "none" || rec.previous_versions.is_empty() {
        anyhow::bail!("롤백할 이전 버전이 없습니다");
    }

    tracing::info!(
        target = target.display_name(),
        from = rec.version,
        to = rollback_version,
        "롤백 시작"
    );

    let backup_dir = base.join("backups").join(format!("{:?}", target));
    let backup_path = backup_dir.join(format!("{}.tar.zst", rollback_version));

    if backup_path.exists() {
        let dest = target.data_path(base);
        std::fs::create_dir_all(&dest)?;

        let status = std::process::Command::new("tar")
            .args([
                "--use-compress-program=zstd -d",
                "-xf",
                backup_path.to_str().unwrap(),
                "-C",
                dest.to_str().unwrap(),
            ])
            .status()?;

        if status.success() {
            println!("✓ {} → {} 롤백 완료", target.display_name(), rollback_version);
        } else {
            anyhow::bail!("롤백 실패");
        }
    } else {
        tracing::warn!("백업 파일을 찾을 수 없습니다: {:?}", backup_path);
        println!("⚠ 백업 파일 없음 — 매니페스트만 업데이트");
    }

    // 매니페스트 업데이트
    if let Some(rec) = manifest.components.iter_mut()
        .find(|c| format!("{:?}", c.target) == format!("{:?}", target))
    {
        let old_ver = rec.version.clone();
        rec.version = rollback_version.to_string();
        rec.previous_versions.retain(|v| v != rollback_version);
        if !rec.previous_versions.contains(&old_ver) {
            rec.previous_versions.push(old_ver);
        }
    }
    manifest.save(base)?;

    Ok(())
}
