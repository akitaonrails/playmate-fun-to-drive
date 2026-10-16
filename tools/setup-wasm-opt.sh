#!/usr/bin/env bash
# setup-wasm-opt.sh — make trunk's cached wasm-opt tolerate bulk-memory ops.
#
# trunk 0.21 invokes wasm-opt with a bare `-O`; binaryen releases still
# validate memory.copy only with --all-features, so release
# builds fail ("memory.copy operations require bulk memory operations").
# This installs a wrapper over the cached binaryen 131 tool that appends
# the flag. Idempotent.
set -euo pipefail

CACHE="$HOME/.cache/trunk/wasm-opt-version_131/bin"
REAL="$CACHE/wasm-opt.real"
WRAP="$CACHE/wasm-opt"
URL="https://github.com/WebAssembly/binaryen/releases/download/version_131/binaryen-version_131-x86_64-linux.tar.gz"

if [[ -x "$REAL" ]]; then
  exit 0
fi
mkdir -p "$CACHE"

# trunk may already have downloaded the plain binary into the cache
if [[ ! -x "$CACHE/wasm-opt" ]]; then
  TMP=$(mktemp -d)
  curl -fsSL "$URL" -o "$TMP/b.tar.gz"
  tar -xzf "$TMP/b.tar.gz" -C "$TMP"
  cp "$TMP"/binaryen-version_131/bin/wasm-opt "$CACHE/wasm-opt"
  rm -rf "$TMP"
fi

cp "$CACHE/wasm-opt" "$REAL"
cat > "$WRAP" <<EOF
#!/bin/sh
exec "$REAL" "\$@" --all-features
EOF
chmod +x "$WRAP" "$REAL"
echo "wasm-opt wrapper installed at $WRAP"
