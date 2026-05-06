import { useMemo, useState } from "react";
import type { SlopTimeline } from "@slop/schemas";

interface TimelineCanvasProps {
  timeline: SlopTimeline;
  selectedItem: string | null;
  onSelectItem: (id: string | null) => void;
  onTogglePin: (trackId: string, itemId: string, locked: boolean) => void;
  onRegenerateRange: (
    trackId: string,
    timelineIn: number,
    timelineOut: number,
  ) => void;
}

const PX_PER_SEC = 60;
const TRACK_HEIGHT = 56;
const RULER_HEIGHT = 24;

export function TimelineCanvas({
  timeline,
  selectedItem,
  onSelectItem,
  onTogglePin,
  onRegenerateRange,
}: TimelineCanvasProps) {
  const totalDuration = useMemo(() => {
    let max = 0;
    for (const t of timeline.tracks) {
      for (const i of t.items) {
        if ("timeline_out" in i && i.timeline_out > max) max = i.timeline_out;
      }
    }
    return Math.max(max, 30);
  }, [timeline]);

  const totalWidth = totalDuration * PX_PER_SEC;
  const totalHeight = RULER_HEIGHT + timeline.tracks.length * TRACK_HEIGHT;

  const [range, setRange] = useState<{
    trackId: string;
    in: number;
    out: number;
  } | null>(null);

  return (
    <div style={{ position: "relative", padding: 12 }}>
      <div
        style={{
          position: "relative",
          width: totalWidth,
          height: totalHeight,
          background: "var(--bg-1)",
          borderRadius: 6,
          border: "1px solid var(--bg-2)",
          overflow: "hidden",
        }}
        onClick={(e) => {
          if (e.target === e.currentTarget) onSelectItem(null);
        }}
      >
        <Ruler totalDuration={totalDuration} />
        {timeline.tracks.map((t, ti) => (
          <TrackRow
            key={t.track_id}
            track={t}
            trackIndex={ti}
            selectedItem={selectedItem}
            onSelectItem={onSelectItem}
            onTogglePin={onTogglePin}
            onSetRange={(inSec, outSec) =>
              setRange({ trackId: t.track_id, in: inSec, out: outSec })
            }
          />
        ))}
        {range && (
          <div
            style={{
              position: "absolute",
              top: RULER_HEIGHT,
              left: range.in * PX_PER_SEC,
              width: (range.out - range.in) * PX_PER_SEC,
              height: timeline.tracks.length * TRACK_HEIGHT,
              background: "var(--accent-soft)",
              border: "1px solid var(--accent)",
              pointerEvents: "none",
            }}
          />
        )}
      </div>
      {range && (
        <div
          style={{
            display: "flex",
            gap: 8,
            marginTop: 8,
            alignItems: "center",
          }}
        >
          <span style={{ color: "var(--fg-2)", fontSize: 12 }}>
            Range {range.in.toFixed(2)}s → {range.out.toFixed(2)}s on{" "}
            {range.trackId}
          </span>
          <button
            onClick={() => {
              onRegenerateRange(range.trackId, range.in, range.out);
            }}
          >
            Regenerate this range
          </button>
          <button onClick={() => setRange(null)}>Clear</button>
        </div>
      )}
    </div>
  );
}

function Ruler({ totalDuration }: { totalDuration: number }) {
  const ticks = [];
  for (let s = 0; s <= totalDuration; s += 5) {
    ticks.push(
      <div
        key={s}
        style={{
          position: "absolute",
          left: s * PX_PER_SEC,
          top: 0,
          bottom: 0,
          borderLeft: "1px solid var(--bg-3)",
          paddingLeft: 4,
          fontSize: 10,
          color: "var(--fg-2)",
        }}
      >
        {s}s
      </div>,
    );
  }
  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        right: 0,
        top: 0,
        height: RULER_HEIGHT,
        background: "var(--bg-2)",
        borderBottom: "1px solid var(--bg-3)",
      }}
    >
      {ticks}
    </div>
  );
}

interface TrackRowProps {
  track: SlopTimeline["tracks"][number];
  trackIndex: number;
  selectedItem: string | null;
  onSelectItem: (id: string | null) => void;
  onTogglePin: (trackId: string, itemId: string, locked: boolean) => void;
  onSetRange: (inSec: number, outSec: number) => void;
}

function TrackRow({
  track,
  trackIndex,
  selectedItem,
  onSelectItem,
  onTogglePin,
  onSetRange,
}: TrackRowProps) {
  const top = RULER_HEIGHT + trackIndex * TRACK_HEIGHT;
  const [drag, setDrag] = useState<{
    startX: number;
    startSec: number;
  } | null>(null);

  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        right: 0,
        top,
        height: TRACK_HEIGHT,
        borderBottom: "1px solid var(--bg-2)",
        background:
          track.kind === "video" ? "rgba(50,150,255,0.04)" : "rgba(50,255,150,0.03)",
      }}
      onMouseDown={(e) => {
        if (e.target !== e.currentTarget) return;
        const rect = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
        const x = e.clientX - rect.left;
        setDrag({ startX: x, startSec: x / PX_PER_SEC });
      }}
      onMouseUp={(e) => {
        if (drag) {
          const rect = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
          const endX = e.clientX - rect.left;
          const endSec = Math.max(drag.startSec, endX / PX_PER_SEC);
          if (endSec - drag.startSec > 0.5) {
            onSetRange(drag.startSec, endSec);
          }
          setDrag(null);
        }
      }}
    >
      <div
        style={{
          position: "absolute",
          left: 4,
          top: 4,
          fontSize: 10,
          color: "var(--fg-2)",
          textTransform: "uppercase",
          letterSpacing: 0.5,
          pointerEvents: "none",
        }}
      >
        {track.track_id} ({track.kind})
      </div>
      {track.items.map((item) => {
        if (item.type !== "clip") {
          return (
            <div
              key={item.item_id}
              style={{
                position: "absolute",
                left: item.timeline_in * PX_PER_SEC,
                width:
                  Math.max(item.timeline_out - item.timeline_in, 0.1) *
                  PX_PER_SEC,
                top: 14,
                height: TRACK_HEIGHT - 18,
                background: "transparent",
                border: "1px dashed var(--bg-3)",
                borderRadius: 4,
              }}
            />
          );
        }
        const c = item;
        const locked = c.metadata?.locked_by_user ?? false;
        const score = c.metadata?.score;
        const selected = selectedItem === c.item_id;
        return (
          <div
            key={c.item_id}
            onClick={(ev) => {
              ev.stopPropagation();
              onSelectItem(c.item_id);
            }}
            onDoubleClick={(ev) => {
              ev.stopPropagation();
              onTogglePin(track.track_id, c.item_id, locked);
            }}
            style={{
              position: "absolute",
              left: c.timeline_in * PX_PER_SEC,
              width:
                Math.max(c.timeline_out - c.timeline_in, 0.1) * PX_PER_SEC,
              top: 14,
              height: TRACK_HEIGHT - 18,
              background: locked ? "var(--accent-soft)" : "var(--bg-3)",
              border: selected
                ? "2px solid var(--accent)"
                : "1px solid var(--bg-3)",
              borderRadius: 4,
              cursor: "pointer",
              padding: "2px 6px",
              fontSize: 11,
              color: "var(--fg-0)",
              overflow: "hidden",
              whiteSpace: "nowrap",
              textOverflow: "ellipsis",
            }}
            title={c.metadata?.selection_reason ?? c.item_id}
          >
            {locked && "📌 "}
            {c.metadata?.selection_reason ?? c.asset_id}
            {score !== undefined && (
              <span style={{ color: "var(--fg-2)", marginLeft: 4 }}>
                {score.toFixed(2)}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}
