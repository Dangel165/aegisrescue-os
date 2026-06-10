//! 공유 앱 상태

use crate::db::Database;
use aegis_common::policy::{AegisPolicy, NetworkMode};
use aegis_common::types::{AegisJob, JobStatus, QuarantineEntry};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use uuid::Uuid;
use aegis_common::ipc::DaemonStream;

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
        let (event_tx, _) = broadcast::channel(1024);

        let network_mode = policy.network.default.clone();

        Ok(Self(Arc::new(Inner {
            policy,
            network_mode: RwLock::new(network_mode),
            jobs: RwLock::new(HashMap::new()),
            quarantine: RwLock::new(vec![]),
            db: Mutex::new(db),
            event_tx,
            start_time: std::time::Instant::now(),
            stats: RwLock::new(DaemonStats::default()),
        })))
    }

    pub async fn shutdown(&self) {
        tracing::info!("aegisd shutdown initiated");
        // 진행 중 작업 상태 저장
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

    pub async fn network_mode(&self) -> NetworkMode {
        self.0.network_mode.read().await.clone()
    }

    pub async fn set_network_mode(&self, mode: NetworkMode) {
        *self.0.network_mode.write().await = mode.clone();
        let mut net_policy = self.0.policy.network.clone();
        net_policy.default = mode;
        crate::network::apply_policy(&net_policy).await;
    }
}
