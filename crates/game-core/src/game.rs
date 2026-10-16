//! The game state machine + fixed-step simulation (PLAN.md M5).
//!
//! Model mirrors the toy (research/toy-mechanics.md): the car is a FIXED
//! decal at screen center-bottom; the printed drum (road loop) scrolls and
//! shifts laterally under it. Steering moves the ROAD; hazards ride the road;
//! crash = hazard under the car or road slipped out from under it (off-road).

use crate::input::{Gear, InputState};
use crate::rng::Rng;
use crate::road::{HazardKind, RoadLoop};
use crate::tuning::Tuning;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Crash,
    Fuel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Ready,
    Running,
    /// crash flash window; transitions to `GameOver(Crash)` when it ends
    Crashed {
        timer: f32,
    },
    GameOver {
        reason: EndReason,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StepEvents {
    pub started: bool,
    /// Toy-ish bump (hazard or off-road): the run continues.
    pub bumped: bool,
    pub fuel_out: bool,
    pub game_over: bool,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub seed: u64,
    pub phase: Phase,
    pub road: RoadLoop,
    /// Current drum speed, m/s (ramps between gear targets).
    pub speed: f32,
    /// Total distance traveled, meters (== the mileage score).
    pub distance: f32,
    /// Road-center lateral offset at the car row, in half-widths. Car is at 0.
    pub road_x: f32,
    /// Fuel remaining, seconds.
    pub fuel: f32,
    /// Crash flash intensity in [0, 1] while > 0.
    pub flash: f32,
    /// Seconds accumulated off the road (bumps after grace).
    pub off_road: f32,
    /// True while the run is paused (Start button / P).
    pub paused: bool,
    /// Completed laps (full drum revolutions since the run started).
    pub laps: u32,
    /// Seconds in the current lap.
    pub lap_time: f32,
    /// Best lap time in seconds (0 = none yet).
    pub lap_best: f32,
    /// Bump slow-down timer + kick direction.
    pub bump_t: f32,
    pub bump_dir: f32,
    gear: Gear,
}

impl Game {
    pub fn new(seed: u64, t: &Tuning) -> Self {
        let mut rng = Rng::new(seed);
        let road = RoadLoop::generate(&mut rng);
        Self::with_road(seed, road, t)
    }

    /// Build a game on an explicit course (tests, attract mode).
    pub fn with_road(seed: u64, road: RoadLoop, t: &Tuning) -> Self {
        Game {
            seed,
            phase: Phase::Ready,
            road,
            speed: 0.0,
            distance: 0.0,
            road_x: 0.0,
            fuel: t.fuel_secs,
            flash: 0.0,
            off_road: 0.0,
            paused: false,
            laps: 0,
            lap_time: 0.0,
            lap_best: 0.0,
            bump_t: 0.0,
            bump_dir: 0.0,
            gear: Gear::Low,
        }
    }

    pub fn gear(&self) -> Gear {
        self.gear
    }

    pub fn loop_pos(&self) -> f32 {
        self.distance.rem_euclid(self.road.length)
    }

    /// Lateral offset of the road row `ahead_m` meters ahead of the car.
    pub fn road_offset_at(&self, ahead_m: f32) -> f32 {
        let drum_shift = self.road_x - self.road.curve_at(self.loop_pos());
        self.road.curve_at(self.loop_pos() + ahead_m) + drum_shift
    }

    pub fn step(&mut self, input: &InputState, dt: f32, t: &Tuning) -> StepEvents {
        let mut ev = StepEvents::default();

        // pause works in every phase except Ready
        if input.pause && self.phase != Phase::Ready {
            self.paused = !self.paused;
        }
        if self.paused {
            if input.reset {
                self.paused = false;
                self.start_run(t);
                ev.started = true;
            }
            return ev;
        }

        match self.phase {
            Phase::Ready => {
                if input.reset || input.gear_up || input.steer != 0.0 {
                    self.start_run(t);
                    ev.started = true;
                }
            }
            Phase::Running => {
                if input.gear_up {
                    self.gear = self.gear.toggle();
                }
                if input.reset {
                    self.start_run(t);
                    ev.started = true;
                    return ev;
                }
                self.step_running(input, dt, t, &mut ev);
            }
            Phase::Crashed { timer } => {
                self.speed = (self.speed - 24.0 * dt).max(0.0);
                self.flash = (self.flash - dt / t.crash_flash_secs).max(0.0);
                let timer = timer - dt;
                if timer <= 0.0 {
                    self.phase = Phase::GameOver {
                        reason: EndReason::Crash,
                    };
                    ev.game_over = true;
                } else {
                    self.phase = Phase::Crashed { timer };
                }
            }
            Phase::GameOver { .. } => {
                self.flash = (self.flash - dt / t.crash_flash_secs).max(0.0);
                if input.reset {
                    self.start_run(t);
                    ev.started = true;
                }
            }
        }
        ev
    }

    fn start_run(&mut self, t: &Tuning) {
        self.phase = Phase::Running;
        self.speed = 0.0;
        self.distance = 0.0;
        self.road_x = 0.0;
        self.fuel = t.fuel_secs;
        self.flash = 0.0;
        self.off_road = 0.0;
        self.laps = 0;
        self.lap_time = 0.0;
        self.bump_t = 0.0;
        self.bump_dir = 0.0;
        // best lap persists across runs
    }

    fn step_running(&mut self, input: &InputState, dt: f32, t: &Tuning, ev: &mut StepEvents) {
        // bump slow-down window
        if self.bump_t > 0.0 {
            self.bump_t = (self.bump_t - dt).max(0.0);
        }
        // speed ramps toward the gear target (exponential smoothing), capped
        // while the bump rings out
        let mut target = match self.gear {
            Gear::Low => t.scroll_speed_low,
            Gear::High => t.scroll_speed_high,
        };
        if self.bump_t > 0.0 {
            target *= 0.35;
        }
        let k = 1.0 - (-dt / t.gear_ramp_secs).exp();
        self.speed += (target - self.speed) * k;

        let prev_pos = self.loop_pos();
        self.distance += self.speed * dt;
        let pos = self.loop_pos();
        let step_m = self.speed * dt;

        // road lateral dynamics: road drifts toward the printed curve target;
        // steering shifts the drum the other way (car "moves" relatively).
        let curve_target = self.road.curve_at(pos) * t.curve_gain;
        let drift = 1.0 - (-dt * t.drift_rate * (self.speed / t.scroll_speed_low)).exp();
        self.road_x += (curve_target - self.road_x) * drift;
        self.road_x -= input.steer * t.steer_rate * dt;
        self.road_x = self.road_x.clamp(-1.35, 1.35);

        // off-road accumulation (grace), then a toy-ish bump
        if self.road_x.abs() > t.off_road_limit {
            self.off_road += dt;
        } else {
            self.off_road = (self.off_road - 2.0 * dt).max(0.0);
        }
        if self.off_road > t.off_road_grace {
            self.bump(ev, t, self.road_x.signum());
            return;
        }

        // hazards crossing the car row this step (swept)
        let hit_band = (t.car_half_width + t.hazard_half_width) * (1.0 - t.collision_forgiveness);
        for h in &self.road.hazards {
            let prev_ahead = (h.s - prev_pos).rem_euclid(self.road.length);
            let ahead = (h.s - pos).rem_euclid(self.road.length);
            let crossed = prev_ahead <= step_m && ahead > prev_ahead;
            if crossed && h.kind == HazardKind::Oncoming {
                let x_eff = h.x + self.road_x;
                if x_eff.abs() < hit_band {
                    self.bump(ev, t, -x_eff.signum());
                    return;
                }
            }
        }

        // lap accounting: loop_pos wrapped => lap complete
        if pos < prev_pos && self.speed > 0.0 {
            if self.lap_time > 0.5 {
                self.lap_best = if self.lap_best <= 0.0 {
                    self.lap_time
                } else {
                    self.lap_best.min(self.lap_time)
                };
            }
            self.laps += 1;
            self.lap_time = 0.0;
        }
        self.lap_time += dt;

        // fuel
        self.fuel -= dt;
        if self.fuel <= 0.0 {
            self.fuel = 0.0;
            self.phase = Phase::GameOver {
                reason: EndReason::Fuel,
            };
            ev.fuel_out = true;
            ev.game_over = true;
        }
    }

    /// Toy-ish bump: kick the drum aside, drop speed, flash, keep running.
    fn bump(&mut self, ev: &mut StepEvents, t: &Tuning, dir: f32) {
        let d = if dir == 0.0 { 1.0 } else { dir };
        self.bump_t = t.crash_flash_secs;
        self.bump_dir = d;
        self.road_x = (self.road_x + d * 0.38).clamp(-1.35, 1.35);
        self.speed *= 0.45;
        self.flash = 0.55;
        ev.bumped = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::road::Hazard;

    const DT: f32 = 1.0 / 60.0;

    fn game(seed: u64) -> (Game, Tuning) {
        let t = Tuning::default();
        (Game::new(seed, &t), t)
    }

    trait Validated {
        fn validated(self) -> Self;
    }
    impl Validated for Tuning {
        fn validated(self) -> Self {
            assert!(self.validate().is_ok());
            self
        }
    }

    fn idle() -> InputState {
        InputState::default()
    }

    fn run_seconds(g: &mut Game, secs: f32, input: &InputState, t: &Tuning) -> StepEvents {
        let mut last = StepEvents::default();
        let steps = (secs / DT).round() as usize;
        for _ in 0..steps {
            last = g.step(input, DT, t);
        }
        last
    }

    #[test]
    fn ready_starts_on_input_then_distance_advances() {
        let (mut g, t) = game(1);
        for _ in 0..10 {
            g.step(&idle(), DT, &t);
        }
        assert_eq!(g.phase, Phase::Ready);
        assert_eq!(g.distance, 0.0);

        let steer = InputState {
            steer: 0.3,
            ..Default::default()
        };
        let ev = g.step(&steer, DT, &t);
        assert!(ev.started);
        assert_eq!(g.phase, Phase::Running);

        run_seconds(&mut g, 3.0, &idle(), &t);
        assert!(g.distance > 5.0, "should cover ground: {}", g.distance);
        assert!(g.speed > 0.0);
    }

    #[test]
    fn speed_ramps_and_high_is_faster() {
        let (mut g, t) = game(2);
        g.step(
            &InputState {
                steer: 1.0,
                ..Default::default()
            },
            DT,
            &t,
        );

        let high = InputState {
            gear: Gear::High,
            gear_up: true,
            ..Default::default()
        };
        g.step(&high, DT, &t);
        let v_high_after_1s = {
            let input = InputState {
                gear: Gear::High,
                ..Default::default()
            };
            run_seconds(&mut g, 1.0, &input, &t);
            g.speed
        };

        let (mut g2, t2) = game(2);
        g2.step(
            &InputState {
                steer: 1.0,
                ..Default::default()
            },
            DT,
            &t2,
        );
        run_seconds(&mut g2, 1.0, &idle(), &t2);
        assert!(v_high_after_1s > g2.speed, "HIGH must out-run LOW");
        assert!(v_high_after_1s <= t.scroll_speed_high * 1.001);
    }

    #[test]
    fn steering_moves_road_opposite_direction() {
        let (mut g, t) = game(3);
        g.step(
            &InputState {
                steer: 1.0,
                ..Default::default()
            },
            DT,
            &t,
        );
        let x0 = g.road_x;
        let right = InputState {
            steer: 1.0,
            ..Default::default()
        };
        run_seconds(&mut g, 0.25, &right, &t);
        assert!(g.road_x < x0, "steer right shifts road left");

        let (mut g, t) = game(3);
        g.step(
            &InputState {
                steer: -1.0,
                ..Default::default()
            },
            DT,
            &t,
        );
        let x0 = g.road_x;
        let left = InputState {
            steer: -1.0,
            ..Default::default()
        };
        run_seconds(&mut g, 0.25, &left, &t);
        assert!(g.road_x > x0, "steer left shifts road right");
    }

    #[test]
    fn off_road_no_steering_eventually_crashes() {
        // find a seed whose loop pushes the road past the limit early
        for seed in 1..40u64 {
            let (mut g, t) = game(seed);
            g.step(
                &InputState {
                    steer: 0.001,
                    ..Default::default()
                },
                DT,
                &t,
            );
            let mut bumped = false;
            for _ in 0..(10.0 / DT) as usize {
                let ev = g.step(&idle(), DT, &t);
                if ev.bumped {
                    assert!(g.flash > 0.0, "flash must be lit at the bump moment");
                    bumped = true;
                    break;
                }
            }
            if bumped {
                // toy-ish bump: run continues, drum keeps rolling
                let d0 = g.distance;
                run_seconds(&mut g, 2.0, &idle(), &t);
                assert!(matches!(g.phase, Phase::Running));
                assert!(g.distance > d0, "drum keeps rolling after a bump");
                return;
            }
        }
        panic!("no seed produced an off-road bump within 10s; drift model too weak?");
    }

    #[test]
    fn sustained_bad_steering_crashes_quickly() {
        let (mut g, t) = game(5);
        g.step(
            &InputState {
                steer: 1.0,
                ..Default::default()
            },
            DT,
            &t,
        );
        let right = InputState {
            steer: 1.0,
            ..Default::default()
        };
        let mut ev = StepEvents::default();
        for _ in 0..(6.0 / DT) as usize {
            ev = g.step(&right, DT, &t);
            if ev.bumped {
                break;
            }
        }
        assert!(ev.bumped, "holding full steer must bump off the road");
        assert!(matches!(g.phase, Phase::Running), "no game-over on bumps");
    }

    #[test]
    fn hazard_dead_ahead_crashes() {
        let t = Tuning::default();
        let hazard = Hazard {
            kind: HazardKind::Oncoming,
            x: 0.0,
            s: 20.0,
        };
        let mut g = Game::with_road(1, RoadLoop::straight(60.0, vec![hazard]), &t);
        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        let mut bumped = false;
        for _ in 0..(6.0 / DT) as usize {
            let ev = g.step(&idle(), DT, &t);
            if ev.bumped {
                bumped = true;
                break;
            }
        }
        assert!(bumped, "a dead-ahead oncoming hazard must bump");
    }

    #[test]
    fn hazard_in_side_lane_is_a_near_miss() {
        let t = Tuning::default();
        let hazard = Hazard {
            kind: HazardKind::Oncoming,
            x: 0.62,
            s: 20.0,
        };
        let mut g = Game::with_road(1, RoadLoop::straight(60.0, vec![hazard]), &t);
        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        for _ in 0..(6.0 / DT) as usize {
            let ev = g.step(&idle(), DT, &t);
            assert!(!ev.bumped, "side-lane hazard with no steering must miss");
        }
    }

    #[test]
    fn countersteering_holds_the_road() {
        // road-hold check on a hazard-free course: steering must beat drift
        let t = Tuning::default();
        let mut road = RoadLoop::generate(&mut Rng::new(11));
        road.hazards.clear();
        let mut g = Game::with_road(11, road, &t);
        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        for _ in 0..(60.0 / DT) as usize {
            // road_x > 0 = road sits right of the car: steer right (+) to pull it back
            let steer = (g.road_x * 4.0).clamp(-1.0, 1.0);
            let input = InputState {
                steer,
                ..Default::default()
            };
            let ev = g.step(&input, DT, &t);
            assert!(
                !ev.bumped,
                "autopilot should hold the road (road_x {})",
                g.road_x
            );
        }
        assert!(g.distance > 100.0);
    }

    #[test]
    fn fuel_runs_out_and_freezes_score() {
        let t = Tuning {
            fuel_secs: 2.0,
            ..Default::default()
        };
        let mut g = Game::with_road(6, RoadLoop::straight(60.0, vec![]), &t);
        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        let mut ev = StepEvents::default();
        for _ in 0..(5.0 / DT) as usize {
            ev = g.step(&idle(), DT, &t);
            if ev.game_over {
                break;
            }
        }
        assert!(ev.fuel_out);
        assert_eq!(g.fuel, 0.0);
        assert!(matches!(
            g.phase,
            Phase::GameOver {
                reason: EndReason::Fuel
            }
        ));
        let d = g.distance;
        run_seconds(&mut g, 2.0, &idle(), &t);
        assert_eq!(g.distance, d);
    }

    #[test]
    fn reset_restarts_with_same_course() {
        let (mut g, t) = game(8);
        g.step(
            &InputState {
                steer: 1.0,
                ..Default::default()
            },
            DT,
            &t,
        );
        let right = InputState {
            steer: 1.0,
            ..Default::default()
        };
        for _ in 0..(8.0 / DT) as usize {
            let _ = g.step(&right, DT, &t);
        }
        run_seconds(&mut g, 2.0, &idle(), &t);
        assert!(matches!(g.phase, Phase::Running));

        let ev = g.step(
            &InputState {
                reset: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert!(ev.started);
        assert_eq!(g.phase, Phase::Running);
        assert_eq!(g.distance, 0.0);
        assert_eq!(g.fuel, t.fuel_secs);
        // same seed => same course
        let (g2, _) = game(8);
        assert_eq!(g.road.hazards, g2.road.hazards);
    }

    #[test]
    fn gear_up_in_running_toggles() {
        let (mut g, t) = game(9);
        g.step(
            &InputState {
                steer: 1.0,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert_eq!(g.gear(), Gear::Low);
        g.step(
            &InputState {
                gear_up: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert_eq!(g.gear(), Gear::High);
    }

    #[test]
    fn pause_freezes_and_resume_continues() {
        let (mut g, t) = game(21);
        g.step(
            &InputState {
                steer: 1.0,
                ..Default::default()
            },
            DT,
            &t,
        );
        run_seconds(&mut g, 1.0, &idle(), &t);
        let d_before = g.distance;
        g.step(
            &InputState {
                pause: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert!(g.paused);
        run_seconds(&mut g, 2.0, &idle(), &t);
        assert_eq!(g.distance, d_before, "paused: drum frozen");
        g.step(
            &InputState {
                pause: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert!(!g.paused);
        run_seconds(&mut g, 0.5, &idle(), &t);
        assert!(g.distance > d_before, "resumed: drum rolling");
    }

    #[test]
    fn pause_then_restart_fresh_run() {
        let (mut g, t) = game(22);
        g.step(
            &InputState {
                steer: 1.0,
                ..Default::default()
            },
            DT,
            &t,
        );
        run_seconds(&mut g, 1.0, &idle(), &t);
        g.step(
            &InputState {
                pause: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        let ev = g.step(
            &InputState {
                reset: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert!(ev.started);
        assert!(!g.paused);
        assert_eq!(g.distance, 0.0);
    }

    #[test]
    fn laps_count_and_best_time_tracks() {
        let t = Tuning {
            fuel_secs: 600.0,
            scroll_speed_low: 30.0, // fast: wrap the ~60 m loop quickly
            scroll_speed_high: 31.0,
            ..Default::default()
        }
        .validated();
        let road = RoadLoop::straight(60.0, vec![]);
        let mut g = Game::with_road(23, road, &t);
        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        // run ~4.5 loops
        for _ in 0..(9.0 / DT) as usize {
            let _ = g.step(&idle(), DT, &t);
        }
        assert!(g.laps >= 4, "laps: {}", g.laps);
        assert!(g.lap_best > 0.0);
        let prev_best = g.lap_best;
        for _ in 0..(2.0 / DT) as usize {
            let _ = g.step(&idle(), DT, &t);
        }
        assert!(g.lap_best <= prev_best || g.laps == 5);
    }

    #[test]
    fn road_offset_at_is_continuous_ahead() {
        let (mut g, t) = game(10);
        g.step(
            &InputState {
                steer: 1.0,
                ..Default::default()
            },
            DT,
            &t,
        );
        run_seconds(&mut g, 4.0, &idle(), &t);
        let a = g.road_offset_at(0.0);
        let b = g.road_offset_at(0.5);
        assert!((a - b).abs() < 0.5, "half a meter ahead should be close");
    }
}
