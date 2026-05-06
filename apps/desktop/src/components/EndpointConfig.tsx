import { useEffect, useState } from "react";
import { useStore } from "../store";
import { getEndpointConfig, setEndpointConfig } from "../ipc";

export function EndpointConfig() {
  const close = useStore((s) => () => s.toggleEndpointConfig(false));
  const [baseUrl, setBaseUrl] = useState("http://localhost:11434/v1");
  const [model, setModel] = useState("qwen3:8b");
  const [apiKey, setApiKey] = useState("");
  const [temperature, setTemperature] = useState(0);
  const [hasKey, setHasKey] = useState(false);

  useEffect(() => {
    void getEndpointConfig().then((cfg) => {
      setBaseUrl(cfg.base_url);
      setModel(cfg.model);
      setHasKey(cfg.api_key_set);
      setTemperature(cfg.temperature);
    });
  }, []);

  const isLocal =
    baseUrl.startsWith("http://localhost") ||
    baseUrl.startsWith("http://127.0.0.1") ||
    baseUrl.startsWith("http://[::1]");

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.6)",
        display: "grid",
        placeItems: "center",
        zIndex: 100,
      }}
      onClick={close}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          background: "var(--bg-1)",
          border: "1px solid var(--bg-3)",
          borderRadius: 10,
          padding: 24,
          width: 480,
          display: "flex",
          flexDirection: "column",
          gap: 12,
        }}
      >
        <h2 style={{ margin: 0 }}>LLM endpoint</h2>
        <p style={{ color: "var(--fg-2)", margin: 0, fontSize: 12 }}>
          Slop AI talks to any OpenAI-compatible chat-completions endpoint.
          Recommended local stacks: <code>Ollama + qwen3:8b</code>,{" "}
          <code>llama.cpp server</code>, <code>LM Studio</code>.
        </p>

        <Field label="Base URL">
          <input
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            style={{ width: "100%" }}
          />
        </Field>
        <Field label="Model">
          <input
            value={model}
            onChange={(e) => setModel(e.target.value)}
            style={{ width: "100%" }}
          />
        </Field>
        <Field label="API key (optional, blank for local)">
          <input
            type="password"
            value={apiKey}
            placeholder={hasKey ? "(unchanged)" : ""}
            onChange={(e) => setApiKey(e.target.value)}
            style={{ width: "100%" }}
          />
        </Field>
        <Field label={`Temperature: ${temperature.toFixed(2)}`}>
          <input
            type="range"
            min={0}
            max={1.5}
            step={0.05}
            value={temperature}
            onChange={(e) => setTemperature(Number(e.target.value))}
            style={{ width: "100%" }}
          />
        </Field>

        {!isLocal && (
          <div
            style={{
              padding: 8,
              border: "1px solid var(--warn)",
              borderRadius: 6,
              fontSize: 12,
              color: "var(--warn)",
            }}
          >
            Warning: this endpoint is not on localhost. Slop AI will send your
            project goal and candidate metadata to it. No media files leave
            your machine.
          </div>
        )}

        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button onClick={close}>Cancel</button>
          <button
            className="primary"
            onClick={async () => {
              await setEndpointConfig({
                base_url: baseUrl,
                model,
                api_key: apiKey || undefined,
                temperature,
              });
              close();
            }}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <span style={{ fontSize: 11, color: "var(--fg-2)" }}>{label}</span>
      {children}
    </label>
  );
}
