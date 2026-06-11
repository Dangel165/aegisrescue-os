//! AI 분류 모델 — .aegismodel / ONNX(tract) / heuristic 폴백

use crate::features::FeatureVector;
use crate::inference::LinearModel;
use crate::result::{AiResult, MalwareClass};
use std::path::Path;

enum Backend {
    Heuristic,
    Linear(LinearModel),
    #[cfg(feature = "onnx")]
    Onnx(PathBuf),
}

pub struct AiClassifier {
    backend: Backend,
}

impl AiClassifier {
    pub fn new(model_dir: &Path) -> Self {
        let aegismodel = model_dir.join("aegis_classifier.aegismodel");
        let onnx_path = model_dir.join("aegis_classifier.onnx");

        if let Ok(linear) = LinearModel::load(&aegismodel) {
            tracing::info!(model = %aegismodel.display(), "loaded .aegismodel linear classifier");
            return Self {
                backend: Backend::Linear(linear),
            };
        }

        #[cfg(feature = "onnx")]
        if onnx_path.exists() {
            tracing::info!(model = %onnx_path.display(), "loaded ONNX model (tract backend)");
            return Self {
                backend: Backend::Onnx(onnx_path),
            };
        }

        if !onnx_path.exists() && !aegismodel.exists() {
            tracing::warn!(
                dir = %model_dir.display(),
                "no aegis_classifier.aegismodel/onnx — using heuristic fallback"
            );
        }

        Self {
            backend: Backend::Heuristic,
        }
    }

    pub fn classify(&self, features: &FeatureVector) -> AiResult {
        match &self.backend {
            Backend::Linear(model) => model.classify(features),
            #[cfg(feature = "onnx")]
            Backend::Onnx(path) => match crate::onnx_tract::classify(path, features) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, "ONNX inference failed, falling back to heuristic");
                    self.heuristic_classify(features)
                }
            },
            Backend::Heuristic => self.heuristic_classify(features),
        }
    }

    fn heuristic_classify(&self, f: &FeatureVector) -> AiResult {
        let mut scores = [0.0f32; 7];

        scores[0] += f.size_normalized * 0.1;
        scores[0] += f.entropy * 0.4;
        scores[0] += f.suspicious_strings * 0.3;
        scores[0] += f.crypto_imports * 0.3;
        scores[0] += f.file_imports * 0.2;
        if f.has_wx_section > 0.5 {
            scores[0] += 0.2;
        }

        scores[1] += f.process_imports * 0.5;
        scores[1] += f.stripped * 0.2;
        if f.section_count < 0.2 {
            scores[1] += 0.2;
        }

        if f.unsigned > 0.5 {
            scores[2] += 0.15;
        }
        scores[2] += f.network_imports * 0.4;
        scores[2] += f.process_imports * 0.3;
        scores[2] += f.registry_imports * 0.2;

        scores[3] += f.network_imports * 0.5;
        scores[3] += f.process_imports * 0.3;

        scores[4] += f.network_imports * 0.4;
        scores[4] += f.crypto_imports * 0.3;
        scores[4] += f.registry_imports * 0.2;

        scores[5] += f.crypto_imports * 0.5;
        scores[5] += f.network_imports * 0.3;

        let hist_peak = f.byte_histogram.iter().cloned().fold(0.0f32, f32::max);
        if hist_peak < 0.02 {
            scores[0] += 0.15;
            scores[5] += 0.1;
        }

        scores[6] = 0.5 - scores.iter().sum::<f32>() * 0.1;
        scores[6] = scores[6].max(0.0);

        let total: f32 = scores.iter().sum::<f32>().max(0.001);
        let probs: Vec<f32> = scores.iter().map(|&s| s / total).collect();

        let (best_idx, &best_prob) = probs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        let class = match best_idx {
            0 => MalwareClass::Ransomware,
            1 => MalwareClass::Rootkit,
            2 => MalwareClass::Trojan,
            3 => MalwareClass::Worm,
            4 => MalwareClass::Backdoor,
            5 => MalwareClass::Cryptominer,
            _ => MalwareClass::Clean,
        };

        AiResult {
            class,
            confidence: best_prob,
            probabilities: class_probabilities(&probs),
            key_features: self.explain(f, best_idx),
            model_version: "heuristic-v1".into(),
        }
    }

    fn explain(&self, f: &FeatureVector, class_idx: usize) -> Vec<String> {
        let mut reasons = vec![];

        let class_hint = match class_idx {
            0 => "Primary class: Ransomware",
            1 => "Primary class: Rootkit",
            2 => "Primary class: Trojan",
            3 => "Primary class: Worm",
            4 => "Primary class: Backdoor",
            5 => "Primary class: Cryptominer",
            _ => "Primary class: Clean",
        };
        reasons.push(class_hint.into());

        if f.entropy > 0.85 {
            reasons.push(format!("High entropy ({:.0}%)", f.entropy * 100.0));
        }
        if f.unsigned > 0.5 {
            reasons.push("Unsigned executable".into());
        }
        if f.size_normalized > 0.75 {
            reasons.push(format!(
                "Large binary footprint ({:.0}% of 100MB cap)",
                f.size_normalized * 100.0
            ));
        }
        if f.has_wx_section > 0.5 {
            reasons.push("W+X section present".into());
        }
        if f.packing_score > 0.5 {
            reasons.push("Packed binary".into());
        }
        if f.suspicious_strings > 0.3 {
            reasons.push("Suspicious strings found".into());
        }
        if f.crypto_imports > 0.3 {
            reasons.push("Cryptographic API usage".into());
        }
        if f.network_imports > 0.3 {
            reasons.push("Network API usage".into());
        }
        if f.process_imports > 0.3 {
            reasons.push("Process injection API".into());
        }
        if f.registry_imports > 0.3 {
            reasons.push("Registry manipulation".into());
        }

        reasons
    }
}

fn class_probabilities(probs: &[f32]) -> Vec<(String, f32)> {
    let names = [
        "Ransomware",
        "Rootkit",
        "Trojan",
        "Worm",
        "Backdoor",
        "Cryptominer",
        "Clean",
    ];
    names
        .iter()
        .zip(probs.iter())
        .map(|(&n, &p)| (n.to_string(), p))
        .collect()
}
