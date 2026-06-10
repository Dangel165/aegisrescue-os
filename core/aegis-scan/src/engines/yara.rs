//! YARA 엔진 — yara-x(Rust) 또는 yara CLI 래퍼

use aegis_common::types::{DetectionEngine, DetectionResult};
use std::path::Path;
use tracing::warn;

pub struct YaraEngine {
    rules_path: std::path::PathBuf,
    available: bool,
}

impl YaraEngine {
    pub fn new(rules_path: &Path) -> Self {
        let available = rules_path.exists() && which_yara();
        if !available {
            warn!("YARA engine not available (yara binary missing or rules not found)");
        } else {
            let rule_count = count_yara_rules(rules_path);
            tracing::info!(rules = rule_count, "YARA engine initialized");
        }

        Self {
            rules_path: rules_path.to_path_buf(),
            available,
        }
    }

    pub fn scan(&self, path: &Path, _data: &[u8]) -> Option<DetectionResult> {
        if !self.available { return None; }

        let rules_arg = self.rules_path.to_string_lossy().into_owned();
        let path_arg = path.to_string_lossy().into_owned();
        let output = std::process::Command::new("yara")
            .args([
                "--recursive",
                "--no-warnings",
                rules_arg.as_str(),
                path_arg.as_str(),
            ])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let matches: Vec<&str> = stdout.lines()
            .filter(|l| !l.is_empty())
            .collect();

        if matches.is_empty() {
            return None;
        }

        let signatures: Vec<String> = matches.iter()
            .map(|l| {
                // "RuleName /path/to/file" 형식에서 룰 이름 추출
                l.split_whitespace().next().unwrap_or("Unknown").to_string()
            })
            .collect();

        let confidence = (0.3 + (signatures.len() as f64 * 0.15)).min(0.9);

        Some(DetectionResult {
            engine: DetectionEngine::Yara,
            signature: Some(signatures.join(", ")),
            confidence,
            details: Some(format!("{} YARA rule(s) matched", signatures.len())),
        })
    }
}

impl super::Engine for YaraEngine {
    fn name(&self) -> &'static str {
        "yara"
    }

    fn scan_bytes(&self, data: &[u8]) -> super::EngineResult {
        let tmp = std::env::temp_dir().join(format!("aegis_yara_{}", std::process::id()));
        if std::fs::write(&tmp, data).is_err() {
            return None;
        }
        let result = self.scan(&tmp, data);
        let _ = std::fs::remove_file(&tmp);
        result
    }
}

fn which_yara() -> bool {
    std::process::Command::new("yara")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn count_yara_rules(path: &Path) -> usize {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "yar" || s == "yara")
                .unwrap_or(false)
        })
        .count()
}
