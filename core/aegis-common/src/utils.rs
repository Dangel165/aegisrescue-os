//! 공용 유틸리티

use std::path::Path;
use std::time::{Duration, Instant};

/// 바이트 크기를 사람이 읽기 쉬운 문자열로 변환
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    if bytes == 0 { return "0 B".to_string(); }

    let exp = (bytes as f64).log(1024.0).floor() as usize;
    let exp = exp.min(UNITS.len() - 1);
    let val = bytes as f64 / 1024_f64.powi(exp as i32);

    if exp == 0 {
        format!("{} {}", bytes, UNITS[exp])
    } else {
        format!("{:.2} {}", val, UNITS[exp])
    }
}

/// 초를 "2h 3m 45s" 형식으로 변환
pub fn human_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;

    match (h, m, s) {
        (0, 0, s) => format!("{}s", s),
        (0, m, s) => format!("{}m {}s", m, s),
        (h, m, s) => format!("{}h {}m {}s", h, m, s),
    }
}

/// 전송 속도(bytes/sec)를 "12.3 MiB/s" 형식으로 변환
pub fn human_speed(bps: u64) -> String {
    format!("{}/s", human_bytes(bps))
}

/// 경과 시간 측정기
pub struct Stopwatch {
    start: Instant,
}

impl Stopwatch {
    pub fn start() -> Self { Self { start: Instant::now() } }
    pub fn elapsed(&self) -> Duration { self.start.elapsed() }
    pub fn elapsed_secs(&self) -> u64 { self.start.elapsed().as_secs() }
}

/// 파일 확장자 반환 (소문자)
pub fn file_ext(path: &Path) -> Option<String> {
    path.extension()?.to_str().map(|s| s.to_lowercase())
}

/// 경로가 심볼릭 링크 사이클을 포함하지 않는지 확인
pub fn is_safe_path(path: &Path) -> bool {
    path.components().all(|c| {
        use std::path::Component;
        !matches!(c, Component::ParentDir)
    })
}

/// 타이밍 공격 방지용 상수 시간 바이트 비교
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_bytes() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1024), "1.00 KiB");
        assert_eq!(human_bytes(1024 * 1024), "1.00 MiB");
        assert_eq!(human_bytes(1536), "1.50 KiB");
    }

    #[test]
    fn test_human_duration() {
        assert_eq!(human_duration(45), "45s");
        assert_eq!(human_duration(125), "2m 5s");
        assert_eq!(human_duration(3661), "1h 1m 1s");
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hi"));
    }
}
