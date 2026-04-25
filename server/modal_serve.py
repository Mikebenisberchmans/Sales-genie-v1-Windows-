"""
modal_serve.py — Salenie Phi-4 inference endpoint (Modal serverless GPU)
=========================================================================

Serves the fine-tuned Phi-4 sales analysis model on Modal's A10G GPU.
Model weights are pulled from HuggingFace Hub on first cold start, then
cached in a Modal Volume — subsequent starts are instant (~10 s).

Setup (one-time)
----------------
1.  pip install modal
2.  modal setup                        # authenticates your Modal account
3.  modal secret create salenie-api-secret \\
        HF_TOKEN=hf_xxxx \\
        API_TOKEN=your-chosen-secret-token
4.  modal deploy modal_serve.py        # deploys to Modal cloud

Test from CLI
-------------
    modal run modal_serve.py           # runs the @app.local_entrypoint test

Endpoints (auto-generated URLs shown after deploy)
---------------------------------------------------
    POST  https://<org>--salenie-generate.modal.run
          Headers:  Authorization: Bearer <API_TOKEN>
          Body:     { "prompt": "<raw transcript>",
                      "options": { "num_predict": 800, "temperature": 0.1 } }
          Returns:  { "response": "<json string>", "done": true }

    GET   https://<org>--salenie-health.modal.run
          Returns:  { "status": "ready" }

Model
-----
    HuggingFace: Mike-Benis/Salenie-Phi4-v1
    Training:    https://github.com/Mikebenisberchmans/Salenie-trainer-pipeline
"""

import modal
from fastapi import Request as FastAPIRequest
from fastapi.responses import JSONResponse

# ---------------------------------------------------------------------------
# App + infrastructure
# ---------------------------------------------------------------------------

app = modal.App("salenie-phi")

# Persistent volume — model is downloaded once and reused across all containers
model_volume = modal.Volume.from_name("salenie-phi-weights", create_if_missing=True)
MODEL_CACHE  = "/model-cache"

HF_REPO = "Mike-Benis/Salenie-Phi4-v1"

image = (
    modal.Image.debian_slim(python_version="3.11")
    .pip_install(
        "torch>=2.4.0",
        "transformers==4.53.3",   # must match training version exactly
        "accelerate>=0.30.0",
        "bitsandbytes>=0.43.0",
        "huggingface_hub>=0.21.0",
        "safetensors",
        "fastapi[standard]",
    )
)


# ---------------------------------------------------------------------------
# Model class
# ---------------------------------------------------------------------------

@app.cls(
    gpu="A10G",
    image=image,
    volumes={MODEL_CACHE: model_volume},
    timeout=600,
    scaledown_window=120,   # keep container warm for 2 min after last request
    secrets=[modal.Secret.from_name("salenie-api-secret")],
)
class SalenieModel:

    @modal.enter()
    def load(self):
        """Download model from HF Hub (first run only) then load into GPU memory."""
        import os, torch
        from pathlib import Path
        from huggingface_hub import snapshot_download
        from transformers import AutoModelForCausalLM, AutoTokenizer

        model_dir = Path(MODEL_CACHE) / "phi-sales-production"

        if not (model_dir / "config.json").exists():
            print(f"[salenie] Downloading {HF_REPO} from HuggingFace Hub …")
            snapshot_download(
                repo_id=HF_REPO,
                local_dir=str(model_dir),
                token=os.environ["HF_TOKEN"],
                ignore_patterns=["*.msgpack", "*.h5", "flax_*"],
            )
            model_volume.commit()
            print("[salenie] Download complete — cached in volume.")
        else:
            print("[salenie] Model found in volume — skipping download.")

        print("[salenie] Loading model into GPU memory …")
        self.tokenizer = AutoTokenizer.from_pretrained(
            str(model_dir), trust_remote_code=True
        )
        self.model = AutoModelForCausalLM.from_pretrained(
            str(model_dir),
            torch_dtype=torch.bfloat16,   # bfloat16 matches training dtype
            device_map="cuda",
            trust_remote_code=True,
        )
        self.model.eval()
        print("[salenie] Model ready.")

    # ------------------------------------------------------------------
    # Instruction — must be IDENTICAL to what was used during fine-tuning.
    # Do NOT modify this string.
    # ------------------------------------------------------------------
    _INSTRUCTION = (
        "Analyze the following raw transcript and extract data according to the exact JSON schema "
        "for summary, deal metadata, qualification metrics, sentiment analysis, and next steps."
        "ONLY extract entities explicitly mentioned in the transcript. "
        "If a competitor,pain point or any other entity is not stated, do not guess."
    )

    def _generate(self, transcript: str, max_new: int, temperature: float) -> str:
        """Format prompt exactly as during training, run inference, return clean text."""
        import torch, re

        # Prompt format must match training template exactly
        formatted = (
            f"<|user|>\n{self._INSTRUCTION}\n\n"
            f"Transcript:\n{transcript}<|end|>\n<|assistant|>\n"
        )
        inputs = self.tokenizer(formatted, return_tensors="pt").to(self.model.device)

        with torch.no_grad():
            output = self.model.generate(
                **inputs,
                max_new_tokens=max_new,
                temperature=temperature,
                do_sample=temperature > 0,
                repetition_penalty=1.1,
                pad_token_id=self.tokenizer.eos_token_id,
                eos_token_id=[
                    self.tokenizer.convert_tokens_to_ids("<|end|>"),
                    self.tokenizer.eos_token_id,
                ],
            )

        input_len = inputs.input_ids.shape[1]
        text = self.tokenizer.decode(output[0][input_len:], skip_special_tokens=True)
        return re.sub(r"<\|.*?\|>", "", text).strip()

    # ------------------------------------------------------------------
    # Callable method — used by the test entrypoint below
    # ------------------------------------------------------------------
    @modal.method()
    def run(self, transcript: str, max_new: int = 800, temperature: float = 0.1) -> dict:
        """Direct method call (for CLI testing via modal run)."""
        return {"response": self._generate(transcript, max_new, temperature), "done": True}

    # ------------------------------------------------------------------
    # HTTP endpoints
    # ------------------------------------------------------------------
    @modal.fastapi_endpoint(method="GET", label="salenie-health")
    def health(self):
        return {"status": "ready"}

    @modal.fastapi_endpoint(method="POST", label="salenie-generate")
    async def generate(self, req: FastAPIRequest):
        import os

        # Verify Bearer token if API_TOKEN secret is set
        token = os.environ.get("API_TOKEN", "")
        if token:
            auth = req.headers.get("authorization", "")
            if auth != f"Bearer {token}":
                return JSONResponse({"error": "unauthorized"}, status_code=401)

        body        = await req.json()
        transcript  = body.get("prompt", "")   # "prompt" key for API compatibility
        options     = body.get("options", {})
        max_new     = int(options.get("num_predict", 800))
        temperature = float(options.get("temperature", 0.1))

        if not transcript:
            return JSONResponse({"error": "prompt is required"}, status_code=400)

        result = self._generate(transcript, max_new, temperature)
        return {"response": result, "done": True}


# ---------------------------------------------------------------------------
# CLI test entrypoint — run with:  modal run modal_serve.py
# ---------------------------------------------------------------------------
@app.local_entrypoint()
def test():
    transcript = """Speaker 1: Can you hear me okay?
Speaker 2: Yes, loud and clear.
Speaker 1: Great. So just to confirm — you mentioned the budget is locked at $45,000?
Speaker 2: That's right, finance cleared it yesterday. We need this live in 4 weeks for the Q3 board meeting.
Speaker 1: Perfect. I'll send the implementation timeline by tomorrow morning."""

    m = SalenieModel()
    result = m.run.remote(transcript, max_new=800, temperature=0.1)
    print(result)
