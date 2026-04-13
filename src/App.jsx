import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { message } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import Genie from "./components/Genie.jsx";
import { playStartSound, playPauseSound, playStopSound, playDisclaimer } from "./sounds";

const DISCLAIMER_TEXT = "This call is being recorded for quality and training purposes.";

export default function App() {
  const [recording, setRecording] = useState(false);
  const [paused, setPaused] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [entered, setEntered] = useState(false);

  useEffect(() => {
    const t = setTimeout(() => setEntered(true), 150);
    return () => clearTimeout(t);
  }, []);

  useEffect(() => {
    if (!recording || paused) return;
    const id = setInterval(() => setElapsed((e) => e + 1), 1000);
    return () => clearInterval(id);
  }, [recording, paused]);

  const handleStart = async () => {
    try {
      await invoke("start_recording");
      setRecording(true);
      setPaused(false);
      setElapsed(0);
      playStartSound();
    } catch (e) {
      await message("Failed to start: " + e, { title: "Error", kind: "error" });
    }
  };

  const handlePauseToggle = () => {
    if (paused) {
      playStartSound();
    } else {
      playPauseSound();
    }
    setPaused((p) => !p);
  };

  const handleStop = async () => {
    try {
      playStopSound();
      const res = await invoke("stop_recording");
      setRecording(false);
      setPaused(false);
      await message(
        `Two files saved:\n\n🎤 Mic:\n${res.mic_path}\n\n🔊 System:\n${res.sys_path}`,
        { title: "Recording saved", kind: "info" }
      );
      try { await revealItemInDir(res.mic_path); } catch {}
    } catch (e) {
      await message("Stop failed: " + e, { title: "Error", kind: "error" });
    }
  };

  const handleDisclaimer = () => playDisclaimer(DISCLAIMER_TEXT);

  const fmt = (s) =>
    `${Math.floor(s / 60).toString().padStart(2, "0")}:${(s % 60).toString().padStart(2, "0")}`;

  return (
    <div
      className={`stage ${entered ? "entered" : ""}`}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div className={`genie-wrap ${recording ? "recording" : ""}`}>
        <div className={`controls ${hovered ? "show" : ""}`}>
          {/* Always-available disclaimer / consent button */}
          <button className="btn btn-consent" onClick={handleDisclaimer} title="Play recording disclaimer">
            <svg viewBox="0 0 24 24" width="14" height="14">
              <path d="M3 10v4h3l5 4V6L6 10H3zm13.5 2a4.5 4.5 0 0 0-2.5-4v8a4.5 4.5 0 0 0 2.5-4zM14 3.2v2.1a7 7 0 0 1 0 13.4v2.1a9 9 0 0 0 0-17.6z"
                fill="currentColor"/>
            </svg>
          </button>

          {!recording ? (
            <button className="btn btn-rec" onClick={handleStart} title="Start recording">
              <svg viewBox="0 0 24 24" width="14" height="14"><circle cx="12" cy="12" r="7" fill="currentColor"/></svg>
            </button>
          ) : (
            <>
              <button className="btn btn-pause" onClick={handlePauseToggle} title={paused ? "Resume" : "Pause"}>
                {paused ? (
                  <svg viewBox="0 0 24 24" width="13" height="13"><path d="M8 5v14l11-7z" fill="currentColor"/></svg>
                ) : (
                  <svg viewBox="0 0 24 24" width="13" height="13"><rect x="6" y="5" width="4" height="14" fill="currentColor"/><rect x="14" y="5" width="4" height="14" fill="currentColor"/></svg>
                )}
              </button>
              <button className="btn btn-stop" onClick={handleStop} title="Stop">
                <svg viewBox="0 0 24 24" width="13" height="13"><rect x="6" y="6" width="12" height="12" rx="1" fill="currentColor"/></svg>
              </button>
            </>
          )}
        </div>

        <Genie recording={recording && !paused} />

        {recording && <div className="timer">{fmt(elapsed)}</div>}
      </div>
    </div>
  );
}
