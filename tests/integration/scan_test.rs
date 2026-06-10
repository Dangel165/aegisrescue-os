//! aegis-scan 통합 테스트

use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

/// EICAR 테스트 파일 (안전한 테스트용)
const EICAR: &[u8] = b"X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*";

#[test]
fn test_scan_clean_file() {
    let dir = TempDir::new().unwrap();
    let clean_file = dir.path().join("clean.txt");
    std::fs::write(&clean_file, b"Hello, World! This is a clean file.").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--bin", "aegis-scan", "--",
               "--quick", clean_file.to_str().unwrap()])
        .output()
        .unwrap();

    // 종료 코드 0 = 위협 없음
    assert_eq!(output.status.code(), Some(0),
        "Clean file should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_scan_high_entropy_file() {
    let dir = TempDir::new().unwrap();
    let random_file = dir.path().join("random.bin");

    // 높은 엔트로피 파일 (랜덤 바이트)
    use std::collections::HashSet;
    let mut data = vec![0u8; 4096];
    for (i, b) in data.iter_mut().enumerate() {
        *b = (i % 256) as u8;
    }
    data.extend(data.clone()); // 반복으로 8KB
    std::fs::write(&random_file, &data).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--bin", "aegis-scan", "--",
               "--full", random_file.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // 파일은 처리되어야 함 (오류 없이)
    assert!(!stdout.contains("ERROR"), "Should process without errors");
}

#[test]
fn test_scan_json_output() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, b"safe content").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--bin", "aegis-scan", "--",
               "--json", "--quick", file.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // JSON 파싱 가능해야 함
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "JSON output should be valid: {}", stdout);
}
