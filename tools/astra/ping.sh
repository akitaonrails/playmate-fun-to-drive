#!/usr/bin/env bash
# ping.sh — verify the codex subscription login and the astra model round-trip.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "== codex login =="
codex login status

echo "== astra round-trip (model: ${ASTRA_MODEL:-gpt-6-astra}) =="
ASTRA_EFFORT=low ASTRA_SANDBOX=read-only \
  "$here/astra.sh" "Reply with exactly: ASTRA-OK"
