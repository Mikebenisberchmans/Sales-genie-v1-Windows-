# Salenie — Server Components

This folder contains the two Python servers that power Genie Recorder's AI pipeline.

| File | Purpose |
|---|---|
| `modal_serve.py` | **Recommended** — Phi-4 inference on Modal's A10G GPU (serverless, ~5 s/call) |
| `local_serve.py` | **Alternative** — Phi-4 inference on your own NVIDIA GPU (8 GB+ VRAM) |
| `stt_server.py` | Whisper speech-to-text server (runs locally, required for both paths) |
| `requirements-modal.txt` | Deps for Modal deployment |
| `requirements-stt.txt` | Deps for the STT server |
| `requirements-local.txt` | Deps for local GPU inference |

---

## Prerequisites

- Python 3.10 or 3.11
- A Python virtual environment (strongly recommended)
- For local inference: NVIDIA GPU with 8 GB+ VRAM, CUDA 12.1+
- For Modal inference: a free Modal account (https://modal.com)

---

## Option A — Modal Cloud Inference (recommended)

Modal runs your model on an A10G GPU in the cloud. You pay per second of GPU
time used (~$0.001 per typical call). No GPU required on your machine.

### Step 1 — Install Modal CLI

```bash
python -m venv .venv
.venv\Scripts\activate          # Windows
# source .venv/bin/activate     # macOS / Linux

pip install -r requirements-modal.txt
modal setup                     # opens browser to authenticate
```

### Step 2 — Create Modal secrets

You need two secrets: your HuggingFace token (to download the private model)
and an API token (to protect your inference endpoint).

```bash
modal secret create salenie-api-secret \
    HF_TOKEN=hf_YOUR_HUGGINGFACE_TOKEN \
    API_TOKEN=your-chosen-secret-token
```

- **HF_TOKEN**: get from https://huggingface.co/settings/tokens
  - Must have **read** access to `Mike-Benis/Salenie-Phi4-v1`
  - Request access at: https://huggingface.co/Mike-Benis/Salenie-Phi4-v1
- **API_TOKEN**: choose any strong random string — this is the Bearer token the
  Genie Recorder app will send with every request

### Step 3 — Deploy

```bash
modal deploy modal_serve.py
```

After deployment, Modal prints two URLs:

```
✓ Created web endpoint for SalenieModel.generate => https://YOUR-ORG--salenie-generate.modal.run
✓ Created web endpoint for SalenieModel.health   => https://YOUR-ORG--salenie-health.modal.run
```

Copy the **generate** URL — you'll paste it into the Genie Recorder config.

### Step 4 — Test from CLI

```bash
modal run modal_serve.py
```

You should see a JSON object with `summary`, `deal_size`, `sentiment_score`, etc.

### Step 5 — Configure Genie Recorder

Open the config ledger (click the ⬡ button) → **AI Analysis** tab:

| Setting | Value |
|---|---|
| Toggle | **On** |
| Inference Endpoint | `https://YOUR-ORG--salenie-generate.modal.run YOUR-API-TOKEN` |
| Whisper Model | `base` (or `small` for better accuracy) |
| STT Port | `8765` |
| Python Path | path to `python.exe` in your venv |
| STT Script | path to `stt_server.py` |

> **Format for Inference Endpoint:** URL and token separated by a single space.
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
| Inference Endpoint | `http://localhost:8766` (no token needed for local) |
| STT Port | `8765` |
| Python Path | path to your venv `python.exe` |
| STT Script | path to `stt_server.py` |

---

## STT Server (required for both options)

The Whisper STT server must be running for AI analysis to work. The Genie
Recorder app auto-spawns it at startup if AI Analysis is enabled and the
Python/script paths are configured. You can also run it manually:

### Install

```bash
pip install -r requirements-stt.txt
```

### Run

```bash
python stt_server.py
# Whisper 'base' model loads in ~5 s (downloads ~74 MB on first run)
```

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `WHISPER_MODEL` | `base` | `tiny` / `base` / `small` / `medium` |
| `STT_PORT` | `8765` | Port to listen on |

### Model size guide

| Model | VRAM / RAM | Speed | Accuracy |
|---|---|---|---|
| `tiny` | ~400 MB | Fastest | Low |
| `base` | ~500 MB | Fast | Good — recommended default |
| `small` | ~1 GB | Moderate | Better |
| `medium` | ~3 GB | Slow | Best |

---

## How the pipeline works

```
Genie Recorder (Rust/Tauri)
    │  stop_recording() returns mic.wav + sys.wav
    │
    ▼
stt_server.py  (port 8765)
    ├─ Transcribes mic.wav  → "Speaker 1: ..." segments
    ├─ Transcribes sys.wav  → "Speaker 2: ..." segments
    └─ Merges chronologically → full transcript string
    │
    ▼
modal_serve.py  OR  local_serve.py
    ├─ Prepends exact training instruction to transcript
    ├─ Runs Phi-4 fine-tuned model
    └─ Returns structured JSON analysis
    │
    ▼
Genie Recorder
    ├─ Uploads WAVs to S3 / Azure Blob / GCS
    └─ Inserts metadata row to Snowflake / BigQuery / ClickHouse / Databricks / Redshift
```

---

## Troubleshooting

| Problem | Fix |
|---|---|
| Whisper shows red in "Test Services" | Make sure Python Path and STT Script are filled in the config ledger, then click Test Services again — it auto-spawns and waits up to 15 s for the model to load |
| `HF_TOKEN` not found | Run `modal secret create salenie-api-secret HF_TOKEN=...` |
| `401 unauthorized` | Check the API_TOKEN in your Modal secret matches the token in the Inference Endpoint field |
| `503 Model still loading` | Wait ~20 s after the first deploy; container is cold-starting |
| Local inference OOM | Use a smaller model or switch to Modal cloud |
| `faster-whisper` install fails on Windows | Install `ctranslate2` wheel manually from https://github.com/OpenNMT/CTranslate2/releases |

---

## Related repositories

| Repo | Description |
|---|---|
| [Genie Recorder](https://github.com/Mikebenisberchmans/Sales-genie-v1-Windows-) | This app — Tauri desktop recorder |
| [Salenie Trainer Pipeline](https://github.com/Mikebenisberchmans/Salenie-trainer-pipeline) | Fine-tuning notebooks, dataset preparation, training scripts |
| [Salenie-Phi4-v1](https://huggingface.co/Mike-Benis/Salenie-Phi4-v1) | Fine-tuned model weights on HuggingFace Hub |
