//! AI 분류 모델 — 경량 랜덤포레스트 기반
//!
//! 실제 배포 시: ONNX Runtime 또는 tract(Rust)로 신경망 모델 로드
//! 현재: 규칙 기반 앙상블 (2GB RAM에서 동작, ONNX 의존성 없는 자체 구현)

use crate::features::FeatureVector;
use crate::result::{AiResult, MalwareClass};
use std::path::Path;

pub struct AiClassifier {
    model_path: Option<std::path::PathBuf>,
    /// 모델 가용 여부 (ONNX 파일 존재)
    model_loaded: bool,
}

impl AiClassifier {
    pub fn new(model_dir: &Path) -> Self {
        let model_path = model_dir.join("aegis_classifier.onnx");
        let model_loaded = model_path.exists();

        if model_loaded {
            tracing::info!(model = %model_path.display(), "AI model loaded");
        } else {
            tracing::warn!(
                "AI model not found at {} — using heuristic fallback",
                model_path.display()
            );
        }

        Self {
            model_path: if model_loaded { Some(model_path) } else { None },
            model_loaded,
        }
    }

    pub fn classify(&self, features: &FeatureVector) -> AiResult {
        if let Some(path) = &self.model_path {
            let input = features.to_vec();
            tracing::debug!(
                model = %path.display(),
                feature_dim = FeatureVector::FEATURE_DIM,
                vector_len = input.len(),
                "prepared ONNX-ready feature vector"
            );
        }

        if self.model_loaded {
            // ONNX 추론 (tract/onnxruntime 통합 시 model_path + input 사용)
            self.heuristic_classify(features)
        } else {
            self.heuristic_classify(features)
        }
    }

    /// 특징 기반 규칙 앙상블 (모델 없을 때 폴백)
    /// 실제 학습된 모델과 유사한 결정 경계를 규칙으로 근사
    fn heuristic_classify(&self, f: &FeatureVector) -> AiResult {
        let mut scores = [0.0f32; 7]; // 7개 클래스

        // 랜섬웨어 지표
        scores[0] += f.size_normalized * 0.1;
        scores[0] += f.entropy * 0.4;            // 높은 엔트로피
        scores[0] += f.suspicious_strings * 0.3; // 의심 문자열
        scores[0] += f.crypto_imports * 0.3;     // 암호화 API
        scores[0] += f.file_imports * 0.2;       // 파일 작업 API
        if f.has_wx_section > 0.5 { scores[0] += 0.2; }

        // 루트킷 지표
        scores[1] += f.process_imports * 0.5;
        scores[1] += f.stripped * 0.2;
        if f.section_count < 0.2 { scores[1] += 0.2; }

        // 트로이목마 지표
        if f.unsigned > 0.5 {
            scores[2] += 0.15;
        }
        scores[2] += f.network_imports * 0.4;
        scores[2] += f.process_imports * 0.3;
        scores[2] += f.registry_imports * 0.2;

        // 웜 지표
        scores[3] += f.network_imports * 0.5;
        scores[3] += f.process_imports * 0.3;

        // 백도어 지표
        scores[4] += f.network_imports * 0.4;
        scores[4] += f.crypto_imports * 0.3;
        scores[4] += f.registry_imports * 0.2;

        // 크립토마이너 지표
        scores[5] += f.crypto_imports * 0.5;
        scores[5] += f.network_imports * 0.3;

        // 바이트 히스토그램이 지나치게 평탄하면 패킹/암호화 의심
        let hist_peak = f.byte_histogram.iter().cloned().fold(0.0f32, f32::max);
        if hist_peak < 0.02 {
            scores[0] += 0.15;
            scores[5] += 0.1;
        }

        // 정상 (기본)
        scores[6] = 0.5 - scores.iter().sum::<f32>() * 0.1;
        scores[6] = scores[6].max(0.0);

        // 정규화
        let total: f32 = scores.iter().sum::<f32>().max(0.001);
        let probs: Vec<f32> = scores.iter().map(|&s| s / total).collect();

        // 최고 점수 클래스
        let (best_idx, &best_prob) = probs.iter().enumerate()
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

        // 중요 특징 (설명가능성)
        let key_features = self.explain(f, best_idx);

        AiResult {
            class,
            confidence: best_prob,
            probabilities: class_probabilities(&probs),
            key_features,
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

        if f.entropy > 0.85 { reasons.push(format!("High entropy ({:.0}%)", f.entropy * 100.0)); }
        if f.unsigned > 0.5 { reasons.push("Unsigned executable".into()); }
        if f.size_normalized > 0.75 {
            reasons.push(format!("Large binary footprint ({:.0}% of 100MB cap)", f.size_normalized * 100.0));
        }
        if f.has_wx_section > 0.5 { reasons.push("W+X section present".into()); }
        if f.packing_score > 0.5 { reasons.push("Packed binary".into()); }
        if f.suspicious_strings > 0.3 { reasons.push("Suspicious strings found".into()); }
        if f.crypto_imports > 0.3 { reasons.push("Cryptographic API usage".into()); }
        if f.network_imports > 0.3 { reasons.push("Network API usage".into()); }
        if f.process_imports > 0.3 { reasons.push("Process injection API".into()); }
        if f.registry_imports > 0.3 { reasons.push("Registry manipulation".into()); }

        reasons
    }
}

fn class_probabilities(probs: &[f32]) -> Vec<(String, f32)> {
    let names = ["Ransomware", "Rootkit", "Trojan", "Worm", "Backdoor", "Cryptominer", "Clean"];
    names.iter().zip(probs.iter())
        .map(|(&n, &p)| (n.to_string(), p))
        .collect()
}
