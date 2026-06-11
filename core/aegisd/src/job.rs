//! 작업 실행기 — 각 JobType을 해당 aegis-* 서브프로세스로 디스패치

use crate::state::AppState;
use aegis_common::ipc::DaemonStream;
use aegis_common::types::{
    AegisJob, EventSeverity, EventType, JobProgress, JobStatus, JobType, ThreatInfo,
};
use chrono::Utc;
use serde::Deserialize;
use std::path::PathBuf;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct ScanOutput {
    total_files: u64,
    scanned_files: u64,
    malicious_count: u64,
    suspicious_count: u64,
    threats: Vec<ThreatInfo>,
}

pub async fn execute(state: AppState, mut job: AegisJob) -> AegisJob {
    job.status = JobStatus::Running;
    job.started_at = Some(Utc::now());
    job.progress = Some(JobProgress {
        current: 0,
        total: 100,
        message: "Starting…".into(),
        speed_bps: None,
        eta_secs: None,
    });
    state.update_job(job.clone()).await;
    state.broadcast(DaemonStream::JobProgress {
        job_id: job.id,
        progress: job.progress.clone().unwrap(),
    });

    info!(job_id = %job.id, job_type = ?job.job_type, "executing job");

    let result = dispatch(&state, &mut job).await;

    job.completed_at = Some(Utc::now());
    match result {
        Ok(()) => {
            job.status = JobStatus::Completed;
            if let Some(ref mut p) = job.progress {
                p.current = p.total.max(1);
                p.message = "Completed".into();
            }
            info!(job_id = %job.id, "job completed");
        }
        Err(e) => {
            job.status = JobStatus::Failed;
            job.error = Some(e.to_string());
            error!(job_id = %job.id, error = %e, "job failed");
        }
    }

    state.update_job(job.clone()).await;
    if let Some(progress) = job.progress.clone() {
        state.broadcast(DaemonStream::JobProgress {
            job_id: job.id,
            progress,
        });
    }
    job
}

async fn dispatch(state: &AppState, job: &mut AegisJob) -> anyhow::Result<()> {
    if state.is_job_cancelled(job.id).await {
        anyhow::bail!("Job cancelled");
    }

    match job.job_type.clone() {
        JobType::QuickScan { target } => run_scan(state, job, &target, true, false).await,
        JobType::StandardScan { target } => run_scan(state, job, &target, false, false).await,
        JobType::FullScan { target } => run_scan(state, job, &target, false, true).await,
        JobType::RansomwareRecovery { target, output } => {
            run_tool(
                state,
                job.id,
                "aegis-recover",
                &[
                    "auto".into(),
                    target.display().to_string(),
                    "--output".into(),
                    output.display().to_string(),
                ],
            )
            .await
        }
        JobType::BootRepair { device, windows, linux, efi } => {
            let mut args = vec!["repair".into(), device.display().to_string()];
            if windows {
                args.push("--windows".into());
            }
            if linux {
                args.push("--linux".into());
            }
            if efi {
                args.push("--efi".into());
            }
            run_tool(state, job.id, "aegis-boot", &args).await
        }
        JobType::DiskImage { source, dest } => {
            run_tool(
                state,
                job.id,
                "aegis-image",
                &[
                    source.display().to_string(),
                    dest.display().to_string(),
                ],
            )
            .await
        }
        JobType::ForensicCapture { target, output } => {
            run_tool(
                state,
                job.id,
                "aegis-forensic",
                &[
                    "capture".into(),
                    target.display().to_string(),
                    "--output".into(),
                    output.display().to_string(),
                ],
            )
            .await
        }
        JobType::Backup { source, dest } => {
            run_tool(
                state,
                job.id,
                "aegis-image",
                &[
                    "backup".into(),
                    source.display().to_string(),
                    dest.display().to_string(),
                ],
            )
            .await
        }
        JobType::MountDisk { device } => {
            run_tool(
                state,
                job.id,
                "aegis-mount",
                &["mount".into(), device.display().to_string()],
            )
            .await
        }
        JobType::SandboxAnalysis { sample } => {
            run_tool(
                state,
                job.id,
                "aegis-sandbox",
                &["run".into(), sample.display().to_string(), "--json".into()],
            )
            .await
        }
    }
}

async fn run_scan(
    state: &AppState,
    job: &mut AegisJob,
    target: &PathBuf,
    quick: bool,
    full: bool,
) -> anyhow::Result<()> {
    state
        .emit_event(
            EventType::ScanStarted,
            EventSeverity::Info,
            format!("Scan started: {}", target.display()),
            serde_json::json!({ "job_id": job.id, "target": target }),
        )
        .await;

    state
        .set_job_progress(
            job.id,
            JobProgress {
                current: 10,
                total: 100,
                message: "Running aegis-scan…".into(),
                speed_bps: None,
                eta_secs: None,
            },
        )
        .await;

    let mut args = vec!["--json".into(), target.display().to_string()];
    if quick {
        args.insert(0, "--quick".into());
    } else if full {
        args.insert(0, "--full".into());
    }

    let output = tokio::process::Command::new("aegis-scan")
        .args(&args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to spawn aegis-scan: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let results: ScanOutput = if stdout.trim().is_empty() {
        anyhow::bail!(
            "aegis-scan produced no JSON output (code {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        serde_json::from_str(stdout.trim()).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse scan JSON: {} — stdout: {}",
                e,
                stdout.chars().take(200).collect::<String>()
            )
        })?
    };

    for threat in &results.threats {
        state.emit_threat(threat).await;
        maybe_quarantine(state, threat).await;
    }

    job.progress = Some(JobProgress {
        current: results.scanned_files.max(1),
        total: results.total_files.max(results.scanned_files.max(1)),
        message: format!(
            "{} malicious, {} suspicious",
            results.malicious_count, results.suspicious_count
        ),
        speed_bps: None,
        eta_secs: None,
    });

    state
        .emit_event(
            EventType::ScanCompleted,
            EventSeverity::Info,
            format!(
                "Scan completed: {} files, {} threats",
                results.scanned_files,
                results.threats.len()
            ),
            serde_json::json!({
                "job_id": job.id,
                "malicious_count": results.malicious_count,
                "suspicious_count": results.suspicious_count,
            }),
        )
        .await;

    if !output.status.success() && results.malicious_count == 0 {
        anyhow::bail!(
            "aegis-scan exited with code {:?}",
            output.status.code()
        );
    }

    Ok(())
}

async fn maybe_quarantine(state: &AppState, threat: &ThreatInfo) {
    use aegis_common::types::ThreatVerdict;
    if threat.verdict != ThreatVerdict::Malicious {
        return;
    }

    let id = Uuid::new_v4();
    let mgr = state.quarantine_manager();
    let quarantine_path = mgr.quarantine_path_for(id);
    let entry = aegis_common::types::QuarantineEntry {
        id,
        original_path: threat.path.clone(),
        quarantine_path: quarantine_path.clone(),
        threat: threat.clone(),
        quarantined_at: Utc::now(),
        can_restore: true,
    };

    match mgr.quarantine(&entry) {
        Ok(()) => {
            if let Err(e) = state.add_quarantine_entry(entry.clone()).await {
                warn!(error = %e, "failed to persist quarantine entry");
            }
            state
                .emit_event(
                    EventType::QuarantineAdded,
                    EventSeverity::Warning,
                    format!("Quarantined: {}", threat.path.display()),
                    serde_json::json!({ "entry_id": id, "path": threat.path }),
                )
                .await;
        }
        Err(e) => {
            warn!(error = %e, path = %threat.path.display(), "quarantine failed");
        }
    }
}

async fn run_tool(
    state: &AppState,
    job_id: Uuid,
    tool: &str,
    args: &[String],
) -> anyhow::Result<()> {
    state
        .set_job_progress(
            job_id,
            JobProgress {
                current: 25,
                total: 100,
                message: format!("Running {}…", tool),
                speed_bps: None,
                eta_secs: None,
            },
        )
        .await;

    let status = tokio::process::Command::new(tool)
        .args(args)
        .status()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to spawn {}: {}", tool, e))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "{} exited with code {:?}",
            tool,
            status.code()
        ))
    }
}
