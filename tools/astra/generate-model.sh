#!/usr/bin/env bash
# generate-model.sh — 3D asset pipeline (DECISIONS.md D3):
#   spec -> astra writes a deterministic Blender python script
#         -> headless blender exports glTF + parts manifest
# Usage: generate-model.sh <spec.md> <name> [--no-blender]
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$here/../.." && pwd)"
BLENDER="${ASTRA_BLNDER:-blender}"

[[ $# -ge 2 ]] || { printf 'usage: %s <spec.md> <name> [--no-blender]\n' "$0" >&2; exit 2; }
spec="$1"; name="$2"; run_blender=1
[[ "${3:-}" == "--no-blender" ]] && run_blender=0
[[ -f "$spec" ]] || { printf 'spec not found: %s\n' "$spec" >&2; exit 2; }
spec_rel="$(realpath --relative-to="$ROOT" "$spec")"

mkdir -p "$ROOT/assets/blender" "$ROOT/assets/models"
script="$ROOT/assets/blender/${name}.py"
glb="$ROOT/assets/models/${name}.glb"

prompt="Task: author ONE deterministic Blender generator script for a toy part.

Read the part spec at: ${spec_rel}
Also skim research/toy-mechanics.md and research/images/ for context if useful
(you may attach no images; rely on the written specs).

Write the file assets/blender/${name}.py — plain 'bpy' Python 3, and it MUST:
  - be fully deterministic: no randomness without a fixed seed, no time/date,
    no external downloads, no gui operators that need a window;
  - be parametric: expose a PARAMS dict at the top (dimensions, segment
    counts, colors) matching the spec's numbers;
  - run headless: 'blender --background --factory-startup --python <script>'
    executed from the repo root must exit 0;
  - build every part the spec lists as a SEPARATE object, named exactly as
    the spec's 'Part names', each with its pivot/origin at the specified
    pivot point (parent empties allowed, named '<Part>_Pivot' where useful);
  - use only primitives + modifiers (no sculpt, no external assets);
  - assign simple Principled BSDF materials per the spec (matte plastic);
  - export via bpy.ops.export_scene.gltf to assets/models/${name}.glb
    (embedded data) AND write assets/models/${name}.manifest.json listing:
    parts, their world-space rest transforms, pivot descriptions, material
    names, and mesh counts — so the engine can assert on it;
  - end with a summary print of parts created.

Constraints: keep polygon counts toy-appropriate (spec budget). Do not
modify anything else in the repo. After writing, run the script headless
yourself with: blender --background --factory-startup --python assets/blender/${name}.py
(from the repo root), fix until it exits 0, and report the part list +
vertex counts. Blender binary: ${BLENDER}"

"$here/astra.sh" "$prompt"

if [[ $run_blender -eq 1 ]] && command -v "$BLENDER" >/dev/null 2>&1; then
  echo "== re-running ${name}.py headless for reproducibility check =="
  ( cd "$ROOT" && "$BLENDER" --background --factory-startup --python "$script" )
fi

[[ -s "$glb" ]] && printf 'OK %s\n' "$glb" || { printf 'MISSING %s\n' "$glb" >&2; exit 1; }

cat <<'HINT'
NOTE: the app embeds the glb via include_bytes. If `trunk serve` is running,
its watcher will NOT notice the new file — touch crates/game-app/src/main.rs
or restart the server to force a wasm rebuild.
HINT

