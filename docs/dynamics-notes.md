# Dynamics notes (M11/M12)

Choice: **procedural spring-damper rig**, not IK chains.

The toy is one rigid shell (no skeleton, no bones), so a solver-style IK
chain has nothing to solve. What the user asked for — "when I turn left the
toy simulate G-force to the right and move/rotate slightly" — is a body
ATTITUDE response to pseudo-forces. That maps exactly to springs:

- **Roll** follows `-lateral G`: target `roll = steer * speed_norm * gain`
  (turn left → body leans right). Under-damped so it swings a little past
  and settles, like a light plastic toy on a desk.
- **Shift**: the body also slides a few mm toward the G direction.
- **Pitch** follows longitudinal acceleration (gear changes pitch the nose).
- **Heave** is a gentle speed-scaled road wobble (sinusoid target into an
  under-damped spring so it lags and rings subtly).
- **Crash (M12)**: one impulse into the same springs (roll velocity kick +
  heave + pitch) plus a short high-frequency deterministic jitter (sin
  sweeps, no RNG) while a shake timer runs down. The under-damped springs
  ring out the rest — the whole toy visibly shakes, then recovers.
- **Rumble**: `GamepadRumbleRequest::Add` (STRONG_MAX, 450 ms) to every
  connected pad at the crash moment.

All constants live in `Tuning` (body_k/c shared by channels + per-channel
gains) with validated bounds; the rig is unit-tested in game-core
(convergence, symmetry, bounded overshoot, crash settle, no NaN).

Sign convention: toy front faces +Z (glTF); positive Z-rotation tilts the
body top toward -X (viewer's left with X right). steer < 0 (left) → roll
target < 0 → top tilts +X (right) — leaning INTO the pseudo-force, away
from the turn, like a real body.
