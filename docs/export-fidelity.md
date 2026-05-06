# Export fidelity

Slop AI exports rough cuts to several professional NLE formats. This document
tells you exactly what survives the round trip and what does not.

The TL;DR: V1 promises **cuts, trims, markers, captions, and a single linear
speed scalar**. Everything else is a "best effort" target that may degrade.

## Export targets

| Target | Path | Status |
| --- | --- | --- |
| MP4 preview / final draft | direct FFmpeg render from app-state | First-class. The deterministic source of truth for what the cut looks like. |
| OTIO / OTIOZ | written natively by `slop-otio` | First-class. Lossless within the V1 feature subset. |
| Premiere | OTIO -> FCP7 XML adapter | Best effort. Cuts and markers reliable; effects do not survive. |
| DaVinci Resolve | OTIO direct (Resolve 17+); XML/AAF fallback for older | Best effort. Cuts, markers, basic timeline structure reliable. |
| Kdenlive | native OTIO import; `.kdenlive`/MLT XML as fallback | First-class for cuts and markers. |

## What survives reliably

These are the V1 features we promise across all export targets:

- **Cuts** (clip in/out on the timeline)
- **Trims** (source in/out within a clip)
- **Markers** (clip-relative and timeline-relative)
- **Captions** as separate text/subtitle tracks (where the format supports it)
- **Linear speed** scalar per clip (where the format supports it)

## What partially survives

- **Cross-dissolve transitions**: survive in OTIO and Premiere FCP7 XML.
  Best-effort in Resolve. May degrade in older Kdenlive.
- **Fade-in / fade-out**: rendered as audio fades and video opacity ramps in
  the FFmpeg preview, exported as transitions where supported, otherwise
  approximated with adjacent gaps.

## What does not survive (V1 explicitly)

- **Effect stacks beyond fade/dissolve.** Slop AI does not produce them.
- **Speed ramps.** Only a single speed scalar; no curves.
- **Color grades.** Slop AI does not produce them.
- **Per-caption styling.** Captions are plain text in V1.
- **Compound clips / nested timelines.** Slop AI does not produce them.

## Why these limits

The OpenTimelineIO project documents adapter-level gaps. The FCPX XML
adapter, for example, does not support transitions, A/V effects, linear
speed effects, or fancy speed effects. Rather than fight the adapters, V1
deliberately constrains itself to the subset that round-trips cleanly.

If you need effect-perfect interchange, hand off to a real NLE *after* the
rough cut. That is the workflow Slop AI is designed for.

## Test matrix

The CI pipeline runs golden-file regression tests on:

- a hand-edited reference timeline,
- the Slop-generated rough cut for each example project in
  `examples/sample-projects/`.

Each timeline is exported to every format above and compared against checked-in
golden outputs. Tests fail if any byte changes that we have not explicitly
approved.
