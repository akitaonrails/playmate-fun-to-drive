#!/usr/bin/env bash
# astra.sh — run a one-shot task on the astra model via the codex CLI,
# billed to the ChatGPT subscription login. See README.md.
set -euo pipefail

ASTRA_MODEL="${ASTRA_MODEL:-gpt-6-astra}"
ASTRA_EFFORT="${ASTRA_EFFORT:-high}"
ASTRA_SANDBOX="${ASTRA_SANDBOX:-workspace-write}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

usage() {
  cat <<'EOF'
Usage: astra.sh [-C DIR] [-i IMAGE]... [-- EXTRA_CODEX_ARGS] "PROMPT"

  -C DIR      working directory for the agent (default: repo root)
  -i IMAGE    attach an image (repeatable; astra accepts image input)
  -- ARGS...  pass remaining flags through to `codex exec`

Env: ASTRA_MODEL (gpt-6-astra) ASTRA_EFFORT (high) ASTRA_SANDBOX (workspace-write)
EOF
}

dir="$ROOT"
imgs=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    -C) dir="$2"; shift 2 ;;
    -i) imgs+=(-i "$2"); shift 2 ;;
    -h|--help) usage; exit 0 ;;
    --) shift; break ;;
    -*) printf 'unknown flag: %s\n' "$1" >&2; usage >&2; exit 2 ;;
    *) break ;;
  esac
done
prompt="${*:-}"
[[ -n "$prompt" ]] || { usage >&2; exit 2; }

# Prompt goes via stdin: codex reads the prompt from stdin whenever stdin is
# piped — and with `-i` images it IGNORES the positional prompt entirely
# (verified empirically on codex 0.162.1). One code path for both cases.
printf '%s' "$prompt" | codex exec \
  --skip-git-repo-check \
  -m "$ASTRA_MODEL" \
  -c model_reasoning_effort="$ASTRA_EFFORT" \
  -s "$ASTRA_SANDBOX" \
  -C "$dir" \
  "${imgs[@]+"${imgs[@]}"}" \
  "$@"
