#!/usr/bin/env bash
# SDK smoke: boot a real gigi-stream, run the python SDK end-to-end
# against it, then the js SDK contract tests. Used by CI (sdk-smoke job)
# and runnable locally:
#
#     ./scripts/sdk_smoke.sh            # builds release binary first
#     GIGI_BIN=target/release/gigi-stream ./scripts/sdk_smoke.sh
#
# Exits nonzero if any layer breaks its promise.
set -euo pipefail
cd "$(dirname "$0")/.."

PORT="${GIGI_SMOKE_PORT:-3142}"
# loopback must not be routed through a corporate/agent proxy
export NO_PROXY="localhost,127.0.0.1${NO_PROXY:+,$NO_PROXY}"
export no_proxy="$NO_PROXY"
export GIGI_URL="http://127.0.0.1:${PORT}"
DATA_DIR="$(mktemp -d)"
BIN="${GIGI_BIN:-target/release/gigi-stream}"

if [ ! -x "$BIN" ]; then
  echo "building gigi-stream…"
  cargo build --release --bin gigi-stream
fi

echo "booting gigi-stream on :${PORT} (data: ${DATA_DIR})"
GIGI_DATA_DIR="$DATA_DIR" PORT="$PORT" "$BIN" &
SERVER_PID=$!
cleanup() {
  kill "$SERVER_PID" 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true
  rm -rf "$DATA_DIR"
}
trap cleanup EXIT

# wait for the ready flip (503 before it is correct behavior)
for i in $(seq 1 60); do
  if curl -sf "${GIGI_URL}/v1/health" >/dev/null 2>&1; then break; fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "gigi-stream died during boot" >&2; exit 1
  fi
  sleep 0.5
done
curl -sf "${GIGI_URL}/v1/health" >/dev/null || { echo "server never became healthy" >&2; exit 1; }

echo "— python SDK smoke —"
python3 scripts/sdk_smoke.py

echo "— js SDK contract tests —"
if command -v node >/dev/null 2>&1; then
  (cd sdk/js && node --test test/client.test.js)
else
  echo "node not installed — skipping js SDK tests" >&2
fi

echo "SDK smoke: PASS"
