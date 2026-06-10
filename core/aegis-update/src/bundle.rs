//! 오프라인 업데이트 번들 생성 및 적용

use crate::{manifest::{ComponentRecord, Manifest}, verify, UpdateTarget};
use std::path::{Path, PathBuf};

/// 오프라인 번들 적용
pub fn apply_bundle(
    bundle_path: &Path,
    targets: &[UpdateTarget],
    data_dir: &Path,
    pub_key: &Path,
    dry_run: bool,
) -> anyhow::Result<()> {
    println!("📦 번들 적용: {:?}", bundle_path);

    // 서명 파일 확인
    let sig_path = bundle_path.with_extension("sig");
    if pub_key.exists() && sig_path.exists() {
        let valid = verify::verify_signature(bundle_path, &sig_path, pub_key)?;
        if !valid {
            anyhow::bail!("❌ 번들 서명 검증 실패 — 위조된 번들일 수 있습니다");
        }
        println!("✓ 서명 검증 완료");
    } else {
        tracing::warn!("서명 파일 또는 공개 키 없음 — 서명 검증 건너뜀");
    }

    // 임시 디렉터리에 압축 해제
    let tmp_dir = PathBuf::from("/tmp/aegis_update_bundle");
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir)?;
    }
    std::fs::create_dir_all(&tmp_dir)?;

    let status = std::process::Command::new("tar")
        .args([
            "--use-compress-program=zstd -d",
            "-xf",
            bundle_path.to_str().unwrap(),
            "-C",
            tmp_dir.to_str().unwrap(),
        ])
        .status()
        .or_else(|_| {
            // 폴백: zstd + tar
            let decompressed = tmp_dir.join("bundle.tar");
            std::process::Command::new("zstd")
                .args(["-d", bundle_path.to_str().unwrap(),
                       "-o", decompressed.to_str().unwrap()])
                .status()
                .and_then(|_| {
                    std::process::Command::new("tar")
                        .args(["-xf", decompressed.to_str().unwrap(),
                               "-C", tmp_dir.to_str().unwrap()])
                        .status()
                })
        })?;

    if !status.success() {
        anyhow::bail!("번들 압축 해제 실패");
    }

    // 매니페스트 파일 읽기
    let bundle_manifest_path = tmp_dir.join("manifest.json");
    let bundle_manifest: serde_json::Value = if bundle_manifest_path.exists() {
        let s = std::fs::read_to_string(&bundle_manifest_path)?;
        serde_json::from_str(&s)?
    } else {
        serde_json::json!({})
    };

    let mut manifest = Manifest::load(data_dir)?;
    let mut updated = vec![];

    for target in targets {
        let target_name = format!("{:?}", target).to_lowercase();
        let src = tmp_dir.join(&target_name);
        if !src.exists() {
            tracing::warn!("{} — 번들에 없음", target.display_name());
            continue;
        }

        let dst = target.data_path(data_dir);

        if dry_run {
            println!("  [dry-run] {} → {:?}", target.display_name(), dst);
            updated.push(target.display_name());
            continue;
        }

        // 백업 생성
        if dst.exists() {
            backup_component(target, &dst, data_dir)?;
        }

        std::fs::create_dir_all(&dst)?;

        // 파일 복사
        copy_dir_all(&src, &dst)?;

        // 매니페스트 갱신
        let version = bundle_manifest[&target_name]["version"]
            .as_str()
            .unwrap_or("bundle")
            .to_string();
        let sha256 = verify::compute_sha256(&src).unwrap_or_default();
        let size = dir_size(&src);

        manifest.upsert(ComponentRecord {
            target: target.clone(),
            version,
            updated_at: chrono::Utc::now().to_rfc3339(),
            sha256,
            size_bytes: size,
            previous_versions: vec![],
        });

        println!("  ✓ {} 업데이트 완료", target.display_name());
        updated.push(target.display_name());
    }

    if !dry_run {
        manifest.save(data_dir)?;
    }

    println!("\n✓ 총 {} 개 컴포넌트 {}",
             updated.len(),
             if dry_run { "확인됨 (dry-run)" } else { "업데이트됨" });

    // 임시 디렉터리 정리
    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok(())
}

/// 에어갭 환경으로 내보낼 번들 생성
pub fn create_bundle(
    output: &Path,
    targets: &[UpdateTarget],
    data_dir: &Path,
) -> anyhow::Result<()> {
    let tmp_dir = PathBuf::from("/tmp/aegis_bundle_stage");
    if tmp_dir.exists() { std::fs::remove_dir_all(&tmp_dir)?; }
    std::fs::create_dir_all(&tmp_dir)?;

    let manifest = Manifest::load(data_dir)?;
    let mut bundle_manifest = serde_json::Map::new();

    for target in targets {
        let src = target.data_path(data_dir);
        if !src.exists() {
            tracing::warn!("{} 소스 없음 — 건너뜀", target.display_name());
            continue;
        }

        let target_name = format!("{:?}", target).to_lowercase();
        let dst = tmp_dir.join(&target_name);
        copy_dir_all(&src, &dst)?;

        let version = manifest.get(target)
            .map(|r| r.version.as_str())
            .unwrap_or("unknown")
            .to_string();
        bundle_manifest.insert(target_name, serde_json::json!({ "version": version }));
    }

    // 매니페스트 저장
    std::fs::write(
        tmp_dir.join("manifest.json"),
        serde_json::to_string_pretty(&bundle_manifest)?,
    )?;

    // tar.zst 생성
    let output_path = if output.extension().is_none() {
        output.with_extension("tar.zst")
    } else {
        output.to_path_buf()
    };

    let status = std::process::Command::new("tar")
        .args([
            "--use-compress-program=zstd -T0",
            "-cf", output_path.to_str().unwrap(),
            "-C", tmp_dir.to_str().unwrap(),
            ".",
        ])
        .status()?;

    if !status.success() {
        anyhow::bail!("번들 생성 실패");
    }

    let hash = verify::compute_sha256(&output_path)?;
    println!("✓ 번들 생성 완료: {:?}", output_path);
    println!("  SHA256: {}", hash);
    println!("  에어갭 환경에 번들을 전달한 후:");
    println!("  aegis-update apply --bundle {:?}", output_path);

    std::fs::remove_dir_all(&tmp_dir)?;
    Ok(())
}

fn backup_component(target: &UpdateTarget, src: &Path, base: &Path) -> anyhow::Result<()> {
    let manifest = Manifest::load(base)?;
    let version = manifest.get(target)
        .map(|r| r.version.clone())
        .unwrap_or_else(|| "unknown".into());

    let backup_dir = base.join("backups").join(format!("{:?}", target));
    std::fs::create_dir_all(&backup_dir)?;
    let backup_path = backup_dir.join(format!("{}.tar.zst", version));

    let _ = std::process::Command::new("tar")
        .args([
            "--use-compress-program=zstd",
            "-cf", backup_path.to_str().unwrap(),
            "-C", src.parent().unwrap().to_str().unwrap(),
            src.file_name().unwrap().to_str().unwrap(),
        ])
        .status();

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn dir_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }
    let Ok(entries) = std::fs::read_dir(path) else { return 0 };
    entries.flatten().map(|e| dir_size(&e.path())).sum()
}
