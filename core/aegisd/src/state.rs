//! 공유 앱 상태

use crate::db::Database;
use crate::quarantine::QuarantineManager;
use aegis_common::ipc::DaemonStream;
use aegis_common::policy::{AegisPolicy, NetworkMode};
use aegis_common::types::{
    AegisEvent, AegisJob, EventSeverity, EventType, JobProgress, JobStatus, QuarantineEntry,
    ThreatInfo,
};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

struct Inner {
    pub policy: AegisPolicy,
    pub network_mode: RwLock<NetworkMode>,
    pub jobs: RwLock<HashMap<Uuid, AegisJob>>,
    pub quarantine: RwLock<Vec<QuarantineEntry>>,
    pub db: Mutex<Database>,
    pub event_tx: broadcast::Sender<DaemonStream>,
    pub start_time: std::time::Instant,
    pub stats: RwLock<DaemonStats>,
}

#[derive(Default)]
pub struct DaemonStats {
    pub total_threats_detected: u64,
}

impl AppState {
    pub async fn new(policy: AegisPolicy, db_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let db = Database::open(db_path).await?;
        let quarantine_entries = db.load_quarantine().unwrap_or_default();
        let (event_tx, _) = broadcast::channel(1024);

        let network_mode = policy.network.default.clone();

        Ok(Self(Arc::new(Inner {
            policy,
            network_mode: RwLock::new(network_mode),
            jobs: RwLock::new(HashMap::new()),
            quarantine: RwLock::new(quarantine_entries),
            db: Mutex::new(db),
            event_tx,
            start_time: std::time::Instant::now(),
            stats: RwLock::new(DaemonStats::default()),
        })))
    }

    pub async fn shutdown(&self) {
        tracing::info!("aegisd shutdown initiated");
        let mut jobs = self.0.jobs.write().await;
        for job in jobs.values_mut() {
            if job.status == JobStatus::Running {
                job.status = JobStatus::Failed;
                job.error = Some("Daemon shutdown".to_string());
            }
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<DaemonStream> {
        self.0.event_tx.subscribe()
    }

    pub fn broadcast(&self, event: DaemonStream) {
        let _ = self.0.event_tx.send(event);
    }

    pub async fn emit_event(
        &self,
        event_type: EventType,
        severity: EventSeverity,
        message: impl Into<String>,
        context: serde_json::Value,
    ) {
        let event = AegisEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            severity,
            message: message.into(),
            context,
        };
        let db = self.0.db.lock().await;
        let _ = db.log_aegis_event(&event);
        self.broadcast(DaemonStream::Event(event));
    }

    pub async fn emit_threat(&self, threat: &ThreatInfo) {
        self.increment_threats().await;
        self.broadcast(DaemonStream::ThreatFound {
            threat: threat.clone(),
        });
        self.emit_event(
            EventType::ThreatDetected,
            EventSeverity::Critical,
            format!("Threat detected: {}", threat.path.display()),
            serde_json::json!({
                "path": threat.path,
                "risk_score": threat.risk_score,
                "verdict": threat.verdict,
            }),
        )
        .await;
    }

    pub async fn set_job_progress(&self, job_id: Uuid, progress: JobProgress) {
        if let Some(mut job) = self.get_job(job_id).await {
            job.progress = Some(progress.clone());
            self.update_job(job).await;
            self.broadcast(DaemonStream::JobProgress { job_id, progress });
        }
    }

    pub async fn add_job(&self, job: AegisJob) {
        let id = job.id;
        self.0.jobs.write().await.insert(id, job);
    }

    pub async fn get_job(&self, id: Uuid) -> Option<AegisJob> {
        self.0.jobs.read().await.get(&id).cloned()
    }

    pub async fn list_jobs(&self) -> Vec<AegisJob> {
        self.0.jobs.read().await.values().cloned().collect()
    }

    pub async fn update_job(&self, job: AegisJob) {
        self.0.jobs.write().await.insert(job.id, job);
    }

    pub async fn is_job_cancelled(&self, id: Uuid) -> bool {
        self.get_job(id)
            .await
            .map(|j| j.status == JobStatus::Cancelled)
            .unwrap_or(true)
    }

    pub fn uptime_secs(&self) -> u64 {
        self.0.start_time.elapsed().as_secs()
    }

    pub async fn total_threats(&self) -> u64 {
        self.0.stats.read().await.total_threats_detected
    }

    pub async fn increment_threats(&self) {
        self.0.stats.write().await.total_threats_detected += 1;
    }

    pub fn policy(&self) -> &AegisPolicy {
        &self.0.policy
    }

    pub fn quarantine_base(&self) -> PathBuf {
        self.0.policy.quarantine.quarantine_path.clone()
    }

    pub fn quarantine_manager(&self) -> QuarantineManager {
        QuarantineManager::new(self.quarantine_base())
    }

    pub async fn quarantine_count(&self) -> usize {
        self.0.quarantine.read().await.len()
    }

    pub async fn list_quarantine(&self) -> Vec<QuarantineEntry> {
        self.0.quarantine.read().await.clone()
    }

    pub async fn get_quarantine(&self, id: Uuid) -> Option<QuarantineEntry> {
        self.0
            .quarantine
            .read()
            .await
            .iter()
            .find(|e| e.id == id)
            .cloned()
    }

    pub async fn add_quarantine_entry(&self, entry: QuarantineEntry) -> anyhow::Result<()> {
        {
            let db = self.0.db.lock().await;
            db.insert_quarantine(&entry)?;
        }
        self.0.quarantine.write().await.push(entry);
        Ok(())
    }

    pub async fn remove_quarantine_entry(&self, id: Uuid) -> anyhow::Result<Option<QuarantineEntry>> {
        let mut list = self.0.quarantine.write().await;
        if let Some(pos) = list.iter().position(|e| e.id == id) {
            let entry = list.remove(pos);
            {
                let db = self.0.db.lock().await;
                db.delete_quarantine(&id.to_string())?;
            }
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    pub async fn list_events(
        &self,
        since: Option<chrono::DateTime<Utc>>,
        limit: Option<u32>,
    ) -> anyhow::Result<Vec<AegisEvent>> {
        let since_str = since.map(|t| t.to_rfc3339());
        let limit = limit.unwrap_or(100);
        let db = self.0.db.lock().await;
        db.list_events(since_str.as_deref(), limit)
    }

    pub async fn network_mode(&self) -> NetworkMode {
        self.0.network_mode.read().await.clone()
    }

    pub async fn set_network_mode(&self, mode: NetworkMode) {
        *self.0.network_mode.write().await = mode.clone();
        let mut net_policy = self.0.policy.network.clone();
        net_policy.default = mode.clone();
        crate::network::apply_policy(&net_policy).await;
        self.emit_event(
            EventType::NetworkPolicyChanged,
            EventSeverity::Info,
            format!("Network mode changed to {:?}", mode),
            serde_json::json!({ "mode": format!("{:?}", mode) }),
        )
        .await;
    }
}
