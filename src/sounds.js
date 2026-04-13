// Professional UI sounds synthesized on the fly — no audio files needed.
let ctx;
function getCtx() {
  if (!ctx) ctx = new (window.AudioContext || window.webkitAudioContext)();
  if (ctx.state === "suspended") ctx.resume();
  return ctx;
}

function tone({ freq = 600, duration = 0.18, type = "sine", gain = 0.18, attack = 0.01, release = 0.12 }) {
  const c = getCtx();
  const osc = c.createOscillator();
  const g = c.createGain();
  osc.type = type;
  osc.frequency.value = freq;
  const now = c.currentTime;
  g.gain.setValueAtTime(0, now);
  g.gain.linearRampToValueAtTime(gain, now + attack);
  g.gain.exponentialRampToValueAtTime(0.0001, now + duration + release);
  osc.connect(g).connect(c.destination);
  osc.start(now);
  osc.stop(now + duration + release);
}

// Rising two-note chime — "resumed / starting"
export function playStartSound() {
  tone({ freq: 660, duration: 0.12, gain: 0.15 });
  setTimeout(() => tone({ freq: 880, duration: 0.18, gain: 0.18 }), 90);
}

// Falling two-note — "paused"
export function playPauseSound() {
  tone({ freq: 720, duration: 0.12, gain: 0.15 });
  setTimeout(() => tone({ freq: 480, duration: 0.18, gain: 0.15 }), 90);
}

// Short confirmation — "stopped / saved"
export function playStopSound() {
  tone({ freq: 520, duration: 0.1, gain: 0.15 });
  setTimeout(() => tone({ freq: 392, duration: 0.14, gain: 0.15 }), 80);
  setTimeout(() => tone({ freq: 330, duration: 0.22, gain: 0.15 }), 170);
}

// Spoken disclaimer via SpeechSynthesis — professional voice
export function playDisclaimer(text) {
  if (!("speechSynthesis" in window)) return;
  const u = new SpeechSynthesisUtterance(text);
  // Pick a clear English voice if available
  const voices = window.speechSynthesis.getVoices();
  const preferred = voices.find(v => /en-US|en-GB/i.test(v.lang) && /female|samantha|google|aria|zira/i.test(v.name));
  if (preferred) u.voice = preferred;
  u.rate = 0.95;
  u.pitch = 1.0;
  u.volume = 1.0;
  window.speechSynthesis.cancel();
  window.speechSynthesis.speak(u);
}
