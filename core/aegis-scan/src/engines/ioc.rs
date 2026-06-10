//! IOC (Indicators of Compromise) 평판 엔진

use aegis_common::types::{DetectionEngine, DetectionResult};
use std::collections::HashSet;
use std::path::Path;

pub struct IocEngine {
    malicious_hashes: HashSet<String>,
}

impl IocEngine {
    pub fn new(signatures_path: &Path) -> Self {
        let malicious_hashes = load_ioc_hashes(signatures_path);
        tracing::info!(count = malicious_hashes.len(), "IOC engine loaded");
        Self { malicious_hashes }
    }

    pub fn check(&self, sha256: &str) -> Option<DetectionResult> {
        if self.malicious_hashes.contains(&sha256.to_lowercase()) {
            Some(DetectionResult {
                engine: DetectionEngine::Ioc,
                signature: Some(format!("IOC:{}", &sha256[..16])),
                confidence: 1.0,
                details: Some("Hash matches known malicious IOC database".into()),
            })
        } else {
            None
        }
    }
}

impl super::Engine for IocEngine {
    fn name(&self) -> &'static str {
        "ioc"
    }

    fn scan_bytes(&self, data: &[u8]) -> super::EngineResult {
        let hash = aegis_common::hash::sha256_bytes(data);
        self.check(&hash)
    }
}

fn load_ioc_hashes(path: &Path) -> HashSet<String> {
    // IOC DB는 여러 파일에서 로드 (STIX/MISP 변환된 단순 해시 목록)
    let mut hashes = HashSet::new();

    let files = [
        "ioc_sha256.txt",
        "ransomware_hashes.txt",
        "rootkit_hashes.txt",
        "trojan_hashes.txt",
    ];

    for filename in &files {
        let filepath = path.join(filename);
        if let Ok(content) = std::fs::read_to_string(&filepath) {
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    // "hash [optional_description]" 형식 지원
                    let hash = line.split_whitespace().next().unwrap_or(line);
                    hashes.insert(hash.to_lowercase());
                }
            }
        }
    }

    hashes
}
