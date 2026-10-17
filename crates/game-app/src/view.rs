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

// pseudo-3D projection (classic OutRun style): geometric row spacing,
// cumulative curve bend, sprites scaled by distance
pub const ROWS: usize = 30;
pub const D_MIN: f32 = 2.2; // meters to the nearest row
pub const S_FAR: f32 = 0.045; // scale at the horizon row
pub const CURVE_BEND: f32 = 0.055; // lateral half-widths gained per curve-meter

fn row_scale(i: usize) -> f32 {
    S_FAR.powf(i as f32 / ROWS as f32)
}
fn scale_to_y(s: f32) -> f32 {
    CAR_Y + (HORIZON_Y - CAR_Y) * (1.0 - s) / (1.0 - S_FAR)
}
fn scale_to_d(s: f32) -> f32 {
    D_MIN / s
}
const VIEW_TOP: f32 = 322.0;
const PALM_POOL: usize = 10;
const HAZARD_POOL: usize = 16;
const FUEL_X_LEFT: f32 = 240.0;
const FUEL_Y: f32 = 270.0;
const NITRO_X_LEFT: f32 = 302.0;
const NITRO_Y: f32 = 226.0;

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

/// Fills the screen below the nearest projected row (foreground apron).
#[derive(Component)]
struct Apron;

#[derive(Component)]
struct HazardPart;

#[derive(Component)]
struct NitroPickupPart;

#[derive(Component)]
struct PalmPart;

#[derive(Component)]
struct CarPart;

#[derive(Component)]
struct FlashOverlay;

#[derive(Component)]
struct FuelBarFill;

#[derive(Component)]
struct NitroBarSegment(u8);

#[derive(Component)]
pub struct ScoreText;

/// backing panel behind an in-screen HUD block (contrast)
#[derive(Component)]
pub struct HudPanel;

/// celestial sprites: sun disc, sun stripe, moon, stars
#[derive(Component)]
pub struct SkyObject;

/// clouds (day-dimmed, night-invisible)
#[derive(Component)]
pub struct Cloud;

/// horizon haze strip
#[derive(Component)]
pub struct HazeStrip;

/// snow particles
#[derive(Component)]
pub struct SnowFlake;

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

/// Per-frame road projection shared by road/sprite sync systems.
#[derive(Resource, Default)]
pub struct Proj {
    /// lateral center (half-width units incl. bend) at each row's distance
    pub rows_x: [f32; ROWS],
    /// cumulative curve at each row distance (half-width units)
    pub cum: [f32; ROWS],
}

impl Proj {
    /// Project a world distance to (screen_y, scale, lateral_units).
    pub fn project(&self, d: f32, road_x: f32) -> Option<(f32, f32, f32)> {
        if d < D_MIN * 0.55 {
            return None;
        }
        let s = (D_MIN / d).min(1.0);
        if s < S_FAR * 1.05 {
            return None;
        }
        // interpolate cumulative bend at d via the row table
        let cum = {
            // rows are ordered near(0)..far(ROWS-1); d decreases with index
            let mut val = self.cum[ROWS - 1];
            for i in 0..ROWS {
                let di = scale_to_d(row_scale(i));
                if d <= di {
                    let d0 = if i == 0 {
                        D_MIN
                    } else {
                        scale_to_d(row_scale(i - 1))
                    };
                    let t = if di > d0 { (d - d0) / (di - d0) } else { 1.0 };
                    let c0 = if i == 0 { 0.0 } else { self.cum[i - 1] };
                    val = c0 + (self.cum[i] - c0) * t;
                    break;
                }
                val = self.cum[i];
            }
            val
        };
        let y = scale_to_y(s);
        Some((y, s, road_x + CURVE_BEND * cum))
    }
}

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
    commands.init_resource::<Proj>();
    commands.init_resource::<Env>();
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
        SkyObject,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(1.0, 0.62, 0.25), Vec2::new(88.0, 14.0)),
        Transform::from_xyz(-250.0, HORIZON_Y + 62.0, -4.8),
        SkyObject,
    ));
    for (cx, cy, w) in [
        (-60.0, 205.0, 90.0),
        (170.0, 240.0, 120.0),
        (330.0, 190.0, 70.0),
    ] {
        commands.spawn((
            Sprite::from_color(Color::srgba(0.95, 0.96, 1.0, 0.85), Vec2::new(w, 12.0)),
            Transform::from_xyz(cx, cy, -4.8),
            Cloud,
        ));
    }
    // moon + stars (alpha driven by night), haze strip, snow pool
    commands.spawn((
        Sprite::from_color(Color::srgb(0.9, 0.93, 0.98), Vec2::new(46.0, 46.0)),
        Transform::from_xyz(240.0, HORIZON_Y + 82.0, -4.85),
        SkyObject,
    ));
    for k in 0..14 {
        let sx = -450.0 + (k * 71 % 917) as f32;
        let sy = 200.0 + (k * 43 % 95) as f32;
        commands.spawn((
            Sprite::from_color(Color::srgb(1.0, 1.0, 0.95), Vec2::new(3.0, 3.0)),
            Transform::from_xyz(sx, sy, -4.85),
            SkyObject,
        ));
    }
    commands.spawn((
        Sprite {
            color: Color::srgba(0.7, 0.78, 0.86, 0.0),
            custom_size: Some(Vec2::new(960.0, 30.0)),
            ..default()
        },
        Transform::from_xyz(0.0, HORIZON_Y + 15.0, -4.55),
        HazeStrip,
    ));
    for k in 0..36 {
        let fx = -480.0 + (k * 53 % 960) as f32;
        let fy = -300.0 + (k * 37 % 600) as f32;
        commands.spawn((
            Sprite {
                color: Color::srgba(0.95, 0.97, 1.0, 0.0),
                custom_size: Some(Vec2::new(3.0, 3.0)),
                ..default()
            },
            Transform::from_xyz(fx, fy, 1.4),
            SnowFlake,
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

    // ---- perspective world rows (classic super-scaler feel) ----
    // geometry (y, height, width) is STATIC from the row scale; only the
    // lateral x positions and stripe colors animate per frame.
    for i in 0..ROWS {
        let s = row_scale(i);
        let y = scale_to_y(s);
        let y_next = if i + 1 < ROWS {
            scale_to_y(row_scale(i + 1))
        } else {
            HORIZON_Y + 2.0
        };
        let h = (y_next - y).max(1.5);
        commands.spawn((
            Sprite {
                custom_size: Some(Vec2::new(HALF_ROAD_PX * 2.0 * s, h + 1.0)),
                ..default()
            },
            Transform::from_xyz(0.0, y + h * 0.5, -4.0),
            RoadRow,
        ));
        commands.spawn((
            Sprite {
                custom_size: Some(Vec2::new(960.0, h + 1.0)),
                ..default()
            },
            Transform::from_xyz(0.0, y + h * 0.5, -4.4),
            GrassRow,
        ));
        for side in [-1.0f32, 1.0] {
            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::new((16.0 * s).max(1.5), h + 1.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, y + h * 0.5, -3.9),
                EdgeRumble,
            ));
            let _ = side;
        }
        for _lane in [-0.4f32, 0.4] {
            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::new((7.0 * s).max(1.0), h * 0.6)),
                    // dashes sit inside their row span
                    ..default()
                },
                Transform::from_xyz(0.0, y + h * 0.5, -3.8),
                CenterDash,
            ));
        }
    }

    // ---- foreground apron (below the nearest row, static geometry) ----
    let apron_h = CAR_Y - (-322.0);
    for (w, z, marker_apply) in [(960.0f32, -4.4, false), (HALF_ROAD_PX * 2.0, -4.0, true)] {
        let _ = marker_apply;
        commands.spawn((
            Sprite {
                custom_size: Some(Vec2::new(w, apron_h + 2.0)),
                ..default()
            },
            Transform::from_xyz(0.0, -322.0 + apron_h * 0.5, z),
            Apron,
        ));
    }
    for side in [-1.0f32, 1.0] {
        commands.spawn((
            Sprite {
                custom_size: Some(Vec2::new(16.0, apron_h + 2.0)),
                ..default()
            },
            Transform::from_xyz(0.0, -322.0 + apron_h * 0.5, -3.9),
            Apron,
        ));
        let _ = side;
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

    // ---- next nitro pickup (one deterministic road item at a time) ----
    for _ in 0..4 {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.15, 0.92, 1.0), Vec2::new(36.0, 48.0)),
            Transform::from_xyz(0.0, CAR_Y, -2.3),
            Visibility::Hidden,
            NitroPickupPart,
        ));
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

    // Three-cell nitro meter, rendered into the toy screen. During a boost,
    // the active cell shrinks over five seconds; a stored full meter blinks.
    commands.spawn((
        Sprite::from_color(Color::srgba(0.02, 0.02, 0.04, 0.82), Vec2::new(196.0, 34.0)),
        Transform::from_xyz(332.0, NITRO_Y, 1.0),
        HudPanel,
    ));
    commands.spawn((
        Text2d::new("NITRO Y"),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgb(0.55, 0.95, 1.0)),
        Transform::from_xyz(253.0, NITRO_Y, 1.1),
    ));
    for i in 0..3u8 {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.10, 0.16, 0.20), Vec2::new(34.0, 14.0)),
            Transform::from_xyz(NITRO_X_LEFT + i as f32 * 40.0 + 17.0, NITRO_Y, 1.1),
            NitroBarSegment(i),
        ));
    }

    // in-screen HUD (Text2d => rendered INTO the game texture, so it shows
    // on the toy screen itself) with dark backing panels for contrast
    for (cx, w) in [(-330.0, 210.0), (0.0, 330.0), (332.0, 196.0)] {
        commands.spawn((
            Sprite {
                color: Color::srgba(0.02, 0.02, 0.04, 0.75),
                custom_size: Some(Vec2::new(w, 40.0)),
                ..default()
            },
            Transform::from_xyz(cx, 274.0, 1.0),
            HudPanel,
        ));
    }
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor(Color::srgb(0.99, 0.99, 0.94)),
        Transform::from_xyz(-330.0, 274.0, 1.1),
        ScoreText,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(26.0),
            ..default()
        },
        TextColor(Color::srgb(0.99, 0.95, 0.75)),
        Transform::from_xyz(0.0, 274.0, 1.1),
        LapText,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.93, 0.99)),
        Transform::from_xyz(330.0, 254.0, 1.1),
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

fn row_dist(i: usize) -> f32 {
    scale_to_d(row_scale(i))
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_roads(
    sim: Res<Sim>,
    colors: Res<Palette>,
    mut proj: ResMut<Proj>,
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
    mut apron: Query<
        (&mut Transform, &mut Sprite),
        (
            With<Apron>,
            Without<RoadRow>,
            Without<EdgeRumble>,
            Without<CenterDash>,
            Without<GrassRow>,
        ),
    >,
) {
    // walk rows near->far, accumulating the curve bend (classic pseudo-3D:
    // the bend is CUMULATIVE, so the road sweeps smoothly instead of
    // zig-zagging row to row)
    let mut cum = 0.0f32;
    for i in 0..ROWS {
        let s = row_scale(i);
        let d = row_dist(i);
        let pos = sim.0.distance + d;
        cum += sim.0.road.curve_at(pos) * (d - if i == 0 { D_MIN } else { row_dist(i - 1) });
        proj.rows_x[i] = HALF_ROAD_PX * s * (sim.0.road_x + CURVE_BEND * cum);
        proj.cum[i] = cum;
    }

    for (i, (mut tf, mut sp)) in rows.iter_mut().enumerate() {
        tf.translation.x = proj.rows_x[i];
        let pos = sim.0.distance + row_dist(i);
        let on = ((pos / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on {
            colors.asphalt_a
        } else {
            colors.asphalt_b
        };
    }
    for (i, (mut tf, mut sp)) in grass.iter_mut().enumerate() {
        tf.translation.x = 0.0;
        let pos = sim.0.distance + row_dist(i);
        let on = ((pos / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on { colors.grass_a } else { colors.grass_b };
    }
    // foreground apron follows row 0 (grass full width + road band + edges)
    if let Some(&x0) = proj.rows_x.first() {
        let pos0 = sim.0.distance + D_MIN;
        let on0 = ((pos0 / 3.5).floor() as i64).rem_euclid(2) == 0;
        for (idx, (mut tf, mut sp)) in apron.iter_mut().enumerate() {
            let w = sp.custom_size.map(|c| c.x).unwrap_or(0.0);
            if w > 900.0 {
                // grass
                tf.translation.x = 0.0;
                sp.color = if on0 { colors.grass_a } else { colors.grass_b };
            } else if w > 100.0 {
                // road band
                tf.translation.x = x0;
                sp.color = if on0 {
                    colors.asphalt_a
                } else {
                    colors.asphalt_b
                };
            } else {
                // rumble strips
                let side: f32 = if idx % 2 == 0 { -1.0 } else { 1.0 };
                tf.translation.x = x0 + side * (HALF_ROAD_PX - 8.0);
                sp.color = if on0 {
                    colors.rumble_red
                } else {
                    colors.rumble_white
                };
            }
        }
    }
    for (idx, (mut tf, mut sp)) in rumble.iter_mut().enumerate() {
        let row = idx / 2;
        let side: f32 = if idx % 2 == 0 { -1.0 } else { 1.0 };
        let s = row_scale(row);
        tf.translation.x = proj.rows_x[row] + side * (HALF_ROAD_PX - 8.0) * s;
        let pos = sim.0.distance + row_dist(row);
        let red = ((pos / 7.0).floor() as i64).rem_euclid(2) == 0;
        sp.color = if red {
            colors.rumble_red
        } else {
            colors.rumble_white
        };
    }
    for (i, (mut tf, mut sp)) in dashes.iter_mut().enumerate() {
        let row = i / 2;
        let lane: f32 = if i % 2 == 0 { -0.4 } else { 0.4 };
        let s = row_scale(row);
        tf.translation.x = proj.rows_x[row] + lane * HALF_ROAD_PX * s;
        let pos = sim.0.distance + row_dist(row);
        let on = ((pos / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on { colors.dash } else { colors.asphalt_a };
    }
}

#[allow(clippy::type_complexity)]
fn sync_hazards(
    sim: Res<Sim>,
    proj: Res<Proj>,
    mut pool: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<HazardPart>>,
) {
    let lp = sim.0.loop_pos();
    let mut parts = pool.iter_mut();
    let mut place = |body: Color, accent: Color, dark: Color, w: f32, x: f32, y: f32, s: f32| {
        let w = (w * s).max(2.0);
        if let Some((mut tf, mut sp, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y, -2.5);
            sp.color = body;
            sp.custom_size = Some(Vec2::new(w, w * 0.42));
            *vis = Visibility::Visible;
        }
        if let Some((mut tf, mut sp, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y + w * 0.12, -2.4);
            sp.color = dark;
            sp.custom_size = Some(Vec2::new(w * 0.5, w * 0.18));
            *vis = Visibility::Visible;
        }
        if let Some((mut tf, mut sp, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y - w * 0.18, -2.4);
            sp.color = accent;
            sp.custom_size = Some(Vec2::new(w * 0.9, w * 0.09));
            *vis = Visibility::Visible;
        }
        if let Some((mut tf, mut sp, mut vis)) = parts.next() {
            tf.translation = Vec3::new(x, y - w * 0.24, -2.6);
            sp.color = Color::srgb(0.06, 0.06, 0.07);
            sp.custom_size = Some(Vec2::new(w * 0.95, w * 0.1));
            *vis = Visibility::Visible;
        }
    };

    for h in &sim.0.road.hazards {
        let ahead = (h.s - lp).rem_euclid(sim.0.road.length);
        let Some((y, s, lat)) = proj.project(ahead, sim.0.road_x) else {
            continue;
        };
        let x = (lat + h.x) * HALF_ROAD_PX * s;
        match h.kind {
            game_core::HazardKind::Oncoming => place(
                Color::srgb(0.93, 0.91, 0.85),
                Color::srgb(1.0, 0.9, 0.4),
                Color::srgb(0.15, 0.22, 0.35),
                56.0,
                x,
                y,
                s,
            ),
            game_core::HazardKind::Roadside => place(
                Color::srgb(0.95, 0.45, 0.10),
                Color::srgb(0.95, 0.95, 0.9),
                Color::srgb(0.95, 0.95, 0.9),
                30.0,
                x,
                y,
                s,
            ),
        }
    }
    for (_, _, mut vis) in parts.by_ref() {
        *vis = Visibility::Hidden;
    }
}

#[allow(clippy::type_complexity)]
fn sync_nitro_pickup(
    sim: Res<Sim>,
    proj: Res<Proj>,
    mut parts: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<NitroPickupPart>>,
) {
    let mut parts = parts.iter_mut();
    let Some((y, s, lat)) = proj.project(sim.0.nitro_pickup_ahead(), sim.0.road_x) else {
        for (_, _, mut vis) in parts {
            *vis = Visibility::Hidden;
        }
        return;
    };
    let x = (lat + sim.0.next_nitro_lane) * HALF_ROAD_PX * s;
    let w = (38.0 * s).max(3.0);
    let h = (50.0 * s).max(4.0);
    let specs = [
        (Color::srgb(0.08, 0.72, 0.95), Vec2::new(w, h), 0.0, 0.0),
        (
            Color::srgb(1.0, 0.55, 0.08),
            Vec2::new(w * 0.52, h * 0.16),
            0.0,
            h * 0.55,
        ),
        (
            Color::srgb(0.88, 1.0, 1.0),
            Vec2::new(w * 0.70, h * 0.16),
            0.0,
            -h * 0.05,
        ),
        (
            Color::srgb(0.02, 0.18, 0.28),
            Vec2::new(w * 0.22, h * 0.52),
            0.0,
            0.0,
        ),
    ];
    for ((mut tf, mut sprite, mut vis), (color, size, dx, dy)) in parts.by_ref().zip(specs) {
        tf.translation = Vec3::new(x + dx, y + dy, -2.3);
        sprite.color = color;
        sprite.custom_size = Some(size);
        *vis = Visibility::Visible;
    }
    for (_, _, mut vis) in parts {
        *vis = Visibility::Hidden;
    }
}

/// Palms ride the loop on both grass shoulders.
#[allow(clippy::type_complexity)]
fn sync_palms(
    sim: Res<Sim>,
    proj: Res<Proj>,
    mut pool: Query<
        (&mut Transform, &mut Visibility, &mut Sprite),
        (With<PalmPart>, Without<HazardPart>),
    >,
) {
    let lp = sim.0.loop_pos();
    let l = sim.0.road.length;
    // sparse roadside scenery: 3 per side per loop (not traffic-dense)
    let mut spots: Vec<(f32, f32, f32)> = Vec::new();
    for k in 0..3u32 {
        for side in [-1.45f32, 1.45] {
            let s_pos = (k as f32 + if side < 0.0 { 0.5 } else { 0.0 }) / 3.0 * l;
            let ahead = (s_pos - lp).rem_euclid(l);
            if let Some((y, s, lat)) = proj.project(ahead, sim.0.road_x) {
                let x = (lat + side) * HALF_ROAD_PX * s;
                spots.push((x, y, s));
            }
        }
    }
    let mut parts = pool.iter_mut();
    for &(x, y, s) in spots.iter() {
        let h = (46.0 * s).max(2.0);
        // trunk
        if let Some((mut tf, mut vis, mut sp)) = parts.next() {
            tf.translation = Vec3::new(x, y + h * 0.2, -3.0);
            sp.custom_size = Some(Vec2::new((9.0 * s).max(1.0), h));
            *vis = Visibility::Visible;
        }
        // wide fronds
        if let Some((mut tf, mut vis, mut sp)) = parts.next() {
            tf.translation = Vec3::new(x, y + h * 0.95, -2.9);
            sp.custom_size = Some(Vec2::new((52.0 * s).max(2.0), (14.0 * s).max(1.0)));
            *vis = Visibility::Visible;
        }
        // cross fronds
        if let Some((mut tf, mut vis, mut sp)) = parts.next() {
            tf.translation = Vec3::new(x, y + h * 0.8, -2.9);
            sp.custom_size = Some(Vec2::new((30.0 * s).max(2.0), (10.0 * s).max(1.0)));
            *vis = Visibility::Visible;
        }
    }
    for (_, mut vis, _) in parts.by_ref() {
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
    mut score: Query<&mut Text2d, (With<ScoreText>, Without<GearText>, Without<StatusText>)>,
    mut gear: Query<&mut Text2d, (With<GearText>, Without<StatusText>)>,
    mut status: Query<&mut Text, (With<StatusText>, Without<LapText>)>,
    mut lap: Query<
        &mut Text2d,
        (
            With<LapText>,
            Without<StatusText>,
            Without<ScoreText>,
            Without<GearText>,
        ),
    >,
    mut nitro: Query<
        (&NitroBarSegment, &mut Transform, &mut Sprite),
        (With<NitroBarSegment>, Without<FuelBarFill>),
    >,
    time: Res<Time>,
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
        text.0 = format!("{:05.0} m", sim.0.distance);
    }
    if let Ok(mut text) = gear.single_mut() {
        text.0 = match sim.0.gear() {
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
        text.0 = format!(
            "LAP {} | {} | BEST {}",
            sim.0.laps + 1,
            format_lap(sim.0.lap_time),
            best
        );
    }
    let active = sim.0.nitro_active();
    let active_frac = (sim.0.nitro_t / tuning.0.nitro_duration_secs).clamp(0.0, 1.0);
    let full_blink = sim.0.nitro_charges == 3 && !active;
    let blink_on = !full_blink || (time.elapsed_secs() * 4.0).floor() as i32 % 2 == 0;
    for (segment, mut tf, mut sp) in &mut nitro {
        let i = segment.0;
        let frac = if i < sim.0.nitro_charges {
            1.0
        } else if active && i == sim.0.nitro_charges {
            active_frac
        } else {
            0.0
        };
        let w = if frac > 0.0 { 34.0 * frac } else { 34.0 };
        tf.translation.x = NITRO_X_LEFT + i as f32 * 40.0 + w * 0.5;
        sp.custom_size = Some(Vec2::new(w, 14.0));
        sp.color = if frac > 0.0 {
            Color::srgba(0.10, 0.86, 1.0, if blink_on { 1.0 } else { 0.28 })
        } else {
            Color::srgb(0.10, 0.16, 0.20)
        };
    }
    if let Ok(mut text) = status.single_mut() {
        **text = if sim.0.paused {
            "PAUSED | Start/P resume | Select/R restart".to_string()
        } else if view.mode == crate::toy_view::ViewMode::Toy {
            match &sim.0.phase {
                Phase::Ready => {
                    "ARROWS drive | G gear | Y nitro | R restart | T flat | L toy look".to_string()
                }
                Phase::Running => "Y nitro | T flat view | L toy look".to_string(),
                Phase::Crashed { .. } => "CRASHED!".to_string(),
                Phase::GameOver { reason } => match reason {
                    EndReason::Crash => "GAME OVER - CRASH | R to restart".to_string(),
                    EndReason::Fuel => "GAME OVER - OUT OF FUEL | R to restart".to_string(),
                },
            }
        } else {
            match &sim.0.phase {
                Phase::Ready => {
                    "ARROWS steer | G gear | Y nitro | R restart | T toy | L toy look".to_string()
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

/// Environment state shared by the render systems (day/night + seasons).
#[derive(Resource, Default)]
pub struct Env {
    pub daylight: f32,
    pub weights: [f32; 4], // spring, summer, autumn, winter
}

/// QA speed-up for the day/night + season cycles (1.0 = real pacing:
/// 100 s day/night, 5 min seasons, 3 cycles per season).
pub const ENV_TIME_SCALE: f32 = 1.0;

/// scalar multiply in linear space (dim for night)
fn scale_color(c: Color, k: f32) -> Color {
    let l = c.to_linear();
    Color::LinearRgba(LinearRgba::rgb(l.red * k, l.green * k, l.blue * k))
}

fn blend4(cols: [Color; 4], w: [f32; 4]) -> Color {
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    for i in 0..4 {
        let LinearRgba {
            red, green, blue, ..
        } = cols[i].to_linear();
        r += red * w[i];
        g += green * w[i];
        b += blue * w[i];
    }
    Color::LinearRgba(LinearRgba::rgb(r, g, b))
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sync_environment(
    time: Res<Time>,
    mut env: ResMut<Env>,
    mut colors: ResMut<Palette>,
    mut sky: Query<&mut Sprite, (With<SkyBand>, Without<SkyObject>)>,
    mut sky_objs: Query<&mut Sprite, (With<SkyObject>, Without<SkyBand>, Without<Cloud>)>,
    mut clouds: Query<&mut Sprite, (With<Cloud>, Without<SkyBand>, Without<SkyObject>)>,
    mut haze: Query<
        &mut Sprite,
        (
            With<HazeStrip>,
            Without<SkyBand>,
            Without<SkyObject>,
            Without<SnowFlake>,
            Without<Cloud>,
        ),
    >,
    mut snow: Query<
        (&mut Transform, &mut Sprite),
        (
            With<SnowFlake>,
            Without<HazeStrip>,
            Without<SkyBand>,
            Without<SkyObject>,
            Without<Cloud>,
        ),
    >,
) {
    let t = time.elapsed_secs() * ENV_TIME_SCALE;
    let day = game_core::environment::daylight(t);
    let w = game_core::environment::season_weights(t);
    env.daylight = day;
    env.weights = w;

    let winter = w[3];
    let autumn = w[2];
    let bright = 0.35 + 0.65 * day;

    // ground colors across seasons, dimmed by night
    let g1 = blend4(
        [
            Color::srgb(0.16, 0.55, 0.28),
            Color::srgb(0.12, 0.58, 0.24),
            Color::srgb(0.55, 0.38, 0.15),
            Color::srgb(0.82, 0.85, 0.89),
        ],
        w,
    );
    let g2 = blend4(
        [
            Color::srgb(0.13, 0.47, 0.24),
            Color::srgb(0.09, 0.50, 0.20),
            Color::srgb(0.45, 0.30, 0.12),
            Color::srgb(0.72, 0.76, 0.82),
        ],
        w,
    );
    colors.grass_a = scale_color(g1, bright);
    colors.grass_b = scale_color(g2, bright);
    colors.asphalt_a = scale_color(Color::srgb(0.34, 0.35, 0.40), bright);
    colors.asphalt_b = scale_color(Color::srgb(0.30, 0.31, 0.36), bright);
    // rumble goes snow-pale in deep winter
    let rum_a = Color::srgb(0.85, 0.15, 0.12).mix(&Color::srgb(0.88, 0.9, 0.95), winter * 0.6);
    let rum_b = Color::srgb(0.95, 0.95, 0.92).mix(&Color::srgb(0.7, 0.75, 0.85), winter * 0.5);
    colors.rumble_red = scale_color(rum_a, bright);
    colors.rumble_white = scale_color(rum_b, bright);
    colors.dash = scale_color(Color::srgb(0.95, 0.95, 0.90), bright);

    // sky: day bands blended toward a night palette; winter grays, autumn warms
    let day_bands = sky_colors();
    let night = [
        Color::srgb(0.015, 0.02, 0.06),
        Color::srgb(0.02, 0.03, 0.09),
        Color::srgb(0.03, 0.05, 0.14),
        Color::srgb(0.05, 0.07, 0.18),
        Color::srgb(0.07, 0.10, 0.22),
        Color::srgb(0.09, 0.12, 0.26),
    ];
    for (i, mut sp) in sky.iter_mut().enumerate() {
        let mut c = day_bands[i].0.mix(&night[i], 1.0 - day);
        // autumn: warm shift; winter: desaturated gray-blue
        c = c.mix(&Color::srgb(0.9, 0.6, 0.35), autumn * 0.22);
        c = c.mix(&Color::srgb(0.55, 0.6, 0.68), winter * 0.45);
        sp.color = c;
    }

    // celestial: sun bright by day, moon + stars by night
    let (sun, moon, star) = (
        Color::srgb(1.0, 0.85, 0.35),
        Color::srgb(0.88, 0.91, 0.97),
        Color::srgb(1.0, 1.0, 0.95),
    );
    let mut objs = sky_objs.iter_mut();
    if let Some(mut sp) = objs.next() {
        // sun disc
        sp.color = scale_color(sun, day.max(0.0));
    }
    if let Some(mut sp) = objs.next() {
        // sun stripe
        sp.color = scale_color(Color::srgb(1.0, 0.62, 0.25), day.max(0.0))
            .mix(&Color::srgb(0.9, 0.6, 0.35), autumn * 0.3);
    }
    if let Some(mut sp) = objs.next() {
        // moon
        sp.color = scale_color(moon, 1.0 - day);
    }
    for mut sp in objs.by_ref() {
        // stars
        sp.color = scale_color(star, ((1.0 - day) * 0.9).max(0.0));
    }
    for mut sp in clouds.iter_mut() {
        let a = (0.85 * (day * 1.2).clamp(0.0, 1.0)).min(0.85);
        sp.color = Color::srgba(0.95, 0.96, 1.0, a);
    }

    // haze: denser in winter and at dawn/dusk
    if let Ok(mut sp) = haze.single_mut() {
        let haze_amt = (0.18 + winter * 0.35 + autumn * 0.18) * (0.65 + 0.35 * (1.0 - day));
        let c = Color::srgb(0.75, 0.8, 0.88).mix(&Color::srgb(1.0, 1.0, 1.0), winter * 0.5);
        sp.color = c.with_alpha(haze_amt.clamp(0.0, 0.75));
    }

    // snow: fall while winter holds weight
    let fall = 90.0 + winter * 60.0;
    let dt = time.delta_secs();
    for (k, (mut tf, mut sp)) in snow.iter_mut().enumerate() {
        let drift = ((k % 5) as f32 - 2.0) * 14.0;
        tf.translation.y -= fall * dt;
        tf.translation.x += drift * dt;
        if tf.translation.y < -310.0 {
            tf.translation.y = 310.0;
            tf.translation.x = -480.0 + (k * 53 % 960) as f32;
        }
        sp.color = Color::srgba(0.95, 0.97, 1.0, (winter * 0.9).clamp(0.0, 1.0));
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
            sync_environment,
            sync_roads,
            sync_palms,
            sync_hazards,
            sync_nitro_pickup,
            sync_flash,
            sync_drum_overlays,
            sync_hud,
        )
            .chain(),
    );
}
