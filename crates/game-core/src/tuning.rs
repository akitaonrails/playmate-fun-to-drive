//! Central tuning table (PLAN.md regression protocol: all magic numbers live
//! here with tested bounds so feel-changes are reviewable and revertable).

use crate::road::TrafficMotion;

/// Every gameplay constant in one place. Values are the M5 starting points,
/// derived from research/gameplay.md ("Tuning reference points").
///
/// Lateral units: 1.0 == one half road width. Longitudinal: meters/seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Overall drivetrain/scroll scale per difficulty.
    pub speed_easy_multiplier: f32,
    pub speed_medium_multiplier: f32,
    pub speed_hard_multiplier: f32,
    /// Drum scroll speeds (m/s) per gear.
    pub scroll_speed_low: f32,
    pub scroll_speed_high: f32,
    pub shift_up_min_speed: f32,
    pub recovery_bog_speed: f32,
    pub downshift_safe_speed: f32,
    pub downshift_safe_retention: f32,
    pub downshift_unsafe_retention: f32,
    pub low_overrev_start: f32,
    /// Time constant of the speed ramp between gears (seconds).
    pub gear_ramp_secs: f32,
    /// Time constant while braking; larger than the gear ramp for a gradual stop.
    pub brake_ramp_secs: f32,
    /// How strongly the printed curve pulls the road aside at the car row.
    pub curve_gain: f32,
    /// Fractional target-speed response per unit of road grade. Positive
    /// slopes slow the car; negative slopes give the same modest assistance.
    pub slope_speed_gain: f32,
    pub slope_speed_min_factor: f32,
    pub slope_speed_max_factor: f32,
    /// Rate at which road_x chases the curve target (per second at LOW speed).
    pub drift_rate: f32,
    /// Drum shift velocity at full steering input (half-widths/s).
    pub steer_rate: f32,
    /// |road_x| beyond this counts as off-road.
    pub off_road_limit: f32,
    /// Seconds of grace off-road before crashing.
    pub off_road_grace: f32,
    /// Lateral simulation clamp and rate at which off-road grace recovers.
    pub max_lateral_offset: f32,
    pub off_road_recovery_rate: f32,
    /// Speed retained at maximum lateral excursion. Between the road edge
    /// and the excursion clamp, drag increases smoothly toward this factor.
    pub off_road_min_speed_factor: f32,
    /// Player car half-width in half-road-width units.
    pub car_half_width: f32,
    /// Hazard half-width in half-road-width units.
    pub hazard_half_width: f32,
    /// Near-miss forgiveness factor shrinks the hit band.
    pub collision_forgiveness: f32,
    /// Nitro pickup spacing and boost behavior.
    pub nitro_pickup_first_m: f32,
    pub nitro_pickup_interval_m: f32,
    pub nitro_duration_secs: f32,
    pub nitro_cooldown_secs: f32,
    pub nitro_speed_multiplier: f32,
    pub nitro_pickup_half_width: f32,
    pub nitro_lane_offset: f32,
    pub nitro_capacity: u8,
    /// Difficulty-specific oncoming lane changes. Easy changes least often
    /// and across the longest/farthest horizon band; Hard is more active.
    pub traffic_easy: TrafficMotion,
    pub traffic_medium: TrafficMotion,
    pub traffic_hard: TrafficMotion,
    /// Crash/bump response.
    pub bump_target_speed_factor: f32,
    pub bump_lateral_kick: f32,
    pub bump_speed_retention: f32,
    pub bump_flash_intensity: f32,
    /// Duration of the crash flash / crashed phase.
    pub crash_flash_secs: f32,
    /// Body dynamics (M11): shared spring stiffness/damping for the rig.
    pub body_k: f32,
    pub body_c: f32,
    /// Roll (radians) at full steer and full speed.
    pub body_roll_gain: f32,
    /// Lateral shift (meters, in half-road-width scale) at full steer.
    pub body_shift_gain: f32,
    /// Pitch gain from longitudinal acceleration.
    pub body_pitch_gain: f32,
    /// Heave wobble amplitude (meters) at full speed and its frequency.
    pub heave_gain: f32,
    pub heave_freq: f32,
    pub heave_k: f32,
    pub heave_c: f32,
    /// Crash shake (M12).
    pub crash_roll_impulse: f32,
    pub crash_shake_secs: f32,
    pub crash_shake_gain: f32,
    /// Roll jitter amplitude (radians) at crash moment.
    pub crash_shake_roll: f32,
    /// Gamepad analog deadzone.
    pub pad_deadzone: f32,
    /// Keyboard: steering intensity on a short tap (short vs long press,
    /// REQUEST R0/R8).
    pub kb_tap_intensity: f32,
    /// Keyboard: seconds of hold to ramp from tap intensity to full steer.
    pub kb_ramp_secs: f32,
    /// Keyboard: release decay rate (higher = snappier centering).
    pub kb_release_decay: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            speed_easy_multiplier: 0.80,
            speed_medium_multiplier: 0.90,
            speed_hard_multiplier: 1.00,
            scroll_speed_low: 12.0,
            scroll_speed_high: 24.0,
            shift_up_min_speed: 10.0,
            recovery_bog_speed: 4.5,
            downshift_safe_speed: 12.0,
            downshift_safe_retention: 0.94,
            downshift_unsafe_retention: 0.82,
            low_overrev_start: 11.0,
            gear_ramp_secs: 0.6,
            brake_ramp_secs: 1.2,
            curve_gain: 1.20,
            slope_speed_gain: 1.6,
            slope_speed_min_factor: 0.90,
            slope_speed_max_factor: 1.10,
            drift_rate: 0.75,
            steer_rate: 2.0,
            off_road_limit: 1.02,
            off_road_grace: 0.45,
            max_lateral_offset: 1.35,
            off_road_recovery_rate: 2.0,
            off_road_min_speed_factor: 0.42,
            car_half_width: 0.20,
            hazard_half_width: 0.14,
            collision_forgiveness: 0.25,
            nitro_pickup_first_m: 90.0,
            nitro_pickup_interval_m: 180.0,
            nitro_duration_secs: 5.0,
            nitro_cooldown_secs: 3.0,
            nitro_speed_multiplier: 1.30,
            nitro_pickup_half_width: 0.20,
            nitro_lane_offset: 0.62,
            nitro_capacity: 3,
            traffic_easy: TrafficMotion {
                chance_percent: 12,
                change_start_m: 42.0,
                change_end_m: 22.0,
                ..TrafficMotion::default()
            },
            traffic_medium: TrafficMotion::default(),
            traffic_hard: TrafficMotion {
                chance_percent: 32,
                change_start_m: 34.0,
                change_end_m: 18.0,
                ..TrafficMotion::default()
            },
            bump_target_speed_factor: 0.35,
            bump_lateral_kick: 0.38,
            bump_speed_retention: 0.45,
            bump_flash_intensity: 0.35,
            crash_flash_secs: 0.9,
            body_k: 55.0,
            body_c: 7.5,
            body_roll_gain: 0.045,
            body_shift_gain: 0.005,
            body_pitch_gain: 0.045,
            heave_gain: 0.0022,
            heave_freq: 9.0,
            heave_k: 90.0,
            heave_c: 10.0,
            crash_roll_impulse: 1.8,
            crash_shake_secs: 1.1,
            crash_shake_gain: 0.020,
            crash_shake_roll: 0.13,
            pad_deadzone: 0.08,
            kb_tap_intensity: 0.35,
            kb_ramp_secs: 0.8,
            kb_release_decay: 9.0,
        }
    }
}

impl Tuning {
    /// Sanity bounds — tuned values must stay playable; tested so a bad edit
    /// fails `make check` instead of shipping a regression.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(0.5..=1.0).contains(&self.speed_easy_multiplier)
            || !(self.speed_easy_multiplier..=1.0).contains(&self.speed_medium_multiplier)
            || !(self.speed_medium_multiplier..=1.0).contains(&self.speed_hard_multiplier)
            || self.speed_hard_multiplier != 1.0
        {
            return Err("difficulty speed multipliers must increase from Easy to Hard");
        }
        if !(4.0..=40.0).contains(&self.scroll_speed_low) {
            return Err("scroll_speed_low out of [4, 40]");
        }
        if self.scroll_speed_high <= self.scroll_speed_low {
            return Err("HIGH gear must be faster than LOW");
        }
        if self.scroll_speed_high > self.scroll_speed_low * 3.0 {
            return Err("HIGH gear unreasonably faster than LOW");
        }
        if !(0.0..self.scroll_speed_low).contains(&self.shift_up_min_speed) {
            return Err("shift_up_min_speed must be below LOW target");
        }
        if !(0.0..self.shift_up_min_speed).contains(&self.recovery_bog_speed) {
            return Err("recovery_bog_speed must be below shift threshold");
        }
        if self.downshift_safe_speed != self.scroll_speed_low {
            return Err("downshift safe speed must equal LOW target");
        }
        if !(0.0..=1.0).contains(&self.downshift_safe_retention)
            || !(0.0..self.downshift_safe_retention).contains(&self.downshift_unsafe_retention)
        {
            return Err("downshift retention factors invalid");
        }
        if !(0.0..self.scroll_speed_low).contains(&self.low_overrev_start) {
            return Err("low_overrev_start must be below LOW target");
        }
        if !(0.1..=2.0).contains(&self.gear_ramp_secs) {
            return Err("gear_ramp_secs out of [0.1, 2.0]");
        }
        if !(self.gear_ramp_secs..=3.0).contains(&self.brake_ramp_secs) {
            return Err("brake_ramp_secs must be gradual and at most 3 seconds");
        }
        if !(0.0..=2.0).contains(&self.curve_gain) {
            return Err("curve_gain out of [0, 2]");
        }
        if !(0.0..=3.0).contains(&self.slope_speed_gain) {
            return Err("slope_speed_gain out of [0, 3]");
        }
        if !(0.5..=1.0).contains(&self.slope_speed_min_factor)
            || !(1.0..=1.5).contains(&self.slope_speed_max_factor)
            || self.slope_speed_min_factor > self.slope_speed_max_factor
        {
            return Err("slope speed factor bounds invalid");
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
        if !(self.off_road_limit..=2.0).contains(&self.max_lateral_offset) {
            return Err("max_lateral_offset must exceed the road limit");
        }
        if !(0.1..=10.0).contains(&self.off_road_recovery_rate) {
            return Err("off_road_recovery_rate out of [0.1, 10]");
        }
        if !(0.2..=0.9).contains(&self.off_road_min_speed_factor) {
            return Err("off_road_min_speed_factor out of [0.2, 0.9]");
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
        if !(30.0..=300.0).contains(&self.nitro_pickup_first_m) {
            return Err("nitro_pickup_first_m out of [30, 300]");
        }
        if !(80.0..=260.0).contains(&self.nitro_pickup_interval_m) {
            return Err("nitro_pickup_interval_m out of [80, 260]");
        }
        if !(2.0..=8.0).contains(&self.nitro_duration_secs) {
            return Err("nitro_duration_secs out of [2, 8]");
        }
        if !(1.0..=8.0).contains(&self.nitro_cooldown_secs) {
            return Err("nitro_cooldown_secs out of [1, 8]");
        }
        if !(1.05..=1.75).contains(&self.nitro_speed_multiplier) {
            return Err("nitro_speed_multiplier out of [1.05, 1.75]");
        }
        if !(0.05..=0.5).contains(&self.nitro_pickup_half_width) {
            return Err("nitro_pickup_half_width out of [0.05, 0.5]");
        }
        if !(0.2..=0.9).contains(&self.nitro_lane_offset) {
            return Err("nitro_lane_offset out of [0.2, 0.9]");
        }
        if !(1..=8).contains(&self.nitro_capacity) {
            return Err("nitro_capacity out of [1, 8]");
        }
        for traffic in [self.traffic_easy, self.traffic_medium, self.traffic_hard] {
            traffic.validate()?;
            if traffic.change_end_m < 16.0 {
                return Err("traffic lane changes must finish near the horizon");
            }
        }
        if !(self.traffic_easy.chance_percent <= self.traffic_medium.chance_percent
            && self.traffic_medium.chance_percent <= self.traffic_hard.chance_percent)
        {
            return Err("traffic chance must increase with difficulty");
        }
        if !(0.0..=1.0).contains(&self.bump_target_speed_factor)
            || !(0.0..=1.0).contains(&self.bump_speed_retention)
        {
            return Err("bump speed factors out of [0, 1]");
        }
        if !(0.0..=0.8).contains(&self.bump_lateral_kick) {
            return Err("bump_lateral_kick out of [0, 0.8]");
        }
        if !(0.0..=1.0).contains(&self.bump_flash_intensity) {
            return Err("bump_flash_intensity out of [0, 1]");
        }
        if !(0.2..=3.0).contains(&self.crash_flash_secs) {
            return Err("crash_flash_secs out of [0.2, 3]");
        }
        if !(20.0..=200.0).contains(&self.body_k) {
            return Err("body_k out of [20, 200]");
        }
        if !(4.0..=25.0).contains(&self.body_c) {
            return Err("body_c out of [4, 25]");
        }
        if !(0.01..=0.15).contains(&self.body_roll_gain) {
            return Err("body_roll_gain out of [0.01, 0.15]");
        }
        if !(0.001..=0.03).contains(&self.body_shift_gain) {
            return Err("body_shift_gain out of [0.001, 0.03]");
        }
        if !(0.005..=0.12).contains(&self.body_pitch_gain) {
            return Err("body_pitch_gain out of [0.005, 0.12]");
        }
        if !(0.0005..=0.01).contains(&self.heave_gain) {
            return Err("heave_gain out of [0.0005, 0.01]");
        }
        if !(2.0..=25.0).contains(&self.heave_freq) {
            return Err("heave_freq out of [2, 25]");
        }
        if !(20.0..=240.0).contains(&self.heave_k) {
            return Err("heave_k out of [20, 240]");
        }
        if !(2.0..=30.0).contains(&self.heave_c) {
            return Err("heave_c out of [2, 30]");
        }
        if !(1.0..=8.0).contains(&self.crash_roll_impulse) {
            return Err("crash_roll_impulse out of [1, 8]");
        }
        if !(0.02..=0.2).contains(&self.crash_shake_roll) {
            return Err("crash_shake_roll out of [0.02, 0.2]");
        }
        if !(0.2..=2.5).contains(&self.crash_shake_secs) {
            return Err("crash_shake_secs out of [0.2, 2.5]");
        }
        if !(0.001..=0.1).contains(&self.crash_shake_gain) {
            return Err("crash_shake_gain out of [0.001, 0.1]");
        }
        if !(0.0..=0.3).contains(&self.pad_deadzone) {
            return Err("pad_deadzone out of [0, 0.3]");
        }
        if !(0.15..=0.8).contains(&self.kb_tap_intensity) {
            return Err("kb_tap_intensity out of [0.15, 0.8]");
        }
        if !(0.2..=2.0).contains(&self.kb_ramp_secs) {
            return Err("kb_ramp_secs out of [0.2, 2.0]");
        }
        if !(3.0..=20.0).contains(&self.kb_release_decay) {
            return Err("kb_release_decay out of [3, 20]");
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
    fn rejects_incoherent_difficulty_speed_multipliers() {
        for invalid in [
            Tuning {
                speed_easy_multiplier: 0.95,
                speed_medium_multiplier: 0.90,
                ..Tuning::default()
            },
            Tuning {
                speed_hard_multiplier: 0.95,
                ..Tuning::default()
            },
            Tuning {
                speed_easy_multiplier: f32::NAN,
                ..Tuning::default()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn rejects_steering_weaker_than_drift() {
        let mut t = Tuning::default();
        t.steer_rate = t.drift_rate * t.curve_gain;
        assert!(t.validate().is_err());
    }

    #[test]
    fn rejects_extreme_off_road_drag() {
        let t = Tuning {
            off_road_min_speed_factor: 0.0,
            ..Default::default()
        };
        assert!(t.validate().is_err());
    }

    #[test]
    fn rejects_incoherent_drivetrain_and_cooldown_values() {
        let t = Tuning {
            shift_up_min_speed: Tuning::default().scroll_speed_low,
            ..Default::default()
        };
        assert!(t.validate().is_err());

        let t = Tuning {
            downshift_unsafe_retention: Tuning::default().downshift_safe_retention,
            ..Default::default()
        };
        assert!(t.validate().is_err());

        let t = Tuning {
            nitro_cooldown_secs: 0.0,
            ..Default::default()
        };
        assert!(t.validate().is_err());
    }

    #[test]
    fn validates_every_spring_and_shake_parameter() {
        for invalid in [
            Tuning {
                heave_k: -1.0,
                ..Tuning::default()
            },
            Tuning {
                heave_c: f32::NAN,
                ..Tuning::default()
            },
            Tuning {
                crash_shake_gain: -0.01,
                ..Tuning::default()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn validates_extracted_driving_and_pickup_parameters() {
        for invalid in [
            Tuning {
                slope_speed_min_factor: 1.1,
                slope_speed_max_factor: 1.0,
                ..Tuning::default()
            },
            Tuning {
                max_lateral_offset: 0.5,
                ..Tuning::default()
            },
            Tuning {
                off_road_recovery_rate: f32::NAN,
                ..Tuning::default()
            },
            Tuning {
                nitro_capacity: 0,
                ..Tuning::default()
            },
            Tuning {
                bump_speed_retention: 1.1,
                ..Tuning::default()
            },
            Tuning {
                traffic_easy: TrafficMotion {
                    change_end_m: 8.0,
                    ..Tuning::default().traffic_easy
                },
                ..Tuning::default()
            },
            Tuning {
                traffic_easy: TrafficMotion {
                    chance_percent: 40,
                    ..Tuning::default().traffic_easy
                },
                ..Tuning::default()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }
}
