//! Whole-toy body dynamics (PLAN M11/M12): a procedural spring-damper rig
//! standing in for "IK-like" animation. The toy has no skeleton; instead
//! pseudo-forces drive three spring channels (roll, pitch, heave) plus a
//! lateral shift, and crashes inject impulses that the same springs ring
//! out as shake. See docs/dynamics-notes.md for the rationale.

use crate::tuning::Tuning;

/// Under-damped spring step shared by all channels (and by the app's
/// wheel/lever animation). Returns the new position; `vel` is updated in
/// place.
pub fn spring_step(current: f32, vel: &mut f32, target: f32, k: f32, c: f32, dt: f32) -> f32 {
    *vel += (k * (target - current) - c * *vel) * dt;
    current + *vel * dt
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BodyDynamics {
    pub roll: f32,
    roll_vel: f32,
    pub pitch: f32,
    pitch_vel: f32,
    pub heave: f32,
    heave_vel: f32,
    pub shift: f32,
    shift_vel: f32,
    prev_speed: f32,
    /// Seconds of crash shake remaining (M12).
    pub shake_t: f32,
    /// Direction the crash knocked the body (-1..1).
    pub shake_dir: f32,
}

impl BodyDynamics {
    /// `steer` in [-1, 1]; `speed_norm` in [0, 1] (fraction of top speed);
    /// `speed` in m/s (for longitudinal pitch); `time_secs` drives heave.
    pub fn step(&mut self, steer: f32, speed_norm: f32, speed: f32, time_secs: f32, t: &Tuning) {
        let dt = 1.0 / 60.0; // fixed-step: called from the sim tick rate
        let sn = speed_norm.clamp(0.0, 1.0);

        // pseudo lateral G: turning left (steer<0) throws the body right
        let roll_target = steer * sn * t.body_roll_gain;
        self.roll = spring_step(
            self.roll,
            &mut self.roll_vel,
            roll_target,
            t.body_k,
            t.body_c,
            dt,
        );
        let shift_target = -steer * sn * t.body_shift_gain;
        self.shift = spring_step(
            self.shift,
            &mut self.shift_vel,
            shift_target,
            t.body_k,
            t.body_c,
            dt,
        );

        // longitudinal: acceleration pitches the nose
        let accel = ((speed - self.prev_speed) / dt).clamp(-30.0, 30.0);
        self.prev_speed = speed;
        let pitch_target = -accel * 0.01 * t.body_pitch_gain;
        self.pitch = spring_step(
            self.pitch,
            &mut self.pitch_vel,
            pitch_target,
            t.body_k,
            t.body_c,
            dt,
        );

        // speed wobble: a gentle road-frequency heave
        let heave_target = (time_secs * t.heave_freq).sin() * sn * t.heave_gain;
        self.heave = spring_step(
            self.heave,
            &mut self.heave_vel,
            heave_target,
            t.heave_k,
            t.heave_c,
            dt,
        );

        if self.shake_t > 0.0 {
            self.shake_t = (self.shake_t - dt).max(0.0);
        }
    }

    /// Crash impulse (M12): kicks the springs; `dir` in [-1, 1] favors the
    /// side the road slipped toward. Deterministic (no RNG).
    pub fn crash_impulse(&mut self, dir: f32, t: &Tuning) {
        let d = dir.clamp(-1.0, 1.0);
        self.shake_dir = d;
        self.roll_vel += d * t.crash_roll_impulse;
        self.pitch_vel += 0.9;
        self.heave_vel -= t.crash_roll_impulse * 0.35;
        self.shake_t = t.crash_shake_secs;
    }

    /// High-frequency crash jitter while `shake_t` runs down:
    /// (x shift, z heave, roll jitter).
    pub fn shake_offset(&self, time_secs: f32, t: &Tuning) -> (f32, f32, f32) {
        if self.shake_t <= 0.0 {
            return (0.0, 0.0, 0.0);
        }
        // monotonic decaying bias (always visible, even in a single sampled
        // frame) + a slow sine so it trembles while recovering
        let bias = self.shake_t * self.shake_t;
        let x = self.shake_dir * bias * t.crash_shake_gain
            + (time_secs * 11.0).sin() * self.shake_t * t.crash_shake_gain * 0.4;
        let z = (time_secs * 9.0).sin() * self.shake_t * t.crash_shake_gain * 0.3;
        let roll = self.shake_dir * bias * t.crash_shake_roll
            + (time_secs * 13.0).sin() * self.shake_t * t.crash_shake_roll * 0.35;
        (x, z, roll)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> Tuning {
        Tuning::default()
    }

    #[test]
    fn sustained_steer_rolls_and_settles() {
        let mut b = BodyDynamics::default();
        let t = t();
        for i in 0..(5.0 * 60.0) as usize {
            b.step(1.0, 1.0, 12.0, i as f32 / 60.0, &t);
        }
        let target = t.body_roll_gain;
        assert!((b.roll - target).abs() < target * 0.05, "roll {}", b.roll);
        assert!((b.shift + t.body_shift_gain).abs() < t.body_shift_gain * 0.1);
    }

    #[test]
    fn roll_mirrors_with_steer_direction() {
        let t = t();
        let mut left = BodyDynamics::default();
        let mut right = BodyDynamics::default();
        for i in 0..(2.0 * 60.0) as usize {
            let ts = i as f32 / 60.0;
            left.step(-1.0, 1.0, 12.0, ts, &t);
            right.step(1.0, 1.0, 12.0, ts, &t);
        }
        assert!(
            left.roll < 0.0 && right.roll > 0.0,
            "{} {}",
            left.roll,
            right.roll
        );
        assert!((left.roll + right.roll).abs() < 1e-4, "symmetric");
    }

    #[test]
    fn overshoot_bounded_and_release_decays() {
        let t = t();
        let mut b = BodyDynamics::default();
        let mut peak = 0.0f32;
        for i in 0..(3.0 * 60.0) as usize {
            b.step(1.0, 1.0, 12.0, i as f32 / 60.0, &t);
            peak = peak.max(b.roll);
        }
        assert!(peak > t.body_roll_gain, "should overshoot a little");
        assert!(peak < t.body_roll_gain * 1.45, "bounded: {peak}");
        for i in 0..(2.0 * 60.0) as usize {
            b.step(0.0, 1.0, 12.0, 3.0 + i as f32 / 60.0, &t);
        }
        assert!(b.roll.abs() < t.body_roll_gain * 0.05, "decay {}", b.roll);
    }

    #[test]
    fn idle_stays_put() {
        let t = t();
        let mut b = BodyDynamics::default();
        for i in 0..120 {
            b.step(0.0, 0.0, 0.0, i as f32 / 60.0, &t);
        }
        assert!(b.roll.abs() < 1e-5 && b.pitch.abs() < 1e-3 && b.heave.abs() < 1e-4);
    }

    #[test]
    fn crash_impulse_rings_then_settles() {
        let t = t();
        let mut b = BodyDynamics::default();
        b.crash_impulse(0.7, &t);
        assert!(b.shake_t > 0.5);
        // the visible shake at impact lives in shake_offset
        let (_, _, roll_jit) = b.shake_offset(0.12, &t);
        assert!(
            roll_jit.abs() > t.body_roll_gain,
            "crash must shake visibly"
        );
        let mut peak = 0.0f32;
        for i in 0..(3.0 * 60.0) as usize {
            b.step(0.0, 0.0, 0.0, i as f32 / 60.0, &t);
            peak = peak.max(b.roll.abs());
        }
        assert!(peak > 0.0);
        assert!(b.roll.abs() < 1e-3, "settles: {}", b.roll);
        assert!(b.shake_t == 0.0);
        assert!(b.roll.is_finite() && b.heave.is_finite() && b.pitch.is_finite());
    }

    #[test]
    fn shake_offset_decays_to_zero() {
        let t = t();
        let mut b = BodyDynamics::default();
        b.crash_impulse(1.0, &t);
        let (x0, z0, r0) = b.shake_offset(0.1, &t);
        assert!(x0.abs() > 0.0 && r0.abs() > 0.0);
        let _ = z0;
        for i in 0..(2.0 * 60.0) as usize {
            b.step(0.0, 0.0, 0.0, i as f32 / 60.0, &t);
        }
        assert_eq!(b.shake_offset(10.0, &t), (0.0, 0.0, 0.0));
    }

    #[test]
    fn speed_wobble_scales_with_speed() {
        let t = t();
        let mut fast = BodyDynamics::default();
        let mut slow = BodyDynamics::default();
        for i in 0..(2.0 * 60.0) as usize {
            let ts = i as f32 / 60.0;
            fast.step(0.0, 1.0, 22.0, ts, &t);
            slow.step(0.0, 0.2, 5.0, ts, &t);
        }
        assert!(fast.heave.abs() >= slow.heave.abs() - 1e-6);
    }
}
