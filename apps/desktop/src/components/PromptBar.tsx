import { useStore } from "../store";

export function PromptBar() {
  const prompt = useStore((s) => s.ui.prompt);
  const setPrompt = useStore((s) => s.setPrompt);
  const planRoughCut = useStore((s) => s.planRoughCut);
  const isPlanning = useStore((s) => s.ui.isPlanning);
  const last = useStore((s) => s.ui.lastPlannerStatus);

  return (
    <div
      style={{
        padding: 12,
        display: "grid",
        gridTemplateColumns: "1fr auto",
        gap: 12,
        height: "100%",
      }}
    >
      <textarea
        placeholder="Describe the rough cut. e.g.: '45-second founder story, warm and credible, open with the strongest line.'"
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        style={{
          height: "100%",
          resize: "none",
          fontSize: 13,
          lineHeight: 1.5,
        }}
      />
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 8,
          minWidth: 220,
        }}
      >
        <button
          className="primary"
          disabled={isPlanning || !prompt.trim()}
          onClick={() => void planRoughCut().catch(() => undefined)}
          style={{ height: 64, fontSize: 16 }}
        >
          {isPlanning ? "Planning..." : "Generate rough cut"}
        </button>
        {last && (
          <div
            style={{
              padding: 8,
              borderRadius: 6,
              background: last.ok ? "var(--bg-2)" : "rgba(255,112,112,0.1)",
              border: last.ok
                ? "1px solid var(--bg-3)"
                : "1px solid var(--bad)",
              fontSize: 11,
              maxHeight: 100,
              overflow: "auto",
            }}
          >
            <div style={{ color: last.ok ? "var(--good)" : "var(--bad)" }}>
              {last.ok ? "ok" : "failed"}: {last.message}
            </div>
            {last.repair_notes.map((n, i) => (
              <div key={i} style={{ color: "var(--fg-2)" }}>
                · {n}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
