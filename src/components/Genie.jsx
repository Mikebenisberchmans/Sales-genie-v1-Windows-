export default function Genie({ recording }) {
  return (
    <div className={`genie ${recording ? "nodding" : "idle"}`}>
      <svg viewBox="0 0 200 280" xmlns="http://www.w3.org/2000/svg">
        <defs>
          <radialGradient id="bodyG" cx="50%" cy="35%" r="75%">
            <stop offset="0%" stopColor="#BBF7D0" />
            <stop offset="55%" stopColor="#34D399" />
            <stop offset="100%" stopColor="#047857" />
          </radialGradient>
          <radialGradient id="glowG" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor="rgba(52, 211, 153, 0.45)" />
            <stop offset="100%" stopColor="rgba(52, 211, 153, 0)" />
          </radialGradient>
        </defs>

        <ellipse cx="100" cy="130" rx="100" ry="120" fill="url(#glowG)" className="aura" />

        {/* Crescent-moon tail curling right — sperm-like wiggle */}
        <g className="tail">
          <path
            d="M 100 170
               Q 115 210 155 215
               Q 180 218 175 200
               Q 170 185 150 188
               Q 125 192 110 175 Z"
            fill="url(#bodyG)"
            stroke="#065F46"
            strokeWidth="2"
          />
        </g>

        {/* Sash / belt */}
        <ellipse cx="100" cy="175" rx="35" ry="6" fill="#065F46" />

        {/* Body (pear shape) */}
        <path
          d="M 70 175
             Q 60 150 70 125
             Q 82 105 100 105
             Q 118 105 130 125
             Q 140 150 130 175
             Q 115 180 100 180
             Q 85 180 70 175 Z"
          fill="url(#bodyG)"
          stroke="#065F46"
          strokeWidth="2.2"
        />

        {/* Arms (small, at sides) */}
        <g className="arms">
          <path d="M 72 160 Q 62 168 65 178" stroke="#065F46" strokeWidth="3" fill="none" strokeLinecap="round" />
          <path d="M 128 160 Q 138 168 135 178" stroke="#065F46" strokeWidth="3" fill="none" strokeLinecap="round" />
          {/* little hands */}
          <circle cx="64" cy="179" r="4" fill="#34D399" stroke="#065F46" strokeWidth="1.5" />
          <circle cx="136" cy="179" r="4" fill="#34D399" stroke="#065F46" strokeWidth="1.5" />
        </g>

        {/* Head (pear-top, rounded) */}
        <g className="head">
          <path
            d="M 62 85
               Q 55 55 80 40
               Q 100 32 120 40
               Q 145 55 138 85
               Q 138 110 115 115
               Q 100 118 85 115
               Q 62 110 62 85 Z"
            fill="url(#bodyG)"
            stroke="#065F46"
            strokeWidth="2.2"
          />

          {/* Swept-back topknot (tall, leaning right) */}
          <path
            d="M 95 40
               Q 100 10 125 5
               Q 138 3 132 18
               Q 128 30 118 38
               Q 108 45 100 44 Z"
            fill="#1F2937"
            stroke="#0F172A"
            strokeWidth="1.5"
          />
          {/* Hair base wrap */}
          <ellipse cx="102" cy="42" rx="14" ry="5" fill="#1F2937" />

          {/* Closed happy eyes (curves) with eyelashes */}
          <path d="M 78 78 Q 86 72 94 78" stroke="#0a0a0a" strokeWidth="2.2" fill="none" strokeLinecap="round" />
          <path d="M 106 78 Q 114 72 122 78" stroke="#0a0a0a" strokeWidth="2.2" fill="none" strokeLinecap="round" />
          {/* eyelashes */}
          <path d="M 78 78 L 75 74 M 82 75 L 81 71 M 86 73 L 86 69 M 90 74 L 91 70 M 94 78 L 96 74"
            stroke="#0a0a0a" strokeWidth="1.5" strokeLinecap="round" />
          <path d="M 106 78 L 104 74 M 110 75 L 110 71 M 114 73 L 114 69 M 118 74 L 119 70 M 122 78 L 124 74"
            stroke="#0a0a0a" strokeWidth="1.5" strokeLinecap="round" />

          {/* Small nose */}
          <circle cx="100" cy="88" r="1.5" fill="#065F46" />

          {/* Smile */}
          <path d="M 90 96 Q 100 104 110 96" stroke="#0a0a0a" strokeWidth="2.2" fill="none" strokeLinecap="round" />

          {/* Soft blush */}
          <ellipse cx="76" cy="92" rx="5" ry="3" fill="#F87171" opacity="0.45" />
          <ellipse cx="124" cy="92" rx="5" ry="3" fill="#F87171" opacity="0.45" />
        </g>
      </svg>
    </div>
  );
}
