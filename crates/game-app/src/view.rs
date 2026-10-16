//! Fun to Drive — the 2D drum view, OutRun-inspired (PLAN: user tweak 2).
//!
//! Classic 16-bit racer look: gradient sky with sun and clouds, sea strip at
//! the horizon, sandy shoulder, scrolling grass stripes, red/white rumble
//! edges, lane dashes, palm trees, and composite car sprites — all drawn
//! with flat sprites so it renders identically into the toy's screen.

use bevy::prelude::*;
use game_core::{EndReason, Gear, Phase};

use crate::{Sim, TuningRes};

pub const M_PER_PX: f32 = 0.08;
pub const HALF_ROAD_PX: f32 = 190.0;
pub const CAR_Y: f32 = -215.0;
pub const HORIZON_Y: f32 = 120.0;
const ROW_H: f32 = 26.0;
const ROW_COUNT: usize = 14;
const TOP_ROW_Y: f32 = HORIZON_Y - ROW_H * 0.5;
const VIEW_TOP: f32 = 322.0;
const PALM_POOL: usize = 10;
const HAZARD_POOL: usize = 16;
const FUEL_X_LEFT: f32 = 240.0;
const FUEL_Y: f32 = 270.0;

#[derive(Component)]
struct SkyBand;

#[derive(Component)]
struct SeaStrip;

#[derive(Component)]
struct GrassBase;

#[derive(Component)]
struct RoadRow;

#[derive(Component)]
struct EdgeRumble;

#[derive(Component)]
struct CenterDash;

#[derive(Component)]
struct GrassRow;

#[derive(Component)]
struct HazardPart;

#[derive(Component)]
struct PalmPart;

#[derive(Component)]
struct CarPart;

#[derive(Component)]
struct FlashOverlay;

#[derive(Component)]
struct FuelBarFill;

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct GearText;

#[derive(Component)]
pub struct StatusText;

#[derive(Component)]
pub struct LapText;

/// M10 rolling-tube overlays: bands printed on the drum + recess shading.
#[derive(Component)]
struct DrumBand;

#[derive(Component)]
struct RecessShade;

#[derive(Resource, Default)]
struct Palette {
    asphalt_a: Color,
    asphalt_b: Color,
    grass_a: Color,
    grass_b: Color,
    rumble_red: Color,
    rumble_white: Color,
    dash: Color,
}

fn palette() -> Palette {
    Palette {
        asphalt_a: Color::srgb(0.34, 0.35, 0.40),
        asphalt_b: Color::srgb(0.30, 0.31, 0.36),
        grass_a: Color::srgb(0.16, 0.55, 0.28),
        grass_b: Color::srgb(0.13, 0.47, 0.24),
        rumble_red: Color::srgb(0.85, 0.15, 0.12),
        rumble_white: Color::srgb(0.95, 0.95, 0.92),
        dash: Color::srgb(0.95, 0.95, 0.90),
    }
}

fn format_lap(secs: f32) -> String {
    let m = (secs / 60.0) as u32;
    let s = secs - m as f32 * 60.0;
    format!("{m}:{s:05.2}")
}

fn sky_colors() -> [(Color, f32); 6] {
    [
        (Color::srgb(0.17, 0.27, 0.70), 60.0),
        (Color::srgb(0.25, 0.40, 0.92), 50.0),
        (Color::srgb(0.40, 0.60, 1.00), 44.0),
        (Color::srgb(0.64, 0.84, 1.00), 40.0),
        (Color::srgb(0.90, 0.97, 1.00), 38.0),
        (Color::srgb(0.97, 0.99, 1.00), 36.0),
    ]
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(palette());
    commands.spawn((Camera2d, crate::toy_view::GameCamera));

    // ---- sky gradient bands (static), sun, clouds, sea, sand ----
    let mut y = VIEW_TOP;
    for (color, h) in sky_colors() {
        let h = h.max(y - HORIZON_Y).min(h);
        commands.spawn((
            Sprite::from_color(color, Vec2::new(960.0, h + 1.0)),
            Transform::from_xyz(0.0, y - h * 0.5, -5.0),
            SkyBand,
        ));
        y -= h;
        if y <= HORIZON_Y + 1.0 {
            break;
        }
    }
    // sun (OutRun striped sun) + clouds
    commands.spawn((
        Sprite::from_color(Color::srgb(1.0, 0.85, 0.35), Vec2::new(64.0, 56.0)),
        Transform::from_xyz(-250.0, HORIZON_Y + 78.0, -4.9),
        SkyBand,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(1.0, 0.62, 0.25), Vec2::new(88.0, 14.0)),
        Transform::from_xyz(-250.0, HORIZON_Y + 62.0, -4.8),
        SkyBand,
    ));
    for (cx, cy, w) in [
        (-60.0, 205.0, 90.0),
        (170.0, 240.0, 120.0),
        (330.0, 190.0, 70.0),
    ] {
        commands.spawn((
            Sprite::from_color(Color::srgba(0.95, 0.96, 1.0, 0.85), Vec2::new(w, 12.0)),
            Transform::from_xyz(cx, cy, -4.8),
            SkyBand,
        ));
    }
    // sea + sand at the horizon
    commands.spawn((
        Sprite::from_color(Color::srgb(0.04, 0.52, 0.68), Vec2::new(960.0, 22.0)),
        Transform::from_xyz(0.0, HORIZON_Y - 11.0, -4.7),
        SeaStrip,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.94, 0.86, 0.58), Vec2::new(960.0, 13.0)),
        Transform::from_xyz(0.0, HORIZON_Y - 28.0, -4.6),
        SeaStrip,
    ));
    // static distant grass fills down to the first striped row
    commands.spawn((
        Sprite::from_color(Color::srgb(0.15, 0.52, 0.27), Vec2::new(960.0, 40.0)),
        Transform::from_xyz(0.0, HORIZON_Y - 48.0, -4.5),
        GrassBase,
    ));

    // ---- scrolling world rows: grass stripe + road + rumble + dashes ----
    for i in 0..ROW_COUNT {
        let y = TOP_ROW_Y - i as f32 * ROW_H;
        commands.spawn((
            Sprite {
                custom_size: Some(Vec2::new(HALF_ROAD_PX * 2.0, ROW_H + 1.0)),
                ..default()
            },
            Transform::from_xyz(0.0, y, -4.0),
            RoadRow,
        ));
        commands.spawn((
            Sprite {
                custom_size: Some(Vec2::new(960.0, ROW_H + 1.0)),
                ..default()
            },
            Transform::from_xyz(0.0, y, -4.4),
            GrassRow,
        ));
        for side in [-1.0f32, 1.0] {
            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::new(14.0, ROW_H - 6.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, y, -3.9),
                EdgeRumble,
            ));
            let _ = side;
        }
        for lane in [-0.4f32, 0.4] {
            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::new(7.0, ROW_H - 10.0)),
                    ..default()
                },
                Transform::from_xyz(lane * HALF_ROAD_PX, y, -3.8),
                CenterDash,
            ));
        }
    }

    // ---- palms (pooled, ride the loop) ----
    for _ in 0..PALM_POOL {
        // trunk + fronds
        commands.spawn((
            Sprite::from_color(Color::srgb(0.42, 0.28, 0.14), Vec2::new(9.0, 46.0)),
            Transform::from_xyz(0.0, CAR_Y, -3.0),
            Visibility::Hidden,
            PalmPart,
        ));
        commands.spawn((
            Sprite::from_color(Color::srgb(0.10, 0.52, 0.20), Vec2::new(52.0, 14.0)),
            Transform::from_xyz(0.0, CAR_Y, -2.9),
            Visibility::Hidden,
            PalmPart,
        ));
        commands.spawn((
            Sprite::from_color(Color::srgb(0.13, 0.60, 0.24), Vec2::new(30.0, 10.0)),
            Transform::from_xyz(0.0, CAR_Y, -2.9),
            Visibility::Hidden,
            PalmPart,
        ));
    }

    // ---- hazards (pooled composite sprites) ----
    for _ in 0..HAZARD_POOL {
        for _ in 0..4 {
            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::new(56.0, 24.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, CAR_Y, -2.5),
                Visibility::Hidden,
                HazardPart,
            ));
        }
    }

    // ---- the fixed player car "decal" (composite) ----
    commands.spawn((
        Sprite::from_color(Color::srgb(0.86, 0.10, 0.08), Vec2::new(92.0, 30.0)),
        Transform::from_xyz(0.0, CAR_Y - 4.0, 0.5),
        CarPart,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.90, 0.16, 0.10), Vec2::new(78.0, 12.0)),
        Transform::from_xyz(0.0, CAR_Y + 10.0, 0.5),
        CarPart,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.12, 0.16, 0.28), Vec2::new(40.0, 14.0)),
        Transform::from_xyz(0.0, CAR_Y + 2.0, 0.6),
        CarPart,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.05, 0.05, 0.06), Vec2::new(96.0, 8.0)),
        Transform::from_xyz(0.0, CAR_Y + 17.0, 0.4),
        CarPart,
    ));
    for wx in [-36.0f32, 36.0] {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.07, 0.07, 0.08), Vec2::new(18.0, 14.0)),
            Transform::from_xyz(wx, CAR_Y - 14.0, 0.45),
            CarPart,
        ));
    }

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
        TextColor(Color::srgb(0.98, 0.98, 0.92)),
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
        TextColor(Color::srgb(0.75, 0.92, 0.98)),
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
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.88, 0.75)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(14.0),
            ..default()
        },
        StatusText,
    ));
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor(Color::srgb(0.98, 0.92, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(14.0),
            left: Val::Px(16.0),
            ..default()
        },
        LapText,
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

    // M10 drum bands + recess shading (alpha driven by toy_look)
    for i in 0..5 {
        commands.spawn((
            Sprite {
                color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(960.0, 8.0)),
                ..default()
            },
            Transform::from_xyz(0.0, -300.0 + i as f32 * 120.0, 1.2),
            DrumBand,
        ));
    }
    for (y, h) in [(290.0, 46.0), (252.0, 26.0), (-290.0, 46.0), (-252.0, 26.0)] {
        commands.spawn((
            Sprite {
                color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(960.0, h)),
                ..default()
            },
            Transform::from_xyz(0.0, y, 1.3),
            RecessShade,
        ));
    }
}

fn row_y(i: usize) -> f32 {
    TOP_ROW_Y - i as f32 * ROW_H
}

fn row_ahead(i: usize) -> f32 {
    (CAR_Y - row_y(i)) * M_PER_PX
}

#[allow(clippy::type_complexity)]
fn sync_roads(
    sim: Res<Sim>,
    colors: Res<Palette>,
    mut rows: Query<
        (&mut Transform, &mut Sprite),
        (With<RoadRow>, Without<EdgeRumble>, Without<CenterDash>),
    >,
    mut rumble: Query<
        (&mut Transform, &mut Sprite),
        (With<EdgeRumble>, Without<CenterDash>, Without<RoadRow>),
    >,
    mut dashes: Query<
        (&mut Transform, &mut Sprite),
        (With<CenterDash>, Without<EdgeRumble>, Without<RoadRow>),
    >,
    mut grass: Query<
        (&mut Transform, &mut Sprite),
        (
            With<GrassRow>,
            Without<RoadRow>,
            Without<EdgeRumble>,
            Without<CenterDash>,
        ),
    >,
) {
    let xs: Vec<f32> = (0..ROW_COUNT)
        .map(|i| sim.0.road_offset_at(row_ahead(i)) * HALF_ROAD_PX)
        .collect();

    for (i, (mut tf, mut sp)) in rows.iter_mut().enumerate() {
        if let Some(x) = xs.get(i) {
            tf.translation.x = *x;
        }
        let s_abs = sim.0.distance + row_ahead(i);
        let on = ((s_abs / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on {
            colors.asphalt_a
        } else {
            colors.asphalt_b
        };
    }
    for (i, (mut tf, mut sp)) in grass.iter_mut().enumerate() {
        let y = row_y(i);
        tf.translation = Vec3::new(0.0, y, -4.4);
        let s_abs = sim.0.distance + row_ahead(i);
        let on = ((s_abs / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on { colors.grass_a } else { colors.grass_b };
    }
    for (idx, (mut tf, mut sp)) in rumble.iter_mut().enumerate() {
        let row = idx / 2;
        let side: f32 = if idx % 2 == 0 { -1.0 } else { 1.0 };
        let Some(x) = xs.get(row) else { continue };
        tf.translation.x = x + side * (HALF_ROAD_PX - 7.0);
        let s_abs = sim.0.distance + row_ahead(row);
        let red = ((s_abs / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if red {
            colors.rumble_red
        } else {
            colors.rumble_white
        };
    }
    for (i, (mut tf, mut sp)) in dashes.iter_mut().enumerate() {
        let row = i / 2;
        let lane: f32 = if i % 2 == 0 { -0.4 } else { 0.4 };
        let Some(x) = xs.get(row) else { continue };
        tf.translation.x = x + lane * HALF_ROAD_PX;
        let s_abs = sim.0.distance + row_ahead(row);
        let on = ((s_abs / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on { colors.dash } else { colors.asphalt_a };
    }
}

#[allow(clippy::type_complexity)]
fn sync_hazards(
    sim: Res<Sim>,
    mut pool: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<HazardPart>>,
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
            if y < CAR_Y - 60.0 || y > HORIZON_Y - 16.0 {
                return None;
            }
            let x = (sim.0.road_offset_at(ahead) + h.x) * HALF_ROAD_PX;
            Some((x, y, h.kind))
        })
        .collect();

    let mut parts = pool.iter_mut();
    let mut place = |body: Color, accent: Color, dark: Color, w: f32, x: f32, y: f32| {
        // 4 parts per hazard: body, window, bumper-lights, shadow strip
        if let Some((mut tf, mut sp, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y, -2.5);
            sp.color = body;
            sp.custom_size = Some(Vec2::new(w, 24.0));
            *vis = Visibility::Visible;
        }
        if let Some((mut tf, mut sp, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y + 3.0, -2.4);
            sp.color = dark;
            sp.custom_size = Some(Vec2::new(w * 0.5, 10.0));
            *vis = Visibility::Visible;
        }
        if let Some((mut tf, mut sp, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y - 10.0, -2.4);
            sp.color = accent;
            sp.custom_size = Some(Vec2::new(w * 0.9, 5.0));
            *vis = Visibility::Visible;
        }
        if let Some((mut tf, mut sp, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y - 13.0, -2.6);
            sp.color = Color::srgb(0.06, 0.06, 0.07);
            sp.custom_size = Some(Vec2::new(w * 0.95, 6.0));
            *vis = Visibility::Visible;
        }
    };

    for &(x, y, kind) in visible.iter() {
        match kind {
            game_core::HazardKind::Oncoming => place(
                Color::srgb(0.93, 0.91, 0.85),
                Color::srgb(1.0, 0.9, 0.4),
                Color::srgb(0.15, 0.22, 0.35),
                56.0,
                x,
                y,
            ),
            game_core::HazardKind::Roadside => place(
                Color::srgb(0.95, 0.45, 0.10),
                Color::srgb(0.95, 0.95, 0.9),
                Color::srgb(0.95, 0.95, 0.9),
                30.0,
                x,
                y,
            ),
        }
    }
    for (_, _, mut vis) in parts.by_ref() {
        *vis = Visibility::Hidden;
    }
}

/// Palms ride the loop on both grass shoulders.
#[allow(clippy::type_complexity)]
fn sync_palms(
    sim: Res<Sim>,
    mut pool: Query<(&mut Transform, &mut Visibility), (With<PalmPart>, Without<HazardPart>)>,
) {
    let lp = sim.0.loop_pos();
    let l = sim.0.road.length;
    let mut spots: Vec<(f32, f32)> = Vec::new(); // (x, y)
    for k in 0..5u32 {
        for side in [-1.0f32, 1.0] {
            let s = (k as f32 + 0.5) / 5.0 * l;
            let ahead = (s - lp).rem_euclid(l);
            let y = CAR_Y + ahead / M_PER_PX;
            if y < CAR_Y - 60.0 || y > HORIZON_Y - 10.0 {
                continue;
            }
            let x = (sim.0.road_offset_at(ahead) + side * 1.5) * HALF_ROAD_PX;
            spots.push((x, y));
        }
    }
    let mut parts = pool.iter_mut();
    for &(x, y) in spots.iter() {
        // trunk / wide fronds / cross fronds
        if let Some((mut tf, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y, -3.0);
            *vis = Visibility::Visible;
        }
        if let Some((mut tf, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y + 26.0, -2.9);
            *vis = Visibility::Visible;
        }
        if let Some((mut tf, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y + 20.0, -2.9);
            *vis = Visibility::Visible;
        }
    }
    for (_, mut vis) in parts.by_ref() {
        *vis = Visibility::Hidden;
    }
}

fn sync_flash(sim: Res<Sim>, mut q: Query<&mut Sprite, With<FlashOverlay>>) {
    if let Ok(mut sp) = q.single_mut() {
        sp.color = Color::srgba(1.0, 1.0, 1.0, sim.0.flash.clamp(0.0, 1.0) * 0.85);
    }
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn sync_hud(
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    view: Res<crate::toy_view::ViewState>,
    mut fuel: Query<
        (&mut Transform, &mut Sprite),
        (
            With<FuelBarFill>,
            Without<RoadRow>,
            Without<FlashOverlay>,
            Without<HazardPart>,
        ),
    >,
    mut score: Query<&mut Text, (With<ScoreText>, Without<GearText>, Without<StatusText>)>,
    mut gear: Query<&mut Text, (With<GearText>, Without<StatusText>)>,
    mut status: Query<&mut Text, (With<StatusText>, Without<LapText>)>,
    mut lap: Query<
        &mut Text,
        (
            With<LapText>,
            Without<StatusText>,
            Without<ScoreText>,
            Without<GearText>,
        ),
    >,
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
    if let Ok(mut text) = lap.single_mut() {
        let best = if sim.0.lap_best > 0.0 {
            format_lap(sim.0.lap_best)
        } else {
            "--:--.--".to_string()
        };
        **text = format!(
            "LAP {} | {} | BEST {}",
            sim.0.laps + 1,
            format_lap(sim.0.lap_time),
            best
        );
    }
    if let Ok(mut text) = status.single_mut() {
        **text = if sim.0.paused {
            "PAUSED | Start/P resume | Select/R restart".to_string()
        } else if view.mode == crate::toy_view::ViewMode::Toy {
            match &sim.0.phase {
                Phase::Ready => {
                    "ARROWS drive | G gear | R restart | T flat view | Y toy look".to_string()
                }
                Phase::Running => "T flat view | Y toy look".to_string(),
                Phase::Crashed { .. } => "CRASHED!".to_string(),
                Phase::GameOver { reason } => match reason {
                    EndReason::Crash => "GAME OVER - CRASH | R to restart".to_string(),
                    EndReason::Fuel => "GAME OVER - OUT OF FUEL | R to restart".to_string(),
                },
            }
        } else {
            match &sim.0.phase {
                Phase::Ready => {
                    "ARROWS steer | G gear | R restart | T toy view | Y toy look".to_string()
                }
                Phase::Running => String::new(),
                Phase::Crashed { .. } => "CRASHED!".to_string(),
                Phase::GameOver { reason } => match reason {
                    EndReason::Crash => "GAME OVER - CRASH | R to restart".to_string(),
                    EndReason::Fuel => "GAME OVER - OUT OF FUEL | R to restart".to_string(),
                },
            }
        };
    }
}

/// M10: scroll the drum bands with the drum and set overlay alpha by dial.
#[allow(clippy::type_complexity)]
fn sync_drum_overlays(
    sim: Res<Sim>,
    view: Res<crate::toy_view::ViewState>,
    time: Res<Time>,
    mut bands: Query<
        (&mut Transform, &mut Sprite),
        (With<DrumBand>, Without<RecessShade>, Without<PalmPart>),
    >,
    mut shades: Query<&mut Sprite, (With<RecessShade>, Without<DrumBand>, Without<PalmPart>)>,
) {
    let look = view.toy_look;
    let px_per_s = sim.0.speed / M_PER_PX;
    for (mut tf, mut sp) in bands.iter_mut() {
        tf.translation.y -= px_per_s * time.delta_secs();
        if tf.translation.y < -310.0 {
            tf.translation.y += 600.0;
        }
        sp.color = Color::srgba(0.0, 0.0, 0.0, look * 0.10);
    }
    for (i, mut sp) in shades.iter_mut().enumerate() {
        let outer = i % 2 == 0;
        let a = if outer { look * 0.30 } else { look * 0.14 };
        sp.color = Color::srgba(0.0, 0.0, 0.0, a);
    }
}

pub fn add_view_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            sync_roads,
            sync_palms,
            sync_hazards,
            sync_flash,
            sync_drum_overlays,
            sync_hud,
        )
            .chain(),
    );
}
