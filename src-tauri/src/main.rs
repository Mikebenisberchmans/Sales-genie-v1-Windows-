#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod recorder;
mod commands;

use tauri::{Manager, PhysicalPosition};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::is_recording,
        ])
        .setup(|app| {
            if let Some(win) = app.get_webview_window("genie") {
                if let Ok(Some(monitor)) = win.current_monitor() {
                    let size = monitor.size();
                    let win_size = win.outer_size().unwrap_or_default();
                    // bottom-left corner, with a small margin
                    let x = 0i32;
                    let y = (size.height as i32) - (win_size.height as i32) - 40;
                    let _ = win.set_position(PhysicalPosition::new(x, y));
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
