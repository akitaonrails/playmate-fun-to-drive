//! Fun to Drive — the 2D drum view, OutRun-inspired (PLAN: user tweak 2).
//!
//! Classic 16-bit racer look: gradient sky with sun and clouds, sea strip at
//! the horizon, sandy shoulder, scrolling grass stripes, red/white rumble
//! edges, lane dashes, palm trees, and composite car sprites — all drawn
//! with flat sprites so it renders identically into the toy's screen.

mod ambient;

use bevy::prelude::*;
use game_core::{Gear, Phase, WeatherKind, WeatherTransition};

use crate::{Sim, TuningRes};

pub const M_PER_PX: f32 = 0.08;
pub const HALF_ROAD_PX: f32 = 210.0;
pub const CAR_Y: f32 = -215.0;
pub const HORIZON_Y: f32 = 120.0;
const PLAYER_CAR_WIDTH_PX: f32 = 96.0;
const TRAFFIC_CAR_WIDTH_PX: f32 = 56.0;

// pseudo-3D projection (classic OutRun style): geometric row spacing,
// cumulative curve bend, sprites scaled by distance
pub const ROWS: usize = 30;
pub const D_MIN: f32 = 2.2; // meters to the nearest row
pub const S_FAR: f32 = 0.045; // scale at the horizon row
pub const CURVE_BEND: f32 = 0.075; // lateral half-widths gained per curve-meter
const ELEVATION_PX_PER_M: f32 = 105.0;

fn row_scale(i: usize) -> f32 {
    S_FAR.powf(i as f32 / ROWS as f32)
}
fn scale_to_y(s: f32) -> f32 {
    CAR_Y + (HORIZON_Y - CAR_Y) * (1.0 - s) / (1.0 - S_FAR)
}
fn scale_to_d(s: f32) -> f32 {
    D_MIN / s
}
fn elevated_row_y(scale: f32, relative_elevation: f32) -> f32 {
    scale_to_y(scale) + relative_elevation * ELEVATION_PX_PER_M * scale
}
const VIEW_TOP: f32 = 322.0;
const PALM_POOL: usize = 10;
const HAZARD_POOL: usize = 40;
const HAZARD_PARTS: usize = 6;
const NITRO_X_LEFT: f32 = 302.0;
const NITRO_Y: f32 = 216.0;
const RAIN_POOL: usize = 84;
const SPEED_LINE_POOL: usize = 28;
const VISIBILITY_BANDS: usize = 8;

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
struct HazardPart(u8);

#[derive(Component)]
struct NightShade;

#[derive(Component)]
struct RoadFogBand(usize);

#[derive(Component)]
struct HeadlightBand(usize);

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
struct SpeedNeedle;
#[derive(Component)]
struct SpeedDigital;
#[derive(Component)]
struct SpeedTick {
    red_zone: bool,
}
#[derive(Component)]
struct GearPanelText(Gear);
#[derive(Component)]
struct RecoveryText;
#[derive(Component)]
struct CooldownFill;
#[derive(Component)]
struct CooldownText;

#[derive(Component)]
struct DifficultyOverlayRoot;
#[derive(Component)]
struct DifficultyCard(usize);
#[derive(Component)]
struct DifficultyCardText(usize);

#[derive(Resource, Default)]
pub struct InstrumentVisuals {
    pub speed_kmh: f32,
    pub needle_angle: f32,
    pub overrev: f32,
    pub cooldown: f32,
    pub gauge_fill: f32,
    pub recovery: &'static str,
    pub cue: &'static str,
    pub low_shift_hint_secs: f32,
}

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

/// Visibility envelope shared by the road overlays and distant traffic.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct RoadVisibilityVisuals {
    pub night: f32,
    pub rain: f32,
    pub shade_alpha: f32,
    pub fog_alpha: f32,
    pub headlights: f32,
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
#[derive(Resource)]
pub struct Proj {
    /// lateral center (half-width units incl. bend) at each row's distance
    pub rows_x: [f32; ROWS],
    /// projected lower edge of each road row after terrain elevation
    pub rows_y: [f32; ROWS],
    /// cumulative curve at each row distance (half-width units)
    pub cum: [f32; ROWS],
    scales: [f32; ROWS],
    distances: [f32; ROWS],
}

impl Default for Proj {
    fn default() -> Self {
        let scales = std::array::from_fn(row_scale);
        let distances = std::array::from_fn(|index| scale_to_d(scales[index]));
        Self {
            rows_x: [0.0; ROWS],
            rows_y: [0.0; ROWS],
            cum: [0.0; ROWS],
            scales,
            distances,
        }
    }
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
                let di = self.distances[i];
                if d <= di {
                    let d0 = if i == 0 { D_MIN } else { self.distances[i - 1] };
                    let t = if di > d0 { (d - d0) / (di - d0) } else { 1.0 };
                    let c0 = if i == 0 { 0.0 } else { self.cum[i - 1] };
                    val = c0 + (self.cum[i] - c0) * t;
                    break;
                }
                val = self.cum[i];
            }
            val
        };
        let y = {
            let mut val = self.rows_y[ROWS - 1];
            for i in 0..ROWS {
                let di = self.distances[i];
                if d <= di {
                    let d0 = if i == 0 { D_MIN } else { self.distances[i - 1] };
                    let t = if di > d0 { (d - d0) / (di - d0) } else { 1.0 };
                    let y0 = if i == 0 { CAR_Y } else { self.rows_y[i - 1] };
                    val = y0 + (self.rows_y[i] - y0) * t;
                    break;
                }
                val = self.rows_y[i];
            }
            val
        };
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

const SPEED_CENTER: Vec2 = Vec2::new(365.0, -198.0);
const SPEED_RADIUS: f32 = 82.0;

fn speedometer_angle(kmh: f32) -> f32 {
    225.0_f32.to_radians() - (kmh.clamp(0.0, 120.0) / 120.0) * 270.0_f32.to_radians()
}

fn speedometer_danger(kmh: f32, nitro_active: bool) -> bool {
    nitro_active || kmh >= 90.0
}

fn normal_recovery_cue(gear: Gear, speed: f32, shift_threshold: f32) -> &'static str {
    if gear == Gear::Low && speed >= shift_threshold {
        "SHIFT HIGH"
    } else {
        "LB LOW | RB HIGH"
    }
}

fn spawn_instruments(commands: &mut Commands) {
    commands.spawn((
        Sprite::from_color(
            Color::srgba(0.015, 0.02, 0.035, 0.86),
            Vec2::new(190.0, 190.0),
        ),
        Transform::from_xyz(SPEED_CENTER.x, SPEED_CENTER.y, 0.82),
        HudPanel,
    ));
    for kmh in (0..=120).step_by(10) {
        let a = speedometer_angle(kmh as f32);
        let p = SPEED_CENTER + Vec2::new(a.cos(), a.sin()) * SPEED_RADIUS;
        let major = kmh % 20 == 0;
        commands.spawn((
            Sprite::from_color(
                if kmh >= 90 {
                    Color::srgb(0.9, 0.16, 0.12)
                } else {
                    Color::srgb(0.70, 0.90, 0.96)
                },
                Vec2::new(
                    if major { 3.0 } else { 2.0 },
                    if major { 16.0 } else { 10.0 },
                ),
            ),
            Transform::from_xyz(p.x, p.y, 0.90)
                .with_rotation(Quat::from_rotation_z(a - std::f32::consts::FRAC_PI_2)),
            SpeedTick {
                red_zone: kmh >= 90,
            },
        ));
        if major {
            let lp = SPEED_CENTER + Vec2::new(a.cos(), a.sin()) * 61.0;
            commands.spawn((
                Text2d::new(kmh.to_string()),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.88, 0.93)),
                Transform::from_xyz(lp.x, lp.y, 0.91),
            ));
        }
    }
    commands.spawn((
        Sprite::from_color(Color::srgb(1.0, 0.85, 0.28), Vec2::new(4.0, 64.0)),
        Transform::from_xyz(SPEED_CENTER.x, SPEED_CENTER.y + 32.0, 0.96),
        SpeedNeedle,
    ));
    commands.spawn((
        Text2d::new("000 km/h"),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.93, 0.62)),
        Transform::from_xyz(SPEED_CENTER.x, SPEED_CENTER.y - 27.0, 0.97),
        SpeedDigital,
    ));
    commands.spawn((
        Sprite::from_color(
            Color::srgba(0.015, 0.02, 0.035, 0.86),
            Vec2::new(176.0, 108.0),
        ),
        Transform::from_xyz(-378.0, -226.0, 0.82),
        HudPanel,
    ));
    for (gear, label, x) in [(Gear::Low, "LOW", -421.0), (Gear::High, "HIGH", -337.0)] {
        commands.spawn((
            Text2d::new(label),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(if gear == Gear::Low {
                Color::srgb(1.0, 0.86, 0.32)
            } else {
                Color::srgb(0.20, 0.28, 0.32)
            }),
            Transform::from_xyz(x, -205.0, 0.94),
            GearPanelText(gear),
        ));
    }
    commands.spawn((
        Text2d::new("LB LOW | RB HIGH"),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgb(0.65, 0.86, 0.94)),
        Transform::from_xyz(-378.0, -246.0, 0.94),
        RecoveryText,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.08, 0.11, 0.16), Vec2::new(12.0, 108.0)),
        Transform::from_xyz(459.0, -205.0, 0.90),
        HudPanel,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.12, 0.86, 0.95), Vec2::new(8.0, 104.0)),
        Transform::from_xyz(459.0, -205.0, 0.92),
        CooldownFill,
    ));
    commands.spawn((
        Text2d::new("BOOST"),
        TextFont {
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.66, 0.92, 1.0)),
        Transform::from_xyz(438.0, -271.0, 0.94),
        CooldownText,
    ));
}

fn spawn_difficulty_overlay(commands: &mut Commands) {
    commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, 30.0),
            Visibility::Visible,
            DifficultyOverlayRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite::from_color(
                    Color::srgba(0.01, 0.02, 0.05, 0.82),
                    Vec2::new(960.0, 600.0),
                ),
                Transform::default(),
            ));
            parent.spawn((
                Sprite::from_color(
                    Color::srgba(0.025, 0.07, 0.14, 0.98),
                    Vec2::new(780.0, 390.0),
                ),
                Transform::from_xyz(0.0, 0.0, 0.1),
            ));
            parent.spawn((
                Text2d::new("SELECT RUN"),
                TextFont {
                    font_size: FontSize::Px(38.0),
                    ..default()
                },
                TextColor(Color::srgb(0.50, 0.95, 1.0)),
                Transform::from_xyz(0.0, 135.0, 0.3),
            ));
            for (index, x, title, duration) in [
                (0, -230.0, "EASY", "2:00"),
                (1, 0.0, "MEDIUM", "6:00"),
                (2, 230.0, "HARD", "10:00"),
            ] {
                parent.spawn((
                    Sprite::from_color(
                        Color::srgba(0.06, 0.16, 0.27, 0.96),
                        Vec2::new(200.0, 150.0),
                    ),
                    Transform::from_xyz(x, 5.0, 0.2),
                    DifficultyCard(index),
                ));
                parent.spawn((
                    Text2d::new(format!("{title}\n{duration}")),
                    TextFont {
                        font_size: FontSize::Px(25.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.78, 0.92, 0.98)),
                    Transform::from_xyz(x, 5.0, 0.3),
                    DifficultyCardText(index),
                ));
            }
            parent.spawn((
                Text2d::new("<   ARROWS / LEFT STICK   >"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.90, 1.0)),
                Transform::from_xyz(0.0, -112.0, 0.3),
            ));
            parent.spawn((
                Text2d::new("ENTER / A / START TO BEGIN"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.88, 0.45)),
                Transform::from_xyz(0.0, -153.0, 0.3),
            ));
        });
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(palette());
    commands.init_resource::<Proj>();
    commands.init_resource::<WeatherVisuals>();
    commands.init_resource::<RoadVisibilityVisuals>();
    commands.init_resource::<NitroVisuals>();
    commands.init_resource::<InstrumentVisuals>();
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
    ambient::spawn(&mut commands);
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
    // Visibility layers sit over the projected world but below the player,
    // precipitation, and HUD. Distant traffic lamps are raised above these.
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::new(960.0, 600.0)),
        Transform::from_xyz(0.0, 0.0, 0.10),
        NightShade,
    ));
    for index in 0..VISIBILITY_BANDS {
        let (y, size, _) = fog_band_geometry(index);
        commands.spawn((
            Sprite::from_color(Color::srgba(0.07, 0.09, 0.13, 0.0), size),
            Transform::from_xyz(0.0, y, 0.14),
            RoadFogBand(index),
        ));
        let (y, size, _) = headlight_band_geometry(index);
        commands.spawn((
            Sprite::from_color(Color::srgba(1.0, 0.88, 0.58, 0.0), size),
            Transform::from_xyz(0.0, y, 0.20),
            HeadlightBand(index),
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
        for part in 0..HAZARD_PARTS {
            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::new(56.0, 24.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, CAR_Y, -2.5),
                Visibility::Hidden,
                HazardPart(part as u8),
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
        Sprite::from_color(
            Color::srgb(0.86, 0.10, 0.08),
            Vec2::new(PLAYER_CAR_WIDTH_PX - 4.0, 30.0),
        ),
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

    // Three-cell nitro meter, rendered into the toy screen. It must be filled
    // completely before use; firing consumes all three cells at once.
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 0.78, 0.12, 0.0), Vec2::new(228.0, 42.0)),
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
        TextColor(Color::srgb(1.0, 0.86, 0.34)),
        Transform::from_xyz(258.0, NITRO_Y, 1.1),
    ));
    for i in 0..3u8 {
        commands.spawn((
            Sprite::from_color(Color::srgba(1.0, 0.84, 0.24, 0.0), Vec2::new(52.0, 28.0)),
            Transform::from_xyz(NITRO_X_LEFT + i as f32 * 40.0 + 17.0, NITRO_Y, 1.05),
            NitroSegmentHalo(i),
        ));
        commands.spawn((
            Sprite::from_color(Color::srgb(0.10, 0.16, 0.20), Vec2::new(34.0, 14.0)),
            Transform::from_xyz(NITRO_X_LEFT + i as f32 * 40.0 + 17.0, NITRO_Y, 1.1),
            NitroBarSegment(i),
        ));
    }

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
    spawn_instruments(&mut commands);
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

    spawn_difficulty_overlay(&mut commands);

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
    let player_elevation = sim.0.road.elevation_at(sim.0.distance);
    for i in 0..ROWS {
        let s = proj.scales[i];
        let d = proj.distances[i];
        let pos = sim.0.distance + d;
        cum += sim.0.road.curve_at(pos) * (d - if i == 0 { D_MIN } else { proj.distances[i - 1] });
        proj.rows_x[i] = HALF_ROAD_PX * s * (sim.0.road_x + CURVE_BEND * cum);
        let relative_elevation = sim.0.road.elevation_at(pos) - player_elevation;
        proj.rows_y[i] = elevated_row_y(s, relative_elevation);
        proj.cum[i] = cum;
    }

    for (i, (mut tf, mut sp)) in rows.iter_mut().enumerate() {
        tf.translation.x = proj.rows_x[i];
        let y0 = proj.rows_y[i];
        let y1 = if i + 1 < ROWS {
            proj.rows_y[i + 1]
        } else {
            HORIZON_Y + 2.0
        };
        let h = (y1 - y0).max(1.5);
        tf.translation.y = y0 + h * 0.5;
        if let Some(size) = sp.custom_size.as_mut() {
            size.y = h + 1.0;
        }
        let pos = sim.0.distance + proj.distances[i];
        let on = ((pos / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on {
            colors.asphalt_a
        } else {
            colors.asphalt_b
        };
    }
    for (i, (mut tf, mut sp)) in grass.iter_mut().enumerate() {
        tf.translation.x = 0.0;
        let y0 = proj.rows_y[i];
        let y1 = if i + 1 < ROWS {
            proj.rows_y[i + 1]
        } else {
            HORIZON_Y + 2.0
        };
        let h = (y1 - y0).max(1.5);
        tf.translation.y = y0 + h * 0.5;
        if let Some(size) = sp.custom_size.as_mut() {
            size.y = h + 1.0;
        }
        let pos = sim.0.distance + proj.distances[i];
        let on = ((pos / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on { colors.grass_a } else { colors.grass_b };
    }
    // foreground apron follows row 0 (grass full width + road band + edges)
    if let Some(&x0) = proj.rows_x.first() {
        let pos0 = sim.0.distance + D_MIN;
        let on0 = ((pos0 / 3.5).floor() as i64).rem_euclid(2) == 0;
        for (idx, (mut tf, mut sp)) in apron.iter_mut().enumerate() {
            let apron_bottom = -322.0;
            let h = (proj.rows_y[0] - apron_bottom).max(1.0);
            tf.translation.y = apron_bottom + h * 0.5;
            if let Some(size) = sp.custom_size.as_mut() {
                size.y = h + 2.0;
            }
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
        let s = proj.scales[row];
        tf.translation.x = proj.rows_x[row] + side * (HALF_ROAD_PX - 8.0) * s;
        let y0 = proj.rows_y[row];
        let y1 = if row + 1 < ROWS {
            proj.rows_y[row + 1]
        } else {
            HORIZON_Y + 2.0
        };
        let h = (y1 - y0).max(1.5);
        tf.translation.y = y0 + h * 0.5;
        if let Some(size) = sp.custom_size.as_mut() {
            size.y = h + 1.0;
        }
        let pos = sim.0.distance + proj.distances[row];
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
        let s = proj.scales[row];
        tf.translation.x = proj.rows_x[row] + lane * HALF_ROAD_PX * s;
        let y0 = proj.rows_y[row];
        let y1 = if row + 1 < ROWS {
            proj.rows_y[row + 1]
        } else {
            HORIZON_Y + 2.0
        };
        let h = (y1 - y0).max(1.5);
        tf.translation.y = y0 + h * 0.5;
        if let Some(size) = sp.custom_size.as_mut() {
            size.y = h * 0.6;
        }
        let pos = sim.0.distance + proj.distances[row];
        let on = ((pos / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on { colors.dash } else { colors.asphalt_a };
    }
}

#[allow(clippy::type_complexity)]
fn sync_hazards(
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    proj: Res<Proj>,
    road_visibility: Res<RoadVisibilityVisuals>,
    mut pool: Query<(&HazardPart, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let lp = sim.0.loop_pos();
    let mut parts = pool.iter_mut();
    let mut place = |kind: game_core::HazardKind, color_variant: usize, x: f32, y: f32, s: f32| {
        let (body_alpha, light_alpha) = traffic_visibility(s, *road_visibility);
        for _ in 0..HAZARD_PARTS {
            let Some((part, mut tf, mut sp, mut vis)) = parts.next() else {
                return;
            };
            let part = part.0 as usize;
            if kind != game_core::HazardKind::Oncoming && part >= 4 {
                *vis = Visibility::Hidden;
                continue;
            }
            let (color, size, dy, z) = match (kind, part) {
                (game_core::HazardKind::Oncoming, 0) => (
                    traffic_car_color(color_variant, body_alpha),
                    Vec2::new(TRAFFIC_CAR_WIDTH_PX, TRAFFIC_CAR_WIDTH_PX * 0.42),
                    0.0,
                    -2.5,
                ),
                (game_core::HazardKind::Oncoming, 1) => (
                    Color::srgba(0.15, 0.22, 0.35, body_alpha),
                    Vec2::new(TRAFFIC_CAR_WIDTH_PX * 0.5, TRAFFIC_CAR_WIDTH_PX * 0.18),
                    TRAFFIC_CAR_WIDTH_PX * 0.12,
                    -2.4,
                ),
                (game_core::HazardKind::Oncoming, 2) => (
                    Color::srgba(1.0, 0.9, 0.4, body_alpha),
                    Vec2::new(TRAFFIC_CAR_WIDTH_PX * 0.9, TRAFFIC_CAR_WIDTH_PX * 0.09),
                    -TRAFFIC_CAR_WIDTH_PX * 0.18,
                    -2.4,
                ),
                (game_core::HazardKind::Oncoming, 3) => (
                    Color::srgba(0.06, 0.06, 0.07, body_alpha),
                    Vec2::new(TRAFFIC_CAR_WIDTH_PX * 0.95, TRAFFIC_CAR_WIDTH_PX * 0.1),
                    -TRAFFIC_CAR_WIDTH_PX * 0.24,
                    -2.6,
                ),
                (game_core::HazardKind::Oncoming, _) => (
                    Color::srgba(1.0, 0.91, 0.54, light_alpha),
                    Vec2::new(8.0, 7.0),
                    -TRAFFIC_CAR_WIDTH_PX * 0.08,
                    0.26,
                ),
                (game_core::HazardKind::Tree, 0) => (
                    Color::srgb(0.34, 0.18, 0.07),
                    Vec2::new(8.0, 34.0),
                    17.0,
                    -2.55,
                ),
                (game_core::HazardKind::Tree, 1) => (
                    Color::srgb(0.05, 0.30, 0.12),
                    Vec2::new(35.0, 25.0),
                    37.0,
                    -2.45,
                ),
                (game_core::HazardKind::Tree, 2) => (
                    Color::srgb(0.10, 0.48, 0.19),
                    Vec2::new(25.0, 20.0),
                    51.0,
                    -2.42,
                ),
                (game_core::HazardKind::Tree, _) => (
                    Color::srgba(0.02, 0.08, 0.03, 0.55),
                    Vec2::new(30.0, 6.0),
                    0.0,
                    -2.62,
                ),
                (game_core::HazardKind::Billboard, 0) => (
                    Color::srgb(0.95, 0.35, 0.12),
                    Vec2::new(44.0, 27.0),
                    30.0,
                    -2.46,
                ),
                (game_core::HazardKind::Billboard, 1) => (
                    Color::srgb(0.98, 0.90, 0.38),
                    Vec2::new(35.0, 18.0),
                    30.0,
                    -2.42,
                ),
                (game_core::HazardKind::Billboard, 2) => (
                    Color::srgb(0.30, 0.24, 0.20),
                    Vec2::new(5.0, 29.0),
                    9.0,
                    -2.54,
                ),
                (game_core::HazardKind::Billboard, _) => (
                    Color::srgb(0.12, 0.60, 0.84),
                    Vec2::new(38.0, 4.0),
                    38.0,
                    -2.39,
                ),
            };
            let light_x = if kind == game_core::HazardKind::Oncoming && part >= 4 {
                let side = if part == 4 { -1.0 } else { 1.0 };
                side * (TRAFFIC_CAR_WIDTH_PX * 0.29 * s).max(2.4)
            } else {
                0.0
            };
            tf.translation = Vec3::new(x + light_x, y + dy * s, z);
            sp.color = color;
            let min_size = if part >= 4 { 2.4 } else { 1.5 };
            sp.custom_size = Some(Vec2::new(
                (size.x * s).max(min_size),
                (size.y * s).max(min_size),
            ));
            *vis = Visibility::Visible;
        }
    };

    for (hazard_index, h) in sim.0.road.hazards.iter().enumerate() {
        let ahead = (h.s - lp).rem_euclid(sim.0.road.length);
        let Some((y, s, lat)) = proj.project(ahead, sim.0.road_x) else {
            continue;
        };
        let x = (lat + sim.0.hazard_x_at(h, ahead, &tuning.0)) * HALF_ROAD_PX * s;
        place(h.kind, hazard_index, x, y, s);
    }
    for (_, _, _, mut vis) in parts.by_ref() {
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
    let mut spots = [None; 6];
    let mut spot_count = 0;
    for k in 0..3u32 {
        for side in [-1.45f32, 1.45] {
            let s_pos = (k as f32 + if side < 0.0 { 0.5 } else { 0.0 }) / 3.0 * l;
            let ahead = (s_pos - lp).rem_euclid(l);
            if let Some((y, s, lat)) = proj.project(ahead, sim.0.road_x) {
                let x = (lat + side) * HALF_ROAD_PX * s;
                spots[spot_count] = Some((x, y, s));
                spot_count += 1;
            }
        }
    }
    let mut parts = pool.iter_mut();
    for (x, y, s) in spots.into_iter().flatten() {
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
    audio: Res<crate::audio::AudioState>,
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
    let full_pulse = 0.5 + 0.5 * (time.elapsed_secs() * std::f32::consts::TAU * 1.5).sin();
    if let Ok(mut halo) = meter_halo.single_mut() {
        let alpha = if sim.0.nitro_charges == 3 && !active {
            0.10 + 0.22 * full_pulse
        } else {
            0.0
        };
        halo.color = Color::srgba(1.0, 0.78, 0.12, alpha);
    }
    let pickup_pulse = nitro_pickup_pulse(effects.nitro_pickup_glow);
    for (segment, mut tf, mut sp) in &mut nitro {
        let i = segment.0;
        let filled = i < sim.0.nitro_charges;
        let w = 34.0;
        tf.translation.x = NITRO_X_LEFT + i as f32 * 40.0 + w * 0.5;
        sp.custom_size = Some(Vec2::new(w, 14.0));
        tf.scale = if effects.nitro_segment == Some(i) {
            Vec3::splat(1.0 + 0.22 * pickup_pulse)
        } else {
            Vec3::ONE
        };
        sp.color = if filled {
            Color::srgb(1.0, 0.72 + 0.20 * pickup_pulse, 0.10)
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
        sprite.color = Color::srgba(1.0, 0.84, 0.24, if selected { 0.65 * fade } else { 0.0 });
    }
    if let Ok(mut text) = status.single_mut() {
        let next = if audio.muted {
            "SOUND OFF | M to unmute"
        } else if sim.0.paused {
            "PAUSED | Start/P resume | Select/R restart"
        } else if view.mode == crate::toy_view::ViewMode::Toy {
            match &sim.0.phase {
                Phase::Ready => {
                    "ARROWS drive | Q low / E high | Y nitro | R restart | T flat | L toy look"
                }
                Phase::Running => "Y nitro | T flat view | L toy look",
                Phase::GameOver { .. } => "",
            }
        } else {
            match &sim.0.phase {
                Phase::Ready => {
                    "ARROWS steer | Q low / E high | Y nitro | R restart | T toy | L toy look"
                }
                Phase::Running | Phase::GameOver { .. } => "",
            }
        };
        if text.as_str() != next {
            **text = next.to_owned();
        }
    }
}

fn sync_end_overlay(
    sim: Res<Sim>,
    end: Res<crate::EndSession>,
    leaderboard: Res<crate::LeaderboardState>,
    mut root: Query<&mut Visibility, With<EndOverlayRoot>>,
    mut texts: Query<(&EndOverlayText, &mut Text2d)>,
) {
    let visible = matches!(
        end.0.mode,
        crate::session::EndMode::NameEntry | crate::session::EndMode::Leaderboard
    );
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible || (!sim.is_changed() && !end.is_changed() && !leaderboard.is_changed()) {
        return;
    }
    let result = sim.0.result().copied().unwrap_or_else(|| {
        game_core::SessionResult::new(
            sim.0.difficulty(),
            sim.0.day_cycles,
            sim.0.cars_passed,
            sim.0.cars_bumped,
        )
    });
    let table = leaderboard_table(&leaderboard.entries, sim.0.difficulty());
    for (marker, mut text) in &mut texts {
        text.0 = match marker.0 {
            EndTextKind::Summary => format!(
                "SCORE {:06}   CYCLES {:02}   PASSED {:03}   BUMPED {:03}",
                result.score, result.day_cycles, result.cars_passed, result.cars_bumped
            ),
            EndTextKind::Prompt => match end.0.mode {
                crate::session::EndMode::NameEntry => {
                    format!(
                        "{}  |  TYPE YOUR 3-LETTER NAME  |  ENTER TO SUBMIT",
                        sim.0.difficulty().label()
                    )
                }
                crate::session::EndMode::Leaderboard => {
                    format!("{}  |  LOCAL TOP 5", sim.0.difficulty().label())
                }
                crate::session::EndMode::DifficultySelect => String::new(),
                crate::session::EndMode::Playing => String::new(),
            },
            EndTextKind::Initial(index) => end.0.initials.cell(index as usize).to_string(),
            EndTextKind::Table => table.clone(),
            EndTextKind::Footer => match end.0.mode {
                crate::session::EndMode::NameEntry => {
                    "A-Z enters letters  |  BACKSPACE edits".to_string()
                }
                crate::session::EndMode::Leaderboard => "R or ENTER starts a new run".to_string(),
                crate::session::EndMode::DifficultySelect => String::new(),
                crate::session::EndMode::Playing => String::new(),
            },
        };
    }
}

fn leaderboard_table(
    entries: &[game_core::LeaderboardEntry],
    difficulty: game_core::Difficulty,
) -> String {
    let mut lines = vec!["#   TAG    SCORE    CYC   PASS   BUMP".to_string()];
    let mut ranked = game_core::leaderboard_for(entries, difficulty);
    for rank in 0..5 {
        if let Some(entry) = ranked.next() {
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

fn road_visibility(daylight: f32, rain: f32) -> RoadVisibilityVisuals {
    let daylight = daylight.clamp(0.0, 1.0);
    let night = 1.0 - daylight;
    let rain = rain.clamp(0.0, 1.0);
    let night_headlights = ((0.55 - daylight) / 0.40).clamp(0.0, 1.0);
    RoadVisibilityVisuals {
        night,
        rain,
        shade_alpha: (night * 0.18 + rain * 0.08).min(0.26),
        fog_alpha: (night * 0.30 + rain * 0.38).min(0.58),
        headlights: night_headlights.max(rain * 0.88),
    }
}

fn fog_band_geometry(index: usize) -> (f32, Vec2, f32) {
    let band_h = 435.0 / VISIBILITY_BANDS as f32;
    let t = index as f32 / (VISIBILITY_BANDS - 1) as f32;
    let y = -300.0 + band_h * (index as f32 + 0.5);
    (y, Vec2::new(960.0, band_h + 1.0), 0.28 + t * 0.72)
}

fn headlight_band_geometry(index: usize) -> (f32, Vec2, f32) {
    let t = index as f32 / (VISIBILITY_BANDS - 1) as f32;
    let band_h = 300.0 / VISIBILITY_BANDS as f32;
    let y = -205.0 + band_h * (index as f32 + 0.5);
    let width = 82.0 + (390.0 - 82.0) * t;
    (y, Vec2::new(width, band_h + 1.5), 0.55 + t * 0.45)
}

fn traffic_visibility(scale: f32, visibility: RoadVisibilityVisuals) -> (f32, f32) {
    let near = ((scale - S_FAR) / (0.42 - S_FAR)).clamp(0.0, 1.0);
    let distance = 1.0 - near;
    let visibility_loss = (visibility.night * 0.78 + visibility.rain * 0.70).min(0.88);
    let body_alpha = (1.0 - visibility_loss * distance).clamp(0.12, 1.0);
    let light_alpha = visibility.headlights * (0.58 + distance * 0.42);
    (body_alpha, light_alpha.clamp(0.0, 1.0))
}

fn traffic_car_color(variant: usize, alpha: f32) -> Color {
    let [r, g, b] = match variant % 4 {
        0 => [0.93, 0.91, 0.85],
        1 => [0.90, 0.16, 0.12],
        2 => [0.10, 0.65, 0.82],
        _ => [0.72, 0.24, 0.78],
    };
    Color::srgba(r, g, b, alpha)
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
    let bright = (0.24 + 0.76 * day) * weather_dim;

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

#[allow(clippy::type_complexity)]
fn sync_road_visibility(
    sim: Res<Sim>,
    mut visuals: ResMut<RoadVisibilityVisuals>,
    mut shade: Query<
        &mut Sprite,
        (
            With<NightShade>,
            Without<RoadFogBand>,
            Without<HeadlightBand>,
        ),
    >,
    mut fog: Query<(&RoadFogBand, &mut Sprite), (Without<NightShade>, Without<HeadlightBand>)>,
    mut headlights: Query<
        (&HeadlightBand, &mut Sprite),
        (Without<NightShade>, Without<RoadFogBand>),
    >,
) {
    let daylight = game_core::environment::daylight(sim.0.environment_time);
    let seasons = game_core::environment::season_weights(sim.0.environment_time);
    let rain = weather_weights(sim.0.weather())[2];
    *visuals = road_visibility(daylight, rain_visibility(rain, seasons[3]));

    if let Ok(mut sprite) = shade.single_mut() {
        sprite.color = Color::srgba(0.005, 0.008, 0.018, visuals.shade_alpha);
    }
    for (band, mut sprite) in &mut fog {
        let (_, _, weight) = fog_band_geometry(band.0);
        sprite.color = Color::srgba(
            0.06 + visuals.rain * 0.08 - visuals.night * 0.02,
            0.08 + visuals.rain * 0.10 - visuals.night * 0.02,
            0.13 + visuals.rain * 0.12 - visuals.night * 0.02,
            visuals.fog_alpha * weight,
        );
    }
    for (band, mut sprite) in &mut headlights {
        let (_, _, weight) = headlight_band_geometry(band.0);
        sprite.color = Color::srgba(1.0, 0.88, 0.58, visuals.headlights * weight * 0.105);
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

fn cooldown_gauge(nitro_t: f32, cooldown: f32, t: &game_core::Tuning) -> (f32, &'static str) {
    if nitro_t > 0.0 {
        ((nitro_t / t.nitro_duration_secs).clamp(0.0, 1.0), "BURN")
    } else if cooldown > 0.0 {
        (
            (1.0 - cooldown / t.nitro_cooldown_secs).clamp(0.0, 1.0),
            "COOL",
        )
    } else {
        (1.0, "READY")
    }
}

fn low_shift_hint_active(gear: Gear, speed: f32, threshold: f32) -> bool {
    gear == Gear::Low && speed >= threshold * 0.92
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sync_instruments(
    time: Res<Time>,
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    effects: Res<crate::VisualEffects>,
    mut visuals: ResMut<InstrumentVisuals>,
    mut needle: Query<
        (&mut Transform, &mut Sprite),
        (With<SpeedNeedle>, Without<SpeedTick>, Without<CooldownFill>),
    >,
    mut digital: Query<
        &mut Text2d,
        (
            With<SpeedDigital>,
            Without<GearPanelText>,
            Without<RecoveryText>,
            Without<CooldownText>,
        ),
    >,
    mut gear: Query<
        (&GearPanelText, &mut TextColor),
        (
            With<GearPanelText>,
            Without<SpeedDigital>,
            Without<RecoveryText>,
            Without<CooldownText>,
        ),
    >,
    mut recovery: Query<
        (&mut Text2d, &mut TextColor),
        (
            With<RecoveryText>,
            Without<GearPanelText>,
            Without<SpeedDigital>,
            Without<CooldownText>,
        ),
    >,
    mut fill: Query<
        (&mut Transform, &mut Sprite),
        (With<CooldownFill>, Without<SpeedNeedle>, Without<SpeedTick>),
    >,
    mut cooldown_text: Query<
        &mut Text2d,
        (
            With<CooldownText>,
            Without<SpeedDigital>,
            Without<GearPanelText>,
            Without<RecoveryText>,
        ),
    >,
    mut ticks: Query<(&SpeedTick, &mut Sprite), (Without<SpeedNeedle>, Without<CooldownFill>)>,
) {
    let kmh = sim.0.speed * 3.6;
    let angle = speedometer_angle(kmh);
    let danger = speedometer_danger(kmh, crate::nitro_effects_active(&sim.0));
    let dir = Vec2::new(angle.cos(), angle.sin());
    if let Ok((mut tf, mut sprite)) = needle.single_mut() {
        tf.translation.x = SPEED_CENTER.x + dir.x * 31.0;
        tf.translation.y = SPEED_CENTER.y + dir.y * 31.0;
        tf.rotation = Quat::from_rotation_z(angle - std::f32::consts::FRAC_PI_2);
        sprite.color = if danger {
            Color::srgb(1.0, 0.24, 0.12)
        } else {
            Color::srgb(1.0, 0.85, 0.28)
        };
    }
    if let Ok(mut text) = digital.single_mut() {
        text.0 = format!("{:03.0} km/h", kmh.min(999.0));
    }
    let hint_charging = matches!(sim.0.phase, Phase::Running)
        && sim.0.recovery() == game_core::Recovery::Normal
        && low_shift_hint_active(sim.0.gear(), sim.0.speed, sim.0.shift_up_speed(&tuning.0));
    if hint_charging {
        visuals.low_shift_hint_secs += time.delta_secs();
    } else {
        visuals.low_shift_hint_secs = 0.0;
    }
    let hint_blink =
        visuals.low_shift_hint_secs >= 1.0 && (time.elapsed_secs() * 3.5).fract() < 0.58;
    for (indicator, mut color) in &mut gear {
        color.0 = if indicator.0 == sim.0.gear() {
            match indicator.0 {
                Gear::Low => Color::srgb(1.0, 0.86, 0.32),
                Gear::High => Color::srgb(0.25, 0.92, 1.0),
            }
        } else if indicator.0 == Gear::High && hint_blink {
            Color::srgb(1.0, 0.66, 0.12)
        } else {
            Color::srgb(0.20, 0.28, 0.32)
        };
    }
    let (recovery_name, base_cue) = match sim.0.recovery() {
        game_core::Recovery::Normal => (
            "normal",
            normal_recovery_cue(sim.0.gear(), sim.0.speed, sim.0.shift_up_speed(&tuning.0)),
        ),
        game_core::Recovery::NeedLow => ("need_low", "SELECT LOW"),
        game_core::Recovery::BuildSpeed => ("build_speed", "BUILD SPEED"),
        game_core::Recovery::ReadyForHigh => ("ready_high", "SHIFT HIGH"),
    };
    let cue = match effects.action_cue {
        crate::ActionCue::TooSoon => "TOO SOON",
        crate::ActionCue::ShiftLow => "LOW ENGAGED",
        crate::ActionCue::None => base_cue,
    };
    if let Ok((mut text, mut color)) = recovery.single_mut() {
        text.0 = cue.to_string();
        color.0 = if effects.action_cue == crate::ActionCue::TooSoon {
            Color::srgb(1.0, 0.25, 0.15)
        } else if cue == "SHIFT HIGH" {
            Color::srgb(1.0, 0.76, 0.20)
        } else {
            Color::srgb(0.65, 0.86, 0.94)
        };
    }
    let (gauge, gauge_label) = cooldown_gauge(sim.0.nitro_t, sim.0.nitro_cooldown, &tuning.0);
    if let Ok((mut tf, mut sprite)) = fill.single_mut() {
        let h = 104.0 * gauge.max(0.02);
        tf.translation.y = -257.0 + h * 0.5;
        sprite.custom_size = Some(Vec2::new(8.0, h));
        sprite.color = if sim.0.nitro_t > 0.0 {
            Color::srgb(1.0, 0.25, 0.08)
        } else if sim.0.nitro_cooldown > 0.0 {
            Color::srgb(1.0, 0.66, 0.12)
        } else {
            Color::srgb(0.12, 0.86, 0.95)
        };
    }
    if let Ok(mut text) = cooldown_text.single_mut() {
        text.0 = gauge_label.to_string();
    }
    for (tick, mut sprite) in &mut ticks {
        if tick.red_zone {
            sprite.color = if danger {
                Color::srgb(1.0, 0.48, 0.12)
            } else {
                Color::srgb(0.9, 0.16, 0.12)
            };
        }
    }
    visuals.speed_kmh = kmh;
    visuals.needle_angle = angle;
    visuals.overrev = sim.0.low_overrev(&tuning.0);
    visuals.cooldown = sim.0.nitro_cooldown;
    visuals.gauge_fill = gauge;
    visuals.recovery = recovery_name;
    visuals.cue = cue;
}

fn sync_difficulty_overlay(
    end: Res<crate::EndSession>,
    mut root: Query<&mut Visibility, With<DifficultyOverlayRoot>>,
    mut cards: Query<(&DifficultyCard, &mut Sprite)>,
    mut texts: Query<(&DifficultyCardText, &mut TextColor)>,
) {
    if !end.is_changed() {
        return;
    }
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = if end.0.mode == crate::session::EndMode::DifficultySelect {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (card, mut sprite) in &mut cards {
        sprite.color = if card.0 == end.0.difficulty_index {
            Color::srgba(0.07, 0.48, 0.64, 0.98)
        } else {
            Color::srgba(0.06, 0.16, 0.27, 0.96)
        };
    }
    for (card, mut color) in &mut texts {
        color.0 = if card.0 == end.0.difficulty_index {
            Color::srgb(1.0, 0.94, 0.55)
        } else {
            Color::srgb(0.78, 0.92, 0.98)
        };
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ViewSet {
    Environment,
    Projection,
    Effects,
    Presentation,
}

pub fn add_view_systems(app: &mut App) {
    app.configure_sets(
        Update,
        (
            ViewSet::Environment,
            ViewSet::Projection,
            ViewSet::Effects,
            ViewSet::Presentation,
        )
            .chain()
            .in_set(crate::AppSet::View),
    );
    app.add_systems(
        Update,
        (sync_environment, sync_road_visibility).in_set(ViewSet::Environment),
    );
    app.add_systems(Update, sync_roads.in_set(ViewSet::Projection));
    app.add_systems(Update, sync_nitro_visual_intensity.in_set(ViewSet::Effects));
    app.add_systems(
        Update,
        (
            ambient::sync,
            sync_palms,
            sync_hazards,
            sync_nitro_pickup,
            sync_nitro_exhaust,
            sync_nitro_speed_lines,
            sync_flash,
            sync_drum_overlays,
            sync_hud,
            sync_instruments,
            sync_end_overlay,
            sync_difficulty_overlay,
        )
            .in_set(ViewSet::Presentation),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_difficulty_finishes_lane_changes_near_the_horizon() {
        let tuning = game_core::Tuning::default();
        for motion in [
            tuning.traffic_easy,
            tuning.traffic_medium,
            tuning.traffic_hard,
        ] {
            let end_scale = D_MIN / motion.change_end_m;
            assert!(scale_to_y(end_scale) >= 90.0);
        }
    }

    #[test]
    fn rolling_terrain_moves_projected_rows_in_the_expected_direction() {
        let scale = 0.25;
        let flat = elevated_row_y(scale, 0.0);
        assert!(elevated_row_y(scale, 0.8) > flat);
        assert!(elevated_row_y(scale, -0.8) < flat);
        assert!((elevated_row_y(scale, 0.8) - flat) < 25.0);
    }

    #[test]
    fn speedometer_maps_true_kmh_across_full_sweep() {
        assert!((speedometer_angle(0.0) - 225.0_f32.to_radians()).abs() < 1e-5);
        assert!((speedometer_angle(60.0) - 90.0_f32.to_radians()).abs() < 1e-5);
        assert!((speedometer_angle(120.0) + 45.0_f32.to_radians()).abs() < 1e-5);
        assert!(speedometer_angle(0.0) > speedometer_angle(120.0));
    }

    #[test]
    fn collision_band_tracks_visible_car_widths_on_the_wider_road() {
        let tuning = game_core::Tuning::default();
        let effective_hit_px = (tuning.car_half_width + tuning.hazard_half_width)
            * (1.0 - tuning.collision_forgiveness)
            * HALF_ROAD_PX;
        let visible_overlap_px = (PLAYER_CAR_WIDTH_PX + TRAFFIC_CAR_WIDTH_PX) * 0.5;
        let ratio = effective_hit_px / visible_overlap_px;

        assert_eq!(HALF_ROAD_PX, 210.0);
        assert!(
            (0.65..=0.75).contains(&ratio),
            "collision band should be forgiving but visually honest: {ratio}"
        );
    }

    #[test]
    fn nitro_forces_speedometer_danger_before_redline() {
        assert!(!speedometer_danger(50.0, false));
        assert!(speedometer_danger(90.0, false));
        assert!(speedometer_danger(50.0, true));
    }

    #[test]
    fn normal_low_cue_requests_high_at_shift_threshold() {
        assert_eq!(
            normal_recovery_cue(Gear::Low, 9.99, 10.0),
            "LB LOW | RB HIGH"
        );
        assert_eq!(normal_recovery_cue(Gear::Low, 10.0, 10.0), "SHIFT HIGH");
        assert_eq!(
            normal_recovery_cue(Gear::High, 24.0, 10.0),
            "LB LOW | RB HIGH"
        );
    }

    #[test]
    fn high_gear_hint_arms_only_near_low_gear_limit() {
        assert!(!low_shift_hint_active(Gear::Low, 8.9, 10.0));
        assert!(low_shift_hint_active(Gear::Low, 9.2, 10.0));
        assert!(!low_shift_hint_active(Gear::High, 12.0, 10.0));
    }

    #[test]
    fn cooldown_gauge_drains_then_refills() {
        let t = game_core::Tuning::default();
        assert_eq!(cooldown_gauge(5.0, 0.0, &t), (1.0, "BURN"));
        assert_eq!(cooldown_gauge(0.0, 3.0, &t), (0.0, "COOL"));
        assert_eq!(cooldown_gauge(0.0, 1.5, &t), (0.5, "COOL"));
        assert_eq!(cooldown_gauge(0.0, 0.0, &t), (1.0, "READY"));
    }

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
    fn night_and_rain_reduce_visibility_without_blackout() {
        let day = road_visibility(1.0, 0.0);
        let night = road_visibility(0.0, 0.0);
        let storm = road_visibility(1.0, 1.0);
        assert_eq!(day.shade_alpha, 0.0);
        assert!(night.shade_alpha > day.shade_alpha);
        assert!(night.fog_alpha > day.fog_alpha);
        assert!(storm.fog_alpha > night.fog_alpha);
        assert_eq!(day.headlights, 0.0);
        assert_eq!(road_visibility(0.70, 0.0).headlights, 0.0);
        assert!(night.headlights > 0.9);
        assert!(storm.headlights > 0.8);
        assert!(night.shade_alpha < 0.3 && storm.fog_alpha < 0.6);
    }

    #[test]
    fn distant_car_bodies_fade_before_their_lights() {
        let conditions = road_visibility(0.0, 0.0);
        let (far_body, far_lights) = traffic_visibility(S_FAR, conditions);
        let (near_body, near_lights) = traffic_visibility(0.42, conditions);
        assert!(far_body < near_body);
        assert!(far_lights > far_body);
        assert!(far_lights > near_lights);
    }

    #[test]
    fn headlight_cone_starts_at_the_car_and_widens_ahead() {
        let (_, near, near_weight) = headlight_band_geometry(0);
        let (_, far, far_weight) = headlight_band_geometry(VISIBILITY_BANDS - 1);
        assert!(near.x < far.x);
        assert!(near_weight < far_weight);
    }

    #[test]
    fn traffic_palette_has_four_stable_variants() {
        let variants = [
            traffic_car_color(0, 1.0),
            traffic_car_color(1, 1.0),
            traffic_car_color(2, 1.0),
            traffic_car_color(3, 1.0),
        ];
        for left in 0..variants.len() {
            for right in left + 1..variants.len() {
                assert_ne!(variants[left], variants[right]);
            }
        }
        assert_eq!(traffic_car_color(0, 1.0), traffic_car_color(4, 1.0));
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
