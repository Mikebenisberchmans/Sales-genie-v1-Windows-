/// Ollama HTTP client for Phi-3 sales analysis.
/// Calls the local Ollama server (default: http://localhost:11434).
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

const DEFAULT_ENDPOINT: &str = "https://mikebenisberchmans--salenie-generate.modal.run";
const TIMEOUT_SECS: u64 = 120; // Modal A10G — full bf16, ~5-15s per call

/// Send the raw transcript — the Modal endpoint wraps it with the exact
/// instruction and chat-template tokens used during fine-tuning.
fn build_prompt(transcript: &str) -> String {
    transcript.to_string()
}

/// Call Ollama /api/generate and return parsed JSON analysis.
/// Falls back to `{"raw": "<model_text>"}` if the output isn't valid JSON.
pub async fn analyze(
    transcript: &str,
    model: &str,
    endpoint: &str,
) -> Result<Value, String> {
    // endpoint may carry an optional Bearer token after a space:
    // "https://org--salenie-generate.modal.run TOKEN"
    let (url, token) = match endpoint.split_once(' ') {
        Some((u, t)) => (u.trim().to_string(), t.trim().to_string()),
        None         => (endpoint.to_string(), String::new()),
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    let prompt = build_prompt(transcript);

    let body = json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.1,
            "num_predict": 900,
            "stop": ["<|end|>", "<|user|>"],
            "repeat_penalty": 1.1
        }
    });

    let mut req = client.post(&url).json(&body);
    if !token.is_empty() {
        req = req.header("authorization", format!("Bearer {token}"));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Inference request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Ollama error {status}: {text}"));
    }

    let resp_json: Value = resp
        .json()
        .await
        .map_err(|e| format!("Ollama response parse error: {e}"))?;

    let raw_text = resp_json["response"].as_str().unwrap_or("").trim().to_string();

    // Extract JSON from the response — model sometimes wraps it in ```json ... ```
    let json_str = extract_json(&raw_text);

    match serde_json::from_str::<Value>(&json_str) {
        Ok(parsed) => Ok(parsed),
        Err(_) => {
            // Return raw text so callers can still store it
            Ok(json!({ "raw": raw_text }))
        }
    }
}

/// Strip markdown code fences and extract the first JSON object/array.
fn extract_json(text: &str) -> String {
    // Remove ```json ... ``` fences
    let stripped = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Find first { or [
    let start = stripped
        .find(|c| c == '{' || c == '[')
        .unwrap_or(0);

    // Find matching last } or ]
    let end_brace = stripped.rfind('}');
    let end_bracket = stripped.rfind(']');
    let end = match (end_brace, end_bracket) {
        (Some(a), Some(b)) => a.max(b) + 1,
        (Some(a), None) => a + 1,
        (None, Some(b)) => b + 1,
        (None, None) => stripped.len(),
    };

    if start < end {
        stripped[start..end].to_string()
    } else {
        stripped.to_string()
    }
}

/// Quick liveness check — pings the /health endpoint.
/// endpoint format: "https://org--salenie-generate.modal.run TOKEN"
/// health URL is derived by swapping "generate" for "health".
pub async fn check_ollama(endpoint: &str) -> bool {
    let (url, _) = match endpoint.split_once(' ') {
        Some((u, t)) => (u.trim().to_string(), t.trim().to_string()),
        None         => (endpoint.to_string(), String::new()),
    };
    // Derive health URL from generate URL
    let health_url = url.replace("salenie-generate", "salenie-health");

    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .unwrap_or_else(|_| Client::new());

    client
        .get(&health_url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// For the BitsAndBytes server the model is always loaded — just check health.
#[allow(dead_code)]
pub async fn model_exists(_model: &str, endpoint: &str) -> bool {
    check_ollama(endpoint).await
}

pub fn default_endpoint() -> &'static str {
    DEFAULT_ENDPOINT
}
