//! AI 분류 모듈 테스트

use aegis_ai::{AiClassifier, features::FeatureVector, result::MalwareClass};
use tempfile::NamedTempFile;
use std::io::Write;
use std::path::Path;

#[test]
fn test_clean_file_classification() {
    // 일반 텍스트 파일은 Clean 또는 낮은 신뢰도로 분류되어야 함
    let features = FeatureVector {
        size_normalized: 0.01,
        entropy: 0.5, // 낮은 엔트로피
        byte_histogram: vec![0.0; 256],
        section_count: 0.0,
        import_count: 0.0,
        has_wx_section: 0.0,
        packing_score: 0.0,
        suspicious_strings: 0.0,
        stripped: 0.0,
        unsigned: 0.0,
        network_imports: 0.0,
        process_imports: 0.0,
        file_imports: 0.0,
        crypto_imports: 0.0,
        registry_imports: 0.0,
    };

    let classifier = AiClassifier::new(Path::new("/nonexistent_model_dir"));
    let result = classifier.classify(&features);

    // 모든 지표가 0인 경우 Clean이어야 함
    assert!(!result.is_malicious() || result.confidence < 0.5,
        "Clean file features should not be classified as malicious with high confidence");
}

#[test]
fn test_ransomware_indicators() {
    // 높은 엔트로피 + 암호화 API + 의심 문자열 = 랜섬웨어
    let features = FeatureVector {
        size_normalized: 0.3,
        entropy: 0.92, // 매우 높은 엔트로피
        byte_histogram: vec![0.004; 256],
        section_count: 0.3,
        import_count: 0.5,
        has_wx_section: 1.0,
        packing_score: 1.0,
        suspicious_strings: 0.8,
        stripped: 1.0,
        unsigned: 1.0,
        network_imports: 0.2,
        process_imports: 0.5,
        file_imports: 0.8,  // 파일 암호화
        crypto_imports: 0.9, // 암호화 API 많이 사용
        registry_imports: 0.3,
    };

    let classifier = AiClassifier::new(Path::new("/nonexistent_model_dir"));
    let result = classifier.classify(&features);

    println!("Ransomware test: {:?} ({:.1}%)", result.class, result.confidence * 100.0);
    // 랜섬웨어 또는 악성으로 분류되어야 함
    assert!(result.is_malicious(), "High entropy + crypto APIs should be flagged");
}

#[test]
fn test_feature_vector_dim() {
    let fv = FeatureVector {
        size_normalized: 0.0,
        entropy: 0.5,
        byte_histogram: vec![0.0; 256],
        section_count: 0.0,
        import_count: 0.0,
        has_wx_section: 0.0,
        packing_score: 0.0,
        suspicious_strings: 0.0,
        stripped: 0.0,
        unsigned: 0.0,
        network_imports: 0.0,
        process_imports: 0.0,
        file_imports: 0.0,
        crypto_imports: 0.0,
        registry_imports: 0.0,
    };

    assert_eq!(fv.to_vec().len(), FeatureVector::FEATURE_DIM);
}
