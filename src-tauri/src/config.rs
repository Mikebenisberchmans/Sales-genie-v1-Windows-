use serde_json::Value;
use std::path::PathBuf;
use tauri::Manager;

pub fn config_path(app: &tauri::AppHandle) -> PathBuf {
    let mut p = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    p.push("genie-recorder");
    std::fs::create_dir_all(&p).ok();
    p.push("config.json");
    p
}

pub fn read_config(app: &tauri::AppHandle) -> Value {
    let path = config_path(app);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

pub fn write_config(app: &tauri::AppHandle, cfg: &Value) -> Result<(), String> {
    let path = config_path(app);
    let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, s).map_err(|e| e.to_string())
}
