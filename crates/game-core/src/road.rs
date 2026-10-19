//! The road drum: a fixed, repeating loop with a printed centerline curve and
//! hazard placements — generated once from a seed, like the toy's printed
//! drum (the Playmates loop repeated every ~2 s; ours is a few seconds).

use crate::rng::Rng;

pub const CURVE_SAMPLES: usize = 128;

fn smoothstep(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardKind {
    Oncoming,
    Tree,
    Billboard,
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

/// Tweakable behavior for occasional distant lane changes. Keeping this with
/// the road makes generation, rendering, and collision share one contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficMotion {
    pub chance_percent: u8,
    pub change_start_m: f32,
    pub change_end_m: f32,
    pub lane_offset: f32,
    pub lane_jitter: f32,
}

impl TrafficMotion {
    pub const DISABLED: Self = Self {
        chance_percent: 0,
        change_start_m: 38.0,
        change_end_m: 20.0,
        lane_offset: 0.72,
        lane_jitter: 0.05,
    };

    pub fn validate(self) -> Result<(), &'static str> {
        if self.chance_percent > 100 {
            return Err("traffic lane-change chance exceeds 100%");
        }
        if !self.change_start_m.is_finite()
            || !self.change_end_m.is_finite()
            || self.change_end_m < 0.0
            || self.change_start_m <= self.change_end_m
        {
            return Err("traffic lane-change distances are invalid");
        }
        if !(0.3..=0.9).contains(&self.lane_offset) {
            return Err("traffic lane offset out of [0.3, 0.9]");
        }
        if !(0.0..=0.15).contains(&self.lane_jitter) {
            return Err("traffic lane jitter out of [0, 0.15]");
        }
        Ok(())
    }
}

impl Default for TrafficMotion {
    fn default() -> Self {
        Self {
            chance_percent: 25,
            ..Self::DISABLED
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoadLoop {
    pub length: f32,
    pub hazards: Vec<Hazard>,
    /// Sampled centerline curve, periodic, normalized to max |1.0|.
    curve: Vec<f32>,
    /// Periodic terrain elevation in meters, kept deliberately low-relief.
    elevation: Vec<f32>,
    traffic_motion_enabled: bool,
}

impl RoadLoop {
    pub fn generate(rng: &mut Rng) -> Self {
        let length = rng.range(180.0, 240.0);
        let direction = if rng.next_f32() < 0.5 { -1.0 } else { 1.0 };
        let curve = Self::technical_curve(direction);

        // Two low harmonics make broad, seamless rises and dips. Their
        // amplitude keeps grades around a few percent rather than mountains.
        let elevation_phase = rng.next_f32() * std::f32::consts::TAU;
        let mut elevation = Vec::with_capacity(CURVE_SAMPLES);
        for n in 0..CURVE_SAMPLES {
            let t = n as f32 / CURVE_SAMPLES as f32;
            elevation.push(
                0.94 * (std::f32::consts::TAU * t + elevation_phase).sin()
                    + 0.31 * (std::f32::consts::TAU * 2.0 * t + elevation_phase * 0.47).sin(),
            );
        }

        let traffic_count = rng.range(12.0, 17.0).round() as usize;
        let scenery_count = rng.range(8.0, 13.0).round() as usize;
        let traffic_motion = TrafficMotion::default();
        debug_assert!(traffic_motion.validate().is_ok());
        let lanes = [-traffic_motion.lane_offset, 0.0, traffic_motion.lane_offset];
        let mut hazards = Vec::with_capacity(traffic_count + scenery_count);
        for k in 0..traffic_count {
            // first hazard no earlier than 22% into the loop (fair start)
            let t = 0.22 + 0.78 * (k as f32 + rng.range(0.25, 0.75)) / traffic_count as f32;
            let s = t * length;
            let lane = lanes[(rng.next_u64() % lanes.len() as u64) as usize];
            let x = (lane + rng.range(-0.05, 0.05)).clamp(-0.9, 0.9);
            hazards.push(Hazard {
                kind: HazardKind::Oncoming,
                x,
                s,
            });
        }
        for k in 0..scenery_count {
            let t = (k as f32 + rng.range(0.15, 0.85)) / scenery_count as f32;
            let side = if rng.next_f32() < 0.5 { -1.0 } else { 1.0 };
            hazards.push(Hazard {
                kind: if rng.next_f32() < 0.62 {
                    HazardKind::Tree
                } else {
                    HazardKind::Billboard
                },
                x: side * rng.range(1.08, 1.26),
                s: t * length,
            });
        }
        hazards.sort_by(|a, b| a.s.total_cmp(&b.s));

        RoadLoop {
            length,
            hazards,
            curve,
            elevation,
            traffic_motion_enabled: true,
        }
    }

    /// Deterministic approach line for traffic. A minority of cars move to an
    /// adjacent lane while distant, then lock their line before reaching the
    /// player. Rendering and collision both call this function.
    pub fn hazard_x_at(&self, hazard: &Hazard, ahead: f32, motion: TrafficMotion) -> f32 {
        if hazard.kind != HazardKind::Oncoming
            || !self.traffic_motion_enabled
            || motion.chance_percent == 0
        {
            return hazard.x;
        }
        let Some(target) = oncoming_lane_target(hazard, motion) else {
            return hazard.x;
        };
        let t = ((motion.change_start_m - ahead) / (motion.change_start_m - motion.change_end_m))
            .clamp(0.0, 1.0);
        let smooth = t * t * (3.0 - 2.0 * t);
        hazard.x + (target - hazard.x) * smooth
    }

    /// A designed arcade course: a fair opening straight followed by broad
    /// sweepers, three sustained short-radius corners, and an opposing S-bend.
    /// Curve magnitude represents inverse radius; segment length represents
    /// sweep. Every corner has a deliberate eased entry, constant-radius body,
    /// and eased exit so the road reads as one continuous ribbon rather than a
    /// sequence of abruptly shifted straight sections.
    fn technical_curve(direction: f32) -> Vec<f32> {
        #[derive(Clone, Copy)]
        struct Corner {
            start: f32,
            full: f32,
            release: f32,
            end: f32,
            curvature: f32,
        }

        let corners = [
            // Long, readable opening sweeper.
            Corner {
                start: 0.13,
                full: 0.17,
                release: 0.27,
                end: 0.31,
                curvature: 0.66 * direction,
            },
            // A tighter opposing corner after only a short breath.
            Corner {
                start: 0.32,
                full: 0.35,
                release: 0.42,
                end: 0.46,
                curvature: -direction,
            },
            // Overlapping releases create one fluid left-right S transition.
            Corner {
                start: 0.49,
                full: 0.52,
                release: 0.58,
                end: 0.62,
                curvature: 0.90 * direction,
            },
            Corner {
                start: 0.58,
                full: 0.62,
                release: 0.68,
                end: 0.72,
                curvature: -0.90 * direction,
            },
            Corner {
                start: 0.75,
                full: 0.78,
                release: 0.85,
                end: 0.88,
                curvature: direction,
            },
            // Finish with a medium-radius bend and settle before the seam.
            Corner {
                start: 0.90,
                full: 0.925,
                release: 0.955,
                end: 0.985,
                curvature: -0.78 * direction,
            },
        ];
        let envelope = |t: f32, corner: Corner| {
            if !(corner.start..corner.end).contains(&t) {
                return 0.0;
            }
            if t < corner.full {
                smoothstep((t - corner.start) / (corner.full - corner.start))
            } else if t <= corner.release {
                1.0
            } else {
                1.0 - smoothstep((t - corner.release) / (corner.end - corner.release))
            }
        };

        (0..CURVE_SAMPLES)
            .map(|index| {
                let t = index as f32 / CURVE_SAMPLES as f32;
                corners
                    .iter()
                    .map(|corner| corner.curvature * envelope(t, *corner))
                    .sum::<f32>()
                    .clamp(-1.0, 1.0)
            })
            .collect()
    }

    /// Flat loop with explicit hazards: deterministic course for tests and
    /// simple scenarios (attract mode etc.).
    pub fn straight(length: f32, hazards: Vec<Hazard>) -> Self {
        Self::try_straight(length, hazards).expect("straight road must have valid geometry")
    }

    /// Validated constructor for authored/test courses.
    pub fn try_straight(length: f32, hazards: Vec<Hazard>) -> Result<Self, &'static str> {
        if !length.is_finite() || length <= 0.0 {
            return Err("road length must be finite and positive");
        }
        if hazards.iter().any(|hazard| {
            !hazard.x.is_finite() || !hazard.s.is_finite() || hazard.s < 0.0 || hazard.s >= length
        }) {
            return Err("hazard must be finite and inside the road loop");
        }
        Ok(RoadLoop {
            length,
            hazards,
            curve: vec![0.0; CURVE_SAMPLES],
            elevation: vec![0.0; CURVE_SAMPLES],
            traffic_motion_enabled: false,
        })
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

    /// Terrain height at loop position `s`, in meters.
    pub fn elevation_at(&self, s: f32) -> f32 {
        self.sample_periodic(&self.elevation, s)
    }

    /// Signed rise/run. Positive is uphill in the driving direction.
    pub fn grade_at(&self, s: f32) -> f32 {
        let ds = (self.length / CURVE_SAMPLES as f32).clamp(0.35, 1.0);
        (self.elevation_at(s + ds) - self.elevation_at(s - ds)) / (2.0 * ds)
    }

    pub fn max_abs_grade(&self) -> f32 {
        (0..CURVE_SAMPLES)
            .map(|i| {
                self.grade_at(self.length * i as f32 / CURVE_SAMPLES as f32)
                    .abs()
            })
            .fold(0.0, f32::max)
    }

    fn sample_periodic(&self, samples: &[f32], s: f32) -> f32 {
        let n = samples.len() as f32;
        let pos = (s.rem_euclid(self.length) / self.length) * n;
        let i = (pos.floor() as usize) % samples.len();
        let j = (i + 1) % samples.len();
        let f = pos - pos.floor();
        samples[i] * (1.0 - f) + samples[j] * f
    }
}

fn oncoming_lane_target(hazard: &Hazard, tuning: TrafficMotion) -> Option<f32> {
    let mut hash = u64::from(hazard.s.to_bits())
        ^ u64::from(hazard.x.to_bits()).rotate_left(23)
        ^ 0x9e37_79b9_7f4a_7c15;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    // Roughly three cars in ten change lane on each deterministic course.
    if hash % 100 >= u64::from(tuning.chance_percent) {
        return None;
    }

    let half_lane = tuning.lane_offset * 0.5;
    let lane = if hazard.x < -half_lane {
        -1
    } else if hazard.x > half_lane {
        1
    } else {
        0
    };
    let target_lane = match lane {
        -1 | 1 => 0,
        _ if hash & 0x10 == 0 => -1,
        _ => 1,
    };
    let original_center = lane as f32 * tuning.lane_offset;
    let jitter = (hazard.x - original_center).clamp(-tuning.lane_jitter, tuning.lane_jitter);
    Some(target_lane as f32 * tuning.lane_offset + jitter)
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
    fn technical_track_has_straights_and_mixed_radius_corners() {
        let r = loop_from(5);
        let at = |t: f32| r.curve_at(t * r.length);
        assert!(at(0.04).abs() < 0.1, "fair opening straight");
        assert!((0.58..0.75).contains(&at(0.22).abs()), "broad sweeper");
        assert!(at(0.36).abs() > 0.9, "first short-radius corner");
        assert!(at(0.53).signum() != at(0.64).signum(), "opposing S-bend");
        assert!(at(0.79).abs() > 0.9, "second short-radius corner");
        assert!((at(0.0) - at(1.0)).abs() < 1e-4, "seamless wrap");

        let curved = r.curve.iter().filter(|value| value.abs() >= 0.55).count();
        let sharp = r.curve.iter().filter(|value| value.abs() >= 0.85).count();
        assert!(
            curved >= CURVE_SAMPLES / 2,
            "at least half the loop should be in a readable corner"
        );
        assert!(
            sharp >= CURVE_SAMPLES / 4,
            "tight corners should be frequent"
        );
        let largest_step = r
            .curve
            .iter()
            .zip(r.curve.iter().cycle().skip(1))
            .map(|(left, right)| (right - left).abs())
            .fold(0.0f32, f32::max);
        assert!(largest_step < 0.55, "corner transitions must remain eased");
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
        assert!((20..=30).contains(&r.hazards.len()));
        for h in &r.hazards {
            assert!(h.s >= 0.0 && h.s < r.length);
            match h.kind {
                HazardKind::Oncoming => assert!(h.x.abs() <= 0.9),
                HazardKind::Tree | HazardKind::Billboard => {
                    assert!((1.08..=1.26).contains(&h.x.abs()))
                }
            }
        }
        assert!(r.hazards.iter().any(|h| h.kind == HazardKind::Tree));
        assert!(r.hazards.iter().any(|h| h.kind == HazardKind::Billboard));
    }

    #[test]
    fn some_generated_traffic_changes_lane_then_locks_near_player() {
        let r = loop_from(77);
        let motion = TrafficMotion::default();
        let mut changed = 0;
        for hazard in r
            .hazards
            .iter()
            .filter(|hazard| hazard.kind == HazardKind::Oncoming)
        {
            let far = r.hazard_x_at(hazard, motion.change_start_m + 1.0, motion);
            let mid = r.hazard_x_at(
                hazard,
                (motion.change_start_m + motion.change_end_m) * 0.5,
                motion,
            );
            let near = r.hazard_x_at(hazard, motion.change_end_m - 1.0, motion);
            assert_eq!(far, hazard.x);
            assert_eq!(near, r.hazard_x_at(hazard, 0.0, motion));
            if near != hazard.x {
                changed += 1;
                assert!(mid > hazard.x.min(near) && mid < hazard.x.max(near));
            }
        }
        assert!(changed > 0, "seed should produce occasional lane changes");
        assert!(changed < 8, "lane changes should remain occasional");
    }

    #[test]
    fn elevation_is_periodic_gentle_and_non_flat() {
        let r = loop_from(91);
        assert!((r.elevation_at(0.0) - r.elevation_at(r.length)).abs() < 1e-5);
        assert!(r.max_abs_grade() > 0.01);
        assert!(r.max_abs_grade() < 0.08, "small hills, not mountains");
        let (low, high) = (0..256)
            .map(|i| r.elevation_at(r.length * i as f32 / 256.0))
            .fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(low, high), elevation| (low.min(elevation), high.max(elevation)),
            );
        assert!(high - low > 1.8, "rolling relief should be clearly visible");
        assert!(r.elevation_at(r.length * 0.2) != r.elevation_at(r.length * 0.4));
    }

    #[test]
    fn authored_roads_reject_invalid_geometry() {
        assert!(RoadLoop::try_straight(0.0, Vec::new()).is_err());
        assert!(RoadLoop::try_straight(f32::NAN, Vec::new()).is_err());
        assert!(RoadLoop::try_straight(
            10.0,
            vec![Hazard {
                kind: HazardKind::Oncoming,
                x: 0.0,
                s: 10.0,
            }],
        )
        .is_err());
        assert!(TrafficMotion::default().validate().is_ok());
    }
}
