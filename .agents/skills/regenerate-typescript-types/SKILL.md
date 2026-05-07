---
name: regenerate-typescript-types
description: >-
  Regenerate TypeScript types from the JSON Schemas in packages/schemas/.
  Use when the user says "regen schemas", "TS types out of date", "I edited
  a schema", or as part of any schema-change workflow. This is a one-liner
  in normal cases; the skill exists to make the dependency between schema
  edits and downstream typecheck/lint visible.
---

# Regenerate TypeScript types

The four JSON Schemas at [packages/schemas/](packages/schemas/) drive the TypeScript types in [packages/schemas/generated/](packages/schemas/generated/) (gitignored). The frontend imports from `@slop/schemas`. After editing a schema, regenerate.

## Workflow

```bash
cd /Users/eike/slop.ai
pnpm --filter @slop/schemas validate   # exits 0 only if all 4 schemas are valid
pnpm --filter @slop/schemas codegen    # writes 4 .ts files + index.ts
pnpm -r typecheck                      # confirms no TS package broke
```

If `validate` fails, the schema is malformed; fix the JSON before regenerating.

If `codegen` succeeds but `typecheck` fails, the frontend uses a field whose name or shape changed. Either:

- update the frontend to match the new shape, or
- back out the schema change and reconsider — schemas are an API.

## Why it isn't fully automatic

A pre-save hook *could* run codegen, but:

- Codegen is fast (~200ms) but not free; running it on every save would be noticeable.
- The CI `frontend` job re-runs codegen and the TS typecheck, so a missed regen is caught before merge.
- The work of updating the Rust types in [crates/slop-core/src/](crates/slop-core/src/) to match a schema change is manual anyway; bundling a chunk of human review into one explicit step is cleaner.

So we keep it manual. The [validate-schema-change](../validate-schema-change/SKILL.md) skill is the master workflow that calls this one as step 3.

## Anti-patterns

- **Don't hand-edit `packages/schemas/generated/`**. They're gitignored and regenerated; your edits are lost on next codegen.
- **Don't commit before re-running typecheck**. The CI gate will catch it but it's a wasted CI run.
