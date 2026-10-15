# DECISIONS

Running decision log. Each entry: context → decision → consequences.
Stack decision D1 is *proposed* pending user confirmation.

## D1 (proposed) — Rust + Bevy compiled to WASM for the game

**Context**: needs WASM + GPU (WebGPU) + gamepad + tests + a 3D toy with a
live game texture on its screen. User's cross-project profile defaults to
Rust for new engineering; user asked to prefer popular libs over reinvention
(R13).

Full option matrix (three.js r186, Babylon 9.26, macroquad, PlayCanvas,
Unity/Godot web) and the dependency policy live in **`TECH-STACK.md`**.

**Decision (pending user OK)**: Rust + Bevy + trunk. Single language for game
logic, rig math, and input adapters; `cargo test` covers the pure-logic
crate natively (fast, no browser); Bevy supports render-to-texture (M8) and
WebGPU/WebGL2 on the web target. Open risk — web gamepad via gilrs — is
spiked at M4 with a tiny `navigator.getGamepads()` fallback adapter.

**Consequences**: larger WASM binary (acceptable for a game); Bevy upgrade
discipline needed (pin versions); iteration on visuals slightly slower than
TS but compensated by the astra/eyes.sh QA loop.

## D2 — astra = `gpt-6-astra` via local codex CLI on the ChatGPT subscription

**Context**: user asked for tools/ instrumentation so their codex harness
loads "astra" on their subscription plan (no API key).

**Facts observed**: `codex` CLI 0.162.1 installed; `codex login status` →
"Logged in using ChatGPT"; user default model `gpt-5.6-sol`; the CLI's model
cache lists **`gpt-6-astra`**.

**Decision**: `tools/astra/astra.sh` pins `-m gpt-6-astra` (override with
`ASTRA_MODEL`), runs `codex exec` non-interactively with workspace-write
sandbox from the repo root. Ping script validates before first real use.

**Consequences**: no extra billing setup; if the plan ever drops the model,
one env var swaps it.

## D3 — 3D asset pipeline: astra writes deterministic Blender Python; headless Blender exports glTF

**Context**: astra generates 3D models; the repo has `blender` CLI
installed; the user's other harness has the Blender MCP.

**Decision**: astra's job is to *write parametric, deterministic Blender
Python scripts* (committed) which `blender --background --python` executes
to export `assets/models/*.glb` + a parts manifest. The Blender MCP remains
an optional interactive path for the user; CI/`make check` only depends on
the headless path.

**Consequences**: reproducible builds (no binary blobs as source of truth);
scripts reviewable; model regen never requires a GUI.

## D4 — Visual QA runs through astra because this dev model can't see images

**Context**: the opencode harness model (glm) has no image input; R12
requires agent-driven screenshot checks.

**Decision**: `tools/astra/eyes.sh` sends screenshots + expectations to
astra (`-i` image flag on codex exec) and returns a written critique; the
agent acts on the text.

**Consequences**: one extra codex call per QA pass; expectations must be
written down per milestone (kept in PLAN.md checklists).

## D5 — Research images are reference-only; shipped art is original

**Context**: toy shells and printed artwork are third-party IP.

**Decision**: `research/images/` stays private study material; the 3D shell
is an original design in the *category* of the toy; no trademarked logos or
copied decal art in shipped assets.

## D6 — Gameplay DNA: Playmates silhouette, Tomy rules

**Context**: the family differs (Tomy 1983 has crash/fuel/score rules;
Playmates 1985 has no crash; Tectoy 1991 is the Brazilian hybrid).

**Decision**: shell aesthetics follow the Playmates Corvette formula
(pop-up lights, centered screen); game rules (rolling road, hazards, score,
fuel, game-over, HI/LO) follow the Tomy because they match the user's
described behavior and are the best documented.
