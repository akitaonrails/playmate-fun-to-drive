# PLAN — milestones, testing, regression discipline

Ordering follows the user's numbered request (research → docs → tools →
game → 3D → mapping → input → effects → dynamics → flare), with testability
built in from M0. Nothing ships forward with a red test.

## Milestone map

| # | Milestone | Depends on | Status |
|---|---|---|---|
| M1 | Research + reference library (`research/`) | — | **done 2026-10-15** |
| M2 | Docs: charter + this plan + decisions | M1 | **done 2026-10-15** |
| M3 | `tools/astra` instrumentation + ping verified | — | **done** |
| M4 | Repo bootstrap: git init, stack scaffold (per TECH-STACK.md), `make serve/check/test`, gamepad-on-web spike, screenshot harness | M2 | **done** (physical-pad check pending as manual item) |
| M5 | Core 2D game loop (flat modern look) | M4 | **done** (all loops verified in-browser) |
| M6 | Input layer: gamepad analog + keyboard short/long press, HI/LO gear | M5 | **done** (physical-pad check still a user manual item) |
| M7 | 3D toy shell (astra → Blender headless → glTF), separate wheel/gear parts | M3 | pending |
| M8 | Game → toy screen mapping (render-to-texture) | M5, M7 | pending |
| M9 | Wheel + gear lever animation bound to input | M6, M8 | pending |
| M10 | Rolling-tube shader effects (flat look stays default; toy-look toggle) | M8 | pending |
| M11 | IK-like body dynamics (G-force lean/rotate) | M9 | pending |
| M12 | Crash flare: whole-toy shake + screen flash + gamepad rumble | M11 | pending |
| M13 | Polish: perf pass, fallbacks, packaging, final regression sweep | all | pending |

M7 can start as soon as M3 lands (parallel to M5/M6) since it only needs the
astra tooling and Blender.

---

## M3 — tools/astra instrumentation

**Deliverables**

- `tools/astra/astra.sh` — one-shot `codex exec` wrapper pinned to
  `gpt-6-astra` via the ChatGPT login (subscription billing, no API key).
- `tools/astra/ping.sh` — auth + model sanity check.
- `tools/astra/eyes.sh` — **visual QA**: attach screenshots to astra with a
  QA prompt; this is how the dev agent "sees" (this harness's model has no
  image input; astra does).
- `tools/astra/generate-model.sh` — spec → astra writes a deterministic,
  parametric Blender Python script → headless `blender` exports glTF/glb.
- `tools/astra/specs/toy-shell.md` — first part spec (see M7).

**Tests / exit criteria**

- `ping.sh` prints `ASTRA-OK` from `gpt-6-astra` using the subscription.

## M4 — Repo bootstrap

- `git init` (if user approves), `.gitignore`, `CHANGELOG.md`.
- Stack scaffold per `TECH-STACK.md` (recommended: Rust + Bevy → WASM,
  decision D1). Crate/app layout with `game-core` (pure logic, no I/O) split
  from the presentation layer. Bundler/dev-server: **trunk** (TECH-STACK).
- **Spike: gamepad on web** — verify analog steering in a browser via the
  engine's input path; if broken, wire the ~50-line `navigator.getGamepads()`
  adapter (decide in D1's follow-up entry). Exit criterion of the spike:
  analog stick value visible on screen in WASM build.
- `make serve` — one command to build + serve the game with auto-reload for
  manual browser testing at every step (R12).
- `make check` — fmt + lint + unit tests + debug build.
- `tools/screenshots/` — Playwright harness: loads the page headless, waits
  for readiness, captures named PNGs into `docs/screenshots/<milestone>/`,
  and dumps game state JSON for assertions.
- CI-able script `tools/ci.sh` running check + screenshot smoke.

**Exit criteria**: `make check` green on a hello-world scene; `make serve`
opens a playable blank canvas in the user's browser; screenshot harness
produces a PNG + state dump on demand.

## M5 — Core 2D game loop (flat, modern)

Everything in `game-core`, deterministic (seeded RNG, fixed-step update) so
it is fully unit-testable headless.

Systems:
1. **Drum scroll** — rolling background with a looping road texture; scroll
   speed by gear (LOW/HIGH) with acceleration smoothing.
2. **Player car** — fixed sprite, bottom-center; lateral position from
   steering input; near-miss band collision (toy-era forgiveness).
3. **Hazards** — spawn list on the road loop (oncoming cars, roadside
   objects); scroll with the drum; collision detection.
4. **Score/mileage counter** — advances with distance; rate multiplied in
   HIGH gear; stops on crash.
5. **Fuel gauge** — depletes; empty = game over.
6. **Game states** — attract/ready → running → crashed/game-over → restart.
7. Keyboard-only temporary input (left/right/gear/restart) so it is playable
   immediately.

**Unit tests (min set)**: scroll math & loop wrap; steering integrates input
to lateral position with correct limits; gear changes speed/score-rate;
hazard spawn cadence by seed; collision band hit/miss table; fuel drain →
game over; score freezes on crash; state machine transitions.

**Manual checklist**: play in browser; steering feels toy-like; HIGH is
visibly faster; crash stops score and flashes; restart works.

**Screenshot QA**: `tools/screenshots` + `eyes.sh` critique for layout
sanity (car centered, road rolling across frames, HUD readable).

**Exit criteria**: all unit tests green; user confirms feel is right.

## M6 — Input layer

- Gamepad: analog axis 0 (left/right) with deadzone + response curve; gear
  on a face button or shoulder; pause; rumble capability reserved for M12.
- Keyboard: arrows — **short press = nudge, hold = ramp** steering ramp;
  gear on a key (e.g. G or Up); restart.
- Unified `InputState` (steer: f32 [-1,1], gear: Low/High, events) produced
  by adapters; game consumes only `InputState` (keeps logic testable).
- Gamepad connect/disconnect handled gracefully; on-screen hint of active
  device.

**Unit tests**: keyboard ramp profile over time series; deadzone/curve
mapping table; gear toggle edge cases; recorded input sequence replays.

**Manual checklist**: play with a real gamepad and with keyboard-only.

## M7 — 3D toy shell (astra + Blender)

- Shell inspired by the Playmates Corvette silhouette (pop-up headlights,
  centered screen) at toy scale (~25 cm wide; Tectoy 15×25×25 cm reference),
  original design — no cloned decals/logos (see research/README legal note).
- **Separate rigged parts**: body shell, windshield/screen recess (with a
  named empty/marker `SCREEN` for M8), steering wheel (pivot at column),
  gear lever (pivot at base), plus key/headlights detail.
- Deterministic, parametric Blender Python (generated via astra, reviewed,
  committed) → `assets/models/toy.glb` + parts manifest (names, pivots,
  rest poses) committed alongside.
- Engine loads glb, renders the toy on a simple desk scene.

**Tests**: script runs clean in headless Blender (exit 0, glb exists, mesh
count/parts manifest assertions in a small Python check); engine smoke test
loads the model.

**Screenshot QA**: `eyes.sh` on renders from 3 angles vs reference images in
`research/images/`; iterate the spec/script until proportions read right.

**Exit criteria**: toy renders; parts animate independently when wiggled in
a debug view; user approves the look.

## M8 — Game mapped onto the toy screen

- Render the game (M5/M6) to an offscreen texture; sample it on the toy's
  screen mesh (`SCREEN` marker from M7).
- Camera framing: the toy fills the view like sitting in front of it;
  game remains fully playable in this view (input unchanged).
- Aspect handling: game's internal resolution vs screen mesh aspect; no
  stretching.

**Tests**: render-graph smoke; frame latency budget check (game tick +
texture + toy render at 60 fps budget in headless run); aspect-ratio unit
test on the mapping math.

**Manual**: play a full game "through the toy windshield".

## M9 — Wheel + gear animation

- Steering wheel rotates with `InputState.steer` (with a little lag/spring
  so it feels mechanical, matching the rotary→linear linkage feel).
- Gear lever snaps LOW↔HIGH with a springy overshoot; "turbo" indicator on
  the dashboard in HIGH (nods to the Tomy).

**Tests**: animation driver unit tests (input → target angle, spring
constants); screenshot sequence QA (wheel follows input direction).

## M10 — Rolling-tube shader effects (toggleable)

Layered post effects on the game texture only (flat look remains the
default, per R6):
- Drum curvature (cylindrical UV warp) + subtle seam period.
- Lamp glow gradient/falloff from the top of the tube; warm tint.
- Paper/plastic grain, faint scanline/banding, vignette at screen recess
  edges; slight brightness flicker at motor speed.
- Intensity uniform → `toy_look: 0.0..1.0` so effects can be dialed in
  (and off for the modern flat look).

**Tests**: shader compile smoke on WebGPU + WebGL2 fallback; golden-ish
screenshot pair (toy_look 0 vs 1) diffed via `eyes.sh`; perf frame-time
check with effects on.

## M11 — IK-like body dynamics

- Research note (docs/dynamics-notes.md) choosing the approach: procedural
  spring-damper rig (most likely) vs true IK chains — the toy has no
  skeleton, so "IK-like" = layered springs driving body roll/pitch/heave.
- G-force model: lateral accel from steering (turn left → body rolls right),
  longitudinal from gear changes/crash, heave wobble from speed.
- Whole-toy transform stack: base pose ← dynamics springs ← crash impulses
  (M12 reuses the same rig).

**Tests**: spring-damper unit tests (impulse → overshoot → settle times);
extreme-input stability (no NaN/explosion); screenshot QA for visible lean
on sustained steering.

## M12 — Crash flare

- Hazard hit → whole-toy shake (decaying noise on the M11 rig), screen
  flash + static/click (nods to the Tomy's crash flash), gamepad rumble,
  score freeze, game-over beat, quick restart.

**Tests**: impulse → shake envelope unit test; rumble triggered exactly on
hit events; full-loop screenshot sequence QA.

## M13 — Polish & release

- Perf: 60 fps target on integrated GPU; WebGPU → WebGL2 fallback ladder;
  asset size trim; loading screen with ignition-key "start" moment.
- Packaging: static hosting (GitHub Pages or itch.io zip), README with
  controls, final full regression sweep, tag `v0.1.0`.

---

## Testing strategy (applies to every milestone)

1. **Unit tests** for all pure logic (`game-core` + input adapters + rig
   math) — deterministic, seeded, no window/GPU required.
2. **Headless browser harness** (`tools/screenshots/`): loads the served
   build, drives synthetic input, captures PNGs + state JSON.
3. **Visual QA via astra** (`tools/astra/eyes.sh`): the dev agent cannot see
   images in this harness; astra critiques screenshots against the
   milestone's expectations; findings become tweaks or tickets.
4. **Manual user checklist** per milestone (kept in this file) — `make
   serve` is always one command away.
5. **Regression protocol**: `make check` + screenshot smoke must be green
   before any commit; one milestone = one tagged commit series; screenshots
   archived per milestone under `docs/screenshots/M*/` for before/after
   comparison; `CHANGELOG.md` updated per milestone.

## Risks & mitigations

- **astra plan limits/availability** → all astra outputs are committed
  artifacts (scripts/specs); game builds fine without any astra call.
- **WebGPU not everywhere** → WebGL2 fallback from day one (stack choice D1).
- **Blender scripting drift** → deterministic scripts + committed glb +
  parts manifest; re-export verified in `make check`.
- **Feel regression while tuning** → all tuning constants live in one
  tuning module with unit-tested bounds; screenshot pairs archived.
- **Scope creep (effects, sounds)** → toy_look dial keeps flat mode the
  default; extras land only after their milestone's tests exist.
