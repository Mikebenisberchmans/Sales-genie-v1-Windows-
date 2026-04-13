# 🧞 Genie Recorder

A lightweight Tauri + Rust desktop app that records **two separate audio tracks** simultaneously — your microphone AND your system audio (whatever's playing through speakers/headphones). Designed for sales call recording where you need the rep's voice and the customer's voice as independent files (perfect for speaker-labeled transcription downstream).

A friendly green genie sits at the left edge of your screen. Hover over him to reveal record/pause/stop buttons. He nods his head while listening.

## Features

- 🎤 Records mic + system audio as **two separate WAV files**
- 🪟 Transparent, frameless, always-on-top window — sits at the left screen edge
- 🧞 Animated SVG genie: flies up from below on launch, idle float, head-nod while recording
- 🎬 Hover-to-reveal controls slide out from behind the genie
- 💾 Files saved to `~/Music/GenieRecordings/mic_TIMESTAMP.wav` and `system_TIMESTAMP.wav`
- 📦 Tiny binary (~10 MB) thanks to Tauri
- 🖥️ Cross-platform-ready: Windows works out of the box, Linux works with PipeWire/PulseAudio, macOS needs BlackHole virtual device (see notes)

## Prerequisites (one-time install on your machine)

1. **Rust** — https://rustup.rs
2. **Node.js 18+** — https://nodejs.org
3. **Windows:** Microsoft C++ Build Tools + WebView2 (Win 11 has it pre-installed)
4. **Linux:** `sudo apt install libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`
5. **macOS:** Xcode Command Line Tools (`xcode-select --install`)

## Run in dev mode

```bash
npm install
npm run tauri dev
```

First build will take **5–10 minutes** as Cargo compiles all dependencies. After that, incremental builds are seconds.

## Build a release binary

```bash
npm run tauri build
```

Output: `src-tauri/target/release/bundle/` — installer for your OS.

## Project structure

```
genie-recorder/
├── package.json
├── vite.config.js
├── index.html
├── src/                          ← React frontend
│   ├── main.jsx
│   ├── App.jsx                   ← orchestrator + hover controls
│   ├── styles.css                ← all animations (entry, float, nod, hover-reveal)
│   └── components/
│       └── Genie.jsx             ← inline SVG green genie
└── src-tauri/                    ← Rust backend
    ├── Cargo.toml
    ├── tauri.conf.json           ← transparent + frameless + always-on-top window
    ├── build.rs
    ├── capabilities/
    │   └── default.json
    └── src/
        ├── main.rs               ← Tauri entrypoint
        ├── recorder.rs           ← cpal mic + system streams → WAV via hound
        └── commands.rs           ← #[tauri::command] start/stop/is_recording
```

## How dual-track recording works

| Track | Source | cpal call |
|---|---|---|
| Mic | `default_input_device()` | `build_input_stream` |
| System | `default_output_device()` | `build_input_stream` (loopback) |

On **Windows**, cpal uses WASAPI which natively supports loopback on the default output device — no virtual driver needed. On **Linux**, PulseAudio/PipeWire monitor sources work the same way. On **macOS**, you'll need to install [BlackHole](https://github.com/ExistentialAudio/BlackHole) and route system audio through it (Apple doesn't allow direct loopback).

Both streams write to separate WAV files via the `hound` crate. When you click Stop, the streams are dropped (which flushes and closes the files), and a dialog shows you both file paths.

## Animation details

The genie's entry animation uses a `cubic-bezier(0.34, 1.56, 0.64, 1)` curve to create the overshoot-and-settle effect:

```
0%   → off-screen below (translateY: 120%)
40%  → overshoots above resting position (translateY: -25%)
65%  → bounces back down (translateY: 8%)
100% → settles at resting position
```

While idle: 3-second floating bob. While recording: 1.1-second head nod + faster aura pulse.

## Known limitations / Phase 2 ideas

- **Pause is UI-only** — cpal streams can't truly pause; v2 should drop and re-create streams on resume to actually stop writing samples
- **No mixed track output** — we save mic and system separately. Mixing would be trivial via `hound` if you also want a single combined WAV
- **Device hot-swap** — if the rep unplugs headphones mid-call, the loopback may break. v2 should detect device changes and re-attach
- **macOS loopback** — needs BlackHole; document this in your demo
- **Icons** — `tauri.conf.json` references `icons/icon.png`; for the demo you can comment out the bundle section or drop any PNG in `src-tauri/icons/`

## How to demo this to your manager

1. Launch app → genie flies up from bottom, settles at left edge
2. Hover → record button slides out
3. Click record → genie starts nodding, timer appears, aura pulses faster
4. Play a YouTube video + speak into mic → both are being captured simultaneously
5. Click stop → dialog shows two saved file paths, file explorer opens to the folder
6. Open both WAVs → the system audio file has only the YouTube audio, the mic file has only your voice

That's the "wow" moment: **two clean tracks, one click**. Tell your manager: "Now imagine running Whisper on each track separately — you get a perfect speaker-labeled transcript for free, no diarization model needed."
