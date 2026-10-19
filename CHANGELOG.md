# Changelog

All notable changes. Format loosely based on Keep a Changelog; one entry per
milestone (docs/PLAN.md).

## Unreleased

## [0.9.2] — 2026-10-19

- Tightened the authored toy camera another four percent along the existing
  screen-centered sightline, matching the latest reference crop by trimming
  more of the wheel and lower controls while preserving the screen and bezel.
- Kept the startup orbit calibrated to the revised final framing and updated
  its camera regression coverage.
- Prevented iOS Safari selection callouts and context menus from interrupting
  touch controls while leaving the nickname text field unaffected (#1).

## [0.9.1] — 2026-10-19

- Pushed the authored toy camera ten percent closer along its existing
  screen-centered sightline, enlarging the play screen while retaining the
  full bezel, dashboard Roadster, upper wheel arc, and a narrow room border.
- Kept the startup orbit and CRT power-on calibrated to the same final camera
  transform, with regression coverage for the push-in ratio and sightline.

## [0.9.0] — 2026-10-19

- Rebuilt the timed-run finale: the scenery clears, the perspective road rises
  into a constant-width vertical ribbon, and a dedicated overhead Roadster
  sprite fires twin cold-air streams as it launches toward the sky.
- Added deterministic finale geometry and timing coverage plus a debug-build-only
  `?dev_ending=1` preview that never enters optimized production bundles.
- Raised soundtrack presence relative to effects and added a regression check
  for the intended music-to-engine balance.
- Added restrained specular highlights to the steering wheel and shift knob so
  both controls remain readable in the moody dorm-room lighting.
- Replaced the README screenshots, animated preview, and full playthrough with
  the latest v0.9.0 captures.

## [0.8.0] — 2026-10-19

- Added a one-time page-load cinematic: the camera begins with the complete
  dashboard toy in view, arcs around it while pushing in, lands exactly on the
  normal play framing, then powers the dark screen on with an old-CRT trace,
  screen-light bloom, and a dedicated 16-bit mechanical switch click before
  revealing the difficulty menu.
- Kept normal top speed near 87 on the unchanged speedometer scale. Holding a
  calm line through a sustained straight now opens a small overdrive window
  beyond 88; steering, braking, leaving the road, or crashing interrupts its
  charge.
- Added a once-per-session 88 mph event with a stainless time-machine-inspired
  car racing past from behind, blue electrical trails, a bright disappearance
  flash below the horizon, and a blinking red 088 readout.
- Relabeled the speedometer to mph without converting its familiar numeric
  sweep, slightly softened LOW-gear acceleration, and added a small amount of
  traffic to Medium and Hard.
- Prevented COLD-AIR and Jump Kick pickups from spawning on traffic, including
  a deterministic alternate-lane and forward-clearance search when the
  preferred lane is occupied.
- Replaced the oversized generic Rust target cache in GitHub Actions with a
  pinned Rust-aware cache that prunes non-reusable build artifacts and writes
  only from the default branch.

## [0.7.0] — 2026-10-19

- Increased rare Jump Kick availability across every difficulty while keeping
  Easy most generous, Medium intermediate, and Hard the most demanding. The
  COLD-AIR pickup schedule remains unchanged.
- Reworked COLD-AIR velocity streaks into shorter anime-style lines clustered
  around the Roadster instead of extending toward the horizon.
- Added subtle precipitation handling: rain retains 94 percent steering
  response and snow retains 90 percent, preserving control while increasing
  the value of early braking and safer lines.
- Strengthened night, rain, and snow distance visibility with a foreground-safe
  gradient that hides the upper road and distant car bodies while preserving
  their rear lights.
- Enlarged and clarified the hurricane into a high-contrast rotating funnel
  rendered between the road haze and its flying cars, trees, and leaves.
- Added a README feature overview covering driving systems, COLD-AIR, Jump
  Kick, seasons, weather, scenery, rare events, audio, CRT, touch, and mobile
  support.

## [0.6.0] — 2026-10-19

- Added a full in-screen pause presentation with clear keyboard/gamepad help,
  plus safe-area-aware touch PAUSE/RESUME and RESTART buttons. Select or Escape
  now returns to run selection with the prior simulation fully reset, while
  Start also restarts from the final leaderboard.
- Made each run begin in the season and day/night phase reported by the
  player's local date, clock, and timezone. Southern Hemisphere timezone
  regions receive the corresponding inverted meteorological season.
- Corrected Jump Kick to rotate the car sprite as a simple circle in the
  screen plane, enlarged both collectible collision bands, and made Jump Kick
  glow more clearly against traffic.
- Kept the dorm-room wall, window, props, and room lighting stationary while
  steering and crash dynamics move only the dashboard toy.
- Improved mobile performance by defaulting coarse-pointer devices to the
  lower pixel budget, disabling CRT bloom until explicitly enabled, avoiding
  excessive device-pixel-ratio rendering, and reducing shadow work.
- Fixed soundtrack selection to follow the actual environment cycle, cancelled
  active COLD-AIR thrust on a crash, and made environment-start accounting
  preserve competitive cycle scores.
- Rebuilt the foreground road apron so its edges continue widening in
  perspective, kept headlight projection independent from road curves, and
  increased season-aware roadside scenery density.
- Expanded pause, reset, environment, collision, visual projection, mobile
  quality, music lifecycle, and browser-bridge regression coverage.
- Replaced the README hero screenshot and animated/full playthrough with the
  latest v0.6.0 captures.

## [0.5.1] — 2026-10-19

- Fixed the browser freeze at the first day/night soundtrack transition by
  replacing synchronous audio seeking with a bounded current/outgoing-player
  crossfade. Only one track remains after each transition instead of decoding
  all six soundtrack files continuously.
- Made browser audio readiness follow the real WebAudio context state and
  capped simultaneous one-shot voices, preventing suspended audio from
  accumulating entities.
- Smoothed and lengthened the environmental noise loops to remove periodic
  wind, rain, braking, skid, gravel, and cold-air boundary artifacts.
- Restored the intended roadside density by allocating all three sprite parts
  for each of the eighteen pooled props, and removed repeated allocations from
  hurricane/UFO traffic selection.
- Cleaned up cancelled touch pointers for both screen taps and gear swipes.
- Enlarged the COLD-AIR and Jump Kick road pickups by 22 percent so they remain
  distinct from traffic without changing their collision areas.
- Moved the toy camera subtly closer so the game screen is easier to read while
  retaining the existing 16:10 framing on standard and ultrawide displays.
- Added an audible browser endurance regression that crosses the first music
  transition, checks the countdown remains live, and verifies the outgoing
  player is released. Production bundles now fail verification if soundtrack
  or display assets are missing.
- Stopped tracking stale generated release bundles; release ZIPs are rebuilt
  from the verified production output and attached to the matching GitHub
  release.

## [0.5.0] — 2026-10-18

- Added a 30-second difficulty-menu idle animation that double-blinks the toy
  Roadster's front lights, plus rare shooting stars through the dorm window.
- Enlarged roadside trees and synchronized shoulder scenery with the horizon:
  sakura routes get blossoms, forests become denser, winter uses sparse bare
  trees, and city routes use streetlamps that illuminate at night. Vegetation
  now spreads across the open grass, with occasional Tesla, Sega, and Namco
  pixel billboards appearing more frequently around city skylines.
- Lowered hurricane and UFO assists into the road's middle-distance band so
  their pull/beam animation visibly meets the traffic cars they remove; the
  hurricane also uproots and tumbles two trees from the outer grass.
- Added optional multi-touch mobile controls with safe-area-aware steering,
  brake, gear swipe, cold-air, Jump Kick, pause, difficulty/CRT menus, initials,
  and restart controls. Touch mode persists locally and portrait devices get a
  clear landscape-orientation prompt.
- Added the rare once-per-day/night-cycle Jump Kick collectible. B/gamepad B
  launches a speed-carrying 360-degree Y-axis jump that clears traffic in the
  air but reuses normal crash recovery when the car lands on traffic or a
  roadside obstacle.
- Recalibrated steering deadzone, ramp, keyboard taps, and maximum authority so
  center-road curvature cannot overpower player input on sharp bends.
- Replaced the off-road audio artifact with a speed/excursion-driven gravel
  loop, separated from the rain-only wet skid layer, and added 16-bit pickup,
  launch, and landing cues for Jump Kick.
- Added deterministic tests for collectible rarity and capacity, activation,
  airborne traffic passes, unsafe landings, timer/reset cleanup, steering
  authority, touch edge latching, and audio-layer separation.
- Made the Jump Kick's barrel roll explicitly rotate around the car's
  front-to-back Y axis, with a basis-vector regression test that rejects flat
  Z-axis spinning.
- Rebuilt thunder as a compact 16-bit crack with two chip-noise echoes and a
  gated descending rumble, replacing the strange sustained low drone.
- Kept touch controls functional when browser storage is unavailable; the
  toggle falls back to session-only state instead of aborting the adapter.
- Redesigned both player-car representations around the new Tesla Roadster's
  recognizable low silver wedge, black canopy, full-width rear light blade,
  four outlets, oversized wheels, and raised wing while retaining the game's
  pixel-art and low-poly visual language.
- Nudged distant traffic lane-change frequency from 12/22/32% to 14/25/35%
  across Easy/Medium/Hard, without moving the near-player lane lockout.
- Made the in-game Roadster's complete rear diffuser blade illuminate at night;
  the toy-top model now faces the camera at a front three-quarter hero angle
  and reproduces the paired lamps plus thin white reveal-style light strip.
- Renamed the player-facing boost system to COLD-AIR for the Roadster's
  cold-air thrusters; internal telemetry keys remain stable for QA tooling.
- Increased the road's rolling elevation and gently strengthened uphill drag;
  an active COLD-AIR boost now gives the Roadster a short automatic hop when it
  crests a rise, without consuming or interfering with the Jump Kick item.
- Added a post-countdown Roadster finale: the road rises into a full vertical
  building facade and the top-down car climbs it using its suction system
  before the score and three-letter name entry appear.
- Added a run-opening sky drop with two layered cold-air jet streams; the
  simulation remains still until the Roadster lands, then begins normal
  automatic acceleration in LOW gear.
- Guaranteed an early Jump Kick pickup in Easy mode within the first minute,
  while leaving later spawns and Medium/Hard rarity unchanged.
- Reorganized the mobile driving overlay so dedicated COLD-AIR and JUMP KICK
  buttons sit together above the right-thumb gear swipe and brake area.
- Preserved the authored 16:10 composition on ultrawide monitors and phones;
  extra horizontal space now extends the moody dorm-room backdrop instead of
  stretching the toy, car, or game screen.
- Made the first touch START or keyboard Enter confirmation request fullscreen
  automatically, while retaining the fullscreen toggle for entering or exiting
  later; browser gesture rules still apply to gamepad-only starts.
- Replaced the three short soundtrack loops with six original 100-second PSG
  arrangements covering bright pop-rock, tropical fusion, breezy city-pop,
  dramatic racing rock, hard-rock anthem, and tense industrial styles. Tracks
  now align one-per-day/night cycle, restart at their transition, and retain the
  existing smooth crossfade and cold-air ducking.

## [0.4.0] — 2026-10-18

- Rebuilt road curvature around true perspective arcs: curvature now integrates
  through heading into lateral displacement, while the authored loop uses eased
  entries, sustained mixed-radius corners, and a flowing opposing S-section.

## [0.3.0] — 2026-10-18

- Added run-randomized Tokyo and Paris pixel-art horizons with distinct
  landmarks, windows, and signage. A run keeps one city identity while its
  palette and visibility respond to day/night, weather, and season.
- Replaced the toy view's plain rear wall with a lightweight nighttime dorm
  composition: moonlit curtained window and skyline, shelf and books, abstract
  racing print, and warm fairy lights.
- Corrected the toy camera to use dim-indoor exposure and lifted the room's
  blue-violet wall lighting so the dark screen bezel remains distinguishable.

## [0.2.1] — 2026-10-17

- Softened the dorm-room darkness around the toy and added a subtle warm,
  off-axis overhead light that reveals the red shell without washing out the
  screen or rear wall.

## [0.2.0] — 2026-10-17

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

## Development history

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
