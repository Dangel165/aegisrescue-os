//! Unix 소켓 IPC 서버

use crate::state::AppState;
use aegis_common::ipc::{DaemonRequest, DaemonResponse, SystemStatus};
use aegis_common::types::{AegisJob, JobStatus, JobType};
use chrono::Utc;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};
use uuid::Uuid;

pub async fn run(state: AppState, socket_path: &Path) -> anyhow::Result<()> {
    // 기존 소켓 파일 제거
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let listener = UnixListener::bind(socket_path)?;
    info!("IPC server listening on {}", socket_path.display());

    // 소켓 퍼미션: 660 (root:aegis 그룹만 접근)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o660))?;
    }

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, state).await {
                        warn!("Client handler error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}

async fn handle_client(stream: UnixStream, state: AppState) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 { break; } // 연결 종료

        let request: DaemonRequest = match serde_json::from_str(line.trim()) {
            Ok(r) => r,
            Err(e) => {
                let resp = DaemonResponse::Error {
                    code: "PARSE_ERROR".into(),
                    message: e.to_string(),
                };
                let mut out = serde_json::to_string(&resp)?;
                out.push('\n');
                writer.write_all(out.as_bytes()).await?;
                continue;
            }
        };

        let response = process_request(request, &state).await;
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        writer.write_all(out.as_bytes()).await?;
    }

    Ok(())
}

async fn process_request(req: DaemonRequest, state: &AppState) -> DaemonResponse {
    match req {
        DaemonRequest::SubmitJob { job_type } => {
            let job = AegisJob {
                id: Uuid::new_v4(),
                job_type,
                status: JobStatus::Queued,
                created_at: Utc::now(),
                started_at: None,
                completed_at: None,
                progress: None,
                error: None,
            };
            let id = job.id;
            state.add_job(job).await;
            info!(job_id = %id, "job submitted");
            DaemonResponse::JobSubmitted { job_id: id }
        }

        DaemonRequest::GetJob { job_id } => {
            match state.get_job(job_id).await {
                Some(job) => DaemonResponse::Job(job),
                None => DaemonResponse::Error {
                    code: "NOT_FOUND".into(),
                    message: format!("Job {} not found", job_id),
                },
            }
        }

        DaemonRequest::ListJobs => {
            DaemonResponse::Jobs(state.list_jobs().await)
        }

        DaemonRequest::CancelJob { job_id } => {
            if let Some(mut job) = state.get_job(job_id).await {
                if job.status == JobStatus::Running || job.status == JobStatus::Queued {
                    job.status = JobStatus::Cancelled;
                    job.completed_at = Some(Utc::now());
                    state.update_job(job).await;
                    DaemonResponse::Ok
                } else {
                    DaemonResponse::Error {
                        code: "INVALID_STATE".into(),
                        message: format!("Job {} cannot be cancelled in state {:?}", job_id, JobStatus::Cancelled),
                    }
                }
            } else {
                DaemonResponse::Error {
                    code: "NOT_FOUND".into(),
                    message: format!("Job {} not found", job_id),
                }
            }
        }

        DaemonRequest::GetStatus => {
            DaemonResponse::Status(SystemStatus {
                version: env!("CARGO_PKG_VERSION").to_string(),
                uptime_secs: state.uptime_secs(),
                active_jobs: state.list_jobs().await
                    .iter().filter(|j| j.status == JobStatus::Running).count(),
                queued_jobs: state.list_jobs().await
                    .iter().filter(|j| j.status == JobStatus::Queued).count(),
                network_mode: format!("{:?}", state.policy().network.default),
                secure_boot_active: check_secure_boot(),
                dm_verity_active: check_dm_verity(),
                quarantine_count: 0,
                total_threats_detected: state.total_threats().await,
            })
        }

        DaemonRequest::Shutdown => {
            info!("Shutdown requested via IPC");
            tokio::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                std::process::exit(0);
            });
            DaemonResponse::Ok
        }

        _ => DaemonResponse::Error {
            code: "NOT_IMPLEMENTED".into(),
            message: "This command is not yet implemented".into(),
        },
    }
}

fn check_secure_boot() -> bool {
    // EFI 변수에서 SecureBoot 값 확인
    std::fs::read("/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c")
        .map(|data| data.get(4).copied() == Some(1))
        .unwrap_or(false)
}

fn check_dm_verity() -> bool {
    // dm-verity 활성 여부: /sys/block/dm-*/dm/name 에 verity 포함
    std::fs::read_to_string("/proc/mounts")
        .map(|m| m.contains("dm-verity"))
        .unwrap_or(false)
}
