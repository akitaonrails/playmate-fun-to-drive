//! The game state machine + fixed-step simulation (PLAN.md M5).
//!
//! Model mirrors the toy (research/toy-mechanics.md): the car is a FIXED
//! decal at screen center-bottom; the printed drum (road loop) scrolls and
//! shifts laterally under it. Steering moves the ROAD; hazards ride the road;
//! bump = an oncoming car or roadside hazard under the player marker. Bumps
//! slow the toy but do not end the timed session; leaving the road adds drag.

use crate::environment::{
    season_weights, WeatherKind, WeatherSchedule, WeatherTransition, DAY_SECS,
};
use crate::input::{Gear, InputState};
use crate::rng::Rng;
use crate::road::{Hazard, HazardKind, RoadLoop, TrafficMotion};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GearChange {
    pub gear: Gear,
    pub downshift_severity: Option<DownshiftSeverity>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StepEvents {
    pub started: bool,
    /// Toy-ish bump (hazard or off-road): the run continues.
    pub bump_direction: Option<f32>,
    pub time_up: bool,
    pub game_over: bool,
    /// Zero-based meter segment filled by this pickup.
    pub nitro_collected_segment: Option<u8>,
    pub nitro_started: bool,
    pub jump_collected: bool,
    pub jump_started: bool,
    pub jump_landed: bool,
    pub cold_air_hop_started: bool,
    pub cold_air_hop_landed: bool,
    pub high_rejected: bool,
    pub gear_changed: Option<GearChange>,
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

pub const DAY_CYCLE_SCORE: u64 = 10_000;
pub const CLEAN_PASS_SCORE: u64 = 100;
pub const BUMP_PENALTY: u64 = 200;

/// Cycles reward sustained pace, passes reward controlled aggression, and a
/// collision creates a three-pass swing: the missed +100 pass plus a -200
/// penalty. The physical speed loss remains an additional cost. Wide signed
/// arithmetic prevents either overflow or unsigned penalty underflow.
pub fn score_for(day_cycles: u64, cars_passed: u64, cars_bumped: u64) -> u64 {
    let positive = i128::from(day_cycles)
        .saturating_mul(i128::from(DAY_CYCLE_SCORE))
        .saturating_add(i128::from(cars_passed).saturating_mul(i128::from(CLEAN_PASS_SCORE)));
    let score =
        positive.saturating_sub(i128::from(cars_bumped).saturating_mul(i128::from(BUMP_PENALTY)));
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RareTrafficHelp {
    pub hazard_index: usize,
    pub start_ahead: f32,
    pub start_x: f32,
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
    /// Environment clock at the start of this run. Visuals may begin at any
    /// point in the year, while competitive cycle counting still starts at 0.
    environment_origin: f32,
    pub day_cycles: u64,
    pub cars_passed: u64,
    pub cars_bumped: u64,
    /// Crash flash intensity in [0, 1] while > 0.
    pub flash: f32,
    /// Seconds accumulated off the road, used for presentation/audio intensity.
    pub off_road: f32,
    /// Seconds of calm HIGH-gear driving accumulated on the current straight.
    pub straightaway_charge: f32,
    /// True while the run is paused (Start button / P).
    pub paused: bool,
    /// Bump slow-down timer + kick direction.
    pub bump_t: f32,
    pub bump_dir: f32,
    /// Filled cells in the three-cell nitro meter. A boost consumes all cells.
    pub nitro_charges: u8,
    /// Seconds remaining in the active boost.
    pub nitro_t: f32,
    /// Seconds before another charge may be activated.
    pub nitro_cooldown: f32,
    /// Hidden four-use cadence for the time-machine easter egg.
    cold_air_uses_toward_time_travel: u8,
    time_travel_sequence: u32,
    time_travel_boost: bool,
    /// Absolute course distance of the next pickup.
    pub next_nitro_distance: f32,
    /// Lane of the next pickup, relative to road center.
    pub next_nitro_lane: f32,
    nitro_pickup_index: u32,
    /// Single stored Jump Kick and its deterministic per-cycle pickup.
    pub jump_charge: bool,
    pub jump_t: f32,
    /// Short automatic crest hop while the cold-air thruster is active.
    pub cold_air_hop_t: f32,
    cold_air_hop_lockout_t: f32,
    pub next_jump_distance: Option<f32>,
    pub next_jump_lane: f32,
    jump_pickup_cycle: u64,
    jump_start_distance: f32,
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
        let traffic_bonus = match difficulty {
            Difficulty::Easy => 0,
            Difficulty::Medium => t.traffic_medium_extra,
            Difficulty::Hard => t.traffic_hard_extra,
        };
        let road = RoadLoop::generate_with_traffic_bonus(&mut rng, traffic_bonus);
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
        let mut game = Game {
            seed,
            phase: Phase::Ready,
            road,
            speed: 0.0,
            distance: 0.0,
            road_x: 0.0,
            session_remaining: duration_secs,
            environment_time: 0.0,
            environment_origin: 0.0,
            day_cycles: 0,
            cars_passed: 0,
            cars_bumped: 0,
            flash: 0.0,
            off_road: 0.0,
            straightaway_charge: 0.0,
            paused: false,
            bump_t: 0.0,
            bump_dir: 0.0,
            nitro_charges: 0,
            nitro_t: 0.0,
            nitro_cooldown: 0.0,
            cold_air_uses_toward_time_travel: 0,
            time_travel_sequence: 0,
            time_travel_boost: false,
            next_nitro_distance: t.nitro_pickup_first_m,
            next_nitro_lane: Self::nitro_lane(seed, 0, t),
            nitro_pickup_index: 0,
            jump_charge: false,
            jump_t: 0.0,
            cold_air_hop_t: 0.0,
            cold_air_hop_lockout_t: 0.0,
            next_jump_distance: None,
            next_jump_lane: 0.0,
            jump_pickup_cycle: 0,
            jump_start_distance: 0.0,
            gear: Gear::Low,
            recovery: Recovery::Normal,
            difficulty,
            session_duration: duration_secs,
            weather: WeatherSchedule::new(seed),
            result: None,
        };
        (game.next_nitro_distance, game.next_nitro_lane) = game.safe_pickup_placement(
            game.next_nitro_distance,
            game.next_nitro_lane,
            t.nitro_lane_offset,
            t.nitro_pickup_half_width,
            t,
        );
        game.with_initial_jump_pickup(t)
    }

    fn with_initial_jump_pickup(mut self, t: &Tuning) -> Self {
        self.schedule_jump_pickup_from(0, t);
        self
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

    /// Start the visual environment at an arbitrary point in its year without
    /// awarding already elapsed day/night cycles to the run score.
    pub fn set_environment_start(&mut self, environment_time: f32) {
        let start = environment_time
            .max(0.0)
            .rem_euclid(crate::environment::YEAR_SECS);
        self.environment_time = start;
        self.environment_origin = start;
        self.day_cycles = 0;
    }

    /// Restore an absolute environment clock, including its completed-cycle
    /// count. Used by deterministic QA scenarios that begin near a boundary.
    pub fn set_environment_progress(&mut self, environment_time: f32) {
        self.environment_time = environment_time.max(0.0);
        self.environment_origin = 0.0;
        self.day_cycles = (self.environment_time / DAY_SECS).floor() as u64;
    }

    /// Absolute day/night cycle used by presentation systems such as music.
    pub fn environment_cycle_index(&self) -> u64 {
        (self.environment_time / DAY_SECS).floor().max(0.0) as u64
    }

    fn rare_event_salt(&self) -> u64 {
        self.road
            .hazards
            .first()
            .map_or(0, |hazard| u64::from(hazard.s.to_bits()))
    }

    pub fn rare_event(&self) -> Option<crate::rare_event::RareEvent> {
        crate::rare_event::event_at(self.environment_time, self.rare_event_salt())
    }

    /// Cars removed by the active hurricane/UFO assist. Selection uses the
    /// event's starting distance, keeping rendering and collision in lockstep.
    pub fn rare_traffic_help(&self, tuning: &Tuning) -> [Option<RareTrafficHelp>; 2] {
        use crate::rare_event::RareEventKind;

        let Some(event) = self.rare_event() else {
            return [None; 2];
        };
        let wanted = match event.kind {
            RareEventKind::Hurricane => {
                1 + (crate::rare_event::selection_hash(event.slot, self.seed, 0) % 2) as usize
            }
            RareEventKind::Ufo => 1,
            _ => return [None; 2],
        };
        let start_distance =
            event.slot as f32 * crate::rare_event::SLOT_SECS * tuning.scroll_speed_low;
        let start_pos = start_distance.rem_euclid(self.road.length);
        let mut best: [Option<(u64, RareTrafficHelp)>; 2] = [None; 2];
        for (hazard_index, hazard) in self.road.hazards.iter().enumerate() {
            if hazard.kind != HazardKind::Oncoming {
                continue;
            }
            let ahead = (hazard.s - start_pos).rem_euclid(self.road.length);
            // Keep rare assists in the readable middle-distance band:
            // close enough to watch the pickup, but clear of the player.
            if !(8.0..=28.0).contains(&ahead) {
                continue;
            }
            let mut candidate = Some((
                crate::rare_event::selection_hash(event.slot, self.seed, hazard_index),
                RareTrafficHelp {
                    hazard_index,
                    start_ahead: ahead,
                    start_x: self.hazard_x_at(hazard, ahead, tuning),
                },
            ));
            for slot in best.iter_mut().take(wanted) {
                let should_insert = match (*slot, candidate) {
                    (None, Some(_)) => true,
                    (Some(current), Some(next)) => next.0 < current.0,
                    _ => false,
                };
                if should_insert {
                    std::mem::swap(slot, &mut candidate);
                }
                if candidate.is_none() {
                    break;
                }
            }
        }
        best.map(|candidate| candidate.map(|(_, traffic)| traffic))
    }

    pub fn speed_multiplier(&self, tuning: &Tuning) -> f32 {
        match self.difficulty {
            Difficulty::Easy => tuning.speed_easy_multiplier,
            Difficulty::Medium => tuning.speed_medium_multiplier,
            Difficulty::Hard => tuning.speed_hard_multiplier,
        }
    }

    pub fn low_speed_target(&self, tuning: &Tuning) -> f32 {
        tuning.scroll_speed_low * self.speed_multiplier(tuning)
    }

    pub fn high_speed_target(&self, tuning: &Tuning) -> f32 {
        tuning.scroll_speed_high * self.speed_multiplier(tuning)
    }

    pub fn shift_up_speed(&self, tuning: &Tuning) -> f32 {
        tuning.shift_up_min_speed * self.speed_multiplier(tuning)
    }

    fn recovery_bog_speed(&self, tuning: &Tuning) -> f32 {
        tuning.recovery_bog_speed * self.speed_multiplier(tuning)
    }

    fn downshift_safe_speed(&self, tuning: &Tuning) -> f32 {
        tuning.downshift_safe_speed * self.speed_multiplier(tuning)
    }

    pub fn traffic_motion(&self, tuning: &Tuning) -> TrafficMotion {
        match self.difficulty {
            Difficulty::Easy => tuning.traffic_easy,
            Difficulty::Medium => tuning.traffic_medium,
            Difficulty::Hard => tuning.traffic_hard,
        }
    }

    /// Shared traffic resolver used by both presentation and collision.
    pub fn hazard_x_at(&self, hazard: &Hazard, ahead: f32, tuning: &Tuning) -> f32 {
        self.road
            .hazard_x_at(hazard, ahead, self.traffic_motion(tuning))
    }

    pub fn session_duration(&self) -> f32 {
        self.session_duration
    }

    /// Normalized LOW-gear over-rev warning in [0, 1].
    pub fn low_overrev(&self, t: &Tuning) -> f32 {
        if self.gear != Gear::Low {
            return 0.0;
        }
        let multiplier = self.speed_multiplier(t);
        let start = t.low_overrev_start * multiplier;
        let limit = t.scroll_speed_low * multiplier;
        ((self.speed - start) / (limit - start)).clamp(0.0, 1.0)
    }

    pub fn loop_pos(&self) -> f32 {
        self.distance.rem_euclid(self.road.length)
    }

    pub fn nitro_active(&self) -> bool {
        self.nitro_t > 0.0
    }

    /// Monotonic within one run; advances on every fourth successful COLD-AIR
    /// activation so presentation can play the hidden time-machine reward.
    pub fn time_travel_sequence(&self) -> u32 {
        self.time_travel_sequence
    }

    pub fn nitro_pickup_ahead(&self) -> f32 {
        self.next_nitro_distance - self.distance
    }

    pub fn jump_active(&self) -> bool {
        self.jump_t > 0.0
    }

    pub fn jump_progress(&self, t: &Tuning) -> f32 {
        if self.jump_t <= 0.0 {
            0.0
        } else {
            (1.0 - self.jump_t / t.jump_duration_secs).clamp(0.0, 1.0)
        }
    }

    pub fn cold_air_hop_active(&self) -> bool {
        self.cold_air_hop_t > 0.0
    }

    pub fn cold_air_hop_progress(&self, t: &Tuning) -> f32 {
        if self.cold_air_hop_t <= 0.0 {
            0.0
        } else {
            (1.0 - self.cold_air_hop_t / t.cold_air_hop_duration_secs).clamp(0.0, 1.0)
        }
    }

    pub fn jump_pickup_ahead(&self) -> Option<f32> {
        (!self.jump_charge)
            .then_some(self.next_jump_distance?)
            .map(|distance| distance - self.distance)
    }

    pub fn weather(&self) -> WeatherTransition {
        self.weather.at(self.environment_time)
    }

    pub fn result(&self) -> Option<&SessionResult> {
        self.result.as_ref()
    }

    fn nitro_lane(seed: u64, index: u32, t: &Tuning) -> f32 {
        let lanes = [-t.nitro_lane_offset, 0.0, t.nitro_lane_offset];
        let mixed = seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        lanes[((mixed ^ (mixed >> 32)) % lanes.len() as u64) as usize]
    }

    fn advance_nitro_pickup(&mut self, t: &Tuning) {
        self.nitro_pickup_index += 1;
        self.next_nitro_distance += t.nitro_pickup_interval_m;
        let preferred = Self::nitro_lane(self.seed, self.nitro_pickup_index, t);
        (self.next_nitro_distance, self.next_nitro_lane) = self.safe_pickup_placement(
            self.next_nitro_distance,
            preferred,
            t.nitro_lane_offset,
            t.nitro_pickup_half_width,
            t,
        );
    }

    fn overlaps_pickup(&self, lane: f32, pickup_half_width: f32, t: &Tuning) -> bool {
        (lane + self.road_x).abs() < t.car_half_width + pickup_half_width
    }

    fn safe_pickup_placement(
        &self,
        distance: f32,
        preferred: f32,
        lane_offset: f32,
        pickup_half_width: f32,
        t: &Tuning,
    ) -> (f32, f32) {
        let lanes = [preferred, -lane_offset, 0.0, lane_offset];
        let lateral_clearance = pickup_half_width + t.hazard_half_width;
        // Try alternate lanes first, then nudge the item forward in small
        // deterministic steps if a three-wide traffic pack blocks all lanes.
        for distance_step in 0..=8 {
            let candidate_distance = distance + distance_step as f32 * 2.0;
            let pickup_pos = candidate_distance.rem_euclid(self.road.length);
            for (index, lane) in lanes.into_iter().enumerate() {
                if lanes[..index]
                    .iter()
                    .any(|previous| (previous - lane).abs() < f32::EPSILON)
                {
                    continue;
                }
                let clear = self.road.hazards.iter().all(|hazard| {
                    if hazard.kind != HazardKind::Oncoming {
                        return true;
                    }
                    let longitudinal = signed_loop_delta(hazard.s, pickup_pos, self.road.length);
                    longitudinal.abs() > t.pickup_traffic_clearance_m
                        || (self.hazard_x_at(hazard, longitudinal.max(0.0), t) - lane).abs()
                            >= lateral_clearance
                });
                if clear {
                    return (candidate_distance, lane);
                }
            }
        }
        // The generated road cannot saturate all three lanes for this entire
        // 16 m search band, but retain a deterministic fallback for explicit
        // adversarial test roads.
        (distance + 18.0, preferred)
    }

    fn jump_pickup_for_cycle(&self, cycle: u64, t: &Tuning) -> Option<(f32, f32)> {
        let mut hash =
            self.seed ^ cycle.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x4A55_4D50_4B49_434B;
        hash ^= hash >> 30;
        hash = hash.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        hash ^= hash >> 27;
        let easy_intro = self.difficulty == Difficulty::Easy && cycle == 0;
        let spawn_chance = match self.difficulty {
            Difficulty::Easy => t.jump_spawn_chance_easy_percent,
            Difficulty::Medium => t.jump_spawn_chance_medium_percent,
            Difficulty::Hard => t.jump_spawn_chance_hard_percent,
        };
        if !easy_intro && hash % 100 >= u64::from(spawn_chance) {
            return None;
        }
        let phase = if easy_intro {
            // Guarantee that Easy teaches the rare item during its first
            // minute, even after accounting for the slower speed multiplier
            // and the opening acceleration ramp.
            0.14 + ((hash.rotate_left(17) % 18) as f32 / 100.0)
        } else {
            0.28 + ((hash.rotate_left(17) % 47) as f32 / 100.0)
        };
        let distance = (cycle as f32 + phase) * DAY_SECS * t.scroll_speed_low;
        let lanes = [-t.jump_lane_offset, 0.0, t.jump_lane_offset];
        let preferred = lanes[(hash.rotate_left(41) % lanes.len() as u64) as usize];
        let (distance, lane) = self.safe_pickup_placement(
            distance,
            preferred,
            t.jump_lane_offset,
            t.jump_pickup_half_width,
            t,
        );
        Some((distance, lane))
    }

    fn schedule_jump_pickup_from(&mut self, first_cycle: u64, t: &Tuning) {
        self.next_jump_distance = None;
        for cycle in first_cycle..first_cycle.saturating_add(64) {
            if let Some((distance, lane)) = self.jump_pickup_for_cycle(cycle, t) {
                self.jump_pickup_cycle = cycle;
                self.next_jump_distance = Some(distance);
                self.next_jump_lane = lane;
                return;
            }
        }
        self.jump_pickup_cycle = first_cycle.saturating_add(64);
    }

    fn advance_jump_pickup(&mut self, t: &Tuning) {
        self.schedule_jump_pickup_from(self.jump_pickup_cycle.saturating_add(1), t);
    }

    /// Lateral offset of the road row `ahead_m` meters ahead of the car.
    pub fn road_offset_at(&self, ahead_m: f32) -> f32 {
        let drum_shift = self.road_x - self.road.curve_at(self.loop_pos());
        self.road.curve_at(self.loop_pos() + ahead_m) + drum_shift
    }

    pub fn road_grade(&self) -> f32 {
        self.road.grade_at(self.loop_pos())
    }

    pub fn slope_speed_factor(&self, tuning: &Tuning) -> f32 {
        (1.0 - self.road_grade() * tuning.slope_speed_gain)
            .clamp(tuning.slope_speed_min_factor, tuning.slope_speed_max_factor)
    }

    pub fn straightaway_overdrive_ready(&self, tuning: &Tuning) -> bool {
        self.straightaway_charge >= tuning.straightaway_charge_secs
    }

    fn calm_straight(&self, input: &InputState, tuning: &Tuning) -> bool {
        self.gear == Gear::High
            && self.recovery == Recovery::Normal
            && input.brake <= f32::EPSILON
            && input.steer.abs() <= tuning.straightaway_steer_threshold
            && self.road_x.abs() <= tuning.off_road_limit
            && self.road.curve_at(self.loop_pos()).abs() <= tuning.straightaway_curve_threshold
    }

    pub fn off_road_intensity(&self, tuning: &Tuning) -> f32 {
        ((self.road_x.abs() - tuning.off_road_limit)
            / (tuning.max_lateral_offset - tuning.off_road_limit))
            .clamp(0.0, 1.0)
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
        self.environment_origin = 0.0;
        self.day_cycles = 0;
        self.cars_passed = 0;
        self.cars_bumped = 0;
        self.flash = 0.0;
        self.off_road = 0.0;
        self.straightaway_charge = 0.0;
        self.bump_t = 0.0;
        self.bump_dir = 0.0;
        self.nitro_charges = 0;
        self.nitro_t = 0.0;
        self.nitro_cooldown = 0.0;
        self.cold_air_uses_toward_time_travel = 0;
        self.time_travel_sequence = 0;
        self.time_travel_boost = false;
        self.nitro_pickup_index = 0;
        self.next_nitro_distance = t.nitro_pickup_first_m;
        (self.next_nitro_distance, self.next_nitro_lane) = self.safe_pickup_placement(
            self.next_nitro_distance,
            Self::nitro_lane(self.seed, 0, t),
            t.nitro_lane_offset,
            t.nitro_pickup_half_width,
            t,
        );
        self.jump_charge = false;
        self.jump_t = 0.0;
        self.cold_air_hop_t = 0.0;
        self.cold_air_hop_lockout_t = 0.0;
        self.jump_start_distance = 0.0;
        self.schedule_jump_pickup_from(0, t);
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

    /// Clear the active run while preserving the selected course/difficulty.
    /// The next menu confirmation can then start from a clean Ready state.
    pub fn return_to_ready(&mut self, t: &Tuning) {
        self.start_run(t);
        self.phase = Phase::Ready;
        self.paused = false;
    }

    fn update_recovery_readiness(&mut self, t: &Tuning) {
        if self.recovery == Recovery::BuildSpeed && self.speed >= self.shift_up_speed(t) {
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
                if self.speed < self.shift_up_speed(t) || !recovery_allows {
                    ev.high_rejected = true;
                    return;
                }
                self.gear = Gear::High;
                if self.recovery == Recovery::ReadyForHigh {
                    self.recovery = Recovery::Normal;
                }
                ev.gear_changed = Some(GearChange {
                    gear: Gear::High,
                    downshift_severity: None,
                });
            }
            Gear::Low => {
                let severity = if self.speed <= self.downshift_safe_speed(t) {
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
                ev.gear_changed = Some(GearChange {
                    gear: Gear::Low,
                    downshift_severity: Some(severity),
                });
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
            && self.nitro_charges == t.nitro_capacity
        {
            self.nitro_charges = 0;
            self.nitro_t = t.nitro_duration_secs;
            self.cold_air_uses_toward_time_travel += 1;
            self.time_travel_boost = self.cold_air_uses_toward_time_travel >= 4;
            if self.time_travel_boost {
                self.cold_air_uses_toward_time_travel = 0;
                self.time_travel_sequence = self.time_travel_sequence.saturating_add(1);
            }
            ev.nitro_started = true;
        }
        if input.jump
            && self.jump_charge
            && self.jump_t <= 0.0
            && self.cold_air_hop_t <= 0.0
            && self.bump_t <= 0.0
        {
            self.jump_charge = false;
            self.jump_t = t.jump_duration_secs;
            self.jump_start_distance = self.distance;
            ev.jump_started = true;
        }
        let grade = self.road_grade();
        let grade_ahead = self
            .road
            .grade_at(self.loop_pos() + t.cold_air_hop_lookahead_m);
        if self.nitro_t > 0.0
            && self.cold_air_hop_t <= 0.0
            && self.cold_air_hop_lockout_t <= 0.0
            && self.jump_t <= 0.0
            && self.bump_t <= 0.0
            && self.speed >= self.low_speed_target(t) * 0.8
            && grade >= t.cold_air_hop_min_grade
            && grade_ahead <= 0.0
        {
            self.cold_air_hop_t = t.cold_air_hop_duration_secs;
            // A lockout longer than the airborne window prevents the same
            // broad crest from launching the car twice at unusually low speed.
            self.cold_air_hop_lockout_t = t.cold_air_hop_duration_secs + 0.8;
            ev.cold_air_hop_started = true;
        }

        // Timer boundaries split one outer fixed step into constant-target
        // intervals. The active boost therefore applies for its complete five
        // seconds, and any time remaining after expiry ages the cooldown.
        let mut remaining = dt;
        let mut obstacle_collision = false;
        let mut obstacle_bump_dir = 1.0;
        while remaining > 0.0 {
            let nitro_active = self.nitro_t > 0.0;
            let bump_active = self.bump_t > 0.0;
            let jump_active = self.jump_t > 0.0;
            let cold_air_hop_active = self.cold_air_hop_t > 0.0;
            let straightaway_charging = self.calm_straight(input, t);
            let mut interval = remaining;
            if nitro_active {
                interval = interval.min(self.nitro_t);
            }
            if bump_active {
                interval = interval.min(self.bump_t);
            }
            if jump_active {
                interval = interval.min(self.jump_t);
            }
            if cold_air_hop_active {
                interval = interval.min(self.cold_air_hop_t);
            }
            if straightaway_charging && !self.straightaway_overdrive_ready(t) {
                interval = interval.min(t.straightaway_charge_secs - self.straightaway_charge);
            } else if !straightaway_charging && self.straightaway_charge > 0.0 {
                interval = interval.min(self.straightaway_charge * 0.5);
            }

            self.integrate_running_interval(
                input,
                interval,
                t,
                ev,
                nitro_active,
                bump_active,
                jump_active,
                cold_air_hop_active,
                &mut obstacle_collision,
                &mut obstacle_bump_dir,
            );

            if nitro_active {
                self.nitro_t = (self.nitro_t - interval).max(0.0);
                if self.nitro_t <= f32::EPSILON {
                    self.nitro_t = 0.0;
                    self.time_travel_boost = false;
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
            if jump_active {
                self.jump_t = (self.jump_t - interval).max(0.0);
                if self.jump_t <= 1e-5 {
                    self.jump_t = 0.0;
                    ev.jump_landed = true;
                    self.resolve_jump_landing(t, &mut obstacle_collision, &mut obstacle_bump_dir);
                }
            }
            if cold_air_hop_active {
                self.cold_air_hop_t = (self.cold_air_hop_t - interval).max(0.0);
                if self.cold_air_hop_t <= 1e-5 {
                    self.cold_air_hop_t = 0.0;
                    ev.cold_air_hop_landed = true;
                }
            }
            if straightaway_charging {
                self.straightaway_charge =
                    (self.straightaway_charge + interval).min(t.straightaway_charge_secs);
            } else {
                self.straightaway_charge = (self.straightaway_charge - interval * 2.0).max(0.0);
            }
            remaining = (remaining - interval).max(0.0);
        }
        self.cold_air_hop_lockout_t = (self.cold_air_hop_lockout_t - dt).max(0.0);

        // Preserve the outer fixed-step contract: all encounters are counted,
        // but collectively cause at most one physical obstacle impulse.
        if obstacle_collision {
            self.bump(ev, t, obstacle_bump_dir);
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
        jump_active: bool,
        cold_air_hop_active: bool,
        obstacle_collision: &mut bool,
        obstacle_bump_dir: &mut f32,
    ) {
        let mut target = match self.recovery {
            Recovery::NeedLow => self.recovery_bog_speed(t),
            _ => match self.gear {
                Gear::Low => self.low_speed_target(t),
                Gear::High => self.high_speed_target(t),
            },
        };
        if bump_active && self.recovery != Recovery::NeedLow {
            target *= t.bump_target_speed_factor;
        }
        if self.straightaway_overdrive_ready(t)
            && self.gear == Gear::High
            && self.recovery == Recovery::Normal
        {
            target *= t.straightaway_speed_multiplier;
        }
        if nitro_active && self.recovery == Recovery::Normal {
            target *= t.nitro_speed_multiplier;
        }
        if jump_active {
            target *= t.jump_speed_multiplier;
        }
        target *= self.slope_speed_factor(t);
        if nitro_active && self.time_travel_boost && self.recovery == Recovery::Normal {
            // The fourth boost must genuinely cross 88 on every difficulty,
            // even on a gentle uphill. Player-imposed brake/off-road drag is
            // applied below and can still spoil the attempt.
            target = target.max(t.straightaway_high_speed_cap);
        }
        if self.gear == Gear::High
            && self.recovery == Recovery::Normal
            && !nitro_active
            && !jump_active
        {
            let cap = if self.straightaway_overdrive_ready(t) {
                t.straightaway_high_speed_cap
            } else {
                t.normal_high_speed_cap
            };
            target = target.min(cap);
        }
        target *= off_road_speed_factor(self.road_x, t);
        let brake = input.brake.clamp(0.0, 1.0);
        target *= 1.0 - brake;
        let initial_speed = self.speed;
        let ramp_secs = if brake > 0.0 {
            t.brake_ramp_secs
        } else if self.gear == Gear::Low {
            t.low_gear_ramp_secs
        } else {
            t.gear_ramp_secs
        };
        let decay = (-dt / ramp_secs).exp();
        self.speed = target + (initial_speed - target) * decay;
        if brake > 0.0
            && self.gear == Gear::High
            && self.recovery == Recovery::Normal
            && self.speed < self.shift_up_speed(t)
        {
            self.recovery = Recovery::NeedLow;
        }
        self.update_recovery_readiness(t);

        let previous_distance = self.distance;
        // Exact integral of the exponential speed ramp for a constant target.
        // This makes distance identical whether an interval is stepped whole
        // or partitioned by callers.
        let distance_delta = target * dt + (initial_speed - target) * ramp_secs * (1.0 - decay);
        self.distance += distance_delta;
        self.environment_time += distance_delta / t.scroll_speed_low;
        self.day_cycles = ((self.environment_time - self.environment_origin) / DAY_SECS)
            .floor()
            .max(0.0) as u64;
        let pos = self.loop_pos();

        // road lateral dynamics: road drifts toward the printed curve target;
        // steering shifts the drum the other way (car "moves" relatively).
        let curve_target = self.road.curve_at(pos) * t.curve_gain;
        let drift_speed = (self.speed / t.scroll_speed_low).min(t.curve_drift_speed_cap);
        let drift = 1.0 - (-dt * t.drift_rate * drift_speed).exp();
        self.road_x += (curve_target - self.road_x) * drift;
        let steer_factor = if jump_active {
            t.jump_air_steer_factor
        } else if cold_air_hop_active {
            t.cold_air_hop_air_steer_factor
        } else {
            1.0
        };
        let weather = self.weather();
        let rain = weather_kind_weight(weather, WeatherKind::Rain);
        let winter = season_weights(self.environment_time)[3];
        let weather_grip = precipitation_steer_factor(rain, winter, t);
        self.road_x -= input.steer * t.steer_rate * steer_factor * weather_grip * dt;
        self.road_x = self
            .road_x
            .clamp(-t.max_lateral_offset, t.max_lateral_offset);

        // One deterministic pickup at a time. The absolute-distance schedule
        // keeps appearances occasional and guarantees more than four per
        // 100-second light cycle at LOW speed.
        while self.distance >= self.next_nitro_distance {
            if self.overlaps_pickup(self.next_nitro_lane, t.nitro_pickup_half_width, t)
                && self.nitro_charges < t.nitro_capacity
            {
                self.nitro_charges += 1;
                ev.nitro_collected_segment = self.nitro_charges.checked_sub(1);
            }
            self.advance_nitro_pickup(t);
        }

        while self
            .next_jump_distance
            .is_some_and(|distance| self.distance >= distance)
        {
            if self.overlaps_pickup(self.next_jump_lane, t.jump_pickup_half_width, t)
                && !self.jump_charge
            {
                self.jump_charge = true;
                ev.jump_collected = true;
            }
            self.advance_jump_pickup(t);
        }

        // Leaving the road progressively lowers target speed. This timer only
        // drives presentation/audio; road edges never create a crash impulse.
        if self.road_x.abs() > t.off_road_limit {
            self.off_road += dt;
        } else {
            self.off_road = (self.off_road - t.off_road_recovery_rate * dt).max(0.0);
        }

        // Fixed-step traffic contract: account every oncoming crossing in
        // (previous_distance, distance], including repeated-loop encounters.
        // Crossings use the resolved road offset for their timer-bounded
        // interval.
        // Passed and bumped are mutually exclusive per car encounter.
        // Roadside scenery can cause the same physical crash but deliberately
        // does not alter either car-specific leaderboard counter.
        let hit_band = (t.car_half_width + t.hazard_half_width) * (1.0 - t.collision_forgiveness);
        let helped = self.rare_traffic_help(t);
        for (hazard_index, h) in self.road.hazards.iter().enumerate() {
            let mut crossings =
                crossings_between(previous_distance, self.distance, h.s, self.road.length);
            if h.kind == HazardKind::Oncoming
                && helped
                    .iter()
                    .flatten()
                    .any(|traffic| traffic.hazard_index == hazard_index)
            {
                crossings = crossings.saturating_sub(1);
            }
            if crossings == 0 {
                continue;
            }
            let x_eff = self.hazard_x_at(h, 0.0, t) + self.road_x;
            if jump_active || cold_air_hop_active {
                if h.kind == HazardKind::Oncoming {
                    self.cars_passed = self.cars_passed.saturating_add(crossings);
                }
                continue;
            }
            match h.kind {
                HazardKind::Oncoming if x_eff.abs() < hit_band => {
                    self.cars_bumped = self.cars_bumped.saturating_add(crossings);
                    *obstacle_collision = true;
                    *obstacle_bump_dir = -x_eff.signum();
                }
                HazardKind::Oncoming => {
                    self.cars_passed = self.cars_passed.saturating_add(crossings);
                }
                HazardKind::Tree | HazardKind::Billboard if x_eff.abs() < hit_band => {
                    *obstacle_collision = true;
                    *obstacle_bump_dir = -x_eff.signum();
                }
                HazardKind::Tree | HazardKind::Billboard => {}
            }
        }
    }

    fn resolve_jump_landing(
        &mut self,
        t: &Tuning,
        obstacle_collision: &mut bool,
        obstacle_bump_dir: &mut f32,
    ) {
        let hit_band = (t.car_half_width + t.hazard_half_width) * (1.0 - t.collision_forgiveness);
        let pos = self.loop_pos();
        let mut landing: Option<(Hazard, f32, f32)> = None;
        for hazard in &self.road.hazards {
            let longitudinal = signed_loop_delta(hazard.s, pos, self.road.length);
            if longitudinal.abs() > t.jump_landing_window_m {
                continue;
            }
            let x_eff = self.hazard_x_at(hazard, longitudinal.max(0.0), t) + self.road_x;
            if x_eff.abs() >= hit_band {
                continue;
            }
            if landing.is_none_or(|(_, previous, _)| longitudinal.abs() < previous.abs()) {
                landing = Some((*hazard, longitudinal, x_eff));
            }
        }
        let Some((hazard, longitudinal, x_eff)) = landing else {
            return;
        };
        if hazard.kind == HazardKind::Oncoming {
            let absolute_hazard = self.distance + longitudinal;
            if absolute_hazard > self.jump_start_distance && absolute_hazard <= self.distance {
                self.cars_passed = self.cars_passed.saturating_sub(1);
            }
            self.cars_bumped = self.cars_bumped.saturating_add(1);
        }
        *obstacle_collision = true;
        *obstacle_bump_dir = -x_eff.signum();
    }

    fn finish_session(&mut self, ev: &mut StepEvents) {
        if self.result.is_some() {
            return;
        }
        self.session_remaining = 0.0;
        // The results/name-entry views reuse the gameplay scene. Always put
        // the player back on the road instead of freezing a mid-air pose when
        // the countdown expires during a Jump Kick.
        self.jump_t = 0.0;
        self.cold_air_hop_t = 0.0;
        self.cold_air_hop_lockout_t = 0.0;
        self.time_travel_boost = false;
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
        self.straightaway_charge = 0.0;
        self.road_x = (self.road_x + d * t.bump_lateral_kick)
            .clamp(-t.max_lateral_offset, t.max_lateral_offset);
        self.speed *= t.bump_speed_retention;
        // A collision immediately extinguishes the cold-air burn. Charges
        // were already consumed at ignition, and the normal cooldown still
        // applies so a crash cannot be used to retrigger the boost early.
        if self.nitro_t > 0.0 {
            self.nitro_t = 0.0;
            self.time_travel_boost = false;
            self.nitro_cooldown = self.nitro_cooldown.max(t.nitro_cooldown_secs);
            self.cold_air_hop_t = 0.0;
            self.cold_air_hop_lockout_t = 0.0;
        }
        self.recovery = match self.gear {
            Gear::High => Recovery::NeedLow,
            Gear::Low => Recovery::BuildSpeed,
        };
        self.flash = t.bump_flash_intensity;
        ev.bump_direction = Some(d);
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

fn signed_loop_delta(target: f32, origin: f32, loop_length: f32) -> f32 {
    (target - origin + loop_length * 0.5).rem_euclid(loop_length) - loop_length * 0.5
}

fn off_road_speed_factor(road_x: f32, t: &Tuning) -> f32 {
    let excursion = ((road_x.abs() - t.off_road_limit) / (t.max_lateral_offset - t.off_road_limit))
        .clamp(0.0, 1.0);
    let smooth = excursion * excursion * (3.0 - 2.0 * excursion);
    1.0 + (t.off_road_min_speed_factor - 1.0) * smooth
}

fn weather_kind_weight(weather: WeatherTransition, kind: WeatherKind) -> f32 {
    if weather.from == weather.to {
        return if weather.to == kind { 1.0 } else { 0.0 };
    }
    let from = if weather.from == kind {
        1.0 - weather.blend
    } else {
        0.0
    };
    let to = if weather.to == kind {
        weather.blend
    } else {
        0.0
    };
    from + to
}

fn precipitation_steer_factor(rain: f32, winter: f32, t: &Tuning) -> f32 {
    let rain = rain.clamp(0.0, 1.0);
    let winter = winter.clamp(0.0, 1.0);
    let wet_or_snow = t.rain_steer_factor * (1.0 - winter) + t.snow_steer_factor * winter;
    1.0 + (wet_or_snow - 1.0) * rain
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

    #[test]
    fn precipitation_subtly_reduces_grip_with_snow_slipperiest() {
        let t = Tuning::default();
        assert_eq!(precipitation_steer_factor(0.0, 0.0, &t), 1.0);
        assert_eq!(precipitation_steer_factor(1.0, 0.0, &t), 0.94);
        assert_eq!(precipitation_steer_factor(1.0, 1.0, &t), 0.90);
        let transition = precipitation_steer_factor(0.5, 0.0, &t);
        assert!((transition - 0.97).abs() < 1e-6);
    }

    fn running_jump_game(hazard: Hazard) -> (Game, Tuning) {
        let t = Tuning::default();
        let mut game = Game::with_road(
            17,
            RoadLoop::straight(100.0, vec![hazard]),
            Difficulty::Hard,
            &t,
        );
        game.phase = Phase::Running;
        game.speed = t.scroll_speed_high;
        game.gear = Gear::High;
        game.jump_charge = true;
        (game, t)
    }

    #[test]
    fn jump_pickup_schedule_is_rare_and_never_duplicates_a_cycle() {
        let t = Tuning::default();
        let game = Game::new(71, Difficulty::Hard, &t);
        let mut present = 0;
        for cycle in 0..32 {
            if let Some((distance, lane)) = game.jump_pickup_for_cycle(cycle, &t) {
                present += 1;
                let environment_time = distance / t.scroll_speed_low;
                assert_eq!((environment_time / DAY_SECS).floor() as u64, cycle);
                assert!([-t.jump_lane_offset, 0.0, t.jump_lane_offset].contains(&lane));
            }
        }
        assert!(
            (8..=24).contains(&present),
            "rare schedule count: {present}"
        );
    }

    #[test]
    fn jump_pickups_become_progressively_rarer_by_difficulty() {
        let t = Tuning::default();
        let scheduled = |difficulty| {
            let game = Game::new(71, difficulty, &t);
            (1..=2_000)
                .filter(|&cycle| game.jump_pickup_for_cycle(cycle, &t).is_some())
                .count()
        };
        let easy = scheduled(Difficulty::Easy);
        let medium = scheduled(Difficulty::Medium);
        let hard = scheduled(Difficulty::Hard);
        assert!(easy > medium && medium > hard, "{easy} > {medium} > {hard}");
    }

    #[test]
    fn easy_guarantees_its_first_jump_pickup_inside_the_first_minute() {
        let t = Tuning::default();
        for seed in 0..128 {
            let game = Game::new(seed, Difficulty::Easy, &t);
            let distance = game
                .next_jump_distance
                .expect("Easy must always schedule a cycle-zero tutorial pickup");
            let cruise_secs = distance / (t.scroll_speed_low * t.speed_easy_multiplier);
            assert!(
                cruise_secs < 45.0,
                "seed {seed} scheduled at {cruise_secs}s"
            );
            assert_eq!(game.jump_pickup_cycle, 0);
        }
    }

    #[test]
    fn jump_pickup_capacity_is_one_and_activation_consumes_it() {
        let t = Tuning {
            jump_spawn_chance_hard_percent: 100,
            ..Tuning::default()
        };
        let mut game = Game::with_road(5, RoadLoop::straight(100.0, vec![]), Difficulty::Hard, &t);
        game.phase = Phase::Running;
        game.next_jump_distance = Some(0.0);
        game.next_jump_lane = 0.0;
        let collected = game.step(&InputState::default(), DT, &t);
        assert!(collected.jump_collected);
        assert!(game.jump_charge);

        let started = game.step(
            &InputState {
                jump: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert!(started.jump_started);
        assert!(!game.jump_charge);
        assert!(game.jump_active());
    }

    #[test]
    fn pickup_hitboxes_accept_a_small_visible_edge_overlap() {
        let (mut game, tuning) = game(50);
        game.road_x = 0.0;

        let nitro_edge = tuning.car_half_width + tuning.nitro_pickup_half_width;
        assert!(game.overlaps_pickup(nitro_edge - 0.001, tuning.nitro_pickup_half_width, &tuning));
        assert!(!game.overlaps_pickup(nitro_edge + 0.001, tuning.nitro_pickup_half_width, &tuning));

        let jump_edge = tuning.car_half_width + tuning.jump_pickup_half_width;
        assert!(game.overlaps_pickup(jump_edge - 0.001, tuning.jump_pickup_half_width, &tuning));
        assert!(!game.overlaps_pickup(jump_edge + 0.001, tuning.jump_pickup_half_width, &tuning));
        assert!(tuning.nitro_pickup_half_width > 0.20);
        assert!(tuning.jump_pickup_half_width > 0.20);
    }

    #[test]
    fn pickup_placement_moves_clear_of_three_wide_traffic() {
        let tuning = Tuning::default();
        let hazards = [-tuning.nitro_lane_offset, 0.0, tuning.nitro_lane_offset]
            .map(|x| Hazard {
                kind: HazardKind::Oncoming,
                x,
                s: tuning.nitro_pickup_first_m,
            })
            .to_vec();
        let game = Game::with_road(
            81,
            RoadLoop::straight(200.0, hazards),
            Difficulty::Hard,
            &tuning,
        );

        assert!(game.next_nitro_distance > tuning.nitro_pickup_first_m);
        for hazard in &game.road.hazards {
            let delta = signed_loop_delta(
                hazard.s,
                game.next_nitro_distance.rem_euclid(game.road.length),
                game.road.length,
            );
            let separated_laterally = (hazard.x - game.next_nitro_lane).abs()
                >= tuning.nitro_pickup_half_width + tuning.hazard_half_width;
            assert!(delta.abs() > tuning.pickup_traffic_clearance_m || separated_laterally);
        }
    }

    #[test]
    fn jumping_over_traffic_is_a_pass_but_landing_on_it_is_a_bump() {
        let landing_hazard = Hazard {
            kind: HazardKind::Oncoming,
            x: 0.0,
            s: 28.1,
        };
        let (mut landing, t) = running_jump_game(landing_hazard);
        let ev = landing.step(
            &InputState {
                jump: true,
                ..Default::default()
            },
            t.jump_duration_secs,
            &t,
        );
        assert!(ev.jump_started && ev.jump_landed);
        assert!(ev.bump_direction.is_some());
        assert_eq!(landing.cars_passed, 0);
        assert_eq!(landing.cars_bumped, 1);

        let safe_hazard = Hazard {
            x: t.jump_lane_offset,
            ..landing_hazard
        };
        let (mut safe, t) = running_jump_game(safe_hazard);
        let ev = safe.step(
            &InputState {
                jump: true,
                ..Default::default()
            },
            t.jump_duration_secs,
            &t,
        );
        assert!(ev.jump_landed);
        assert!(ev.bump_direction.is_none());
        assert_eq!(safe.cars_passed, 1);
        assert_eq!(safe.cars_bumped, 0);
    }

    #[test]
    fn landing_on_a_roadside_hazard_uses_the_normal_crash_recovery() {
        let tree = Hazard {
            kind: HazardKind::Tree,
            x: 0.0,
            s: 28.1,
        };
        let (mut game, t) = running_jump_game(tree);
        let ev = game.step(
            &InputState {
                jump: true,
                ..Default::default()
            },
            t.jump_duration_secs,
            &t,
        );
        assert!(ev.jump_landed);
        assert!(ev.bump_direction.is_some());
        assert!(game.bump_t > 0.0);
        assert_eq!(game.cars_bumped, 0);
        assert_eq!(game.recovery, Recovery::NeedLow);
    }

    #[test]
    fn jump_without_a_charge_is_a_no_op() {
        let (mut game, t) = game(73);
        game.phase = Phase::Running;
        let before = game.distance;
        let ev = game.step(
            &InputState {
                jump: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert!(!ev.jump_started);
        assert!(!game.jump_active());
        assert!(game.distance >= before);
    }

    #[test]
    fn time_up_during_a_jump_restores_the_resting_pose() {
        let (mut game, t) = game(74);
        game.phase = Phase::Running;
        game.jump_charge = true;
        game.session_remaining = DT * 0.5;
        let ev = game.step(
            &InputState {
                jump: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert!(ev.jump_started && ev.game_over);
        assert_eq!(game.jump_t, 0.0);
        assert!(!game.jump_active());
    }

    #[test]
    fn jump_timing_is_timestep_partition_invariant() {
        let hazard = Hazard {
            kind: HazardKind::Oncoming,
            x: 0.72,
            s: 28.1,
        };
        let (mut whole, t) = running_jump_game(hazard);
        let mut split = whole.clone();
        whole.step(
            &InputState {
                jump: true,
                ..Default::default()
            },
            t.jump_duration_secs,
            &t,
        );
        for index in 0..66 {
            split.step(
                &InputState {
                    jump: index == 0,
                    ..Default::default()
                },
                t.jump_duration_secs / 66.0,
                &t,
            );
        }
        assert!((whole.distance - split.distance).abs() < 1e-3);
        assert_eq!(whole.cars_passed, split.cars_passed);
        assert_eq!(whole.cars_bumped, split.cars_bumped);
        assert_eq!(whole.jump_t, split.jump_t);
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
    fn low_gear_launch_is_gentler_than_the_high_gear_ramp() {
        let t = Tuning::default();
        let mut gentler = Game::with_road(
            2,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &t,
        );
        gentler.start_run(&t);
        let mut original = gentler.clone();
        let original_tuning = Tuning {
            low_gear_ramp_secs: t.gear_ramp_secs,
            ..t
        };
        run_seconds(&mut gentler, 0.5, &idle(), &t);
        run_seconds(&mut original, 0.5, &idle(), &original_tuning);
        assert!(gentler.speed < original.speed);
        assert!(gentler.speed > original.speed * 0.9);
    }

    #[test]
    fn sustained_straight_high_gear_creeps_past_88_on_the_unchanged_dial() {
        let t = Tuning::default();
        let mut game = Game::with_road(
            2,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &t,
        );
        game.start_run(&t);
        game.gear = Gear::High;
        game.speed = game.high_speed_target(&t);
        assert!(game.speed * 3.6 < 88.0);

        run_seconds(&mut game, 4.0, &idle(), &t);

        assert!(game.straightaway_overdrive_ready(&t));
        assert!(game.speed * 3.6 > 88.0);
        assert!(game.speed * 3.6 < 91.0);
    }

    #[test]
    fn steering_prevents_straightaway_overdrive() {
        let t = Tuning::default();
        let mut game = Game::with_road(
            2,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &t,
        );
        game.start_run(&t);
        game.gear = Gear::High;
        game.speed = game.high_speed_target(&t);
        run_seconds(
            &mut game,
            4.0,
            &InputState {
                steer: 0.25,
                ..idle()
            },
            &t,
        );
        assert!(!game.straightaway_overdrive_ready(&t));
        assert!(game.speed * 3.6 < 88.0);
    }

    #[test]
    fn difficulty_scales_speed_and_gear_thresholds_together() {
        let t = Tuning::default();
        let mut speeds = [0.0; 3];
        let mut distances = [0.0; 3];
        for (index, difficulty) in Difficulty::ALL.into_iter().enumerate() {
            let mut game = Game::with_road(2, RoadLoop::straight(10_000.0, vec![]), difficulty, &t);
            game.start_run(&t);
            run_seconds(&mut game, 5.0, &idle(), &t);
            speeds[index] = game.speed;
            distances[index] = game.distance;

            game.speed = game.shift_up_speed(&t);
            let event = game.step(
                &InputState {
                    gear_request: Some(Gear::High),
                    ..idle()
                },
                0.0,
                &t,
            );
            assert!(!event.high_rejected);
            assert_eq!(game.gear(), Gear::High);
        }

        assert!((speeds[0] / speeds[2] - 0.8).abs() < 0.001);
        assert!((speeds[1] / speeds[2] - 0.9).abs() < 0.001);
        assert!((distances[0] / distances[2] - 0.8).abs() < 0.001);
        assert!((distances[1] / distances[2] - 0.9).abs() < 0.001);
    }

    #[test]
    fn gentle_grade_resists_uphill_and_assists_downhill_speed() {
        let t = Tuning::default();
        let mut road = RoadLoop::generate(&mut crate::Rng::new(204));
        road.hazards.clear();
        let positions: Vec<f32> = (0..crate::road::CURVE_SAMPLES)
            .map(|i| road.length * i as f32 / crate::road::CURVE_SAMPLES as f32)
            .collect();
        let uphill_pos = *positions
            .iter()
            .max_by(|a, b| road.grade_at(**a).total_cmp(&road.grade_at(**b)))
            .unwrap();
        let downhill_pos = *positions
            .iter()
            .min_by(|a, b| road.grade_at(**a).total_cmp(&road.grade_at(**b)))
            .unwrap();
        assert!(road.grade_at(uphill_pos) > 0.01);
        assert!(road.grade_at(downhill_pos) < -0.01);

        let mut uphill = Game::with_road(204, road.clone(), Difficulty::Hard, &t);
        let mut downhill = Game::with_road(204, road, Difficulty::Hard, &t);
        for (game, position) in [(&mut uphill, uphill_pos), (&mut downhill, downhill_pos)] {
            game.start_run(&t);
            game.distance = position;
            game.speed = t.scroll_speed_low;
            game.step(&idle(), 0.25, &t);
        }
        assert!(uphill.speed < t.scroll_speed_low);
        assert!(downhill.speed > t.scroll_speed_low);
        assert!(
            downhill.speed - uphill.speed < 1.0,
            "hills should stay subtle"
        );
    }

    #[test]
    fn active_cold_air_launches_once_over_a_crest() {
        let t = Tuning::default();
        let mut road = RoadLoop::generate(&mut crate::Rng::new(912));
        road.hazards.clear();
        let crest = (0..crate::road::CURVE_SAMPLES * 4)
            .map(|i| road.length * i as f32 / (crate::road::CURVE_SAMPLES * 4) as f32)
            .find(|position| {
                road.grade_at(*position) >= t.cold_air_hop_min_grade
                    && road.grade_at(*position + t.cold_air_hop_lookahead_m) <= 0.0
            })
            .expect("generated rolling road should contain a launchable crest");
        let mut game = Game::with_road(912, road, Difficulty::Hard, &t);
        game.start_run(&t);
        game.distance = crest;
        game.speed = t.scroll_speed_low;
        game.nitro_t = 2.0;

        let event = game.step(&idle(), DT, &t);
        assert!(event.cold_air_hop_started);
        assert!(game.cold_air_hop_active());
        assert!(game.cold_air_hop_progress(&t) > 0.0);

        let mut starts = 1;
        for _ in 0..120 {
            starts += usize::from(game.step(&idle(), DT, &t).cold_air_hop_started);
        }
        assert_eq!(starts, 1, "one broad crest must not retrigger the launch");
    }

    #[test]
    fn crest_does_not_launch_without_cold_air() {
        let t = Tuning::default();
        let mut road = RoadLoop::generate(&mut crate::Rng::new(913));
        road.hazards.clear();
        let crest = (0..crate::road::CURVE_SAMPLES * 4)
            .map(|i| road.length * i as f32 / (crate::road::CURVE_SAMPLES * 4) as f32)
            .find(|position| {
                road.grade_at(*position) >= t.cold_air_hop_min_grade
                    && road.grade_at(*position + t.cold_air_hop_lookahead_m) <= 0.0
            })
            .unwrap();
        let mut game = Game::with_road(913, road, Difficulty::Hard, &t);
        game.start_run(&t);
        game.distance = crest;
        game.speed = t.scroll_speed_low;
        assert!(!game.step(&idle(), DT, &t).cold_air_hop_started);
        assert!(!game.cold_air_hop_active());
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
    fn sustained_off_road_driving_slows_without_crashing() {
        let t = Tuning::default();
        let mut g = Game::with_road(40, RoadLoop::straight(500.0, vec![]), Difficulty::Hard, &t);
        g.start_run(&t);
        g.speed = t.scroll_speed_low;
        for _ in 0..120 {
            g.road_x = t.max_lateral_offset;
            let event = g.step(&idle(), DT, &t);
            assert!(event.bump_direction.is_none());
            assert_eq!(g.flash, 0.0);
            assert_eq!(g.bump_t, 0.0);
        }
        assert!(g.speed < t.scroll_speed_low * 0.7);
        assert_eq!(g.recovery(), Recovery::Normal);
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
            if ev.bump_direction.is_some() {
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
            assert!(
                ev.bump_direction.is_none(),
                "side-lane hazard with no steering must miss"
            );
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
                ev.bump_direction.is_none(),
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
    fn returning_to_menu_clears_the_run_and_stays_ready() {
        let (mut game, tuning) = game(241);
        game.start_run(&tuning);
        game.speed = 18.0;
        game.distance = 321.0;
        game.environment_time = 77.0;
        game.day_cycles = 3;
        game.cars_passed = 9;
        game.nitro_charges = 3;
        game.nitro_t = 1.5;
        game.paused = true;

        game.return_to_ready(&tuning);

        assert_eq!(game.phase, Phase::Ready);
        assert!(!game.paused);
        assert_eq!(game.speed, 0.0);
        assert_eq!(game.distance, 0.0);
        assert_eq!(game.environment_time, 0.0);
        assert_eq!(game.day_cycles, 0);
        assert_eq!(game.cars_passed, 0);
        assert_eq!(game.nitro_charges, 0);
        assert_eq!(game.nitro_t, 0.0);
        assert_eq!(game.session_remaining, game.session_duration());

        let events = game.step(&InputState::default(), 1.0, &tuning);
        assert_eq!(game.phase, Phase::Ready);
        assert!(!events.started);
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
        run_seconds(&mut high, 1.4, &idle(), &t);
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
            assert_eq!(ev.nitro_collected_segment, Some(expected - 1));
            assert_eq!(g.nitro_charges, expected);
        }

        g.next_nitro_distance = g.distance;
        g.road_x = -g.next_nitro_lane;
        let ev = g.step(&idle(), DT, &t);
        assert!(
            ev.nitro_collected_segment.is_none(),
            "a full meter cannot overfill"
        );
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
        assert!(ev.nitro_collected_segment.is_none());
        assert_eq!(g.nitro_charges, 0);
        assert!((g.next_nitro_distance - (old_distance + t.nitro_pickup_interval_m)).abs() < 1e-3);
    }

    #[test]
    fn nitro_requires_full_meter_consumes_all_cells_and_expires() {
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
        let early = boosted.step(
            &InputState {
                nitro: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert!(!early.nitro_started);
        assert_eq!(boosted.nitro_charges, 2);
        boosted.nitro_charges = t.nitro_capacity;
        let ev = boosted.step(
            &InputState {
                nitro: true,
                ..Default::default()
            },
            DT,
            &t,
        );
        assert!(ev.nitro_started);
        assert_eq!(boosted.nitro_charges, 0);
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
    fn every_fourth_cold_air_activation_advances_the_hidden_time_travel_sequence() {
        let t = Tuning::default();
        let mut g = Game::with_road(
            34,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &t,
        );
        g.start_run(&t);

        for use_number in 1..=8 {
            g.nitro_t = 0.0;
            g.nitro_cooldown = 0.0;
            g.nitro_charges = t.nitro_capacity;
            let events = g.step(
                &InputState {
                    nitro: true,
                    ..idle()
                },
                DT,
                &t,
            );
            assert!(events.nitro_started);
            assert_eq!(g.time_travel_sequence(), use_number / 4);
        }

        g.start_run(&t);
        assert_eq!(
            g.time_travel_sequence(),
            0,
            "a fresh run resets the cadence"
        );
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
        g.jump_charge = true;
        g.jump_t = 0.5;
        g.next_jump_distance = None;
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
        assert!(!g.jump_charge);
        assert_eq!(g.jump_t, 0.0);
        assert!(g.next_jump_distance.is_some());
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
            if g.step(&idle(), DT, &t).nitro_collected_segment.is_some() {
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
    fn fourth_cold_air_boost_can_cross_88_even_when_normal_easy_boost_cannot() {
        let mut t = Tuning {
            speed_easy_multiplier: 0.60,
            nitro_speed_multiplier: 1.05,
            ..Tuning::default()
        };
        // Keep the reward floor itself at the production value.
        t.straightaway_high_speed_cap = 89.5 / 3.6;
        let mut g = Game::with_road(
            35,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Easy,
            &t,
        );
        g.start_run(&t);

        for _ in 0..3 {
            g.nitro_charges = t.nitro_capacity;
            assert!(
                g.step(
                    &InputState {
                        nitro: true,
                        ..idle()
                    },
                    DT,
                    &t,
                )
                .nitro_started
            );
            g.nitro_t = 0.0;
            g.nitro_cooldown = 0.0;
        }
        g.speed = 0.0;
        g.nitro_charges = t.nitro_capacity;
        assert!(
            g.step(
                &InputState {
                    nitro: true,
                    ..idle()
                },
                DT,
                &t,
            )
            .nitro_started
        );
        g.step(&idle(), t.nitro_duration_secs - DT, &t);

        assert!(g.speed * 3.6 > 88.0);
        assert_eq!(g.time_travel_sequence(), 1);
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
    fn oncoming_crossings_are_mutually_exclusive_and_scenery_is_not_a_car() {
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
                kind: HazardKind::Tree,
                x: 1.15,
                s: 1.0,
            },
        ];
        let mut g = Game::with_road(53, RoadLoop::straight(10.0, hazards), Difficulty::Hard, &t);
        g.start_run(&t);
        g.speed = t.scroll_speed_low;
        let ev = g.step(&idle(), 0.1, &t);
        assert!(ev.bump_direction.is_some());
        assert_eq!(g.cars_bumped, 1);
        assert_eq!(g.cars_passed, 1);
        assert_eq!(g.cars_bumped + g.cars_passed, 2);
    }

    #[test]
    fn roadside_scenery_uses_crash_recovery_without_changing_car_counts() {
        let t = Tuning::default();
        let hazard = Hazard {
            kind: HazardKind::Billboard,
            x: 1.15,
            s: 1.0,
        };
        let mut g = Game::with_road(
            530,
            RoadLoop::straight(20.0, vec![hazard]),
            Difficulty::Hard,
            &t,
        );
        g.start_run(&t);
        g.speed = t.scroll_speed_low;
        g.road_x = -1.15;
        let event = g.step(&idle(), 0.1, &t);
        assert!(event.bump_direction.is_some());
        assert_eq!(g.cars_bumped, 0);
        assert_eq!(g.cars_passed, 0);
        assert_eq!(g.recovery(), Recovery::BuildSpeed);
        assert!(g.speed < t.scroll_speed_low * 0.6);
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
    fn car_collision_still_crashes_while_player_is_off_road() {
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
        let ev = g.step(&idle(), 0.1, &t);
        assert!(ev.bump_direction.is_some());
        assert_eq!(g.cars_bumped, 1);
        assert!(
            g.bump_dir < 0.0,
            "car direction must win over positive off-road side"
        );
    }

    #[test]
    fn score_clamps_and_has_expected_ordering_effects() {
        assert_eq!(score_for(0, 0, 1), 0);
        assert_eq!(score_for(1, 3, 1), 10_100);
        assert_eq!(
            score_for(1, 1, 0) - score_for(1, 0, 1),
            CLEAN_PASS_SCORE + BUMP_PENALTY,
            "a collision should swing three clean-pass values, before speed loss"
        );
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
        let t = Tuning::default();
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
        assert!(same.gear_changed.is_none());
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
        assert_eq!(
            ev.gear_changed,
            Some(GearChange {
                gear: Gear::Low,
                downshift_severity: Some(DownshiftSeverity::Normal),
            })
        );
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
        assert_eq!(
            ev.gear_changed,
            Some(GearChange {
                gear: Gear::Low,
                downshift_severity: Some(DownshiftSeverity::Unsafe),
            })
        );
        assert!((unsafe_shift.speed - 18.0 * 0.82).abs() < 0.001);
        assert!(unsafe_shift.low_overrev(&t) > 0.0);
    }

    #[test]
    fn analog_brake_slows_gradually_and_release_restores_auto_acceleration() {
        let t = Tuning::default();
        let mut g = Game::with_road(
            63,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &t,
        );
        g.start_run(&t);
        g.speed = t.scroll_speed_low;

        run_seconds(
            &mut g,
            1.0,
            &InputState {
                brake: 0.5,
                ..idle()
            },
            &t,
        );
        let braked = g.speed;
        assert!(braked > 6.0 && braked < t.scroll_speed_low);

        run_seconds(&mut g, 1.0, &idle(), &t);
        assert!(g.speed > braked, "releasing LT must resume acceleration");
        assert_eq!(g.recovery(), Recovery::Normal);
    }

    #[test]
    fn braking_trades_distance_for_a_wider_avoidance_window() {
        let t = Tuning::default();
        let road = RoadLoop::straight(
            500.0,
            vec![Hazard {
                kind: HazardKind::Oncoming,
                x: 0.0,
                s: 18.0,
            }],
        );
        let run = |steer: f32, brake: f32| {
            let mut game = Game::with_road(70, road.clone(), Difficulty::Hard, &t);
            game.start_run(&t);
            game.gear = Gear::High;
            game.speed = t.scroll_speed_high;
            run_seconds(
                &mut game,
                2.0,
                &InputState {
                    steer,
                    brake,
                    ..idle()
                },
                &t,
            );
            game
        };

        let cautious = run(0.22, 0.55);
        let precise = run(0.32, 0.0);
        let failed_risk = run(0.22, 0.0);
        assert_eq!((cautious.cars_passed, cautious.cars_bumped), (1, 0));
        assert_eq!((precise.cars_passed, precise.cars_bumped), (1, 0));
        assert_eq!((failed_risk.cars_passed, failed_risk.cars_bumped), (0, 1));
        assert!(
            precise.distance > cautious.distance * 1.15,
            "a clean no-brake dodge should preserve meaningfully more race distance"
        );
        assert!(failed_risk.speed < cautious.speed);
    }

    #[test]
    fn braking_too_slow_in_high_requires_low_gear_recovery() {
        let t = Tuning::default();
        let mut g = Game::with_road(
            64,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &t,
        );
        g.start_run(&t);
        g.gear = Gear::High;
        g.speed = t.scroll_speed_high;

        run_seconds(
            &mut g,
            2.0,
            &InputState {
                brake: 1.0,
                ..idle()
            },
            &t,
        );
        assert_eq!(g.gear(), Gear::High);
        assert_eq!(g.recovery(), Recovery::NeedLow);

        run_seconds(&mut g, 1.0, &idle(), &t);
        assert!(g.speed <= t.recovery_bog_speed * 1.05);
        g.step(
            &InputState {
                gear_request: Some(Gear::Low),
                ..idle()
            },
            DT,
            &t,
        );
        assert_eq!(g.gear(), Gear::Low);
        assert_eq!(g.recovery(), Recovery::BuildSpeed);
    }

    #[test]
    fn bump_recovery_requires_low_then_speed_then_high() {
        let (mut g, t) = game(63);
        g.start_run(&t);
        g.gear = Gear::High;
        g.speed = 20.0;
        let mut bump_ev = StepEvents::default();
        g.bump(&mut bump_ev, &t, 1.0);
        assert!(bump_ev.bump_direction.is_some());
        assert_eq!(g.recovery(), Recovery::NeedLow);
        let after_bump = g.speed;
        g.step(&idle(), 0.5, &t);
        assert!(g.speed < after_bump, "HIGH must bog until LOW is selected");
        g.nitro_charges = t.nitro_capacity;
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
    fn crash_extinguishes_active_cold_air_and_starts_cooldown() {
        let (mut game, tuning) = game(630);
        game.start_run(&tuning);
        game.nitro_t = 2.5;
        game.cold_air_hop_t = 0.4;
        let mut events = StepEvents::default();

        game.bump(&mut events, &tuning, 1.0);

        assert_eq!(game.nitro_t, 0.0);
        assert_eq!(game.cold_air_hop_t, 0.0);
        assert_eq!(game.nitro_cooldown, tuning.nitro_cooldown_secs);
        assert_eq!(events.bump_direction, Some(1.0));
    }

    #[test]
    fn randomized_environment_start_does_not_pre_score_cycles() {
        let tuning = Tuning::default();
        let mut game = Game::with_road(
            631,
            RoadLoop::straight(10_000.0, vec![]),
            Difficulty::Hard,
            &tuning,
        );
        game.start_run(&tuning);
        let start = crate::environment::SEASON_SECS * 3.5;
        game.set_environment_start(start);

        assert_eq!(game.environment_time, start);
        assert_eq!(game.day_cycles, 0);
        assert_eq!(game.environment_cycle_index(), 10);

        game.speed = tuning.scroll_speed_low;
        run_seconds(
            &mut game,
            crate::environment::DAY_SECS + 0.1,
            &idle(),
            &tuning,
        );
        assert_eq!(game.day_cycles, 1);
        assert!(game.environment_cycle_index() >= 11);
    }

    #[test]
    fn maximum_speed_crash_drops_below_dial_midpoint_and_requires_low() {
        let (mut g, t) = game(65);
        g.start_run(&t);
        g.gear = Gear::High;
        g.speed = t.scroll_speed_high;
        let mut ev = StepEvents::default();

        g.bump(&mut ev, &t, -1.0);

        assert!(ev.bump_direction.is_some());
        assert!(g.speed * 3.6 < 60.0, "crash must fall below 60 km/h");
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
        g.nitro_charges = t.nitro_capacity;
        let started = g.step(
            &InputState {
                nitro: true,
                ..idle()
            },
            0.1,
            &t,
        );
        assert!(started.nitro_started);
        assert_eq!(g.nitro_charges, 0);
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
        assert_eq!(g.nitro_charges, 0);

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
        g.nitro_charges = t.nitro_capacity;
        let restarted = g.step(
            &InputState {
                nitro: true,
                ..idle()
            },
            DT,
            &t,
        );
        assert!(restarted.nitro_started);
        assert_eq!(g.nitro_charges, 0);

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
        g.nitro_charges = t.nitro_capacity;

        let ev = g.step(
            &InputState {
                nitro: true,
                ..idle()
            },
            t.nitro_duration_secs,
            &t,
        );
        let target = t.scroll_speed_low * t.nitro_speed_multiplier;
        let decay = (-t.nitro_duration_secs / t.low_gear_ramp_secs).exp();
        let expected_speed = target + (t.scroll_speed_low - target) * decay;
        let expected_distance = target * t.nitro_duration_secs
            + (t.scroll_speed_low - target) * t.low_gear_ramp_secs * (1.0 - decay);

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
            game.nitro_charges = t.nitro_capacity;
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

    #[test]
    fn harder_difficulties_enable_more_lane_changes_in_the_horizon_band() {
        let t = Tuning::default();
        assert_eq!(t.traffic_easy.chance_percent, 14);
        assert_eq!(t.traffic_medium.chance_percent, 25);
        assert_eq!(t.traffic_hard.chance_percent, 35);
        let mut totals = [0usize; 3];
        for seed in 1..=24 {
            for (index, difficulty) in Difficulty::ALL.into_iter().enumerate() {
                let game = Game::new(seed, difficulty, &t);
                totals[index] += game
                    .road
                    .hazards
                    .iter()
                    .filter(|hazard| {
                        hazard.kind == HazardKind::Oncoming
                            && game.hazard_x_at(hazard, 0.0, &t) != hazard.x
                    })
                    .count();
            }
        }
        assert!(totals[0] < totals[1], "Easy must change lanes less often");
        assert!(totals[1] < totals[2], "Hard must change lanes more often");
        for motion in [t.traffic_easy, t.traffic_medium, t.traffic_hard] {
            assert!(motion.change_end_m >= 18.0);
            assert!(motion.change_start_m - motion.change_end_m >= 16.0);
        }
    }

    #[test]
    fn medium_and_hard_add_only_a_small_amount_of_traffic() {
        let t = Tuning::default();
        let traffic_count = |difficulty| {
            Game::new(314, difficulty, &t)
                .road
                .hazards
                .iter()
                .filter(|hazard| hazard.kind == HazardKind::Oncoming)
                .count()
        };
        let easy = traffic_count(Difficulty::Easy);
        let medium = traffic_count(Difficulty::Medium);
        let hard = traffic_count(Difficulty::Hard);
        assert_eq!(medium, easy + usize::from(t.traffic_medium_extra));
        assert_eq!(hard, easy + usize::from(t.traffic_hard_extra));
        assert!(hard <= easy + 2, "difficulty increase must stay subtle");
    }

    fn rare_help_test_game(seed: u64) -> (Game, Tuning, u64) {
        let tuning = Tuning::default();
        let hazards = (1..20)
            .map(|index| Hazard {
                kind: HazardKind::Oncoming,
                x: 0.0,
                s: index as f32 * 10.0,
            })
            .collect();
        let game = Game::with_road(
            seed,
            RoadLoop::straight(200.0, hazards),
            Difficulty::Hard,
            &tuning,
        );
        let salt = game.rare_event_salt();
        (game, tuning, salt)
    }

    #[test]
    fn hurricane_removes_one_or_two_deterministic_approaching_cars() {
        let (mut game, tuning, salt) = rare_help_test_game(71);
        let slot = (0..4096)
            .find(|slot| {
                crate::rare_event::kind_for_slot(*slot, salt)
                    == Some(crate::rare_event::RareEventKind::Hurricane)
            })
            .expect("schedule should contain a hurricane");
        game.phase = Phase::Running;
        game.environment_time = slot as f32 * crate::rare_event::SLOT_SECS + 0.25;
        game.distance = game.environment_time * tuning.scroll_speed_low;
        let first = game.rare_traffic_help(&tuning);
        let second = game.rare_traffic_help(&tuning);
        assert_eq!(first, second);
        assert!((1..=2).contains(&first.iter().flatten().count()));
        assert!(first.iter().flatten().all(|car| car.start_ahead <= 28.0));
    }

    #[test]
    fn ufo_removes_exactly_one_car_and_the_assist_does_not_score_or_crash() {
        let (mut game, tuning, salt) = rare_help_test_game(73);
        let slot = (0..4096)
            .find(|slot| {
                crate::rare_event::kind_for_slot(*slot, salt)
                    == Some(crate::rare_event::RareEventKind::Ufo)
                    && crate::environment::daylight(
                        *slot as f32 * crate::rare_event::SLOT_SECS + 1.0,
                    ) < 0.30
            })
            .expect("schedule should contain a nighttime UFO");
        game.phase = Phase::Running;
        game.environment_time = slot as f32 * crate::rare_event::SLOT_SECS + 0.25;
        game.distance = game.environment_time * tuning.scroll_speed_low;
        let helped = game.rare_traffic_help(&tuning);
        assert_eq!(helped.iter().flatten().count(), 1);
        let traffic = helped[0].expect("UFO should choose one approaching car");
        assert!(traffic.start_ahead <= 28.0);

        let event_start = slot as f32 * crate::rare_event::SLOT_SECS * tuning.scroll_speed_low;
        game.distance = event_start + traffic.start_ahead - 0.05;
        game.environment_time = game.distance / tuning.scroll_speed_low;
        game.speed = tuning.scroll_speed_low;
        game.session_remaining = 60.0;
        let events = game.step(&idle(), 0.02, &tuning);
        assert_eq!((game.cars_passed, game.cars_bumped), (0, 0));
        assert_eq!(events.bump_direction, None);
    }
}
