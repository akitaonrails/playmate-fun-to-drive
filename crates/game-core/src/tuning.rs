//! Central tuning table (PLAN.md regression protocol: all magic numbers live
//! here with tested bounds so feel-changes are reviewable and revertable).

/// Every gameplay constant in one place. Values are the M5 starting points,
/// derived from research/gameplay.md ("Tuning reference points").
///
/// Lateral units: 1.0 == one half road width. Longitudinal: meters/seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Drum scroll speeds (m/s) per gear.
    pub scroll_speed_low: f32,
    pub scroll_speed_high: f32,
    /// Time constant of the speed ramp between gears (seconds).
    pub gear_ramp_secs: f32,
    /// How strongly the printed curve pulls the road aside at the car row.
    pub curve_gain: f32,
    /// Rate at which road_x chases the curve target (per second at LOW speed).
    pub drift_rate: f32,
    /// Drum shift velocity at full steering input (half-widths/s).
    pub steer_rate: f32,
    /// |road_x| beyond this counts as off-road.
    pub off_road_limit: f32,
    /// Seconds of grace off-road before crashing.
    pub off_road_grace: f32,
    /// Player car half-width in half-road-width units.
    pub car_half_width: f32,
    /// Hazard half-width in half-road-width units.
    pub hazard_half_width: f32,
    /// Near-miss forgiveness factor shrinks the hit band.
    pub collision_forgiveness: f32,
    /// Fuel tank duration in seconds.
    pub fuel_secs: f32,
    /// Duration of the crash flash / crashed phase.
    pub crash_flash_secs: f32,
    /// Gamepad analog deadzone.
    pub pad_deadzone: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            scroll_speed_low: 12.0,
            scroll_speed_high: 22.0,
            gear_ramp_secs: 0.6,
            curve_gain: 1.05,
            drift_rate: 0.85,
            steer_rate: 1.7,
            off_road_limit: 1.02,
            off_road_grace: 0.35,
            car_half_width: 0.17,
            hazard_half_width: 0.17,
            collision_forgiveness: 0.25,
            fuel_secs: 75.0,
            crash_flash_secs: 0.9,
            pad_deadzone: 0.08,
        }
    }
}

impl Tuning {
    /// Sanity bounds — tuned values must stay playable; tested so a bad edit
    /// fails `make check` instead of shipping a regression.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(4.0..=40.0).contains(&self.scroll_speed_low) {
            return Err("scroll_speed_low out of [4, 40]");
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
        if !(0.0..=2.0).contains(&self.curve_gain) {
            return Err("curve_gain out of [0, 2]");
        }
        if !(0.05..=5.0).contains(&self.drift_rate) {
            return Err("drift_rate out of [0.05, 5]");
        }
        if !(0.3..=4.0).contains(&self.steer_rate) {
            return Err("steer_rate out of [0.3, 4]");
        }
        if self.steer_rate <= self.drift_rate * self.curve_gain {
            return Err("steering must be able to beat the curve drift");
        }
        if !(0.8..=1.3).contains(&self.off_road_limit) {
            return Err("off_road_limit out of [0.8, 1.3]");
        }
        if !(0.0..=2.0).contains(&self.off_road_grace) {
            return Err("off_road_grace out of [0, 2]");
        }
        if !(0.05..=0.4).contains(&self.car_half_width) {
            return Err("car_half_width out of [0.05, 0.4]");
        }
        if !(0.05..=0.4).contains(&self.hazard_half_width) {
            return Err("hazard_half_width out of [0.05, 0.4]");
        }
        if !(0.0..=0.5).contains(&self.collision_forgiveness) {
            return Err("collision_forgiveness out of [0, 0.5]");
        }
        if !(15.0..=600.0).contains(&self.fuel_secs) {
            return Err("fuel_secs out of [15, 600]");
        }
        if !(0.2..=3.0).contains(&self.crash_flash_secs) {
            return Err("crash_flash_secs out of [0.2, 3]");
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
    fn rejects_steering_weaker_than_drift() {
        let mut t = Tuning::default();
        t.steer_rate = t.drift_rate * t.curve_gain;
        assert!(t.validate().is_err());
    }
}
