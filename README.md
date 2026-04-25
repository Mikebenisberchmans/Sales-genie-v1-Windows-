# 🧞 Genie Recorder

A Tauri + React desktop app for **AI-powered sales call recording and analysis**. A chibi genie lives at the left edge of your screen — hover to reveal controls, record both mic and system audio as separate tracks, then let the pipeline automatically transcribe, analyse with a fine-tuned Phi-4 model, upload to cloud storage, and write structured metadata to your data warehouse.

---

## What it does

```
Record call (mic + system audio, dual-track WAV)
    ↓
Whisper STT (faster-whisper, local Python server)
    → Speaker-labelled transcript: "Speaker 1: ... / Speaker 2: ..."
    ↓
Phi-4 Fine-tuned Model (Modal cloud inference)
    → Structured JSON: summary, deal size, sentiment, next steps
    ↓
Cloud Storage (S3 / Azure Blob / GCS)
    → mic_<ts>.wav + sys_<ts>.wav stored under {opp_id}/
    ↓
Data Warehouse (Snowflake / BigQuery / ClickHouse / Databricks / Redshift)
    → One row per call with all metadata + AI analysis
```

---

## Features

| Feature | Detail |
|---|---|
| 🎤 Dual-track recording | Mic + system audio as separate WAVs via WASAPI loopback |
| 🧞 Animated SVG genie | GPU-composited fly-in, breathing float, head-nod while recording |
| 🕹️ Arc button controls | Frosted-glass buttons arc above genie head on hover |
| 🎙️ Whisper STT | Local faster-whisper server, speaker labels match training format |
| 🧠 AI analysis | Phi-4 fine-tuned on sales calls via Modal serverless GPU |
| ☁️ Object storage | S3 (SigV4), Azure Blob (Shared Key), GCS (service account) |
| 🗄️ Warehouse insert | Snowflake, BigQuery, ClickHouse, Databricks, Redshift |
| ⚙️ Config ledger | Dark-themed tabbed config window — no manual JSON editing |
| 🪪 Opportunity ID | Modal prompts for opp ID on stop — links recording to CRM deal |
| 🔊 Consent disclaimer | One-button TTS disclaimer before the call starts |

---

## Prerequisites

### App (always required)
- [Rust](https://rustup.rs) + `cargo`
- [Node.js 18+](https://nodejs.org)
- **Windows only:** Microsoft C++ Build Tools + WebView2 (pre-installed on Win 11)

### AI pipeline (optional — only needed if AI Analysis is enabled)
- Python 3.10+ with a virtual environment
- `pip install faster-whisper uvicorn fastapi`
- A [Modal](https://modal.com) account with the `salenie-generate` endpoint deployed
- See `salenie_model/phi_sales_model/` for the inference server and training code

---

## Quick start

```bash
npm install
npm run tauri dev   # first build takes 5–10 min (Cargo)
```

### Production build (creates .msi + .exe installer)
```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/
```

---

## Project structure

```
genie-recorder/
├── index.html                        Main window entry
├── ledger.html                       Config ledger window entry
├── vite.config.js                    Multi-page Vite build
├── src/                              React frontend
│   ├── App.jsx                       Main orchestrator — recording state, pipeline
│   ├── styles.css                    All CSS animations (GPU-composited)
│   ├── ledger.jsx                    Config ledger React entry
│   ├── ledger.css                    Dark-themed ledger styles
│   ├── sounds.js                     TTS disclaimer + UI sounds
│   └── components/
│       ├── Genie.jsx                 Chibi SVG genie with animated layers
│       ├── OppModal.jsx              Opportunity ID modal on stop
│       └── ConfigLedger.jsx          4-tab config form (salesperson/storage/warehouse/AI)
└── src-tauri/                        Rust backend
    ├── Cargo.toml
    ├── tauri.conf.json               Window definitions (main + config-ledger)
    ├── capabilities/default.json     Tauri permissions
    └── src/
        ├── main.rs                   Tauri entrypoint + app setup
        ├── recorder.rs               cpal dual-track WAV recording
        ├── commands.rs               All #[tauri::command] handlers
        ├── config.rs                 Read/write config.json in app data dir
        ├── transcriber.rs            STT server subprocess + /transcribe client
        ├── inference.rs              Modal inference HTTP client
        ├── storage.rs                S3 / Azure Blob / GCS upload (manual signing)
        └── warehouse.rs              Snowflake / BigQuery / ClickHouse / Databricks / Redshift
```

---

## Configuration

Click the **database icon** (⬡) in the arc controls to open the config ledger. Settings are saved to:

```
Windows: %APPDATA%\com.genie-recorder\config.json
macOS:   ~/Library/Application Support/com.genie-recorder/config.json
Linux:   ~/.config/com.genie-recorder/config.json
```

### Tab 1 — Salesperson
Your name and employee ID — stamped on every warehouse row.

### Tab 2 — Object Store
Choose S3, Azure Blob, or GCS. Each provider shows its own credential fields. Files are stored as:
```
{bucket}/{prefix}/{opp_id}/mic_{timestamp}.wav
{bucket}/{prefix}/{opp_id}/sys_{timestamp}.wav
```

### Tab 3 — Warehouse
Choose your warehouse. The app creates one row per recording with these columns:

| Column | Type | Description |
|---|---|---|
| `opp_id` | TEXT | Opportunity ID you enter on stop |
| `submission_date` | TEXT | ISO-8601 timestamp |
| `duration_seconds` | INT | Call length |
| `salesperson_name` | TEXT | From config |
| `salesperson_id` | TEXT | From config |
| `mic_url` | TEXT | Cloud URL of mic WAV |
| `sys_url` | TEXT | Cloud URL of system audio WAV |
| `transcript_text` | TEXT | Full speaker-labelled transcript |
| `ai_summary` | TEXT | One-line AI summary |
| `deal_amount` | FLOAT | Extracted deal size |
| `sentiment_score` | FLOAT | 0–1 sentiment |
| `next_steps` | TEXT | JSON array of action items |
| `full_analysis_json` | TEXT | Complete Phi-4 output |

### Tab 4 — AI Analysis
Toggle AI on/off. When enabled:
- **Inference Endpoint** — your Modal URL + API token (space-separated): `https://org--salenie-generate.modal.run TOKEN`
- **Whisper Model** — `tiny` / `base` / `small` / `medium`
- **STT Port** — port for the local faster-whisper server (default `8765`)
- **Python Path** — path to Python in your venv
- **STT Script** — path to `salenie_stt_server.py`

Click **Test Services** — this spawns the STT server automatically (waits up to 15 s for the Whisper model to load), then probes both services.

---

## How dual-track recording works

| Track | Source | API |
|---|---|---|
| Mic | Default input device | `cpal::build_input_stream` |
| System audio | Default output device (loopback) | `cpal::build_input_stream` on WASAPI loopback |

On **Windows**, WASAPI natively supports loopback on the default output device — no virtual driver needed. On **macOS**, install [BlackHole](https://github.com/ExistentialAudio/BlackHole).

---

## Animation architecture

All animations run on the GPU compositor (zero main-thread jank):

```
.genie-wrap   → fly-in only (translate3d + scale, no rotate)
.genie-float  → breathing bob (child element, no animation chain conflict)
.tail         → tail-wave (rotate around tail base)
.arms         → arms-sway (gentle rock)
.head         → head-nod (only while recording)
.aura         → pulse (2.8s idle → 1.4s while recording)
```

`will-change: transform`, `backface-visibility: hidden`, and `isolation: isolate` are set on every animated layer to keep them on separate compositor layers.

---

## Warehouse table DDL

Run once against your warehouse before first use:

### Snowflake / Redshift / Databricks
```sql
CREATE TABLE genie_recordings (
  opp_id             VARCHAR,
  submission_date    VARCHAR,
  duration_seconds   INTEGER,
  salesperson_name   VARCHAR,
  salesperson_id     VARCHAR,
  mic_url            VARCHAR,
  sys_url            VARCHAR,
  mic_local_path     VARCHAR,
  sys_local_path     VARCHAR,
  sample_rate        INTEGER,
  channels           INTEGER,
  transcript_text    TEXT,
  ai_summary         TEXT,
  deal_amount        FLOAT,
  deal_company       VARCHAR,
  deal_stage         VARCHAR,
  sentiment_score    FLOAT,
  next_steps         TEXT,
  full_analysis_json TEXT
);
```

### BigQuery
```json
[
  {"name":"opp_id","type":"STRING"},
  {"name":"submission_date","type":"STRING"},
  {"name":"duration_seconds","type":"INTEGER"},
  {"name":"salesperson_name","type":"STRING"},
  {"name":"salesperson_id","type":"STRING"},
  {"name":"mic_url","type":"STRING"},
  {"name":"sys_url","type":"STRING"},
  {"name":"transcript_text","type":"STRING"},
  {"name":"ai_summary","type":"STRING"},
  {"name":"deal_amount","type":"FLOAT"},
  {"name":"sentiment_score","type":"FLOAT"},
  {"name":"next_steps","type":"STRING"},
  {"name":"full_analysis_json","type":"STRING"}
]
```

---

## Known limitations

- **Pause is UI-only** — cpal streams keep writing during "pause"; future: drop and re-create streams
- **macOS loopback** — requires BlackHole virtual audio driver
- **STT cold start** — first run downloads the Whisper model (~74 MB for `base`); subsequent starts are fast
- **Modal warm-up** — Modal containers spin down after inactivity; first inference after idle takes ~20 s
