# Salenie — Server Components

This folder contains the Python AI servers that power Genie Recorder's analysis pipeline, plus scripts to deploy them and build the bundled Windows sidecar.

| File | Purpose |
|---|---|
| `modal_serve.py` | **Recommended** — Phi-4 inference on Modal's A10G GPU (serverless, ~5 s/call) |
| `local_serve.py` | **Alternative** — Phi-4 inference on your own NVIDIA GPU (8 GB+ VRAM) |
| `stt_server.py` | Whisper speech-to-text server (runs locally; bundled into the installer) |
| `setup-modal.ps1` | One-click Modal setup wizard — installs, deploys, and writes app config automatically |
| `build_sidecar.ps1` | Builds `stt_server.exe` via PyInstaller for the production installer |
| `stt_server.spec` | PyInstaller spec (onedir mode, no Python runtime needed on end-user machine) |
| `requirements-modal.txt` | Deps for Modal deployment |
| `requirements-stt.txt` | Deps for the STT server |
| `requirements-local.txt` | Deps for local GPU inference |

---

## Prerequisites

- Python 3.10 or 3.11
- A Python virtual environment (strongly recommended)
- For local inference: NVIDIA GPU with 8 GB+ VRAM, CUDA 12.1+
- For Modal inference: a free Modal account (https://modal.com)

> **Installer users:** The STT server is bundled as a self-contained exe inside the installer — you do **not** need Python installed to use the transcription features. You only need Python here to deploy the Modal inference endpoint or to run in developer mode.

---

## STT Server — how it works

`stt_server.py` is a FastAPI server that wraps `faster-whisper`. It accepts two WAV paths, transcribes each separately with speaker labels, then merges the segments into a chronological transcript:

```
POST /transcribe  { mic_path, sys_path, mic_label, sys_label }
→ { transcript: "Speaker 1: Hello...\nSpeaker 2: Hi there..." }

GET /health
→ { status: "ready" }
```

Speaker labels **must** be `"Speaker 1"` (mic) and `"Speaker 2"` (system audio) — these match the exact format used in the Salenie-Phi4-v1 training data.

### Running manually (dev mode)

When you run `npm run tauri dev`, the bundled sidecar is not active. Start the server manually:

```bash
# From this server/ directory:
python -m venv .venv
.venv\Scripts\activate          # Windows
# source .venv/bin/activate     # macOS / Linux

pip install -r requirements-stt.txt
python stt_server.py
# → INFO: Uvicorn running on http://0.0.0.0:8765
```

Keep this terminal open while using the app.

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `WHISPER_MODEL` | `base` | `tiny` / `base` / `small` / `medium` |
| `STT_PORT` | `8765` | Port to listen on |

### Model size guide

| Model | RAM | Speed | Accuracy |
|---|---|---|---|
| `tiny` | ~400 MB | Fastest | Low |
| `base` | ~500 MB | Fast | Good — recommended default |
| `small` | ~1 GB | Moderate | Better |
| `medium` | ~3 GB | Slow | Best |

---

## Building the bundled sidecar (for the installer)

The production installer bundles `stt_server.exe` so end users need no Python installation. Build it with:

```powershell
# From this server/ directory:
powershell -ExecutionPolicy Bypass -File build_sidecar.ps1
```

What it does:
1. Creates/activates a venv, installs `requirements-stt.txt`
2. Runs PyInstaller with `stt_server.spec` (onedir mode — no single-file slowness)
3. Copies `dist/stt_server/` → `../src-tauri/resources/stt_server/`

After running this script, do `npm run tauri build` to produce the final `.msi`/`.exe` installer with the sidecar embedded.

> The `.gitignore` excludes `dist/`, `build/`, and the built `src-tauri/resources/stt_server/stt_server.exe` — every developer needs to run `build_sidecar.ps1` once before building the installer.

---

## Option A — Modal Cloud Inference (recommended)

Modal runs your model on an A10G GPU in the cloud. You pay per second of GPU time used (~$0.001 per typical call). No GPU required on your machine.

### One-click wizard (Windows)

```powershell
powershell -ExecutionPolicy Bypass -File setup-modal.ps1
```

The wizard:
1. Creates a venv and installs `modal`
2. Opens your browser to authenticate with Modal
3. Prompts for your HuggingFace token and a chosen API token
4. Runs `modal deploy modal_serve.py`
5. Extracts the generated endpoint URL from the deploy output
6. Writes both the URL and token directly to `%APPDATA%\com.demo.genierecorder\config.json`

After the wizard completes, open the app and click **Test Services** — everything should be green.

### Manual setup

#### Step 1 — Install Modal CLI

```bash
python -m venv .venv
.venv\Scripts\activate          # Windows
# source .venv/bin/activate     # macOS / Linux

pip install -r requirements-modal.txt
modal setup                     # opens browser to authenticate
```

#### Step 2 — Create Modal secrets

You need two secrets: your HuggingFace token (to download the model) and an API token (to protect your endpoint).

```bash
modal secret create salenie-api-secret \
    HF_TOKEN=hf_YOUR_HUGGINGFACE_TOKEN \
    API_TOKEN=your-chosen-secret-token
```

- **HF_TOKEN**: get from https://huggingface.co/settings/tokens
  - Must have **read** access to `Mike-Benis/Salenie-Phi4-v1`
  - Request access at: https://huggingface.co/Mike-Benis/Salenie-Phi4-v1
- **API_TOKEN**: choose any strong random string — this is the Bearer token the app sends with every request

#### Step 3 — Deploy

```bash
modal deploy modal_serve.py
```

Modal prints two URLs:

```
✓ Created web endpoint for SalenieModel.generate => https://YOUR-ORG--salenie-generate.modal.run
✓ Created web endpoint for SalenieModel.health   => https://YOUR-ORG--salenie-health.modal.run
```

Copy the **generate** URL — you'll paste it into the Genie Recorder config.

#### Step 4 — Test from CLI

```bash
modal run modal_serve.py
# Should print a JSON object with summary, deal_size, sentiment_score, etc.
```

#### Step 5 — Configure Genie Recorder

Open the config ledger (click the ⬡ button) → **AI Analysis** tab:

| Setting | Value |
|---|---|
| Toggle | **On** |
| Inference Endpoint | `https://YOUR-ORG--salenie-generate.modal.run YOUR-API-TOKEN` |
| Whisper Model | `base` (or `small` for better accuracy) |
| STT Port | `8765` |

> **Format:** URL and token separated by a single space.
> Example: `https://acme--salenie-generate.modal.run mysecrettoken123`

---

## Option B — Local GPU Inference

Use this if you have an NVIDIA GPU and prefer to keep everything on-premise.

### Step 1 — Install dependencies

```bash
python -m venv .venv
.venv\Scripts\activate

pip install -r requirements-local.txt
```

> On Windows, install PyTorch with CUDA first:
> ```
> pip install torch --index-url https://download.pytorch.org/whl/cu121
> ```

### Step 2 — Download the model

```bash
huggingface-cli login           # enter your HF token when prompted
huggingface-cli download Mike-Benis/Salenie-Phi4-v1 --local-dir ./phi-sales-production
```

### Step 3 — Start the local inference server

```bash
python local_serve.py
# Server starts at http://localhost:8766
# First load takes ~60 s (model loading into GPU memory)
```

### Step 4 — Configure Genie Recorder

Open config ledger → **AI Analysis** tab:

| Setting | Value |
|---|---|
| Inference Endpoint | `http://localhost:8766` *(no token — just the URL)* |
| Whisper Model | `base` |
| STT Port | `8765` |

---

## How the full pipeline works

```
Genie Recorder (Rust/Tauri)
    │  stop_recording() returns mic.wav + sys.wav
    │
    ▼
stt_server  (port 8765)
    ├─ [installer] bundled stt_server.exe — auto-spawned at app startup
    └─ [dev mode]  python server/stt_server.py — started manually
    │
    │  Transcribes mic.wav  → "Speaker 1: ..." segments
    │  Transcribes sys.wav  → "Speaker 2: ..." segments
    │  Merges chronologically → full transcript string
    │
    ▼
modal_serve.py  OR  local_serve.py
    ├─ Prepends exact training instruction to transcript
    ├─ Runs Salenie-Phi4-v1 fine-tuned model
    └─ Returns structured JSON analysis
    │
    ▼
Genie Recorder
    ├─ Displays transcript + analysis card in the UI
    ├─ Uploads WAVs to S3 / Azure Blob / GCS  (if configured)
    └─ Inserts metadata row to Snowflake / BigQuery / ClickHouse / Databricks / Redshift  (if configured)
```

---

## Troubleshooting

| Problem | Fix |
|---|---|
| Whisper shows red in "Test Services" (installer) | The bundled server may still be loading the model on first run (~15 s). Click Test Services again. Check `%APPDATA%\com.demo.genierecorder\` for error logs. |
| Whisper shows red in "Test Services" (dev mode) | Make sure you started `python server/stt_server.py` in a separate terminal and it shows `Uvicorn running on http://0.0.0.0:8765` |
| `HF_TOKEN` not found / model download fails | Run `modal secret create salenie-api-secret HF_TOKEN=hf_... API_TOKEN=...` (include both keys — `--force` replaces the entire secret) |
| `401 Unauthorized` from inference endpoint | The API_TOKEN in your Modal secret must match the token after the space in your Inference Endpoint config field |
| `503 Model still loading` | Wait ~20 s after the first deploy; container is cold-starting. Subsequent calls are ~5 s. |
| Modal container cold start (~15-20 s delay) | Normal behaviour — Modal spins down containers after ~2 min of inactivity. The model is cached in a Modal Volume so it doesn't re-download. |
| Local inference OOM | Use a smaller model or switch to Modal cloud |
| `faster-whisper` install fails on Windows | Install `ctranslate2` wheel manually from https://github.com/OpenNMT/CTranslate2/releases |
| `stt_server.exe` missing (build from source) | Run `server/build_sidecar.ps1` to build it via PyInstaller before running `npm run tauri build` |

---

## Related repositories

| Repo | Description |
|---|---|
| [Genie Recorder](https://github.com/Mikebenisberchmans/Sales-genie-v1-Windows-) | This app — Tauri desktop recorder |
| [Salenie Trainer Pipeline](https://github.com/Mikebenisberchmans/Salenie-trainer-pipeline) | Fine-tuning notebooks, dataset preparation, training scripts |
| [Salenie-Phi4-v1](https://huggingface.co/Mike-Benis/Salenie-Phi4-v1) | Fine-tuned model weights on HuggingFace Hub |
