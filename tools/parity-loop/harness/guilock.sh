#!/usr/bin/env bash
# Usage: guilock.sh <owner> <command...>  — run command while holding the shared GUI lock.
set -euo pipefail
owner="$1"; shift
lock="$HOME/parity/.guilock"
while ! mkdir "$lock" 2>/dev/null; do
  holder="$(cat "$lock/pid" 2>/dev/null || true)"
  if [[ -n "$holder" ]] && ! kill -0 "$holder" 2>/dev/null; then
    rm -rf "$lock"; continue
  fi
  sleep 3
done
echo "$$" > "$lock/pid"; echo "$owner" > "$lock/owner"
trap 'rm -rf "$lock"' EXIT
"$@"
