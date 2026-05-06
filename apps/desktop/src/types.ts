import type { SlopTimeline } from "@slop/schemas";

export interface Asset {
  asset_id: string;
  uri: string;
  duration_sec: number;
  has_video: boolean;
  has_audio: boolean;
  fps: number | null;
  resolution: { w: number; h: number } | null;
  proxy_path?: string | null;
  thumb_strip_path?: string | null;
  transcript_status: "missing" | "running" | "ready" | "error";
  scenes_status: "missing" | "running" | "ready" | "error";
}

export interface ProjectState {
  path: string;
  timeline: SlopTimeline;
  assets: Asset[];
  pending_jobs: number;
}

export interface PlannerStatus {
  ok: boolean;
  message: string;
  repair_notes: string[];
}
