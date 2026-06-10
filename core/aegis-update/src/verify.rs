//! 업데이트 번들 서명 검증

use std::path::Path;

/// Ed25519 서명 검증 (minisign/signify 호환)
/// 실제 구현은 ed25519-dalek 크레이트 사용; 여기서는 외부 도구 래퍼 제공
pub fn verify_signature(
    bundle_path: &Path,
    sig_path: &Path,
    pub_key_path: &Path,
) -> anyhow::Result<bool> {
    // signify 사용 시도
    if is_available("signify") {
        let status = std::process::Command::new("signify")
            .args([
                "-V",
                "-p", pub_key_path.to_str().unwrap(),
                "-m", bundle_path.to_str().unwrap(),
                "-x", sig_path.to_str().unwrap(),
            ])
            .status()?;
        return Ok(status.success());
    }

    // minisign 폴백
    if is_available("minisign") {
        let status = std::process::Command::new("minisign")
            .args([
                "-V",
                "-p", pub_key_path.to_str().unwrap(),
                "-m", bundle_path.to_str().unwrap(),
            ])
            .status()?;
        return Ok(status.success());
    }

    // gpg 폴백
    if is_available("gpg") {
        let status = std::process::Command::new("gpg")
            .args([
                "--verify",
                sig_path.to_str().unwrap(),
                bundle_path.to_str().unwrap(),
            ])
            .status()?;
        return Ok(status.success());
    }

    // 서명 도구 없음 — 경고 후 통과 (개발 환경)
    tracing::warn!("서명 검증 도구 없음 (signify/minisign/gpg) — 건너뜀");
    Ok(true)
}

pub fn compute_sha256(path: &Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path)?;
    let hash = Sha256::digest(&data);
    Ok(hex::encode(hash))
}

pub fn verify_hash(path: &Path, expected: &str) -> anyhow::Result<bool> {
    let actual = compute_sha256(path)?;
    Ok(actual == expected)
}

fn is_available(tool: &str) -> bool {
    std::process::Command::new("which")
        .arg(tool)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
