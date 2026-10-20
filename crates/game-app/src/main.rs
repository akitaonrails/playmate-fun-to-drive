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
const JUMP_PICKUP_GLOW_SECS: f32 = 0.80;
const FINISH_SEQUENCE_DURATION_SECS: f32 = 3.2;
const SKY_DROP_DURATION_SECS: f32 = 1.65;
const MENU_HEADLIGHT_IDLE_SECS: f32 = 30.0;
#[cfg(debug_assertions)]
const DEV_ENDING_RUN_SECS: f32 = 0.01;

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
    qa_start_environment_time: Option<f32>,
}

/// Presentation-only entropy for selecting a different horizon on each new
/// menu-confirmed run without changing the deterministic road or leaderboard.
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct SceneryRun {
    pub(crate) entropy: u64,
    pub(crate) serial: u64,
}

impl SceneryRun {
    fn new(qa_mode: bool) -> Self {
        Self {
            entropy: if qa_mode {
                20_261_015
            } else {
                runtime_scenery_entropy()
            },
            serial: 0,
        }
    }

    fn advance(&mut self) {
        self.serial = self.serial.wrapping_add(1);
    }

    fn environment_start(self, calendar: bridge::LocalCalendar) -> f32 {
        calendar_environment_start(calendar)
    }
}

fn calendar_environment_start(calendar: bridge::LocalCalendar) -> f32 {
    let month = calendar.month.clamp(1, 12);
    let day = calendar.day.clamp(1, 31);
    // Meteorological seasons: Mar/Jun/Sep/Dec in the north, offset by six
    // months in the south. Progress through the real season is preserved.
    let northern_season = ((month + 9) % 12) / 3;
    let season = if calendar.southern {
        (northern_season + 2) % 4
    } else {
        northern_season
    };
    let month_in_season = (month + 9) % 3;
    let season_progress = (month_in_season as f32 + (day - 1) as f32 / 31.0) / 3.0;
    let season_offset = season_progress * game_core::environment::SEASON_SECS;
    let day_cycle = (season_offset / game_core::environment::DAY_SECS).floor();
    let local_day =
        (calendar.hour.min(23) as f32 * 60.0 + calendar.minute.min(59) as f32) / (24.0 * 60.0);
    // The environment's phase peaks at 0.25; shift civil noon there so local
    // midnight maps to deep night and local sunrise/sunset straddle the fades.
    let environment_day_phase = (local_day + 0.75).fract();
    season as f32 * game_core::environment::SEASON_SECS
        + day_cycle * game_core::environment::DAY_SECS
        + environment_day_phase * game_core::environment::DAY_SECS
}

#[cfg(target_arch = "wasm32")]
fn runtime_scenery_entropy() -> u64 {
    js_sys::Date::now().to_bits() ^ js_sys::Math::random().to_bits().rotate_left(23)
}

#[cfg(not(target_arch = "wasm32"))]
fn runtime_scenery_entropy() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(20_261_015, |duration| duration.as_nanos() as u64)
}

#[cfg(debug_assertions)]
fn dev_ending_requested(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("on")
    })
}

#[cfg(debug_assertions)]
fn dev_ending_game(tuning: &Tuning) -> Game {
    let mut rng = game_core::Rng::new(20261015);
    let road =
        game_core::RoadLoop::generate_with_traffic_bonus(&mut rng, tuning.traffic_medium_extra);
    let mut game = Game::with_road_for_duration(
        20261015,
        road,
        Difficulty::Medium,
        DEV_ENDING_RUN_SECS,
        tuning,
    );
    let _ = game.step(
        &InputState {
            reset: true,
            ..default()
        },
        0.0,
        tuning,
    );
    let _ = game.step(&InputState::default(), DEV_ENDING_RUN_SECS, tuning);
    game
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
    fn from_query(value: Option<&str>, coarse_pointer: bool) -> Self {
        let explicit_high = value.is_some_and(|value| value.eq_ignore_ascii_case("high"));
        let low = value.is_some_and(|value| value.eq_ignore_ascii_case("low"))
            || (coarse_pointer && !explicit_high);
        if low {
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

/// Presentation-only finale between the fair terminal simulation tick and
/// name entry. The recorded result is already frozen while this runs.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub(crate) struct FinishSequence {
    pub(crate) active: bool,
    pub(crate) elapsed: f32,
}

impl FinishSequence {
    pub(crate) fn progress(self) -> f32 {
        (self.elapsed / FINISH_SEQUENCE_DURATION_SECS).clamp(0.0, 1.0)
    }
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub(crate) struct LaunchSequence {
    pub(crate) active: bool,
    pub(crate) elapsed: f32,
}

impl LaunchSequence {
    pub(crate) fn progress(self) -> f32 {
        (self.elapsed / SKY_DROP_DURATION_SECS).clamp(0.0, 1.0)
    }
}

/// Presentation-only inactivity clock for the difficulty screen. It never
/// enters the deterministic simulation or changes a run's result.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub(crate) struct MenuIdle {
    pub(crate) seconds: f32,
}

impl MenuIdle {
    pub(crate) fn headlight_blink(self) -> f32 {
        menu_headlight_blink(self.seconds)
    }
}

fn menu_headlight_blink(idle_seconds: f32) -> f32 {
    if idle_seconds < MENU_HEADLIGHT_IDLE_SECS {
        return 0.0;
    }
    let phase = (idle_seconds - MENU_HEADLIGHT_IDLE_SECS) % 6.0;
    let pulse = |start: f32| {
        let progress = ((phase - start) / 0.22).clamp(0.0, 1.0);
        if (start..=start + 0.22).contains(&phase) {
            (progress * std::f32::consts::PI).sin()
        } else {
            0.0
        }
    };
    pulse(0.0).max(pulse(0.36))
}

#[derive(Resource)]
pub(crate) struct LeaderboardState {
    pub(crate) entries: Vec<game_core::LeaderboardEntry>,
    pub(crate) persistent: bool,
    pub(crate) qa_mode: bool,
}

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct DisplaySettings {
    pub(crate) crt_enabled: bool,
    pub(crate) large_screen: bool,
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
    pub(crate) jump_pickup_glow: f32,
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
    menu_axis: MenuAxisLatch,
}

#[derive(Resource, Debug, Clone, Copy)]
struct ControlTuning {
    bumper_press_threshold: f32,
    gear_request_buffer_secs: f32,
    menu_axis_threshold: f32,
}

#[derive(Resource, Default)]
pub(crate) struct TouchInput {
    enabled: bool,
    steer: f32,
    brake: f32,
    low: bool,
    high: bool,
    nitro: bool,
    jump: bool,
    pause: bool,
    difficulty: Option<usize>,
    confirm: bool,
    toggle_crt: bool,
    toggle_large: bool,
    restart: bool,
    initials: Option<String>,
    submit: bool,
    pub(crate) active_this_frame: bool,
    previous: bridge::TouchSnapshot,
}

fn sequence_advanced(current: u32, previous: u32) -> bool {
    current != previous
}

impl TouchInput {
    fn update(&mut self, next: bridge::TouchSnapshot) {
        let enabled = next.enabled;
        self.enabled = enabled;
        self.steer = if enabled {
            next.steer.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        self.brake = if enabled {
            next.brake.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.low = enabled && sequence_advanced(next.low_seq, self.previous.low_seq);
        self.high = enabled && sequence_advanced(next.high_seq, self.previous.high_seq);
        self.nitro = enabled && sequence_advanced(next.nitro_seq, self.previous.nitro_seq);
        self.jump = enabled && sequence_advanced(next.jump_seq, self.previous.jump_seq);
        self.pause = enabled && sequence_advanced(next.pause_seq, self.previous.pause_seq);
        let difficulty_changed =
            enabled && sequence_advanced(next.difficulty_seq, self.previous.difficulty_seq);
        self.difficulty = difficulty_changed
            .then_some(next.difficulty_index)
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < Difficulty::ALL.len());
        self.confirm = enabled && sequence_advanced(next.confirm_seq, self.previous.confirm_seq);
        self.toggle_crt = enabled && sequence_advanced(next.crt_seq, self.previous.crt_seq);
        self.toggle_large = enabled && sequence_advanced(next.large_seq, self.previous.large_seq);
        self.restart = enabled && sequence_advanced(next.restart_seq, self.previous.restart_seq);
        let initials_changed =
            enabled && sequence_advanced(next.initials_seq, self.previous.initials_seq);
        self.initials = initials_changed.then(|| next.initials.clone());
        self.submit = enabled && sequence_advanced(next.submit_seq, self.previous.submit_seq);
        self.active_this_frame = enabled
            && (self.steer != 0.0
                || self.brake != 0.0
                || self.low
                || self.high
                || self.nitro
                || self.jump
                || self.pause
                || self.difficulty.is_some()
                || self.confirm
                || self.toggle_crt
                || self.toggle_large
                || self.restart
                || self.initials.is_some()
                || self.submit);
        self.previous = next;
    }
}

fn poll_touch(mut touch: ResMut<TouchInput>) {
    touch.update(bridge::touch_snapshot());
}

impl Default for ControlTuning {
    fn default() -> Self {
        Self {
            bumper_press_threshold: 0.28,
            gear_request_buffer_secs: 0.15,
            menu_axis_threshold: 0.55,
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
        if !(0.20..=0.90).contains(&self.menu_axis_threshold) {
            return Err("menu axis threshold out of [0.20, 0.90]");
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct GearRequestLatch {
    request: Option<game_core::Gear>,
    remaining: f32,
}

#[derive(Debug, Default)]
struct MenuAxisLatch {
    direction: i8,
}

impl MenuAxisLatch {
    fn update(&mut self, axis: f32, threshold: f32) -> (bool, bool) {
        let direction = if axis <= -threshold {
            -1
        } else if axis >= threshold {
            1
        } else {
            0
        };
        let pressed = direction != 0 && direction != self.direction;
        self.direction = direction;
        (pressed && direction < 0, pressed && direction > 0)
    }
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
    #[cfg(debug_assertions)]
    let dev_ending = dev_ending_requested(bridge::query_param("dev_ending").as_deref());
    #[cfg(not(debug_assertions))]
    let dev_ending = false;
    let coarse_pointer = bridge::coarse_pointer();
    let render_quality =
        RenderQuality::from_query(bridge::query_param("quality").as_deref(), coarse_pointer);
    let (qa_mode, qa_duration) = session::qa_mode_and_duration(qa_parameter.as_deref());
    let qa_start_environment_time = qa_mode
        .then(|| bridge::query_param("qa_start_environment_time"))
        .flatten()
        .as_deref()
        .and_then(session::validated_qa_environment_time);
    let transient_run = qa_mode || dev_ending;
    let (leaderboard, persistent) = if transient_run {
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
    let initial_game = if let Some(duration) = qa_duration {
        let mut rng = game_core::Rng::new(20261015);
        let road =
            game_core::RoadLoop::generate_with_traffic_bonus(&mut rng, tuning.traffic_medium_extra);
        Game::with_road_for_duration(20261015, road, Difficulty::Medium, duration, &tuning)
    } else {
        Game::new(20261015, Difficulty::Medium, &tuning)
    };
    #[cfg(debug_assertions)]
    let initial_game = if dev_ending {
        dev_ending_game(&tuning)
    } else {
        initial_game
    };
    let mut initial_end = EndFlow::default();
    if dev_ending {
        initial_end.mode = EndMode::Playing;
    }
    let mut app = App::new();
    let audio_muted = bridge::load_local(audio::STORAGE_KEY).0.as_deref() == Some("muted");
    let crt_setting = bridge::load_local(toy_view::CRT_STORAGE_KEY).0;
    let crt_enabled = initial_crt_enabled(crt_setting.as_deref(), coarse_pointer);
    let large_setting = bridge::load_local(toy_view::LARGE_SCREEN_STORAGE_KEY).0;
    let large_screen = initial_large_screen(large_setting.as_deref());

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
                    // Pixel art gains no useful detail from a 3x mobile DPR,
                    // while the extra fragments make bloom/CRT/3D needlessly
                    // expensive. Render one physical pixel per CSS pixel.
                    resolution: bevy::window::WindowResolution::new(960, 600)
                        .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(MaterialPlugin::<toy_view::CrtScreenMaterial>::default())
    .add_plugins(audio::ArcadeAudioPlugin)
    .insert_resource(ClearColor(Color::srgb(0.025, 0.031, 0.055)))
    .insert_resource(TuningRes(tuning))
    .insert_resource(control_tuning)
    .insert_resource(RunConfig {
        qa_duration,
        qa_start_environment_time,
    })
    .insert_resource(SceneryRun::new(transient_run))
    .insert_resource(toy_view::StartupIntro::new(!transient_run))
    .insert_resource(render_quality)
    .insert_resource(LeaderboardState {
        entries: leaderboard,
        persistent,
        qa_mode,
    })
    .insert_resource(DisplaySettings {
        crt_enabled,
        large_screen,
    })
    // Seeded technical loop: mixed-radius corners, rolling elevation,
    // traffic, and collidable roadside scenery.
    .insert_resource(Sim(initial_game))
    .init_resource::<FrameInput>()
    .init_resource::<TouchInput>()
    .init_resource::<VisualEffects>()
    .insert_resource(EndSession(initial_end))
    .init_resource::<FinishSequence>()
    .init_resource::<LaunchSequence>()
    .init_resource::<MenuIdle>()
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
    .add_systems(
        PreUpdate,
        (poll_touch, gather_input)
            .chain()
            .after(bevy::input::InputSystems),
    )
    .add_systems(
        Update,
        (sync_end_flow, tick_launch_sequence, tick_menu_idle).in_set(AppSet::Flow),
    )
    .add_systems(
        Update,
        (toggle_large_screen, handle_end_input)
            .chain()
            .in_set(AppSet::Controls),
    )
    .add_systems(
        Update,
        (
            tick_visual_effects,
            (toy_view::spawn_toy, toy_view::paint_screen).chain(),
            toy_view::decorate_control_shines,
            toy_view::animate_startup_intro,
            toy_view::toggle_mode,
            (toy_view::apply_body_dynamics, toy_view::animate_parts).chain(),
            toy_view::turbo_lamp,
            toy_view::animate_room_shooting_stars,
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

fn initial_crt_enabled(stored: Option<&str>, coarse_pointer: bool) -> bool {
    match stored {
        Some("on") => true,
        Some("off") => false,
        _ => !coarse_pointer,
    }
}

fn initial_large_screen(stored: Option<&str>) -> bool {
    stored == Some("on")
}

fn toggle_large_screen(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    touch: Res<TouchInput>,
    end: Res<EndSession>,
    intro: Res<toy_view::StartupIntro>,
    mut display: ResMut<DisplaySettings>,
    mut audio_events: ResMut<audio::AudioEvents>,
) {
    if !intro.complete() {
        return;
    }
    let keyboard_allowed = matches!(end.0.mode, EndMode::DifficultySelect | EndMode::Playing);
    let gamepad_requested = keyboard_allowed
        && pads.iter().any(|pad| {
            large_screen_chord(
                pad.pressed(GamepadButton::LeftThumb),
                pad.pressed(GamepadButton::RightThumb),
                pad.just_pressed(GamepadButton::LeftThumb),
                pad.just_pressed(GamepadButton::RightThumb),
            )
        });
    if !touch.toggle_large
        && !(keyboard_allowed && keys.just_pressed(KeyCode::KeyV))
        && !gamepad_requested
    {
        return;
    }
    display.large_screen = !display.large_screen;
    let value = if display.large_screen { "on" } else { "off" };
    let _ = bridge::save_local(toy_view::LARGE_SCREEN_STORAGE_KEY, value);
    audio_events.emit(audio::AudioCue::MenuMove);
}

fn large_screen_chord(
    left_pressed: bool,
    right_pressed: bool,
    left_just_pressed: bool,
    right_just_pressed: bool,
) -> bool {
    left_pressed && right_pressed && (left_just_pressed || right_just_pressed)
}

#[allow(clippy::too_many_arguments)]
fn tick_menu_idle(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pads: Query<&Gamepad>,
    touch: Res<TouchInput>,
    end: Res<EndSession>,
    intro: Res<toy_view::StartupIntro>,
    mut idle: ResMut<MenuIdle>,
) {
    if end.0.mode != EndMode::DifficultySelect || !intro.complete() {
        idle.seconds = 0.0;
        return;
    }
    let interacted = keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
        || pads
            .iter()
            .any(|pad| pad.get_just_pressed().next().is_some())
        || touch.active_this_frame;
    idle.seconds = if interacted {
        0.0
    } else {
        idle.seconds + time.delta_secs()
    };
}

/// M5 interim input: keyboard-first, gamepad passthrough (formal layer = M6).
#[allow(clippy::too_many_arguments)]
fn gather_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut frame: ResMut<FrameInput>,
    tuning: Res<TuningRes>,
    controls: Res<ControlTuning>,
    time: Res<Time>,
    end: Res<EndSession>,
    touch: Res<TouchInput>,
    launch: Res<LaunchSequence>,
    sim: Res<Sim>,
) {
    if end.0.mode != EndMode::Playing || launch.active {
        frame.state.steer = 0.0;
        frame.state.brake = 0.0;
        frame.state.gear_request = None;
        frame.state.pause = false;
        frame.state.nitro = false;
        frame.state.jump = false;
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
    if touch.enabled && touch.steer != 0.0 {
        steer = touch.steer;
    }

    let mut low_requested = keys.just_pressed(KeyCode::KeyQ) || touch.low;
    let mut high_requested = keys.just_pressed(KeyCode::KeyE) || touch.high;
    let mut low_bumper_down = false;
    let mut high_bumper_down = false;
    let mut brake = touch.brake;
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
            // During pause, Select is handled by the menu flow below and
            // returns to run selection instead of silently restarting.
            if !sim.0.paused {
                frame.state.reset = true;
            }
            frame.pad_seen = true;
        }
        if pad.just_pressed(GamepadButton::North) {
            frame.state.nitro = true;
            frame.pad_seen = true;
        }
        if pad.just_pressed(GamepadButton::East) {
            frame.state.jump = true;
            frame.pad_seen = true;
        }
        // Xbox face-button aliases: X shifts up, A shifts down. They are
        // handled only during gameplay, so A remains menu confirmation.
        if pad.just_pressed(GamepadButton::West) {
            high_requested = true;
            frame.pad_seen = true;
        }
        if pad.just_pressed(GamepadButton::South) {
            low_requested = true;
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
    if touch.pause {
        frame.state.pause = true;
    }
    if keys.just_pressed(KeyCode::KeyY) {
        frame.state.nitro = true;
    }
    if touch.nitro {
        frame.state.nitro = true;
    }
    if keys.just_pressed(KeyCode::KeyB) || keys.just_pressed(KeyCode::Space) {
        frame.state.jump = true;
    }
    if touch.jump {
        frame.state.jump = true;
    }
}

fn tick_launch_sequence(time: Res<Time>, mut launch: ResMut<LaunchSequence>) {
    if !launch.active {
        return;
    }
    launch.elapsed = (launch.elapsed + time.delta_secs()).min(SKY_DROP_DURATION_SECS);
    if launch.elapsed >= SKY_DROP_DURATION_SECS {
        launch.active = false;
    }
}

fn sync_end_flow(
    time: Res<Time>,
    sim: Res<Sim>,
    mut end: ResMut<EndSession>,
    mut finish: ResMut<FinishSequence>,
) {
    match sim.0.phase {
        game_core::Phase::GameOver { .. }
            if end.0.mode == EndMode::Playing && sim.0.result().is_some() =>
        {
            if !finish.active {
                finish.active = true;
                finish.elapsed = 0.0;
            }
            finish.elapsed =
                (finish.elapsed + time.delta_secs()).min(FINISH_SEQUENCE_DURATION_SECS);
            if finish.elapsed >= FINISH_SEQUENCE_DURATION_SECS {
                finish.active = false;
                end.0.begin_entry();
            }
        }
        _ => {
            finish.active = false;
            finish.elapsed = 0.0;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_end_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut sim: ResMut<Sim>,
    tuning: Res<TuningRes>,
    controls: Res<ControlTuning>,
    config: Res<RunConfig>,
    mut scenery: ResMut<SceneryRun>,
    mut end: ResMut<EndSession>,
    mut leaderboard: ResMut<LeaderboardState>,
    mut display: ResMut<DisplaySettings>,
    mut frame: ResMut<FrameInput>,
    mut audio_events: ResMut<audio::AudioEvents>,
    touch: Res<TouchInput>,
    mut launch: ResMut<LaunchSequence>,
    intro: Res<toy_view::StartupIntro>,
) {
    if !intro.complete() {
        return;
    }
    match end.0.mode {
        EndMode::Playing => {
            let select = pads
                .iter()
                .any(|pad| pad.just_pressed(GamepadButton::Select));
            let start = pads
                .iter()
                .any(|pad| pad.just_pressed(GamepadButton::Start));
            if restart_to_menu_requested(
                sim.0.paused,
                select,
                start,
                keys.just_pressed(KeyCode::Escape),
                touch.restart,
            ) {
                audio_events.emit(audio::AudioCue::MenuConfirm);
                end.0.mode = EndMode::DifficultySelect;
                end.0.difficulty_index = difficulty_index(sim.0.difficulty());
                end.0.initials = session::Initials::default();
                sim.0.return_to_ready(&tuning.0);
                frame.state = InputState::default();
                frame.menu_axis = MenuAxisLatch::default();
            }
        }
        EndMode::DifficultySelect => {
            let previous_index = end.0.difficulty_index;
            let menu_axis = pads
                .iter()
                .filter_map(|pad| pad.get(GamepadAxis::LeftStickX))
                .max_by(|left, right| left.abs().total_cmp(&right.abs()))
                .unwrap_or_default();
            let (stick_left, stick_right) = frame
                .menu_axis
                .update(menu_axis, controls.menu_axis_threshold);
            let left = keys.just_pressed(KeyCode::ArrowLeft)
                || stick_left
                || pads
                    .iter()
                    .any(|pad| pad.just_pressed(GamepadButton::DPadLeft));
            let right = keys.just_pressed(KeyCode::ArrowRight)
                || stick_right
                || pads
                    .iter()
                    .any(|pad| pad.just_pressed(GamepadButton::DPadRight));
            end.0.difficulty_index = menu_index_after_input(end.0.difficulty_index, left, right);
            if let Some(index) = touch.difficulty {
                end.0.difficulty_index = index;
            }
            if end.0.difficulty_index != previous_index {
                audio_events.emit(audio::AudioCue::MenuMove);
            }
            let toggle_crt = touch.toggle_crt
                || keys.just_pressed(KeyCode::KeyC)
                || pads.iter().any(|pad| pad.just_pressed(GamepadButton::West));
            if toggle_crt {
                display.crt_enabled = !display.crt_enabled;
                let value = if display.crt_enabled { "on" } else { "off" };
                let _ = bridge::save_local(toy_view::CRT_STORAGE_KEY, value);
                audio_events.emit(audio::AudioCue::MenuMove);
            }
            let confirm = touch.confirm
                || keys.just_pressed(KeyCode::Enter)
                || pads.iter().any(|pad| {
                    pad.just_pressed(GamepadButton::South) || pad.just_pressed(GamepadButton::Start)
                });
            if confirm {
                audio_events.emit(audio::AudioCue::MenuConfirm);
                audio_events.emit(audio::AudioCue::RunStart);
                scenery.advance();
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
                    sim.0 = Game::new(sim.0.seed, difficulty, &tuning.0);
                    let _ = sim.0.step(
                        &InputState {
                            reset: true,
                            ..default()
                        },
                        0.0,
                        &tuning.0,
                    );
                }
                if let Some(environment_time) = config.qa_start_environment_time {
                    sim.0.set_environment_progress(environment_time);
                    sim.0.distance = environment_time * tuning.0.scroll_speed_low;
                } else {
                    sim.0
                        .set_environment_start(scenery.environment_start(bridge::local_calendar()));
                }
                end.0.mode = EndMode::Playing;
                launch.active = true;
                launch.elapsed = 0.0;
                frame.state = InputState::default();
                frame.kb = KeyboardSteering::default();
                frame.menu_axis = MenuAxisLatch::default();
                frame.suppress_gameplay_once = true;
            }
        }
        EndMode::NameEntry => {
            if let Some(initials) = touch.initials.as_deref() {
                while end.0.initials.pop() {}
                for letter in initials.chars().filter(char::is_ascii_alphabetic).take(3) {
                    let _ = end.0.initials.push(letter.to_ascii_uppercase());
                }
                audio_events.emit(audio::AudioCue::Key);
            } else if keys.just_pressed(KeyCode::Backspace) {
                if end.0.initials.pop() {
                    audio_events.emit(audio::AudioCue::Delete);
                }
            } else if let Some(letter) = just_pressed_letter(&keys) {
                if end.0.initials.push(letter) {
                    audio_events.emit(audio::AudioCue::Key);
                }
            }
            if touch.submit || keys.just_pressed(KeyCode::Enter) {
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
            let start = pads
                .iter()
                .any(|pad| pad.just_pressed(GamepadButton::Start));
            if leaderboard_restart_requested(
                touch.restart,
                touch.confirm,
                keys.just_pressed(KeyCode::KeyR),
                keys.just_pressed(KeyCode::Enter),
                start,
            ) {
                audio_events.emit(audio::AudioCue::MenuConfirm);
                end.0.mode = EndMode::DifficultySelect;
                end.0.difficulty_index = difficulty_index(sim.0.difficulty());
                end.0.initials = session::Initials::default();
                frame.state = InputState::default();
            }
        }
    }
}

fn restart_to_menu_requested(
    paused: bool,
    select: bool,
    start: bool,
    escape: bool,
    touch_restart: bool,
) -> bool {
    touch_restart || escape || (select && (paused || start))
}

fn leaderboard_restart_requested(
    touch_restart: bool,
    touch_confirm: bool,
    key_r: bool,
    enter: bool,
    gamepad_start: bool,
) -> bool {
    touch_restart || touch_confirm || key_r || enter || gamepad_start
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

#[allow(clippy::too_many_arguments)]
fn step_sim(
    mut sim: ResMut<Sim>,
    mut frame: ResMut<FrameInput>,
    mut effects: ResMut<VisualEffects>,
    mut crash: ResMut<toy_view::CrashPulse>,
    tuning: Res<TuningRes>,
    time: Res<Time>,
    mut audio_events: ResMut<audio::AudioEvents>,
    launch: Res<LaunchSequence>,
) {
    if launch.active {
        frame.state = InputState::default();
        frame.gear_latch.consume();
        return;
    }
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
    if events.jump_collected {
        effects.jump_pickup_glow = JUMP_PICKUP_GLOW_SECS;
        audio_events.emit(audio::AudioCue::JumpPickup);
    }
    if events.jump_started {
        audio_events.emit(audio::AudioCue::JumpStart);
    }
    if events.jump_landed && events.bump_direction.is_none() {
        audio_events.emit(audio::AudioCue::JumpLand);
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
    frame.state.jump = false;
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
    effects.jump_pickup_glow = (effects.jump_pickup_glow - time.delta_secs()).max(0.0);
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

    #[cfg(debug_assertions)]
    #[test]
    fn dev_ending_flag_accepts_explicit_truthy_values_only() {
        assert!(dev_ending_requested(Some("1")));
        assert!(dev_ending_requested(Some("true")));
        assert!(dev_ending_requested(Some("ON")));
        assert!(!dev_ending_requested(None));
        assert!(!dev_ending_requested(Some("0")));
        assert!(!dev_ending_requested(Some("false")));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn dev_ending_game_starts_with_a_frozen_result() {
        let game = dev_ending_game(&Tuning::default());
        assert!(matches!(
            game.phase,
            game_core::Phase::GameOver {
                reason: game_core::EndReason::Time
            }
        ));
        assert!(game.result().is_some());
        assert_eq!(game.session_remaining, 0.0);
    }

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
    fn qa_scenery_seed_is_repeatable_and_fresh_runs_advance() {
        let mut scenery = SceneryRun::new(true);
        assert_eq!(scenery.entropy, 20_261_015);
        assert_eq!(scenery.serial, 0);
        let calendar = bridge::LocalCalendar {
            month: 10,
            day: 19,
            hour: 2,
            minute: 30,
            southern: true,
        };
        let first = scenery.environment_start(calendar);
        scenery.advance();
        assert_eq!(scenery.serial, 1);
        let second = scenery.environment_start(calendar);
        assert!((0.0..game_core::environment::YEAR_SECS).contains(&first));
        assert!((0.0..game_core::environment::YEAR_SECS).contains(&second));
        assert_eq!(first, second);
        assert!(game_core::environment::season_index(first) < 1.0);
        assert!(game_core::environment::season_index(second) < 1.0);
    }

    #[test]
    fn local_date_selects_the_correct_hemisphere_season() {
        let northern_october = calendar_environment_start(bridge::LocalCalendar {
            month: 10,
            day: 19,
            hour: 12,
            minute: 0,
            southern: false,
        });
        let southern_october = calendar_environment_start(bridge::LocalCalendar {
            month: 10,
            day: 19,
            hour: 12,
            minute: 0,
            southern: true,
        });
        assert!((2.0..3.0).contains(&game_core::environment::season_index(northern_october)));
        assert!((0.0..1.0).contains(&game_core::environment::season_index(southern_october)));
    }

    #[test]
    fn local_clock_selects_matching_daylight() {
        let at = |hour| {
            calendar_environment_start(bridge::LocalCalendar {
                month: 10,
                day: 19,
                hour,
                minute: 0,
                southern: true,
            })
        };
        assert!(game_core::environment::daylight(at(12)) > 0.99);
        assert!(game_core::environment::daylight(at(0)) < 0.01);
        let dawn = game_core::environment::daylight(at(6));
        let dusk = game_core::environment::daylight(at(18));
        assert!((dawn - 0.5).abs() < 0.01);
        assert!((dusk - 0.5).abs() < 0.01);
    }

    #[test]
    fn pause_select_and_leaderboard_start_have_distinct_navigation() {
        assert!(restart_to_menu_requested(true, true, false, false, false));
        assert!(restart_to_menu_requested(false, true, true, false, false));
        assert!(restart_to_menu_requested(false, false, false, true, false));
        assert!(restart_to_menu_requested(false, false, false, false, true));
        assert!(!restart_to_menu_requested(false, true, false, false, false));
        assert!(leaderboard_restart_requested(
            false, false, false, false, true
        ));
        assert!(!leaderboard_restart_requested(
            false, false, false, false, false
        ));
    }

    #[test]
    fn menu_stick_moves_once_until_released_or_reversed() {
        let mut latch = MenuAxisLatch::default();
        assert_eq!(latch.update(0.8, 0.55), (false, true));
        assert_eq!(latch.update(0.9, 0.55), (false, false));
        assert_eq!(latch.update(0.0, 0.55), (false, false));
        assert_eq!(latch.update(-0.8, 0.55), (true, false));
        assert_eq!(latch.update(0.8, 0.55), (false, true));
    }

    #[test]
    fn touch_sequences_emit_each_action_once_and_clamp_analogs() {
        let mut touch = TouchInput::default();
        let snapshot = bridge::TouchSnapshot {
            enabled: true,
            steer: 4.0,
            brake: 2.0,
            jump_seq: 1,
            high_seq: 3,
            large_seq: 1,
            difficulty_seq: 2,
            difficulty_index: 2,
            ..default()
        };
        touch.update(snapshot.clone());
        assert_eq!(touch.steer, 1.0);
        assert_eq!(touch.brake, 1.0);
        assert!(touch.jump && touch.high && touch.toggle_large);
        assert_eq!(touch.difficulty, Some(2));
        assert!(touch.active_this_frame);

        touch.update(snapshot);
        assert!(!touch.jump && !touch.high && !touch.toggle_large);
        assert_eq!(touch.difficulty, None);
    }

    #[test]
    fn disabled_touch_snapshot_releases_continuous_controls() {
        let mut touch = TouchInput::default();
        touch.update(bridge::TouchSnapshot {
            enabled: true,
            steer: -1.0,
            brake: 1.0,
            ..default()
        });
        assert_eq!((touch.steer, touch.brake), (-1.0, 1.0));
        touch.update(bridge::TouchSnapshot {
            enabled: false,
            jump_seq: 1,
            ..default()
        });
        assert_eq!((touch.steer, touch.brake), (0.0, 0.0));
        assert!(!touch.jump);
        assert!(!touch.active_this_frame);
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
        assert!(ControlTuning {
            menu_axis_threshold: 0.0,
            ..ControlTuning::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn low_quality_preserves_aspect_and_disables_bloom() {
        let high = RenderQuality::from_query(None, false);
        let low = RenderQuality::from_query(Some("LOW"), false);
        assert!(high.bloom);
        assert!(!low.bloom);
        assert!(low.game_width < high.game_width);
        assert_eq!(
            high.game_width * low.game_height,
            low.game_width * high.game_height
        );
    }

    #[test]
    fn coarse_pointer_defaults_low_but_explicit_high_wins() {
        assert!(!RenderQuality::from_query(None, true).bloom);
        assert!(RenderQuality::from_query(Some("high"), true).bloom);
    }

    #[test]
    fn mobile_defaults_crt_off_but_respects_saved_choice() {
        assert!(!initial_crt_enabled(None, true));
        assert!(initial_crt_enabled(None, false));
        assert!(initial_crt_enabled(Some("on"), true));
        assert!(!initial_crt_enabled(Some("off"), false));
    }

    #[test]
    fn large_screen_defaults_off_but_respects_saved_choice() {
        assert!(!initial_large_screen(None));
        assert!(initial_large_screen(Some("on")));
        assert!(!initial_large_screen(Some("off")));
        assert!(!initial_large_screen(Some("invalid")));
    }

    #[test]
    fn large_screen_gamepad_chord_is_edge_triggered_and_requires_both_sticks() {
        assert!(large_screen_chord(true, true, true, false));
        assert!(large_screen_chord(true, true, false, true));
        assert!(!large_screen_chord(true, true, false, false));
        assert!(!large_screen_chord(true, false, true, false));
        assert!(!large_screen_chord(false, true, false, true));
    }

    #[test]
    fn menu_headlights_wait_then_double_blink() {
        assert_eq!(menu_headlight_blink(29.999), 0.0);
        assert_eq!(menu_headlight_blink(30.0), 0.0);
        assert!(menu_headlight_blink(30.11) > 0.9);
        assert!(menu_headlight_blink(30.47) > 0.9);
        assert_eq!(menu_headlight_blink(31.0), 0.0);
        assert!(menu_headlight_blink(36.11) > 0.9);
    }
}
