#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod inference;
mod recorder;
mod storage;
mod transcriber;
mod warehouse;

use tauri::{Manager, PhysicalPosition};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::is_recording,
            commands::save_config,
            commands::get_config,
            commands::open_config_window,
            commands::upload_to_storage,
            commands::save_to_warehouse,
            commands::transcribe_and_analyze,
            commands::check_analysis_services,
        ])
        .setup(|app| {
            // ── Position genie window at bottom-left ──────────────────────
            if let Some(win) = app.get_webview_window("genie") {
                if let Ok(Some(monitor)) = win.current_monitor() {
                    let size     = monitor.size();
                    let win_size = win.outer_size().unwrap_or_default();
                    let x = 0i32;
                    let y = (size.height as i32) - (win_size.height as i32) - 40;
                    let _ = win.set_position(PhysicalPosition::new(x, y));
                }
            }

            // ── Resolve the bundled stt_server path ───────────────────────
            // The stt_server exe is bundled via tauri.conf.json `bundle.resources`.
            // At runtime, Tauri extracts it to a platform-specific resources dir
            // and we resolve it here — no Python installation needed.
            // Resolve the bundled stt_server directory from Tauri's resource dir.
            // At install time this is populated by build_sidecar.ps1 + PyInstaller.
            let stt_exe_path: String = app
                .path()
                .resource_dir()
                .map(|dir| dir.join("stt_server").to_string_lossy().to_string())
                .unwrap_or_default();

            // ── Auto-start STT server if AI analysis is enabled ───────────
            let cfg         = config::read_config(&app.handle());
            let ai_enabled  = cfg["analysis"]["enabled"].as_bool().unwrap_or(false);

            if ai_enabled {
                let model_size = cfg["analysis"]["whisperModel"]
                    .as_str()
                    .unwrap_or("base")
                    .to_string();
                let stt_port = cfg["analysis"]["sttPort"]
                    .as_u64()
                    .unwrap_or(8765) as u16;

                // Store the resolved path so commands.rs can use it too
                commands::set_stt_exe_path(stt_exe_path.clone());

                transcriber::spawn_stt_server(&stt_exe_path, &model_size, stt_port);
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                transcriber::kill_stt_server();
            }
        });
}
