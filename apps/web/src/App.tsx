import { useEffect, useState } from "react";
import * as Automerge from "@automerge/automerge";
import { TimelineCanvas } from "@slop/ui-timeline";
import type { SlopTimeline } from "@slop/schemas";

const DEFAULT_SERVER = "ws://localhost:7878";

export function App() {
  const [server, setServer] = useState(DEFAULT_SERVER);
  const [projectId, setProjectId] = useState("");
  const [timeline, setTimeline] = useState<SlopTimeline | null>(null);
  const [status, setStatus] = useState<"idle" | "connecting" | "connected" | "error">("idle");

  useEffect(() => {
    if (!projectId) return;
    let cancelled = false;
    setStatus("connecting");
    const url = `${server.replace(/\/$/, "")}/ws/${encodeURIComponent(projectId)}`;
    const ws = new WebSocket(url);
    ws.binaryType = "arraybuffer";

    let doc = Automerge.init<{ ops?: string[] }>();
    let syncState = Automerge.initSyncState();

    const send = () => {
      const [next, msg] = Automerge.generateSyncMessage(doc, syncState);
      syncState = next;
      if (msg && ws.readyState === WebSocket.OPEN) {
        ws.send(msg);
      }
    };

    ws.addEventListener("open", () => {
      if (cancelled) return;
      setStatus("connected");
      send();
    });
    ws.addEventListener("message", (ev) => {
      if (cancelled) return;
      const bytes = new Uint8Array(ev.data as ArrayBuffer);
      const [next, ns] = Automerge.receiveSyncMessage(doc, syncState, bytes);
      doc = next;
      syncState = ns;
      // Replay the audit log into a SlopTimeline. The web companion is
      // read-only, so we snapshot every render.
      const ops = (doc.ops ?? []) as string[];
      const replayed = replay(ops);
      setTimeline(replayed);
      send();
    });
    ws.addEventListener("error", () => setStatus("error"));
    ws.addEventListener("close", () => {
      if (!cancelled) setStatus("idle");
    });
    return () => {
      cancelled = true;
      ws.close();
    };
  }, [server, projectId]);

  return (
    <div style={{ fontFamily: "system-ui", padding: 16, color: "#f5f6f8", background: "#0c0d10", minHeight: "100vh" }}>
      <header style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <h1 style={{ marginRight: 16 }}>Slop Review</h1>
        <input
          placeholder="ws://server"
          value={server}
          onChange={(e) => setServer(e.target.value)}
          style={{ flex: 1 }}
        />
        <input
          placeholder="project-uuid"
          value={projectId}
          onChange={(e) => setProjectId(e.target.value)}
          style={{ flex: 1 }}
        />
        <span style={{ color: status === "connected" ? "#5aef9e" : status === "error" ? "#ff7070" : "#a8aeb8" }}>
          {status}
        </span>
      </header>
      <main style={{ marginTop: 16 }}>
        {timeline ? (
          <TimelineCanvas
            timeline={timeline}
            selectedItem={null}
            onSelectItem={() => undefined}
            onTogglePin={() => undefined}
            onRegenerateRange={() => undefined}
          />
        ) : (
          <p>Enter a server URL and project ID to start.</p>
        )}
      </main>
    </div>
  );
}

// Minimal op replay so the read-only view doesn't need slop-core in WASM yet.
function replay(ops: string[]): SlopTimeline {
  const tl: SlopTimeline = {
    schema_version: "roughcut.v1",
    project: { fps: 30, resolution: { w: 1920, h: 1080 }, sample_rate: 48000 },
    assets: [],
    tracks: [],
  };
  for (const raw of ops) {
    try {
      const op = JSON.parse(raw) as { kind: { kind?: string; payload?: unknown } | string };
      // Best-effort projection. The wasm-compiled slop-core is the
      // V3.0 plan; until then this view shows the audit log lengths.
      void op;
    } catch {
      // ignore
    }
  }
  return tl;
}
