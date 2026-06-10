//! 보안 정책 정의 및 파싱
//!
//! `/etc/aegis/policy.toml` 을 읽어 시스템 전체 보안 정책으로 사용

use crate::error::{AegisError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AegisPolicy {
    pub mount: MountPolicy,
    pub network: NetworkPolicy,
    pub scan: ScanPolicy,
    pub forensic: ForensicPolicy,
    pub integrity: IntegrityPolicy,
    pub quarantine: QuarantinePolicy,
}

impl Default for AegisPolicy {
    fn default() -> Self {
        Self {
            mount: MountPolicy::default(),
            network: NetworkPolicy::default(),
            scan: ScanPolicy::default(),
            forensic: ForensicPolicy::default(),
            integrity: IntegrityPolicy::default(),
            quarantine: QuarantinePolicy::default(),
        }
    }
}

impl AegisPolicy {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AegisError::Config(format!("Cannot read policy file {}: {}", path.display(), e)))?;
        toml::from_str(&content)
            .map_err(|e| AegisError::Config(format!("Invalid policy TOML: {}", e)))
    }

    pub fn load_or_default(path: &Path) -> Self {
        Self::load(path).unwrap_or_else(|_| {
            tracing::warn!("Failed to load policy from {}, using defaults", path.display());
            Self::default()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPolicy {
    /// 기본 마운트 모드 ("ro" | "rw")
    pub default_mode: MountMode,
    /// RO 외 옵션 목록
    pub default_options: Vec<String>,
    /// RW 마운트 전 사용자 확인 필요
    pub require_confirmation_for_rw: bool,
    /// 쓰기 시 자동 스냅샷 생성
    pub auto_snapshot_on_write: bool,
    /// 감염 의심 디스크 자동 RO 강제
    pub force_ro_on_suspicious: bool,
}

impl Default for MountPolicy {
    fn default() -> Self {
        Self {
            default_mode: MountMode::ReadOnly,
            default_options: vec![
                "noexec".into(), "nodev".into(), "nosuid".into(),
            ],
            require_confirmation_for_rw: true,
            auto_snapshot_on_write: true,
            force_ro_on_suspicious: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MountMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// "airgap" | "whitelist" | "restricted" | "full"
    pub default: NetworkMode,
    /// 업데이트 서버 허용 여부 (whitelist 모드)
    pub allow_update_whitelist: bool,
    /// 허용 호스트 목록 (whitelist 모드)
    pub allowed_hosts: Vec<String>,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            default: NetworkMode::AirGap,
            allow_update_whitelist: false,
            allowed_hosts: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    /// 완전 차단
    AirGap,
    /// 지정 호스트만 허용
    Whitelist,
    /// 아웃바운드만 허용
    Restricted,
    /// 전체 허용 (경고)
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanPolicy {
    pub engines: Vec<String>,
    /// 악성 판정 임계값
    pub malicious_threshold: f64,
    /// 의심 판정 임계값
    pub suspicious_threshold: f64,
    /// 스캔 제외 경로
    pub exclude_paths: Vec<PathBuf>,
    /// 최대 파일 크기 (MB, 0 = 무제한)
    pub max_file_size_mb: u64,
}

impl Default for ScanPolicy {
    fn default() -> Self {
        Self {
            engines: vec![
                "signature".into(), "heuristic".into(),
                "yara".into(), "ai".into(), "ioc".into(),
            ],
            malicious_threshold: 0.85,
            suspicious_threshold: 0.50,
            exclude_paths: vec![],
            max_file_size_mb: 512,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicPolicy {
    /// 소프트웨어 쓰기 차단기 강제 활성
    pub write_blocker: bool,
    pub hash_algorithm: HashAlgorithm,
    /// 체인 오브 커스터디 로깅
    pub chain_of_custody: bool,
    /// 증거 저장 경로
    pub evidence_path: PathBuf,
}

impl Default for ForensicPolicy {
    fn default() -> Self {
        Self {
            write_blocker: true,
            hash_algorithm: HashAlgorithm::Sha256,
            chain_of_custody: true,
            evidence_path: PathBuf::from("/mnt/evidence"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    Sha256, Sha512, Blake3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityPolicy {
    /// Secure Boot 미활성 시 경고 (but 부팅 허용)
    pub require_secure_boot: bool,
    /// 루트 FS dm-verity 검증
    pub verify_root_dmverity: bool,
    /// 서명된 커널 모듈만 허용
    pub module_signature_required: bool,
}

impl Default for IntegrityPolicy {
    fn default() -> Self {
        Self {
            require_secure_boot: false,
            verify_root_dmverity: true,
            module_signature_required: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantinePolicy {
    /// 격리 저장소 경로
    pub quarantine_path: PathBuf,
    /// 격리 항목 암호화
    pub encrypt_quarantine: bool,
    /// 격리 보관 최대 일수 (0 = 무제한)
    pub retention_days: u32,
}

impl Default for QuarantinePolicy {
    fn default() -> Self {
        Self {
            quarantine_path: PathBuf::from("/var/lib/aegis/quarantine"),
            encrypt_quarantine: true,
            retention_days: 30,
        }
    }
}
