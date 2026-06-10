//! 해시 유틸리티 통합 테스트

use aegis_common::hash::*;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_sha256_empty() {
    assert_eq!(
        sha256_bytes(b""),
        "e3b0c44298fc1c149afbf4c8996fb924\
         27ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_sha256_file() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"hello").unwrap();
    let hash = sha256_file(f.path()).unwrap();
    assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
}

#[test]
fn test_forensic_hashes() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"AegisRescue test").unwrap();

    let hashes = ForensicHashes::compute(f.path()).unwrap();
    assert!(!hashes.sha256.is_empty());
    assert!(!hashes.blake3.is_empty());
    assert_eq!(hashes.size_bytes, 16);
}

#[test]
fn test_entropy_range() {
    let zeros = vec![0u8; 1024];
    assert_eq!(bytes_entropy(&zeros), 0.0);

    let all_bytes: Vec<u8> = (0..=255u8).collect::<Vec<_>>()
        .into_iter().cycle().take(1024).collect();
    let e = bytes_entropy(&all_bytes);
    assert!(e > 7.9, "High entropy bytes should have entropy ~8: {}", e);
}
