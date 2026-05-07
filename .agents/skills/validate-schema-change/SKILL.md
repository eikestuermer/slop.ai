---
name: validate-schema-change
description: >-
  Walk through the canonical schema-change workflow when any JSON Schema in
  packages/schemas/ has been edited. Use when the user says "I changed a
  schema", "regen types", "add a field to Asset", or whenever you yourself
  have edited timeline.v1.json / timeline.v2.json / ops.v1.json /
  plan.v1.json. Validates the schema, regenerates TypeScript, updates the
  Rust mirrors, runs the migration tests, and only then is the change
  shippable.
---

# Validate a schema change

The four schemas at [packages/schemas/](packages/schemas/) are the contract that ties Rust, TypeScript, and the planner LLM together. A bad change here is a silent correctness bug that propagates everywhere.

## Inputs

- Which schema changed: `timeline.v1.json`, `timeline.v2.json`, `ops.v1.json`, `plan.v1.json`.
- Whether the change is **additive** (new optional field, new enum variant) or **breaking** (removed field, narrowed type, renamed property).

## Workflow (additive change)

1. Edit the schema file.
2. `cd packages/schemas && node scripts/validate-schemas.mjs` — must exit 0. If ajv rejects, the schema is malformed; fix before proceeding.
3. `node scripts/generate-types.mjs` — regenerates `packages/schemas/generated/*.ts`. The `frontend` CI job will diff and fail if you forget.
4. Update the matching Rust types in [crates/slop-core/src/timeline.rs](crates/slop-core/src/timeline.rs) / [ops.rs](crates/slop-core/src/ops.rs) / [plan.rs](crates/slop-core/src/plan.rs):
   - For optional fields whose schema does NOT allow `null`, use `#[serde(default, skip_serializing_if = "Option::is_none")]`. Otherwise the Rust serializer emits `"field": null` and the validator rejects it. This is the bug Phase A surfaced in `ClipMetadata`.
   - For new enum variants, add the variant.
   - For new required fields, decide on a default and make it explicit.
5. `cargo test -p slop-core` — every existing test must still pass. If a fixture test fails, update it to reflect the new shape.
6. Add a new unit test in the relevant crate covering the new field's behaviour (validator + reducer).

## Workflow (breaking change)

A breaking change requires a new schema version and a migration. Do this when:

- You're removing a field.
- You're narrowing a type (e.g. `string | number` -> `string`).
- You're renaming a property.
- You're changing the meaning of an existing field.

Steps in addition to the additive workflow:

1. Copy `timeline.v1.json` -> `timeline.v2.json` (or `vN -> vN+1`). Edit the new file.
2. Update [packages/schemas/scripts/validate-schemas.mjs](packages/schemas/scripts/validate-schemas.mjs) to include the new schema in its loop. Same for [generate-types.mjs](packages/schemas/scripts/generate-types.mjs).
3. Add a migration in [crates/slop-core/src/migrations/](crates/slop-core/src/migrations/) following [v1_to_v2.rs](crates/slop-core/src/migrations/v1_to_v2.rs):
   - Idempotent (re-running on a v2 doc is a no-op).
   - Lossless from v1 to v2 (every old document loads correctly).
   - Well-tested (at minimum: minimal-doc migration, idempotence, rejects-unknown-version).
4. Wire the migration into the project loader (whichever code does `serde_json::from_str` on `timeline.v?.json` files). Today that's the V1 -> V2 path inside the desktop host's project-load logic.
5. Update the `Migration` table in [docs/schema.md](docs/schema.md) if such a doc exists; otherwise note the new migration in [CHANGELOG.md](CHANGELOG.md).

## After the change

- Run the entire local matrix: `cargo test --workspace`, `pnpm -r typecheck`, `pnpm --filter @slop/desktop test`, `pnpm --filter @slop/schemas validate`.
- If any of these fail, **stop**. The schema change isn't done; you've left an inconsistency between the schema, the Rust types, and the TS types.
- Commit message format: `schema: add <field> to <type> (additive)` or `schema: bump v1 -> v2 (breaking, with migration)`.

## Anti-patterns

- **Don't hand-edit `packages/schemas/generated/`**. Re-run codegen.
- **Don't silently change a schema's `additionalProperties: false` to `true`**. If a schema needs to accept extra fields, that's a deliberate API decision, not a workaround for "the validator complained".
- **Don't add fields with default values in only one of {Rust, TS}**. Defaults belong in the schema.
- **Don't re-use a schema version**. If you bump v1 to v2, never edit v1 again. v1 is part of the historical contract.
