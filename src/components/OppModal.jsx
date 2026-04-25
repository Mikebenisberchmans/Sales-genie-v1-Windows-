import { useState } from "react";

export default function OppModal({ onSubmit, onCancel }) {
  const [oppId, setOppId] = useState("");

  const handleSubmit = () => {
    const trimmed = oppId.trim();
    if (trimmed) onSubmit(trimmed);
  };

  return (
    <div className="opp-overlay">
      <div className="opp-card">
        <div className="opp-title">Opportunity ID</div>
        <div className="opp-sub">Enter the CRM opportunity for this call</div>
        <input
          className="opp-input"
          value={oppId}
          onChange={(e) => setOppId(e.target.value)}
          placeholder="e.g. OPP-00123"
          autoFocus
          onKeyDown={(e) => {
            if (e.key === "Enter") handleSubmit();
            if (e.key === "Escape") onCancel();
          }}
        />
        <div className="opp-actions">
          <button className="btn-opp-cancel" onClick={onCancel}>
            Skip
          </button>
          <button
            className="btn-opp-submit"
            onClick={handleSubmit}
            disabled={!oppId.trim()}
          >
            Submit
          </button>
        </div>
      </div>
    </div>
  );
}
