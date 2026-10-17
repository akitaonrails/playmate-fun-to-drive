//! The game state machine + fixed-step simulation (PLAN.md M5).
//!
//! Model mirrors the toy (research/toy-mechanics.md): the car is a FIXED
//! decal at screen center-bottom; the printed drum (road loop) scrolls and
//! shifts laterally under it. Steering moves the ROAD; hazards ride the road;
//! bump = an oncoming hazard under the car or the road slipping out from
//! under it (off-road). Bumps slow the toy but do not end the timed session.

use crate::environment::{WeatherSchedule, WeatherTransition, DAY_SECS};
use crate::input::{Gear, InputState};
use crate::rng::Rng;
use crate::road::{HazardKind, RoadLoop};
use crate::tuning::Tuning;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Time,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Ready,
    Running,
    GameOver { reason: EndReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Difficulty {
    Easy,
    Medium,
    #[default]
    Hard,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Medium, Self::Hard];

    pub const fn duration_secs(self) -> f32 {
        match self {
            Self::Easy => 120.0,
            Self::Medium => 360.0,
            Self::Hard => 600.0,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Easy => "EASY",
            Self::Medium => "MEDIUM",
            Self::Hard => "HARD",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Recovery {
    #[default]
    Normal,
    NeedLow,
    BuildSpeed,
    ReadyForHigh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownshiftSeverity {
    Normal,
    Unsafe,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StepEvents {
    pub started: bool,
    /// Toy-ish bump (hazard or off-road): the run continues.
    pub bumped: bool,
    pub time_up: bool,
    pub game_over: bool,
    pub nitro_collected: bool,
    pub nitro_started: bool,
    pub high_rejected: bool,
    pub downshifted: bool,
    pub downshift_severity: Option<DownshiftSeverity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionResult {
    pub difficulty: Difficulty,
    pub day_cycles: u64,
    pub cars_passed: u64,
    pub cars_bumped: u64,
    pub score: u64,
}

impl SessionResult {
    pub fn new(
        difficulty: Difficulty,
        day_cycles: u64,
        cars_passed: u64,
        cars_bumped: u64,
    ) -> Self {
        Self {
            difficulty,
            day_cycles,
            cars_passed,
            cars_bumped,
            score: score_for(day_cycles, cars_passed, cars_bumped),
        }
    }
}

/// Cycles dominate the ranking, passes distinguish similarly paced runs, and
/// one collision cancels three clean passes. Wide signed arithmetic prevents
/// either overflow or unsigned penalty underflow.
pub fn score_for(day_cycles: u64, cars_passed: u64, cars_bumped: u64) -> u64 {
    let positive = i128::from(day_cycles)
        .saturating_mul(10_000)
        .saturating_add(i128::from(cars_passed).saturating_mul(100));
    let score = positive.saturating_sub(i128::from(cars_bumped).saturating_mul(300));
    score.clamp(0, i128::from(u64::MAX)) as u64
}

pub const LEADERBOARD_CAPACITY_PER_DIFFICULTY: usize = 10;
pub const LEADERBOARD_CAPACITY: usize = LEADERBOARD_CAPACITY_PER_DIFFICULTY * 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderboardEntry {
    pub initials: [u8; 3],
    pub result: SessionResult,
}

impl LeaderboardEntry {
    pub fn new(
        initials: &str,
        difficulty: Difficulty,
        day_cycles: u64,
        cars_passed: u64,
        cars_bumped: u64,
    ) -> Option<Self> {
        let bytes = initials.as_bytes();
        if bytes.len() != 3 || !bytes.iter().all(u8::is_ascii_uppercase) {
            return None;
        }
        Some(Self {
            initials: [bytes[0], bytes[1], bytes[2]],
            result: SessionResult::new(difficulty, day_cycles, cars_passed, cars_bumped),
        })
    }

    pub fn initials_str(&self) -> &str {
        // Construction guarantees uppercase ASCII.
        std::str::from_utf8(&self.initials).expect("validated ASCII initials")
    }
}

pub fn rank_leaderboard(entries: &mut Vec<LeaderboardEntry>) {
    entries.sort_by(|a, b| {
        a.result.difficulty.cmp(&b.result.difficulty).then_with(|| {
            b.result
                .score
                .cmp(&a.result.score)
                .then_with(|| b.result.day_cycles.cmp(&a.result.day_cycles))
                .then_with(|| b.result.cars_passed.cmp(&a.result.cars_passed))
                .then_with(|| a.result.cars_bumped.cmp(&b.result.cars_bumped))
        })
    });
    let mut counts = [0usize; 3];
    entries.retain(|entry| {
        let index = entry.result.difficulty as usize;
        counts[index] += 1;
        counts[index] <= LEADERBOARD_CAPACITY_PER_DIFFICULTY
    });
}

pub fn leaderboard_for(
    entries: &[LeaderboardEntry],
    difficulty: Difficulty,
) -> impl Iterator<Item = &LeaderboardEntry> {
    entries
        .iter()
        .filter(move |entry| entry.result.difficulty == difficulty)
}

#[derive(Debug, Clone)]
pub struct Game {
    pub seed: u64,
    pub phase: Phase,
    pub road: RoadLoop,
    /// Current drum speed, m/s (ramps between gear targets).
    pub speed: f32,
    /// Total distance traveled in meters; score is derived from run statistics.
    pub distance: f32,
    /// Road-center lateral offset at the car row, in half-widths. Car is at 0.
    pub road_x: f32,
    /// Authoritative competitive clock. Only advances while actively running.
    pub session_remaining: f32,
    /// Performance-driven environment clock; LOW speed advances at real time.
    pub environment_time: f32,
    pub day_cycles: u64,
    pub cars_passed: u64,
    pub cars_bumped: u64,
    /// Crash flash intensity in [0, 1] while > 0.
    pub flash: f32,
    /// Seconds accumulated off the road (bumps after grace).
    pub off_road: f32,
    /// True while the run is paused (Start button / P).
    pub paused: bool,
    /// Bump slow-down timer + kick direction.
    pub bump_t: f32,
    pub bump_dir: f32,
    /// Stored pickups (the active charge, if any, lives in `nitro_t`).
    pub nitro_charges: u8,
    /// Seconds remaining in the active boost.
    pub nitro_t: f32,
    /// Seconds before another charge may be activated.
    pub nitro_cooldown: f32,
    /// Absolute course distance of the next pickup.
    pub next_nitro_distance: f32,
    /// Lane of the next pickup, relative to road center.
    pub next_nitro_lane: f32,
    nitro_pickup_index: u32,
    gear: Gear,
    recovery: Recovery,
    difficulty: Difficulty,
    session_duration: f32,
    weather: WeatherSchedule,
    result: Option<SessionResult>,
}

impl Game {
    pub fn new(seed: u64, difficulty: Difficulty, t: &Tuning) -> Self {
        let mut rng = Rng::new(seed);
        let road = RoadLoop::generate(&mut rng);
        Self::with_road(seed, road, difficulty, t)
    }

    /// Build a game on an explicit course (tests, attract mode).
    pub fn with_road(seed: u64, road: RoadLoop, difficulty: Difficulty, t: &Tuning) -> Self {
        Self::with_road_for_duration(seed, road, difficulty, difficulty.duration_secs(), t)
    }

    /// Build a run with an explicit duration for short-lived QA sessions.
    /// Difficulty remains part of the result/ranking contract.
    pub fn with_road_for_duration(
        seed: u64,
        road: RoadLoop,
        difficulty: Difficulty,
        duration_secs: f32,
        t: &Tuning,
    ) -> Self {
        assert!(duration_secs.is_finite() && duration_secs > 0.0);
        Game {
            seed,
            phase: Phase::Ready,
            road,
            speed: 0.0,
            distance: 0.0,
            road_x: 0.0,
            session_remaining: duration_secs,
            environment_time: 0.0,
            day_cycles: 0,
            cars_passed: 0,
            cars_bumped: 0,
            flash: 0.0,
            off_road: 0.0,
            paused: false,
            bump_t: 0.0,
            bump_dir: 0.0,
            nitro_charges: 0,
            nitro_t: 0.0,
            nitro_cooldown: 0.0,
            next_nitro_distance: t.nitro_pickup_first_m,
            next_nitro_lane: Self::nitro_lane(seed, 0),
            nitro_pickup_index: 0,
            gear: Gear::Low,
            recovery: Recovery::Normal,
            difficulty,
            session_duration: duration_secs,
            weather: WeatherSchedule::new(seed),
            result: None,
        }
    }

    pub fn gear(&self) -> Gear {
        self.gear
    }

    pub fn recovery(&self) -> Recovery {
        self.recovery
    }

    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    pub fn session_duration(&self) -> f32 {
        self.session_duration
    }

    /// Normalized LOW-gear over-rev warning in [0, 1].
    pub fn low_overrev(&self, t: &Tuning) -> f32 {
        if self.gear != Gear::Low {
            return 0.0;
        }
        ((self.speed - t.low_overrev_start) / (t.scroll_speed_low - t.low_overrev_start))
            .clamp(0.0, 1.0)
    }

    pub fn loop_pos(&self) -> f32 {
        self.distance.rem_euclid(self.road.length)
    }

    pub fn nitro_active(&self) -> bool {
        self.nitro_t > 0.0
    }

    pub fn nitro_pickup_ahead(&self) -> f32 {
        self.next_nitro_distance - self.distance
    }

    pub fn weather(&self) -> WeatherTransition {
        self.weather.at(self.environment_time)
    }

    pub fn result(&self) -> Option<&SessionResult> {
        self.result.as_ref()
    }

    fn nitro_lane(seed: u64, index: u32) -> f32 {
        const LANES: [f32; 3] = [-0.62, 0.0, 0.62];
        let mixed = seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        LANES[((mixed ^ (mixed >> 32)) % LANES.len() as u64) as usize]
    }

    fn advance_nitro_pickup(&mut self, t: &Tuning) {
        self.nitro_pickup_index += 1;
        self.next_nitro_distance += t.nitro_pickup_interval_m;
        self.next_nitro_lane = Self::nitro_lane(self.seed, self.nitro_pickup_index);
    }

    /// Lateral offset of the road row `ahead_m` meters ahead of the car.
    pub fn road_offset_at(&self, ahead_m: f32) -> f32 {
        let drum_shift = self.road_x - self.road.curve_at(self.loop_pos());
        self.road.curve_at(self.loop_pos() + ahead_m) + drum_shift
    }

    pub fn step(&mut self, input: &InputState, dt: f32, t: &Tuning) -> StepEvents {
        let mut ev = StepEvents::default();
        let dt = dt.max(0.0);

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
        // Running systems all share the same capped delta on the terminal
        // tick. Other phases may use the full wall-clock delta safely.
        let active_dt = if self.phase == Phase::Running {
            dt.min(self.session_remaining)
        } else {
            dt
        };
        self.flash = (self.flash - active_dt / t.crash_flash_secs).max(0.0);

        match self.phase {
            Phase::Ready => {
                if input.reset || input.gear_request.is_some() || input.steer != 0.0 {
                    self.start_run(t);
                    ev.started = true;
                    if let Some(request) = input.gear_request {
                        self.select_gear(request, t, &mut ev);
                    }
                }
            }
            Phase::Running => {
                if input.reset {
                    self.start_run(t);
                    ev.started = true;
                    return ev;
                }
                self.update_recovery_readiness(t);
                if let Some(request) = input.gear_request {
                    self.select_gear(request, t, &mut ev);
                }
                self.step_running(input, active_dt, t, &mut ev);
            }
            Phase::GameOver { .. } => {
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
        self.session_remaining = self.session_duration;
        self.environment_time = 0.0;
        self.day_cycles = 0;
        self.cars_passed = 0;
        self.cars_bumped = 0;
        self.flash = 0.0;
        self.off_road = 0.0;
        self.bump_t = 0.0;
        self.bump_dir = 0.0;
        self.nitro_charges = 0;
        self.nitro_t = 0.0;
        self.nitro_cooldown = 0.0;
        self.nitro_pickup_index = 0;
        self.next_nitro_distance = t.nitro_pickup_first_m;
        self.next_nitro_lane = Self::nitro_lane(self.seed, 0);
        self.result = None;
        self.gear = Gear::Low;
        self.recovery = Recovery::Normal;
    }

    /// Begin a fresh standard-duration run at the selected difficulty.
    pub fn restart_with_difficulty(&mut self, difficulty: Difficulty, t: &Tuning) {
        self.difficulty = difficulty;
        self.session_duration = difficulty.duration_secs();
        self.start_run(t);
    }

    fn update_recovery_readiness(&mut self, t: &Tuning) {
        if self.recovery == Recovery::BuildSpeed && self.speed >= t.shift_up_min_speed {
            self.recovery = Recovery::ReadyForHigh;
        }
    }

    fn select_gear(&mut self, request: Gear, t: &Tuning, ev: &mut StepEvents) {
        if request == self.gear {
            return;
        }
        match request {
            Gear::High => {
                let recovery_allows =
                    matches!(self.recovery, Recovery::Normal | Recovery::ReadyForHigh);
                if self.speed < t.shift_up_min_speed || !recovery_allows {
                    ev.high_rejected = true;
                    return;
                }
                self.gear = Gear::High;
                if self.recovery == Recovery::ReadyForHigh {
                    self.recovery = Recovery::Normal;
                }
            }
            Gear::Low => {
                let severity = if self.speed <= t.downshift_safe_speed {
                    self.speed *= t.downshift_safe_retention;
                    DownshiftSeverity::Normal
                } else {
                    self.speed *= t.downshift_unsafe_retention;
                    DownshiftSeverity::Unsafe
                };
                self.gear = Gear::Low;
                if self.recovery == Recovery::NeedLow {
                    self.recovery = Recovery::BuildSpeed;
                }
                ev.downshifted = true;
                ev.downshift_severity = Some(severity);
            }
        }
    }

    fn step_running(&mut self, input: &InputState, dt: f32, t: &Tuning, ev: &mut StepEvents) {
        let dt = dt.max(0.0).min(self.session_remaining);
        if dt <= 0.0 {
            if self.session_remaining <= 0.0 {
                self.finish_session(ev);
            }
            return;
        }
        if input.nitro
            && self.recovery == Recovery::Normal
            && self.nitro_t <= 0.0
            && self.nitro_cooldown <= 0.0
            && self.nitro_charges > 0
        {
            self.nitro_charges -= 1;
            self.nitro_t = t.nitro_duration_secs;
            ev.nitro_started = true;
        }

        // Timer boundaries split one outer fixed step into constant-target
        // intervals. The active boost therefore applies for its complete five
        // seconds, and any time remaining after expiry ages the cooldown.
        let mut remaining = dt;
        let mut car_collision = false;
        let mut car_bump_dir = 1.0;
        let mut off_road_bump = false;
        while remaining > 0.0 {
            let nitro_active = self.nitro_t > 0.0;
            let bump_active = self.bump_t > 0.0;
            let mut interval = remaining;
            if nitro_active {
                interval = interval.min(self.nitro_t);
            }
            if bump_active {
                interval = interval.min(self.bump_t);
            }

            self.integrate_running_interval(
                input,
                interval,
                t,
                ev,
                nitro_active,
                bump_active,
                &mut car_collision,
                &mut car_bump_dir,
                &mut off_road_bump,
            );

            if nitro_active {
                self.nitro_t = (self.nitro_t - interval).max(0.0);
                if self.nitro_t <= f32::EPSILON {
                    self.nitro_t = 0.0;
                    self.nitro_cooldown = t.nitro_cooldown_secs;
                }
            } else if self.nitro_cooldown > 0.0 {
                self.nitro_cooldown = (self.nitro_cooldown - interval).max(0.0);
            }
            if bump_active {
                self.bump_t = (self.bump_t - interval).max(0.0);
                if self.bump_t <= f32::EPSILON {
                    self.bump_t = 0.0;
                }
            }
            remaining = (remaining - interval).max(0.0);
        }

        // Preserve the outer fixed-step contract: all encounters are counted,
        // but collectively cause at most one impulse, with car-over-off-road
        // precedence even when a timer boundary split the integration.
        if car_collision {
            self.bump(ev, t, car_bump_dir);
        } else if off_road_bump {
            self.bump(ev, t, self.road_x.signum());
        }

        self.session_remaining = (self.session_remaining - dt).max(0.0);
        if self.session_remaining <= 0.0 {
            self.finish_session(ev);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn integrate_running_interval(
        &mut self,
        input: &InputState,
        dt: f32,
        t: &Tuning,
        ev: &mut StepEvents,
        nitro_active: bool,
        bump_active: bool,
        car_collision: &mut bool,
        car_bump_dir: &mut f32,
        off_road_bump: &mut bool,
    ) {
        let mut target = match self.recovery {
            Recovery::NeedLow => t.recovery_bog_speed,
            _ => match self.gear {
                Gear::Low => t.scroll_speed_low,
                Gear::High => t.scroll_speed_high,
            },
        };
        if bump_active && self.recovery != Recovery::NeedLow {
            target *= 0.35;
        }
        if nitro_active && self.recovery == Recovery::Normal {
            target *= t.nitro_speed_multiplier;
        }
        target *= off_road_speed_factor(self.road_x, t);
        let initial_speed = self.speed;
        let decay = (-dt / t.gear_ramp_secs).exp();
        self.speed = target + (initial_speed - target) * decay;
        self.update_recovery_readiness(t);

        let previous_distance = self.distance;
        // Exact integral of the exponential speed ramp for a constant target.
        // This makes distance identical whether an interval is stepped whole
        // or partitioned by callers.
        let distance_delta =
            target * dt + (initial_speed - target) * t.gear_ramp_secs * (1.0 - decay);
        self.distance += distance_delta;
        self.environment_time += distance_delta / t.scroll_speed_low;
        self.day_cycles = (self.environment_time / DAY_SECS).floor().max(0.0) as u64;
        let pos = self.loop_pos();

        // road lateral dynamics: road drifts toward the printed curve target;
        // steering shifts the drum the other way (car "moves" relatively).
        let curve_target = self.road.curve_at(pos) * t.curve_gain;
        let drift = 1.0 - (-dt * t.drift_rate * (self.speed / t.scroll_speed_low)).exp();
        self.road_x += (curve_target - self.road_x) * drift;
        self.road_x -= input.steer * t.steer_rate * dt;
        self.road_x = self.road_x.clamp(-1.35, 1.35);

        // One deterministic pickup at a time. The absolute-distance schedule
        // keeps appearances occasional and guarantees more than four per
        // 100-second light cycle at LOW speed.
        while self.distance >= self.next_nitro_distance {
            let capacity_used = self.nitro_charges + u8::from(self.nitro_t > 0.0);
            let x_eff = self.next_nitro_lane + self.road_x;
            let pickup_band = t.car_half_width + 0.20;
            if x_eff.abs() < pickup_band && capacity_used < 3 {
                self.nitro_charges += 1;
                ev.nitro_collected = true;
            }
            self.advance_nitro_pickup(t);
        }

        // Accumulate off-road time, but defer its impulse until after all
        // traffic crossings have been classified. A car collision wins when
        // both conditions occur in the same tick.
        if self.road_x.abs() > t.off_road_limit {
            self.off_road += dt;
        } else {
            self.off_road = (self.off_road - 2.0 * dt).max(0.0);
        }
        *off_road_bump |= self.off_road > t.off_road_grace;

        // Fixed-step traffic contract: account every oncoming crossing in
        // (previous_distance, distance], including repeated-loop encounters.
        // Crossings use the resolved road offset for their timer-bounded
        // interval.
        // Passed and bumped are mutually exclusive per encounter; roadside
        // scenery never affects either counter, and collisions collectively
        // produce at most one physical impulse below.
        let hit_band = (t.car_half_width + t.hazard_half_width) * (1.0 - t.collision_forgiveness);
        for h in &self.road.hazards {
            if h.kind == HazardKind::Oncoming {
                let crossings =
                    crossings_between(previous_distance, self.distance, h.s, self.road.length);
                if crossings == 0 {
                    continue;
                }
                let x_eff = h.x + self.road_x;
                if x_eff.abs() < hit_band {
                    self.cars_bumped = self.cars_bumped.saturating_add(crossings);
                    *car_collision = true;
                    *car_bump_dir = -x_eff.signum();
                } else {
                    self.cars_passed = self.cars_passed.saturating_add(crossings);
                }
            }
        }
    }

    fn finish_session(&mut self, ev: &mut StepEvents) {
        if self.result.is_some() {
            return;
        }
        self.session_remaining = 0.0;
        self.phase = Phase::GameOver {
            reason: EndReason::Time,
        };
        self.result = Some(SessionResult::new(
            self.difficulty,
            self.day_cycles,
            self.cars_passed,
            self.cars_bumped,
        ));
        ev.time_up = true;
        ev.game_over = true;
    }

    /// Toy-ish bump: kick the drum aside, drop speed, flash, keep running.
    fn bump(&mut self, ev: &mut StepEvents, t: &Tuning, dir: f32) {
        let d = if dir == 0.0 { 1.0 } else { dir };
        self.bump_t = t.crash_flash_secs;
        self.bump_dir = d;
        self.off_road = 0.0;
        self.road_x = (self.road_x + d * 0.38).clamp(-1.35, 1.35);
        self.speed *= 0.45;
        self.recovery = match self.gear {
            Gear::High => Recovery::NeedLow,
            Gear::Low => Recovery::BuildSpeed,
        };
        self.flash = 0.35;
        ev.bumped = true;
    }
}

fn crossings_between(start: f32, end: f32, hazard_s: f32, loop_length: f32) -> u64 {
    if end <= start || loop_length <= 0.0 {
        return 0;
    }
    let first = ((start - hazard_s) / loop_length).floor() as i64 + 1;
    let last = ((end - hazard_s) / loop_length).floor() as i64;
    last.saturating_sub(first).saturating_add(1).max(0) as u64
}

fn off_road_speed_factor(road_x: f32, t: &Tuning) -> f32 {
    const MAX_EXCURSION: f32 = 1.35;
    let excursion =
        ((road_x.abs() - t.off_road_limit) / (MAX_EXCURSION - t.off_road_limit)).clamp(0.0, 1.0);
    let smooth = excursion * excursion * (3.0 - 2.0 * excursion);
    1.0 + (t.off_road_min_speed_factor - 1.0) * smooth
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::road::Hazard;

    const DT: f32 = 1.0 / 60.0;

    fn game(seed: u64) -> (Game, Tuning) {
        let t = Tuning::default();
        (Game::new(seed, Difficulty::Hard, &t), t)
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
        run_seconds(&mut g, 1.2, &idle(), &t);

        let high = InputState {
            gear_request: Some(Gear::High),
            ..Default::default()
        };
        g.step(&high, DT, &t);
        let v_high_after_1s = {
            let input = InputState {
                gear_request: None,
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
    fn bump_resets_off_road_grace_so_recovery_is_possible() {
        let t = Tuning::default();
        let mut g = Game::with_road(40, RoadLoop::straight(500.0, vec![]), Difficulty::Hard, &t);
        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        g.road_x = 1.2;
        let mut bumped = false;
        for _ in 0..120 {
            if g.step(&idle(), DT, &t).bumped {
                bumped = true;
                break;
            }
        }
        assert!(bumped);
        assert_eq!(g.off_road, 0.0);
        assert!(g.bump_t > 0.0);
    }

    #[test]
    fn hazard_dead_ahead_crashes() {
        let t = Tuning::default();
        let hazard = Hazard {
            kind: HazardKind::Oncoming,
            x: 0.0,
            s: 20.0,
        };
        let mut g = Game::with_road(
            1,
            RoadLoop::straight(60.0, vec![hazard]),
            Difficulty::Hard,
            &t,
        );
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
        let mut g = Game::with_road(
            1,
            RoadLoop::straight(60.0, vec![hazard]),
            Difficulty::Hard,
            &t,
        );
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
        let mut g = Game::with_road(11, road, Difficulty::Hard, &t);
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
    fn session_clock_ends_once_and_freezes_result() {
        let t = Tuning::default();
        let mut g = Game::with_road_for_duration(
            6,
            RoadLoop::straight(60.0, vec![]),
            Difficulty::Hard,
            2.0,
            &t,
        );
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
        assert!(ev.time_up);
        assert_eq!(g.session_remaining, 0.0);
        assert!(matches!(
            g.phase,
            Phase::GameOver {
                reason: EndReason::Time
            }
        ));
        let d = g.distance;
        let result = *g.result().unwrap();
        run_seconds(&mut g, 2.0, &idle(), &t);
        assert_eq!(g.distance, d);
        assert_eq!(g.result(), Some(&result));
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
        assert_eq!(g.session_remaining, g.session_duration());
        assert_eq!(g.environment_time, 0.0);
        assert_eq!(g.result(), None);
        // same seed => same course
        let (g2, _) = game(8);
        assert_eq!(g.road.hazards, g2.road.hazards);
    }

    #[test]
    fn explicit_high_request_selects_high_once_ready() {
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
        run_seconds(&mut g, 1.2, &idle(), &t);
        g.step(
            &InputState {
                gear_request: Some(Gear::High),
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
    fn faster_driving_completes_more_environment_cycles() {
        let t = Tuning::default().validated();
        let road = RoadLoop::straight(10_000.0, vec![]);
        let mut low = Game::with_road_for_duration(23, road.clone(), Difficulty::Hard, 200.0, &t);
        let mut high = Game::with_road_for_duration(23, road, Difficulty::Hard, 200.0, &t);
        low.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        high.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        run_seconds(&mut high, 1.2, &idle(), &t);
        high.step(
            &InputState {
                gear_request: Some(Gear::High),
                ..Default::default()
            },
            DT,
            &t,
        );
        high.step(
            &InputState {
                gear_request: Some(Gear::High),
                ..Default::default()
            },
            DT,
            &t,
        );
        run_seconds(&mut low, 150.0, &idle(), &t);
        run_seconds(&mut high, 150.0, &idle(), &t);
        assert!(high.environment_time > low.environment_time);
        assert!(high.day_cycles > low.day_cycles);
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

    #[test]
    fn nitro_pickup_collects_in_lane_and_caps_at_three() {
        let t = Tuning::default();
        let road = RoadLoop::straight(1000.0, vec![]);
        let mut g = Game::with_road(31, road, Difficulty::Hard, &t);
        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );

        for expected in 1..=3 {
            g.next_nitro_distance = g.distance;
            g.road_x = -g.next_nitro_lane;
            let ev = g.step(&idle(), DT, &t);
            assert!(ev.nitro_collected);
            assert_eq!(g.nitro_charges, expected);
        }

        g.next_nitro_distance = g.distance;
        g.road_x = -g.next_nitro_lane;
        let ev = g.step(&idle(), DT, &t);
        assert!(!ev.nitro_collected, "a full meter cannot overfill");
        assert_eq!(g.nitro_charges, 3);
    }

    #[test]
    fn missed_nitro_pickup_advances_to_the_next_spawn() {
        let t = Tuning::default();
        let road = RoadLoop::straight(1000.0, vec![]);
        let mut g = Game::with_road(32, road, Difficulty::Hard, &t);
        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        let old_distance = g.distance;
        g.next_nitro_distance = old_distance;
        g.next_nitro_lane = 0.62;
        g.road_x = -0.62;
        // Deliberately move the road away from the pickup lane.
        g.road_x = 0.0;
        let ev = g.step(&idle(), DT, &t);
        assert!(!ev.nitro_collected);
        assert_eq!(g.nitro_charges, 0);
        assert!((g.next_nitro_distance - (old_distance + t.nitro_pickup_interval_m)).abs() < 1e-3);
    }

    #[test]
    fn nitro_spends_one_charge_boosts_and_expires_in_five_seconds() {
        let t = Tuning::default();
        let road = RoadLoop::straight(2000.0, vec![]);
        let mut boosted = Game::with_road(33, road.clone(), Difficulty::Hard, &t);
        let mut normal = Game::with_road(33, road, Difficulty::Hard, &t);
        for g in [&mut boosted, &mut normal] {
            g.step(
                &InputState {
                    steer: 0.001,
                    ..Default::default()
                },
                DT,
                &t,
            );
            run_seconds(g, 2.0, &idle(), &t);
        }
        boosted.nitro_charges = 2;
        let ev = boosted.step(
            &InputState {
                nitro: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert!(ev.nitro_started);
        assert_eq!(boosted.nitro_charges, 1);
        assert!(boosted.nitro_active());

        run_seconds(&mut boosted, 1.0, &idle(), &t);
        run_seconds(&mut normal, 1.0, &idle(), &t);
        assert!(
            boosted.speed > normal.speed * 1.15,
            "boost speed {} vs {}",
            boosted.speed,
            normal.speed
        );

        run_seconds(&mut boosted, 4.1, &idle(), &t);
        assert!(!boosted.nitro_active());
        assert_eq!(boosted.nitro_t, 0.0);
    }

    #[test]
    fn restart_clears_nitro_and_restores_first_pickup() {
        let (mut g, t) = game(34);
        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        g.nitro_charges = 3;
        g.nitro_t = 2.0;
        g.next_nitro_distance = 999.0;
        g.step(
            &InputState {
                reset: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert_eq!(g.nitro_charges, 0);
        assert_eq!(g.nitro_t, 0.0);
        assert_eq!(g.next_nitro_distance, t.nitro_pickup_first_m);
    }

    #[test]
    fn at_least_four_nitro_pickups_appear_per_light_cycle_at_low_speed() {
        let t = Tuning::default();
        let mut g = Game::with_road(35, RoadLoop::straight(3000.0, vec![]), Difficulty::Hard, &t);
        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        let mut collected = 0;
        for _ in 0..(100.0 / DT) as usize {
            g.road_x = -g.next_nitro_lane;
            if g.step(&idle(), DT, &t).nitro_collected {
                collected += 1;
                // Keep capacity open so this test counts every appearance.
                g.nitro_charges = 0;
            }
        }
        assert!(
            collected >= 4,
            "only {collected} pickups in one 100 s light cycle"
        );
    }

    #[test]
    fn ready_pause_and_reset_freeze_or_restore_all_session_clocks() {
        let (mut g, t) = game(50);
        g.step(&idle(), 5.0, &t);
        assert_eq!(g.session_remaining, g.session_duration());
        assert_eq!(g.environment_time, 0.0);

        g.step(
            &InputState {
                steer: 0.001,
                ..Default::default()
            },
            DT,
            &t,
        );
        run_seconds(&mut g, 1.0, &idle(), &t);
        let remaining = g.session_remaining;
        let environment_time = g.environment_time;
        g.step(
            &InputState {
                pause: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        run_seconds(&mut g, 5.0, &idle(), &t);
        assert_eq!(g.session_remaining, remaining);
        assert_eq!(g.environment_time, environment_time);

        g.step(
            &InputState {
                reset: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert_eq!(g.session_remaining, g.session_duration());
        assert_eq!(g.environment_time, 0.0);
        assert_eq!(g.day_cycles, 0);
    }

    #[test]
    fn terminal_step_uses_only_remaining_active_time_even_when_dt_overshoots() {
        let t = Tuning::default();
        let mut exact = Game::with_road(
            51,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &t,
        );
        exact.start_run(&t);
        exact.speed = t.scroll_speed_low;
        exact.session_remaining = 0.25;
        let mut overshoot = exact.clone();

        let exact_ev = exact.step(&idle(), 0.25, &t);
        let overshoot_ev = overshoot.step(&idle(), 10.0, &t);
        assert!(exact_ev.time_up && overshoot_ev.time_up);
        assert_eq!(exact.distance, overshoot.distance);
        assert_eq!(exact.environment_time, overshoot.environment_time);
        assert_eq!(exact.result(), overshoot.result());
    }

    #[test]
    fn environment_and_weather_are_timestep_partition_invariant_at_constant_speed() {
        let t = Tuning::default();
        let road = RoadLoop::straight(10_000.0, vec![]);
        let mut whole = Game::with_road(52, road.clone(), Difficulty::Hard, &t);
        let mut split = Game::with_road(52, road, Difficulty::Hard, &t);
        for g in [&mut whole, &mut split] {
            g.start_run(&t);
            g.speed = t.scroll_speed_low;
        }
        whole.step(&idle(), 0.3, &t);
        for _ in 0..3 {
            split.step(&idle(), 0.1, &t);
        }
        assert!((whole.environment_time - split.environment_time).abs() < 1e-6);
        assert_eq!(whole.day_cycles, split.day_cycles);
        assert_eq!(whole.weather(), split.weather());
    }

    #[test]
    fn oncoming_crossings_are_mutually_exclusive_and_roadside_is_ignored() {
        let t = Tuning::default();
        let hazards = vec![
            Hazard {
                kind: HazardKind::Oncoming,
                x: 0.0,
                s: 1.0,
            },
            Hazard {
                kind: HazardKind::Oncoming,
                x: 0.8,
                s: 1.0,
            },
            Hazard {
                kind: HazardKind::Roadside,
                x: 0.0,
                s: 1.0,
            },
        ];
        let mut g = Game::with_road(53, RoadLoop::straight(10.0, hazards), Difficulty::Hard, &t);
        g.start_run(&t);
        g.speed = t.scroll_speed_low;
        let ev = g.step(&idle(), 0.1, &t);
        assert!(ev.bumped);
        assert_eq!(g.cars_bumped, 1);
        assert_eq!(g.cars_passed, 1);
        assert_eq!(g.cars_bumped + g.cars_passed, 2);
    }

    #[test]
    fn repeated_loop_encounters_count_and_apply_only_one_impulse_per_tick() {
        let t = Tuning::default();
        let hazard = Hazard {
            kind: HazardKind::Oncoming,
            x: 0.0,
            s: 1.0,
        };
        let mut g = Game::with_road(
            54,
            RoadLoop::straight(2.0, vec![hazard]),
            Difficulty::Hard,
            &t,
        );
        g.start_run(&t);
        g.speed = t.scroll_speed_low;
        g.step(&idle(), 0.5, &t);
        assert_eq!(g.cars_bumped, 3, "crossings at 1, 3, and 5 meters");
        assert_eq!(g.bump_t, t.crash_flash_secs);
    }

    #[test]
    fn car_collision_takes_precedence_over_simultaneous_off_road_bump() {
        let t = Tuning::default();
        let hazard = Hazard {
            kind: HazardKind::Oncoming,
            x: -1.0,
            s: 1.0,
        };
        let mut g = Game::with_road(
            55,
            RoadLoop::straight(100.0, vec![hazard]),
            Difficulty::Hard,
            &t,
        );
        g.start_run(&t);
        g.speed = t.scroll_speed_low;
        g.road_x = 1.1;
        g.off_road = t.off_road_grace;
        let ev = g.step(&idle(), 0.1, &t);
        assert!(ev.bumped);
        assert_eq!(g.cars_bumped, 1);
        assert!(
            g.bump_dir < 0.0,
            "car direction must win over positive off-road side"
        );
    }

    #[test]
    fn score_clamps_and_has_expected_ordering_effects() {
        assert_eq!(score_for(0, 0, 1), 0);
        assert_eq!(score_for(1, 3, 1), 10_000);
        assert!(score_for(2, 0, 0) > score_for(1, 99, 0));
        assert_eq!(score_for(u64::MAX, u64::MAX, 0), u64::MAX);
        assert_eq!(score_for(0, 0, u64::MAX), 0);
    }

    #[test]
    fn leaderboard_validates_initials_sorts_stably_and_caps_per_difficulty() {
        assert!(LeaderboardEntry::new("abC", Difficulty::Hard, 1, 0, 0).is_none());
        assert!(LeaderboardEntry::new("TOOLONG", Difficulty::Hard, 1, 0, 0).is_none());
        let mut entries = vec![
            LeaderboardEntry::new("AAA", Difficulty::Hard, 1, 3, 1).unwrap(),
            LeaderboardEntry::new("BBB", Difficulty::Hard, 1, 3, 1).unwrap(),
            LeaderboardEntry::new("CCC", Difficulty::Hard, 2, 0, 0).unwrap(),
            LeaderboardEntry::new("DDD", Difficulty::Hard, 1, 4, 0).unwrap(),
        ];
        for n in 0..9 {
            let name = format!("Z{}A", (b'A' + n as u8) as char);
            entries.push(LeaderboardEntry::new(&name, Difficulty::Hard, 0, n, 0).unwrap());
        }
        rank_leaderboard(&mut entries);
        assert_eq!(entries.len(), LEADERBOARD_CAPACITY_PER_DIFFICULTY);
        assert_eq!(entries[0].initials_str(), "CCC");
        let aaa = entries
            .iter()
            .position(|entry| entry.initials_str() == "AAA")
            .unwrap();
        let bbb = entries
            .iter()
            .position(|entry| entry.initials_str() == "BBB")
            .unwrap();
        assert!(aaa < bbb, "equal entries retain insertion order");
    }

    #[test]
    fn off_road_drag_is_progressive_and_leaves_on_road_speed_untouched() {
        let t = Tuning::default();
        assert_eq!(off_road_speed_factor(0.0, &t), 1.0);
        assert_eq!(off_road_speed_factor(t.off_road_limit, &t), 1.0);
        let near = off_road_speed_factor(t.off_road_limit + 0.08, &t);
        let far = off_road_speed_factor(1.34, &t);
        assert!(near < 1.0 && far < near);
        assert!((off_road_speed_factor(1.35, &t) - t.off_road_min_speed_factor).abs() < 1e-6);
    }

    #[test]
    fn off_road_speed_eases_down_and_recovers_after_returning_to_road() {
        let t = Tuning {
            off_road_grace: 2.0,
            ..Default::default()
        };
        let mut g = Game::with_road(
            56,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &t,
        );
        g.start_run(&t);
        g.speed = t.scroll_speed_low;
        g.road_x = 1.34;
        g.step(&idle(), 0.3, &t);
        let dragged = g.speed;
        assert!(dragged < t.scroll_speed_low);

        g.road_x = 0.0;
        g.off_road = 0.0;
        g.step(&idle(), 0.6, &t);
        assert!(g.speed > dragged, "speed should recover smoothly on-road");
        assert!(g.speed <= t.scroll_speed_low);
    }

    #[test]
    fn shift_threshold_is_exact_and_same_gear_is_idempotent() {
        let (mut g, t) = game(60);
        g.start_run(&t);
        assert_eq!(g.gear(), Gear::Low);
        g.speed = t.shift_up_min_speed - 0.001;
        let rejected = g.step(
            &InputState {
                gear_request: Some(Gear::High),
                ..idle()
            },
            0.000_001,
            &t,
        );
        assert!(rejected.high_rejected);
        assert_eq!(g.gear(), Gear::Low);

        g.speed = t.shift_up_min_speed;
        let accepted = g.step(
            &InputState {
                gear_request: Some(Gear::High),
                ..idle()
            },
            0.000_001,
            &t,
        );
        assert!(!accepted.high_rejected);
        assert_eq!(g.gear(), Gear::High);
        let same = g.step(
            &InputState {
                gear_request: Some(Gear::High),
                ..idle()
            },
            0.000_001,
            &t,
        );
        assert!(!same.high_rejected);
        assert!(!same.downshifted);
        assert_eq!(g.gear(), Gear::High);
    }

    #[test]
    fn downshift_reports_severity_and_retains_tuned_speed() {
        let (mut normal, t) = game(61);
        normal.start_run(&t);
        normal.speed = t.downshift_safe_speed;
        normal.gear = Gear::High;
        let ev = normal.step(
            &InputState {
                gear_request: Some(Gear::Low),
                ..idle()
            },
            0.000_001,
            &t,
        );
        assert!(ev.downshifted);
        assert_eq!(ev.downshift_severity, Some(DownshiftSeverity::Normal));
        assert!((normal.speed - 12.0 * 0.94).abs() < 0.001);

        let (mut unsafe_shift, _) = game(62);
        unsafe_shift.start_run(&t);
        unsafe_shift.speed = 18.0;
        unsafe_shift.gear = Gear::High;
        let ev = unsafe_shift.step(
            &InputState {
                gear_request: Some(Gear::Low),
                ..idle()
            },
            0.000_001,
            &t,
        );
        assert_eq!(ev.downshift_severity, Some(DownshiftSeverity::Unsafe));
        assert!((unsafe_shift.speed - 18.0 * 0.82).abs() < 0.001);
        assert!(unsafe_shift.low_overrev(&t) > 0.0);
    }

    #[test]
    fn bump_recovery_requires_low_then_speed_then_high() {
        let (mut g, t) = game(63);
        g.start_run(&t);
        g.gear = Gear::High;
        g.speed = 20.0;
        let mut bump_ev = StepEvents::default();
        g.bump(&mut bump_ev, &t, 1.0);
        assert!(bump_ev.bumped);
        assert_eq!(g.recovery(), Recovery::NeedLow);
        let after_bump = g.speed;
        g.step(&idle(), 0.5, &t);
        assert!(g.speed < after_bump, "HIGH must bog until LOW is selected");
        g.nitro_charges = 1;
        let charge = g.nitro_charges;
        g.step(
            &InputState {
                nitro: true,
                ..idle()
            },
            DT,
            &t,
        );
        assert_eq!(g.nitro_charges, charge);

        g.step(
            &InputState {
                gear_request: Some(Gear::Low),
                ..idle()
            },
            0.000_001,
            &t,
        );
        assert_eq!(g.recovery(), Recovery::BuildSpeed);
        g.speed = t.shift_up_min_speed - 0.001;
        let early = g.step(
            &InputState {
                gear_request: Some(Gear::High),
                ..idle()
            },
            0.000_001,
            &t,
        );
        assert!(early.high_rejected);
        g.speed = t.shift_up_min_speed;
        let ready = g.step(
            &InputState {
                gear_request: Some(Gear::High),
                ..idle()
            },
            0.000_001,
            &t,
        );
        assert!(!ready.high_rejected);
        assert_eq!(g.recovery(), Recovery::Normal);
        assert_eq!(g.gear(), Gear::High);

        g.bump(&mut StepEvents::default(), &t, -1.0);
        g.bump(&mut StepEvents::default(), &t, 1.0);
        assert_eq!(g.recovery(), Recovery::NeedLow);
    }

    #[test]
    fn low_gear_bump_builds_speed_before_high_is_ready() {
        let (mut g, t) = game(64);
        g.start_run(&t);
        g.speed = 11.0;
        g.bump(&mut StepEvents::default(), &t, 1.0);
        assert_eq!(g.recovery(), Recovery::BuildSpeed);
        g.speed = t.shift_up_min_speed;
        g.step(&idle(), 0.0, &t);
        assert_eq!(g.recovery(), Recovery::ReadyForHigh);
    }

    #[test]
    fn nitro_cooldown_starts_at_expiry_freezes_and_resets() {
        let t = Tuning::default();
        let mut g = Game::with_road(
            65,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &t,
        );
        g.start_run(&t);
        g.nitro_charges = 2;
        let started = g.step(
            &InputState {
                nitro: true,
                ..idle()
            },
            0.1,
            &t,
        );
        assert!(started.nitro_started);
        assert_eq!(g.nitro_charges, 1);
        g.step(&idle(), t.nitro_duration_secs - 0.1, &t);
        assert_eq!(g.nitro_t, 0.0);
        assert_eq!(g.nitro_cooldown, t.nitro_cooldown_secs);
        let ignored = g.step(
            &InputState {
                nitro: true,
                ..idle()
            },
            0.1,
            &t,
        );
        assert!(!ignored.nitro_started);
        assert_eq!(g.nitro_charges, 1);

        g.step(
            &InputState {
                pause: true,
                ..idle()
            },
            DT,
            &t,
        );
        let frozen = g.nitro_cooldown;
        g.step(&idle(), 1.0, &t);
        assert_eq!(g.nitro_cooldown, frozen);
        g.step(
            &InputState {
                pause: true,
                ..idle()
            },
            DT,
            &t,
        );
        g.step(&idle(), frozen, &t);
        assert_eq!(g.nitro_cooldown, 0.0);
        let charges_before_restart = g.nitro_charges;
        let restarted = g.step(
            &InputState {
                nitro: true,
                ..idle()
            },
            DT,
            &t,
        );
        assert!(restarted.nitro_started);
        assert_eq!(g.nitro_charges, charges_before_restart - 1);

        g.nitro_t = 0.0;
        g.nitro_cooldown = 1.0;
        let terminal_frozen = g.nitro_cooldown;
        g.phase = Phase::GameOver {
            reason: EndReason::Time,
        };
        g.step(&idle(), 1.0, &t);
        assert_eq!(g.nitro_cooldown, terminal_frozen);
        g.step(
            &InputState {
                reset: true,
                ..idle()
            },
            DT,
            &t,
        );
        assert_eq!(g.nitro_cooldown, 0.0);
    }

    #[test]
    fn nitro_uses_the_full_five_second_interval_before_cooldown() {
        let t = Tuning::default();
        let mut g = Game::with_road(
            67,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &t,
        );
        g.start_run(&t);
        g.speed = t.scroll_speed_low;
        g.nitro_charges = 1;

        let ev = g.step(
            &InputState {
                nitro: true,
                ..idle()
            },
            t.nitro_duration_secs,
            &t,
        );
        let target = t.scroll_speed_low * t.nitro_speed_multiplier;
        let decay = (-t.nitro_duration_secs / t.gear_ramp_secs).exp();
        let expected_speed = target + (t.scroll_speed_low - target) * decay;
        let expected_distance = target * t.nitro_duration_secs
            + (t.scroll_speed_low - target) * t.gear_ramp_secs * (1.0 - decay);

        assert!(ev.nitro_started);
        assert!((g.speed - expected_speed).abs() < 0.0001);
        assert!((g.distance - expected_distance).abs() < 0.0001);
        assert_eq!(g.nitro_t, 0.0);
        assert_eq!(g.nitro_cooldown, t.nitro_cooldown_secs);
    }

    #[test]
    fn nitro_expiry_is_timestep_partition_invariant() {
        let t = Tuning::default();
        let road = RoadLoop::straight(
            10_000.0,
            vec![Hazard {
                kind: HazardKind::Oncoming,
                x: 0.8,
                s: 50.0,
            }],
        );
        let mut coarse = Game::with_road(68, road.clone(), Difficulty::Hard, &t);
        let mut split = Game::with_road(68, road, Difficulty::Hard, &t);
        for game in [&mut coarse, &mut split] {
            game.start_run(&t);
            game.speed = t.scroll_speed_low;
            game.nitro_charges = 1;
        }
        let activate = InputState {
            nitro: true,
            ..idle()
        };
        coarse.step(&activate, 6.25, &t);
        split.step(&activate, 0.7, &t);
        for dt in [0.55, 1.1, 0.4, 1.35, 0.8, 1.35] {
            split.step(&idle(), dt, &t);
        }

        assert_eq!(coarse.nitro_t, split.nitro_t);
        assert!((coarse.nitro_cooldown - split.nitro_cooldown).abs() < 0.0001);
        assert!((coarse.speed - split.speed).abs() < 0.0001);
        assert!((coarse.distance - split.distance).abs() < 0.001);
        assert!((coarse.environment_time - split.environment_time).abs() < 0.0001);
        assert_eq!(coarse.cars_passed, split.cars_passed);
        assert_eq!(coarse.cars_bumped, split.cars_bumped);
        assert_eq!(coarse.day_cycles, split.day_cycles);
    }

    #[test]
    fn difficulties_define_immutable_run_results_and_isolated_rankings() {
        assert_eq!(Difficulty::Easy.duration_secs(), 120.0);
        assert_eq!(Difficulty::Medium.duration_secs(), 360.0);
        assert_eq!(Difficulty::Hard.duration_secs(), 600.0);
        assert_eq!(Difficulty::Medium.label(), "MEDIUM");

        let t = Tuning::default();
        let mut g = Game::with_road_for_duration(
            66,
            RoadLoop::straight(100.0, vec![]),
            Difficulty::Easy,
            0.1,
            &t,
        );
        g.start_run(&t);
        g.step(&idle(), 0.1, &t);
        let result = *g.result().unwrap();
        assert_eq!(result.difficulty, Difficulty::Easy);
        g.restart_with_difficulty(Difficulty::Hard, &t);
        assert_eq!(result.difficulty, Difficulty::Easy);
        assert_eq!(g.session_remaining, 600.0);

        let mut entries = Vec::new();
        for difficulty in Difficulty::ALL {
            for n in 0..11 {
                let initials = format!("{}AA", (b'A' + n) as char);
                entries.push(
                    LeaderboardEntry::new(&initials, difficulty, 0, u64::from(n), 0).unwrap(),
                );
            }
        }
        rank_leaderboard(&mut entries);
        assert_eq!(entries.len(), LEADERBOARD_CAPACITY);
        for difficulty in Difficulty::ALL {
            assert_eq!(leaderboard_for(&entries, difficulty).count(), 10);
        }
    }
}
