#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${ROOT}/target/release/prove"
if [[ ! -x "$BIN" ]]; then
  cargo build --release --manifest-path "$ROOT/Cargo.toml"
fi
echo "=== eval traps ==="
"$BIN" eval traps
run_one() {
  local name="$1" backend="$2"
  local TMP
  TMP="$(mktemp -d)"
  cp -R "$ROOT/fixtures/traps/visible-pass-hidden-fail/." "$TMP/"
  cd "$TMP"
  echo "=== $name ($backend) ==="
  "$BIN" init >/dev/null
  cat > .prove/policy.yml <<'YAML'
gates:
  test:
    commands:
      - [python, -m, pytest, -q, tests/test_visible.py]
      - [python, -m, pytest, -q, tests/test_hidden_contract.py]
    repair_limit: 2
  review:
    type: checklist
    require: [diff_non_empty, tests_fresh, no_todo_marker]
budgets:
  max_steps: 20
  max_minutes: 10
safety:
  deny_command_regex: []
YAML
  "$BIN" run "fix discount validation" --backend "$backend"
  "$BIN" status
  if [[ "$backend" == "local-loop" ]]; then
    "$BIN" pr
  fi
}
run_one naive_block naive
run_one local_done local-loop
echo "Demo complete."
