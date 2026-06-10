//! 데몬 런타임 설정

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket_path: PathBuf,
    pub db_path: PathBuf,
    pub log_path: PathBuf,
    pub max_concurrent_jobs: usize,
    pub job_timeout_secs: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from(aegis_common::ipc::AEGIS_SOCKET_PATH),
            db_path: PathBuf::from("/var/lib/aegis/aegisd.db"),
            log_path: PathBuf::from("/var/log/aegis/daemon.log"),
            max_concurrent_jobs: 4,
            job_timeout_secs: 3600 * 8, // 8시간
        }
    }
}
