import type { SlopTimeline } from "@slop/schemas";
import { useStore } from "../store";

type TrackItem = SlopTimeline["tracks"][number]["items"][number];

export function Inspector() {
  const project = useStore((s) => s.project);
  const selectedItem = useStore((s) => s.ui.selectedItem);
  const pinClip = useStore((s) => s.pinClip);
  const unpinClip = useStore((s) => s.unpinClip);

  if (!project || !selectedItem) {
    return (
      <div style={{ padding: 12, color: "var(--fg-2)" }}>
        Select a clip to see why the planner picked it.
      </div>
    );
  }

  let trackId: string | null = null;
  let clip: TrackItem | null = null;
  for (const t of project.timeline.tracks) {
    for (const item of t.items) {
      if ("item_id" in item && item.item_id === selectedItem) {
        trackId = t.track_id;
        clip = item;
        break;
      }
    }
    if (clip) break;
  }
  if (!clip || !trackId) {
    return (
      <div style={{ padding: 12, color: "var(--fg-2)" }}>
        Selected item not found.
      </div>
    );
  }
  if (clip.type !== "clip") {
    return (
      <div style={{ padding: 12, color: "var(--fg-2)" }}>
        Selected a gap. Cannot inspect.
      </div>
    );
  }

  const meta = clip.metadata ?? {};
  const locked = meta.locked_by_user ?? false;

  return (
    <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 12 }}>
      <h3 style={{ margin: 0 }}>Clip {clip.item_id.slice(0, 10)}</h3>
      <div style={{ fontSize: 11, color: "var(--fg-2)" }}>
        on track {trackId}
      </div>
      <Field label="Source range">
        {clip.src_in.toFixed(2)}s → {clip.src_out.toFixed(2)}s
      </Field>
      <Field label="Timeline range">
        {clip.timeline_in.toFixed(2)}s → {clip.timeline_out.toFixed(2)}s
      </Field>
      <Field label="Speed">{(clip.speed ?? 1).toFixed(2)}x</Field>
      {meta.score !== undefined && (
        <Field label="Score">{meta.score.toFixed(3)}</Field>
      )}
      {meta.selection_reason && (
        <Field label="Why">
          <em>{meta.selection_reason}</em>
        </Field>
      )}

      <button
        className={locked ? "" : "primary"}
        onClick={() =>
          locked
            ? void unpinClip(trackId!, clip.item_id).catch(() => undefined)
            : void pinClip(trackId!, clip.item_id).catch(() => undefined)
        }
      >
        {locked ? "Unpin" : "Pin (protect from regeneration)"}
      </button>
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
    <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
      <span style={{ fontSize: 10, color: "var(--fg-2)", textTransform: "uppercase", letterSpacing: 0.5 }}>
        {label}
      </span>
      <span>{children}</span>
    </div>
  );
}
