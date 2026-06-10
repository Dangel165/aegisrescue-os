use std::path::Path;

pub struct VerifyResult {
    pub valid: bool,
    pub sha256: String,
}

pub fn verify_image(image: &Path, expected_hash: Option<&str>) -> anyhow::Result<VerifyResult> {
    let sha256 = aegis_common::hash::sha256_file(image)?;

    let valid = match expected_hash {
        Some(expected) => sha256.eq_ignore_ascii_case(expected),
        None => {
            // 동일 경로의 .sha256 파일 확인
            let hash_file = image.with_extension("sha256");
            if hash_file.exists() {
                let content = std::fs::read_to_string(&hash_file)?;
                let expected = content.split_whitespace().next().unwrap_or("");
                sha256.eq_ignore_ascii_case(expected)
            } else {
                true // 해시 파일 없으면 존재만 확인
            }
        }
    };

    Ok(VerifyResult { valid, sha256 })
}
