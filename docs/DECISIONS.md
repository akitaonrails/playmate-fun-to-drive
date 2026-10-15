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

## D7 — Car static, road moves (toy-authentic steering model)

**Context**: PLAN M5 originally sketched "car lateral position from steering"
(classic racer). The toy (and the user's request text: "static car image with
a rolling background") works differently: the car is a fixed decal; the
printed drum scrolls AND shifts laterally.

**Decision**: game models `road_x` (road center offset at the car row, in
half-road-widths). Steering shifts the road opposite the steer direction;
the printed curve drifts the road toward its amplitude; crash when |road_x|
exceeds the limit (off-road, with grace) or an oncoming hazard crosses the
car band. Verified live: deterministic crash at the same meterage for the
same seed+input (regression-friendly).

## D8 — Score = mileage (toy-authentic)

Tomy's trip counter IS the score, so `score = distance (m)`; HIGH gear
scores faster by covering more ground. The planned `score_rate` multiplier
was dropped as double-bookkeeping.

## D9 — Ship the glb embedded in the binary (no HTTP asset serving)

**Context**: trunk 0.21 removed public-dir copying; its post-build hooks are
clobbered by the dist swap; bevy-on-wasm probes `.meta` files that don't
exist (`AssetMetaCheck::Never` fixes that part); the trunk watcher also
misses `include_bytes!` inputs.

**Decision**: register `assets/models/toy.glb` via bevy's first-party
`EmbeddedAssetRegistry` + `include_bytes!` and load
`embedded://models/toy.glb`. Identical behavior native and wasm, one less
moving part. Regenerating the model requires touching a watched source file
(or restarting trunk) — `generate-model.sh` prints that hint.

**Consequences**: +316 KB binary; asset changes need a rebuild (fine for a
game); dev source of truth stays `assets/models/` with the Blender script.

## D10 — M10 effects on the StandardMaterial path (custom WGSL parked)

**Context**: PLAN M10 sketched drum curvature/lamp/grain as a custom shader.
Bevy 0.20 custom materials hit: (1) WESL files need the WESL processor
(plain `#import` in a from_wgsl leaf → "no entry point" pipelines); (2)
shaders must be loaded via the asset server (loader parses imports; direct
`Assets::add` leaves an invalid leaf); (3) even then our texture binding hit
the magenta GpuImage fallback. Each fixed in turn; the last remained
undiagnosed when the timebox hit.

**Decision**: ship M10 on the proven StandardMaterial + 2D-overlay path
(warm tint, emissive glow, drum bands, recess shading behind the `Y` dial).
Retry the custom shader (curvature warp, grain, flicker) when bevy 0.20
custom-material examples/docs mature — or via a post-process pass.

**Consequences**: no UV curvature warp yet (listed in deferred tuning);
everything else is dial-controlled and testable.

## D6 — Gameplay DNA: Playmates silhouette, Tomy rules

**Context**: the family differs (Tomy 1983 has crash/fuel/score rules;
Playmates 1985 has no crash; Tectoy 1991 is the Brazilian hybrid).

**Decision**: shell aesthetics follow the Playmates Corvette formula
(pop-up lights, centered screen); game rules (rolling road, hazards, score,
fuel, game-over, HI/LO) follow the Tomy because they match the user's
described behavior and are the best documented.
