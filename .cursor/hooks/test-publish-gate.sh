#!/usr/bin/env bash
# Smoke test for the publish gate. Reads test JSON from this file and
# pipes it to the gate script. Self-contained so the literal "publish"
# string isn't typed into a shell command line that the gate would block.
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"
GATE="$SCRIPT_DIR/pre-shell-cargo-publish-gate.sh"

# Helper: jq-build a shell-safe JSON envelope around a command string.
mk() {
  if command -v jq >/dev/null 2>&1; then
    jq -nc --arg cmd "$1" --arg ws "$REPO_ROOT" '{command:$cmd,workspace_root:$ws}'
  else
    # Crude fallback. Won't survive embedded quotes; OK for our test fixtures.
    printf '{"command":"%s","workspace_root":"%s"}' "$1" "$REPO_ROOT"
  fi
}

run_case() {
  local name="$1" expect="$2" cmd="$3"
  local out rc
  out="$(printf '%s' "$(mk "$cmd")" | "$GATE" || true)"
  case "$out" in
    *"$expect"*) echo "PASS: $name";;
    *) echo "FAIL: $name (expected '$expect'). Output:"; echo "$out"; exit 1;;
  esac
}

# === True positives: should block ===

# 1. Bare cargo publish.
run_case "bare cargo publish blocks" "deny" "cargo publish -p slop-core"

# 2. cargo yank.
run_case "cargo yank blocks" "deny" "cargo yank --vers 0.1.0 slop-core"

# 3. After a shell separator (real cmd in a chain).
run_case "after && blocks" "deny" "cd crates/slop-core && cargo publish"

# 4. Subshell.
run_case "subshell still blocks" "deny" "(cargo publish)"

# 5. With env prefix.
run_case "env prefix blocks" "deny" "env CARGO_REGISTRY_TOKEN=x cargo publish"

# === False positives: should allow ===

# 6. Documentation in a single-quoted string (e.g. commit message).
run_case "single-quoted commit message allowed" "allow" "git commit -m 'docs: explain the cargo publish gate'"

# 7. Documentation in a heredoc.
run_case "heredoc body allowed" "allow" "git commit -m \"\$(cat <<'EOF'
chore: rewrite gate

Blocks cargo publish in command position. False-positive defense:
heredoc bodies and single-quoted strings are stripped before matching.
EOF
)\""

# 8. echo of a command (logging).
run_case "echo in single quotes allowed" "allow" "echo 'tomorrow we will run cargo publish'"

# === Marker bypass ===

# 9. With the marker, even a real publish is allowed.
touch .allow-cargo-publish
run_case "marker bypass allows real publish" "allow" "cargo publish -p slop-core"
rm -f .allow-cargo-publish

echo
echo "all gate tests pass"

