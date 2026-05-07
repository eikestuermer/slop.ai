import { describe, expect, it, beforeEach, vi } from "vitest";

// Mock the Tauri IPC module before the store imports it. The store touches
// the IPC layer only inside async actions; the synchronous reducers we
// test below never call it, but importing the module evaluates it.
vi.mock("../ipc", () => ({
  loadProject: vi.fn(),
  newProject: vi.fn(),
  importAsset: vi.fn(),
  generateProxies: vi.fn(),
  transcribeAsset: vi.fn(),
  detectScenes: vi.fn(),
  buildCandidates: vi.fn(),
  planRoughCut: vi.fn(),
  pinClip: vi.fn(),
  unpinClip: vi.fn(),
  regenerateRange: vi.fn(),
  renderPreview: vi.fn(),
  exportOtio: vi.fn(),
  getTimeline: vi.fn(),
  getEndpointConfig: vi.fn(),
  setEndpointConfig: vi.fn(),
}));

const { useStore } = await import("../store");

describe("store synchronous reducers", () => {
  beforeEach(() => {
    // Reset to a known initial state (matches the create() default).
    useStore.setState({
      project: null,
      ui: {
        selectedAsset: null,
        selectedItem: null,
        prompt: "",
        isPlanning: false,
        lastPlannerStatus: null,
        endpointConfigOpen: false,
      },
    });
  });

  it("setPrompt updates ui.prompt", () => {
    useStore.getState().setPrompt("hello");
    expect(useStore.getState().ui.prompt).toBe("hello");
  });

  it("selectAsset updates selectedAsset", () => {
    useStore.getState().selectAsset("a_xyz");
    expect(useStore.getState().ui.selectedAsset).toBe("a_xyz");
    useStore.getState().selectAsset(null);
    expect(useStore.getState().ui.selectedAsset).toBeNull();
  });

  it("selectItem updates selectedItem", () => {
    useStore.getState().selectItem("c_42");
    expect(useStore.getState().ui.selectedItem).toBe("c_42");
  });

  it("toggleEndpointConfig flips when called with no arg", () => {
    expect(useStore.getState().ui.endpointConfigOpen).toBe(false);
    useStore.getState().toggleEndpointConfig();
    expect(useStore.getState().ui.endpointConfigOpen).toBe(true);
    useStore.getState().toggleEndpointConfig();
    expect(useStore.getState().ui.endpointConfigOpen).toBe(false);
  });

  it("toggleEndpointConfig forces value when arg is given", () => {
    useStore.getState().toggleEndpointConfig(true);
    expect(useStore.getState().ui.endpointConfigOpen).toBe(true);
    useStore.getState().toggleEndpointConfig(true);
    expect(useStore.getState().ui.endpointConfigOpen).toBe(true);
    useStore.getState().toggleEndpointConfig(false);
    expect(useStore.getState().ui.endpointConfigOpen).toBe(false);
  });

  it("initial UI state has empty selectedAsset and selectedItem", () => {
    const ui = useStore.getState().ui;
    expect(ui.selectedAsset).toBeNull();
    expect(ui.selectedItem).toBeNull();
    expect(ui.prompt).toBe("");
    expect(ui.isPlanning).toBe(false);
    expect(ui.lastPlannerStatus).toBeNull();
  });
});
