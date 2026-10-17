# Changelog

## Unreleased

- Removed the unused KTX2/zstd texture-decoding path from the web build and
  switched the toy camera to shader-only ACES tonemapping, eliminating the
  vulnerable transitive `ruzstd 0.8` decoder without vendoring or suppressing
  its security advisory.
- Pinned GitHub Actions and downloaded release-tool checksums in the Pages
  pipeline so builds execute immutable, verified CI dependencies.
- Reworked the looping rain ambience into a softer, lower-frequency four-second
  rainfall bed without the previous rapid, mechanical tremolo.
- Turned the rare hurricane and nighttime UFO sightings into player assists:
  hurricanes pull one or two approaching traffic cars off the road, while the
  UFO hovers over the road and abducts one car with a tractor beam. Assisted
  cars cannot collide with the player and do not count as passes or bumps.
- Added Easy (2:00), Medium (6:00), and Hard (10:00) selection with isolated
  per-difficulty local leaderboards and v1-to-v2 storage migration.
- Added explicit Q/E and LB/RB LOW/HIGH controls, an analog 0–120 km/h
  speedometer, recovery cues, red-zone/over-rev feedback, and a visible nitro
  burn/cooldown gauge.
- Replaced the shelf turbo dot with a low-poly 1980s wedge-car ornament whose
  rear lights retain HIGH/nitro feedback; the physical lever now follows only
  accepted core gear state and driving presentation includes downshift jolts.
- Corrected the speedometer to sweep left-to-right, reversed the physical
  lever so LOW rests down and HIGH moves up, and rotated the wedge-car ornament
  100 degrees to expose its front quarter.
- Added analog LT braking while preserving automatic acceleration; braking
  below the HIGH-gear stall threshold requires a LOW-gear recovery.
- Widened the projected road by roughly 10%, spread its lanes and scenery
  through the shared projection, and widened the player collision band to
  better match the visible car while retaining near-miss forgiveness.

All notable changes. Format loosely based on Keep a Changelog; one entry per
milestone (docs/PLAN.md).

## [0.1.0-dev] — unreleased

### v0.3.0-dev3 — timed challenge and local leaderboard (2026-10-17)

- Ten-minute challenge mode now ends on one authoritative countdown and ranks
  completed day/night cycles, clean traffic passes, and car bumps using
  `max(0, cycles*10000 + passed*100 - bumped*300)`.
- Time-up opens an in-screen RUN COMPLETE panel with three-letter uppercase
  name entry and a local top-five table. Valid top-10 results persist in
  browser localStorage; corrupt or unavailable storage safely falls back to
  the current in-memory board without blocking play.
- Added deterministic `?qa_session_secs=1..600` browser override and published
  end mode, initials, leaderboard, score, session, weather, traffic, glow, and
  toy-pose state for repeatable browser assertions. QA mode is explicitly
  ephemeral and never reads or writes the normal local leaderboard.
- Added deterministic clear/cloudy/rain weather, performance-driven day/night
  progress, aligned safe-area HUD panels, nitro pickup/meter glow, exhaust and
  road-border perspective streaks, and an eased lean-back/toward-player boost
  pose. Removed the rain road-sheen bars that obscured the road and car.
- Going beyond the road edge now adds progressive drag, increasing smoothly
  with excursion and easing away after recovery; the existing grace window and
  toy-like bump remain intact.

### v0.2.0-dev5 — collectible nitro boost (2026-10-17)

- Deterministic cyan nitro canisters now appear in road lanes often enough
  to offer at least four pickups per 100-second day/night cycle at LOW speed.
- Three-cell in-screen nitro meter: collected charges fill cyan cells, a full
  meter blinks, and the active cell drains over a five-second boost.
- Y / gamepad North spends one charge for a 30% speed boost; the toy kicks
  backward and eases forward with the draining charge, and its turbo lamp
  burns brighter while nitro is active. The rolling-tube look toggle moved
  from Y to L.
- Corrected the previous HUD implementation to use actual Bevy `Text2d`, so
  score/lap/gear/nitro labels render inside the game image and on the toy's
  screen instead of clipping at the browser corner.
- Bumps now reset the off-road grace accumulator, preventing a single
  excursion from trapping the car in a bump-every-tick near-stop.
- Nitro state is published for deterministic browser QA; 53 core tests plus
  3 app tests pass, including pickup, capacity, boost, expiry, and recovery.

### v0.2.0-dev4 — in-screen HUD, softer toy motion, day/night + seasons (2026-10-16)

- HUD moved INTO the game image (Text2d on dark panels): score top-left,
  LAP + live time + best top-center, fuel/gear top-right — shows on the
  toy screen itself; only the controls hint stays window-space.
- Toy motion softened: body roll gain 0.09->0.045, shift halved, springs
  softer (k 55/c 7.5), wheel full lock ~77 deg with a lazier spring.
- Environment cycles (game-core::environment, unit-tested): day/night
  every 100 s nested in 5-minute seasons (3 day cycles per season,
  spring->summer->autumn->winter) — sky palette blends day->night, sun
  swaps to moon + stars, clouds fade out; grass goes green->ochre->snow,
  rumble strips pale in winter; horizon haze + falling snow in winter,
  warm tint in autumn; everything dims at night.
- Bug found via phase-locked captures: the bump flash never decayed after
  the crash-phase removal — permanent white veil. Now decays every step.
- QA tooling: env phase (day/season) published in the state JSON so
  captures can wait for exact cycle phases; ENV_TIME_SCALE for fast QA.

### v0.2.0-dev3 — pseudo-3D OutRun renderer + square test track (2026-10-16)

- Renderer rewritten classic-super-scaler style: 30 geometrically-spaced
  projected rows (near rows tall, far rows compressed), road/grass/rumble/
  dashes all narrow to the horizon, CUMULATIVE curve bend (the drunk
  zig-zag came from per-row independent curve offsets — the bend now
  sweeps smoothly), foreground apron fills below the nearest row.
- Sprites (traffic + palms) projected with distance scaling: cars shrink
  into the distance on the road; palms sparse (3/side/loop) off-road as
  scenery — no longer traffic-like.
- Square test track (user request): `RoadLoop::apply_square_curve()` —
  long straights + 4 constant-curvature corners (eased), used at boot;
  unit-tested (straights straight, corners constant, seamless wrap).
- Bump flash softened 0.55 -> 0.35 (was reading as a pale wash mid-fade).
- Debugging notes: PIL pixel-band analysis (via astra) caught TWO real
  bugs my code edits silently missed — a sign-inverted row height and an
  apron-sync `.zip(None)` no-op. Assert-replace discipline reiterated.

### v0.2.0-dev2 — arcade loop rework (2026-10-16, user-directed)

- Start/P pauses and resumes; Select/R restarts (gamepad + keyboard).
- No more crash game-over: hazards/off-road are toy-ish BUMPS — the car
  slows, the drum kicks left/right, a short flash plays, and the run
  continues (fuel-out remains the only run end). The whole toy shakes and
  the pad rumbles on every bump, synced to the same event.
- Lap racing: the drum loop is a lap — LAP counter, live lap time, and
  best lap time in the HUD (`LAP n | m:ss.xx | BEST m:ss.xx`); pause
  overlay text. Verified live: 3 laps + best 6.0s over a 24-cycle drive.
- Camera moved close: screen dominant upper-center, only the top half of
  the steering wheel at the bottom (dash lowered so nothing occludes the
  screen). eyes QA PASS.
- game-core: bumped/laps/lap_time/lap_best/paused state, 3 new unit tests
  (pause freeze/resume/restart, lap counting + best tracking), bump tests
  replace crash tests — 46 green.

### v0.2.0-dev — OutRun restyle, lighting, playability (2026-10-16)

- 2D game restyled classic-16-bit: gradient sky + striped sun + clouds,
  teal sea & sand at the horizon, scrolling grass stripes, alternating
  asphalt shades, red/white rumble edges, lane dashes, palm trees,
  composite car sprites (cabin/wheels/spoiler; oncoming cars with windows).
- Playability (user-directed tuning pass, first slice): smaller collision
  footprints (car 0.13, hazard 0.14 half-widths), forgiveness 0.35, lanes
  widened to ±0.72, fewer hazards (6-9) starting at 22% of the loop,
  steer authority 2.0, gentler drift/grace, softer keyboard tap (0.35).
- Model: screen enlarged to 0.19 x 0.11875 (playability over realism, per
  user) with aspect-exact mapping; pop-up lamp pods moved to the player
  side and chunkier (previous edits silently no-opped on regenerated
  params — lesson: assert replaces).
- 3D scene: shadow-casting key light + warm desk-lamp fill + cool rim
  light, darker desk, backdrop wall; screen is now a true emitter
  (emissive_texture = game image, toy camera tonemapping off) — backlit
  CRT look from boot (toy_look defaults to 0.5).
- Framing: camera reframed after geometric analysis (near-side wheel was
  outside the half-FOV); wheel fully in frame with balanced margins.

### v0.1.0 fix 3 — wheel direction, screen occlusion, fullscreen canvas (2026-10-16)

- Wheel direction inverted (user report): left steer now rotates the toy
  wheel counter-clockwise as the player sees it — verified unambiguously
  via a mirrored capture pair (top spoke at 10 o'clock on left, 1-2
  o'clock on right).
- The enlarged black cockpit band covered the lower screen — band lowered
  and slimmed; road + car fully visible again (eyes QA PASS).
- Canvas now fills the browser window: winit writes inline 960x600 styles
  on the canvas, so CSS `!important` 100vw/100vh stretches the render
  (internal resolution/logic stay 960x600); `fit_canvas_to_parent` also
  enabled. Verified filling 1280x800 edge to edge.

### v0.1.0 fix 2 — faithful toy model + wheel axis (2026-10-15)

- User report: the steering wheel turned about the wrong axis (felt like Y
  yaw) — glb inspection confirmed the disc normal was local Y. The model
  was regenerated with the disc facing the player (thin local Z, spin axis
  = local Z, asserted in the manifest check); the existing Z-rotation
  animation is now correct.
- Model redesign toward the real Playmates toy (simplified): reference
  photos now ATTACHED to the astra generation call; new spec silhouette —
  red wedge front-clip with tapered nose, broad black cockpit band, center
  screen pod (0.152x0.095, aspect-exact), pop-up headlight pods at the
  nose, protruding dark bumper with running lights, fenders/vents/cowl.
  Turbo lamp + screen constants retargeted; eyes QA: geometry clean, wheel
  faces player, wedge + black fascia reading; fidelity iterated twice
  (further model likeness queued for the tuning pass — the gray game
  texture dominates the "monitor" read; Y-dial warm look helps).

### v0.1.0 fix — toy view is the default (2026-10-15)

- The build opened in the flat 2D game view; the toy was only reachable via
  T and the hint flashed by on the loader — the first playtest read as
  "just a 2D game". Toy view is now the DEFAULT (T toggles the flat view),
  mode/camera state is applied every frame (boot included), and a small
  persistent controls hint sits bottom-left in both views (score/gear HUD
  still only in the flat view).

### M13 — polish & packaging (done 2026-10-15)
- Release pipeline: wasm-opt fixed via `tools/setup-wasm-opt.sh` (binaryen
  needs `--all-features` for our bulk-memory/sat-float output) — release
  wasm 136.7 MB -> **25.0 MB**; `make package` -> `release/fun-to-drive-web.zip`.
- WebGPU-capable build path: `make build-wasm-webgpu` (bevy `webgpu`
  feature; falls back to WebGL2).
- Ignition key on the toy twists when the game runs (spring, M13 flourish);
  styled loading screen with controls hint; root README with controls/dev
  docs; `--fps` rAF sampler in the harness.
- Release build smoke-verified end to end in headless chromium (boot, run,
  crash, toy view, dynamics). Software-GL fps 7.5 median there = pessimistic
  floor; hardware GPUs expected far higher (perf notes in README/PLAN).

### M11 + M12 — body dynamics & crash flare (done 2026-10-15)
- game-core `dynamics`: procedural spring-damper rig (docs/dynamics-notes.md)
  — roll/shift follow steering pseudo-G (antisymmetric, outward), pitch
  follows longitudinal accel, heave speed wobble; crash = spring impulses +
  monotonic decaying shake bias with tremble, all deterministic, 7 new unit
  tests (40 total green).
- App: rig drives the toy root transform (lean + heave + crash jolt);
  `roll` published in the state JSON; turbo lamp (amber emissive dot) lit in
  HIGH gear; gamepad rumble (STRONG_MAX 450 ms) on every crash for all pads.
- Verified: numeric roll antisymmetry with steer, mirrored-lean eyes PASS,
  crash flash + settling eyes PASS, turbo lamp eyes PASS. Shake magnitude
  flagged for the deferred tuning pass (stills undersell motion).

### M10 — rolling-tube toy look (done 2026-10-15)
- `Y` cycles the toy_look dial 0 / 0.5 / 1 (flat modern look stays the
  default, REQUEST R6). At 1.0: warm lamp tint + emissive backlit glow on
  the screen material, drum bands scrolling with the drum (2D overlays,
  physically "printed on the drum"), recess shading at screen edges.
- Custom WGSL/WESL material route attempted and parked: bevy 0.20 custom
  materials hit three walls (WESL entry-point processing, asset-server-only
  `#import` parsing, GpuImage/fallback binding) — documented in DECISIONS
  D10 for a later retry; the StandardMaterial path is proven and used.
- A/B verified via eyes QA (bands, edge shading, warmth visible; exact amber
  balance queued in the deferred tuning pass).

### M9 — wheel + gear lever bound to input (done 2026-10-15)
- Steering wheel tracks live `InputState.steer` (~120° full lock) through an
  under-damped spring (mechanical lag feel); gear lever snaps LOW/HIGH with
  springy overshoot. Replaces the M7 proof-of-pivot sinusoid.
- Verified live: wheel visibly mirrors between held-left and held-right toy
  view captures; spring behavior unit-tested (converges, bounded overshoot).

### M8 — game on the toy screen (done 2026-10-15)
- 2D game renders into an offscreen 960x600 image; the toy's Screen quad
  carries it as an unlit texture (reads like a backlit CRT). Game stays
  playable in toy view; aspect matched by construction (unit test).
- Toy proportions corrected to the real toy (screen raised, wheel lowered)
  so the player car is visible over the wheel rim from the front camera.
- Window-space HUD text hidden in toy mode (world sprites — road/car/fuel
  bar — render inside the texture); eyes QA PASS on all checks.
- Bevy 0.20 notes: RenderTarget is a component now (Camera.target field is
  gone); Image::new_fill takes RenderAssetUsages (usage bits OR'd after).

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
