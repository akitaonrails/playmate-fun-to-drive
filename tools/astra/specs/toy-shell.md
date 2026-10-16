# Part spec: `toy` — the Playmates Fun-to-Drive dashboard toy (simplified)

Input to `tools/astra/generate-model.sh specs/toy-shell.md toy`.
Reference photos are ATTACHED to the astra call (research/images/playmates-
1985/). Match the real toy's SILHOUETTE AND PROPORTIONS in a simplified
low-poly style — it must read at a glance as "that toy": the FRONT HALF of
a red Corvette-style car with a black cockpit, cut off behind the cockpit,
sitting like a display unit. NOT a generic dashboard box.

## Silhouette (from the photos — follow this closely)

- **Base + front clip**: wide, LOW wedge (~0.25 m wide, ~0.22 m deep, body
  top ~0.09 m at the cowl). Rounded-corner footprint, slight forward rake:
  the nose is low, the body rises toward the cockpit.
- **Hood/nose**: sloped wedge; near the front edge TWO POP-UP HEADLIGHTS —
  raised, slightly angled rectangular pods (silver bezel + dark lens),
  left and right of center.
- **Black windshield band** spanning the body above the dash, raked back.
- **Center screen pod**: the screen sits in a raised black bezel pod
  DIRECTLY BEHIND THE WHEEL, centered on the body (the toy's signature —
  screen straight in front of the driver, NOT off to the side).
- **Cockpit**: black dash top spanning between the fenders; below it the
  red console carrying the controls. Rear of the cockpit: a red cowl/hump
  (rear deck) rising behind the screen pod, ending in a small spoiler lip.
- **Sides**: subtle angled side vents/scoops just behind the front wheels'
  fender bulges. Fender bulges over the (implied) front wheel arches.
- **Front bumper**: dark low strip under the nose with two small running
  lights.
- No external wheels — it is a dashboard display unit.

## Controls (separate parts, exact names, pivots)

| Part | Pivot | Notes |
|---|---|---|
| `SteeringWheel` | center of the wheel hub | flat 3-spoke wheel ~0.05 m, facing the player, tilted back ~25° from vertical (top away). **Disc plane ⊥ column: geometry in the node's local XY, thin local Z; the wheel spins about local Z** (assert in manifest) |
| `GearLever` | lever base on console, right of wheel | slim lever + ball knob ~0.045 m along local +Y; tilts ±20° about local X |
| `IgnitionKey` | key slot on dash face, left of wheel | small key cylinder; twists about local X |
| `HeadlightLeft` / `HeadlightRight` | lamp pod base | pop-up pods on the nose |
| `Bumper` | base front | dark strip |
| `DashPanel` | dash face center | black panel with 3 gauge recesses + small `TurboLamp` hole right side |
| `ScreenFrame` | screen recess center | raised black bezel pod, local XY = screen plane, +Z toward player |
| `Screen` | same as ScreenFrame | 0.15 × 0.095 m quad, dark `MatScreenDark`, carries TEXCOORD_0 (engine paints the game here) |
| `Body` | base center | the red wedge clip w/ hood, fenders, vents, cowl hump, spoiler lip |
| plus `SCREEN_MARKER` empty at the Screen center | | |

## Materials

- `MatBodyRed` glossy red (0.82, 0.07, 0.06, roughness 0.35).
- `MatCockpitBlack` soft black (0.05, roughness 0.7) — windshield band,
  dash top, screen bezel pod, bumper.
- `MatConsoleRed` = MatBodyRed.
- `MatChrome` (0.8, 0.8, 0.85, metallic 0.9, roughness 0.2) — headlight
  bezels, key, wheel hub.
- `MatTireBlack` rough black — wheel rim/grips, lever knob, vents.
- `MatLampLens` dark bluish glass (0.1, 0.12, 0.16, roughness 0.15).
- `MatScreenDark` (0.045, 0.055, 0.07) for the Screen quad.

## Budget & rules

- ≤ 60k triangles total; bevels/modifiers where cheap. Deterministic,
  parametric, no external assets, headless-runnable, glTF export with
  Y-up (player at +Z) exactly as before. Manifest: parts + pivots + rest
  transforms + vertex counts, PLUS an assert that `SteeringWheel` geometry
  is thin along its local Z (disc normal = local Z, spin axis = local Z).
- Scale: meters; rest pose sits on ground (min Z... use glTF: min Y = 0).
