# Fun to Drive — web

A web/WASM recreation of the 80s/90s electromechanical dashboard driving
toys (Tomy Turnin' Turbo Dashboard 1983, **Playmates Fun-to-Drive Corvette
Dashboard 1985**, Tectoy Turbo Video Driver 1991): the 2D drum game plays
*on the screen of a 3D toy* sitting on a desk, and the toy physically
reacts — steering wheel, gear lever, ignition key, G-force body lean, and a
full-body jolt (with gamepad rumble) when you crash.

## Controls

| Input | Action |
|---|---|
| Arrow Left/Right | Steer (tap = light nudge, hold = full lock ramp) |
| G | Gear LOW ⇄ HIGH (turbo lamp lights) |
| R / Enter | Start / restart after game over |
| T | Toggle toy view (game view ⇄ toy-on-desk view) |
| Y | Cycle the rolling-tube look dial (0 / 0.5 / 1 — flat by default) |
| Gamepad | Left stick steers (analog), South/RTZ shifts gear, rumble on crash |

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

- Game logic: `crates/game-core` (pure, seeded, 40 unit tests).
- Presentation: `crates/game-app` (Bevy 0.20, wasm32 via trunk).
- Docs: `docs/REQUEST.md` (charter), `docs/PLAN.md` (milestones + deferred
  tuning), `docs/DECISIONS.md`, `research/` (the real toys).
- 3D pipeline: `tools/astra/generate-model.sh` (astra writes a parametric
  Blender script; headless Blender exports `assets/models/toy.glb`).
- Visual QA: `tools/astra/eyes.sh` (astra reads screenshots against written
  expectations — the dev agent's eyes).
