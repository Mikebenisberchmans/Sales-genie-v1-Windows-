use crate::recorder::{start, RecordingSession};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

static SESSION: Lazy<Mutex<Option<RecordingSession>>> = Lazy::new(|| Mutex::new(None));

#[derive(Serialize)]
pub struct StopResult {
    pub mic_path: String,
    pub sys_path: String,
}

fn out_dir() -> PathBuf {
    let mut p = dirs_next::audio_dir().unwrap_or_else(|| std::env::temp_dir());
    p.push("GenieRecordings");
    p
}

#[tauri::command]
pub fn start_recording() -> Result<(), String> {
    let mut s = SESSION.lock().unwrap();
    if s.is_some() {
        return Err("already recording".into());
    }
    *s = Some(start(out_dir())?);
    Ok(())
}

#[tauri::command]
pub fn stop_recording() -> Result<StopResult, String> {
    let mut s = SESSION.lock().unwrap();
    let session = s.take().ok_or("not recording")?;
    let mic = session.mic_path.to_string_lossy().to_string();
    let sys = session.sys_path.to_string_lossy().to_string();
    drop(session); // closes streams + flushes WAVs
    Ok(StopResult { mic_path: mic, sys_path: sys })
}

#[tauri::command]
pub fn is_recording() -> bool {
    SESSION.lock().unwrap().is_some()
}
