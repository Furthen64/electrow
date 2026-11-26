#!/usr/bin/env bash
# Start a local backend and headless Chrome, run the browser UI harness, then clean up.
# Output (screenshots and logs) goes to Temp/browser-ui/.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BACKEND_PORT="${ELECTROW_BROWSER_PORT:-8123}"
CDP_PORT="${ELECTROW_CDP_PORT:-9222}"
CHROME="${ELECTROW_CHROME:-google-chrome}"
OUT="$ROOT/Temp/browser-ui"
CIRCUITS="$ROOT/Temp/browser-ui-circuits"
PROFILE="$ROOT/Temp/browser-ui-chrome"

if ! command -v "$CHROME" >/dev/null 2>&1; then
  echo "browser not found: $CHROME (set ELECTROW_CHROME)" >&2
  exit 2
fi
if ! command -v node >/dev/null 2>&1; then
  echo "node is required for the CDP harness (Node 22+)" >&2
  exit 2
fi

mkdir -p "$OUT" "$CIRCUITS"
rm -rf "$PROFILE"
mkdir -p "$PROFILE"

BACKEND_PID=""
CHROME_PID=""
cleanup() {
  if [ -n "$CHROME_PID" ]; then kill "$CHROME_PID" 2>/dev/null || true; fi
  if [ -n "$BACKEND_PID" ]; then kill "$BACKEND_PID" 2>/dev/null || true; fi
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

(
  cd "$ROOT" || exit 1
  ELECTROW_CIRCUITS_DIR="$CIRCUITS" python -m backend --host 127.0.0.1 --port "$BACKEND_PORT"
) > "$OUT/backend.log" 2>&1 &
BACKEND_PID=$!

"$CHROME" --headless --no-sandbox --disable-gpu --disable-dev-shm-usage \
  --remote-debugging-port="$CDP_PORT" --user-data-dir="$PROFILE" \
  "http://127.0.0.1:$BACKEND_PORT/" > "$OUT/chrome.log" 2>&1 &
CHROME_PID=$!

ready=0
for _ in $(seq 1 60); do
  if curl -fsS "http://127.0.0.1:$BACKEND_PORT/api/health" >/dev/null 2>&1 &&
     curl -fsS "http://127.0.0.1:$CDP_PORT/json/list" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 0.25
done
if [ "$ready" -ne 1 ]; then
  echo "backend or Chrome did not become ready; see $OUT/backend.log and $OUT/chrome.log" >&2
  exit 2
fi

node "$ROOT/tests/browser_ui.cjs" \
  --base "http://127.0.0.1:$BACKEND_PORT" \
  --cdp "http://127.0.0.1:$CDP_PORT" \
  --out "$OUT"
