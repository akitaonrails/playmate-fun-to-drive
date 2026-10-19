//! Typed, throttled browser telemetry for QA and automation.
//!
//! Native builds install a zero-work adapter. Web builds first confirm that
//! the JavaScript hook exists, then publish at a bounded cadence instead of
//! allocating a large JSON document every render frame.

use bevy::prelude::*;

use crate::AppSet;

#[cfg(any(target_arch = "wasm32", test))]
const PUBLISH_INTERVAL_SECS: f32 = 0.10;

pub(crate) fn add_systems(app: &mut App) {
    app.add_systems(Update, publish_state.in_set(AppSet::Telemetry));
}

#[cfg(not(target_arch = "wasm32"))]
fn publish_state() {}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct PublishClock {
    elapsed: f32,
    hook_available: Option<bool>,
}

#[cfg(target_arch = "wasm32")]
#[derive(serde::Serialize)]
struct ResultRow {
    day_cycles: u64,
    cars_passed: u64,
    cars_bumped: u64,
    score: u64,
}

#[cfg(target_arch = "wasm32")]
#[derive(serde::Serialize)]
struct LeaderboardRow<'a> {
    initials: &'a str,
    difficulty: &'static str,
    day_cycles: u64,
    cars_passed: u64,
    cars_bumped: u64,
    score: u64,
}

#[cfg(target_arch = "wasm32")]
#[derive(serde::Serialize)]
struct Snapshot<'a> {
    phase: &'static str,
    end_mode: &'static str,
    difficulty: &'static str,
    difficulty_index: usize,
    initials: &'a str,
    leaderboard: Vec<LeaderboardRow<'a>>,
    leaderboard_persistent: bool,
    qa_mode: bool,
    session_duration: f32,
    score: u64,
    speed: f32,
    speed_kmh: f32,
    speedometer_angle: f32,
    overrev: f32,
    recovery: &'static str,
    action_cue: &'static str,
    nitro_cooldown: f32,
    cooldown_gauge: f32,
    session_remaining: f32,
    environment_time: f32,
    day_cycles: u64,
    cars_passed: u64,
    cars_bumped: u64,
    session_result: Option<ResultRow>,
    weather_from: &'static str,
    weather_to: &'static str,
    weather_transition: f32,
    weather_slot: u64,
    weather_clear_visual: f32,
    weather_cloudy_visual: f32,
    weather_rain_visual: f32,
    weather_rain_visible: f32,
    daylight: f32,
    season: f32,
    gear: &'static str,
    lever_target: f32,
    lever_angle: f32,
    nitro: u8,
    nitro_t: f32,
    nitro_ahead: f32,
    nitro_lane: f32,
    nitro_glow: f32,
    nitro_glow_segment: Option<u8>,
    nitro_visual_intensity: f32,
    jump_charge: bool,
    jump_t: f32,
    jump_progress: f32,
    jump_ahead: Option<f32>,
    jump_lane: f32,
    toy_root_z: f32,
    toy_root_pitch: f32,
    toy: bool,
    look: f32,
    crt_enabled: bool,
    paused: bool,
    steer: f32,
    brake: f32,
    road_x: f32,
    road_elevation: f32,
    road_grade: f32,
    slope_speed_factor: f32,
    off_road: f32,
    roll: f32,
    pad: bool,
    audio_unlocked: bool,
    audio_muted: bool,
    audio_music_gain: f32,
    audio_effects_gain: f32,
    audio_engine_low: f32,
    audio_engine_high: f32,
    audio_wind: f32,
    audio_rain: f32,
    audio_brake: f32,
    audio_skid: f32,
    audio_gravel: f32,
    audio_overrev: f32,
    audio_nitro: f32,
    audio_pitch: f32,
    audio_music_track: u8,
    audio_music_sunset: f32,
    audio_music_neon: f32,
    audio_music_dawn: f32,
    audio_last_cue: &'static str,
    audio_cue_count: u64,
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
fn publish_state(
    time: Res<Time>,
    mut clock: Local<PublishClock>,
    sim: Res<crate::Sim>,
    frame: Res<crate::FrameInput>,
    view: Res<crate::toy_view::ViewState>,
    rig: Res<crate::toy_view::BodyRig>,
    effects: Res<crate::VisualEffects>,
    nitro_pose: Res<crate::toy_view::NitroPose>,
    weather_visuals: Res<crate::view::WeatherVisuals>,
    nitro_visuals: Res<crate::view::NitroVisuals>,
    instruments: Res<crate::view::InstrumentVisuals>,
    end: Res<crate::EndSession>,
    leaderboard: Res<crate::LeaderboardState>,
    audio: Res<crate::audio::AudioState>,
    display: Res<crate::DisplaySettings>,
    tuning: Res<crate::TuningRes>,
) {
    if !*clock
        .hook_available
        .get_or_insert_with(crate::bridge::state_publisher_available)
    {
        return;
    }
    clock.elapsed += time.delta_secs();
    if clock.elapsed < PUBLISH_INTERVAL_SECS {
        return;
    }
    clock.elapsed = clock.elapsed.rem_euclid(PUBLISH_INTERVAL_SECS);

    let game = &sim.0;
    let weather = game.weather();
    let score = game_core::score_for(game.day_cycles, game.cars_passed, game.cars_bumped);
    let session_result = game.result().map(|result| ResultRow {
        day_cycles: result.day_cycles,
        cars_passed: result.cars_passed,
        cars_bumped: result.cars_bumped,
        score: result.score,
    });
    let leaderboard_rows = leaderboard
        .entries
        .iter()
        .filter(|entry| entry.result.difficulty == game.difficulty())
        .take(10)
        .map(|entry| LeaderboardRow {
            initials: entry.initials_str(),
            difficulty: crate::session::difficulty_name(entry.result.difficulty),
            day_cycles: entry.result.day_cycles,
            cars_passed: entry.result.cars_passed,
            cars_bumped: entry.result.cars_bumped,
            score: entry.result.score,
        })
        .collect();
    let snapshot = Snapshot {
        phase: match game.phase {
            game_core::Phase::Ready => "ready",
            game_core::Phase::Running => "running",
            game_core::Phase::GameOver { .. } => "gameover",
        },
        end_mode: end.0.mode.as_str(),
        difficulty: crate::session::difficulty_name(game.difficulty()),
        difficulty_index: end.0.difficulty_index,
        initials: end.0.initials.as_str(),
        leaderboard: leaderboard_rows,
        leaderboard_persistent: leaderboard.persistent,
        qa_mode: leaderboard.qa_mode,
        session_duration: game.session_duration(),
        score,
        speed: game.speed,
        speed_kmh: instruments.speed_kmh,
        speedometer_angle: instruments.needle_angle,
        overrev: instruments.overrev,
        recovery: instruments.recovery,
        action_cue: instruments.cue,
        nitro_cooldown: instruments.cooldown,
        cooldown_gauge: instruments.gauge_fill,
        session_remaining: game.session_remaining,
        environment_time: game.environment_time,
        day_cycles: game.day_cycles,
        cars_passed: game.cars_passed,
        cars_bumped: game.cars_bumped,
        session_result,
        weather_from: weather_name(weather.from),
        weather_to: weather_name(weather.to),
        weather_transition: weather.blend,
        weather_slot: weather.slot,
        weather_clear_visual: weather_visuals.clear,
        weather_cloudy_visual: weather_visuals.cloudy,
        weather_rain_visual: weather_visuals.rain,
        weather_rain_visible: weather_visuals.rain_visible,
        daylight: game_core::daylight(game.environment_time),
        season: game_core::environment::season_index(game.environment_time),
        gear: match game.gear() {
            game_core::Gear::Low => "low",
            game_core::Gear::High => "high",
        },
        lever_target: view.lever_target,
        lever_angle: view.lever_angle,
        nitro: game.nitro_charges,
        nitro_t: game.nitro_t,
        nitro_ahead: game.nitro_pickup_ahead(),
        nitro_lane: game.next_nitro_lane,
        nitro_glow: effects.nitro_pickup_glow,
        nitro_glow_segment: effects.nitro_segment,
        nitro_visual_intensity: nitro_visuals.intensity,
        jump_charge: game.jump_charge,
        jump_t: game.jump_t,
        jump_progress: game.jump_progress(&tuning.0),
        jump_ahead: game.jump_pickup_ahead(),
        jump_lane: game.next_jump_lane,
        toy_root_z: nitro_pose.z,
        toy_root_pitch: nitro_pose.pitch,
        toy: view.mode == crate::toy_view::ViewMode::Toy,
        look: view.toy_look,
        crt_enabled: display.crt_enabled,
        paused: game.paused,
        steer: frame.state.steer,
        brake: frame.state.brake,
        road_x: game.road_x,
        road_elevation: game.road.elevation_at(game.loop_pos()),
        road_grade: game.road_grade(),
        slope_speed_factor: game.slope_speed_factor(&tuning.0),
        off_road: game.off_road,
        roll: rig.0.roll,
        pad: frame.pad_seen,
        audio_unlocked: audio.unlocked,
        audio_muted: audio.muted,
        audio_music_gain: audio.music_gain,
        audio_effects_gain: audio.effects_gain,
        audio_engine_low: audio.engine_low,
        audio_engine_high: audio.engine_high,
        audio_wind: audio.wind,
        audio_rain: audio.rain,
        audio_brake: audio.brake,
        audio_skid: audio.skid,
        audio_gravel: audio.gravel,
        audio_overrev: audio.overrev,
        audio_nitro: audio.nitro,
        audio_pitch: audio.engine_pitch,
        audio_music_track: audio.music_track,
        audio_music_sunset: audio.music_sunset,
        audio_music_neon: audio.music_neon,
        audio_music_dawn: audio.music_dawn,
        audio_last_cue: audio.last_cue,
        audio_cue_count: audio.cue_count,
    };
    if let Ok(payload) = serde_json::to_string(&snapshot) {
        crate::bridge::publish_state(&payload);
    }
}

#[cfg(target_arch = "wasm32")]
const fn weather_name(weather: game_core::WeatherKind) -> &'static str {
    match weather {
        game_core::WeatherKind::Clear => "clear",
        game_core::WeatherKind::Cloudy => "cloudy",
        game_core::WeatherKind::Rain => "rain",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_rate_is_bounded_to_ten_hz() {
        assert_eq!(PUBLISH_INTERVAL_SECS, 0.10);
    }
}
