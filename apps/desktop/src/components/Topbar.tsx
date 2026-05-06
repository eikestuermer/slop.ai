import { useStore } from "../store";

export function Topbar() {
  const project = useStore((s) => s.project);
  const toggleCfg = useStore((s) => s.toggleEndpointConfig);
  const renderPreview = useStore((s) => s.renderPreview);
  const exportOtio = useStore((s) => s.exportOtio);
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        height: 44,
        padding: "0 12px",
        background: "var(--bg-1)",
        borderBottom: "1px solid var(--bg-2)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <strong style={{ letterSpacing: 0.4 }}>SLOP AI</strong>
        <span style={{ color: "var(--fg-2)" }}>
          {project ? project.path : "no project"}
        </span>
      </div>
      <div style={{ display: "flex", gap: 8 }}>
        <button
          onClick={() => {
            void renderPreview().catch(() => undefined);
          }}
        >
          Render preview
        </button>
        <button
          onClick={() => {
            void exportOtio("rough-cut.otio").catch(() => undefined);
          }}
        >
          Export OTIO
        </button>
        <button onClick={() => toggleCfg()}>LLM endpoint</button>
      </div>
    </div>
  );
}
