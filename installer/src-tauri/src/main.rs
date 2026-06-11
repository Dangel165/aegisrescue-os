//! AegisRescue 설치 마법사 — Tauri 백엔드

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod commands;
mod disk;
mod install;
mod preflight;

use commands::InstallerState;
use std::sync::{Arc, Mutex};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_window("main") {
                let _ = window.set_minimizable(false);
            }
            Ok(())
        })
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                api.prevent_close();
                let _ = event.window().emit("close-requested", ());
            }
        })
        .manage(InstallerState(Arc::new(Mutex::new(None))))
        .invoke_handler(tauri::generate_handler![
            // 시스템 점검
            commands::preflight_check,
            // 디스크
            commands::list_target_disks,
            commands::get_disk_info,
            // 설치
            commands::validate_install_config,
            commands::start_installation,
            commands::get_install_progress,
            // 유틸
            commands::get_system_info,
            commands::reboot_system,
            commands::open_aegiscenter,
        ])
        .run(tauri::generate_context!())
        .expect("AegisRescue Installer failed to start");
}
