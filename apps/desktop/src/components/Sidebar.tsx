import { useStore } from "../store";

export function Sidebar() {
  const project = useStore((s) => s.project);
  const importAsset = useStore((s) => s.importAsset);
  const selectAsset = useStore((s) => s.selectAsset);
  const selectedAsset = useStore((s) => s.ui.selectedAsset);

  return (
    <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 12 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <strong>Media</strong>
        <button
          style={{ marginLeft: "auto" }}
          onClick={() => {
            const uri = prompt("file:// URI for asset");
            if (uri) void importAsset(uri).catch(() => undefined);
          }}
        >
          + Import
        </button>
      </div>
      <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 4 }}>
        {(project?.assets ?? []).map((a) => (
          <li
            key={a.asset_id}
            onClick={() => selectAsset(a.asset_id)}
            style={{
              padding: 8,
              borderRadius: 6,
              cursor: "pointer",
              background:
                selectedAsset === a.asset_id ? "var(--accent-soft)" : "var(--bg-2)",
              border:
                selectedAsset === a.asset_id
                  ? "1px solid var(--accent)"
                  : "1px solid transparent",
            }}
          >
            <div style={{ fontWeight: 600 }}>
              {a.uri.split("/").pop() ?? a.asset_id}
            </div>
            <div style={{ color: "var(--fg-2)", fontSize: 11 }}>
              {a.duration_sec.toFixed(1)}s
              {a.has_video ? " · video" : ""}
              {a.has_audio ? " · audio" : ""}
              {" · "}
              <StatusDot label="tx" status={a.transcript_status} />
              {" "}
              <StatusDot label="sc" status={a.scenes_status} />
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}

function StatusDot({
  label,
  status,
}: {
  label: string;
  status: "missing" | "running" | "ready" | "error";
}) {
  const color =
    status === "ready"
      ? "var(--good)"
      : status === "running"
        ? "var(--warn)"
        : status === "error"
          ? "var(--bad)"
          : "var(--fg-2)";
  return (
    <span style={{ color }} title={`${label}: ${status}`}>
      {label}
    </span>
  );
}
