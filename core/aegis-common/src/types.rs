use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

// ─── 위협 탐지 결과 ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatVerdict {
    /// 악성코드 확정 (score >= 0.85)
    Malicious,
    /// 의심 — 추가 분석 권고 (0.50 <= score < 0.85)
    Suspicious,
    /// 정상 (score < 0.50)
    Clean,
    /// 분석 불가 (암호화, 접근 거부 등)
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatInfo {
    pub id: Uuid,
    pub path: PathBuf,
    pub verdict: ThreatVerdict,
    /// 0.0 ~ 1.0 복합 위험 점수
    pub risk_score: f64,
    pub malware_family: Option<String>,
    pub malware_type: Option<MalwareType>,
    pub detections: Vec<DetectionResult>,
    pub hash_sha256: String,
    pub size_bytes: u64,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MalwareType {
    Ransomware,
    Trojan,
    Rootkit,
    Bootkit,
    Worm,
    Adware,
    Spyware,
    Cryptominer,
    Backdoor,
    Exploit,
    PotentiallyUnwanted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub engine: DetectionEngine,
    pub signature: Option<String>,
    pub confidence: f64,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectionEngine {
    Signature,    // ClamAV 시그니처
    Heuristic,    // 정적 휴리스틱
    Yara,         // YARA 룰
    Ai,           // AI 분류기
    Ioc,          // IOC/해시 평판
    Behavioral,   // 동적 행위 분석
}

// ─── 디스크 / 파티션 정보 ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub device: PathBuf,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub size_bytes: u64,
    pub sector_size: u32,
    pub partition_table: PartitionTableType,
    pub partitions: Vec<PartitionInfo>,
    pub health: DiskHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PartitionTableType {
    Mbr,
    Gpt,
    None,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub device: PathBuf,
    pub number: u32,
    pub start_sector: u64,
    pub size_bytes: u64,
    pub filesystem: FilesystemType,
    pub label: Option<String>,
    pub uuid: Option<String>,
    pub flags: Vec<PartitionFlag>,
    pub mount_point: Option<PathBuf>,
    pub is_encrypted: bool,
    pub encryption_type: Option<EncryptionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilesystemType {
    // Windows
    Ntfs, ExFat, Fat32, Fat16, ReFs,
    // Linux
    Ext2, Ext3, Ext4, Btrfs, Xfs, F2fs, Jfs, ReiserFs,
    // macOS
    Apfs, HfsPlus,
    // 공통
    Swap, Efi, Unknown, Unformatted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PartitionFlag {
    Boot, Esp, Msftdata, Msftres, BitlockerEncrypted, LinuxHome, LinuxSwap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EncryptionType {
    Luks1, Luks2, BitLocker, FileVault2, Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskHealth {
    pub smart_available: bool,
    pub overall_health: HealthStatus,
    pub reallocated_sectors: u64,
    pub pending_sectors: u64,
    pub uncorrectable_errors: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Good, Caution, Bad, Unknown,
}

// ─── 마운트 정보 ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInfo {
    pub id: Uuid,
    pub source: PathBuf,
    pub target: PathBuf,
    pub filesystem: FilesystemType,
    pub options: MountOptions,
    pub mounted_at: DateTime<Utc>,
    pub operator: String,
    pub reason: Option<String>,
    pub pre_mount_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountOptions {
    pub read_only: bool,
    pub noexec: bool,
    pub nodev: bool,
    pub nosuid: bool,
    /// COW 스냅샷 레이어를 통한 쓰기 (원본 불변)
    pub use_snapshot: bool,
}

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            read_only: true,
            noexec: true,
            nodev: true,
            nosuid: true,
            use_snapshot: false,
        }
    }
}

// ─── 작업 / 이벤트 ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AegisJob {
    pub id: Uuid,
    pub job_type: JobType,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress: Option<JobProgress>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobType {
    QuickScan { target: PathBuf },
    FullScan { target: PathBuf },
    RansomwareRecovery { target: PathBuf, output: PathBuf },
    BootRepair { device: PathBuf },
    DiskImage { source: PathBuf, dest: PathBuf },
    ForensicCapture { target: PathBuf, output: PathBuf },
    Backup { source: PathBuf, dest: PathBuf },
    MountDisk { device: PathBuf },
    SandboxAnalysis { sample: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Queued, Running, Paused, Completed, Failed, Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub current: u64,
    pub total: u64,
    pub message: String,
    pub speed_bps: Option<u64>,
    pub eta_secs: Option<u64>,
}

impl JobProgress {
    pub fn percent(&self) -> f64 {
        if self.total == 0 { return 0.0; }
        (self.current as f64 / self.total as f64 * 100.0).min(100.0)
    }
}

// ─── 격리 항목 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineEntry {
    pub id: Uuid,
    pub original_path: PathBuf,
    pub quarantine_path: PathBuf,
    pub threat: ThreatInfo,
    pub quarantined_at: DateTime<Utc>,
    pub can_restore: bool,
}

// ─── 시스템 이벤트 ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AegisEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub severity: EventSeverity,
    pub message: String,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    SystemStartup, SystemShutdown,
    DiskMounted, DiskUnmounted,
    ScanStarted, ScanCompleted, ThreatDetected,
    RecoveryStarted, RecoveryCompleted,
    NetworkPolicyChanged,
    QuarantineAdded, QuarantineRestored,
    PolicyViolation, UserAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    Debug, Info, Warning, Error, Critical,
}
