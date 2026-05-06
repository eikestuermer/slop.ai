// IPC bindings to the Tauri Rust host.
//
// Every function here corresponds 1:1 to a `#[tauri::command]` in
// `src-tauri/src/commands.rs`. Types come from the canonical
// `@slop/schemas` package.

import { invoke } from "@tauri-apps/api/core";
import type { SlopTimeline } from "@slop/schemas";
import type { Asset, ProjectState, PlannerStatus } from "./types";

export async function loadProject(path: string): Promise<ProjectState> {
  return invoke<ProjectState>("load_project", { path });
}

export async function newProject(path: string): Promise<ProjectState> {
  return invoke<ProjectState>("new_project", { path });
}

export async function importAsset(uri: string): Promise<Asset> {
  return invoke<Asset>("import_asset", { uri });
}

export async function generateProxies(assetId: string): Promise<void> {
  await invoke("generate_proxies", { assetId });
}

export async function transcribeAsset(assetId: string): Promise<void> {
  await invoke("transcribe_asset", { assetId });
}

export async function detectScenes(assetId: string): Promise<void> {
  await invoke("detect_scenes", { assetId });
}

export async function buildCandidates(): Promise<void> {
  await invoke("build_candidates");
}

export async function planRoughCut(prompt: string): Promise<PlannerStatus> {
  return invoke<PlannerStatus>("plan_rough_cut", { prompt });
}

export async function pinClip(trackId: string, itemId: string): Promise<void> {
  await invoke("pin_clip", { trackId, itemId });
}

export async function unpinClip(trackId: string, itemId: string): Promise<void> {
  await invoke("unpin_clip", { trackId, itemId });
}

export async function regenerateRange(
  trackId: string,
  timelineIn: number,
  timelineOut: number,
  prompt: string,
): Promise<PlannerStatus> {
  return invoke<PlannerStatus>("regenerate_range", {
    trackId,
    timelineIn,
    timelineOut,
    prompt,
  });
}

export async function renderPreview(): Promise<string> {
  return invoke<string>("render_preview");
}

export async function exportOtio(outPath: string): Promise<void> {
  await invoke("export_otio", { outPath });
}

export async function getTimeline(): Promise<SlopTimeline> {
  return invoke<SlopTimeline>("get_timeline");
}

export async function getEndpointConfig(): Promise<{
  base_url: string;
  model: string;
  api_key_set: boolean;
  temperature: number;
}> {
  return invoke("get_endpoint_config");
}

export async function setEndpointConfig(cfg: {
  base_url: string;
  model: string;
  api_key?: string;
  temperature: number;
}): Promise<void> {
  await invoke("set_endpoint_config", { cfg });
}
