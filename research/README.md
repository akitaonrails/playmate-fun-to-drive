# Research: the "Fun to Drive" dashboard-toy family

This directory holds reference images and notes on the 80s/90s electromechanical
dashboard driving toys this project recreates, gathered 2026-10-15.

> Note: the agent model used in this harness cannot view images. Image contents
> were verified as valid JPEG/PNG files and sourced from the pages described in
> `sources.md`. Visual inspection/QA during development is routed through
> `tools/astra/eyes.sh` (astra accepts image input) or reviewed by you directly.

## What we identified

| Toy | Year | Maker | Why it matters |
|---|---|---|---|
| **Turnin' Turbo Dashboard** | 1983 | Tomy | The archetype. Best documented (teardowns, schematic, gameplay rules). |
| **Fun-to-Drive Corvette Dashboard** | 1985 | Playmates | **This is the "Playmate Fun to Drive" the project is named after.** Red Corvette-styled shell, pop-up headlights, screen directly in front of the wheel. |
| **Turbo Video Driver** | 1991 | Tectoy (Brazil) | The Brazilian variant the user remembers alongside it. Hybrid mechanical/electronic: lights, sound, physically moving track, volante + câmbio + ignition key. ~15 × 25 × 25 cm. |
| *Racing Turbo* (clone) | 80s/90s | (various) | Tomy-style clones sold in Europe; one 2026 teardown documents the mechanism photos here. |

Family tree and identification details: `toy-identification.md`
How the physical toy works: `toy-mechanics.md`
How the game plays (what we mimic): `gameplay.md`
Where everything came from: `sources.md`

## Image index

- `images/tomy-1983/` — front, front-end, **schematic diagram**, packaging, teardown video still
- `images/playmates-1985/` — eBay listing photos (5), Etsy listing photo, YouTube short still
- `images/tectoy-1991/` — two Brazilian YouTube video stills
- `images/mechanism/` — clone teardown (drum, screen, headlights) + Circuitbeard
  OutRun-arcade conversion internals (stock condition, shifter, self-center
  mechanism, steering actuator, finished build)

## Legal / usage note

These images are third-party copyright, kept locally for private study and as
modelling reference only. The shipped game's 3D shell and art must be an
original creation "inspired by" the toy category (dashboard shape, wheel, gear
lever), not a replica of any trademarked shell or decal art. Do not copy the
printed road artwork or brand logos into shipped assets.
