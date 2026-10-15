# Changelog

All notable changes. Format loosely based on Keep a Changelog; one entry per
milestone (docs/PLAN.md).

## [0.1.0-dev] — unreleased

### M7 — 3D toy shell (done 2026-10-15)
- astra (`gpt-6-astra`) authored a deterministic, parametric Blender 5.2
  script (`assets/blender/toy.py`) on the first pass: 10 named parts
  (body, screen frame + screen, steering wheel, gear lever, ignition key,
  pop-up headlights, bumper, dash panel) + SCREEN_MARKER, 7.8k triangles,
  glb + parts manifest committed.
- game-app: 3D "toy view" (T toggles) — desk diorama, camera sitting in
  front of the toy, sinusoidal wheel wiggle + gear-following lever tilt
  proving separated pivots; verified via eyes QA (wheel angle deltas seen).
- Screen quad given a dark "unlit CRT" material (eyes-caught default-gray).
- Asset plumbing: glb embedded via `EmbeddedAssetRegistry`/`include_bytes`
  (D9) after trunk 0.21 public-dir/hook dead ends; `AssetMetaCheck::Never`
  for wasm meta probes.

### M6 — input layer (done 2026-10-15)
- `game-core::KeyboardSteering`: short tap = light nudge (0.40), sustained
  hold ramps to full over 0.8 s, release decays fast — unit-tested shape;
  gamepad analog (with deadzone) overrides the keyboard value.
- Fairness fixes driven by live scenarios: road loop now eases in (curve
  amplitude fade-in/-out, hazards placed from 15% of the loop) — verified:
  six 120 ms taps keep the car alive past 77 m; holds still reach full steer
  and can crash.
- Harness: state timeline snapshots are awaited (label/value desync fixed),
  `--quickkeys` tap mode for screenshot-free input scripts; `steer` added to
  the published state JSON.

### M5 — core 2D game loop (done 2026-10-15)
- `game-core`: seeded RNG, generated road drum (curve harmonics + hazards,
  seamless loop), full game state machine (ready/running/crashed/gameover),
  road-shift steering model (D7), off-road grace, swept hazard collision,
  fuel, mileage score (D8) — 28 unit tests green.
- `game-app`: real game view (road rows/edge marks/dashes, pooled hazards,
  car decal, HUD score/gear/fuel, crash flash), fixed-timestep sim, wasm
  state bridge (`window.__FTD_STATE__`).
- Verified live in headless chromium: start -> run -> off-road crash ->
  game-over with frozen score; restart (R) + HIGH gear (G) with faster
  speed/score; hazard-hit crash; deterministic replays (same seed+input =
  same meterage). eyes.sh caught two real bugs (missing Camera2d, default
  white road rows) — both fixed and re-verified.

### M4 — repo bootstrap (done 2026-10-15)
- Rust workspace: `crates/game-core` (pure logic + unit tests) and
  `crates/game-app` (Bevy 0.20 app, native + wasm32 via trunk 0.21).
- `make serve` (trunk dev server), `make check` (fmt+clippy+tests+build),
  `make shots` (Playwright screenshot harness incl. held-key capture).
- Spike scene renders in headless chromium via WebGL2 (wasm target);
  keyboard steering + gear toggle verified by screenshot QA; gamepad analog
  code path wired (physical-pad check = user manual item).
- `tools/astra/eyes.sh` visual-QA loop proven: caught a harness bug and
  read pixel positions + HUD from screenshots.
- astra wrapper fix: with `-i` images, codex only accepts the prompt via
  stdin (wrapper now always pipes it).

### M3 — tools/astra instrumentation (done 2026-10-15)
- `astra.sh` (codex exec pinned to `gpt-6-astra`, subscription billing),
  `ping.sh` (verified `ASTRA-OK`), `eyes.sh` (visual QA with image input),
  `generate-model.sh` (spec → Blender Python → glTF + manifest),
  `specs/toy-shell.md`.

### M2 — docs (done 2026-10-15)
- `docs/REQUEST.md` (verbatim charter + R0–R13), `docs/PLAN.md`
  (M1–M13, testing strategy, regression protocol, risks),
  `docs/DECISIONS.md` (D1–D6), `docs/TECH-STACK.md` (stack survey +
  dependency policy).

### M1 — research (done 2026-10-15)
- `research/` — identified the toy family: Tomy Turnin' Turbo Dashboard
  (1983, archetype), **Playmates Fun-to-Drive Corvette Dashboard (1985 — the
  project's namesake)**, Tectoy Turbo Video Driver (1991, Brazil); mechanism
  (lamp + rolling drum, wheel→drum linkage, HI/LO shifter), gameplay rules
  (rolling road, hazards, mileage-score, fuel, crash), 23 reference images.
