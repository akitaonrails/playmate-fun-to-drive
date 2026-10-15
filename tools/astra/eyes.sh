#!/usr/bin/env bash
# eyes.sh — visual QA through astra (which accepts image input).
# Usage:
#   eyes.sh [-C DIR] [-e EXPECTATIONS.md] IMAGE...
#   echo "expectations..." | eyes.sh IMAGE...
# Sends the images + expectations to astra and prints a terse critique.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'EOF'
Usage: eyes.sh [-C DIR] [-e FILE] IMAGE [IMAGE...]

  -e FILE  expectations markdown (default: stdin; falls back to a generic QA ask)
EOF
}

exp_file=""
dir=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    -e) exp_file="$2"; shift 2 ;;
    -C) dir="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    -*) printf 'unknown flag: %s\n' "$1" >&2; usage >&2; exit 2 ;;
    *) break ;;
  esac
done
imgs=("$@")
[[ ${#imgs[@]} -ge 1 ]] || { usage >&2; exit 2; }
for f in "${imgs[@]}"; do
  [[ -f "$f" ]] || { printf 'not a file: %s\n' "$f" >&2; exit 2; }
done

if [[ -n "$exp_file" ]]; then
  expectations="$(cat "$exp_file")"
elif [[ ! -t 0 ]]; then
  expectations="$(cat)"
else
  expectations="(none provided — judge against the project docs/PLAN.md milestone in general terms)"
fi

prompt="You are the visual QA eyes for 'playmate-fun-to-drive-web' (a web game
recreating an 80s/90s dashboard driving toy; see docs/ in the repo if needed).
You are given screenshot(s), in order. Check them ONLY against the written
expectations below — do not invent requirements.

EXPECTATIONS:
$expectations

Reply in this exact shape, terse:
VERDICT: PASS | FAIL
PER-EXPECTATION:
- <expectation> : PASS|FAIL — <one line of evidence>
ISSUES:
- <concrete visual bug + where>   (omit if none)
TWEAKS:
- <suggested numeric/visual adjustment>   (omit if none)"

args=()
[[ -n "$dir" ]] && args+=(-C "$dir")
for f in "${imgs[@]}"; do args+=(-i "$f"); done

ASTRA_EFFORT="${ASTRA_EFFORT:-low}" "$here/astra.sh" "${args[@]}" "$prompt"
