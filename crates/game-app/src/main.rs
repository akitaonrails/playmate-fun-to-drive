//! M4 bootstrap scene (PLAN.md): blank-playable canvas + gamepad-on-web spike.
//!
//! Deliberately throwaway scaffolding that proves the whole loop:
//! wasm build -> trunk serve -> input (keyboard + gamepad analog) -> render
//! -> Playwright screenshot. Replaced by the real game in M5.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Fun to Drive — dev".into(),
                resolution: bevy::window::WindowResolution::new(960, 600),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.09)))
        .add_systems(Startup, setup)
        .add_systems(Update, (steer_car, scroll_stripes, update_hud))
        .run();
}

#[derive(Component)]
struct Car;

#[derive(Component)]
struct Stripe;

#[derive(Component)]
struct Hud;

#[derive(Resource, Default)]
struct SpikeState {
    steer: f32,
    pad_seen: bool,
    gear: game_core::Gear,
}

fn setup(mut commands: Commands) {
    commands.insert_resource(SpikeState::default());
    commands.spawn(Camera2d);

    // placeholder "car": fixed bottom-center, like the toy's decal
    commands.spawn((
        Sprite::from_color(Color::srgb(0.85, 0.12, 0.10), Vec2::new(120.0, 56.0)),
        Transform::from_xyz(0.0, -190.0, 0.0),
        Car,
    ));

    // placeholder "road stripes": scrolling rows for the M4 screenshot check
    for i in 0..6 {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.30, 0.34, 0.40), Vec2::new(560.0, 14.0)),
            Transform::from_xyz(0.0, -250.0 + i as f32 * 100.0, -1.0),
            Stripe,
        ));
    }

    commands.spawn((
        Text::new("booting…"),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        Hud,
    ));
}

/// M4 spike: keyboard + gamepad analog -> one steer value (game-core does the math).
fn steer_car(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut car: Query<&mut Transform, (With<Car>, Without<Hud>)>,
    mut state: ResMut<SpikeState>,
    time: Res<Time>,
) {
    let tuning = game_core::Tuning::default();

    let mut raw = 0.0;
    if keys.pressed(KeyCode::ArrowLeft) {
        raw -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        raw += 1.0;
    }

    for pad in &pads {
        let raw_axis = pad.get(GamepadAxis::LeftStickX).unwrap_or_default();
        let x = game_core::input::apply_deadzone(raw_axis, tuning.pad_deadzone);
        if x != 0.0 {
            raw = x;
            state.pad_seen = true;
        }
        if pad.just_pressed(GamepadButton::South) || pad.just_pressed(GamepadButton::RightTrigger2)
        {
            state.gear = state.gear.toggle();
        }
    }
    if keys.just_pressed(KeyCode::KeyG) {
        state.gear = state.gear.toggle();
    }

    state.steer = raw;
    if let Ok(mut tf) = car.single_mut() {
        tf.translation.x = raw * tuning.steer_max * 330.0;
    }
    let _ = time.delta();
}

fn scroll_stripes(mut stripes: Query<&mut Transform, With<Stripe>>, time: Res<Time>) {
    let tuning = game_core::Tuning::default();
    let speed = 260.0
        * match game_core::Gear::Low {
            game_core::Gear::Low => tuning.scroll_speed_low,
            game_core::Gear::High => tuning.scroll_speed_high,
        };
    for mut tf in &mut stripes {
        tf.translation.y -= speed * time.delta_secs();
        if tf.translation.y < -260.0 {
            tf.translation.y += 600.0;
        }
    }
}

fn update_hud(state: Res<SpikeState>, mut hud: Query<&mut Text, With<Hud>>) {
    if let Ok(mut text) = hud.single_mut() {
        **text = format!(
            "fun-to-drive dev · game-core v{} · steer {:+.2} · gear {:?} · gamepad: {}",
            game_core::version(),
            state.steer,
            state.gear,
            if state.pad_seen {
                "analog OK"
            } else {
                "press/tilt to test"
            },
        );
    }
}
