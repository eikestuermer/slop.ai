# Schemas

This directory contains the canonical JSON Schemas for Slop AI. All Rust and
TypeScript types are generated from these files. Edit the schemas, then run
the codegen:

```bash
# Rust types: emitted by slop-core's build.rs
cargo build -p slop-core

# TypeScript types
pnpm --filter @slop/schemas codegen
```

## Files

- `timeline.v1.json` - the canonical app-state document. The single source of
  truth for a Slop project. OTIO is *derived* from this; never the other way
  around.
- `ops.v1.json` - the reversible command envelope written to `ops.jsonl`.
  Replaying the log rebuilds project state.
- `plan.v1.json` - the strict contract the planner LLM must satisfy. All
  responses are validated against this schema before any timeline mutation.

## Versioning

- Major bumps (`*.v2.json`) are breaking and require a migration in
  `crates/slop-core/src/migrations/`.
- Minor changes (additive properties, new optional fields) are made in place
  and are non-breaking.

## Why JSON Schema, not Rust types only

Two reasons:

1. The frontend and backend both need exactly the same types. Generating from
   a shared schema avoids drift.
2. The planner LLM consumes the plan schema as a structured-output constraint
   over an OpenAI-compatible HTTP endpoint. The same file is used for both
   server-side validation and prompt-side constraint.
