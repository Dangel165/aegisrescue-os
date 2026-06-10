//! 파일/스트림 해시 유틸리티
//!
//! SHA-256 / SHA-512 / BLAKE3 계산 + 체인 오브 커스터디 지원

use crate::error::Result;
use sha2::{Digest, Sha256, Sha512};
use std::io::{BufReader, Read};
use std::path::Path;

const BUFFER_SIZE: usize = 1024 * 1024; // 1 MiB

/// SHA-256 해시를 hex 문자열로 반환
pub fn sha256_file(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; BUFFER_SIZE];

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// SHA-512 해시를 hex 문자열로 반환
pub fn sha512_file(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut hasher = Sha512::new();
    let mut buf = vec![0u8; BUFFER_SIZE];

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// BLAKE3 해시를 hex 문자열로 반환 (고속)
pub fn blake3_file(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; BUFFER_SIZE];

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// 바이트 슬라이스 SHA-256
pub fn sha256_bytes(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// 바이트 슬라이스 BLAKE3
pub fn blake3_bytes(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// 포렌식용 파일 해시 세트 (SHA-256 + BLAKE3 동시 계산)
pub struct ForensicHashes {
    pub sha256: String,
    pub blake3: String,
    pub size_bytes: u64,
}

impl ForensicHashes {
    pub fn compute(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let metadata = file.metadata()?;
        let size_bytes = metadata.len();

        let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
        let mut sha256_hasher = Sha256::new();
        let mut blake3_hasher = blake3::Hasher::new();
        let mut buf = vec![0u8; BUFFER_SIZE];

        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 { break; }
            sha256_hasher.update(&buf[..n]);
            blake3_hasher.update(&buf[..n]);
        }

        Ok(Self {
            sha256: hex::encode(sha256_hasher.finalize()),
            blake3: blake3_hasher.finalize().to_hex().to_string(),
            size_bytes,
        })
    }

    pub fn matches_sha256(&self, expected: &str) -> bool {
        self.sha256.eq_ignore_ascii_case(expected)
    }
}

/// 파일 엔트로피 계산 (악성코드 패킹 탐지에 활용)
/// 반환: 0.0 (완전 균일) ~ 8.0 (최대 무작위)
pub fn file_entropy(path: &Path) -> Result<f64> {
    let data = std::fs::read(path)?;
    Ok(bytes_entropy(&data))
}

pub fn bytes_entropy(data: &[u8]) -> f64 {
    if data.is_empty() { return 0.0; }

    let mut freq = [0u64; 256];
    for &b in data {
        freq[b as usize] += 1;
    }

    let len = data.len() as f64;
    freq.iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_empty() {
        assert_eq!(bytes_entropy(b""), 0.0);
    }

    #[test]
    fn entropy_uniform() {
        let data: Vec<u8> = (0..=255u8).collect();
        let e = bytes_entropy(&data);
        assert!((e - 8.0).abs() < 0.01);
    }

    #[test]
    fn entropy_constant() {
        let data = vec![0xAA_u8; 1024];
        assert_eq!(bytes_entropy(&data), 0.0);
    }

    #[test]
    fn sha256_bytes_known() {
        let hash = sha256_bytes(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
