//! The road drum: a fixed, repeating loop with a printed centerline curve and
//! hazard placements — generated once from a seed, like the toy's printed
//! drum (the Playmates loop repeated every ~2 s; ours is a few seconds).

use crate::rng::Rng;

pub const CURVE_SAMPLES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardKind {
    Oncoming,
    Roadside,
}

/// All lateral values are normalized: 1.0 = half road width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hazard {
    pub kind: HazardKind,
    /// Lateral center relative to the road centerline, in half-widths.
    pub x: f32,
    /// Position along the loop, meters in [0, length).
    pub s: f32,
}

#[derive(Debug, Clone)]
pub struct RoadLoop {
    pub length: f32,
    pub hazards: Vec<Hazard>,
    /// Sampled centerline curve, periodic, normalized to max |1.0|.
    curve: Vec<f32>,
}

impl RoadLoop {
    pub fn generate(rng: &mut Rng) -> Self {
        let length = rng.range(48.0, 72.0);

        // Integer-period harmonics => seamless wrap at the loop joint.
        let mut phases = [0.0f32; 3];
        let mut amps = [0.0f32; 3];
        for i in 0..3 {
            phases[i] = rng.next_f32() * std::f32::consts::TAU;
            amps[i] = rng.range(0.3, 1.0) / (i as f32 + 1.0);
        }

        let mut curve = Vec::with_capacity(CURVE_SAMPLES);
        let mut peak: f32 = 1.0e-6;
        for n in 0..CURVE_SAMPLES {
            let t = n as f32 / CURVE_SAMPLES as f32;
            let mut v = 0.0;
            for i in 0..3 {
                v += amps[i] * (std::f32::consts::TAU * (i as f32 + 1.0) * t + phases[i]).sin();
            }
            peak = peak.max(v.abs());
            curve.push(v);
        }
        // warm-up/warm-down: amplitude fades in over the first 20% of the
        // loop and out over the last 10%, so runs start gentle and the loop
        // joint stays smooth (a cold full-amplitude first curve is unfair).
        // Peak is re-normalized AFTER the fade so max |curve| stays 1.0.
        let n = CURVE_SAMPLES as f32;
        for (i, v) in curve.iter_mut().enumerate() {
            let i = i as f32;
            let fade = (i / (0.20 * n)).min((n - i) / (0.10 * n)).clamp(0.0, 1.0);
            *v *= fade;
        }
        let peak = curve.iter().fold(1.0e-6f32, |m, v| m.max(v.abs()));
        for v in &mut curve {
            *v /= peak;
        }

        let count = rng.range(6.0, 9.0).round() as usize;
        let lanes = [-0.72f32, 0.0, 0.72];
        let mut hazards = Vec::with_capacity(count);
        for k in 0..count {
            // first hazard no earlier than 22% into the loop (fair start)
            let t = 0.22 + 0.78 * (k as f32 + rng.range(0.25, 0.75)) / count as f32;
            let s = t * length;
            let lane = lanes[(rng.next_u64() % lanes.len() as u64) as usize];
            let x = (lane + rng.range(-0.05, 0.05)).clamp(-0.9, 0.9);
            let kind = if rng.next_f32() < 0.75 {
                HazardKind::Oncoming
            } else {
                HazardKind::Roadside
            };
            hazards.push(Hazard { kind, x, s });
        }

        RoadLoop {
            length,
            hazards,
            curve,
        }
    }

    /// Replace the curve with a "square" track: long straights joined by
    /// four constant-curvature corners (2 right, 2 left) — feel-testing
    /// track per user request (drive straight, turn at corners).
    pub fn apply_square_curve(&mut self) {
        let n = CURVE_SAMPLES as f32;
        let mut raw = vec![0.0f32; CURVE_SAMPLES];
        for (i, v) in raw.iter_mut().enumerate() {
            let t = i as f32 / n;
            *v = if (0.22..=0.34).contains(&t) {
                1.0
            } else if (0.47..=0.59).contains(&t) {
                -1.0
            } else if (0.72..=0.84).contains(&t) {
                1.0
            } else if (0.94..=1.0).contains(&t) || t <= 0.06 {
                -1.0
            } else {
                0.0
            };
        }
        // soften the step transitions a little so corners ease in
        let mut curve = raw.clone();
        for i in 0..CURVE_SAMPLES {
            let prev = raw[(i + CURVE_SAMPLES - 1) % CURVE_SAMPLES];
            let next = raw[(i + 1) % CURVE_SAMPLES];
            curve[i] = (raw[i] * 4.0 + prev + next) / 6.0;
        }
        self.curve = curve;
    }

    /// Flat loop with explicit hazards: deterministic course for tests and
    /// simple scenarios (attract mode etc.).
    pub fn straight(length: f32, hazards: Vec<Hazard>) -> Self {
        RoadLoop {
            length,
            hazards,
            curve: vec![0.0; CURVE_SAMPLES],
        }
    }

    /// Centerline offset at loop position `s` (meters), wrapping smoothly.
    pub fn curve_at(&self, s: f32) -> f32 {
        let n = self.curve.len() as f32;
        let pos = (s.rem_euclid(self.length) / self.length) * n;
        let i = (pos.floor() as usize) % self.curve.len();
        let j = (i + 1) % self.curve.len();
        let f = pos - pos.floor();
        self.curve[i] * (1.0 - f) + self.curve[j] * f
    }

    pub fn max_abs_curve(&self) -> f32 {
        self.curve.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loop_from(seed: u64) -> RoadLoop {
        RoadLoop::generate(&mut Rng::new(seed))
    }

    #[test]
    fn deterministic_from_seed() {
        let a = loop_from(99);
        let b = loop_from(99);
        assert_eq!(a.length, b.length);
        assert_eq!(a.hazards, b.hazards);
    }

    #[test]
    fn curve_is_periodic_at_joint() {
        let r = loop_from(1234);
        let eps = 0.02;
        assert!((r.curve_at(0.0) - r.curve_at(r.length)).abs() < eps);
        assert!((r.curve_at(-0.5) - r.curve_at(r.length - 0.5)).abs() < eps);
    }

    #[test]
    fn loop_starts_gentle_and_joins_smoothly() {
        let r = loop_from(1234);
        let peak = r.max_abs_curve();
        let early: Vec<f32> = r
            .curve
            .iter()
            .take((CURVE_SAMPLES as f32 * 0.15) as usize)
            .map(|v| v.abs())
            .collect();
        let early_max = early.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            early_max < 0.65 * peak,
            "first 15% should be gentler: {early_max} vs peak {peak}"
        );
        assert!(r.curve.first().unwrap().abs() < 1e-4);
        // joint smoothness itself is covered by curve_is_periodic_at_joint
    }

    #[test]
    fn square_track_has_straights_and_corners() {
        let mut r = loop_from(5);
        r.apply_square_curve();
        let n = CURVE_SAMPLES as f32;
        let at = |t: f32| r.curve_at(t * r.length);
        assert!(at(0.10).abs() < 0.1, "straight section");
        assert!(at(0.28) > 0.8, "right corner");
        assert!(at(0.53) < -0.8, "left corner");
        assert!(at(0.65).abs() < 0.1, "straight again");
        assert!((at(0.0) - at(1.0)).abs() < 1e-4, "seamless wrap");
        let _ = n;
    }

    #[test]
    fn curve_bounded_and_alive() {
        for seed in [1u64, 2, 5, 8] {
            let r = loop_from(seed);
            assert!(r.max_abs_curve() <= 1.0 + 1e-4);
            assert!(r.max_abs_curve() > 0.9);
            assert!(r.curve.iter().any(|v| *v != r.curve[0]));
        }
    }

    #[test]
    fn hazards_on_loop_and_spaced() {
        let r = loop_from(77);
        assert!((6..=9).contains(&r.hazards.len()));
        for h in &r.hazards {
            assert!(h.s >= 0.0 && h.s < r.length);
            assert!(h.x.abs() <= 0.9);
        }
        // roughly even spacing in the playable window [22%, 100%) of the loop
        let span = r.length * 0.78;
        let slot = span / r.hazards.len() as f32;
        for h in &r.hazards {
            assert!(h.s >= r.length * 0.22 - 1e-4);
        }
        for (k, h) in r.hazards.iter().enumerate() {
            let rel = h.s - r.length * 0.22;
            assert!(rel >= k as f32 * slot - 1e-4 && rel < (k + 1) as f32 * slot + slot);
        }
    }
}
