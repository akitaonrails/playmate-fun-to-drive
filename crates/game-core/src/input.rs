//! Unified input model (PLAN.md M6 fills the adapters; M4 defines the shape).
//!
//! The game only ever consumes `InputState` — never raw keyboard/gamepad
//! events — so logic stays headless-testable.

/// Two-speed gearbox, OutRun-style (REQUEST R9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Gear {
    #[default]
    Low,
    High,
}

impl Gear {
    pub fn toggle(self) -> Self {
        match self {
            Gear::Low => Gear::High,
            Gear::High => Gear::Low,
        }
    }
}

/// Snapshot of player intent for one tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InputState {
    /// Steering in [-1, 1]; negative = left, positive = right.
    pub steer: f32,
    pub gear: Gear,
    /// Edge-triggered: gear toggle request this tick.
    pub gear_up: bool,
    /// Edge-triggered: restart request this tick.
    pub reset: bool,
}

/// Apply a symmetric deadzone and renormalize to the full [-1, 1] range.
/// Used by the gamepad adapter (M6) and kept here so it is unit-tested once.
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

    #[test]
    fn gear_toggles_low_high() {
        assert_eq!(Gear::Low.toggle(), Gear::High);
        assert_eq!(Gear::High.toggle(), Gear::Low);
    }
}
