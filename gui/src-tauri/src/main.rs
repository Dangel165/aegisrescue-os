//! AegisCenter Tauri 백엔드

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod commands;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::list_disks,
            commands::start_scan,
            commands::start_recovery,
            commands::start_boot_repair,
            commands::start_forensic_capture,
            commands::start_image,
            commands::list_jobs,
            commands::cancel_job,
            commands::get_job_progress,
            commands::set_network_mode,
            commands::list_quarantine,
            commands::restore_quarantine,
            commands::delete_quarantine,
            commands::get_events,
            commands::run_ai_analysis,
            commands::run_sandbox,
        ])
        .setup(|app| {
            let _window = app.get_window("main").unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("AegisCenter failed to start");
}
