//! Fun to Drive — the 2D drum view, OutRun-inspired (PLAN: user tweak 2).
//!
//! Classic 16-bit racer look: gradient sky with sun and clouds, sea strip at
//! the horizon, sandy shoulder, scrolling grass stripes, red/white rumble
//! edges, lane dashes, palm trees, and composite car sprites — all drawn
//! with flat sprites so it renders identically into the toy's screen.

use bevy::prelude::*;
use game_core::{Gear, Phase, WeatherKind, WeatherTransition};

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
const NITRO_X_LEFT: f32 = 302.0;
const NITRO_Y: f32 = 216.0;
const RAIN_POOL: usize = 84;
const SPEED_LINE_POOL: usize = 28;

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
struct NitroPickupPart(u8);

#[derive(Component)]
struct PalmPart;

#[derive(Component)]
struct CarPart;

#[derive(Component)]
struct NitroExhaust {
    side: f32,
    layer: u8,
}

#[derive(Component)]
struct NitroSpeedLine(usize);

#[derive(Component)]
struct FlashOverlay;

#[derive(Component)]
struct NitroBarSegment(u8);

#[derive(Component)]
struct NitroSegmentHalo(u8);

#[derive(Component)]
struct NitroMeterHalo;

#[derive(Component)]
struct RainStreak(usize);

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
pub struct SessionText;

#[derive(Component)]
pub struct TrafficText;

#[derive(Component)]
struct EndOverlayRoot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndTextKind {
    Summary,
    Prompt,
    Initial(u8),
    Table,
    Footer,
}

#[derive(Component)]
struct EndOverlayText(EndTextKind);

/// Resolved visual weather weights are published for deterministic QA.
#[derive(Resource, Default)]
pub struct WeatherVisuals {
    pub clear: f32,
    pub cloudy: f32,
    pub rain: f32,
    pub rain_visible: f32,
}

/// Shared eased envelope for nitro-only screen effects.
#[derive(Resource, Default)]
pub struct NitroVisuals {
    pub intensity: f32,
}

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

fn format_time(secs: f32) -> String {
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
    commands.init_resource::<WeatherVisuals>();
    commands.init_resource::<NitroVisuals>();
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
        (-350.0, 260.0, 130.0),
        (-190.0, 210.0, 95.0),
        (-45.0, 248.0, 145.0),
        (105.0, 190.0, 85.0),
        (215.0, 235.0, 125.0),
        (350.0, 205.0, 105.0),
        (430.0, 272.0, 80.0),
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
    for k in 0..RAIN_POOL {
        let x = -480.0 + (k * 67 % 960) as f32;
        let y = -300.0 + (k * 97 % 600) as f32;
        commands.spawn((
            Sprite {
                color: Color::srgba(0.72, 0.88, 1.0, 0.0),
                custom_size: Some(Vec2::new(2.0, 14.0)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.7).with_rotation(Quat::from_rotation_z(-0.22)),
            RainStreak(k),
        ));
    }
    for k in 0..SPEED_LINE_POOL {
        commands.spawn((
            Sprite::from_color(Color::srgba(0.72, 0.94, 1.0, 0.0), Vec2::new(2.0, 48.0)),
            Transform::from_xyz(0.0, 0.0, 0.68),
            Visibility::Hidden,
            NitroSpeedLine(k),
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
    for i in 0..4u8 {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.15, 0.92, 1.0), Vec2::new(36.0, 48.0)),
            Transform::from_xyz(0.0, CAR_Y, -2.3),
            Visibility::Hidden,
            NitroPickupPart(i),
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

    // Twin layered jets sit behind the player car. They remain pooled and
    // hidden until nitro is active so both the flat and toy-screen views see
    // the same lightweight effect.
    for side in [-1.0f32, 1.0] {
        for layer in 0..3u8 {
            commands.spawn((
                Sprite::from_color(Color::srgba(1.0, 0.5, 0.1, 0.0), Vec2::ONE),
                Transform::from_xyz(side * 27.0, CAR_Y - 24.0, 0.42 + layer as f32 * 0.05),
                Visibility::Hidden,
                NitroExhaust { side, layer },
            ));
        }
    }

    // Three-cell nitro meter, rendered into the toy screen. During a boost,
    // the active cell shrinks over five seconds; collection and a full meter
    // get separate soft glow layers.
    commands.spawn((
        Sprite::from_color(Color::srgba(0.10, 0.86, 1.0, 0.0), Vec2::new(228.0, 42.0)),
        Transform::from_xyz(330.0, NITRO_Y, 0.98),
        NitroMeterHalo,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgba(0.02, 0.02, 0.04, 0.82), Vec2::new(220.0, 34.0)),
        Transform::from_xyz(330.0, NITRO_Y, 1.0),
        HudPanel,
    ));
    commands.spawn((
        Text2d::new("NITRO Y"),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgb(0.55, 0.95, 1.0)),
        Transform::from_xyz(258.0, NITRO_Y, 1.1),
    ));
    for i in 0..3u8 {
        commands.spawn((
            Sprite::from_color(Color::srgba(0.45, 0.95, 1.0, 0.0), Vec2::new(52.0, 28.0)),
            Transform::from_xyz(NITRO_X_LEFT + i as f32 * 40.0 + 17.0, NITRO_Y, 1.05),
            NitroSegmentHalo(i),
        ));
        commands.spawn((
            Sprite::from_color(Color::srgb(0.10, 0.16, 0.20), Vec2::new(34.0, 14.0)),
            Transform::from_xyz(NITRO_X_LEFT + i as f32 * 40.0 + 17.0, NITRO_Y, 1.1),
            NitroBarSegment(i),
        ));
    }

    commands.spawn((
        Sprite::from_color(Color::srgba(0.02, 0.02, 0.04, 0.78), Vec2::new(64.0, 34.0)),
        Transform::from_xyz(178.0, NITRO_Y, 1.0),
        HudPanel,
    ));

    // in-screen HUD (Text2d => rendered INTO the game texture, so it shows
    // on the toy screen itself) with dark backing panels for contrast
    for (cx, w) in [(-355.0, 190.0), (-75.0, 350.0), (275.0, 330.0)] {
        commands.spawn((
            Sprite {
                color: Color::srgba(0.02, 0.02, 0.04, 0.78),
                custom_size: Some(Vec2::new(w, 44.0)),
                ..default()
            },
            Transform::from_xyz(cx, 258.0, 1.0),
            HudPanel,
        ));
    }
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::srgb(0.99, 0.99, 0.94)),
        Transform::from_xyz(-355.0, 258.0, 1.1),
        ScoreText,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::srgb(0.99, 0.95, 0.75)),
        Transform::from_xyz(-75.0, 258.0, 1.1),
        SessionText,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.93, 0.99)),
        Transform::from_xyz(275.0, 258.0, 1.1),
        TrafficText,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.93, 0.99)),
        Transform::from_xyz(178.0, NITRO_Y, 1.1),
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

    // Session-end presentation is rendered into the same 960x600 texture as
    // gameplay, so it remains legible in both flat and toy-screen views.
    commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, 20.0),
            Visibility::Hidden,
            EndOverlayRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite::from_color(
                    Color::srgba(0.01, 0.02, 0.05, 0.78),
                    Vec2::new(960.0, 600.0),
                ),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
            parent.spawn((
                Sprite::from_color(
                    Color::srgba(0.025, 0.07, 0.14, 0.98),
                    Vec2::new(720.0, 500.0),
                ),
                Transform::from_xyz(0.0, 0.0, 0.1),
            ));
            parent.spawn((
                Sprite::from_color(Color::srgba(0.10, 0.86, 1.0, 0.95), Vec2::new(724.0, 4.0)),
                Transform::from_xyz(0.0, 248.0, 0.2),
            ));
            parent.spawn((
                Text2d::new("RUN COMPLETE"),
                TextFont {
                    font_size: FontSize::Px(36.0),
                    ..default()
                },
                TextColor(Color::srgb(0.50, 0.95, 1.0)),
                Transform::from_xyz(0.0, 195.0, 0.3),
            ));
            for (kind, size, color, y) in [
                (
                    EndTextKind::Summary,
                    23.0,
                    Color::srgb(1.0, 0.93, 0.60),
                    137.0,
                ),
                (
                    EndTextKind::Prompt,
                    17.0,
                    Color::srgb(0.75, 0.92, 1.0),
                    91.0,
                ),
                (
                    EndTextKind::Table,
                    17.0,
                    Color::srgb(0.84, 0.94, 1.0),
                    -92.0,
                ),
                (
                    EndTextKind::Footer,
                    15.0,
                    Color::srgb(0.55, 0.82, 0.92),
                    -218.0,
                ),
            ] {
                parent.spawn((
                    Text2d::new(""),
                    TextFont {
                        font_size: FontSize::Px(size),
                        ..default()
                    },
                    TextColor(color),
                    Transform::from_xyz(0.0, y, 0.3),
                    EndOverlayText(kind),
                ));
            }
            for index in 0..3u8 {
                let x = (index as f32 - 1.0) * 78.0;
                parent.spawn((
                    Sprite::from_color(Color::srgba(0.08, 0.62, 0.78, 0.48), Vec2::new(62.0, 62.0)),
                    Transform::from_xyz(x, 37.0, 0.2),
                ));
                parent.spawn((
                    Text2d::new("_"),
                    TextFont {
                        font_size: FontSize::Px(38.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.88, 1.0, 1.0)),
                    Transform::from_xyz(x, 37.0, 0.3),
                    EndOverlayText(EndTextKind::Initial(index)),
                ));
            }
        });

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
    time: Res<Time>,
    mut parts: Query<(
        &NitroPickupPart,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    let mut parts = parts.iter_mut();
    let Some((y, s, lat)) = proj.project(sim.0.nitro_pickup_ahead(), sim.0.road_x) else {
        for (_, _, _, mut vis) in parts {
            *vis = Visibility::Hidden;
        }
        return;
    };
    let x = (lat + sim.0.next_nitro_lane) * HALF_ROAD_PX * s;
    let pulse = 0.5 + 0.5 * (time.elapsed_secs() * 11.3).sin();
    for (part, mut tf, mut sprite, mut vis) in parts.by_ref() {
        let (color, size, dy, z) = match part.0 {
            0 => (
                Color::srgba(0.10, 0.90, 1.0, 0.12 + pulse * 0.18),
                Vec2::new((58.0 * s).max(5.0), (66.0 * s).max(6.0)),
                0.0,
                -2.35,
            ),
            1 => (
                Color::srgb(0.08, 0.78, 0.98),
                Vec2::new((30.0 * s).max(3.0), (42.0 * s).max(4.0)),
                0.0,
                -2.3,
            ),
            2 => (
                Color::srgb(0.82, 1.0, 1.0),
                Vec2::new((14.0 * s).max(2.0), (7.0 * s).max(1.0)),
                23.0 * s,
                -2.28,
            ),
            _ => (
                Color::srgb(1.0, 0.91, 0.42),
                Vec2::new((10.0 * s).max(2.0), (24.0 * s).max(3.0)),
                0.0,
                -2.27,
            ),
        };
        tf.translation = Vec3::new(x, y + dy, z);
        sprite.color = color;
        sprite.custom_size = Some(size);
        *vis = Visibility::Visible;
    }
    for (_, _, _, mut vis) in parts {
        *vis = Visibility::Hidden;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ExhaustShape {
    size: Vec2,
    center_y: f32,
    alpha: f32,
    sway: f32,
}

fn exhaust_shape(layer: u8, side: f32, elapsed: f32, active_fraction: f32) -> ExhaustShape {
    let fraction = active_fraction.clamp(0.0, 1.0);
    let phase = elapsed * 37.0 + side * 1.7 + layer as f32 * 0.83;
    let flicker = (0.84 + phase.sin() * 0.12 + (phase * 0.47).cos() * 0.04).clamp(0.68, 1.0);
    let power = 0.72 + fraction.sqrt() * 0.28;
    let (width, length, alpha) = match layer {
        0 => (13.0, 40.0, 0.78),
        1 => (9.0, 28.0, 0.92),
        _ => (5.0, 14.0, 1.0),
    };
    let length = length * flicker * power;
    ExhaustShape {
        size: Vec2::new(width * power, length),
        center_y: CAR_Y - 20.0 - length * 0.5,
        alpha: alpha * (0.72 + fraction * 0.28),
        sway: phase.sin() * 0.035,
    }
}

fn ease_toward(current: f32, target: f32, dt: f32, time_constant: f32) -> f32 {
    let response = 1.0 - (-dt.max(0.0) / time_constant.max(0.001)).exp();
    current + (target - current) * response
}

fn sync_nitro_visual_intensity(
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    time: Res<Time>,
    mut visuals: ResMut<NitroVisuals>,
) {
    let fraction = (sim.0.nitro_t / tuning.0.nitro_duration_secs).clamp(0.0, 1.0);
    let target = if crate::nitro_effects_active(&sim.0) {
        0.68 + fraction.sqrt() * 0.32
    } else {
        0.0
    };
    let time_constant = if target > visuals.intensity {
        0.09
    } else {
        0.24
    };
    visuals.intensity = ease_toward(
        visuals.intensity,
        target,
        time.delta_secs().min(0.05),
        time_constant,
    );
    if target == 0.0 && visuals.intensity < 0.002 {
        visuals.intensity = 0.0;
    }
}

fn sync_nitro_exhaust(
    visuals: Res<NitroVisuals>,
    time: Res<Time>,
    mut jets: Query<(&NitroExhaust, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    if visuals.intensity <= 0.01 {
        for (_, _, _, mut visibility) in &mut jets {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    for (jet, mut transform, mut sprite, mut visibility) in &mut jets {
        let shape = exhaust_shape(jet.layer, jet.side, time.elapsed_secs(), visuals.intensity);
        transform.translation.x = jet.side * 27.0;
        transform.translation.y = shape.center_y;
        transform.rotation = Quat::from_rotation_z(shape.sway * jet.side);
        sprite.custom_size = Some(shape.size);
        sprite.color = match jet.layer {
            0 => Color::srgba(1.0, 0.38, 0.06, shape.alpha),
            1 => Color::srgba(0.05, 0.82, 1.0, shape.alpha),
            _ => Color::srgba(0.92, 1.0, 1.0, shape.alpha),
        };
        *visibility = Visibility::Visible;
    }
}

fn speed_line_geometry(
    index: usize,
    elapsed: f32,
    intensity: f32,
    road_center: f32,
) -> (Vec2, Vec2, f32) {
    let side = if index.is_multiple_of(2) { -1.0 } else { 1.0 };
    let speed = 1.35 + (index % 5) as f32 * 0.11;
    let phase = (elapsed * speed + index as f32 * 0.618_034).fract();
    let scale = S_FAR + phase * (1.0 - S_FAR);
    let x = road_center + side * HALF_ROAD_PX * scale * 1.04;
    let lane = (index % 7) as f32 - 3.0;
    let y = scale_to_y(scale) + lane * (2.0 + scale * 3.0);
    let size = Vec2::new(
        1.5 + intensity * 1.5,
        (18.0 + scale * 52.0) * (0.58 + intensity * 0.42),
    );
    // A vertical sprite rotated with this sign is parallel to the projected
    // left/right road border: both sides converge toward the horizon.
    let border_angle = side * ((HALF_ROAD_PX * (1.0 - S_FAR)) / (HORIZON_Y - CAR_Y)).atan();
    (Vec2::new(x, y), size, border_angle)
}

fn sync_nitro_speed_lines(
    visuals: Res<NitroVisuals>,
    proj: Res<Proj>,
    time: Res<Time>,
    mut lines: Query<(
        &NitroSpeedLine,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    if visuals.intensity <= 0.015 {
        for (_, _, _, mut visibility) in &mut lines {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    for (line, mut transform, mut sprite, mut visibility) in &mut lines {
        let phase = (time.elapsed_secs() * (1.35 + (line.0 % 5) as f32 * 0.11)
            + line.0 as f32 * 0.618_034)
            .fract();
        let row = (((1.0 - phase) * (ROWS - 1) as f32).round() as usize).min(ROWS - 1);
        let (position, size, angle) = speed_line_geometry(
            line.0,
            time.elapsed_secs(),
            visuals.intensity,
            proj.rows_x[row],
        );
        transform.translation.x = position.x;
        transform.translation.y = position.y;
        transform.rotation = Quat::from_rotation_z(angle);
        sprite.custom_size = Some(size);
        let edge_strength = ((position.y - HORIZON_Y).abs() / (HORIZON_Y - CAR_Y)).clamp(0.0, 1.0);
        sprite.color = Color::srgba(
            0.72,
            0.94,
            1.0,
            visuals.intensity * (0.10 + edge_strength * 0.20),
        );
        *visibility = Visibility::Visible;
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
    effects: Res<crate::VisualEffects>,
    mut score: Query<
        &mut Text2d,
        (
            With<ScoreText>,
            Without<GearText>,
            Without<SessionText>,
            Without<TrafficText>,
        ),
    >,
    mut gear: Query<&mut Text2d, (With<GearText>, Without<TrafficText>)>,
    mut status: Query<&mut Text, (With<StatusText>, Without<SessionText>)>,
    mut session: Query<
        &mut Text2d,
        (
            With<SessionText>,
            Without<StatusText>,
            Without<ScoreText>,
            Without<GearText>,
            Without<TrafficText>,
        ),
    >,
    mut traffic: Query<&mut Text2d, (With<TrafficText>, Without<GearText>)>,
    mut nitro: Query<
        (&NitroBarSegment, &mut Transform, &mut Sprite),
        (
            With<NitroBarSegment>,
            Without<NitroSegmentHalo>,
            Without<NitroMeterHalo>,
        ),
    >,
    mut segment_halos: Query<
        (&NitroSegmentHalo, &mut Transform, &mut Sprite),
        (Without<NitroBarSegment>, Without<NitroMeterHalo>),
    >,
    mut meter_halo: Query<
        &mut Sprite,
        (
            With<NitroMeterHalo>,
            Without<NitroBarSegment>,
            Without<NitroSegmentHalo>,
        ),
    >,
    time: Res<Time>,
) {
    if let Ok(mut text) = score.single_mut() {
        text.0 = format!("{:05.0} m", sim.0.distance);
    }
    if let Ok(mut text) = gear.single_mut() {
        text.0 = match sim.0.gear() {
            Gear::Low => "LOW".to_string(),
            Gear::High => "HIGH".to_string(),
        };
    }
    if let Ok(mut text) = session.single_mut() {
        text.0 = format!(
            "TIME {} | CYCLES {:02}",
            format_time(sim.0.session_remaining),
            sim.0.day_cycles
        );
    }
    if let Ok(mut text) = traffic.single_mut() {
        text.0 = format!(
            "PASS {:03} | BUMP {:03}",
            sim.0.cars_passed, sim.0.cars_bumped
        );
    }
    let active = crate::nitro_effects_active(&sim.0);
    let active_frac = (sim.0.nitro_t / tuning.0.nitro_duration_secs).clamp(0.0, 1.0);
    let full_pulse = 0.5 + 0.5 * (time.elapsed_secs() * std::f32::consts::TAU * 1.5).sin();
    if let Ok(mut halo) = meter_halo.single_mut() {
        let alpha = if sim.0.nitro_charges == 3 && !active {
            0.10 + 0.22 * full_pulse
        } else {
            0.0
        };
        halo.color = Color::srgba(0.10, 0.86, 1.0, alpha);
    }
    let pickup_pulse = nitro_pickup_pulse(effects.nitro_pickup_glow);
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
        tf.scale = if effects.nitro_segment == Some(i) {
            Vec3::splat(1.0 + 0.22 * pickup_pulse)
        } else {
            Vec3::ONE
        };
        sp.color = if frac > 0.0 {
            Color::srgb(0.10, 0.86 + 0.14 * pickup_pulse, 1.0)
        } else {
            Color::srgb(0.10, 0.16, 0.20)
        };
    }
    for (halo, mut tf, mut sprite) in &mut segment_halos {
        let selected = effects.nitro_segment == Some(halo.0);
        let fade = (effects.nitro_pickup_glow / crate::NITRO_PICKUP_GLOW_SECS).clamp(0.0, 1.0);
        tf.scale = Vec3::splat(if selected {
            1.0 + 0.18 * pickup_pulse
        } else {
            1.0
        });
        sprite.color = Color::srgba(0.45, 0.95, 1.0, if selected { 0.65 * fade } else { 0.0 });
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
                Phase::GameOver { .. } => String::new(),
            }
        } else {
            match &sim.0.phase {
                Phase::Ready => {
                    "ARROWS steer | G gear | Y nitro | R restart | T toy | L toy look".to_string()
                }
                Phase::Running => String::new(),
                Phase::GameOver { .. } => String::new(),
            }
        };
    }
}

fn sync_end_overlay(
    sim: Res<Sim>,
    end: Res<crate::EndSession>,
    leaderboard: Res<crate::LeaderboardState>,
    mut root: Query<&mut Visibility, With<EndOverlayRoot>>,
    mut texts: Query<(&EndOverlayText, &mut Text2d)>,
) {
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = if end.0.mode == crate::session::EndMode::Playing {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    let result = sim.0.result().copied().unwrap_or_else(|| {
        game_core::SessionResult::new(sim.0.day_cycles, sim.0.cars_passed, sim.0.cars_bumped)
    });
    let table = leaderboard_table(&leaderboard.entries);
    for (marker, mut text) in &mut texts {
        text.0 = match marker.0 {
            EndTextKind::Summary => format!(
                "SCORE {:06}   CYCLES {:02}   PASSED {:03}   BUMPED {:03}",
                result.score, result.day_cycles, result.cars_passed, result.cars_bumped
            ),
            EndTextKind::Prompt => match end.0.mode {
                crate::session::EndMode::NameEntry => {
                    "TYPE YOUR 3-LETTER NAME  •  ENTER TO SUBMIT".to_string()
                }
                crate::session::EndMode::Leaderboard => "LOCAL TOP 5".to_string(),
                crate::session::EndMode::Playing => String::new(),
            },
            EndTextKind::Initial(index) => end.0.initials.cell(index as usize).to_string(),
            EndTextKind::Table => table.clone(),
            EndTextKind::Footer => match end.0.mode {
                crate::session::EndMode::NameEntry => {
                    "A-Z enters letters  •  BACKSPACE edits".to_string()
                }
                crate::session::EndMode::Leaderboard => "R or ENTER starts a new run".to_string(),
                crate::session::EndMode::Playing => String::new(),
            },
        };
    }
}

fn leaderboard_table(entries: &[game_core::LeaderboardEntry]) -> String {
    let mut lines = vec!["#   TAG    SCORE    CYC   PASS   BUMP".to_string()];
    for rank in 0..5 {
        if let Some(entry) = entries.get(rank) {
            lines.push(format!(
                "{}   {}   {:06}    {:02}    {:03}    {:03}",
                rank + 1,
                entry.initials_str(),
                entry.result.score,
                entry.result.day_cycles,
                entry.result.cars_passed,
                entry.result.cars_bumped
            ));
        } else {
            lines.push(format!("{}   ---   ------    --    ---    ---", rank + 1));
        }
    }
    lines.join("\n")
}

fn nitro_pickup_pulse(remaining: f32) -> f32 {
    let progress = 1.0 - (remaining / crate::NITRO_PICKUP_GLOW_SECS).clamp(0.0, 1.0);
    (progress * std::f32::consts::PI).sin().max(0.0)
}

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

fn weather_weights(transition: WeatherTransition) -> [f32; 3] {
    fn index(kind: WeatherKind) -> usize {
        match kind {
            WeatherKind::Clear => 0,
            WeatherKind::Cloudy => 1,
            WeatherKind::Rain => 2,
        }
    }
    let mut weights = [0.0; 3];
    if transition.from == transition.to {
        weights[index(transition.to)] = 1.0;
    } else {
        weights[index(transition.from)] = 1.0 - transition.blend;
        weights[index(transition.to)] = transition.blend;
    }
    weights
}

fn rain_visibility(rain: f32, winter: f32) -> f32 {
    rain.clamp(0.0, 1.0) * (1.0 - winter.clamp(0.0, 1.0))
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sync_environment(
    sim: Res<Sim>,
    time: Res<Time>,
    mut colors: ResMut<Palette>,
    mut visual_weather: ResMut<WeatherVisuals>,
    mut sky: Query<
        &mut Sprite,
        (
            With<SkyBand>,
            Without<SkyObject>,
            Without<Cloud>,
            Without<HazeStrip>,
            Without<SnowFlake>,
            Without<RainStreak>,
        ),
    >,
    mut sky_objs: Query<
        &mut Sprite,
        (
            With<SkyObject>,
            Without<SkyBand>,
            Without<Cloud>,
            Without<HazeStrip>,
            Without<SnowFlake>,
            Without<RainStreak>,
        ),
    >,
    mut clouds: Query<
        (&mut Transform, &mut Sprite),
        (
            With<Cloud>,
            Without<SkyBand>,
            Without<SkyObject>,
            Without<HazeStrip>,
            Without<SnowFlake>,
            Without<RainStreak>,
        ),
    >,
    mut haze: Query<
        &mut Sprite,
        (
            With<HazeStrip>,
            Without<SkyBand>,
            Without<SkyObject>,
            Without<SnowFlake>,
            Without<Cloud>,
            Without<RainStreak>,
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
            Without<RainStreak>,
        ),
    >,
    mut rain: Query<
        (&RainStreak, &mut Transform, &mut Sprite),
        (
            Without<SkyBand>,
            Without<SkyObject>,
            Without<Cloud>,
            Without<HazeStrip>,
            Without<SnowFlake>,
        ),
    >,
) {
    let t = sim.0.environment_time;
    let day = game_core::environment::daylight(t);
    let w = game_core::environment::season_weights(t);

    let winter = w[3];
    let autumn = w[2];
    let weather = weather_weights(sim.0.weather());
    let (clear, cloudy, rain_amount) = (weather[0], weather[1], weather[2]);
    let overcast = (cloudy + rain_amount).clamp(0.0, 1.0);
    let rain_visible = rain_visibility(rain_amount, winter);
    visual_weather.clear = clear;
    visual_weather.cloudy = cloudy;
    visual_weather.rain = rain_amount;
    visual_weather.rain_visible = rain_visible;
    let weather_dim = 1.0 - cloudy * 0.12 - rain_amount * 0.22;
    let bright = (0.35 + 0.65 * day) * weather_dim;

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
        c = c.mix(&Color::srgb(0.35, 0.41, 0.50), overcast * 0.38);
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
    let dt = time.delta_secs();
    for (i, (mut tf, mut sp)) in clouds.iter_mut().enumerate() {
        let density = 0.08 * clear + 0.68 * cloudy + 0.86 * rain_amount;
        let night_visibility = 0.35 + day * 0.65;
        sp.color = Color::srgba(
            0.78 + day * 0.17,
            0.82 + day * 0.14,
            0.88 + day * 0.10,
            (density * night_visibility).clamp(0.0, 0.9),
        );
        tf.translation.x += (8.0 + (i % 4) as f32 * 2.5) * dt;
        if tf.translation.x > 530.0 {
            tf.translation.x = -530.0;
        }
    }

    // haze: denser in winter and at dawn/dusk
    if let Ok(mut sp) = haze.single_mut() {
        let haze_amt =
            (0.18 + winter * 0.35 + autumn * 0.18 + overcast * 0.18) * (0.65 + 0.35 * (1.0 - day));
        let c = Color::srgb(0.75, 0.8, 0.88).mix(&Color::srgb(1.0, 1.0, 1.0), winter * 0.5);
        sp.color = c.with_alpha(haze_amt.clamp(0.0, 0.75));
    }

    // snow: fall while winter holds weight
    let fall = 90.0 + winter * 60.0;
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

    for (drop, mut tf, mut sp) in &mut rain {
        let speed = 260.0 + (drop.0 % 9) as f32 * 10.0;
        let drift = 60.0 + (drop.0 % 7) as f32 * 5.0;
        tf.translation.y -= speed * dt;
        tf.translation.x -= drift * dt;
        if tf.translation.y < -320.0 || tf.translation.x < -510.0 {
            tf.translation.y = 320.0 + (drop.0 % 11) as f32 * 8.0;
            tf.translation.x = -440.0 + (drop.0 * 67 % 940) as f32;
        }
        sp.color = Color::srgba(0.72, 0.88, 1.0, rain_visible * (0.34 + day * 0.18));
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
            sync_nitro_visual_intensity,
            sync_nitro_exhaust,
            sync_nitro_speed_lines,
            sync_flash,
            sync_drum_overlays,
            sync_hud,
            sync_end_overlay,
        )
            .chain(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_crossfade_weights_are_normalized() {
        let weights = weather_weights(WeatherTransition {
            from: WeatherKind::Clear,
            to: WeatherKind::Rain,
            blend: 0.25,
            slot: 1,
        });
        assert_eq!(weights, [0.75, 0.0, 0.25]);
        assert!((weights.iter().sum::<f32>() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn settled_weather_has_one_full_weight() {
        assert_eq!(
            weather_weights(WeatherTransition {
                from: WeatherKind::Cloudy,
                to: WeatherKind::Cloudy,
                blend: 1.0,
                slot: 2,
            }),
            [0.0, 1.0, 0.0]
        );
    }

    #[test]
    fn winter_suppresses_rain_streaks_without_changing_weather_state() {
        assert_eq!(rain_visibility(1.0, 0.0), 1.0);
        assert_eq!(rain_visibility(1.0, 1.0), 0.0);
        assert!((rain_visibility(0.8, 0.25) - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn pickup_glow_peaks_halfway_and_ends_cleanly() {
        assert!(nitro_pickup_pulse(crate::NITRO_PICKUP_GLOW_SECS) < 1e-6);
        assert!((nitro_pickup_pulse(crate::NITRO_PICKUP_GLOW_SECS * 0.5) - 1.0).abs() < 1e-6);
        assert!(nitro_pickup_pulse(0.0) < 1e-6);
    }

    #[test]
    fn nitro_exhaust_layers_are_nested_and_respond_to_charge() {
        let outer = exhaust_shape(0, -1.0, 0.25, 1.0);
        let cyan = exhaust_shape(1, -1.0, 0.25, 1.0);
        let core = exhaust_shape(2, -1.0, 0.25, 1.0);
        assert!(outer.size.x > cyan.size.x && cyan.size.x > core.size.x);
        assert!(outer.size.y > cyan.size.y && cyan.size.y > core.size.y);
        let draining = exhaust_shape(0, -1.0, 0.25, 0.1);
        assert!(outer.size.y > draining.size.y);
        assert!(outer.center_y < CAR_Y - 20.0);
    }

    #[test]
    fn nitro_visual_envelope_attacks_and_releases_without_overshoot() {
        let attack = ease_toward(0.0, 1.0, 1.0 / 60.0, 0.09);
        let release = ease_toward(attack, 0.0, 1.0 / 60.0, 0.24);
        assert!(attack > 0.0 && attack < 1.0);
        assert!(release >= 0.0 && release < attack);
    }

    #[test]
    fn speed_lines_converge_with_road_borders_and_scale_with_intensity() {
        let (far_left, _, left_angle) = speed_line_geometry(0, 0.0, 1.0, 0.0);
        let (far_right, _, right_angle) = speed_line_geometry(1, -0.618_034 / 1.46, 1.0, 0.0);
        assert!(far_left.x < 0.0 && far_right.x > 0.0);
        assert!(left_angle < 0.0 && right_angle > 0.0);
        assert!((left_angle + right_angle).abs() < 1e-6);

        let (near, _, _) = speed_line_geometry(0, 0.7, 1.0, 0.0);
        assert!(
            near.x.abs() > far_left.x.abs(),
            "border widens toward player"
        );
        let (_, faint, _) = speed_line_geometry(3, 1.25, 0.1, 0.0);
        let (_, strong, _) = speed_line_geometry(3, 1.25, 1.0, 0.0);
        assert!(strong.y > faint.y);
    }
}
