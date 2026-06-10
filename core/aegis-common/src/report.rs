//! 포렌식/복구 리포트 생성

use crate::types::{QuarantineEntry, ThreatInfo};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AegisReport {
    pub id: Uuid,
    pub title: String,
    pub report_type: ReportType,
    pub generated_at: DateTime<Utc>,
    pub operator: String,
    pub tool_version: String,
    /// SHA-256 of the analyzed evidence
    pub evidence_hash: Option<String>,
    pub summary: ReportSummary,
    pub sections: Vec<ReportSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportType {
    MalwareScan,
    ForensicAnalysis,
    RansomwareRecovery,
    BootRepair,
    DiskImage,
    Backup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_files_scanned: u64,
    pub threats_found: u64,
    pub files_cleaned: u64,
    pub files_quarantined: u64,
    pub files_recovered: u64,
    pub risk_level: RiskLevel,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Critical, High, Medium, Low, Clean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub content: SectionContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SectionContent {
    ThreatList { threats: Vec<ThreatInfo> },
    QuarantineList { entries: Vec<QuarantineEntry> },
    Timeline { events: Vec<TimelineEvent> },
    HashVerification { entries: Vec<HashEntry> },
    Text { body: String },
    KeyValue { pairs: Vec<(String, String)> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub description: String,
    pub artifact_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEntry {
    pub path: PathBuf,
    pub algorithm: String,
    pub hash: String,
    pub verified: bool,
}

impl AegisReport {
    pub fn new(report_type: ReportType, title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            report_type,
            generated_at: Utc::now(),
            operator: whoami(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            evidence_hash: None,
            summary: ReportSummary {
                total_files_scanned: 0,
                threats_found: 0,
                files_cleaned: 0,
                files_quarantined: 0,
                files_recovered: 0,
                risk_level: RiskLevel::Clean,
                recommendations: vec![],
            },
            sections: vec![],
        }
    }

    /// JSON으로 직렬화
    pub fn to_json(&self) -> crate::error::Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| crate::error::AegisError::Other(e.into()))
    }
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".into())
}
