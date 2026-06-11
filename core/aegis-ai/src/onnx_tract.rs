//! ONNX Runtime via tract (optional, glibc/dev builds)

use crate::features::FeatureVector;
use crate::result::{AiResult, MalwareClass};
use std::path::Path;
use tract_onnx::prelude::*;

pub fn classify(model_path: &Path, features: &FeatureVector) -> anyhow::Result<AiResult> {
    let input = features.to_vec();
    let model = tract_onnx::onnx()
        .model_for_path(model_path)?
        .into_optimized()?
        .into_runnable()?;

    let input_tensor: Tensor =
        tract_ndarray::Array2::from_shape_vec((1, input.len()), input)?.into();
    let outputs = model.run(tvec!(input_tensor.into()))?;
    let output = outputs[0].to_array_view::<f32>()?;
    let probs: Vec<f32> = output.iter().copied().collect();

    let (best_idx, &best_prob) = probs
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap_or((6, &0.0));

    let class = match best_idx {
        0 => MalwareClass::Ransomware,
        1 => MalwareClass::Rootkit,
        2 => MalwareClass::Trojan,
        3 => MalwareClass::Worm,
        4 => MalwareClass::Backdoor,
        5 => MalwareClass::Cryptominer,
        _ => MalwareClass::Clean,
    };

    let names = [
        "Ransomware",
        "Rootkit",
        "Trojan",
        "Worm",
        "Backdoor",
        "Cryptominer",
        "Clean",
    ];

    Ok(AiResult {
        class,
        confidence: best_prob,
        probabilities: names
            .iter()
            .zip(probs.iter())
            .map(|(&n, &p)| (n.to_string(), p))
            .collect(),
        key_features: vec![format!("ONNX/tract prediction: {:?}", class)],
        model_version: format!("onnx-tract:{}", model_path.display()),
    })
}
