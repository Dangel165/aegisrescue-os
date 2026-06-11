//! Unix 소켓 IPC 서버

use crate::job;
use crate::state::AppState;
use aegis_common::ipc::{DaemonRequest, DaemonResponse, SystemStatus};
use aegis_common::policy::NetworkMode;
use aegis_common::types::{AegisJob, DiskInfo, EventSeverity, EventType, JobStatus, PartitionTableType};
use chrono::Utc;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use uuid::Uuid;

pub async fn run(state: AppState, socket_path: &Path) -> anyhow::Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let listener = UnixListener::bind(socket_path)?;
    info!("IPC server listening on {}", socket_path.display());

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
        if n == 0 {
            break;
        }

        let request: DaemonRequest = match serde_json::from_str(line.trim()) {
            Ok(r) => r,
            Err(e) => {
                let resp = DaemonResponse::Error {
                    code: "PARSE_ERROR".into(),
                    message: e.to_string(),
                };
                write_response(&mut writer, &resp).await?;
                continue;
            }
        };

        if matches!(request, DaemonRequest::SubscribeEvents) {
            write_response(&mut writer, &DaemonResponse::Subscribed).await?;
            stream_events(&mut writer, state.clone()).await?;
            break;
        }

        let response = process_request(request, &state).await;
        write_response(&mut writer, &response).await?;
    }

    Ok(())
}

async fn write_response(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    response: &DaemonResponse,
) -> anyhow::Result<()> {
    let mut out = serde_json::to_string(response)?;
    out.push('\n');
    writer.write_all(out.as_bytes()).await?;
    Ok(())
}

async fn stream_events(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    state: AppState,
) -> anyhow::Result<()> {
    let mut stream = tokio_stream::wrappers::BroadcastStream::new(state.subscribe_events());
    while let Some(item) = stream.next().await {
        match item {
            Ok(event) => {
                let mut out = serde_json::to_string(&event)?;
                out.push('\n');
                if writer.write_all(out.as_bytes()).await.is_err() {
                    break;
                }
            }
            Err(_) => continue,
        }
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

            let worker_state = state.clone();
            tokio::spawn(async move {
                if let Some(job) = worker_state.get_job(id).await {
                    let finished = job::execute(worker_state.clone(), job).await;
                    worker_state.update_job(finished).await;
                }
            });

            DaemonResponse::JobSubmitted { job_id: id }
        }

        DaemonRequest::GetJob { job_id } => match state.get_job(job_id).await {
            Some(job) => DaemonResponse::Job(job),
            None => DaemonResponse::Error {
                code: "NOT_FOUND".into(),
                message: format!("Job {} not found", job_id),
            },
        },

        DaemonRequest::ListJobs => DaemonResponse::Jobs(state.list_jobs().await),

        DaemonRequest::CancelJob { job_id } => {
            if let Some(mut job) = state.get_job(job_id).await {
                if job.status == JobStatus::Running || job.status == JobStatus::Queued {
                    job.status = JobStatus::Cancelled;
                    job.completed_at = Some(Utc::now());
                    state.update_job(job).await;
                    state
                        .emit_event(
                            EventType::UserAction,
                            EventSeverity::Info,
                            format!("Job {} cancelled", job_id),
                            serde_json::json!({ "job_id": job_id }),
                        )
                        .await;
                    DaemonResponse::Ok
                } else {
                    DaemonResponse::Error {
                        code: "INVALID_STATE".into(),
                        message: format!("Job {} cannot be cancelled", job_id),
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
            let jobs = state.list_jobs().await;
            DaemonResponse::Status(SystemStatus {
                version: env!("CARGO_PKG_VERSION").to_string(),
                uptime_secs: state.uptime_secs(),
                active_jobs: jobs
                    .iter()
                    .filter(|j| j.status == JobStatus::Running)
                    .count(),
                queued_jobs: jobs
                    .iter()
                    .filter(|j| j.status == JobStatus::Queued)
                    .count(),
                network_mode: format!("{:?}", state.network_mode().await),
                secure_boot_active: check_secure_boot(),
                dm_verity_active: check_dm_verity(),
                quarantine_count: state.quarantine_count().await,
                total_threats_detected: state.total_threats().await,
            })
        }

        DaemonRequest::ListDisks => DaemonResponse::Disks(list_block_devices()),

        DaemonRequest::ListQuarantine => {
            DaemonResponse::Quarantine(state.list_quarantine().await)
        }

        DaemonRequest::RestoreQuarantine {
            entry_id,
            restore_path,
        } => match state.get_quarantine(entry_id).await {
            Some(entry) => {
                let mgr = state.quarantine_manager();
                match mgr.restore(&entry, &restore_path) {
                    Ok(()) => {
                        let _ = state.remove_quarantine_entry(entry_id).await;
                        state
                            .emit_event(
                                EventType::QuarantineRestored,
                                EventSeverity::Info,
                                format!("Restored {} → {}", entry.original_path.display(), restore_path.display()),
                                serde_json::json!({ "entry_id": entry_id, "restore_path": restore_path }),
                            )
                            .await;
                        DaemonResponse::Ok
                    }
                    Err(e) => DaemonResponse::Error {
                        code: "RESTORE_FAILED".into(),
                        message: e.to_string(),
                    },
                }
            }
            None => DaemonResponse::Error {
                code: "NOT_FOUND".into(),
                message: format!("Quarantine entry {} not found", entry_id),
            },
        },

        DaemonRequest::DeleteQuarantine { entry_id } => {
            match state.get_quarantine(entry_id).await {
                Some(entry) => {
                    let mgr = state.quarantine_manager();
                    match mgr.delete(&entry) {
                        Ok(()) => {
                            let _ = state.remove_quarantine_entry(entry_id).await;
                            state
                                .emit_event(
                                    EventType::UserAction,
                                    EventSeverity::Warning,
                                    format!("Quarantine entry {} deleted", entry_id),
                                    serde_json::json!({ "entry_id": entry_id }),
                                )
                                .await;
                            DaemonResponse::Ok
                        }
                        Err(e) => DaemonResponse::Error {
                            code: "DELETE_FAILED".into(),
                            message: e.to_string(),
                        },
                    }
                }
                None => DaemonResponse::Error {
                    code: "NOT_FOUND".into(),
                    message: format!("Quarantine entry {} not found", entry_id),
                },
            }
        }

        DaemonRequest::GetEvents { since, limit } => {
            match state.list_events(since, limit).await {
                Ok(events) => DaemonResponse::Events(events),
                Err(e) => DaemonResponse::Error {
                    code: "DB_ERROR".into(),
                    message: e.to_string(),
                },
            }
        }

        DaemonRequest::SetNetworkMode { mode } => match parse_network_mode(&mode) {
            Some(net_mode) => {
                state.set_network_mode(net_mode).await;
                DaemonResponse::Ok
            }
            None => DaemonResponse::Error {
                code: "INVALID_MODE".into(),
                message: format!("Unknown network mode: {}", mode),
            },
        },

        DaemonRequest::Shutdown => {
            info!("Shutdown requested via IPC");
            tokio::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                std::process::exit(0);
            });
            DaemonResponse::Ok
        }

        DaemonRequest::SubscribeEvents => DaemonResponse::Error {
            code: "INTERNAL".into(),
            message: "SubscribeEvents handled in handle_client".into(),
        },
    }
}

fn check_secure_boot() -> bool {
    std::fs::read("/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c")
        .map(|data| data.get(4).copied() == Some(1))
        .unwrap_or(false)
}

fn check_dm_verity() -> bool {
    std::fs::read_to_string("/proc/mounts")
        .map(|m| m.contains("dm-verity"))
        .unwrap_or(false)
}

fn parse_network_mode(mode: &str) -> Option<NetworkMode> {
    match mode.to_ascii_lowercase().as_str() {
        "airgap" | "air-gap" => Some(NetworkMode::AirGap),
        "whitelist" => Some(NetworkMode::Whitelist),
        "restricted" => Some(NetworkMode::Restricted),
        "full" => Some(NetworkMode::Full),
        _ => None,
    }
}

fn list_block_devices() -> Vec<DiskInfo> {
    let output = std::process::Command::new("lsblk")
        .args(["-J", "-b", "-o", "NAME,SIZE,TYPE,MODEL"])
        .output();

    let Ok(output) = output else {
        return vec![];
    };

    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return vec![];
    };

    let mut disks = vec![];
    if let Some(devs) = json["blockdevices"].as_array() {
        for dev in devs {
            if dev["type"].as_str() != Some("disk") {
                continue;
            }
            let name = dev["name"].as_str().unwrap_or("unknown");
            disks.push(DiskInfo {
                device: Path::new("/dev").join(name),
                model: dev["model"].as_str().map(|s| s.to_string()),
                serial: None,
                size_bytes: dev["size"].as_u64().unwrap_or(0),
                sector_size: 512,
                partition_table: PartitionTableType::Unknown,
                partitions: vec![],
                health: aegis_common::types::DiskHealth {
                    smart_available: false,
                    overall_health: aegis_common::types::HealthStatus::Unknown,
                    reallocated_sectors: 0,
                    pending_sectors: 0,
                    uncorrectable_errors: 0,
                },
            });
        }
    }

    disks
}
