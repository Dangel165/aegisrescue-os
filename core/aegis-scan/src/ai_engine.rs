//! AI 엔진 — aegis-ai 라이브러리 연동

use aegis_ai::{AiClassifier, AiResult};
use aegis_common::types::{DetectionEngine, DetectionResult, ThreatVerdict};
use std::path::Path;
use std::sync::OnceLock;

static CLASSIFIER: OnceLock<AiClassifier> = OnceLock::new();

fn classifier() -> &'static AiClassifier {
    CLASSIFIER.get_or_init(|| {
        AiClassifier::new(Path::new("/usr/share/aegis/models"))
    })
}

pub fn scan_file(path: &Path) -> Option<DetectionResult> {
    let features = aegis_ai::features::FeatureVector::extract(path).ok()?;
    let result = classifier().classify(&features);
    ai_result_to_detection(path, &result)
}

fn ai_result_to_detection(path: &Path, result: &AiResult) -> Option<DetectionResult> {
    if !result.is_malicious() {
        return None;
    }

    let score = result.confidence as f64;
    Some(DetectionResult {
        engine: DetectionEngine::Ai,
        signature: Some(format!("AI:{}", result.class)),
        confidence: score,
        details: Some(result.key_features.join("; ")),
    })
}

pub fn verdict_from_ai(result: &AiResult) -> ThreatVerdict {
    if result.confidence >= 0.85 {
        ThreatVerdict::Malicious
    } else if result.is_malicious() {
        ThreatVerdict::Suspicious
    } else {
        ThreatVerdict::Clean
    }
}
