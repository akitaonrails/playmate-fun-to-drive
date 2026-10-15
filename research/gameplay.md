# How the game plays (what we mimic first)

From the Tomy Turnin' Turbo Dashboard documentation (Toy Tales) and owner
accounts; this is the behavior milestone M2 must reproduce before any 3D/toy
work.

## Core loop (original)

1. Turn the **ignition key** → power on, engine sound starts.
2. Shift into **LOW** (or HIGH) → drum motor starts, road rolls toward the
   player under the fixed car.
3. **Steer** left/right → the road shifts/bends under the car.
4. **Hazards** (oncoming cars / roadside objects printed on the drum loop)
   scroll past; avoid them by steering.
5. The **trip mileage counter** advances with distance and doubles as the
  **score**. Higher speed (HIGH gear) scores faster.
6. **Fuel gauge** depletes over time/distance; empty = game over.
7. **Off-road / crash**: clicking sound, the viewing screen **flashes**, and
   the mileage counter **stops** — game over. Restart via the reset button.

## Differences across the family

- **Playmates (1985)**: no real crash state — you drive until the noise
  annoys your parents; hazards pass by but steering "misses" are not fatal.
  Road loop ≈ 2 s.
- **Tomy (1983)**: crash/off-road + fuel are real fail states; scoring as above.
- **Tectoy (1991)**: same formula, hybrid electronics (lights + sound +
  moving track); Brazilian market.

## Our version (user spec, phase 1)

- Static car image fixed to the bottom-center of the screen.
- Rolling background simulating street driving; hazards to avoid.
- Gear = LOW/HIGH only (OutRun-style).
- Crash/hit feedback, score (distance), and game-over/restart.
- "Flat / modern" look first; physical-toy effects (drum curvature, lamp,
  paper grain, shake) come later as shader/animation layers.
- Input: gamepad **analog stick left/right** for steering; keyboard arrows
  with **short/long press** ramping; gear on a gamepad button / keyboard key.

## Tuning reference points

- Road loop period ≈ 2 s at LOW on the Playmates (feels fast; hazards arrive
  roughly every 0.5–1 s within the loop).
- HIGH gear noticeably faster (≈1.5–2×) with higher score rate — the trade
  is reaction time.
- Crash should be forgiving near-miss based (toy-era feel), not pixel-perfect
  collision: use horizontal overlap bands on the car sprite.
