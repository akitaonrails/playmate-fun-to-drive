//! Fun to Drive — M5: the real game view (flat modern look).
//!
//! Renders the game-core simulation: the car is a fixed decal at bottom
//! center; the road drum scrolls and shifts under it (research/toy-mechanics).

use crate::{Sim, TuningRes};
use bevy::prelude::*;
use game_core::{EndReason, Gear, Phase};

pub const M_PER_PX: f32 = 0.08;
pub const HALF_ROAD_PX: f32 = 190.0;
pub const CAR_Y: f32 = -215.0;
const ROW_H: f32 = 26.0;
const ROW_COUNT: usize = 22;
const BOTTOM_Y: f32 = -262.0;
const VIEW_TOP: f32 = 322.0;
const HAZARD_POOL: usize = 16;
const FUEL_X_LEFT: f32 = 240.0;
const FUEL_Y: f32 = 270.0;

#[derive(Component)]
struct RoadRow;

#[derive(Component)]
struct EdgeMark;

#[derive(Component)]
struct CenterDash;

#[derive(Component)]
struct HazardSprite;

#[derive(Component)]
struct FlashOverlay;

#[derive(Component)]
struct FuelBarFill;

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct GearText;

#[derive(Component)]
struct StatusText;

#[derive(Resource, Default)]
struct ViewColors {
    asphalt: Color,
    edge: Color,
    dash: Color,
    oncoming: Color,
    roadside: Color,
}

fn flat_view_colors() -> ViewColors {
    ViewColors {
        asphalt: Color::srgb(0.23, 0.25, 0.30),
        edge: Color::srgb(0.85, 0.86, 0.88),
        dash: Color::srgb(0.75, 0.77, 0.80),
        oncoming: Color::srgb(0.92, 0.90, 0.82),
        roadside: Color::srgb(0.95, 0.55, 0.15),
    }
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(flat_view_colors());
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite::from_color(Color::srgb(0.10, 0.13, 0.17), Vec2::new(960.0, 600.0)),
        Transform::from_xyz(0.0, 0.0, -3.0),
    ));

    let asphalt = flat_view_colors().asphalt;
    for i in 0..ROW_COUNT {
        let y = BOTTOM_Y + i as f32 * ROW_H + ROW_H * 0.5;
        commands.spawn((
            Sprite {
                color: asphalt,
                custom_size: Some(Vec2::new(HALF_ROAD_PX * 2.0, ROW_H + 1.0)),
                ..default()
            },
            Transform::from_xyz(0.0, y, -2.0),
            RoadRow,
        ));
        for side in [-1.0f32, 1.0] {
            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::new(8.0, ROW_H - 8.0)),
                    ..default()
                },
                Transform::from_xyz(side * (HALF_ROAD_PX - 4.0), y, -1.5),
                EdgeMark,
            ));
        }
        commands.spawn((
            Sprite {
                custom_size: Some(Vec2::new(6.0, ROW_H - 10.0)),
                ..default()
            },
            Transform::from_xyz(0.0, y, -1.4),
            CenterDash,
        ));
    }

    for _ in 0..HAZARD_POOL {
        commands.spawn((
            Sprite {
                custom_size: Some(Vec2::new(64.0, 40.0)),
                ..default()
            },
            Transform::from_xyz(0.0, CAR_Y, -1.0),
            Visibility::Hidden,
            HazardSprite,
        ));
    }

    // the fixed car "decal": body + windshield hint
    commands.spawn((
        Sprite::from_color(Color::srgb(0.85, 0.12, 0.10), Vec2::new(120.0, 56.0)),
        Transform::from_xyz(0.0, CAR_Y, 0.5),
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.10, 0.12, 0.16), Vec2::new(56.0, 22.0)),
        Transform::from_xyz(0.0, CAR_Y + 6.0, 0.6),
    ));

    // fuel bar (top right)
    commands.spawn((
        Sprite::from_color(Color::srgb(0.12, 0.13, 0.16), Vec2::new(184.0, 18.0)),
        Transform::from_xyz(FUEL_X_LEFT + 90.0, FUEL_Y, 0.8),
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.20, 0.75, 0.30), Vec2::new(180.0, 14.0)),
        Transform::from_xyz(FUEL_X_LEFT + 90.0, FUEL_Y, 0.9),
        FuelBarFill,
    ));

    // HUD text
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(30.0),
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(14.0),
            left: Val::Px(400.0),
            ..default()
        },
        ScoreText,
    ));
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.8, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(54.0),
            left: Val::Px(430.0),
            ..default()
        },
        GearText,
    ));
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.85, 0.4)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(250.0),
            left: Val::Px(290.0),
            ..default()
        },
        StatusText,
    ));

    // crash flash overlay
    commands.spawn((
        Sprite {
            color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            custom_size: Some(Vec2::new(960.0, 600.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 2.0),
        FlashOverlay,
    ));
}

fn row_y(i: usize) -> f32 {
    BOTTOM_Y + i as f32 * ROW_H + ROW_H * 0.5
}

fn row_ahead(i: usize) -> f32 {
    (row_y(i) - CAR_Y) * M_PER_PX
}

#[allow(clippy::type_complexity)]
fn sync_roads(
    sim: Res<Sim>,
    colors: Res<ViewColors>,
    mut rows: Query<&mut Transform, (With<RoadRow>, Without<EdgeMark>, Without<CenterDash>)>,
    mut edges: Query<
        (&mut Transform, &mut Sprite),
        (With<EdgeMark>, Without<RoadRow>, Without<CenterDash>),
    >,
    mut dashes: Query<
        (&mut Transform, &mut Sprite),
        (With<CenterDash>, Without<EdgeMark>, Without<RoadRow>),
    >,
) {
    // rows: x follows the drum
    let xs: Vec<f32> = (0..ROW_COUNT)
        .map(|i| sim.0.road_offset_at(row_ahead(i)) * HALF_ROAD_PX)
        .collect();

    for (i, mut tf) in rows.iter_mut().enumerate() {
        if let Some(x) = xs.get(i) {
            tf.translation.x = *x;
        }
    }
    for (idx, (mut tf, mut sp)) in edges.iter_mut().enumerate() {
        let row = idx / 2;
        let side: f32 = if idx % 2 == 0 { -1.0 } else { 1.0 };
        let Some(x) = xs.get(row) else { continue };
        tf.translation.x = x + side * (HALF_ROAD_PX - 4.0);
        let s_abs = sim.0.distance + row_ahead(row);
        let phase = (s_abs / 7.0 + 0.25 * side).floor();
        sp.color = if (phase as i64).rem_euclid(2) == 0 {
            colors.edge
        } else {
            colors.asphalt
        };
    }
    for (i, (mut tf, mut sp)) in dashes.iter_mut().enumerate() {
        let Some(x) = xs.get(i) else { continue };
        tf.translation.x = *x;
        let s_abs = sim.0.distance + row_ahead(i);
        let on = ((s_abs / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on { colors.dash } else { colors.asphalt };
    }
}

fn sync_hazards(
    sim: Res<Sim>,
    colors: Res<ViewColors>,
    mut pool: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<HazardSprite>>,
) {
    let lp = sim.0.loop_pos();
    let visible: Vec<(f32, f32, game_core::HazardKind)> = sim
        .0
        .road
        .hazards
        .iter()
        .filter_map(|h| {
            let ahead = (h.s - lp).rem_euclid(sim.0.road.length);
            let y = CAR_Y + ahead / M_PER_PX;
            if y < CAR_Y - 40.0 || y > VIEW_TOP {
                return None;
            }
            let x = (sim.0.road_offset_at(ahead) + h.x) * HALF_ROAD_PX;
            Some((x, y, h.kind))
        })
        .collect();

    for (i, (mut tf, mut sp, mut vis)) in pool.iter_mut().enumerate() {
        if let Some(&(x, y, kind)) = visible.get(i) {
            tf.translation = Vec3::new(x, y, -1.0);
            match kind {
                game_core::HazardKind::Oncoming => {
                    sp.color = colors.oncoming;
                    sp.custom_size = Some(Vec2::new(64.0, 40.0));
                }
                game_core::HazardKind::Roadside => {
                    sp.color = colors.roadside;
                    sp.custom_size = Some(Vec2::new(40.0, 40.0));
                }
            }
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn sync_flash(sim: Res<Sim>, mut q: Query<&mut Sprite, With<FlashOverlay>>) {
    if let Ok(mut sp) = q.single_mut() {
        sp.color = Color::srgba(1.0, 1.0, 1.0, sim.0.flash.clamp(0.0, 1.0) * 0.85);
    }
}

#[allow(clippy::type_complexity)]
fn sync_hud(
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    mut fuel: Query<(&mut Transform, &mut Sprite), With<FuelBarFill>>,
    mut score: Query<&mut Text, (With<ScoreText>, Without<GearText>, Without<StatusText>)>,
    mut gear: Query<&mut Text, (With<GearText>, Without<StatusText>)>,
    mut status: Query<&mut Text, With<StatusText>>,
) {
    if let Ok((mut tf, mut sp)) = fuel.single_mut() {
        let frac = (sim.0.fuel / tuning.0.fuel_secs).clamp(0.0, 1.0);
        let w = 180.0 * frac;
        tf.translation.x = FUEL_X_LEFT + w * 0.5;
        sp.custom_size = Some(Vec2::new(w.max(1.0), 14.0));
        sp.color = if frac > 0.5 {
            Color::srgb(0.20, 0.75, 0.30)
        } else if frac > 0.25 {
            Color::srgb(0.90, 0.75, 0.20)
        } else {
            Color::srgb(0.90, 0.25, 0.15)
        };
    }
    if let Ok(mut text) = score.single_mut() {
        **text = format!("{:05.0} m", sim.0.distance);
    }
    if let Ok(mut text) = gear.single_mut() {
        **text = match sim.0.gear() {
            Gear::Low => "LOW".to_string(),
            Gear::High => "HIGH (turbo)".to_string(),
        };
    }
    if let Ok(mut text) = status.single_mut() {
        **text = match &sim.0.phase {
            Phase::Ready => "ARROWS steer | G gear | R restart".to_string(),
            Phase::Running => String::new(),
            Phase::Crashed { .. } => "CRASHED!".to_string(),
            Phase::GameOver { reason } => match reason {
                EndReason::Crash => "GAME OVER - CRASH | R to restart".to_string(),
                EndReason::Fuel => "GAME OVER - OUT OF FUEL | R to restart".to_string(),
            },
        };
    }
}

pub fn add_view_systems(app: &mut App) {
    app.add_systems(
        Update,
        (sync_roads, sync_hazards, sync_flash, sync_hud).chain(),
    );
}
