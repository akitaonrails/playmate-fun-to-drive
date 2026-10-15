# Part spec: `toy` — the outer dashboard-toy shell

Input to `tools/astra/generate-model.sh specs/toy-shell.md toy`.
Silhouette follows the Playmates "Fun-to-Drive Corvette Dashboard" formula
(pop-up headlights, screen centered in front of the wheel); rules reference
is the Tomy; size reference is the Tectoy Turbo Video Driver (~15 × 25 × 25
cm). Original design — no cloned decals or logos (research/README legal note).

## Scale & orientation

- Units: meters, real toy scale. Overall footprint ≈ 0.25 m wide (X),
  0.25 m deep (Y), 0.15–0.20 m tall (Z) including wheel.
- Player sits at −Y looking toward +Y; toy front (headlights) faces −Y.
- Rest pose: sits on ground plane (min Z = 0), body axis-aligned.

## Part names (exact, engine-asserted) & pivots

| Part | Pivot | Purpose |
|---|---|---|
| `Body` | origin at base center | one-piece dashboard shell (hood scoop, fenders, rear hump), non-moving |
| `ScreenFrame` | origin at screen recess center | recessed windshield bezel around the game screen; its inner face carries the `SCREEN` quad below |
| `Screen` | same as ScreenFrame | flat quad 0.16 × 0.10 m, slightly tilted toward player (~12°), **no material needed** (engine puts the game texture on it) — place a named empty `SCREEN_MARKER` at its center for M8 |
| `SteeringWheel` | origin at steering column top | torus + 3 spokes + hub, radius ≈ 0.055 m; rotates around its local Z (column axis, tilted ~25° from vertical toward player) — M9 |
| `GearLever` | origin at lever base on dashboard | slim lever with ball knob, length ≈ 0.05 m; tilts ±20° around base X axis for LOW/HIGH — M9 |
| `IgnitionKey` | origin at key slot | small key cylinder on dashboard face, rotates for power-on (stretch) |
| `HeadlightLeft`, `HeadlightRight` | at lamp centers | pop-up lamp blocks on the front fascia (raised position) |
| `Bumper` | origin at base front | front bumper bar under the headlights |
| `DashPanel` | origin at dashboard face center | instrument cluster face: speedo/tach/fuel recesses + one small `TurboLamp` hole (HIGH gear light, nods to the Tomy) |

## Materials (Principled BSDF, matte plastic)

- `MatBodyRed` — red (sRGB ≈ 0.80, 0.08, 0.06), roughness 0.45.
- `MatDashBlack` — near-black (0.06, 0.06, 0.06), roughness 0.60.
- `MatChrome` — light gray (0.75, 0.75, 0.78), metallic 0.9, roughness 0.25
  (bumper, key, wheel hub only).
- `MatTireBlack` — rough black for the wheel rim.
- No textures; flat colors only (engine adds the rest).

## Budget

- Total ≲ 25k triangles; `SteeringWheel` ≤ 96-segment torus; bevels via
  modifier where cheap. No subdiv above level 2.

## Manifest expectations

`assets/models/toy.manifest.json` lists every part above with rest
transform, pivot description, material name, and per-part vertex count;
engine smoke test (M7) asserts part presence by exact name.
