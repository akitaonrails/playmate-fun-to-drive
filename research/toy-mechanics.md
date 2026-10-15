# How the physical toy works

Compiled from the Toy Tales article on the Tomy (1983), the Circuitbeard
OutRun-arcade conversion build log (which dissects the original mechanisms),
and the lutfionboard 2026 clone teardown.

## The "display": lamp + rolling drum

- The road scene is **printed on a loop of plastic/paper wrapped around a
  rotating cylinder (drum)** mounted behind the windshield opening.
- A **light bulb inside/behind the drum assembly** lights the scene
  ("simple projector consisting of lightbulb and rotating drum").
- A DC motor (geared down) spins the drum; speed varies with throttle/gear.
  The whole thing runs on 4 D-cells.
- The **player's car is a fixed printed graphic** (decal on the
  windshield/window plane), NOT on the drum — the car never moves; the world
  moves past it. Road loop repeats (≈2 s per revolution on the Playmates).
- Steering does **not** turn a car sprite: turning the wheel **shifts the drum
  laterally left/right** (and the printed road bends side to side), so the
  road appears to curve under the fixed car.

## Steering linkage

- The end of the steering column has an **actuator that converts the wheel's
  rotary motion into linear side-to-side motion**, coupled to the drum
  assembly (Circuitbeard replaced exactly this with a slide potentiometer —
  photos in `images/mechanism/cb-steering-*.jpg`).
- Feel: ratio-limited, springy, purely analog.

## Gear shift

- **Two positions: LOW and HIGH** (OutRun arcade style, minus neutral).
- Internally the lever sweeps a **gear with a rod attachment side to side**
  (`images/mechanism/cb-shifter-*.jpg`).
- HIGH gear spins the drum motor faster (more score, less reaction time) and
  on the Tomy lights a **"turbo" indicator** on the dashboard.
- There is **no accelerator pedal** — gear choice IS the throttle.

## Other controls / outputs

- **Ignition key**: power on/off + engine sound starts.
- **Reset push-button** for the mileage/score counter.
- **Dashboard instruments**: printed speedometer, tachometer, fuel gauge
  (needle/light animations driven mechanically); trip mileage counter doubles
  as the score display.
- **Sound**: motor whine proportional to speed, clicking on off-road/crash.
- **Headlights** on the front fascia (pop-up on the Playmates Corvette).

## Materials / build (for the 3D shell reference)

- Molded ABS-style plastic, red body, black dashboard face, chrome-ish trim,
  sticker decals for instruments; toy-scale (~25 cm wide class; Tectoy unit
  15 × 25 × 25 cm).
- Screen opening is a shallow recessed "windshield" in front of the drum,
  wider than tall, slightly tilted toward the player.

## What we emulate in-engine (per the user's plan)

| Physical part | Digital counterpart |
|---|---|
| Printed drum + lamp | Scrolling road texture (later: drum-curvature/lamp shader pass) |
| Fixed car decal | Static player-car sprite locked to screen bottom-center |
| Wheel → drum lateral shift | Steering input scrolls road laterally / bends road |
| HI/LO gear | Two-speed drum scroll rate, score multiplier, "turbo" light |
| Mileage counter | Score HUD; stops on crash |
| Fuel gauge | Depleting timer → game over when empty |
| Off-road click + flash | Crash/hazard feedback; later: whole-toy shake |
