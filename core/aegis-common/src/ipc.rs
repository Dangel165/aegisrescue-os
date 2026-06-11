//! aegisd ↔ CLI/GUI IPC 프로토콜 (Unix 소켓 + JSON)

use crate::types::{AegisEvent, AegisJob, JobType, QuarantineEntry, ThreatInfo};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub const AEGIS_SOCKET_PATH: &str = "/run/aegis/aegisd.sock";

/// 클라이언트 → 데몬 요청
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum DaemonRequest {
    /// 새 작업 제출
    SubmitJob { job_type: JobType },
    /// 실시간 이벤트 스트림 구독 (응답 후 DaemonStream JSON 라인 전송)
    SubscribeEvents,
    /// 작업 상태 조회
    GetJob { job_id: Uuid },
    /// 작업 목록 조회
    ListJobs,
    /// 작업 취소
    CancelJob { job_id: Uuid },
    /// 디스크 목록 조회
    ListDisks,
    /// 격리 목록 조회
    ListQuarantine,
    /// 격리 해제 (복원)
    RestoreQuarantine { entry_id: Uuid, restore_path: PathBuf },
    /// 격리 항목 삭제
    DeleteQuarantine { entry_id: Uuid },
    /// 이벤트 로그 조회
    GetEvents { since: Option<chrono::DateTime<chrono::Utc>>, limit: Option<u32> },
    /// 네트워크 모드 변경
    SetNetworkMode { mode: String },
    /// 시스템 상태 조회
    GetStatus,
    /// 종료
    Shutdown,
}

/// 데몬 → 클라이언트 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    Ok,
    Subscribed,
    JobSubmitted { job_id: Uuid },
    Job(AegisJob),
    Jobs(Vec<AegisJob>),
    Disks(Vec<crate::types::DiskInfo>),
    Quarantine(Vec<QuarantineEntry>),
    Events(Vec<AegisEvent>),
    Status(SystemStatus),
    Error { code: String, message: String },
}

/// 데몬 → 클라이언트 실시간 이벤트 스트림
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stream", rename_all = "snake_case")]
pub enum DaemonStream {
    JobProgress { job_id: Uuid, progress: crate::types::JobProgress },
    ThreatFound { threat: ThreatInfo },
    Event(AegisEvent),
}

/// 시스템 전체 상태
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub version: String,
    pub uptime_secs: u64,
    pub active_jobs: usize,
    pub queued_jobs: usize,
    pub network_mode: String,
    pub secure_boot_active: bool,
    pub dm_verity_active: bool,
    pub quarantine_count: usize,
    pub total_threats_detected: u64,
}
