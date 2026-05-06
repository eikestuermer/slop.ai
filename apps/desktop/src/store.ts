import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { ProjectState, Asset, PlannerStatus } from "./types";
import * as ipc from "./ipc";

interface UiState {
  selectedAsset: string | null;
  selectedItem: string | null;
  prompt: string;
  isPlanning: boolean;
  lastPlannerStatus: PlannerStatus | null;
  endpointConfigOpen: boolean;
}

interface AppStore {
  project: ProjectState | null;
  ui: UiState;

  loadProject: (path: string) => Promise<void>;
  newProject: (path: string) => Promise<void>;
  importAsset: (uri: string) => Promise<void>;
  refreshTimeline: () => Promise<void>;

  setPrompt: (p: string) => void;
  selectAsset: (id: string | null) => void;
  selectItem: (id: string | null) => void;

  planRoughCut: () => Promise<void>;
  regenerateRange: (
    trackId: string,
    inSec: number,
    outSec: number,
  ) => Promise<void>;

  pinClip: (trackId: string, itemId: string) => Promise<void>;
  unpinClip: (trackId: string, itemId: string) => Promise<void>;

  renderPreview: () => Promise<string>;
  exportOtio: (path: string) => Promise<void>;

  toggleEndpointConfig: (open?: boolean) => void;
}

export const useStore = create<AppStore>()(
  immer((set, get) => ({
    project: null,
    ui: {
      selectedAsset: null,
      selectedItem: null,
      prompt: "",
      isPlanning: false,
      lastPlannerStatus: null,
      endpointConfigOpen: false,
    },

    async loadProject(path) {
      const project = await ipc.loadProject(path);
      set((s) => {
        s.project = project;
      });
    },

    async newProject(path) {
      const project = await ipc.newProject(path);
      set((s) => {
        s.project = project;
      });
    },

    async importAsset(uri) {
      const asset: Asset = await ipc.importAsset(uri);
      set((s) => {
        if (s.project) s.project.assets.push(asset);
      });
      // Kick off background jobs.
      void ipc.generateProxies(asset.asset_id).catch(() => undefined);
      void ipc.transcribeAsset(asset.asset_id).catch(() => undefined);
      void ipc.detectScenes(asset.asset_id).catch(() => undefined);
    },

    async refreshTimeline() {
      const tl = await ipc.getTimeline();
      set((s) => {
        if (s.project) s.project.timeline = tl;
      });
    },

    setPrompt(p) {
      set((s) => {
        s.ui.prompt = p;
      });
    },
    selectAsset(id) {
      set((s) => {
        s.ui.selectedAsset = id;
      });
    },
    selectItem(id) {
      set((s) => {
        s.ui.selectedItem = id;
      });
    },

    async planRoughCut() {
      const prompt = get().ui.prompt.trim();
      if (!prompt) return;
      set((s) => {
        s.ui.isPlanning = true;
      });
      try {
        await ipc.buildCandidates();
        const status = await ipc.planRoughCut(prompt);
        set((s) => {
          s.ui.lastPlannerStatus = status;
        });
        await get().refreshTimeline();
      } finally {
        set((s) => {
          s.ui.isPlanning = false;
        });
      }
    },

    async regenerateRange(trackId, inSec, outSec) {
      const prompt = get().ui.prompt.trim() || "Improve this section.";
      set((s) => {
        s.ui.isPlanning = true;
      });
      try {
        const status = await ipc.regenerateRange(
          trackId,
          inSec,
          outSec,
          prompt,
        );
        set((s) => {
          s.ui.lastPlannerStatus = status;
        });
        await get().refreshTimeline();
      } finally {
        set((s) => {
          s.ui.isPlanning = false;
        });
      }
    },

    async pinClip(trackId, itemId) {
      await ipc.pinClip(trackId, itemId);
      await get().refreshTimeline();
    },
    async unpinClip(trackId, itemId) {
      await ipc.unpinClip(trackId, itemId);
      await get().refreshTimeline();
    },

    async renderPreview() {
      return ipc.renderPreview();
    },
    async exportOtio(path) {
      await ipc.exportOtio(path);
    },

    toggleEndpointConfig(open) {
      set((s) => {
        s.ui.endpointConfigOpen =
          open === undefined ? !s.ui.endpointConfigOpen : open;
      });
    },
  })),
);
