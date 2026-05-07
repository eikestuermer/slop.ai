#!/usr/bin/env bash
# pre-shell-cargo-publish-gate.sh
#
# Cursor beforeShellExecution hook. Blocks `cargo publish` and `cargo yank`
# unless the user has explicitly authorized via an `.allow-cargo-publish`
# marker file in the workspace root. Publishing a slop-* crate to crates.io
# is a governance decision — see docs/governance.md and the funding doc —
# not something an agent (or a slipped autocomplete) should do unattended.
#
# Hook contract:
# - stdin: JSON with `command` (full shell command string) and other context.
# - stdout: JSON. To block we return {"permission":"deny", "user_message":"..."}
#   and exit 2. To allow we return {"permission":"allow"} and exit 0.
# - failClosed: true is set in hooks.json so a script crash blocks rather
#   than letting the command through.
#
# False-positive defense (scope):
# The matcher in hooks.json fires whenever the literal substring
# `cargo publish` or `cargo yank` appears in the command string. That
# matcher is broad on purpose — we'd rather have the script decide than
# miss a real publish via a clever subshell. To avoid blocking commands
# that merely *describe* publishing (e.g. a `git commit` whose message
# documents this hook), this script tokenizes the command:
#
# - strips body content of single-quoted strings ('...')
# - strips body content of `<<HEREDOC ... HEREDOC` and `<<'HEREDOC' ... HEREDOC`
# - then checks whether the remaining program text contains a
#   command-position `cargo publish` or `cargo yank` (i.e. preceded by
#   start-of-string, a shell separator like `;`, `&&`, `||`, `|`,
#   newline, or whitespace following one of those).
#
# Inside `"..."` double-quotes we *do* still match: bash expands variables
# inside them, and an attacker could obfuscate via `cargo "publish"` or
# `c"argo publish"`. We accept some over-blocking inside double quotes
# rather than build a full bash tokenizer.

set -u

LOG="$HOME/.cursor/slop-ai-hooks.log"
log() { printf '[%s] %s\n' "$(date -u +%FT%TZ)" "$*" >>"$LOG" 2>/dev/null || true; }

input="$(head -c 262144 || true)"

cmd=""
if command -v jq >/dev/null 2>&1; then
  cmd="$(printf '%s' "$input" | jq -r '.command // .tool_input.command // empty' 2>/dev/null || true)"
else
  cmd="$(printf '%s' "$input" | grep -oE '"command" *: *"[^"]+"' | head -1 | sed -E 's/^"command" *: *"([^"]+)".*/\1/' || true)"
fi

# Find the workspace root. The hook script's $PWD when fired is unreliable;
# try Cursor's commonly-passed `cwd` field, fall back to traversing from
# the script's own location.
ws=""
if command -v jq >/dev/null 2>&1; then
  ws="$(printf '%s' "$input" | jq -r '.workspace_root // .cwd // .workspace_roots[0] // empty' 2>/dev/null || true)"
fi
if [[ -z "$ws" ]]; then
  # Script lives at <repo>/.cursor/hooks/. So the repo root is the parent
  # of the parent of $0.
  script_dir="$(cd "$(dirname "$0")" && pwd)"
  ws="$(cd "$script_dir/../.." && pwd)"
fi

# Allow the pass-through marker. Checked BEFORE the false-positive filter
# so that intentional publishes are never accidentally allowed by the
# filter "deciding" that the command is a false positive.
if [[ -f "$ws/.allow-cargo-publish" ]]; then
  log "publish allowed via $ws/.allow-cargo-publish: $cmd"
  printf '{"permission":"allow"}\n'
  exit 0
fi

# False-positive filter. Use Python (always available on dev machines) to
# strip quoted-string bodies and heredoc bodies, then check for
# command-position cargo publish/yank.
#
# Implementation note: we pass the Python program via `python3 -c` because
# `python3 - <<'PY' ... PY` mixes heredoc and piped stdin and produces a
# corrupted read. With -c, stdin is reserved entirely for the command we're
# inspecting.
GATE_PY='import re, sys
cmd = sys.stdin.read()
# 1. Strip single-quoted bodies. In bash, '"'"'...'"'"' is fully literal.
cmd = re.sub(r"\x27[^\x27]*\x27", "\x27\x27", cmd)
# 2. Strip heredoc bodies: `<<EOF`, `<<-EOF`, `<<'"'"'EOF'"'"'`, `<<\"EOF\"`.
def strip_heredocs(s):
    out = []
    i = 0
    pat = re.compile(r"<<-?\s*(?:\x27([^\x27]+)\x27|\"([^\"]+)\"|([A-Za-z_][A-Za-z0-9_]*))")
    while i < len(s):
        m = pat.search(s, i)
        if not m:
            out.append(s[i:])
            break
        out.append(s[i:m.end()])
        delim = m.group(1) or m.group(2) or m.group(3)
        rest = s[m.end():]
        nl = rest.find("\n")
        if nl < 0:
            out.append(rest)
            break
        out.append(rest[:nl + 1])
        body_start = nl + 1
        end_pat = re.compile(r"^\s*" + re.escape(delim) + r"\s*$", re.MULTILINE)
        em = end_pat.search(rest, body_start)
        if not em:
            break
        out.append(rest[em.start():em.end()])
        i = m.end() + em.end()
    return "".join(out)
cmd = strip_heredocs(cmd)
# 3. Look for command-position cargo publish/yank.
m = re.search(
    r"(?:^|[;&|()]|\\\n|\n)\s*(?:env\s+\S+=\S+\s+)*cargo\s+(publish|yank)\b",
    cmd,
)
print("BLOCK" if m else "ALLOW")
'

if command -v python3 >/dev/null 2>&1; then
  decision="$(printf '%s' "$cmd" | python3 -c "$GATE_PY")"
  if [[ "$decision" == "ALLOW" ]]; then
    log "false positive (cargo publish/yank not in command position): $cmd"
    printf '{"permission":"allow"}\n'
    exit 0
  fi
fi

log "blocked publish/yank: $cmd"
cat <<'JSON'
{
  "permission": "deny",
  "user_message": "Slop AI's cargo gate blocked this command. Publishing or yanking a slop-* crate on crates.io is a governance decision (see docs/governance.md). To proceed: open an RFC in docs/rfcs/, get approval, and create an empty .allow-cargo-publish marker file in the repo root for the duration of the publish session."
}
JSON
exit 2
