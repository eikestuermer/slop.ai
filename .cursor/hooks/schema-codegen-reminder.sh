#!/usr/bin/env bash
# schema-codegen-reminder.sh
#
# Cursor afterFileEdit hook. Fires when any file is written by Write or
# TabWrite. Filters script-side for edits to packages/schemas/*.json
# (the canonical JSON Schemas) and emits a reminder to regenerate
# TypeScript types and update the matching Rust mirrors.
#
# Hook contract:
# - stdin: JSON with the edit details. We tolerate both `file_path` and
#   `tool_input.path` shapes since the exact field name has shifted between
#   Cursor versions; we look for both and bail quietly if neither matches.
# - stdout: JSON object. Returning {"agent_message": "..."} on
#   afterFileEdit surfaces a note to the agent's next turn; exit 0 keeps
#   the edit. Exit 2 would block the edit, which we never want here.
# - We never block. This hook is advisory only.

set -u

LOG="$HOME/.cursor/slop-ai-hooks.log"
log() { printf '[%s] %s\n' "$(date -u +%FT%TZ)" "$*" >>"$LOG" 2>/dev/null || true; }

exit_silent() { printf '{}\n'; exit 0; }

# Read stdin (cap at 256 KB; hook input is always tiny).
input="$(head -c 262144 || true)"
if [[ -z "$input" ]]; then exit_silent; fi

# Extract the file path. Try a couple of likely keys.
file_path=""
if command -v jq >/dev/null 2>&1; then
  file_path="$(printf '%s' "$input" | jq -r '
    .file_path // .tool_input.path // .tool_input.file_path //
    .input.file_path // .input.path // empty
  ' 2>/dev/null || true)"
else
  # Fallback: a tolerant grep for any "*path*": "..." line.
  file_path="$(printf '%s' "$input" | grep -oE '"(file_)?path" *: *"[^"]+"' | head -1 | sed -E 's/^[^"]*"[^"]*" *: *"([^"]+)".*/\1/' || true)"
fi

if [[ -z "$file_path" ]]; then exit_silent; fi

# Filter: we only react to packages/schemas/*.json. Note we accept both
# absolute and repo-relative paths.
case "$file_path" in
  *"/packages/schemas/"*.json|"packages/schemas/"*.json) ;;
  *) exit_silent ;;
esac

# Exclude generated/ — it's gitignored and regenerated.
case "$file_path" in
  *"/packages/schemas/generated/"*) exit_silent ;;
esac

log "schema edit detected: $file_path"

cat <<'JSON'
{
  "agent_message": "A JSON Schema in packages/schemas/ was edited. Reminder: run `pnpm --filter @slop/schemas codegen` to regenerate TypeScript types, update the matching Rust mirror in crates/slop-core/src/, and run `pnpm -r typecheck` to confirm. If the change is breaking (removed/renamed field, narrowed type), bump the schema version and add a migration in crates/slop-core/src/migrations/. See .agents/skills/validate-schema-change/SKILL.md."
}
JSON
exit 0
