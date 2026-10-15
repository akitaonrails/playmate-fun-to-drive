//! Central tuning table (PLAN.md regression protocol: all magic numbers live
//! here with tested bounds so feel-changes are reviewable and revertable).

/// Every gameplay constant in one place. Values are the M5 starting points,
/// derived from research/gameplay.md ("Tuning reference points").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Drum scroll speeds (screen-heights per second) for each gear.
    pub scroll_speed_low: f32,
    pub scroll_speed_high: f32,
    /// Seconds to ramp between gear speeds.
    pub gear_ramp_secs: f32,
    /// Lateral car travel limits (fraction of half-screen-width).
    pub steer_max: f32,
    /// Keyboard: steering velocity at press start (units/s).
    pub kb_nudge_rate: f32,
    /// Keyboard: extra velocity per second of sustained hold (short vs long
    /// press ramp, REQUEST R0/R8).
    pub kb_hold_ramp: f32,
    /// Near-miss forgiveness band as fraction of car width.
    pub collision_forgiveness: f32,
    /// Fuel tank duration in seconds (LOW gear baseline).
    pub fuel_secs: f32,
    /// Score (mileage) per second at LOW; HIGH multiplies by speed ratio.
    pub score_rate_low: f32,
    /// Gamepad analog deadzone.
    pub pad_deadzone: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            scroll_speed_low: 0.90,
            scroll_speed_high: 1.60,
            gear_ramp_secs: 0.6,
            steer_max: 0.82,
            kb_nudge_rate: 1.4,
            kb_hold_ramp: 2.2,
            collision_forgiveness: 0.25,
            fuel_secs: 75.0,
            score_rate_low: 10.0,
            pad_deadzone: 0.08,
        }
    }
}

impl Tuning {
    /// Sanity bounds — tuned values must stay playable; tested so a bad edit
    /// fails `make check` instead of shipping a regression.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(0.2..=3.0).contains(&self.scroll_speed_low) {
            return Err("scroll_speed_low out of [0.2, 3.0]");
        }
        if self.scroll_speed_high <= self.scroll_speed_low {
            return Err("HIGH gear must be faster than LOW");
        }
        if self.scroll_speed_high > self.scroll_speed_low * 3.0 {
            return Err("HIGH gear unreasonably faster than LOW");
        }
        if !(0.1..=2.0).contains(&self.gear_ramp_secs) {
            return Err("gear_ramp_secs out of [0.1, 2.0]");
        }
        if !(0.5..=1.0).contains(&self.steer_max) {
            return Err("steer_max out of [0.5, 1.0]");
        }
        if self.kb_hold_ramp <= self.kb_nudge_rate {
            return Err("kb_hold_ramp must exceed kb_nudge_rate (short vs long press)");
        }
        if !(0.0..=0.5).contains(&self.collision_forgiveness) {
            return Err("collision_forgiveness out of [0, 0.5]");
        }
        if !(15.0..=600.0).contains(&self.fuel_secs) {
            return Err("fuel_secs out of [15, 600]");
        }
        if !(0.0..=0.3).contains(&self.pad_deadzone) {
            return Err("pad_deadzone out of [0, 0.3]");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        assert!(Tuning::default().validate().is_ok());
    }

    #[test]
    fn rejects_high_not_faster() {
        let mut t = Tuning::default();
        t.scroll_speed_high = t.scroll_speed_low;
        assert!(t.validate().is_err());
    }

    #[test]
    fn rejects_missing_hold_ramp() {
        let mut t = Tuning::default();
        t.kb_hold_ramp = t.kb_nudge_rate;
        assert!(t.validate().is_err());
    }
}
