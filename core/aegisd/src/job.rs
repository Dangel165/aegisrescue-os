//! 작업 실행기 — 각 JobType을 해당 aegis-* 서브프로세스로 디스패치

use aegis_common::types::{AegisJob, JobStatus, JobType};
use chrono::Utc;
use tracing::{error, info};

pub async fn execute(mut job: AegisJob) -> AegisJob {
    job.status = JobStatus::Running;
    job.started_at = Some(Utc::now());

    info!(job_id = %job.id, job_type = ?job.job_type, "executing job");

    let result = dispatch(&job.job_type).await;

    job.completed_at = Some(Utc::now());
    match result {
        Ok(()) => {
            job.status = JobStatus::Completed;
            info!(job_id = %job.id, "job completed");
        }
        Err(e) => {
            job.status = JobStatus::Failed;
            job.error = Some(e.to_string());
            error!(job_id = %job.id, error = %e, "job failed");
        }
    }

    job
}

async fn dispatch(job_type: &JobType) -> anyhow::Result<()> {
    match job_type {
        JobType::QuickScan { target } => {
            run_tool("aegis-scan", &["--quick".into(), target.display().to_string()]).await
        }
        JobType::FullScan { target } => {
            run_tool("aegis-scan", &["--full".into(), target.display().to_string()]).await
        }
        JobType::RansomwareRecovery { target, output } => {
            run_tool("aegis-recover", &[
                "auto".into(),
                target.display().to_string(),
                "--output".into(),
                output.display().to_string(),
            ]).await
        }
        JobType::BootRepair { device } => {
            run_tool("aegis-boot", &["repair".into(), device.display().to_string()]).await
        }
        JobType::DiskImage { source, dest } => {
            run_tool("aegis-image", &[
                source.display().to_string(),
                dest.display().to_string(),
            ]).await
        }
        JobType::ForensicCapture { target, output } => {
            run_tool("aegis-forensic", &[
                "capture".into(),
                target.display().to_string(),
                "--output".into(),
                output.display().to_string(),
            ]).await
        }
        JobType::Backup { source, dest } => {
            run_tool("aegis-image", &[
                "backup".into(),
                source.display().to_string(),
                dest.display().to_string(),
            ]).await
        }
        JobType::MountDisk { device } => {
            run_tool("aegis-mount", &["mount".into(), device.display().to_string()]).await
        }
        JobType::SandboxAnalysis { sample } => {
            run_tool("aegis-sandbox", &["run".into(), sample.display().to_string()]).await
        }
    }
}

async fn run_tool(tool: &str, args: &[String]) -> anyhow::Result<()> {
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
