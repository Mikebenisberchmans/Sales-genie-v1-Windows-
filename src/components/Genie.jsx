export default function Genie({ recording }) {
  return (
    <div className={`genie ${recording ? "nodding" : "idle"}`}>
      <svg viewBox="0 0 200 280" xmlns="http://www.w3.org/2000/svg" overflow="visible">
        <defs>
          {/* Body gradient — translucent mint */}
          <radialGradient id="bodyG" cx="50%" cy="30%" r="70%">
            <stop offset="0%"   stopColor="rgba(167,243,208,0.95)" />
            <stop offset="55%"  stopColor="rgba(52,211,153,0.82)" />
            <stop offset="100%" stopColor="rgba(6,95,70,0.55)" />
          </radialGradient>

          {/* Head gradient — warmer mint, opaque */}
          <radialGradient id="headG" cx="42%" cy="35%" r="68%">
            <stop offset="0%"   stopColor="#d1fae5" />
            <stop offset="60%"  stopColor="#6ee7b7" />
            <stop offset="100%" stopColor="#34d399" />
          </radialGradient>

          {/* Large outer aura glow */}
          <radialGradient id="auraG" cx="50%" cy="50%" r="50%">
            <stop offset="0%"   stopColor="rgba(52,211,153,0.55)" />
            <stop offset="70%"  stopColor="rgba(52,211,153,0.18)" />
            <stop offset="100%" stopColor="rgba(52,211,153,0)" />
          </radialGradient>

          {/* Inner body glow */}
          <radialGradient id="innerGlow" cx="50%" cy="40%" r="50%">
            <stop offset="0%"   stopColor="rgba(167,243,208,0.9)" />
            <stop offset="100%" stopColor="rgba(52,211,153,0)" />
          </radialGradient>

          {/* Drop shadow filter */}
          <filter id="shadowF" x="-40%" y="-40%" width="180%" height="180%">
            <feGaussianBlur in="SourceAlpha" stdDeviation="5" result="blur" />
            <feOffset dx="0" dy="4" result="offsetBlur" />
            <feFlood floodColor="rgba(6,95,70,0.4)" result="color" />
            <feComposite in="color" in2="offsetBlur" operator="in" result="shadow" />
            <feMerge>
              <feMergeNode in="shadow" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>

          {/* Glow filter for body */}
          <filter id="glowF" x="-30%" y="-30%" width="160%" height="160%">
            <feGaussianBlur in="SourceGraphic" stdDeviation="4" result="blur" />
            <feMerge>
              <feMergeNode in="blur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>

        {/* ── AURA GLOW ── */}
        <ellipse
          cx="100" cy="165"
          rx="88" ry="105"
          fill="url(#auraG)"
          className="aura"
        />

        {/* ── TAIL ── */}
        <g className="tail">
          <path
            d="M 98 215
               C 95 230 78 245 62 240
               C 46 234 44 216 56 208
               C 68 200 85 210 90 225
               C 96 242 82 260 62 262
               C 46 264 34 252 38 238"
            fill="url(#bodyG)"
            stroke="rgba(6,95,70,0.5)"
            strokeWidth="2"
            strokeLinecap="round"
            filter="url(#glowF)"
          />
        </g>

        {/* ── BODY blob ── */}
        <path
          d="M 64 188
             Q 56 168 60 148
             Q 66 122 84 116
             Q 100 112 116 116
             Q 134 122 140 148
             Q 144 168 136 188
             Q 120 198 100 198
             Q 80 198 64 188 Z"
          fill="url(#bodyG)"
          stroke="rgba(6,95,70,0.35)"
          strokeWidth="1.5"
          filter="url(#glowF)"
        />

        {/* Body inner highlight */}
        <ellipse
          cx="96" cy="152"
          rx="22" ry="28"
          fill="url(#innerGlow)"
          opacity="0.6"
        />

        {/* ── ARM NUBS ── */}
        <g className="arms">
          <ellipse
            cx="58" cy="165"
            rx="11" ry="7"
            fill="url(#bodyG)"
            stroke="rgba(6,95,70,0.3)"
            strokeWidth="1.2"
            transform="rotate(-30,58,165)"
          />
          <ellipse
            cx="142" cy="165"
            rx="11" ry="7"
            fill="url(#bodyG)"
            stroke="rgba(6,95,70,0.3)"
            strokeWidth="1.2"
            transform="rotate(30,142,165)"
          />
        </g>

        {/* ── HEAD ── */}
        <g className="head" filter="url(#shadowF)">

          {/* Main head circle */}
          <circle
            cx="100" cy="92"
            r="60"
            fill="url(#headG)"
            stroke="rgba(6,95,70,0.3)"
            strokeWidth="1.5"
          />

          {/* ── HAIR ── */}
          {/* Main dark dome */}
          <path
            d="M 42 82
               Q 40 44 68 22
               Q 84 10 100 10
               Q 116 10 132 22
               Q 160 44 158 82
               Q 148 62 130 56
               Q 115 52 100 51
               Q 85 52 70 56
               Q 52 62 42 82 Z"
            fill="#0f172a"
          />

          {/* Left side volume */}
          <path
            d="M 44 80
               Q 26 68 30 48
               Q 34 32 46 28
               Q 38 44 40 60
               Q 41 72 44 80 Z"
            fill="#1e293b"
          />
          {/* Left lower curl */}
          <path
            d="M 44 90
               Q 30 88 28 78
               Q 26 68 36 66
               Q 30 76 34 84
               Q 38 90 44 90 Z"
            fill="#1e293b"
          />

          {/* Right side volume */}
          <path
            d="M 156 80
               Q 174 68 170 48
               Q 166 32 154 28
               Q 162 44 160 60
               Q 159 72 156 80 Z"
            fill="#1e293b"
          />
          {/* Right lower curl */}
          <path
            d="M 156 90
               Q 170 88 172 78
               Q 174 68 164 66
               Q 170 76 166 84
               Q 162 90 156 90 Z"
            fill="#1e293b"
          />

          {/* Top center puff */}
          <ellipse cx="100" cy="20" rx="24" ry="16" fill="#1e293b" />
          {/* Left top puff */}
          <ellipse cx="74"  cy="26" rx="18" ry="14" fill="#0f172a" />
          {/* Right top puff */}
          <ellipse cx="126" cy="26" rx="18" ry="14" fill="#0f172a" />
          {/* Top center overlap for volume */}
          <ellipse cx="100" cy="14" rx="16" ry="10" fill="#334155" />

          {/* Hair shine highlight */}
          <path
            d="M 72 28 Q 90 18 110 22"
            stroke="rgba(148,163,184,0.35)"
            strokeWidth="3"
            fill="none"
            strokeLinecap="round"
          />

          {/* ── PEARL TIARA ── */}
          <path
            d="M 58 74 Q 100 60 142 74"
            stroke="#e2e8f0"
            strokeWidth="2.5"
            fill="none"
            strokeLinecap="round"
          />
          {/* Center pearl */}
          <circle cx="100" cy="62"  r="5.5" fill="white" stroke="#cbd5e1" strokeWidth="1" />
          <circle cx="100" cy="61"  r="2"   fill="rgba(167,243,208,0.7)" />
          <circle cx="98.5" cy="60" r="0.8" fill="rgba(255,255,255,0.9)" />
          {/* Inner left pearl */}
          <circle cx="82"  cy="67" r="4"   fill="white" stroke="#cbd5e1" strokeWidth="0.8" />
          <circle cx="81"  cy="66" r="1.4" fill="rgba(167,243,208,0.6)" />
          {/* Inner right pearl */}
          <circle cx="118" cy="67" r="4"   fill="white" stroke="#cbd5e1" strokeWidth="0.8" />
          <circle cx="117" cy="66" r="1.4" fill="rgba(167,243,208,0.6)" />
          {/* Outer left pearl */}
          <circle cx="66"  cy="73" r="3"   fill="white" stroke="#cbd5e1" strokeWidth="0.7" />
          {/* Outer right pearl */}
          <circle cx="134" cy="73" r="3"   fill="white" stroke="#cbd5e1" strokeWidth="0.7" />

          {/* ── EYES ── */}
          {/* Left eye */}
          <ellipse cx="78"  cy="102" rx="13" ry="14" fill="#1e293b" />
          <ellipse cx="78"  cy="102" rx="11" ry="12" fill="#0f172a" />
          <circle  cx="72"  cy="96"  r="4"   fill="white" />
          <circle  cx="83"  cy="98"  r="1.8" fill="white" opacity="0.85" />
          {/* Left sparkle cross */}
          <line x1="72" y1="94" x2="72" y2="91" stroke="white" strokeWidth="1"   strokeLinecap="round" />
          <line x1="70" y1="93" x2="74" y2="93" stroke="white" strokeWidth="1"   strokeLinecap="round" />

          {/* Right eye */}
          <ellipse cx="122" cy="102" rx="13" ry="14" fill="#1e293b" />
          <ellipse cx="122" cy="102" rx="11" ry="12" fill="#0f172a" />
          <circle  cx="116" cy="96"  r="4"   fill="white" />
          <circle  cx="127" cy="98"  r="1.8" fill="white" opacity="0.85" />
          {/* Right sparkle cross */}
          <line x1="116" y1="94" x2="116" y2="91" stroke="white" strokeWidth="1" strokeLinecap="round" />
          <line x1="114" y1="93" x2="118" y2="93" stroke="white" strokeWidth="1" strokeLinecap="round" />

          {/* Eyelashes left */}
          <path d="M 66 94 L 64 90 M 70 91 L 69 87 M 74 90 L 74 86 M 78 90 L 79 86 M 82 92 L 84 88 M 86 95 L 89 92"
            stroke="#0f172a" strokeWidth="1.8" strokeLinecap="round" fill="none" />
          {/* Eyelashes right */}
          <path d="M 110 94 L 108 90 M 114 91 L 113 87 M 118 90 L 118 86 M 122 90 L 123 86 M 126 92 L 128 88 M 130 95 L 133 92"
            stroke="#0f172a" strokeWidth="1.8" strokeLinecap="round" fill="none" />

          {/* ── ROSY CHEEKS ── */}
          <ellipse cx="64"  cy="116" rx="12" ry="8" fill="#fda4af" opacity="0.45" />
          <ellipse cx="136" cy="116" rx="12" ry="8" fill="#fda4af" opacity="0.45" />

          {/* ── NOSE ── */}
          <circle cx="100" cy="118" r="2" fill="#059669" opacity="0.55" />

          {/* ── SMILE ── */}
          <path
            d="M 88 128 Q 100 138 112 128"
            stroke="#065f46"
            strokeWidth="2.5"
            fill="none"
            strokeLinecap="round"
          />
          <circle cx="87"  cy="127" r="1.5" fill="#6ee7b7" opacity="0.6" />
          <circle cx="113" cy="127" r="1.5" fill="#6ee7b7" opacity="0.6" />
        </g>
      </svg>
    </div>
  );
}
