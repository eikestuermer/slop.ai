# Non-goals (V1)

This document is the explicit "we are not building this" list. Every sprint
review checks against this list. Adding to V1 scope requires updating this
file first.

## Out of scope for V1

### Editing features

- **Complex transitions.** V1 supports cuts, fade-in, fade-out, and a single
  cross-dissolve effect. No wipes, dips-to-color, or curve-driven transitions.
- **Effect stacks.** No keyframed color, no LUTs, no warp stabilizer, no
  audio plug-ins, no per-clip EQ.
- **Speed ramps.** A single linear `speed` scalar per clip is supported. No
  animated speed curves.
- **Compound clips / nested timelines.** One flat composition.
- **Multicam.** No multi-angle sync or angle switching.
- **Color grading.** No color wheels, scopes, or grade carry.
- **J/L cuts as first-class.** They can be modeled by manual offset, but
  there is no UI affordance.
- **Subtitles styling.** Captions are plain text in V1; no positioning,
  fonts, or per-line colors.

### Workflow features

- **Collaborative editing.** Single-user, single-machine.
- **Cloud sync.** No project-state cloud storage.
- **Mobile.** Desktop only.
- **Marketplace / plugins.** Wasmtime plugin scaffolding may exist (inherited
  from the OpenReelio shell) but is not a first-class V1 feature.

### Export fidelity

- We do not promise round-trip fidelity to professional NLEs for anything
  beyond cuts, markers, captions, and a single linear speed scalar. See
  `docs/export-fidelity.md` for the complete promise.
- We do not promise round-trip of effects via OTIO; OTIO's own adapters
  document gaps for transitions, A/V effects, and speed ramps.

### AI features

- **Auto-color.** No.
- **Auto-leveling.** Loudness normalization is in scope as a render-time
  filter, but is not surfaced as an AI feature.
- **Voice cloning, dubbing, translation.** No.
- **Image / video generation.** No. Slop AI is a rough-cut editor, not a
  generator. Hooks for generated B-roll might appear post-V1.
- **Online model fine-tuning.** No. The planner is stateless; we do not
  train on user content.

## When to update this list

If you find yourself implementing one of these capabilities, stop and:

1. Open an issue describing why V1 needs it.
2. Get explicit reviewer approval to move it to in-scope.
3. Update this file *first* in the same PR.

The point of this list is not to be precious; it is to keep the V1 roadmap
honest.
