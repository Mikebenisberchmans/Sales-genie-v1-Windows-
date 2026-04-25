"""
stt_server.py — Salenie Speech-to-Text Server
==============================================

Local FastAPI server that transcribes two audio files (mic + system audio)
into a merged, speaker-labelled transcript using faster-whisper.

The Genie Recorder app spawns this process automatically when AI Analysis
is enabled in the config ledger. You can also run it manually for testing.

Usage
-----
    python stt_server.py                   # defaults: model=base, port=8765
    WHISPER_MODEL=small STT_PORT=9000 python stt_server.py

Environment variables
---------------------
    WHISPER_MODEL   Whisper model size: tiny | base | small | medium (default: base)
    STT_PORT        Port to listen on (default: 8765)

API
---
    GET  /health
         Returns: { "status": "ready"|"loading", "model": "<size>" }

    POST /transcribe
         Body: {
           "mic_path": "/path/to/mic.wav",
           "sys_path": "/path/to/system.wav",
           "mic_label": "Speaker 1",        (optional, default: "Speaker 1")
           "sys_label": "Speaker 2"         (optional, default: "Speaker 2")
         }
         Returns: {
           "transcript": "Speaker 1: ...\nSpeaker 2: ...",
           "mic_words": 120,
           "sys_words": 98
         }

Speaker labels
--------------
    The default labels ("Speaker 1" / "Speaker 2") MUST match the format
    used during model fine-tuning. Do not change them unless you retrained
    the model with different labels.
"""

import os
import sys
import logging
from contextlib import asynccontextmanager
from typing import Optional

import uvicorn
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [STT] %(levelname)s %(message)s",
    stream=sys.stdout,
)
log = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Global model state
# ---------------------------------------------------------------------------
whisper_model = None
model_ready   = False


def _has_cuda() -> bool:
    try:
        import torch
        return torch.cuda.is_available()
    except ImportError:
        return False


def load_whisper():
    global whisper_model, model_ready
    from faster_whisper import WhisperModel

    model_size   = os.environ.get("WHISPER_MODEL", "base")
    device       = "cuda" if _has_cuda() else "cpu"
    compute_type = "float16" if device == "cuda" else "int8"

    log.info(f"Loading Whisper '{model_size}' on {device} ({compute_type}) …")
    whisper_model = WhisperModel(model_size, device=device, compute_type=compute_type)
    model_ready   = True
    log.info("Whisper model ready.")


@asynccontextmanager
async def lifespan(app: FastAPI):
    import asyncio
    loop = asyncio.get_event_loop()
    await loop.run_in_executor(None, load_whisper)   # load in thread, don't block event loop
    yield
    global whisper_model
    whisper_model = None


app = FastAPI(title="Salenie STT Server", lifespan=lifespan)


# ---------------------------------------------------------------------------
# Request / response models
# ---------------------------------------------------------------------------
class TranscribeRequest(BaseModel):
    mic_path:  str
    sys_path:  str
    mic_label: Optional[str] = "Speaker 1"   # matches training data format
    sys_label: Optional[str] = "Speaker 2"   # matches training data format


class TranscribeResponse(BaseModel):
    transcript: str
    mic_words:  int
    sys_words:  int


# ---------------------------------------------------------------------------
# Endpoints
# ---------------------------------------------------------------------------
@app.get("/health")
def health():
    return {
        "status": "ready" if model_ready else "loading",
        "model":  os.environ.get("WHISPER_MODEL", "base"),
    }


@app.post("/transcribe", response_model=TranscribeResponse)
def transcribe(req: TranscribeRequest):
    if not model_ready or whisper_model is None:
        raise HTTPException(status_code=503, detail="Whisper model not ready yet")

    if not os.path.exists(req.mic_path):
        raise HTTPException(status_code=400, detail=f"mic_path not found: {req.mic_path}")
    if not os.path.exists(req.sys_path):
        raise HTTPException(status_code=400, detail=f"sys_path not found: {req.sys_path}")

    log.info(f"Transcribing mic:  {req.mic_path}")
    mic_segments, _ = whisper_model.transcribe(
        req.mic_path,
        beam_size=5,
        vad_filter=True,
        vad_parameters={"min_silence_duration_ms": 500},
    )
    mic_items = [
        (seg.start, seg.end, seg.text.strip(), req.mic_label)
        for seg in mic_segments
        if seg.text.strip()
    ]

    log.info(f"Transcribing sys:  {req.sys_path}")
    sys_segments, _ = whisper_model.transcribe(
        req.sys_path,
        beam_size=5,
        vad_filter=True,
        vad_parameters={"min_silence_duration_ms": 500},
    )
    sys_items = [
        (seg.start, seg.end, seg.text.strip(), req.sys_label)
        for seg in sys_segments
        if seg.text.strip()
    ]

    # Merge chronologically by segment start time
    all_items = sorted(mic_items + sys_items, key=lambda x: x[0])

    # Collapse consecutive same-speaker segments into one turn
    merged: list[tuple[str, str]] = []
    for _, _, text, speaker in all_items:
        if merged and merged[-1][0] == speaker:
            merged[-1] = (speaker, merged[-1][1] + " " + text)
        else:
            merged.append([speaker, text])

    transcript = "\n".join(f"{speaker}: {text}" for speaker, text in merged)
    mic_words  = sum(len(t.split()) for _, _, t, _ in mic_items)
    sys_words  = sum(len(t.split()) for _, _, t, _ in sys_items)

    log.info(f"Done — mic={mic_words} words, sys={sys_words} words, turns={len(merged)}")
    return TranscribeResponse(transcript=transcript, mic_words=mic_words, sys_words=sys_words)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------
if __name__ == "__main__":
    port = int(os.environ.get("STT_PORT", "8765"))
    log.info(f"Starting Salenie STT Server on port {port} …")
    uvicorn.run(app, host="127.0.0.1", port=port, log_level="warning")
