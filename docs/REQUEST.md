# REQUEST — project charter (verbatim + structured)

Captured 2026-10-15, before any implementation, so no detail is lost.

## Original request (verbatim)

> This is to become a new web-based/wasm game, gpu accelerated if possible,
> gamepad support/keyboard support. First I want you to research online for
> images and references of the old 90s playmate fun to drive toy (and also the
> brazilian tectoy variants). Create a research directory to hold images and
> information about how this toy looks like, how they work, how the game
> itself works. Now, you will need to write down this request to not lose
> details, and you will have to carefully plan steps before implementing
> anything, all written down in docs/. First: I want you to create some tools/
> directory and create instrumentation for my codex harness to load astra
> using my subscription plan. I will want you to use astra to generate 3d
> models (you may also open Blender - which has the Blender MCP, if you need
> to). 2. I will want you tou use astra to design the outer toy shell, with
> wheel and gear shift separated so they are move and animate. 3. you will
> build the game itself separately first, the original is a static car image
> with a rolling background that simulates the car driving on the streets and
> avoiding hazards along the way. I want to first mimic this same behavior.
> 4. plan for effects such as simulating the rolling background coming from a
> physical rolling tube on top become maybe some shader effects, the game can
> look "flat" as if it's modern game, we add toy-looking effects later 5. when
> the game loop itself is working, we map the game window to the 3d model toy
> screen area, so it feels like the game is playing as it was the real toy 6.
> then you map the wheel animation with analog left/right on the gamepad or
> short/long press on keyboard arrow left/right. 7. gear shift should work
> like in the original outrun game: just a low and high gears. 8. in the end,
> when the game loop works and it's playable, then I want you to research and
> add inverse kinematics-like animation to the whole toy, so when I turn left
> the toy simulate G-force to the right and move/rotate slightely. 9. add
> extra flare when car hits a hazard on the streets, make the entire toy shake
> like it crashed, for example. 10. make this all using proper test-based
> developement, unit test suite, and an easy way for me to load the game in
> the browser and test myself on every step. plan to testability first (both
> manual myself, but also you taking screenshots and checking for tweaks and
> fixes). we want to progress avoiding regressions on each step.

## Structured requirements

### R0 — Platform & inputs (constraints on everything)

- Web-based, WASM, GPU-accelerated if possible (WebGPU with fallback).
- **Gamepad** support (analog left/right steering, buttons) AND **keyboard**
  (arrow left/right; short vs long press ramping; gear key).

### R1 — Research (done 2026-10-15)

- Online research for images/references of the 90s "Playmate fun to drive"
  toy + Brazilian Tectoy variants → `research/` with images and write-ups of
  looks, mechanism, and gameplay.

### R2 — Docs & planning (this directory, before implementing)

- Write this request down verbatim; plan all steps carefully in `docs/PLAN.md`.

### R3 — tools/: astra instrumentation (user's "First")

- `tools/` directory with instrumentation for the user's **codex harness** to
  load **astra** (`gpt-6-astra`) using their **ChatGPT subscription plan**
  (no API key).
- astra is used to **generate 3D models**; Blender may also be driven (user
  has the Blender MCP available in another harness; this repo uses headless
  `blender` CLI, which is installed).

### R4 — 3D toy shell via astra

- Design the **outer toy shell** with **steering wheel and gear shift as
  separated parts** so they can move/animate independently.

### R5 — Game first, separately

- Build the 2D game loop first, mimicking the original: **static car image +
  rolling background** simulating driving on streets, **avoiding hazards**.

### R6 — Effects planned (implemented later)

- Simulate the rolling background coming from a **physical rolling tube**
  (drum) — likely as **shader effects** layered on the game screen.
- Game may look "flat"/modern at first; toy-look effects added later.

### R7 — Game mapped onto the toy screen

- Once the game loop works, **map the game window to the 3D model's screen
  area** so it feels like the real toy playing.

### R8 — Wheel animation ← input

- Map wheel animation to **gamepad analog left/right** or keyboard
  **short/long press** on arrows.

### R9 — Gear = LOW/HIGH only

- Exactly like original OutRun: two gears.

### R10 — IK-like whole-toy dynamics (late phase)

- Research and add inverse-kinematics-like animation to the whole toy:
  steering left makes the toy **simulate G-force to the right** (slight
  move/rotate), etc. Only after the game loop is playable.

### R11 — Crash flare

- Extra flair when the car hits a hazard: the **entire toy shakes** like it
  crashed (plus screen/flash/sound feedback).

### R12 — TDD, testability first, no regressions

- Proper test-based development with a **unit test suite**.
- An **easy way to load the game in the browser and self-test at every step**.
- Testability first: manual testing by the user, **and** agent-driven
  screenshot checks with tweak/fix iterations.
- Progress must **avoid regressions at each step**.

### R13 — Don't reinvent the wheel (added 2026-10-15, mid-planning)

- Research popular libraries/frameworks that make the work easier and use
  them where they are load-bearing.
- **But** do not add small/unimportant libs when the functionality is easy
  to re-implement (and would be better unit-tested in-house).
- Survey + dependency policy: `docs/TECH-STACK.md`; adoptions logged in
  `docs/DECISIONS.md`.

## Acceptance criteria (per phase) → see PLAN.md milestones

Each milestone in `docs/PLAN.md` carries: deliverable, unit tests, manual
browser checklist, screenshot-QA step, and exit criteria. A milestone is only
done when all four are green and recorded.
