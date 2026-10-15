# tools/astra — codex-harness instrumentation for the astra model

One-shot, non-interactive access to OpenAI's **astra** (`gpt-6-astra`) through
the locally installed `codex` CLI, billed to the **ChatGPT subscription**
login (no API key, no extra billing setup).

Verified environment: `codex` CLI 0.162.1, `codex login status` → "Logged in
using ChatGPT", model cache contains `gpt-6-astra`.

## Scripts

| Script | Purpose |
|---|---|
| `astra.sh` | Core wrapper. Any prompt, optional images (`-i`), optional working dir (`-C`, defaults to repo root). Environment knobs below. |
| `ping.sh` | Sanity check: login status + tiny astra round-trip that must answer `ASTRA-OK`. |
| `eyes.sh` | **Visual QA** — sends screenshots + written expectations to astra and prints a critique. This is the dev agent's eyes (see DECISIONS.md D4). |
| `generate-model.sh` | 3D pipeline step: spec → astra writes a deterministic Blender Python script → headless Blender exports glTF + parts manifest (DECISIONS.md D3). |
| `specs/` | Part specs consumed by `generate-model.sh` (first: `toy-shell.md`). |

## Usage

```bash
tools/astra/ping.sh                      # verify subscription + model

tools/astra/astra.sh "Summarize docs/PLAN.md milestone M5 in 5 bullets"

tools/astra/astra.sh -i docs/screenshots/M5/frame-002.png \
  "Describe exactly what is on this game screen"

tools/astra/eyes.sh -e /tmp/expectations.md docs/screenshots/M5/frame-*.png

echo "road scrolls down, car centered" | tools/astra/eyes.sh frame.png

tools/astra/generate-model.sh specs/toy-shell.md toy   # → assets/models/toy.glb
```

## Environment knobs

| Var | Default | Meaning |
|---|---|---|
| `ASTRA_MODEL` | `gpt-6-astra` | Override if the plan's model id changes. |
| `ASTRA_EFFORT` | `high` | `model_reasoning_effort` (use `low` for cheap pings). |
| `ASTRA_SANDBOX` | `workspace-write` | Codex sandbox mode for generated-command execution. |
| `ASTRA_BLNDER` | `blender` | Blender binary used by `generate-model.sh`. |

## Notes

- Every astra-produced artifact that the build depends on (Blender scripts,
  glb exports, manifests) is **committed**; the game and `make check` never
  require a live astra call (PLAN.md risk mitigation).
- The user's other harness has the **Blender MCP** for interactive work;
  this repo's pipeline is the headless CLI path so it stays reproducible.
- Calls consume the subscription's usage quota — keep prompts purposeful,
  prefer `ASTRA_EFFORT=low` for pings and small QA passes.
