//! Tauri 커맨드 — aegisd IPC 브리지

use aegis_common::ipc::{DaemonRequest, DaemonResponse, SystemStatus, AEGIS_SOCKET_PATH};
use aegis_common::types::JobType;
use serde_json::Value;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

async fn ipc_call(req: DaemonRequest) -> Result<DaemonResponse, String> {
    let stream = UnixStream::connect(AEGIS_SOCKET_PATH).await
        .map_err(|_| "aegisd not running. Start daemon first.".to_string())?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut msg = serde_json::to_string(&req)
        .map_err(|e| e.to_string())?;
    msg.push('\n');
    writer.write_all(msg.as_bytes()).await
        .map_err(|e| e.to_string())?;

    let mut line = String::new();
    reader.read_line(&mut line).await
        .map_err(|e| e.to_string())?;

    serde_json::from_str(line.trim()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_status() -> Result<Value, String> {
    match ipc_call(DaemonRequest::GetStatus).await? {
        DaemonResponse::Status(s) => serde_json::to_value(s).map_err(|e| e.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn list_disks() -> Result<Value, String> {
    match ipc_call(DaemonRequest::ListDisks).await? {
        DaemonResponse::Disks(d) => serde_json::to_value(d).map_err(|e| e.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn start_scan(target: String, quick: bool) -> Result<String, String> {
    let job_type = if quick {
        JobType::QuickScan { target: PathBuf::from(target) }
    } else {
        JobType::FullScan { target: PathBuf::from(target) }
    };

    match ipc_call(DaemonRequest::SubmitJob { job_type }).await? {
        DaemonResponse::JobSubmitted { job_id } => Ok(job_id.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn start_recovery(target: String, output: String) -> Result<String, String> {
    let job_type = JobType::RansomwareRecovery { target: PathBuf::from(target) };
    match ipc_call(DaemonRequest::SubmitJob { job_type }).await? {
        DaemonResponse::JobSubmitted { job_id } => Ok(job_id.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn start_boot_repair(device: String) -> Result<String, String> {
    let job_type = JobType::BootRepair { device: PathBuf::from(device) };
    match ipc_call(DaemonRequest::SubmitJob { job_type }).await? {
        DaemonResponse::JobSubmitted { job_id } => Ok(job_id.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn start_forensic_capture(source: String, output: String) -> Result<String, String> {
    let job_type = JobType::ForensicCapture { target: PathBuf::from(source) };
    match ipc_call(DaemonRequest::SubmitJob { job_type }).await? {
        DaemonResponse::JobSubmitted { job_id } => Ok(job_id.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn start_image(source: String, dest: String) -> Result<String, String> {
    let job_type = JobType::DiskImage {
        source: PathBuf::from(source),
        dest: PathBuf::from(dest),
    };
    match ipc_call(DaemonRequest::SubmitJob { job_type }).await? {
        DaemonResponse::JobSubmitted { job_id } => Ok(job_id.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn list_jobs() -> Result<Value, String> {
    match ipc_call(DaemonRequest::ListJobs).await? {
        DaemonResponse::Jobs(j) => serde_json::to_value(j).map_err(|e| e.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn cancel_job(job_id: String) -> Result<(), String> {
    let id = job_id.parse().map_err(|_| "Invalid job ID".to_string())?;
    match ipc_call(DaemonRequest::CancelJob { job_id: id }).await? {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn get_job_progress(job_id: String) -> Result<Value, String> {
    let id = job_id.parse().map_err(|_| "Invalid job ID".to_string())?;
    match ipc_call(DaemonRequest::GetJob { job_id: id }).await? {
        DaemonResponse::Job(j) => serde_json::to_value(j).map_err(|e| e.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn set_network_mode(mode: String) -> Result<(), String> {
    match ipc_call(DaemonRequest::SetNetworkMode { mode }).await? {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn list_quarantine() -> Result<Value, String> {
    match ipc_call(DaemonRequest::ListQuarantine).await? {
        DaemonResponse::Quarantine(q) => serde_json::to_value(q).map_err(|e| e.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn restore_quarantine(entry_id: String, restore_path: String) -> Result<(), String> {
    let id = entry_id.parse().map_err(|_| "Invalid entry ID".to_string())?;
    match ipc_call(DaemonRequest::RestoreQuarantine {
        entry_id: id,
        restore_path: PathBuf::from(restore_path),
    }).await? {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn delete_quarantine(entry_id: String) -> Result<(), String> {
    let id = entry_id.parse().map_err(|_| "Invalid entry ID".to_string())?;
    match ipc_call(DaemonRequest::DeleteQuarantine { entry_id: id }).await? {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn get_events(limit: Option<u32>) -> Result<Value, String> {
    match ipc_call(DaemonRequest::GetEvents { since: None, limit }).await? {
        DaemonResponse::Events(e) => serde_json::to_value(e).map_err(|e| e.to_string()),
        DaemonResponse::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".into()),
    }
}

#[tauri::command]
pub async fn run_ai_analysis(file: String) -> Result<Value, String> {
    let output = tokio::process::Command::new("aegis-ai")
        .args([&file, "--json"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_sandbox(sample: String, level: u8) -> Result<Value, String> {
    let output = tokio::process::Command::new("aegis-sandbox")
        .args(["run", &sample, "--level", &level.to_string()])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    serde_json::from_slice(&output.stdout)
        .or_else(|_| Ok::<Value, String>(serde_json::json!({
            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        })))
}
