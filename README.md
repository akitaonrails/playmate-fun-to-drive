# Fun to Drive — web

A web/WASM recreation of the 80s/90s electromechanical dashboard driving
toys (Tomy Turnin' Turbo Dashboard 1983, **Playmates Fun-to-Drive Corvette
Dashboard 1985**, Tectoy Turbo Video Driver 1991): the 2D drum game plays
*on the screen of a 3D toy* sitting on a desk, and the toy physically
reacts — steering wheel, gear lever, ignition key, G-force body lean, and a
full-body jolt (with gamepad rumble) when you crash.

Choose a two-, six-, or ten-minute run. Drive cleanly and quickly to complete as many
day/night cycles as possible, pass traffic, collect nitro, and avoid bumps.
Easy runs at 80% of Hard's road speed, Medium at 90%, and Hard at full speed;
gear thresholds scale with the selected difficulty.
At time-up, enter three initials for the local top-10 leaderboard for that
difficulty (top five shown on the toy screen). Easy, Medium, and Hard keep
separate local top-10 boards.

## Controls

| Input | Action |
|---|---|
| Arrow Left/Right | Steer (tap = light nudge, hold = full lock ramp) |
| Q / E | Select LOW / HIGH gear (HIGH unlocks at 10 m/s / 36 km/h) |
| Y | Use one collected nitro charge (five-second boost) |
| A-Z / Backspace / Enter | Enter and submit three initials after time-up |
| Left/Right + Enter | Select and start Easy / Medium / Hard |
| R | Restart while driving; return to difficulty selection from the leaderboard |
| T | Toggle toy view (game view ⇄ toy-on-desk view) |
| L | Cycle the rolling-tube look dial (0 / 0.5 / 1) |
| M | Mute / unmute audio |
| Gamepad | Left stick steers, LT brakes, LB/RB select LOW/HIGH, South confirms menus, North uses nitro, rumble on bumps |

## Run it

```sh
make serve        # dev server with auto-reload (trunk), opens the browser
```

## Develop

```sh
make check        # fmt + clippy (-D warnings) + unit tests + native build
make test         # unit tests only (game-core: deterministic sim logic)
make shots        # Playwright screenshot/state harness (needs make serve)
make package      # release wasm bundle -> release/fun-to-drive-web.zip
make build-wasm-webgpu  # WebGPU-capable build (falls back to WebGL2)
```

- Game logic: `crates/game-core` (pure, seeded, deterministic unit tests).
- Presentation: `crates/game-app` (Bevy 0.20, wasm32 via trunk).
- Docs: `docs/REQUEST.md` (charter), `docs/PLAN.md` (milestones + deferred
  tuning), `docs/DECISIONS.md`, `research/` (the real toys).
- 3D pipeline: `tools/astra/generate-model.sh` (astra writes a parametric
  Blender script; headless Blender exports `assets/models/toy.glb`).
- Visual QA: `tools/astra/eyes.sh` (astra reads screenshots against written
  expectations — the dev agent's eyes).

For deterministic browser QA, append `?qa_session_secs=2` (accepted range:
1–600 seconds). This replaces only the active QA run duration and does not
change the competitive Easy/Medium/Hard durations; first-load selection defaults
to Medium (six minutes). QA mode uses an empty ephemeral leaderboard and never
reads or writes production leaderboard storage. Normal leaderboard data is
local to the browser under the versioned `fun-to-drive.leaderboard.v2` key;
valid v1 rows migrate to the Hard board;
stored totals above conservative 600-second plausibility ceilings (100 cycles
or 100,000 passed/bumped cars) invalidate the stored board.

For weaker GPUs, append `?quality=low`. This reduces the offscreen game view
from 960×600 to 720×450 and disables bloom without changing simulation logic.
