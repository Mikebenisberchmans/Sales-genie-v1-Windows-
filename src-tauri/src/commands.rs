use crate::config;
use crate::inference;
use crate::recorder::{start, RecordingSession};
use crate::storage;
use crate::transcriber;
use crate::warehouse;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

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
    drop(session);
    Ok(StopResult {
        mic_path: mic,
        sys_path: sys,
    })
}

#[tauri::command]
pub fn is_recording() -> bool {
    SESSION.lock().unwrap().is_some()
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn save_config(app: tauri::AppHandle, config: serde_json::Value) -> Result<(), String> {
    config::write_config(&app, &config)
}

#[tauri::command]
pub fn get_config(app: tauri::AppHandle) -> serde_json::Value {
    config::read_config(&app)
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn open_config_window(app: tauri::AppHandle) -> Result<(), String> {
    match app.get_webview_window("config-ledger") {
        Some(win) => {
            // On Windows, show() alone is not enough if the window is minimised
            // or if focus-steal prevention kicks in. The sequence below is
            // the most reliable way to force it to the foreground.
            win.show().map_err(|e| e.to_string())?;
            win.unminimize().ok(); // no-op if not minimised, ignore error
            win.set_focus().map_err(|e| e.to_string())?;
        }
        None => {
            // Window was never created — tauri.conf.json may be missing the
            // "config-ledger" entry.
            return Err("Config window not found — check tauri.conf.json 'windows' list".into());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Blob upload + warehouse
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn upload_to_storage(
    mic_path: String,
    sys_path: String,
    opp_id: String,
    config: serde_json::Value,
) -> Result<serde_json::Value, String> {
    storage::upload(&mic_path, &sys_path, &opp_id, &config).await
}

#[tauri::command]
pub async fn save_to_warehouse(
    metadata: serde_json::Value,
    warehouse_config: serde_json::Value,
) -> Result<(), String> {
    warehouse::insert(&metadata, &warehouse_config).await
}

// ---------------------------------------------------------------------------
// AI pipeline
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AnalysisResult {
    pub transcript: String,
    pub analysis:   serde_json::Value,
}

/// Transcribe mic + sys WAVs via the STT server, then run the Phi-3 analysis
/// via Ollama.  Returns both transcript text and the structured JSON analysis.
#[tauri::command]
pub async fn transcribe_and_analyze(
    mic_path: String,
    sys_path: String,
    config:   serde_json::Value,
) -> Result<AnalysisResult, String> {
    let stt_port: u16 = config["analysis"]["sttPort"]
        .as_u64()
        .unwrap_or(8765) as u16;

    let inference_endpoint = config["analysis"]["inferenceEndpoint"]
        .as_str()
        .unwrap_or(inference::default_endpoint())
        .to_string();

    // --- Step 1: Transcription ---
    let transcript =
        transcriber::transcribe(&mic_path, &sys_path, stt_port).await?;

    // --- Step 2: Analysis (BitsAndBytes NF4 inference server) ---
    let analysis =
        inference::analyze(&transcript, "salenie-phi", &inference_endpoint).await?;

    Ok(AnalysisResult { transcript, analysis })
}

/// Health-check both services. Returns { stt: bool, ollama: bool, model: bool }.
/// If the STT server is not yet running but paths are configured, this spawns
/// it first and waits up to 15 seconds for it to become ready.
#[tauri::command]
pub async fn check_analysis_services(config: serde_json::Value) -> serde_json::Value {
    let stt_port: u16 = config["analysis"]["sttPort"]
        .as_u64()
        .unwrap_or(8765) as u16;

    let inference_endpoint = config["analysis"]["inferenceEndpoint"]
        .as_str()
        .unwrap_or(inference::default_endpoint())
        .to_string();

    // If the STT server isn't already running, try to spawn it so the health
    // check has something to talk to (only if paths are configured).
    let python_path = config["analysis"]["pythonPath"].as_str().unwrap_or("").to_string();
    let script_path = config["analysis"]["scriptPath"].as_str().unwrap_or("").to_string();
    let whisper_model = config["analysis"]["whisperModel"].as_str().unwrap_or("base").to_string();

    if !python_path.is_empty() && !script_path.is_empty() {
        // Spawn is a no-op if already running (guarded by STT_PROCESS mutex)
        transcriber::spawn_stt_server(&python_path, &script_path, &whisper_model, stt_port);

        // Wait up to 15 s for Whisper to load (it downloads model on first run)
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        for _ in 0..5 {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            if let Ok(resp) = client.get(format!("http://127.0.0.1:{stt_port}/health")).send().await {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if json["status"] == "ready" { break; }
                }
            }
        }
    }

    let (stt_ok, infer_ok) = tokio::join!(
        transcriber::check_stt_ready(stt_port),
        inference::check_ollama(&inference_endpoint),
    );

    serde_json::json!({
        "stt":    stt_ok,
        "ollama": infer_ok,
        "model":  infer_ok,
    })
}
