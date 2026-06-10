//! 시그니처 기반 탐지 엔진 (ClamAV 통합 + 자체 해시 DB)

use aegis_common::types::{DetectionEngine, DetectionResult};
use std::collections::HashSet;
use std::path::Path;

pub struct SignatureEngine {
    /// MD5/SHA-256 해시 악성 DB
    known_hashes: HashSet<String>,
    /// ClamAV 사용 가능 여부
    clamav_available: bool,
}

impl SignatureEngine {
    pub fn new(signatures_path: &Path) -> Self {
        let known_hashes = load_hash_db(signatures_path);
        let clamav_available = which_clamdscan();

        tracing::info!(
            hashes = known_hashes.len(),
            clamav = clamav_available,
            "signature engine initialized"
        );

        Self { known_hashes, clamav_available }
    }

    pub fn scan(&self, data: &[u8], sha256: &str) -> Option<DetectionResult> {
        // 1) 자체 해시 DB 조회
        if self.known_hashes.contains(&sha256.to_lowercase()) {
            return Some(DetectionResult {
                engine: DetectionEngine::Signature,
                signature: Some(format!("Hash-DB:{}", &sha256[..16])),
                confidence: 1.0,
                details: Some("Known malicious hash".into()),
            });
        }

        // 2) ClamAV 시그니처 확인
        if self.clamav_available {
            if let Some(r) = self.scan_with_clamav(data) {
                return Some(r);
            }
        }

        None
    }

    fn scan_with_clamav(&self, data: &[u8]) -> Option<DetectionResult> {
        let tmp_path = format!("/tmp/aegis_clam_{}", std::process::id());
        std::fs::write(&tmp_path, data).ok()?;

        let output = std::process::Command::new("clamdscan")
            .args(["--no-summary", "--infected", &tmp_path])
            .output()
            .ok()?;

        let _ = std::fs::remove_file(&tmp_path);

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.code() == Some(1) {
            // ClamAV 발견: "<path>: <SIGNATURE> FOUND"
            let signature = stdout.lines()
                .find(|l| l.contains("FOUND"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().trim_end_matches(" FOUND").to_string())
                .unwrap_or_else(|| "Unknown.ClamAV".into());

            Some(DetectionResult {
                engine: DetectionEngine::Signature,
                signature: Some(signature),
                confidence: 0.95,
                details: Some("ClamAV detection".into()),
            })
        } else {
            None
        }
    }
}

impl super::Engine for SignatureEngine {
    fn name(&self) -> &'static str {
        "signature"
    }

    fn scan_bytes(&self, data: &[u8]) -> super::EngineResult {
        let hash = aegis_common::hash::sha256_bytes(data);
        self.scan(data, &hash)
    }
}

fn load_hash_db(path: &Path) -> HashSet<String> {
    let hash_file = path.join("malicious_hashes.txt");
    std::fs::read_to_string(&hash_file)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.trim().to_lowercase().to_string())
        .collect()
}

fn which_clamdscan() -> bool {
    std::process::Command::new("clamdscan")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
