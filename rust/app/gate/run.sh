#!/usr/bin/env bash
# Synara native-backend parity gate.
# Boots the Rust server against fake provider fixtures and runs the behavioral
# scenario suite in parity_gate.mjs. Exits non-zero when any scenario fails.
set -euo pipefail
cd "$(dirname "$0")/.."
GATE_DIR="$(pwd)/gate"
PORT="${GATE_PORT:-8317}"
HOME_DIR="$(mktemp -d /tmp/synara-gate-home-XXXXXX)"
PROJECT_ID="00000000-0000-4000-8000-gateproject1"
# sqlite ids are uuid-ish; use a valid v4 shape
PROJECT_ID="1188f799-438f-4bd2-8c01-aa0000000001"
SERVER_PID=""

cleanup() {
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  rm -rf "$HOME_DIR"
}
trap cleanup EXIT

echo "== building native server =="
cargo build -q 2>/dev/null || cargo build

echo "== bootstrapping schema =="
mkdir -p "$HOME_DIR/userdata"
DB="$HOME_DIR/userdata/state.sqlite"
sqlite3 "$DB" < "$GATE_DIR/schema.sql"

echo "== seeding test project =="
sqlite3 "$DB" "INSERT OR REPLACE INTO projection_projects
  (project_id, kind, title, workspace_root, scripts_json, created_at, updated_at, is_pinned, additional_folders_json)
  VALUES ('$PROJECT_ID', 'project', 'Gate Project', '/tmp', '[]', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z', 0, '[]');"
sqlite3 "$DB" "update projection_threads set pending_approval_count=0; delete from projection_pending_interactions;" 2>/dev/null || true

echo "== booting gate server =="
FIX="$GATE_DIR/fixtures"
SYNARA_AUTOMATION_TICK_MS=3000 SYNARA_NO_UPSTREAM=1 GH_TOKEN=fake GIT_CONFIG_GLOBAL=/dev/null PORT=$PORT SYNARA_HOME="$HOME_DIR" SYNARA_AUTOMATION_TICK_MS=2000 \
  SYNARA_CLAUDE_EDGE_RETRY_DELAYS_MS=100 SYNARA_CODEX_CLI="$FIX/fake-codex-rlimit" \
  SYNARA_CLAUDE_CLI="$FIX/fake-claude-unified" \
  SYNARA_OMP_CLI="$FIX/fake-omp-acp" \
  GATE_FIXTURES="$FIX" \
  ./target/debug/index --home-dir "$HOME_DIR" >/tmp/synara-gate-server.log 2>&1 &
SERVER_PID=$!
sleep 2

echo "== running scenarios =="
GATE_FIXTURES="$FIX" GATE_REPO_ROOT="$(cd "$GATE_DIR/../../.." && pwd)" node "$GATE_DIR/parity_gate.mjs" "ws://127.0.0.1:$PORT/ws" "$PROJECT_ID" "$DB"
