#!/usr/bin/env bash
# Verify that Trunk produced a self-contained bundle suitable for a GitHub
# Pages project path, rather than only for an origin root.
set -euo pipefail

DIST="${1:-crates/game-app/dist}"
INDEX="$DIST/index.html"

if [[ ! -s "$INDEX" ]]; then
  echo "missing built index: $INDEX" >&2
  exit 1
fi
if [[ -z "$(find "$DIST" -maxdepth 1 -type f -name '*.wasm' -print -quit)" ]]; then
  echo "missing WebAssembly output in $DIST" >&2
  exit 1
fi
if [[ -z "$(find "$DIST" -maxdepth 1 -type f -name '*.js' -print -quit)" ]]; then
  echo "missing JavaScript loader in $DIST" >&2
  exit 1
fi
if ! grep -q 'id="fullscreen-toggle"' "$INDEX"; then
  echo "fullscreen control was not included in the built page" >&2
  exit 1
fi
if grep -Eq "(href|src)=[\"']/|from [\"']/|module_or_path: [\"']/" "$INDEX"; then
  echo "root-relative asset URL found; the bundle would break on a Pages project path" >&2
  exit 1
fi

for track in \
  music_voltage_hearts music_coastal_splash music_passing_lights \
  music_redline_overture music_overpass_anthem music_static_signal
do
  if [[ ! -s "$DIST/assets/audio/$track.ogg" ]]; then
    echo "missing soundtrack asset: $track.ogg" >&2
    exit 1
  fi
done

for effect in hurricane ufo
do
  if [[ ! -s "$DIST/assets/audio/$effect.ogg" ]]; then
    echo "missing rare-event audio asset: $effect.ogg" >&2
    exit 1
  fi
done

if [[ ! -s "$DIST/assets/shaders/crt_screen.wesl" ]]; then
  echo "missing CRT shader from production bundle" >&2
  exit 1
fi

echo "web bundle: OK ($DIST)"
