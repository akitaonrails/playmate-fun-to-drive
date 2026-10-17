#!/usr/bin/env bash
# serve.sh — `make serve` backend: start trunk, wait until the port actually
# accepts connections, tell the user, and stay in the foreground (Ctrl-C stops).
set -uo pipefail
cd "$(dirname "$0")/.."
PORT="${SERVE_PORT:-8080}"

cleanup() {
  [[ -n "${TRUNK_PID:-}" ]] && kill "$TRUNK_PID" 2>/dev/null
}
trap cleanup EXIT INT TERM

# a previous dead-watcher trunk can hold the port invisibly — clear it
if pgrep -x trunk >/dev/null 2>&1; then
  echo ">> stopping a leftover trunk instance"
  pgrep -x trunk | xargs -r kill 2>/dev/null
  sleep 1
fi

echo ">> starting trunk (first build can take a couple of minutes)"
(cd crates/game-app && env -u NO_COLOR trunk serve --port "$PORT") &
TRUNK_PID=$!

echo ">> waiting for http://localhost:$PORT to accept connections..."
for i in $(seq 1 240); do
  if curl -fsS -o /dev/null "http://127.0.0.1:$PORT/" 2>/dev/null; then
    echo ""
    echo ">> READY: open http://localhost:$PORT"
    echo ">> controls: ARROWS drive | G gear | Y nitro | R restart | P pause | T flat view | L toy look"
    break
  fi
  if ! kill -0 "$TRUNK_PID" 2>/dev/null; then
    echo "!! trunk exited before serving — check the build output above" >&2
    exit 1
  fi
  sleep 1
done

wait "$TRUNK_PID"
