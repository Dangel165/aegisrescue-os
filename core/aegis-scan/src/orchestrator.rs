//! 스캔 오케스트레이터 — 멀티엔진 스캔 + 스코어 융합

use crate::engines::heuristic::HeuristicEngine;
use crate::engines::ioc::IocEngine;
use crate::engines::signature::SignatureEngine;
use crate::engines::yara::YaraEngine;
use crate::pe_analyzer;
use crate::persistence::{PersistenceChecker, PersistenceItem};
use crate::quarantine_client;
use aegis_common::types::{
    DetectionEngine, DetectionResult, MalwareType, ThreatInfo, ThreatVerdict,
};
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub target: PathBuf,
    pub mode: ScanMode,
    pub yara_rules_path: PathBuf,
    pub signatures_path: PathBuf,
    pub threads: usize,
    pub auto_quarantine: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScanMode {
    Quick,    // 자동 실행 지점, 최근 수정 파일
    Standard, // 모든 실행 파일
    Full,     // 모든 파일 심층 스캔
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResults {
    pub scan_id: Uuid,
    pub target: PathBuf,
    pub started_at: chrono::DateTime<Utc>,
    pub completed_at: chrono::DateTime<Utc>,
    pub total_files: u64,
    pub scanned_files: u64,
    pub malicious_count: u64,
    pub suspicious_count: u64,
    pub clean_count: u64,
    pub skipped_count: u64,
    pub threats: Vec<ThreatInfo>,
}

pub struct Scanner {
    config: ScanConfig,
}

impl Scanner {
    pub fn new(config: ScanConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> anyhow::Result<ScanResults> {
        let started_at = Utc::now();
        let scan_id = Uuid::new_v4();

        info!(scan_id = %scan_id, target = %self.config.target.display(), "scan started");
        configure_threads(self.config.threads);

        // 엔진 초기화
        let sig_engine = SignatureEngine::new(&self.config.signatures_path);
        let heur_engine = HeuristicEngine::new();
        let yara_engine = YaraEngine::new(&self.config.yara_rules_path);
        let ioc_engine = IocEngine::new(&self.config.signatures_path);

        // 대상 파일 수집
        let files = self.collect_files();
        let total = files.len() as u64;

        info!(total_files = total, "files collected");

        // 진행률 표시
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:50.cyan/blue} {pos}/{len} {msg}")
                .unwrap()
        );

        let mut threats = vec![];
        let mut scanned = 0u64;
        let mut skipped = 0u64;

        for path in &files {
            pb.set_message(path.file_name().unwrap_or_default().to_string_lossy().to_string());

            match self.scan_file(path, &sig_engine, &heur_engine, &yara_engine, &ioc_engine).await {
                Ok(Some(threat)) => {
                    pb.println(format!("⚠ THREAT: {} [{:?}]", path.display(), threat.verdict));
                    if self.config.auto_quarantine
                        && matches!(threat.verdict, ThreatVerdict::Malicious | ThreatVerdict::Suspicious)
                    {
                        if let Err(e) = quarantine_client::request_quarantine(&threat.path) {
                            warn!(path = %threat.path.display(), error = %e, "quarantine request failed");
                        }
                    }
                    threats.push(threat);
                    scanned += 1;
                }
                Ok(None) => scanned += 1,
                Err(e) => {
                    tracing::debug!("Skip {}: {}", path.display(), e);
                    skipped += 1;
                }
            }

            pb.inc(1);
        }

        pb.finish_with_message("Scan complete");

        // 영속성 메커니즘 추가 점검 (Full/Standard 모드)
        if self.config.mode != ScanMode::Quick {
            let persistence_findings = PersistenceChecker::check(&self.config.target);
            if !persistence_findings.is_empty() {
                info!(count = persistence_findings.len(), "persistence mechanisms found");
                for finding in persistence_findings {
                    let threat = persistence_to_threat(&finding);
                    if self.config.auto_quarantine
                        && threat.verdict == ThreatVerdict::Malicious
                    {
                        if let Err(e) = quarantine_client::request_quarantine(&threat.path) {
                            warn!(path = %threat.path.display(), error = %e, "persistence quarantine request failed");
                        }
                    }
                    threats.push(threat);
                }
            }
        }

        let malicious_count = threats.iter()
            .filter(|t| t.verdict == ThreatVerdict::Malicious).count() as u64;
        let suspicious_count = threats.iter()
            .filter(|t| t.verdict == ThreatVerdict::Suspicious).count() as u64;

        let results = ScanResults {
            scan_id,
            target: self.config.target.clone(),
            started_at,
            completed_at: Utc::now(),
            total_files: total,
            scanned_files: scanned,
            malicious_count,
            suspicious_count,
            clean_count: scanned.saturating_sub(malicious_count + suspicious_count),
            skipped_count: skipped,
            threats,
        };

        info!(
            malicious = malicious_count,
            suspicious = suspicious_count,
            "scan completed"
        );

        Ok(results)
    }

    async fn scan_file(
        &self,
        path: &Path,
        sig: &SignatureEngine,
        heur: &HeuristicEngine,
        yara: &YaraEngine,
        ioc: &IocEngine,
    ) -> anyhow::Result<Option<ThreatInfo>> {
        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() { return Ok(None); }

        let size = metadata.len();
        let max_size = 512 * 1024 * 1024u64; // 512MB 제한
        if size > max_size {
            return Ok(None);
        }

        let hash = aegis_common::hash::sha256_file(path)?;
        let data = std::fs::read(path)?;

        let mut detections = vec![];

        // 엔진별 실행
        if let Some(r) = sig.scan(&data, &hash) { detections.push(r); }
        if let Some(r) = heur.scan(path, &data)  { detections.push(r); }
        if let Some(r) = yara.scan(path, &data)  { detections.push(r); }
        if let Some(r) = ioc.check(&hash)         { detections.push(r); }
        if let Some(r) = analyze_binary(path)     { detections.push(r); }

        if detections.is_empty() {
            return Ok(None);
        }

        // 가중 스코어 계산
        let risk_score = compute_risk_score(&detections);
        let verdict = score_to_verdict(risk_score);

        if verdict == ThreatVerdict::Clean { return Ok(None); }

        let malware_type = infer_malware_type(&detections);

        Ok(Some(ThreatInfo {
            id: Uuid::new_v4(),
            path: path.to_path_buf(),
            verdict,
            risk_score,
            malware_family: detections.first().and_then(|d| d.signature.clone()),
            malware_type: Some(malware_type),
            detections,
            hash_sha256: hash,
            size_bytes: size,
            detected_at: Utc::now(),
        }))
    }

    fn collect_files(&self) -> Vec<PathBuf> {
        let target = &self.config.target;

        if target.is_file() {
            return vec![target.clone()];
        }

        let walker = WalkDir::new(target)
            .follow_links(false)
            .max_depth(if self.config.mode == ScanMode::Quick { 3 } else { usize::MAX });

        walker
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| self.should_scan(e.path()))
            .map(|e| e.into_path())
            .collect()
    }

    fn should_scan(&self, path: &Path) -> bool {
        match self.config.mode {
            ScanMode::Quick => {
                // 실행 파일 + 스크립트만
                let ext = aegis_common::utils::file_ext(path).unwrap_or_default();
                matches!(ext.as_str(),
                    "exe" | "dll" | "sys" | "bat" | "cmd" | "ps1" | "vbs" |
                    "js" | "hta" | "lnk" | "scr" | "com" | "pif" |
                    "sh" | "py" | "elf" | "so"
                )
            }
            ScanMode::Standard => {
                // 실행 파일 + 문서 + 스크립트
                let ext = aegis_common::utils::file_ext(path).unwrap_or_default();
                !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "mp4" | "mkv" | "mp3")
            }
            ScanMode::Full => true,
        }
    }
}

fn configure_threads(threads: usize) {
    if threads > 0 {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global();
        info!(threads = threads, "scanner thread pool configured");
    }
}

fn analyze_binary(path: &Path) -> Option<DetectionResult> {
    let info = pe_analyzer::analyze(path)?;

    let suspicious_imports = info.imports.iter().filter(|name| {
        let lower = name.to_ascii_lowercase();
        lower.contains("crypt")
            || lower.contains("virtualalloc")
            || lower.contains("writeprocessmemory")
            || lower.contains("createremotethread")
            || lower.contains("internet")
            || lower.contains("socket")
    }).count();

    let suspicious_strings = info.strings.iter().filter(|s| {
        let lower = s.to_ascii_lowercase();
        lower.contains("vssadmin")
            || lower.contains("shadowcopy")
            || lower.contains("bitcoin")
            || lower.contains("ransom")
            || lower.contains("powershell")
            || lower.contains("cmd.exe")
    }).count();

    let confidence = ((if info.is_packed { 0.35 } else { 0.0 })
        + (suspicious_imports as f64 * 0.08)
        + (suspicious_strings as f64 * 0.08)
        + if info.entropy > 7.4 { 0.25 } else { 0.0 })
        .min(0.85);

    if confidence < 0.35 {
        return None;
    }

    Some(DetectionResult {
        engine: DetectionEngine::Heuristic,
        signature: Some(format!("Static {:?} analysis", info.format)),
        confidence,
        details: Some(format!(
            "arch={}, packed={}, entropy={:.2}, imports={}, exports={}, suspicious_imports={}, suspicious_strings={}",
            info.architecture,
            info.is_packed,
            info.entropy,
            info.imports.len(),
            info.exports.len(),
            suspicious_imports,
            suspicious_strings,
        )),
    })
}

fn persistence_to_threat(item: &PersistenceItem) -> ThreatInfo {
    let risk_score = match item.severity {
        "CRITICAL" => 0.95,
        "HIGH" => 0.85,
        "MEDIUM" => 0.60,
        _ => 0.40,
    };
    let verdict = score_to_verdict(risk_score);

    ThreatInfo {
        id: Uuid::new_v4(),
        path: PathBuf::from(&item.location),
        verdict,
        risk_score,
        malware_family: Some(format!("{} persistence", item.os)),
        malware_type: Some(MalwareType::PotentiallyUnwanted),
        detections: vec![DetectionResult {
            engine: DetectionEngine::Behavioral,
            signature: Some(item.mechanism.clone()),
            confidence: risk_score,
            details: Some(format!("{} persistence: {}", item.severity, item.entry)),
        }],
        hash_sha256: String::new(),
        size_bytes: 0,
        detected_at: Utc::now(),
    }
}

/// 가중 위험 스코어 계산
fn compute_risk_score(detections: &[DetectionResult]) -> f64 {
    let weights = |engine: &DetectionEngine| -> f64 {
        match engine {
            DetectionEngine::Signature  => 0.35,
            DetectionEngine::Ioc        => 0.30,
            DetectionEngine::Yara       => 0.20,
            DetectionEngine::Heuristic  => 0.10,
            DetectionEngine::Ai         => 0.15,
            DetectionEngine::Behavioral => 0.25,
        }
    };

    let total_weight: f64 = detections.iter().map(|d| weights(&d.engine)).sum();
    if total_weight == 0.0 { return 0.0; }

    let weighted_sum: f64 = detections.iter()
        .map(|d| d.confidence * weights(&d.engine))
        .sum();

    (weighted_sum / total_weight).min(1.0)
}

fn score_to_verdict(score: f64) -> ThreatVerdict {
    if score >= 0.85 { ThreatVerdict::Malicious }
    else if score >= 0.50 { ThreatVerdict::Suspicious }
    else { ThreatVerdict::Clean }
}

fn infer_malware_type(detections: &[DetectionResult]) -> MalwareType {
    for d in detections {
        if let Some(sig) = &d.signature {
            let s = sig.to_lowercase();
            if s.contains("ransom") { return MalwareType::Ransomware; }
            if s.contains("rootkit") { return MalwareType::Rootkit; }
            if s.contains("trojan") { return MalwareType::Trojan; }
            if s.contains("worm") { return MalwareType::Worm; }
            if s.contains("backdoor") { return MalwareType::Backdoor; }
            if s.contains("miner") || s.contains("crypto") { return MalwareType::Cryptominer; }
        }
    }
    MalwareType::Unknown
}
