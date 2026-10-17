#!/usr/bin/env bash
# ci.sh — full local CI: fmt, clippy, tests, native build, wasm build.
# (Screenshot smoke runs separately via `make shots` while `make serve` runs.)
set -euo pipefail
cd "$(dirname "$0")/.."
make check
make build-wasm
make verify-web
echo "ci: OK"
