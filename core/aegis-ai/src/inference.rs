//! 경량 선형 분류기 (.aegismodel) — musl/오프라인 환경용

use crate::features::FeatureVector;
use crate::result::{AiResult, MalwareClass};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct AegisModel {
    version: u32,
    input_dim: usize,
    num_classes: usize,
    class_names: Vec<String>,
    weights: Vec<Vec<f32>>,
    bias: Vec<f32>,
}

pub struct LinearModel {
    inner: AegisModel,
}

impl LinearModel {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let inner: AegisModel = serde_json::from_str(&text)?;
        if inner.weights.len() != inner.num_classes {
            anyhow::bail!("invalid model: weight rows mismatch");
        }
        Ok(Self { inner })
    }

    pub fn classify(&self, features: &FeatureVector) -> AiResult {
        let input = features.to_vec();
        if input.len() != self.inner.input_dim {
            tracing::warn!(
                got = input.len(),
                expected = self.inner.input_dim,
                "feature dim mismatch for .aegismodel"
            );
            return fallback_result(features);
        }

        let mut logits = vec![0.0f32; self.inner.num_classes];
        for (cls, row) in self.inner.weights.iter().enumerate() {
            let mut sum = self.inner.bias.get(cls).copied().unwrap_or(0.0);
            for (i, w) in row.iter().enumerate() {
                sum += w * input.get(i).copied().unwrap_or(0.0);
            }
            logits[cls] = sum;
        }

        let probs = softmax(&logits);
        let (best_idx, &best_prob) = probs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        let class = class_from_name(
            self.inner
                .class_names
                .get(best_idx)
                .map(String::as_str)
                .unwrap_or("Clean"),
        );

        AiResult {
            class: class.clone(),
            confidence: best_prob,
            probabilities: self
                .inner
                .class_names
                .iter()
                .zip(probs.iter())
                .map(|(n, p)| (n.clone(), *p))
                .collect(),
            key_features: vec![format!("Linear model prediction: {:?}", class)],
            model_version: format!("aegismodel-v{}", self.inner.version),
        }
    }
}

fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exps.iter().sum::<f32>().max(1e-6);
    exps.iter().map(|e| e / sum).collect()
}

fn fallback_result(features: &FeatureVector) -> AiResult {
    AiResult {
        class: MalwareClass::Clean,
        confidence: 0.0,
        probabilities: vec![],
        key_features: vec![format!(
            "Model input dim mismatch (got {} features)",
            features.to_vec().len()
        )],
        model_version: "aegismodel-fallback".into(),
    }
}

fn class_from_name(name: &str) -> MalwareClass {
    match name {
        "Ransomware" => MalwareClass::Ransomware,
        "Rootkit" => MalwareClass::Rootkit,
        "Trojan" => MalwareClass::Trojan,
        "Worm" => MalwareClass::Worm,
        "Backdoor" => MalwareClass::Backdoor,
        "Cryptominer" => MalwareClass::Cryptominer,
        _ => MalwareClass::Clean,
    }
}
