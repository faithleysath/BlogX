#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

SITE="$TMP_DIR/site"
"$ROOT/target/debug/blogx" init "$SITE" >/dev/null

PORT="${BLOGX_SMOKE_PORT:-8933}"
HOST="127.0.0.1"
LOG="$TMP_DIR/serve.log"
(cd "$SITE" && "$ROOT/target/debug/blogx" serve --host "$HOST" --port "$PORT" >"$LOG" 2>&1 & echo $! >"$TMP_DIR/pid")

for _ in $(seq 1 60); do
  if curl -fsS "http://$HOST:$PORT/" >/tmp/blogx-serve-watch-before.html 2>/dev/null; then
    break
  fi
  sleep 0.25
done

cat >"$SITE/content/index.md" <<'MARKDOWN'
---
title: Watched
---

# Watched

Updated by serve smoke.
MARKDOWN

for _ in $(seq 1 80); do
  curl -fsS "http://$HOST:$PORT/" >/tmp/blogx-serve-watch-after.html 2>/dev/null || true
  if grep -q "Updated by serve smoke" /tmp/blogx-serve-watch-after.html; then
    break
  fi
  sleep 0.25
done

kill "$(cat "$TMP_DIR/pid")" 2>/dev/null || true
wait "$(cat "$TMP_DIR/pid")" 2>/dev/null || true

grep -q "detected" "$LOG"
grep -q "rebuilding" "$LOG"
grep -q "Updated by serve smoke" /tmp/blogx-serve-watch-after.html

echo "serve watch smoke passed"
