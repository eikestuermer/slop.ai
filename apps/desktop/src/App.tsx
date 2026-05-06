import { useEffect } from "react";
import { useStore } from "./store";
import { Sidebar } from "./components/Sidebar";
import { TimelineView } from "./components/TimelineView";
import { PromptBar } from "./components/PromptBar";
import { Inspector } from "./components/Inspector";
import { EndpointConfig } from "./components/EndpointConfig";
import { Topbar } from "./components/Topbar";

export function App() {
  const project = useStore((s) => s.project);
  const cfgOpen = useStore((s) => s.ui.endpointConfigOpen);

  useEffect(() => {
    // Auto-load demo project for dev mode if no project is open. The Rust
    // host is responsible for creating ./demo on first run.
    if (!project) {
      void useStore.getState().loadProject(".").catch(() => undefined);
    }
  }, [project]);

  return (
    <div
      style={{
        display: "grid",
        gridTemplateRows: "44px 1fr 200px",
        gridTemplateColumns: "260px 1fr 280px",
        gridTemplateAreas: `
          "topbar topbar topbar"
          "sidebar timeline inspector"
          "sidebar prompt prompt"
        `,
        height: "100%",
        background: "var(--bg-0)",
      }}
    >
      <div style={{ gridArea: "topbar" }}>
        <Topbar />
      </div>
      <div
        style={{
          gridArea: "sidebar",
          background: "var(--bg-1)",
          borderRight: "1px solid var(--bg-2)",
          overflow: "auto",
        }}
      >
        <Sidebar />
      </div>
      <div
        style={{
          gridArea: "timeline",
          background: "var(--bg-0)",
          overflow: "auto",
        }}
      >
        <TimelineView />
      </div>
      <div
        style={{
          gridArea: "inspector",
          background: "var(--bg-1)",
          borderLeft: "1px solid var(--bg-2)",
          overflow: "auto",
        }}
      >
        <Inspector />
      </div>
      <div
        style={{
          gridArea: "prompt",
          background: "var(--bg-1)",
          borderTop: "1px solid var(--bg-2)",
        }}
      >
        <PromptBar />
      </div>
      {cfgOpen && <EndpointConfig />}
    </div>
  );
}
