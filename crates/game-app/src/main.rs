//! Fun to Drive — M5 app wiring: fixed-step simulation + input + view.
//!
//! Deterministic game logic lives in game-core; this crate only gathers
//! input, steps the sim on the fixed schedule, renders, and (on wasm)
//! publishes state for the Playwright harness.

mod bridge;
mod toy_view;
mod view;

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use game_core::{Game, InputState, KeyboardSteering, Tuning};

#[derive(Resource)]
struct Sim(pub Game);

#[derive(Resource)]
struct TuningRes(pub Tuning);

#[derive(Resource, Default)]
struct FrameInput {
    pub(crate) state: InputState,
    pub(crate) pad_seen: bool,
    kb: KeyboardSteering,
}

fn main() {
    let tuning = Tuning::default();
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
    .insert_resource(Sim({
        // square feel-test track: straights + 4 constant corners (user req)
        let mut rng = game_core::Rng::new(20261015);
        let mut road = game_core::RoadLoop::generate(&mut rng);
        road.apply_square_curve();
        Game::with_road(20261015, road, &tuning)
    }))
    .init_resource::<FrameInput>()
    .add_systems(
        Startup,
        (view::setup, toy_view::setup, toy_view::setup_diorama),
    )
    .add_systems(
        Update,
        (
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
) {
    // keyboard short/long-press shaping (M6, logic tested in game-core)
    let mut steer = frame.kb.update(
        keys.pressed(KeyCode::ArrowLeft),
        keys.pressed(KeyCode::ArrowRight),
        time.delta_secs(),
        &tuning.0,
    );

    for pad in &pads {
        let axis = pad.get(GamepadAxis::LeftStickX).unwrap_or_default();
        let x = game_core::input::apply_deadzone(axis, tuning.0.pad_deadzone);
        if x != 0.0 {
            steer = x; // analog stick overrides the keyboard shape
            frame.pad_seen = true;
        }
        if pad.just_pressed(GamepadButton::South) || pad.just_pressed(GamepadButton::RightTrigger2)
        {
            frame.state.gear_up = true;
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
    if keys.just_pressed(KeyCode::KeyG) {
        frame.state.gear_up = true;
    }
    if keys.just_pressed(KeyCode::KeyR) || keys.just_pressed(KeyCode::Enter) {
        frame.state.reset = true;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        frame.state.pause = true;
    }
    if keys.just_pressed(KeyCode::KeyY) {
        frame.state.nitro = true;
    }
}

fn step_sim(
    mut sim: ResMut<Sim>,
    mut frame: ResMut<FrameInput>,
    tuning: Res<TuningRes>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let _ = sim.0.step(&frame.state, dt, &tuning.0);
    // edge-triggered flags are consumed by exactly one fixed step
    frame.state.gear_up = false;
    frame.state.reset = false;
    frame.state.pause = false;
    frame.state.nitro = false;
}

fn publish_state(
    sim: Res<Sim>,
    frame: Res<FrameInput>,
    view: Res<toy_view::ViewState>,
    rig: Res<toy_view::BodyRig>,
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
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let payload = format!(
        "{{\"phase\":\"{}\",\"score\":{},\"speed\":{:.3},\"session_remaining\":{:.3},\"environment_time\":{:.3},\"day_cycles\":{},\"cars_passed\":{},\"cars_bumped\":{},\"session_result\":{},\"weather_current\":\"{}\",\"weather_next\":\"{}\",\"weather_transition\":{:.3},\"weather_slot\":{},\"daylight\":{:.3},\"season\":{:.3},\"gear\":\"{}\",\"nitro\":{},\"nitro_t\":{:.3},\"nitro_ahead\":{:.3},\"nitro_lane\":{:.2},\"toy\":{},\"look\":{:.2},\"paused\":{},\"steer\":{:.3},\"road_x\":{:.4},\"off_road\":{:.3},\"roll\":{:.4},\"pad\":{}}}",
        match sim.0.phase {
            game_core::Phase::Ready => "ready",
            game_core::Phase::Running => "running",
            game_core::Phase::GameOver { .. } => "gameover",
        },
        score,
        sim.0.speed,
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
        game_core::daylight(sim.0.environment_time),
        game_core::environment::season_index(sim.0.environment_time),
        match sim.0.gear() {
            game_core::Gear::Low => "low",
            game_core::Gear::High => "high",
        },
        sim.0.nitro_charges,
        sim.0.nitro_t,
        sim.0.nitro_pickup_ahead(),
        sim.0.next_nitro_lane,
        view.mode == toy_view::ViewMode::Toy,
        view.toy_look,
        sim.0.paused,
        frame.state.steer,
        sim.0.road_x,
        sim.0.off_road,
        rig.0.roll,
        frame.pad_seen,
    );
    #[cfg(target_arch = "wasm32")]
    bridge::publish_state(&payload);
}

fn weather_name(weather: game_core::WeatherKind) -> &'static str {
    match weather {
        game_core::WeatherKind::Clear => "clear",
        game_core::WeatherKind::Cloudy => "cloudy",
        game_core::WeatherKind::Rain => "rain",
    }
}
