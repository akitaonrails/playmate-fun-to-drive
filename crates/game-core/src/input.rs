//! Unified input model (PLAN.md M6): adapters turn raw devices into
//! `InputState`; the game only ever consumes `InputState`, never raw events,
//! so logic stays headless-testable.

use crate::tuning::Tuning;

/// Two-speed gearbox, OutRun-style (REQUEST R9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Gear {
    #[default]
    Low,
    High,
}

/// Resolve two edge-triggered selector inputs. Contradictory requests are
/// deliberately ignored rather than depending on adapter polling order.
pub fn gear_request(low: bool, high: bool) -> Option<Gear> {
    match (low, high) {
        (true, false) => Some(Gear::Low),
        (false, true) => Some(Gear::High),
        _ => None,
    }
}

/// Snapshot of player intent for one tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InputState {
    /// Steering in [-1, 1]; negative = left, positive = right.
    pub steer: f32,
    /// Edge-triggered, explicit selector request. `None` preserves the gear.
    pub gear_request: Option<Gear>,
    /// Edge-triggered: restart request this tick.
    pub reset: bool,
    /// Edge-triggered: pause/resume toggle this tick.
    pub pause: bool,
    /// Edge-triggered: spend one nitro charge this tick.
    pub nitro: bool,
}

/// Keyboard steering adapter (REQUEST R0/R8): a short press is a light
/// nudge; holding ramps the intensity to full; releasing decays quickly —
/// an approximation of the toy's analog wheel for digital keys.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct KeyboardSteering {
    held_dir: f32,
    held_secs: f32,
    steer: f32,
}

impl KeyboardSteering {
    /// `left`/`right`: current key states. Returns the shaped steer value.
    pub fn update(&mut self, left: bool, right: bool, dt: f32, t: &Tuning) -> f32 {
        let dir = (right as i32 - left as i32) as f32;
        if dir != 0.0 {
            self.held_dir = dir;
            self.held_secs += dt;
            let ramp = (self.held_secs / t.kb_ramp_secs).min(1.0);
            let intensity = t.kb_tap_intensity + (1.0 - t.kb_tap_intensity) * ramp;
            self.steer = (dir * intensity).clamp(-1.0, 1.0);
        } else {
            self.held_secs = 0.0;
            self.steer *= (-dt * t.kb_release_decay).exp();
            if self.steer.abs() < 0.02 {
                self.steer = 0.0;
            }
        }
        self.steer
    }

    pub fn steer(&self) -> f32 {
        self.steer
    }
}

/// Apply a symmetric deadzone and renormalize to the full [-1, 1] range.
/// Used by the gamepad adapter and kept here so it is unit-tested once.
pub fn apply_deadzone(raw: f32, zone: f32) -> f32 {
    assert!((0.0..=0.5).contains(&zone), "deadzone must be in [0, 0.5]");
    let x = raw.clamp(-1.0, 1.0);
    if x.abs() <= zone {
        0.0
    } else {
        let span = 1.0 - zone;
        ((x.abs() - zone) / span).min(1.0) * x.signum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_zero_inside_zone() {
        assert_eq!(apply_deadzone(0.05, 0.1), 0.0);
        assert_eq!(apply_deadzone(-0.09, 0.1), 0.0);
    }

    #[test]
    fn deadzone_renormalized_and_signed() {
        assert!((apply_deadzone(0.55, 0.1) - (0.45 / 0.9)).abs() < 1e-6);
        assert!(apply_deadzone(-0.55, 0.1) < 0.0);
        assert!((apply_deadzone(1.0, 0.1) - 1.0).abs() < 1e-6);
        assert!((apply_deadzone(-1.0, 0.1) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn deadzone_clamps_out_of_range() {
        assert!((apply_deadzone(7.0, 0.1) - 1.0).abs() < 1e-6);
    }

    fn kb(t: crate::tuning::Tuning) -> crate::tuning::Tuning {
        t
    }

    #[test]
    fn short_tap_is_a_light_nudge() {
        let t = kb(crate::tuning::Tuning::default());
        let mut k = KeyboardSteering::default();
        let mut v = 0.0;
        for _ in 0..6 {
            v = k.update(false, true, 1.0 / 60.0, &t); // 100 ms tap
        }
        assert!(v > 0.3 && v < 0.55, "tap should be light: {v}");
    }

    #[test]
    fn long_hold_ramps_to_full() {
        let t = kb(crate::tuning::Tuning::default());
        let mut k = KeyboardSteering::default();
        let mut v = 0.0;
        for _ in 0..(2.0 / (1.0 / 60.0)) as usize {
            v = k.update(true, false, 1.0 / 60.0, &t); // 2 s hold
        }
        assert!((-1.0..=-0.95).contains(&v), "hold should reach full: {v}");
    }

    #[test]
    fn release_decays_to_zero() {
        let t = kb(crate::tuning::Tuning::default());
        let mut k = KeyboardSteering::default();
        for _ in 0..60 {
            k.update(false, true, 1.0 / 60.0, &t);
        }
        for _ in 0..30 {
            k.update(false, false, 1.0 / 60.0, &t); // 0.5 s release
        }
        assert_eq!(k.steer(), 0.0);
    }

    #[test]
    fn both_keys_cancel_like_release() {
        let t = kb(crate::tuning::Tuning::default());
        let mut k = KeyboardSteering::default();
        for _ in 0..30 {
            k.update(false, true, 1.0 / 60.0, &t);
        }
        let v = k.update(true, true, 1.0 / 60.0, &t);
        assert!(v >= 0.0, "L+R must not steer left");
    }

    #[test]
    fn gear_requests_are_explicit_and_mutually_exclusive() {
        assert_eq!(gear_request(true, false), Some(Gear::Low));
        assert_eq!(gear_request(false, true), Some(Gear::High));
        assert_eq!(gear_request(false, false), None);
        assert_eq!(gear_request(true, true), None);
    }
}
