# stt_server.spec — PyInstaller spec for the Salenie STT server
#
# Packages stt_server.py + faster-whisper + FastAPI + Uvicorn into a
# single Windows executable with NO Python installation required.
#
# Build command (run from inside server/ with venv active):
#   pyinstaller stt_server.spec
#
# Output: server/dist/stt_server/stt_server.exe
# Then run build_sidecar.ps1 to copy it to the right place.

block_cipher = None

a = Analysis(
    ["stt_server.py"],
    pathex=["."],
    binaries=[],
    datas=[],
    hiddenimports=[
        # ctranslate2 (faster-whisper backend) — must be explicit
        "ctranslate2",
        "ctranslate2.extensions",
        # faster-whisper internals
        "faster_whisper",
        "faster_whisper.transcribe",
        "faster_whisper.audio",
        "faster_whisper.vad",
        "faster_whisper.tokenizer",
        # HuggingFace Hub (model download on first run)
        "huggingface_hub",
        "huggingface_hub.file_download",
        # ASGI / FastAPI
        "uvicorn",
        "uvicorn.logging",
        "uvicorn.loops",
        "uvicorn.loops.auto",
        "uvicorn.protocols",
        "uvicorn.protocols.http",
        "uvicorn.protocols.http.auto",
        "uvicorn.protocols.websockets",
        "uvicorn.protocols.websockets.auto",
        "uvicorn.lifespan",
        "uvicorn.lifespan.on",
        "fastapi",
        "anyio",
        "anyio.lowlevel",
        "anyio._backends._asyncio",
        "starlette",
        "starlette.routing",
        "starlette.middleware",
        # tokenizers (Rust extension — needs explicit mention)
        "tokenizers",
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        # Exclude heavy ML packages not needed for STT
        "torch",
        "torchvision",
        "transformers",
        "tensorflow",
        "sklearn",
        "matplotlib",
        "PIL",
        "cv2",
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="stt_server",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,           # compress with UPX if available
    console=False,      # no console window — runs silently in background
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name="stt_server",
)
