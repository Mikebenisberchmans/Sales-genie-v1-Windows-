# 🧞 Genie Recorder

> AI-powered sales call recorder — dual-track audio, automatic transcription, Phi-4 deal analysis, and direct warehouse insert. All from a genie who lives at the edge of your screen.

[![Model](https://img.shields.io/badge/HuggingFace-Salenie--Phi4--v1-yellow)](https://huggingface.co/Mike-Benis/Salenie-Phi4-v1)
[![Trainer](https://img.shields.io/badge/GitHub-Salenie--trainer--pipeline-blue)](https://github.com/Mikebenisberchmans/Salenie-trainer-pipeline)
[![Built with Tauri](https://img.shields.io/badge/Built%20with-Tauri%202-orange)](https://tauri.app)

---

## What it does

```
🎤 Record                    ✍️ Transcribe               🧠 Analyse
─────────────                ────────────────            ────────────────────────
mic.wav   ──┐                Speaker 1: ...              { summary,
            ├──► Whisper ──► Speaker 2: ...  ──► Phi-4 ──  deal_size,
sys.wav   ──┘   STT server  (merged, time-ordered)        sentiment_score,
                                                           next_steps, ... }
                                                                │
                                                    ┌───────────┴───────────┐
                                               ☁️ Upload               🗄️ Insert
                                           S3 / Azure / GCS     Snowflake / BigQuery
                                                                 ClickHouse / Databricks
                                                                 Redshift
```

---

## Quick demo

1. Launch app → genie flies up from the bottom-left corner of your screen
2. Hover → four frosted-glass buttons arc above his head
3. Click **record** → genie nods, timer appears, aura pulses
4. Play a call on your computer + speak into mic → both tracks captured simultaneously
5. Click **stop** → enter the opportunity ID → pipeline runs automatically

---

## Ecosystem

| Component | Location |
|---|---|
| **This app** — Tauri desktop recorder | [github.com/Mikebenisberchmans/Sales-genie-v1-Windows-](https://github.com/Mikebenisberchmans/Sales-genie-v1-Windows-) |
| **Model weights** — fine-tuned Phi-4 | [huggingface.co/Mike-Benis/Salenie-Phi4-v1](https://huggingface.co/Mike-Benis/Salenie-Phi4-v1) |
| **Training pipeline** — notebooks + dataset prep | [github.com/Mikebenisberchmans/Salenie-trainer-pipeline](https://github.com/Mikebenisberchmans/Salenie-trainer-pipeline) |

---

## Complete setup guide

### Prerequisites

| Tool | Install |
|---|---|
| Rust + Cargo | https://rustup.rs |
| Node.js 18+ | https://nodejs.org |
| Python 3.10 or 3.11 | https://python.org |
| Git | https://git-scm.com |
| **Windows only:** Microsoft C++ Build Tools | https://aka.ms/vs/17/release/vs_buildtools.exe |
| **Windows only:** WebView2 | Pre-installed on Windows 11; https://developer.microsoft.com/en-us/microsoft-edge/webview2 for Windows 10 |

---

### Step 1 — Clone the app

```bash
git clone https://github.com/Mikebenisberchmans/Sales-genie-v1-Windows-.git
cd Sales-genie-v1-Windows-
npm install
```

---

### Step 2 — Set up the Python venv (for AI features)

```bash
cd server
python -m venv .venv

# Windows
.venv\Scripts\activate

# macOS / Linux
source .venv/bin/activate
```

Install dependencies for the STT server (always needed):
```bash
pip install -r requirements-stt.txt
```

---

### Step 3 — Get the model

The fine-tuned Phi-4 model is hosted on HuggingFace:
**https://huggingface.co/Mike-Benis/Salenie-Phi4-v1**

You have two options for inference:

#### Option A — Modal (recommended, no GPU required)

Modal runs the model on an A10G cloud GPU. ~$0.001 per call. Setup takes 5 minutes.

```bash
pip install -r requirements-modal.txt
modal setup                      # opens browser to log in / sign up
```

Create your Modal secrets (HF token + API token):
```bash
modal secret create salenie-api-secret \
    HF_TOKEN=hf_YOUR_TOKEN_HERE \
    API_TOKEN=choose_any_strong_secret_string
```

> Get your HF token from https://huggingface.co/settings/tokens (needs read access to the model repo).

Deploy the endpoint:
```bash
modal deploy modal_serve.py
```

Copy the URL that ends in `--salenie-generate.modal.run` — you'll need it in Step 5.

Test it works:
```bash
modal run modal_serve.py
# Should print a JSON object with deal analysis fields
```

#### Option B — Local GPU (requires 8 GB+ VRAM)

```bash
pip install -r requirements-local.txt

# Download model weights (~14 GB):
huggingface-cli login
huggingface-cli download Mike-Benis/Salenie-Phi4-v1 --local-dir ./phi-sales-production

# Start server:
python local_serve.py
# Inference endpoint: http://localhost:8766 (no auth token needed)
```

---

### Step 4 — Run the Genie Recorder app

```bash
# From the repo root:
npm run tauri dev
```

> **First build takes 5–10 minutes** — Cargo is compiling ~200 crates. Every build after that is seconds.

For a production installer (`.msi` + `.exe`):
```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/
```

---

### Step 5 — Configure the app

Click the **⬡ database icon** in the arc controls (hover over the genie to reveal). The config ledger opens in a separate window.

#### Tab 1 — Salesperson
Enter your name and employee ID. These are stamped on every warehouse row.

#### Tab 2 — Object Store
Choose S3, Azure Blob, or GCS. Files are uploaded as:
```
{bucket}/{prefix}/{opp_id}/mic_{timestamp}.wav
{bucket}/{prefix}/{opp_id}/sys_{timestamp}.wav
```

Skip this tab if you only want local files.

#### Tab 3 — Warehouse
Choose your data warehouse. See [Warehouse DDL](#warehouse-ddl) below to create the target table.

Skip this tab if you don't need automatic data insertion.

#### Tab 4 — AI Analysis
| Setting | Value |
|---|---|
| Toggle | **On** |
| Inference Endpoint | `https://YOUR-ORG--salenie-generate.modal.run YOUR-API-TOKEN` *(Modal)* or `http://localhost:8766` *(local)* |
| Whisper Model | `base` *(recommended)* |
| STT Port | `8765` |
| Python Path | Full path to `python.exe` inside your `server/.venv` |
| STT Script | Full path to `server/stt_server.py` |

> **Inference Endpoint format:** URL and API token separated by a single space.
> The Genie Recorder app splits on the space to set the Bearer token header automatically.

Click **Test Services** → waits up to 15 s for Whisper to load → shows green/red dots.

---

## Project structure

```
Sales-genie-v1-Windows-/
│
├── index.html                        Main genie window entry
├── ledger.html                       Config ledger window entry
├── vite.config.js                    Multi-page Vite build
├── package.json
│
├── server/                           ← Python AI servers
│   ├── modal_serve.py                Modal cloud inference (Phi-4 on A10G)
│   ├── local_serve.py                Local GPU inference alternative
│   ├── stt_server.py                 Whisper speech-to-text server
│   ├── requirements-modal.txt        Deps for Modal deployment
│   ├── requirements-stt.txt          Deps for STT server
│   ├── requirements-local.txt        Deps for local GPU inference
│   └── README.md                     Detailed server setup guide
│
├── src/                              ← React frontend
│   ├── App.jsx                       Orchestrator — recording state, pipeline flow
│   ├── styles.css                    GPU-composited CSS animations
│   ├── ledger.jsx                    Config ledger React entry point
│   ├── ledger.css                    Dark-themed ledger styles
│   ├── sounds.js                     TTS disclaimer + UI sounds
│   └── components/
│       ├── Genie.jsx                 Chibi SVG genie (animated layers)
│       ├── OppModal.jsx              Opportunity ID modal on stop
│       └── ConfigLedger.jsx          4-tab config form
│
└── src-tauri/                        ← Rust backend
    ├── Cargo.toml
    ├── tauri.conf.json               Window definitions
    ├── capabilities/default.json     Tauri permissions
    └── src/
        ├── main.rs                   Tauri entrypoint + app lifecycle
        ├── recorder.rs               cpal dual-track WAV recording (WASAPI loopback)
        ├── commands.rs               All Tauri commands exposed to frontend
        ├── config.rs                 Read/write config.json in OS app-data dir
        ├── transcriber.rs            STT subprocess manager + HTTP client
        ├── inference.rs              Modal / local inference HTTP client
        ├── storage.rs                S3 (SigV4) + Azure Blob (Shared Key) + GCS
        └── warehouse.rs              Snowflake, BigQuery, ClickHouse, Databricks, Redshift
```

---

## How dual-track recording works

| Track | Source | Mechanism |
|---|---|---|
| Mic | Default input device | `cpal::build_input_stream` on `default_input_device()` |
| System audio | Default output device | `cpal::build_input_stream` on loopback |

On **Windows**, WASAPI natively supports loopback on the default output device — no virtual driver needed.
On **macOS**, install [BlackHole](https://github.com/ExistentialAudio/BlackHole).

Both streams write to separate WAV files via the `hound` crate. Stopping recording drops both streams (flushes WAV headers) before the upload pipeline starts.

---

## What gets stored in the warehouse

One row is inserted per recording:

| Column | Type | Source |
|---|---|---|
| `opp_id` | TEXT | You enter this in the pop-up modal on stop |
| `submission_date` | TEXT | ISO-8601 UTC timestamp |
| `duration_seconds` | INT | Timer elapsed during recording |
| `salesperson_name` | TEXT | Config ledger |
| `salesperson_id` | TEXT | Config ledger |
| `mic_url` | TEXT | Cloud storage URL of mic WAV |
| `sys_url` | TEXT | Cloud storage URL of system audio WAV |
| `mic_local_path` | TEXT | Local path saved on the machine |
| `sys_local_path` | TEXT | Local path saved on the machine |
| `transcript_text` | TEXT | Full speaker-labelled Whisper output |
| `ai_summary` | TEXT | One-line Phi-4 summary |
| `deal_amount` | FLOAT | Extracted deal size |
| `deal_stage` | TEXT | Extracted deal stage / forecast category |
| `sentiment_score` | FLOAT | 0–1 sentiment score |
| `next_steps` | TEXT | JSON array of action items |
| `full_analysis_json` | TEXT | Complete raw Phi-4 JSON output |

---

## Warehouse DDL

Run this once against your warehouse before first use.

### Snowflake / Databricks / Redshift
```sql
CREATE TABLE IF NOT EXISTS genie_recordings (
    opp_id               VARCHAR,
    submission_date      VARCHAR,
    duration_seconds     INTEGER,
    salesperson_name     VARCHAR,
    salesperson_id       VARCHAR,
    mic_url              VARCHAR,
    sys_url              VARCHAR,
    mic_local_path       VARCHAR,
    sys_local_path       VARCHAR,
    sample_rate          INTEGER,
    channels             INTEGER,
    transcript_text      TEXT,
    ai_summary           TEXT,
    deal_amount          FLOAT,
    deal_company         VARCHAR,
    deal_stage           VARCHAR,
    sentiment_score      FLOAT,
    next_steps           TEXT,
    full_analysis_json   TEXT
);
```

### BigQuery (schema JSON)
```json
[
  {"name": "opp_id",             "type": "STRING"},
  {"name": "submission_date",    "type": "STRING"},
  {"name": "duration_seconds",   "type": "INTEGER"},
  {"name": "salesperson_name",   "type": "STRING"},
  {"name": "salesperson_id",     "type": "STRING"},
  {"name": "mic_url",            "type": "STRING"},
  {"name": "sys_url",            "type": "STRING"},
  {"name": "transcript_text",    "type": "STRING"},
  {"name": "ai_summary",         "type": "STRING"},
  {"name": "deal_amount",        "type": "FLOAT"},
  {"name": "sentiment_score",    "type": "FLOAT"},
  {"name": "next_steps",         "type": "STRING"},
  {"name": "full_analysis_json", "type": "STRING"}
]
```

### ClickHouse
```sql
CREATE TABLE IF NOT EXISTS genie_recordings (
    opp_id               String,
    submission_date      String,
    duration_seconds     Int32,
    salesperson_name     String,
    salesperson_id       String,
    mic_url              String,
    sys_url              String,
    mic_local_path       String,
    sys_local_path       String,
    sample_rate          Int32,
    channels             Int32,
    transcript_text      String,
    ai_summary           String,
    deal_amount          Float64,
    deal_company         String,
    deal_stage           String,
    sentiment_score      Float64,
    next_steps           String,
    full_analysis_json   String
) ENGINE = MergeTree()
ORDER BY (submission_date, opp_id);
```

---

## Animation architecture

All animations run on GPU compositor threads — zero JavaScript or main-thread involvement:

| Element | Animation | Details |
|---|---|---|
| `.genie-wrap` | `fly-in` | `translate3d` + `scale` only, no `rotate` (cheapest GPU path) |
| `.genie-float` | `breathe` | Child element — avoids animation chain conflict with fly-in |
| `.tail` | `tail-wave` | Rotates around tail base |
| `.arms` | `arms-sway` | Gentle rock, ±2° |
| `.head` | `nod` | Only active while recording |
| `.aura` | `pulse` | 2.8 s idle → 1.4 s while recording |

Every animated element has `will-change: transform`, `backface-visibility: hidden`, and lives on its own compositor layer via `isolation: isolate` on the wrapper.

---

## Known limitations

- **Pause is UI-only** — cpal streams keep writing samples during pause. Future: drop + re-create streams on resume.
- **macOS loopback** — requires [BlackHole](https://github.com/ExistentialAudio/BlackHole) virtual audio driver.
- **Modal cold start** — containers spin down after ~2 min of inactivity. First inference after idle takes ~15–20 s to warm up; subsequent calls are ~5 s.
- **Whisper first run** — downloads the model on first use (~74 MB for `base`, ~460 MB for `medium`).

---

## Contributing

Training data, model architecture, and fine-tuning code live in the [Salenie Trainer Pipeline](https://github.com/Mikebenisberchmans/Salenie-trainer-pipeline) repo.

To retrain the model with your own data:
1. Follow the training notebook in that repo
2. Push your fine-tuned weights to HuggingFace Hub
3. Update `HF_REPO` in `server/modal_serve.py`
4. `modal deploy server/modal_serve.py`
