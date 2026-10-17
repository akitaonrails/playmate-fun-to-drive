# TECH STACK — survey & dependency policy

Requirement added 2026-10-15 (REQUEST R13): use popular, proven libraries so
we don't reinvent wheels — but do **not** pull in tiny libs for things that
are trivial to re-implement. Surveyed 2026-10-15.

## Needs that shape the choice

1. WASM in-browser, GPU accelerated (WebGPU if possible, WebGL2 fallback).
2. 2D sprite game loop *and* a 3D scene (toy shell) with the game rendered
   onto the toy's screen (render-to-texture) — so a pure-2D engine is out.
3. Gamepad (analog) + keyboard input in the browser.
4. glTF loading (Blender-produced assets), simple PBR/plastic materials.
5. Unit-testable deterministic game logic + headless screenshot testing.

## Framework options

| Option | Web/WASM | GPU | Gamepad | Render-to-texture | glTF | Notes |
|---|---|---|---|---|---|---|
| **Rust + Bevy** (recommended) | First-class WASM target; WebGL2 default backend, WebGPU build supported (bevy.org/news/bevy-webgpu) | wgpu (WebGPU + WebGL2) | Built-in `bevy_input`/gilrs — works natively; **web gamepad needs an M4 spike** (gilrs wasm support; fallback: ~50-line `navigator.getGamepads()` adapter) | Built-in (camera → image texture on mesh) | Built-in `bevy_gltf` | ECS keeps game logic unit-testable; one language for logic+radi. User profile defaults to Rust. |
| TS + three.js (r186, WebGPU renderer production-ready since early 2026) | Native web | WebGPU + WebGL2 | `navigator.getGamepads()` directly, zero risk | `WebGLRenderTarget` | GLTFLoader | Huge ecosystem; logic in TS (tests via vitest); diverges from Rust profile. |
| TS + Babylon.js (9.26, WebGPU since 6.0) | Native web | WebGPU + WebGL2 | Built-in controllers | DynamicTexture/RTT | Built-in | Full engine (editor, GUI, physics) — more than we need. |
| Rust + macroquad / ggez | WASM ok | WebGL (WebGPU weak/none) | Basic | Manual | Manual | Great for small 2D; poor fit for the 3D toy + RTT. |
| Unity/Godot web export | Heavy | — | — | — | — | Overkill; web builds bloated; against "wasm-light" spirit. |
| Phaser / PixiJS | — | — | — | — | — | 2D-only; cannot host the 3D toy. Rejected. |

**Recommendation (D1, pending user confirmation): Rust + Bevy.** It matches
the user's Rust-default profile, keeps logic in one tested language, and has
native answers for RTT + glTF + WebGPU. The single open risk is web gamepad
through gilrs — mitigated by a tiny direct Gamepad-API adapter if the spike
fails. If the user prefers JS-first iteration, **TS + three.js** is the
runner-up (everything else in the plan stays the same shape).

## Chosen supporting tools (popular, load-bearing)

| Need | Tool | Why not hand-rolled |
|---|---|---|
| WASM build + dev server + auto-reload (`make serve`) | **trunk** (or `wasm-server-runner`) | Standard rust-wasm bundler; hot-reload wiring, asset pipeline. |
| JS↔WASM bridge | **wasm-bindgen** (+ web-sys) | Required plumbing for rust on the web. |
| Headless browser smoke + screenshots | **Playwright** (Node) | De-facto standard; drives synthetic input, captures PNGs + console; used by `tools/screenshots/`. |
| 3D asset authoring/export | **Blender** (headless CLI) | Installed; glTF export built in; scripts generated via astra (D3). |
| astra access | **codex CLI** | Installed; subscription login (D2). |

## Explicitly NOT adding (re-implement in `game-core`, with tests)

| Tempting lib | Why skip |
|---|---|
| Physics engine (Rapier/etc.) | No rigid-body physics needed; springs/dampers for the toy rig are ~100 lines, deterministic, and must be unit-tested in-house anyway. |
| Tweening/easing lib | A handful of easing fns + spring integrator; trivially ours. |
| Input-mapping framework | Two devices, ~6 actions; a thin `InputState` adapter is smaller than any framework's config surface. |
| Audio lib (early) | Engine built-in audio suffices until M13; decide then if engine-hum loops need more. |
| UI/widget lib | HUD and end-of-run leaderboard are lightweight Bevy sprites/Text2d. |
| Random/noise crates beyond `rand` (and maybe a small seeded value-noise for paper grain) | Keep RNG seeded and centralized. |

Policy going forward: any new dependency must name the wheel it avoids, be
maintained/popular, and be approved in `DECISIONS.md` — one line each.
