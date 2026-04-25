"""
local_serve.py — Salenie Phi-4 Local Inference Server (no cloud required)
=========================================================================

Alternative to modal_serve.py for users who have a local NVIDIA GPU
(8 GB+ VRAM recommended). Loads the model with 4-bit NF4 quantisation
via BitsAndBytes so it fits on a consumer GPU.

Mirrors the same HTTP API as the Modal endpoint so the Genie Recorder
app works without any code changes — just point the Inference Endpoint
setting to http://localhost:8766.

Setup
-----
    # In your Python venv (requires CUDA):
    pip install -r requirements-local.txt

    # Download model from HuggingFace:
    huggingface-cli login          # enter your HF token
    huggingface-cli download Mike-Benis/Salenie-Phi4-v1 --local-dir ./phi-sales-production

    # Start server:
    python local_serve.py

    # Or with custom paths/ports:
    MODEL_PATH=./phi-sales-production INFER_PORT=8766 python local_serve.py

Environment variables
---------------------
    MODEL_PATH    Path to the model directory (default: ./phi-sales-production)
    INFER_PORT    Port to listen on (default: 8766)

API
---
    POST /
         Headers:  (no auth required for local)
         Body:     { "prompt": "<raw transcript>",
                     "options": { "num_predict": 800, "temperature": 0.1 } }
         Returns:  { "response": "<json string>", "done": true }

    GET  /health
         Returns:  { "status": "ready"|"loading" }

Note
----
    CPU inference is supported but extremely slow (~10 min per call).
    A GPU with 8 GB+ VRAM gives ~30 s inference time.
    For production use, prefer modal_serve.py (Modal cloud, ~5 s on A10G).
"""

import os
import re
import asyncio
import threading
from contextlib import asynccontextmanager
from pathlib import Path

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig
from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel
import uvicorn

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------
MODEL_PATH = os.environ.get("MODEL_PATH", str(Path(__file__).parent / "phi-sales-production"))
PORT       = int(os.environ.get("INFER_PORT", "8766"))

# ---------------------------------------------------------------------------
# Global model state
# ---------------------------------------------------------------------------
_model     = None
_tokenizer = None
_ready     = False
_lock      = threading.Lock()

# Instruction must be IDENTICAL to what was used during fine-tuning
_INSTRUCTION = (
    "Analyze the following raw transcript and extract data according to the exact JSON schema "
    "for summary, deal metadata, qualification metrics, sentiment analysis, and next steps."
    "ONLY extract entities explicitly mentioned in the transcript. "
    "If a competitor,pain point or any other entity is not stated, do not guess."
)


def _load_model():
    global _model, _tokenizer, _ready
    with _lock:
        if _ready:
            return

        print(f"[local_serve] Loading tokenizer from {MODEL_PATH} …")
        _tokenizer = AutoTokenizer.from_pretrained(MODEL_PATH, trust_remote_code=True)

        use_cuda = torch.cuda.is_available()
        print(f"[local_serve] CUDA available: {use_cuda}")

        if use_cuda:
            # 4-bit NF4 quantisation — matches training quantisation exactly
            bnb_config = BitsAndBytesConfig(
                load_in_4bit=True,
                bnb_4bit_quant_type="nf4",
                bnb_4bit_compute_dtype=torch.bfloat16,
            )
            print("[local_serve] Loading model with BitsAndBytes NF4 on GPU …")
            _model = AutoModelForCausalLM.from_pretrained(
                MODEL_PATH,
                quantization_config=bnb_config,
                device_map={"": 0},
                torch_dtype=torch.bfloat16,
                trust_remote_code=True,
            )
        else:
            # CPU fallback — slow but functional
            print("[local_serve] No CUDA — loading on CPU (inference will be slow) …")
            _model = AutoModelForCausalLM.from_pretrained(
                MODEL_PATH,
                torch_dtype=torch.float32,
                device_map="cpu",
                trust_remote_code=True,
            )

        _model.eval()
        _ready = True
        print("[local_serve] Model ready.")


@asynccontextmanager
async def lifespan(app: FastAPI):
    loop = asyncio.get_event_loop()
    await loop.run_in_executor(None, _load_model)
    yield


app = FastAPI(title="Salenie Local Inference Server", lifespan=lifespan)


# ---------------------------------------------------------------------------
# Endpoints
# ---------------------------------------------------------------------------
@app.get("/health")
def health():
    return {"status": "ready" if _ready else "loading"}


class GenerateRequest(BaseModel):
    prompt:  str
    options: dict = {}


@app.post("/")
async def generate(req: GenerateRequest):
    if not _ready:
        raise HTTPException(status_code=503, detail="Model still loading")

    transcript  = req.prompt
    max_new     = int(req.options.get("num_predict", 800))
    temperature = float(req.options.get("temperature", 0.1))

    # Prompt format must match training template exactly
    formatted = (
        f"<|user|>\n{_INSTRUCTION}\n\n"
        f"Transcript:\n{transcript}<|end|>\n<|assistant|>\n"
    )
    inputs = _tokenizer(formatted, return_tensors="pt").to(_model.device)

    loop = asyncio.get_event_loop()

    def _run():
        with torch.no_grad():
            output = _model.generate(
                **inputs,
                max_new_tokens=max_new,
                temperature=temperature,
                do_sample=temperature > 0,
                repetition_penalty=1.1,
                pad_token_id=_tokenizer.eos_token_id,
                eos_token_id=[
                    _tokenizer.convert_tokens_to_ids("<|end|>"),
                    _tokenizer.eos_token_id,
                ],
            )
        input_len = inputs.input_ids.shape[1]
        text = _tokenizer.decode(output[0][input_len:], skip_special_tokens=True)
        return re.sub(r"<\|.*?\|>", "", text).strip()

    result = await loop.run_in_executor(None, _run)
    return {"response": result, "done": True}


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------
if __name__ == "__main__":
    print(f"[local_serve] Starting on http://localhost:{PORT}")
    print(f"[local_serve] Model path: {MODEL_PATH}")
    uvicorn.run(app, host="127.0.0.1", port=PORT, log_level="warning")
