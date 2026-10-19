#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
port=${BROWSER_TEST_PORT:-4173}
log_file=$(mktemp)
server_pid=""

cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -f "$log_file"
}
trap cleanup EXIT

python3 -m http.server "$port" --bind 127.0.0.1 \
  --directory "$project_root/crates/game-app/dist" >"$log_file" 2>&1 &
server_pid=$!

for _ in {1..50}; do
  if curl --fail --silent --output /dev/null "http://127.0.0.1:$port/"; then
    npm --prefix "$project_root/tools/screenshots" run endurance -- \
      --url "http://127.0.0.1:$port/?quality=low&qa_session_secs=30&qa_start_environment_time=99.8"
    exit 0
  fi
  sleep 0.1
done

cat "$log_file" >&2
echo "browser test server did not start" >&2
exit 1
