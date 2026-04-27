import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { message } from "@tauri-apps/plugin-dialog";
import Genie from "./components/Genie.jsx";
import OppModal from "./components/OppModal.jsx";
import { playStartSound, playPauseSound, playStopSound, playDisclaimer } from "./sounds";

const DISCLAIMER_TEXT = "This call is being recorded for quality and training purposes.";

// Pipeline stages shown in the status pill
const STAGE_LABELS = {
  transcribing: "🎙 Transcribing…",
  analyzing:    "🧠 Analyzing…",
  uploading:    "⏫ Uploading…",
  saving:       "💾 Saving…",
  done:         "✓ Complete",
  error:        "✗ Failed",
};

export default function App() {
  const [recording, setRecording]       = useState(false);
  const [paused, setPaused]             = useState(false);
  const [hovered, setHovered]           = useState(false);
  const [elapsed, setElapsed]           = useState(0);
  const [entered, setEntered]           = useState(false);
  const [showOppModal, setShowOppModal] = useState(false);
  const [pipelineStage, setPipelineStage] = useState(null); // null | keyof STAGE_LABELS
  const [pendingPaths, setPendingPaths] = useState(null);

  useEffect(() => {
    const t = setTimeout(() => setEntered(true), 150);
    return () => clearTimeout(t);
  }, []);

  useEffect(() => {
    if (!recording || paused) return;
    const id = setInterval(() => setElapsed((e) => e + 1), 1000);
    return () => clearInterval(id);
  }, [recording, paused]);

  // ── Recording controls ──────────────────────────────────────────────────

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
    if (paused) playStartSound();
    else playPauseSound();
    setPaused((p) => !p);
  };

  const handleStop = async () => {
    try {
      playStopSound();
      const res = await invoke("stop_recording");
      const duration = elapsed;
      setRecording(false);
      setPaused(false);
      setElapsed(0);
      setPendingPaths({ mic_path: res.mic_path, sys_path: res.sys_path, duration });
      setShowOppModal(true);
    } catch (e) {
      await message("Stop failed: " + e, { title: "Error", kind: "error" });
    }
  };

  // ── Post-recording pipeline ─────────────────────────────────────────────

  const handleOppSubmit = async (oppId) => {
    setShowOppModal(false);
    if (!pendingPaths) return;

    const { mic_path, sys_path, duration } = pendingPaths;
    setPendingPaths(null);

    try {
      const config = await invoke("get_config");
      const aiEnabled   = config?.analysis?.enabled === true;
      const hasStore    = config?.objectStore?.provider && config.objectStore.provider !== "none";
      const hasWarehouse = config?.warehouse?.provider && config.warehouse.provider !== "none";
      const hasPerson   = !!config?.salesperson?.name;

      // ------------------------------------------------------------------
      // Step 1: AI transcription + analysis (optional)
      // ------------------------------------------------------------------
      let transcript = "";
      let analysisData = null;

      if (aiEnabled) {
        setPipelineStage("transcribing");
        try {
          const aiResult = await invoke("transcribe_and_analyze", {
            micPath: mic_path,
            sysPath: sys_path,
            config,
          });
          transcript   = aiResult.transcript   ?? "";
          analysisData = aiResult.analysis      ?? null;
          setPipelineStage("analyzing");
          await _sleep(600);
        } catch (aiErr) {
          // Show the real error so the user knows why AI fields will be empty
          const errStr = String(aiErr);
          let hint = "";
          if (errStr.includes("Connection refused") || errStr.includes("STT request failed")) {
            hint = "\n\nFix: Start the STT server first:\n  python server/stt_server.py";
          } else if (errStr.includes("401") || errStr.includes("unauthorized")) {
            hint = "\n\nFix: Check your API token in the AI Analysis config tab.";
          } else if (errStr.includes("Model still loading") || errStr.includes("503")) {
            hint = "\n\nFix: Modal container is cold-starting — wait 20 s and try again.";
          }
          await message(
            `AI pipeline failed — recording is saved locally but transcript/analysis will be empty.\n\nError: ${errStr}${hint}`,
            { title: "AI Analysis Failed", kind: "warning" }
          );
          transcript   = "";
          analysisData = null;
        }
      }

      // ------------------------------------------------------------------
      // Step 2: Upload WAVs to blob storage (optional)
      // ------------------------------------------------------------------
      let micUrl = null;
      let sysUrl = null;

      if (hasStore && hasPerson) {
        setPipelineStage("uploading");
        try {
          const urls = await invoke("upload_to_storage", {
            micPath: mic_path,
            sysPath: sys_path,
            oppId,
            config,
          });
          micUrl = urls.mic_url;
          sysUrl = urls.sys_url;
        } catch (uploadErr) {
          console.warn("Upload failed:", uploadErr);
        }
      }

      // ------------------------------------------------------------------
      // Step 3: Write metadata + analysis to warehouse (optional)
      // ------------------------------------------------------------------
      if (hasWarehouse) {
        setPipelineStage("saving");

        // Extract fields matching the Salenie-Phi4-v1 output schema:
        // { summary, deal_metadata: { company, industry, deal_size_estimate },
        //   qualification_metrics: { ... }, sentiment_analysis: { lead_score, ... },
        //   next_steps: { action_items: [...], recommended_forecast_category } }
        const a = analysisData ?? {};

        const summary     = a.summary ?? a.ai_summary ?? "";
        const dealMeta    = a.deal_metadata ?? {};
        const sentiment   = a.sentiment_analysis ?? {};
        const nextStepsObj = a.next_steps ?? {};

        const dealAmount  = dealMeta.deal_size_estimate ?? a.deal_size ?? a.deal_amount ?? 0;
        const dealCompany = dealMeta.company ?? a.deal_company ?? "";
        const dealStage   = nextStepsObj.recommended_forecast_category ?? a.deal_stage ?? "";
        const sentScore   = sentiment.lead_score ?? a.sentiment_score ?? 0;
        const nextStepsArr = Array.isArray(nextStepsObj.action_items)
          ? nextStepsObj.action_items
          : Array.isArray(a.next_steps)
            ? a.next_steps
            : [];

        try {
          await invoke("save_to_warehouse", {
            metadata: {
              opp_id:             oppId,
              submission_date:    new Date().toISOString(),
              duration_seconds:   duration,
              salesperson_name:   config?.salesperson?.name  ?? "",
              salesperson_id:     config?.salesperson?.id    ?? "",
              mic_url:            micUrl  ?? "",
              sys_url:            sysUrl  ?? "",
              mic_local_path:     mic_path,
              sys_local_path:     sys_path,
              sample_rate:        44100,
              channels:           1,
              transcript_text:    transcript,
              ai_summary:         summary,
              deal_amount:        dealAmount,
              deal_company:       dealCompany,
              deal_stage:         dealStage,
              sentiment_score:    sentScore,
              next_steps:         JSON.stringify(nextStepsArr),
              full_analysis_json: JSON.stringify(a),
            },
            warehouseConfig: config.warehouse,
          });
        } catch (whErr) {
          console.error("Warehouse save failed:", whErr);
          await message(
            `Warehouse insert failed:\n\n${whErr}`,
            { title: "Snowflake Error", kind: "error" }
          );
        }
      }

      // ------------------------------------------------------------------
      // Done
      // ------------------------------------------------------------------
      setPipelineStage("done");
      setTimeout(() => setPipelineStage(null), 3500);

      // Show local paths only when nothing was uploaded
      if (!hasStore || !hasPerson) {
        const tip = aiEnabled ? "" : "\n\nTip: Enable AI Analysis in settings for automatic transcription.";
        await message(
          `Recording saved locally:\n\n🎤 ${mic_path}\n\n🔊 ${sys_path}${tip}`,
          { title: "Recording saved", kind: "info" }
        );
      }
    } catch (e) {
      setPipelineStage("error");
      setTimeout(() => setPipelineStage(null), 3500);
      await message(`Pipeline error: ${e}`, { title: "Error", kind: "error" });
    }
  };

  const handleOppCancel = async () => {
    setShowOppModal(false);
    if (!pendingPaths) return;
    const { mic_path, sys_path } = pendingPaths;
    setPendingPaths(null);
    await message(
      `Files saved locally:\n\n🎤 ${mic_path}\n\n🔊 ${sys_path}`,
      { title: "Recording saved", kind: "info" }
    );
  };

  // ── Misc ────────────────────────────────────────────────────────────────
  const handleDisclaimer = () => playDisclaimer(DISCLAIMER_TEXT);
  const handleOpenConfig = () =>
    invoke("open_config_window").catch((e) =>
      message("Could not open config window: " + e, { title: "Error", kind: "error" })
    );
  const fmt = (s) =>
    `${Math.floor(s / 60).toString().padStart(2, "0")}:${(s % 60).toString().padStart(2, "0")}`;

  // ── Render ───────────────────────────────────────────────────────────────

  return (
    <div
      className={`stage ${entered ? "entered" : ""}`}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div className={`genie-wrap ${recording ? "recording" : ""}`}>

        {/* Arc controls */}
        <div className={`controls ${hovered ? "show" : ""}`}>
          <button className="btn btn-consent btn-arc-0" onClick={handleDisclaimer} title="Play disclaimer">
            <svg viewBox="0 0 24 24" width="14" height="14">
              <path d="M3 10v4h3l5 4V6L6 10H3zm13.5 2a4.5 4.5 0 0 0-2.5-4v8a4.5 4.5 0 0 0 2.5-4zM14 3.2v2.1a7 7 0 0 1 0 13.4v2.1a9 9 0 0 0 0-17.6z" fill="currentColor" />
            </svg>
          </button>

          <button className="btn btn-db btn-arc-1" onClick={handleOpenConfig} title="Configure database & AI">
            <svg viewBox="0 0 24 24" width="14" height="14">
              <path d="M4 6h16M4 10h16M4 14h16M4 18h10" stroke="currentColor" strokeWidth="2" strokeLinecap="round" fill="none" />
            </svg>
          </button>

          {!recording ? (
            <button className="btn btn-rec btn-arc-2" onClick={handleStart} title="Start recording">
              <svg viewBox="0 0 24 24" width="13" height="13"><circle cx="12" cy="12" r="7" fill="currentColor" /></svg>
            </button>
          ) : paused ? (
            <button className="btn btn-pause btn-arc-2" onClick={handlePauseToggle} title="Resume">
              <svg viewBox="0 0 24 24" width="13" height="13"><path d="M8 5v14l11-7z" fill="currentColor" /></svg>
            </button>
          ) : (
            <button className="btn btn-pause btn-arc-2" onClick={handlePauseToggle} title="Pause">
              <svg viewBox="0 0 24 24" width="13" height="13">
                <rect x="6" y="5" width="4" height="14" fill="currentColor" />
                <rect x="14" y="5" width="4" height="14" fill="currentColor" />
              </svg>
            </button>
          )}

          {recording && (
            <button className="btn btn-stop btn-arc-3" onClick={handleStop} title="Stop recording">
              <svg viewBox="0 0 24 24" width="13" height="13"><rect x="6" y="6" width="12" height="12" rx="1" fill="currentColor" /></svg>
            </button>
          )}
        </div>

        <div className="genie-float">
          <Genie recording={recording && !paused} />
        </div>

        {recording && <div className="timer">{fmt(elapsed)}</div>}

        {/* Pipeline status pill */}
        {pipelineStage && (
          <div className={`upload-bar ${pipelineStage === "done" ? "success" : pipelineStage === "error" ? "error" : "uploading"}`}>
            {STAGE_LABELS[pipelineStage] ?? pipelineStage}
          </div>
        )}
      </div>

      {showOppModal && (
        <OppModal onSubmit={handleOppSubmit} onCancel={handleOppCancel} />
      )}
    </div>
  );
}

function _sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}
