//! Tauri 커맨드 핸들러

use crate::disk::{TargetDisk, DiskInfo};
use crate::install::{InstallConfig, InstallProgress, Installer};
use crate::preflight::{PreflightResult, run_preflight};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

/// 전역 설치 상태 (진행률 공유)
pub struct InstallerState(pub Mutex<Option<InstallProgress>>);

// ─── 시스템 사전 점검 ────────────────────────────────────────

#[tauri::command]
pub async fn preflight_check() -> Result<PreflightResult, String> {
    run_preflight().map_err(|e| e.to_string())
}

// ─── 디스크 관련 ─────────────────────────────────────────────

#[tauri::command]
pub async fn list_target_disks() -> Result<Vec<TargetDisk>, String> {
    crate::disk::list_disks().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_disk_info(device: String) -> Result<DiskInfo, String> {
    crate::disk::get_disk_info(&device).map_err(|e| e.to_string())
}

// ─── 설치 ────────────────────────────────────────────────────

#[tauri::command]
pub async fn validate_install_config(config: InstallConfig) -> Result<Vec<String>, String> {
    config.validate().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_installation(
    config: InstallConfig,
    window: tauri::Window,
) -> Result<(), String> {
    let mut installer = Installer::new(config);

    installer.run(move |progress| {
        let _ = window.emit("install-progress", &progress);
    }).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_install_progress() -> Result<Option<InstallProgress>, String> {
    Ok(None)
}

// ─── 시스템 정보 ─────────────────────────────────────────────

#[derive(Serialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub ram_mb: u64,
    pub cpu_cores: u32,
    pub arch: String,
    pub uefi: bool,
    pub secure_boot: bool,
    pub tpm_available: bool,
}

#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        hostname: hostname(),
        ram_mb: total_ram_mb(),
        cpu_cores: num_cpus(),
        arch: std::env::consts::ARCH.to_string(),
        uefi: is_uefi(),
        secure_boot: is_secure_boot(),
        tpm_available: is_tpm_available(),
    })
}

#[tauri::command]
pub async fn reboot_system() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("reboot").status();
    }
    Ok(())
}

#[tauri::command]
pub async fn open_aegiscenter() -> Result<(), String> {
    let _ = std::process::Command::new("aegiscenter").spawn();
    Ok(())
}

// ─── 내부 헬퍼 ───────────────────────────────────────────────

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn total_ram_mb() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("MemTotal:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u64>().ok())
        .map(|kb| kb / 1024)
        .unwrap_or(0)
}

fn num_cpus() -> u32 {
    std::fs::read_to_string("/proc/cpuinfo")
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("processor"))
        .count() as u32
}

fn is_uefi() -> bool {
    std::path::Path::new("/sys/firmware/efi").exists()
}

fn is_secure_boot() -> bool {
    std::fs::read("/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c")
        .map(|d| d.get(4).copied() == Some(1))
        .unwrap_or(false)
}

fn is_tpm_available() -> bool {
    std::path::Path::new("/dev/tpm0").exists() ||
    std::path::Path::new("/dev/tpmrm0").exists()
}
