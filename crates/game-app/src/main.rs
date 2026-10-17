//! Fun to Drive — M5 app wiring: fixed-step simulation + input + view.
//!
//! Deterministic game logic lives in game-core; this crate only gathers
//! input, steps the sim on the fixed schedule, renders, and (on wasm)
//! publishes state for the Playwright harness.

mod audio;
mod bridge;
mod session;
mod telemetry;
mod toy_view;
mod view;

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use game_core::{Difficulty, Game, Gear, InputState, KeyboardSteering, Tuning};
use session::{EndFlow, EndMode};

const NITRO_PICKUP_GLOW_SECS: f32 = 0.65;

#[cfg(target_arch = "wasm32")]
const ASSET_ROOT: &str = "assets";
#[cfg(not(target_arch = "wasm32"))]
const ASSET_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");

#[derive(Resource)]
struct Sim(pub Game);

#[derive(Resource)]
struct TuningRes(pub Tuning);

#[derive(Resource, Default)]
struct RunConfig {
    qa_duration: Option<f32>,
}

/// Rendering adapter selected at startup. `?quality=low` keeps gameplay
/// unchanged while reducing the toy view's offscreen GPU cost.
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct RenderQuality {
    pub(crate) game_width: u32,
    pub(crate) game_height: u32,
    pub(crate) bloom: bool,
}

impl RenderQuality {
    fn from_query(value: Option<&str>) -> Self {
        if value.is_some_and(|value| value.eq_ignore_ascii_case("low")) {
            Self {
                game_width: 720,
                game_height: 450,
                bloom: false,
            }
        } else {
            Self {
                game_width: toy_view::GAME_W,
                game_height: toy_view::GAME_H,
                bloom: true,
            }
        }
    }
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
    gear_latch: GearRequestLatch,
    low_bumper_down: bool,
    high_bumper_down: bool,
}

#[derive(Resource, Debug, Clone, Copy)]
struct ControlTuning {
    bumper_press_threshold: f32,
    gear_request_buffer_secs: f32,
}

impl Default for ControlTuning {
    fn default() -> Self {
        Self {
            bumper_press_threshold: 0.28,
            gear_request_buffer_secs: 0.15,
        }
    }
}

impl ControlTuning {
    fn validate(self) -> Result<(), &'static str> {
        if !(0.05..=0.75).contains(&self.bumper_press_threshold) {
            return Err("bumper threshold out of [0.05, 0.75]");
        }
        if !(0.05..=0.30).contains(&self.gear_request_buffer_secs) {
            return Err("gear request buffer out of [0.05, 0.30]");
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct GearRequestLatch {
    request: Option<game_core::Gear>,
    remaining: f32,
}

impl GearRequestLatch {
    fn update(
        &mut self,
        request: Option<game_core::Gear>,
        dt: f32,
        hold_secs: f32,
    ) -> Option<game_core::Gear> {
        if let Some(request) = request {
            self.request = Some(request);
            self.remaining = hold_secs;
        } else {
            self.remaining = (self.remaining - dt).max(0.0);
            if self.remaining == 0.0 {
                self.request = None;
            }
        }
        self.request
    }

    fn consume(&mut self) {
        self.request = None;
        self.remaining = 0.0;
    }
}

/// Cross-module frame phases. Systems share these sets instead of relying on
/// plugin registration order or serializing the entire frame in one chain.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AppSet {
    Flow,
    Controls,
    Presentation,
    View,
    Audio,
    Telemetry,
}

fn main() {
    let tuning = Tuning::default();
    tuning
        .validate()
        .expect("built-in gameplay tuning must be valid");
    let control_tuning = ControlTuning::default();
    control_tuning
        .validate()
        .expect("built-in control tuning must be valid");
    let qa_parameter = bridge::query_param("qa_session_secs");
    let render_quality = RenderQuality::from_query(bridge::query_param("quality").as_deref());
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
    let audio_muted = bridge::load_local(audio::STORAGE_KEY).0.as_deref() == Some("muted");

    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                // wasm: don't probe for .meta files next to assets (they don't exist
                // in the trunk dist; probing fails asset loads)
                meta_check: AssetMetaCheck::Never,
                file_path: ASSET_ROOT.into(),
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
    .add_plugins(audio::ArcadeAudioPlugin)
    .insert_resource(ClearColor(Color::srgb(0.008, 0.011, 0.022)))
    .insert_resource(TuningRes(tuning))
    .insert_resource(control_tuning)
    .insert_resource(RunConfig { qa_duration })
    .insert_resource(render_quality)
    .insert_resource(LeaderboardState {
        entries: leaderboard,
        persistent,
        qa_mode,
    })
    .insert_resource(Sim({
        // Seeded technical loop: mixed-radius corners, rolling elevation,
        // traffic, and collidable roadside scenery.
        let mut rng = game_core::Rng::new(20261015);
        let road = game_core::RoadLoop::generate(&mut rng);
        if let Some(duration) = qa_duration {
            Game::with_road_for_duration(20261015, road, Difficulty::Medium, duration, &tuning)
        } else {
            Game::with_road(20261015, road, Difficulty::Medium, &tuning)
        }
    }))
    .init_resource::<FrameInput>()
    .init_resource::<VisualEffects>()
    .init_resource::<EndSession>()
    .insert_resource(audio::AudioState::new(audio_muted))
    .configure_sets(
        Update,
        (
            AppSet::Flow,
            AppSet::Controls,
            AppSet::Presentation,
            AppSet::View,
            AppSet::Audio,
            AppSet::Telemetry,
        )
            .chain(),
    )
    .add_systems(
        Startup,
        (view::setup, toy_view::setup, toy_view::setup_diorama),
    )
    .add_systems(PreUpdate, gather_input.after(bevy::input::InputSystems))
    .add_systems(Update, sync_end_flow.in_set(AppSet::Flow))
    .add_systems(Update, handle_end_input.in_set(AppSet::Controls))
    .add_systems(
        Update,
        (
            tick_visual_effects,
            (toy_view::spawn_toy, toy_view::paint_screen).chain(),
            toy_view::toggle_mode,
            (toy_view::apply_body_dynamics, toy_view::animate_parts).chain(),
            toy_view::turbo_lamp,
            toy_view::update_screen_mat,
        )
            .in_set(AppSet::Presentation),
    )
    .add_systems(FixedUpdate, step_sim);

    app.init_resource::<toy_view::ScreenMatHandle>();
    app.init_resource::<toy_view::ScreenPaintState>();
    app.init_resource::<toy_view::BodyRig>();
    app.init_resource::<toy_view::CrashPulse>();
    app.init_resource::<toy_view::NitroPose>();

    view::add_view_systems(&mut app);
    telemetry::add_systems(&mut app);
    app.run();
}

/// M5 interim input: keyboard-first, gamepad passthrough (formal layer = M6).
fn gather_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut frame: ResMut<FrameInput>,
    tuning: Res<TuningRes>,
    controls: Res<ControlTuning>,
    time: Res<Time>,
    end: Res<EndSession>,
) {
    if end.0.mode != EndMode::Playing {
        frame.state.steer = 0.0;
        frame.state.brake = 0.0;
        frame.state.gear_request = None;
        frame.state.pause = false;
        frame.state.nitro = false;
        frame.gear_latch.consume();
        return;
    }
    if frame.suppress_gameplay_once {
        frame.suppress_gameplay_once = false;
        frame.state = InputState::default();
        frame.gear_latch.consume();
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
    let mut low_bumper_down = false;
    let mut high_bumper_down = false;
    let mut brake = 0.0_f32;
    for pad in &pads {
        let axis = pad.get(GamepadAxis::LeftStickX).unwrap_or_default();
        let x = game_core::input::apply_deadzone(axis, tuning.0.pad_deadzone);
        if x != 0.0 {
            steer = x; // analog stick overrides the keyboard shape
            frame.pad_seen = true;
        }
        // LB/RB can report analog values on some controllers. Bevy's normal
        // digital press point is intentionally firm, so use a lighter edge
        // here and latch it until the fixed simulation consumes the request.
        low_bumper_down |= pad.get(GamepadButton::LeftTrigger).unwrap_or_default()
            >= controls.bumper_press_threshold;
        high_bumper_down |= pad.get(GamepadButton::RightTrigger).unwrap_or_default()
            >= controls.bumper_press_threshold;
        let pad_brake = pad
            .get(GamepadButton::LeftTrigger2)
            .unwrap_or_default()
            .clamp(0.0, 1.0);
        brake = brake.max(pad_brake);
        if pad_brake > 0.02 {
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

    low_requested |= low_bumper_down && !frame.low_bumper_down;
    high_requested |= high_bumper_down && !frame.high_bumper_down;
    if low_requested || high_requested {
        frame.pad_seen |= low_bumper_down || high_bumper_down;
    }
    frame.low_bumper_down = low_bumper_down;
    frame.high_bumper_down = high_bumper_down;

    frame.state.steer = steer.clamp(-1.0, 1.0);
    frame.state.brake = brake;
    let gear_request = game_core::gear_request(low_requested, high_requested);
    frame.state.gear_request = frame.gear_latch.update(
        gear_request,
        time.delta_secs(),
        controls.gear_request_buffer_secs,
    );
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
    mut audio_events: ResMut<audio::AudioEvents>,
) {
    match end.0.mode {
        EndMode::Playing => {}
        EndMode::DifficultySelect => {
            let previous_index = end.0.difficulty_index;
            let left = keys.just_pressed(KeyCode::ArrowLeft)
                || pads
                    .iter()
                    .any(|pad| pad.just_pressed(GamepadButton::DPadLeft));
            let right = keys.just_pressed(KeyCode::ArrowRight)
                || pads
                    .iter()
                    .any(|pad| pad.just_pressed(GamepadButton::DPadRight));
            end.0.difficulty_index = menu_index_after_input(end.0.difficulty_index, left, right);
            if end.0.difficulty_index != previous_index {
                audio_events.emit(audio::AudioCue::MenuMove);
            }
            let confirm = keys.just_pressed(KeyCode::Enter)
                || pads
                    .iter()
                    .any(|pad| pad.just_pressed(GamepadButton::South));
            if confirm {
                audio_events.emit(audio::AudioCue::MenuConfirm);
                audio_events.emit(audio::AudioCue::RunStart);
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
                if end.0.initials.pop() {
                    audio_events.emit(audio::AudioCue::Delete);
                }
            } else if let Some(letter) = just_pressed_letter(&keys) {
                if end.0.initials.push(letter) {
                    audio_events.emit(audio::AudioCue::Key);
                }
            }
            if keys.just_pressed(KeyCode::Enter) {
                let Some(result) = sim.0.result().copied() else {
                    return;
                };
                if let Some(entry) = end.0.submit(result) {
                    audio_events.emit(audio::AudioCue::Submit);
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
                audio_events.emit(audio::AudioCue::MenuConfirm);
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
    mut audio_events: ResMut<audio::AudioEvents>,
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
    if let Some(segment) = events.nitro_collected_segment {
        audio_events.emit(audio::AudioCue::NitroPickup);
        effects.nitro_pickup_glow = NITRO_PICKUP_GLOW_SECS;
        effects.nitro_segment = Some(segment);
    }
    if events.nitro_started {
        audio_events.emit(audio::AudioCue::NitroStart);
    }
    if let Some(direction) = events.bump_direction {
        audio_events.emit(audio::AudioCue::Crash);
        crash.pending = true;
        crash.dir = direction;
    }
    if events.high_rejected {
        audio_events.emit(audio::AudioCue::Clank);
        effects.action_cue = ActionCue::TooSoon;
        effects.action_cue_secs = 0.75;
    }
    if let Some(change) = events.gear_changed {
        audio_events.emit(audio::AudioCue::Shift);
        if change.gear == Gear::Low {
            effects.action_cue = ActionCue::ShiftLow;
            effects.action_cue_secs = 0.55;
            effects.downshift_secs = 0.38;
            effects.downshift_severity = change.downshift_severity;
        }
    }
    // edge-triggered flags are consumed by exactly one fixed step
    frame.state.gear_request = None;
    frame.gear_latch.consume();
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

    #[test]
    fn quick_shift_request_stays_latched_until_fixed_step_consumes_it() {
        let tuning = ControlTuning::default();
        assert!(tuning.validate().is_ok());
        let mut latch = GearRequestLatch::default();
        assert_eq!(
            latch.update(
                Some(game_core::Gear::High),
                1.0 / 144.0,
                tuning.gear_request_buffer_secs
            ),
            Some(game_core::Gear::High)
        );
        assert_eq!(
            latch.update(None, 1.0 / 144.0, tuning.gear_request_buffer_secs),
            Some(game_core::Gear::High),
            "a render frame must not erase the edge before FixedUpdate"
        );
        latch.consume();
        assert_eq!(latch.request, None);
        assert_eq!(latch.remaining, 0.0);
    }

    #[test]
    fn invalid_control_tuning_is_rejected() {
        assert!(ControlTuning {
            bumper_press_threshold: 0.0,
            ..ControlTuning::default()
        }
        .validate()
        .is_err());
        assert!(ControlTuning {
            gear_request_buffer_secs: f32::NAN,
            ..ControlTuning::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn low_quality_preserves_aspect_and_disables_bloom() {
        let high = RenderQuality::from_query(None);
        let low = RenderQuality::from_query(Some("LOW"));
        assert!(high.bloom);
        assert!(!low.bloom);
        assert!(low.game_width < high.game_width);
        assert_eq!(
            high.game_width * low.game_height,
            low.game_width * high.game_height
        );
    }
}
