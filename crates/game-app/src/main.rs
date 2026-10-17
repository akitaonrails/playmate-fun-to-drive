//! Fun to Drive — M5 app wiring: fixed-step simulation + input + view.
//!
//! Deterministic game logic lives in game-core; this crate only gathers
//! input, steps the sim on the fixed schedule, renders, and (on wasm)
//! publishes state for the Playwright harness.

mod bridge;
mod session;
mod toy_view;
mod view;

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use game_core::{Difficulty, Game, InputState, KeyboardSteering, Tuning};
use session::{EndFlow, EndMode};

const NITRO_PICKUP_GLOW_SECS: f32 = 0.65;

#[derive(Resource)]
struct Sim(pub Game);

#[derive(Resource)]
struct TuningRes(pub Tuning);

#[derive(Resource, Default)]
struct RunConfig {
    qa_duration: Option<f32>,
}

#[derive(Resource, Default)]
pub(crate) struct EndSession(pub(crate) EndFlow);

#[derive(Resource)]
pub(crate) struct LeaderboardState {
    pub(crate) entries: Vec<game_core::LeaderboardEntry>,
    pub(crate) persistent: bool,
    pub(crate) qa_mode: bool,
}

/// Nitro can remain numerically charged on the terminal fixed step, so every
/// cosmetic consumer must also require a live running phase.
pub(crate) fn nitro_effects_active(game: &Game) -> bool {
    matches!(game.phase, game_core::Phase::Running) && game.nitro_active()
}

/// Short-lived presentation state captured from the fixed-step simulation.
/// Keeping this outside `Game` prevents cosmetic timing from affecting the
/// deterministic run while still making one-frame events visible to renderers.
#[derive(Resource, Default)]
pub(crate) struct VisualEffects {
    pub(crate) nitro_pickup_glow: f32,
    pub(crate) nitro_segment: Option<u8>,
    pub(crate) action_cue: ActionCue,
    pub(crate) action_cue_secs: f32,
    pub(crate) downshift_secs: f32,
    pub(crate) downshift_severity: Option<game_core::DownshiftSeverity>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ActionCue {
    #[default]
    None,
    ShiftLow,
    TooSoon,
}

#[derive(Resource, Default)]
struct FrameInput {
    pub(crate) state: InputState,
    pub(crate) pad_seen: bool,
    kb: KeyboardSteering,
    suppress_gameplay_once: bool,
}

fn main() {
    let tuning = Tuning::default();
    let qa_parameter = bridge::query_param("qa_session_secs");
    let (qa_mode, qa_duration) = session::qa_mode_and_duration(qa_parameter.as_deref());
    let (leaderboard, persistent) = if qa_mode {
        (Vec::new(), false)
    } else {
        let (stored_v2, mut persistent) = bridge::load_local(session::STORAGE_KEY);
        let legacy_v1 = if stored_v2.is_none() {
            bridge::load_local(session::LEGACY_STORAGE_KEY).0
        } else {
            None
        };
        let (entries, migration) =
            session::load_board_sources(stored_v2.as_deref(), legacy_v1.as_deref());
        if let Some(encoded) = migration {
            persistent = bridge::save_local(session::STORAGE_KEY, &encoded);
        }
        (entries, persistent)
    };
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                // wasm: don't probe for .meta files next to assets (they don't exist
                // in the trunk dist; probing fails asset loads)
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Fun to Drive".into(),
                    resolution: bevy::window::WindowResolution::new(960, 600),
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.09)))
    .insert_resource(TuningRes(tuning))
    .insert_resource(RunConfig { qa_duration })
    .insert_resource(LeaderboardState {
        entries: leaderboard,
        persistent,
        qa_mode,
    })
    .insert_resource(Sim({
        // square feel-test track: straights + 4 constant corners (user req)
        let mut rng = game_core::Rng::new(20261015);
        let mut road = game_core::RoadLoop::generate(&mut rng);
        road.apply_square_curve();
        if let Some(duration) = qa_duration {
            Game::with_road_for_duration(20261015, road, Difficulty::Medium, duration, &tuning)
        } else {
            Game::with_road(20261015, road, Difficulty::Medium, &tuning)
        }
    }))
    .init_resource::<FrameInput>()
    .init_resource::<VisualEffects>()
    .init_resource::<EndSession>()
    .add_systems(
        Startup,
        (view::setup, toy_view::setup, toy_view::setup_diorama),
    )
    .add_systems(
        Update,
        (
            tick_visual_effects,
            sync_end_flow,
            handle_end_input,
            gather_input,
            toy_view::spawn_toy,
            toy_view::paint_screen,
            toy_view::toggle_mode,
            toy_view::apply_body_dynamics,
            toy_view::animate_parts,
            toy_view::turbo_lamp,
        )
            .chain(),
    )
    .add_systems(FixedUpdate, step_sim);

    app.init_resource::<toy_view::ScreenMatHandle>();
    app.init_resource::<toy_view::BodyRig>();
    app.init_resource::<toy_view::CrashPulse>();
    app.init_resource::<toy_view::NitroPose>();

    view::add_view_systems(&mut app);
    app.add_systems(Update, (toy_view::update_screen_mat, publish_state).chain());
    app.run();
}

/// M5 interim input: keyboard-first, gamepad passthrough (formal layer = M6).
fn gather_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut frame: ResMut<FrameInput>,
    tuning: Res<TuningRes>,
    time: Res<Time>,
    end: Res<EndSession>,
) {
    if end.0.mode != EndMode::Playing {
        frame.state.steer = 0.0;
        frame.state.gear_request = None;
        frame.state.pause = false;
        frame.state.nitro = false;
        return;
    }
    if frame.suppress_gameplay_once {
        frame.suppress_gameplay_once = false;
        frame.state = InputState::default();
        return;
    }
    // keyboard short/long-press shaping (M6, logic tested in game-core)
    let mut steer = frame.kb.update(
        keys.pressed(KeyCode::ArrowLeft),
        keys.pressed(KeyCode::ArrowRight),
        time.delta_secs(),
        &tuning.0,
    );

    let mut low_requested = keys.just_pressed(KeyCode::KeyQ);
    let mut high_requested = keys.just_pressed(KeyCode::KeyE);
    for pad in &pads {
        let axis = pad.get(GamepadAxis::LeftStickX).unwrap_or_default();
        let x = game_core::input::apply_deadzone(axis, tuning.0.pad_deadzone);
        if x != 0.0 {
            steer = x; // analog stick overrides the keyboard shape
            frame.pad_seen = true;
        }
        if pad.just_pressed(GamepadButton::LeftTrigger) {
            low_requested = true;
            frame.pad_seen = true;
        }
        if pad.just_pressed(GamepadButton::RightTrigger) {
            high_requested = true;
            frame.pad_seen = true;
        }
        if pad.just_pressed(GamepadButton::Start) {
            frame.state.pause = true;
            frame.pad_seen = true;
        }
        if pad.just_pressed(GamepadButton::Select) {
            frame.state.reset = true;
            frame.pad_seen = true;
        }
        if pad.just_pressed(GamepadButton::North) {
            frame.state.nitro = true;
            frame.pad_seen = true;
        }
    }

    frame.state.steer = steer.clamp(-1.0, 1.0);
    frame.state.gear_request = game_core::gear_request(low_requested, high_requested);
    if keys.just_pressed(KeyCode::KeyR) {
        frame.state.reset = true;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        frame.state.pause = true;
    }
    if keys.just_pressed(KeyCode::KeyY) {
        frame.state.nitro = true;
    }
}

fn sync_end_flow(sim: Res<Sim>, mut end: ResMut<EndSession>) {
    match sim.0.phase {
        game_core::Phase::GameOver { .. }
            if end.0.mode == EndMode::Playing && sim.0.result().is_some() =>
        {
            end.0.begin_entry();
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_end_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut sim: ResMut<Sim>,
    tuning: Res<TuningRes>,
    config: Res<RunConfig>,
    mut end: ResMut<EndSession>,
    mut leaderboard: ResMut<LeaderboardState>,
    mut frame: ResMut<FrameInput>,
) {
    match end.0.mode {
        EndMode::Playing => {}
        EndMode::DifficultySelect => {
            let left = keys.just_pressed(KeyCode::ArrowLeft)
                || pads
                    .iter()
                    .any(|pad| pad.just_pressed(GamepadButton::DPadLeft));
            let right = keys.just_pressed(KeyCode::ArrowRight)
                || pads
                    .iter()
                    .any(|pad| pad.just_pressed(GamepadButton::DPadRight));
            end.0.difficulty_index = menu_index_after_input(end.0.difficulty_index, left, right);
            let confirm = keys.just_pressed(KeyCode::Enter)
                || pads
                    .iter()
                    .any(|pad| pad.just_pressed(GamepadButton::South));
            if confirm {
                let difficulty = Difficulty::ALL[end.0.difficulty_index];
                if let Some(duration) = config.qa_duration {
                    sim.0 = Game::with_road_for_duration(
                        sim.0.seed,
                        sim.0.road.clone(),
                        difficulty,
                        duration,
                        &tuning.0,
                    );
                    let _ = sim.0.step(
                        &InputState {
                            reset: true,
                            ..default()
                        },
                        0.0,
                        &tuning.0,
                    );
                } else {
                    sim.0.restart_with_difficulty(difficulty, &tuning.0);
                }
                end.0.mode = EndMode::Playing;
                frame.state = InputState::default();
                frame.kb = KeyboardSteering::default();
                frame.suppress_gameplay_once = true;
            }
        }
        EndMode::NameEntry => {
            if keys.just_pressed(KeyCode::Backspace) {
                end.0.initials.pop();
            } else if let Some(letter) = just_pressed_letter(&keys) {
                end.0.initials.push(letter);
            }
            if keys.just_pressed(KeyCode::Enter) {
                let Some(result) = sim.0.result().copied() else {
                    return;
                };
                if let Some(entry) = end.0.submit(result) {
                    leaderboard.entries.push(entry);
                    game_core::rank_leaderboard(&mut leaderboard.entries);
                    if !leaderboard.qa_mode {
                        let encoded = session::encode_board(&leaderboard.entries);
                        leaderboard.persistent = bridge::save_local(session::STORAGE_KEY, &encoded);
                    }
                }
            }
        }
        EndMode::Leaderboard => {
            if keys.just_pressed(KeyCode::KeyR) || keys.just_pressed(KeyCode::Enter) {
                end.0.mode = EndMode::DifficultySelect;
                end.0.difficulty_index = difficulty_index(sim.0.difficulty());
                end.0.initials = session::Initials::default();
                frame.state = InputState::default();
            }
        }
    }
}

fn difficulty_index(difficulty: Difficulty) -> usize {
    Difficulty::ALL
        .iter()
        .position(|candidate| *candidate == difficulty)
        .unwrap_or(1)
}

fn menu_index_after_input(index: usize, left: bool, right: bool) -> usize {
    match (left, right) {
        (true, false) => index.saturating_sub(1),
        (false, true) => (index + 1).min(Difficulty::ALL.len() - 1),
        _ => index.min(Difficulty::ALL.len() - 1),
    }
}

fn just_pressed_letter(keys: &ButtonInput<KeyCode>) -> Option<char> {
    const LETTERS: [(KeyCode, char); 26] = [
        (KeyCode::KeyA, 'A'),
        (KeyCode::KeyB, 'B'),
        (KeyCode::KeyC, 'C'),
        (KeyCode::KeyD, 'D'),
        (KeyCode::KeyE, 'E'),
        (KeyCode::KeyF, 'F'),
        (KeyCode::KeyG, 'G'),
        (KeyCode::KeyH, 'H'),
        (KeyCode::KeyI, 'I'),
        (KeyCode::KeyJ, 'J'),
        (KeyCode::KeyK, 'K'),
        (KeyCode::KeyL, 'L'),
        (KeyCode::KeyM, 'M'),
        (KeyCode::KeyN, 'N'),
        (KeyCode::KeyO, 'O'),
        (KeyCode::KeyP, 'P'),
        (KeyCode::KeyQ, 'Q'),
        (KeyCode::KeyR, 'R'),
        (KeyCode::KeyS, 'S'),
        (KeyCode::KeyT, 'T'),
        (KeyCode::KeyU, 'U'),
        (KeyCode::KeyV, 'V'),
        (KeyCode::KeyW, 'W'),
        (KeyCode::KeyX, 'X'),
        (KeyCode::KeyY, 'Y'),
        (KeyCode::KeyZ, 'Z'),
    ];
    LETTERS
        .iter()
        .find_map(|(key, letter)| keys.just_pressed(*key).then_some(*letter))
}

fn step_sim(
    mut sim: ResMut<Sim>,
    mut frame: ResMut<FrameInput>,
    mut effects: ResMut<VisualEffects>,
    mut crash: ResMut<toy_view::CrashPulse>,
    tuning: Res<TuningRes>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let events = sim.0.step(&frame.state, dt, &tuning.0);
    if events.started {
        effects.nitro_pickup_glow = 0.0;
        effects.nitro_segment = None;
        effects.action_cue = ActionCue::None;
        effects.action_cue_secs = 0.0;
        effects.downshift_secs = 0.0;
        effects.downshift_severity = None;
    }
    if events.nitro_collected {
        effects.nitro_pickup_glow = NITRO_PICKUP_GLOW_SECS;
        effects.nitro_segment = sim.0.nitro_charges.checked_sub(1);
    }
    if events.bumped {
        crash.pending = true;
        crash.dir = sim.0.bump_dir;
    }
    if events.high_rejected {
        effects.action_cue = ActionCue::TooSoon;
        effects.action_cue_secs = 0.75;
    }
    if events.downshifted {
        effects.action_cue = ActionCue::ShiftLow;
        effects.action_cue_secs = 0.55;
        effects.downshift_secs = 0.38;
        effects.downshift_severity = events.downshift_severity;
    }
    // edge-triggered flags are consumed by exactly one fixed step
    frame.state.gear_request = None;
    frame.state.reset = false;
    frame.state.pause = false;
    frame.state.nitro = false;
}

fn tick_visual_effects(
    time: Res<Time>,
    sim: Res<Sim>,
    end: Res<EndSession>,
    mut effects: ResMut<VisualEffects>,
) {
    if end.0.mode != EndMode::Playing || !matches!(sim.0.phase, game_core::Phase::Running) {
        effects.action_cue = ActionCue::None;
        effects.action_cue_secs = 0.0;
        effects.downshift_secs = 0.0;
        effects.downshift_severity = None;
    }
    effects.nitro_pickup_glow = (effects.nitro_pickup_glow - time.delta_secs()).max(0.0);
    effects.action_cue_secs = (effects.action_cue_secs - time.delta_secs()).max(0.0);
    effects.downshift_secs = (effects.downshift_secs - time.delta_secs()).max(0.0);
    if effects.action_cue_secs == 0.0 {
        effects.action_cue = ActionCue::None;
    }
    if effects.downshift_secs == 0.0 {
        effects.downshift_severity = None;
    }
    if effects.nitro_pickup_glow <= 0.0 {
        effects.nitro_segment = None;
    }
}

#[allow(clippy::too_many_arguments)]
fn publish_state(
    sim: Res<Sim>,
    frame: Res<FrameInput>,
    view: Res<toy_view::ViewState>,
    rig: Res<toy_view::BodyRig>,
    effects: Res<VisualEffects>,
    nitro_pose: Res<toy_view::NitroPose>,
    weather_visuals: Res<view::WeatherVisuals>,
    nitro_visuals: Res<view::NitroVisuals>,
    instruments: Res<view::InstrumentVisuals>,
    end: Res<EndSession>,
    leaderboard: Res<LeaderboardState>,
) {
    let weather = sim.0.weather();
    let score = game_core::score_for(sim.0.day_cycles, sim.0.cars_passed, sim.0.cars_bumped);
    let result = match sim.0.result() {
        Some(result) => format!(
            "{{\"day_cycles\":{},\"cars_passed\":{},\"cars_bumped\":{},\"score\":{}}}",
            result.day_cycles, result.cars_passed, result.cars_bumped, result.score
        ),
        None => "null".to_string(),
    };
    let leaderboard_json = session::entries_json(&leaderboard.entries, sim.0.difficulty(), 10);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let payload = format!(
        "{{\"phase\":\"{}\",\"end_mode\":\"{}\",\"difficulty\":\"{}\",\"difficulty_index\":{},\"initials\":\"{}\",\"leaderboard\":{},\"leaderboard_persistent\":{},\"qa_mode\":{},\"session_duration\":{:.3},\"score\":{},\"speed\":{:.3},\"speed_kmh\":{:.3},\"speedometer_angle\":{:.5},\"overrev\":{:.3},\"recovery\":\"{}\",\"action_cue\":\"{}\",\"nitro_cooldown\":{:.3},\"cooldown_gauge\":{:.3},\"session_remaining\":{:.3},\"environment_time\":{:.3},\"day_cycles\":{},\"cars_passed\":{},\"cars_bumped\":{},\"session_result\":{},\"weather_from\":\"{}\",\"weather_to\":\"{}\",\"weather_transition\":{:.3},\"weather_slot\":{},\"weather_clear_visual\":{:.3},\"weather_cloudy_visual\":{:.3},\"weather_rain_visual\":{:.3},\"weather_rain_visible\":{:.3},\"daylight\":{:.3},\"season\":{:.3},\"gear\":\"{}\",\"lever_target\":{:.4},\"lever_angle\":{:.4},\"nitro\":{},\"nitro_t\":{:.3},\"nitro_ahead\":{:.3},\"nitro_lane\":{:.2},\"nitro_glow\":{:.3},\"nitro_glow_segment\":{},\"nitro_visual_intensity\":{:.3},\"toy_root_z\":{:.5},\"toy_root_pitch\":{:.5},\"toy\":{},\"look\":{:.2},\"paused\":{},\"steer\":{:.3},\"road_x\":{:.4},\"off_road\":{:.3},\"roll\":{:.4},\"pad\":{}}}",
        match sim.0.phase {
            game_core::Phase::Ready => "ready",
            game_core::Phase::Running => "running",
            game_core::Phase::GameOver { .. } => "gameover",
        },
        end.0.mode.as_str(),
        session::difficulty_name(sim.0.difficulty()),
        end.0.difficulty_index,
        end.0.initials.as_str(),
        leaderboard_json,
        leaderboard.persistent,
        leaderboard.qa_mode,
        sim.0.session_duration(),
        score,
        sim.0.speed,
        instruments.speed_kmh,
        instruments.needle_angle,
        instruments.overrev,
        instruments.recovery,
        instruments.cue,
        instruments.cooldown,
        instruments.gauge_fill,
        sim.0.session_remaining,
        sim.0.environment_time,
        sim.0.day_cycles,
        sim.0.cars_passed,
        sim.0.cars_bumped,
        result,
        weather_name(weather.from),
        weather_name(weather.to),
        weather.blend,
        weather.slot,
        weather_visuals.clear,
        weather_visuals.cloudy,
        weather_visuals.rain,
        weather_visuals.rain_visible,
        game_core::daylight(sim.0.environment_time),
        game_core::environment::season_index(sim.0.environment_time),
        match sim.0.gear() {
            game_core::Gear::Low => "low",
            game_core::Gear::High => "high",
        },
        view.lever_target,
        view.lever_angle,
        sim.0.nitro_charges,
        sim.0.nitro_t,
        sim.0.nitro_pickup_ahead(),
        sim.0.next_nitro_lane,
        effects.nitro_pickup_glow,
        effects
            .nitro_segment
            .map_or_else(|| "null".to_string(), |segment| segment.to_string()),
        nitro_visuals.intensity,
        nitro_pose.z,
        nitro_pose.pitch,
        view.mode == toy_view::ViewMode::Toy,
        view.toy_look,
        sim.0.paused,
        frame.state.steer,
        sim.0.road_x,
        sim.0.off_road,
        rig.0.roll,
        frame.pad_seen,
    );
    bridge::publish_state(&payload);
}

fn weather_name(weather: game_core::WeatherKind) -> &'static str {
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
    fn terminal_phase_disables_frozen_nitro_visuals() {
        let tuning = Tuning::default();
        let mut game = Game::new(7, Difficulty::Hard, &tuning);
        game.phase = game_core::Phase::Running;
        game.nitro_t = 1.0;
        assert!(nitro_effects_active(&game));

        game.phase = game_core::Phase::GameOver {
            reason: game_core::EndReason::Time,
        };
        assert!(
            game.nitro_active(),
            "core charge is frozen for the result frame"
        );
        assert!(!nitro_effects_active(&game));
    }

    #[test]
    fn difficulty_menu_navigation_clamps_and_ignores_conflicts() {
        assert_eq!(menu_index_after_input(1, true, false), 0);
        assert_eq!(menu_index_after_input(0, true, false), 0);
        assert_eq!(menu_index_after_input(1, false, true), 2);
        assert_eq!(menu_index_after_input(2, false, true), 2);
        assert_eq!(menu_index_after_input(1, true, true), 1);
        assert_eq!(difficulty_index(Difficulty::Medium), 1);
    }
}
