/// Manages the salenie_stt_server.py subprocess and calls its /transcribe endpoint.
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::{json, Value};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;

/// Global handle to the STT server subprocess.
static STT_PROCESS: Lazy<Mutex<Option<Child>>> = Lazy::new(|| Mutex::new(None));

// ---------------------------------------------------------------------------
// Process management
// ---------------------------------------------------------------------------

/// Spawn the Python STT server if it isn't already running.
pub fn spawn_stt_server(python_path: &str, script_path: &str, model_size: &str, port: u16) {
    let mut guard = STT_PROCESS.lock().unwrap();
    if guard.is_some() {
        return; // already running
    }

    eprintln!(
        "[transcriber] Spawning STT server: {} {} (whisper={}, port={})",
        python_path, script_path, model_size, port
    );

    let mut cmd = Command::new(python_path);
    cmd.arg(script_path)
        .env("WHISPER_MODEL", model_size)
        .env("STT_PORT", port.to_string());

    // Suppress the console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    match cmd.spawn() {
        Ok(child) => {
            eprintln!("[transcriber] STT server spawned (pid={})", child.id());
            *guard = Some(child);
        }
        Err(e) => {
            eprintln!("[transcriber] Failed to spawn STT server: {e}");
        }
    }
}

/// Kill the STT server subprocess (called on app exit).
pub fn kill_stt_server() {
    if let Ok(mut guard) = STT_PROCESS.lock() {
        if let Some(mut child) = guard.take() {
            eprintln!("[transcriber] Killing STT server (pid={})", child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// ---------------------------------------------------------------------------
// Health check
// ---------------------------------------------------------------------------

/// Poll /health until ready or timeout (up to ~60s).
#[allow(dead_code)]
pub async fn wait_for_stt_ready(port: u16) -> bool {
    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_else(|_| Client::new());

    for _ in 0..30 {
        if let Ok(resp) = client
            .get(format!("http://127.0.0.1:{port}/health"))
            .send()
            .await
        {
            if let Ok(json) = resp.json::<Value>().await {
                if json["status"] == "ready" {
                    return true;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    false
}

/// Quick check — is the server currently accepting requests?
pub async fn check_stt_ready(port: u16) -> bool {
    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_else(|_| Client::new());

    if let Ok(resp) = client
        .get(format!("http://127.0.0.1:{port}/health"))
        .send()
        .await
    {
        if let Ok(json) = resp.json::<Value>().await {
            return json["status"] == "ready";
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Transcription
// ---------------------------------------------------------------------------

/// Transcribe mic + system WAVs into a merged speaker-labelled transcript.
pub async fn transcribe(
    mic_path: &str,
    sys_path: &str,
    port: u16,
) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(600)) // 10-min ceiling for long recordings
        .build()
        .map_err(|e| e.to_string())?;

    // Labels MUST match the training data format used by the fine-tuned model
    let body = json!({
        "mic_path": mic_path,
        "sys_path": sys_path,
        "mic_label": "Speaker 1",
        "sys_label": "Speaker 2",
    });

    let resp = client
        .post(format!("http://127.0.0.1:{port}/transcribe"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("STT request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("STT server error {status}: {text}"));
    }

    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("STT response parse error: {e}"))?;

    json["transcript"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No transcript in STT response".to_string())
}

