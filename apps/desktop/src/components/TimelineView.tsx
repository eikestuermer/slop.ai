import { TimelineCanvas } from "@slop/ui-timeline";
import { useStore } from "../store";

export function TimelineView() {
  const project = useStore((s) => s.project);
  const selectedItem = useStore((s) => s.ui.selectedItem);
  const selectItem = useStore((s) => s.selectItem);
  const pinClip = useStore((s) => s.pinClip);
  const unpinClip = useStore((s) => s.unpinClip);
  const regenerateRange = useStore((s) => s.regenerateRange);

  if (!project) {
    return (
      <div style={{ padding: 24, color: "var(--fg-2)" }}>
        Open a project or import a clip to begin.
      </div>
    );
  }

  return (
    <TimelineCanvas
      timeline={project.timeline}
      selectedItem={selectedItem}
      onSelectItem={selectItem}
      onTogglePin={(trackId, itemId, locked) =>
        locked ? unpinClip(trackId, itemId) : pinClip(trackId, itemId)
      }
      onRegenerateRange={regenerateRange}
    />
  );
}
