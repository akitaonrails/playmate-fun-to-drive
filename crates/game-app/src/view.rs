//! Fun to Drive — the 2D drum view, OutRun-inspired (PLAN: user tweak 2).
//!
//! Classic 16-bit racer look: gradient sky with sun and clouds, sea strip at
//! the horizon, sandy shoulder, scrolling grass stripes, red/white rumble
//! edges, lane dashes, palm trees, and composite car sprites — all drawn
//! with flat sprites so it renders identically into the toy's screen.

mod ambient;
mod backdrop;
mod shoulder;

use bevy::prelude::*;
use game_core::{Gear, Phase, WeatherKind, WeatherTransition};

use crate::{SceneryRun, Sim, TuningRes};

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

// Converts integrated world-space curvature into normalized lateral road
// displacement. Curvature is integrated twice below (heading, then position),
// matching a real constant-radius arc under perspective.
pub const CURVE_BEND: f32 = 0.0085;
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
const ROADSIDE_DECOR_PROPS: usize = 24;
const ROADSIDE_DECOR_PARTS_PER_PROP: usize = 3;
const DECOR_BILLBOARD_POOL: usize = 6;
const HAZARD_POOL: usize = 40;
const HAZARD_PARTS: usize = 6;
const NITRO_X_LEFT: f32 = 302.0;
const NITRO_Y: f32 = 216.0;
const JUMP_METER_X: f32 = 138.0;
const RAIN_POOL: usize = 84;
const SPEED_LINE_POOL: usize = 28;
const SPEED_LINE_FAR_SCALE: f32 = 0.62;
const SPEED_LINE_NEAR_SCALE: f32 = 1.08;
const SPEED_LINE_FAR_ROW: usize = 5;
const VISIBILITY_BANDS: usize = 8;
const HEADLIGHT_LAYERS: usize = 2;
const PICKUP_VISUAL_SCALE: f32 = 1.22;
const TIME_TRAVEL_SPEED_MPH: f32 = 88.0;
const TIME_TRAVEL_DURATION_SECS: f32 = 2.25;
const TIME_TRAIL_SEGMENTS: usize = 14;
const FINISH_ROAD_HALF_WIDTH: f32 = 172.0;
const FINISH_ROAD_BANDS: usize = ROWS + APRON_SEGMENTS;

fn pickup_visual_scale(projection_scale: f32) -> f32 {
    projection_scale * PICKUP_VISUAL_SCALE
}

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
struct Apron {
    segment: usize,
    kind: ApronKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApronKind {
    Grass,
    Road,
    LeftRumble,
    RightRumble,
}

const APRON_SEGMENTS: usize = 10;

#[derive(Component)]
struct HazardPart(u8);

#[derive(Component)]
struct DecorBillboard(usize);

#[derive(Component)]
struct NightShade;

#[derive(Component)]
struct RoadFogBand(usize);

#[derive(Component)]
struct HeadlightBand {
    row: usize,
    layer: usize,
}

#[derive(Component)]
struct NitroPickupPart(u8);

#[derive(Component)]
struct JumpPickupPart(u8);

#[derive(Component)]
struct PalmPart;

#[derive(Component)]
struct CarPart;

#[derive(Component)]
struct PlayerRearLight;

#[derive(Component)]
struct PlayerCarRoot;

#[derive(Component)]
struct PlayerCarShadow;

#[derive(Component)]
struct TimeCarRoot;

#[derive(Component)]
struct TimeTrail(usize);

#[derive(Component)]
struct TimeFlash;

#[derive(Component)]
struct FinishCarRoot;

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
struct JumpMeterFill;

#[derive(Component)]
struct JumpMeterHalo;

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
struct PauseOverlayRoot;
#[derive(Component)]
struct DifficultyCard(usize);
#[derive(Component)]
struct DifficultyCardText(usize);
#[derive(Component)]
struct DifficultyLeaderboard;
#[derive(Component)]
struct CrtMenuText;
#[derive(Component)]
struct LargeMenuText;

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

/// Once-per-session state for the 88 mph time-machine easter egg.
#[derive(Resource, Default)]
struct TimeTravelVisuals {
    triggered: bool,
    active_secs: f32,
    was_running: bool,
    last_distance: f32,
}

impl TimeTravelVisuals {
    fn active(&self) -> bool {
        self.triggered && self.active_secs < TIME_TRAVEL_DURATION_SECS
    }

    fn update(
        &mut self,
        running: bool,
        paused: bool,
        distance: f32,
        displayed_mph: f32,
        straightaway_ready: bool,
        dt: f32,
    ) {
        let new_session = running && (!self.was_running || distance + 0.5 < self.last_distance);
        if new_session {
            self.triggered = false;
            self.active_secs = TIME_TRAVEL_DURATION_SECS;
        }
        if !running {
            self.was_running = false;
            return;
        }
        if !paused {
            if !self.triggered && displayed_mph >= TIME_TRAVEL_SPEED_MPH && straightaway_ready {
                self.triggered = true;
                self.active_secs = 0.0;
            } else if self.active() {
                self.active_secs = (self.active_secs + dt.max(0.0)).min(TIME_TRAVEL_DURATION_SECS);
            }
        }
        self.was_running = true;
        self.last_distance = distance;
    }
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
    /// Integrated lateral bend at each row distance (curvature -> heading -> x).
    pub bend: [f32; ROWS],
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
            bend: [0.0; ROWS],
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
        // Interpolate the twice-integrated lateral bend at d via the row table.
        let bend = {
            // rows are ordered near(0)..far(ROWS-1); d decreases with index
            let mut val = self.bend[ROWS - 1];
            for i in 0..ROWS {
                let di = self.distances[i];
                if d <= di {
                    let d0 = if i == 0 { D_MIN } else { self.distances[i - 1] };
                    let t = if di > d0 { (d - d0) / (di - d0) } else { 1.0 };
                    let b0 = if i == 0 { 0.0 } else { self.bend[i - 1] };
                    val = b0 + (self.bend[i] - b0) * t;
                    break;
                }
                val = self.bend[i];
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
        Some((y, s, road_x + CURVE_BEND * bend))
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

#[derive(Debug, Clone, Copy)]
struct CountdownStyle {
    color: Color,
    scale: f32,
}

fn countdown_style(remaining: f32, elapsed: f32, running: bool) -> CountdownStyle {
    const WARNING_SECS: f32 = 10.0;
    if !running || remaining > WARNING_SECS {
        return CountdownStyle {
            color: Color::srgb(0.99, 0.95, 0.75),
            scale: 1.0,
        };
    }

    let urgency = 1.0 - (remaining / WARNING_SECS).clamp(0.0, 1.0);
    let frequency = 3.0 + urgency * 4.0;
    let pulse = 0.5 + 0.5 * (elapsed * std::f32::consts::TAU * frequency).sin();
    CountdownStyle {
        color: Color::srgba(
            1.0,
            0.18 + 0.54 * (1.0 - urgency) + pulse * 0.18,
            0.08 + pulse * 0.10,
            0.24 + pulse * 0.76,
        ),
        scale: 1.0 + pulse * (0.10 + urgency * 0.12),
    }
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
        "A/LB LOW | X/RB HIGH"
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
        Text2d::new("A/LB LOW | X/RB HIGH"),
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
        Text2d::new("THRUST"),
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
                    Vec2::new(920.0, 510.0),
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
                Transform::from_xyz(-145.0, 205.0, 0.3),
            ));
            for (index, x, title, duration) in [
                (0, -355.0, "EASY", "2:00"),
                (1, -155.0, "MEDIUM", "6:00"),
                (2, 45.0, "HARD", "10:00"),
            ] {
                parent.spawn((
                    Sprite::from_color(
                        Color::srgba(0.06, 0.16, 0.27, 0.96),
                        Vec2::new(180.0, 150.0),
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
                Text2d::new("LOCAL TOP 5 — MEDIUM\n#  TAG  SCORE   CYC PASS BUMP\n1  ---  ------   --  ---  ---\n2  ---  ------   --  ---  ---\n3  ---  ------   --  ---  ---\n4  ---  ------   --  ---  ---\n5  ---  ------   --  ---  ---"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.90, 1.0)),
                Transform::from_xyz(285.0, 22.0, 0.3),
                DifficultyLeaderboard,
            ));
            parent.spawn((
                Text2d::new("CRT DISPLAY: ON   [ C / X ]"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 1.0, 0.74)),
                Transform::from_xyz(-295.0, -112.0, 0.3),
                CrtMenuText,
            ));
            parent.spawn((
                Text2d::new("LARGE SCREEN: OFF   [ V / L3+R3 ]"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.62, 0.38)),
                Transform::from_xyz(15.0, -112.0, 0.3),
                LargeMenuText,
            ));
            parent.spawn((
                Text2d::new("<   ARROWS / LEFT STICK   >"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.90, 1.0)),
                Transform::from_xyz(-145.0, -158.0, 0.3),
            ));
            parent.spawn((
                Text2d::new("ENTER / A / START TO BEGIN"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.88, 0.45)),
                Transform::from_xyz(-145.0, -205.0, 0.3),
            ));
        });
}

fn spawn_pause_overlay(commands: &mut Commands) {
    commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, 25.0),
            Visibility::Hidden,
            PauseOverlayRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite::from_color(
                    Color::srgba(0.005, 0.01, 0.025, 0.66),
                    Vec2::new(960.0, 600.0),
                ),
                Transform::default(),
            ));
            parent.spawn((
                Sprite::from_color(
                    Color::srgba(0.025, 0.08, 0.14, 0.96),
                    Vec2::new(620.0, 250.0),
                ),
                Transform::from_xyz(0.0, 0.0, 0.1),
            ));
            parent.spawn((
                Sprite::from_color(Color::srgba(0.20, 0.90, 1.0, 0.92), Vec2::new(624.0, 4.0)),
                Transform::from_xyz(0.0, 123.0, 0.2),
            ));
            parent.spawn((
                Text2d::new("PAUSED"),
                TextFont {
                    font_size: FontSize::Px(48.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.96, 1.0)),
                Transform::from_xyz(0.0, 62.0, 0.3),
            ));
            parent.spawn((
                Text2d::new("START / P   RESUME\nSELECT / ESC   RESTART"),
                TextFont {
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.90, 0.56)),
                TextLayout::justify(Justify::Center),
                Transform::from_xyz(0.0, -39.0, 0.3),
            ));
        });
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(palette());
    commands.init_resource::<Proj>();
    commands.init_resource::<WeatherVisuals>();
    commands.init_resource::<RoadVisibilityVisuals>();
    commands.init_resource::<NitroVisuals>();
    commands.init_resource::<TimeTravelVisuals>();
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
    backdrop::spawn(&mut commands);
    shoulder::spawn(&mut commands);
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
    }
    for row in 0..ROWS {
        for layer in 0..HEADLIGHT_LAYERS {
            commands.spawn((
                Sprite::from_color(Color::srgba(1.0, 0.88, 0.58, 0.0), Vec2::ONE),
                Transform::from_xyz(0.0, CAR_Y, 0.19 + layer as f32 * 0.01),
                HeadlightBand { row, layer },
            ));
        }
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

    // ---- foreground apron (below the nearest row) ----
    let apron_h = CAR_Y - (-322.0);
    let segment_h = apron_h / APRON_SEGMENTS as f32;
    for segment in 0..APRON_SEGMENTS {
        for (kind, z) in [
            (ApronKind::Grass, -4.4),
            (ApronKind::Road, -4.0),
            (ApronKind::LeftRumble, -3.9),
            (ApronKind::RightRumble, -3.9),
        ] {
            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::new(1.0, segment_h + 1.5)),
                    ..default()
                },
                Transform::from_xyz(0.0, -322.0 + segment_h * (segment as f32 + 0.5), z),
                Apron { segment, kind },
            ));
        }
    }

    // ---- palms (pooled, ride the loop) ----
    for _ in 0..ROADSIDE_DECOR_PROPS {
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

    // Non-collidable brand cameos live farther out in the grass. City scenes
    // draw from the full pool; nature scenes retain only an occasional sign.
    for index in 0..DECOR_BILLBOARD_POOL {
        let (label, face, ink) = match index % 3 {
            0 => (
                "TESLA",
                Color::srgb(0.92, 0.93, 0.95),
                Color::srgb(0.82, 0.06, 0.10),
            ),
            1 => (
                "SEGA",
                Color::srgb(0.06, 0.30, 0.72),
                Color::srgb(0.94, 0.98, 1.0),
            ),
            _ => (
                "namco",
                Color::srgb(0.98, 0.72, 0.08),
                Color::srgb(0.82, 0.08, 0.06),
            ),
        };
        commands
            .spawn((
                Transform::from_xyz(0.0, CAR_Y, -2.72),
                Visibility::Hidden,
                DecorBillboard(index),
            ))
            .with_children(|sign| {
                for x in [-22.0, 22.0] {
                    sign.spawn((
                        Sprite::from_color(Color::srgb(0.24, 0.20, 0.18), Vec2::new(5.0, 38.0)),
                        Transform::from_xyz(x, 18.0, 0.0),
                    ));
                }
                sign.spawn((
                    Sprite::from_color(face, Vec2::new(82.0, 36.0)),
                    Transform::from_xyz(0.0, 48.0, 0.05),
                ));
                sign.spawn((
                    Text2d::new(label),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(ink),
                    Transform::from_xyz(0.0, 48.0, 0.10),
                ));
            });
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

    // ---- rare Jump Kick pickup (one deterministic candidate per cycle max) ----
    for i in 0..5u8 {
        commands.spawn((
            Sprite::from_color(Color::srgb(1.0, 0.22, 0.72), Vec2::new(38.0, 48.0)),
            Transform::from_xyz(0.0, CAR_Y, -2.25),
            Visibility::Hidden,
            JumpPickupPart(i),
        ));
    }

    // ---- the fixed player car "decal" (composite) ----
    // A tiny rear/top hybrid of the new Roadster: silver wedge, black glass
    // canopy, full-width red light blade, four outlets, huge dark wheels, and
    // a raised wing. Those high-contrast cues survive the toy screen's low
    // resolution and the optional CRT pass better than fine body detailing.
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.28), Vec2::new(92.0, 12.0)),
        Transform::from_xyz(0.0, CAR_Y - 18.0, 0.30),
        PlayerCarShadow,
    ));
    commands
        .spawn((
            Transform::from_xyz(0.0, CAR_Y, 0.5),
            Visibility::Visible,
            PlayerCarRoot,
        ))
        .with_children(|car| {
            // Raised wing behind the rear deck.
            car.spawn((
                Sprite::from_color(Color::srgb(0.055, 0.065, 0.080), Vec2::new(96.0, 4.0)),
                Transform::from_xyz(0.0, -21.0, 0.02),
                CarPart,
            ));
            for x in [-32.0f32, 32.0] {
                car.spawn((
                    Sprite::from_color(Color::srgb(0.08, 0.09, 0.11), Vec2::new(4.0, 8.0)),
                    Transform::from_xyz(x, -17.0, 0.01),
                    CarPart,
                ));
            }
            // Oversized wheels frame the low wedge.
            for x in [-43.0f32, 43.0] {
                car.spawn((
                    Sprite::from_color(Color::srgb(0.025, 0.030, 0.040), Vec2::new(17.0, 24.0)),
                    Transform::from_xyz(x, -2.0, -0.04),
                    CarPart,
                ));
                car.spawn((
                    Sprite::from_color(Color::srgb(0.28, 0.31, 0.34), Vec2::new(7.0, 13.0)),
                    Transform::from_xyz(x, -2.0, -0.03),
                    CarPart,
                ));
            }
            // Black diffuser and silver body layers create the stepped rear.
            car.spawn((
                Sprite::from_color(Color::srgb(0.035, 0.045, 0.060), Vec2::new(84.0, 10.0)),
                Transform::from_xyz(0.0, -14.0, 0.00),
                CarPart,
            ));
            car.spawn((
                Sprite::from_color(
                    Color::srgb(0.78, 0.82, 0.84),
                    Vec2::new(PLAYER_CAR_WIDTH_PX - 8.0, 23.0),
                ),
                Transform::from_xyz(0.0, -2.5, 0.02),
                CarPart,
            ));
            car.spawn((
                Sprite::from_color(Color::srgb(0.91, 0.93, 0.92), Vec2::new(73.0, 14.0)),
                Transform::from_xyz(0.0, 10.0, 0.03),
                CarPart,
            ));
            // Panoramic black canopy and the signature razor rear light bar.
            car.spawn((
                Sprite::from_color(Color::srgb(0.035, 0.060, 0.085), Vec2::new(43.0, 14.0)),
                Transform::from_xyz(0.0, 8.0, 0.05),
                CarPart,
            ));
            car.spawn((
                Sprite::from_color(Color::srgb(1.0, 0.035, 0.025), Vec2::new(78.0, 3.0)),
                Transform::from_xyz(0.0, -9.0, 0.07),
                CarPart,
                PlayerRearLight,
            ));
            // Four dark round-ish jet/exhaust openings are readable as four
            // square pixels at this scale and echo the reference rear fascia.
            for x in [-24.0f32, -8.0, 8.0, 24.0] {
                car.spawn((
                    Sprite::from_color(Color::srgb(0.01, 0.015, 0.020), Vec2::new(7.0, 5.0)),
                    Transform::from_xyz(x, -14.0, 0.08),
                    CarPart,
                ));
            }
        });

    // Dedicated overhead Roadster used only after time expires. Its low,
    // wide proportions match the hero decal, but the lamp placement makes the
    // viewpoint unambiguous: paired white headlights lead at the top and the
    // continuous red diffuser blade trails at the bottom.
    commands
        .spawn((
            Transform::from_xyz(0.0, CAR_Y, 0.62),
            Visibility::Hidden,
            FinishCarRoot,
        ))
        .with_children(|car| {
            for x in [-38.0f32, 38.0] {
                car.spawn((
                    Sprite::from_color(Color::srgb(0.02, 0.025, 0.035), Vec2::new(14.0, 31.0)),
                    Transform::from_xyz(x, -1.0, -0.03),
                ));
            }
            car.spawn((
                Sprite::from_color(Color::srgb(0.68, 0.72, 0.74), Vec2::new(78.0, 55.0)),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
            car.spawn((
                Sprite::from_color(Color::srgb(0.88, 0.91, 0.91), Vec2::new(68.0, 43.0)),
                Transform::from_xyz(0.0, 3.0, 0.02),
            ));
            car.spawn((
                Sprite::from_color(Color::srgb(0.025, 0.05, 0.075), Vec2::new(43.0, 27.0)),
                Transform::from_xyz(0.0, 1.0, 0.04),
            ));
            car.spawn((
                Sprite::from_color(Color::srgb(0.48, 0.54, 0.58), Vec2::new(33.0, 12.0)),
                Transform::from_xyz(0.0, -12.0, 0.05),
            ));
            for x in [-22.0f32, 22.0] {
                car.spawn((
                    Sprite::from_color(Color::linear_rgb(1.55, 1.45, 1.05), Vec2::new(19.0, 5.0)),
                    Transform::from_xyz(x, 24.0, 0.07),
                ));
            }
            car.spawn((
                Sprite::from_color(Color::linear_rgb(1.75, 0.025, 0.015), Vec2::new(67.0, 4.0)),
                Transform::from_xyz(0.0, -24.0, 0.08),
            ));
        });

    // Stainless-steel time-machine homage. It is pooled at startup and only
    // becomes visible for the once-per-session 88 mph straightaway pass.
    commands
        .spawn((
            Transform::from_xyz(0.0, -330.0, 0.76),
            Visibility::Hidden,
            TimeCarRoot,
        ))
        .with_children(|car| {
            for x in [-31.0f32, 31.0] {
                car.spawn((
                    Sprite::from_color(Color::srgb(0.025, 0.03, 0.04), Vec2::new(13.0, 24.0)),
                    Transform::from_xyz(x, -2.0, -0.02),
                ));
            }
            car.spawn((
                Sprite::from_color(Color::srgb(0.48, 0.52, 0.55), Vec2::new(72.0, 28.0)),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
            car.spawn((
                Sprite::from_color(Color::srgb(0.72, 0.76, 0.78), Vec2::new(62.0, 17.0)),
                Transform::from_xyz(0.0, 10.0, 0.02),
            ));
            car.spawn((
                Sprite::from_color(Color::srgb(0.035, 0.055, 0.075), Vec2::new(46.0, 12.0)),
                Transform::from_xyz(0.0, 10.0, 0.04),
            ));
            for y in [3.0f32, 6.0, 9.0, 12.0] {
                car.spawn((
                    Sprite::from_color(Color::srgb(0.10, 0.12, 0.14), Vec2::new(50.0, 2.0)),
                    Transform::from_xyz(0.0, y, 0.05),
                ));
            }
            for x in [-22.0f32, 22.0] {
                car.spawn((
                    Sprite::from_color(Color::srgb(0.98, 0.10, 0.05), Vec2::new(18.0, 5.0)),
                    Transform::from_xyz(x, -8.0, 0.06),
                ));
            }
            car.spawn((
                Sprite::from_color(Color::linear_rgb(0.15, 1.30, 2.10), Vec2::new(13.0, 13.0)),
                Transform::from_xyz(0.0, -1.0, 0.07)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            ));
            car.spawn((
                Sprite::from_color(Color::srgb(0.04, 0.05, 0.06), Vec2::new(76.0, 5.0)),
                Transform::from_xyz(0.0, -15.0, 0.03),
            ));
        });
    for index in 0..TIME_TRAIL_SEGMENTS {
        commands.spawn((
            Sprite::from_color(Color::srgba(0.18, 0.78, 1.0, 0.0), Vec2::ONE),
            Transform::from_xyz(0.0, -330.0, 0.70),
            Visibility::Hidden,
            TimeTrail(index),
        ));
    }
    commands.spawn((
        Sprite::from_color(Color::srgba(0.72, 0.95, 1.0, 0.0), Vec2::ONE),
        Transform::from_xyz(0.0, HORIZON_Y - 28.0, 0.84),
        Visibility::Hidden,
        TimeFlash,
    ));

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

    // Three-cell cold-air meter, rendered into the toy screen. It must be filled
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
        Text2d::new("COLD-AIR Y"),
        TextFont {
            font_size: FontSize::Px(12.0),
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

    // One-cell Jump Kick inventory beside the nitro meter.
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 0.22, 0.72, 0.0), Vec2::new(132.0, 42.0)),
        Transform::from_xyz(JUMP_METER_X, NITRO_Y, 0.98),
        JumpMeterHalo,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgba(0.02, 0.02, 0.04, 0.82), Vec2::new(124.0, 34.0)),
        Transform::from_xyz(JUMP_METER_X, NITRO_Y, 1.0),
        HudPanel,
    ));
    commands.spawn((
        Text2d::new("JUMP B"),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.70, 0.92)),
        Transform::from_xyz(JUMP_METER_X - 32.0, NITRO_Y, 1.1),
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.11, 0.12, 0.19), Vec2::new(28.0, 14.0)),
        Transform::from_xyz(JUMP_METER_X + 36.0, NITRO_Y, 1.1),
        JumpMeterFill,
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
    spawn_pause_overlay(&mut commands);

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

fn advance_curve_projection(
    heading: f32,
    lateral: f32,
    curvature: f32,
    distance: f32,
) -> (f32, f32) {
    let next_heading = heading + curvature * distance;
    // Trapezoidal integration keeps transitions smooth even across the large
    // far-row gaps in the perspective table.
    let next_lateral = lateral + (heading + next_heading) * 0.5 * distance;
    (next_heading, next_lateral)
}

fn apron_road_half_width(vertical_t: f32) -> f32 {
    HALF_ROAD_PX + (1.0 - vertical_t.clamp(0.0, 1.0)) * 58.0
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FinishRoadBand {
    center: f32,
    half_width: f32,
    y0: f32,
    y1: f32,
}

fn finish_road_band(
    current_center: f32,
    current_half_width: f32,
    current_y0: f32,
    current_y1: f32,
    band: usize,
    lift: f32,
) -> FinishRoadBand {
    let lift = lift.clamp(0.0, 1.0);
    let screen_bottom = -VIEW_TOP;
    let screen_height = VIEW_TOP * 2.0;
    let target_y0 = screen_bottom + screen_height * band as f32 / FINISH_ROAD_BANDS as f32;
    let target_y1 = screen_bottom + screen_height * (band + 1) as f32 / FINISH_ROAD_BANDS as f32;
    FinishRoadBand {
        center: current_center * (1.0 - lift),
        half_width: current_half_width + (FINISH_ROAD_HALF_WIDTH - current_half_width) * lift,
        y0: current_y0 + (target_y0 - current_y0) * lift,
        y1: current_y1 + (target_y1 - current_y1) * lift,
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_roads(
    sim: Res<Sim>,
    colors: Res<Palette>,
    finish: Res<crate::FinishSequence>,
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
        (&Apron, &mut Transform, &mut Sprite),
        (
            With<Apron>,
            Without<RoadRow>,
            Without<EdgeRumble>,
            Without<CenterDash>,
            Without<GrassRow>,
        ),
    >,
) {
    let lift = finish_pose(finish.progress()).road_lift;
    // Walk near -> far, integrating curvature into heading and heading into
    // lateral position. One integration makes a constant-radius corner look
    // like a shifted straight after perspective scaling; the second produces
    // the characteristic expanding arcade-racer sweep toward the horizon.
    let mut heading = 0.0f32;
    let mut lateral = 0.0f32;
    let player_elevation = sim.0.road.elevation_at(sim.0.distance);
    for i in 0..ROWS {
        let s = proj.scales[i];
        let d = proj.distances[i];
        let previous_d = if i == 0 { D_MIN } else { proj.distances[i - 1] };
        let delta = d - previous_d;
        let sample_d = previous_d + delta * 0.5;
        let pos = sim.0.distance + d;
        (heading, lateral) = advance_curve_projection(
            heading,
            lateral,
            sim.0.road.curve_at(sim.0.distance + sample_d),
            delta,
        );
        proj.rows_x[i] = HALF_ROAD_PX * s * (sim.0.road_x + CURVE_BEND * lateral);
        let relative_elevation = sim.0.road.elevation_at(pos) - player_elevation;
        proj.rows_y[i] = elevated_row_y(s, relative_elevation);
        proj.bend[i] = lateral;
    }

    for (i, (mut tf, mut sp)) in rows.iter_mut().enumerate() {
        let y0 = proj.rows_y[i];
        let y1 = if i + 1 < ROWS {
            proj.rows_y[i + 1]
        } else {
            HORIZON_Y + 2.0
        };
        let band = finish_road_band(
            proj.rows_x[i],
            HALF_ROAD_PX * proj.scales[i],
            y0,
            y1,
            APRON_SEGMENTS + i,
            lift,
        );
        let h = (band.y1 - band.y0).max(1.5);
        tf.translation.x = band.center;
        tf.translation.y = band.y0 + h * 0.5;
        let size = sp.custom_size.get_or_insert(Vec2::ONE);
        size.x = band.half_width * 2.0;
        size.y = h + 1.0;
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
        for (apron, mut tf, mut sp) in &mut apron {
            let apron_bottom = -322.0;
            let total_h = (proj.rows_y[0] - apron_bottom).max(1.0);
            let segment_h = total_h / APRON_SEGMENTS as f32;
            let vertical_t = (apron.segment as f32 + 0.5) / APRON_SEGMENTS as f32;
            // t=0 is the bottom edge closest to the viewer; taper smoothly to
            // the exact row-0 width so the road never pinches at the seam.
            let current_y0 = apron_bottom + segment_h * apron.segment as f32;
            let current_y1 = current_y0 + segment_h;
            let band = finish_road_band(
                x0,
                apron_road_half_width(vertical_t),
                current_y0,
                current_y1,
                apron.segment,
                lift,
            );
            let road_half = band.half_width;
            let rumble_width = (16.0 + (1.0 - vertical_t) * 7.0) * (1.0 - lift) + 10.0 * lift;
            tf.translation.y = (band.y0 + band.y1) * 0.5;
            let size = sp.custom_size.get_or_insert(Vec2::ONE);
            size.y = (band.y1 - band.y0).max(1.0) + 1.5;
            match apron.kind {
                ApronKind::Grass => {
                    tf.translation.x = 0.0;
                    size.x = 960.0;
                    sp.color = if on0 { colors.grass_a } else { colors.grass_b };
                }
                ApronKind::Road => {
                    tf.translation.x = band.center;
                    size.x = road_half * 2.0;
                    sp.color = if on0 {
                        colors.asphalt_a
                    } else {
                        colors.asphalt_b
                    };
                }
                ApronKind::LeftRumble | ApronKind::RightRumble => {
                    let side = if apron.kind == ApronKind::LeftRumble {
                        -1.0
                    } else {
                        1.0
                    };
                    tf.translation.x = band.center + side * (road_half - rumble_width * 0.5);
                    size.x = rumble_width;
                    sp.color = if on0 {
                        colors.rumble_red
                    } else {
                        colors.rumble_white
                    };
                }
            }
        }
    }
    for (idx, (mut tf, mut sp)) in rumble.iter_mut().enumerate() {
        let row = idx / 2;
        let side: f32 = if idx % 2 == 0 { -1.0 } else { 1.0 };
        let s = proj.scales[row];
        let y0 = proj.rows_y[row];
        let y1 = if row + 1 < ROWS {
            proj.rows_y[row + 1]
        } else {
            HORIZON_Y + 2.0
        };
        let band = finish_road_band(
            proj.rows_x[row],
            HALF_ROAD_PX * s,
            y0,
            y1,
            APRON_SEGMENTS + row,
            lift,
        );
        let h = (band.y1 - band.y0).max(1.5);
        let rumble_width = (16.0 * s).max(1.5) * (1.0 - lift) + 10.0 * lift;
        tf.translation.x = band.center + side * (band.half_width - rumble_width * 0.5);
        tf.translation.y = band.y0 + h * 0.5;
        let size = sp.custom_size.get_or_insert(Vec2::ONE);
        size.x = rumble_width;
        size.y = h + 1.0;
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
        let y0 = proj.rows_y[row];
        let y1 = if row + 1 < ROWS {
            proj.rows_y[row + 1]
        } else {
            HORIZON_Y + 2.0
        };
        let band = finish_road_band(
            proj.rows_x[row],
            HALF_ROAD_PX * s,
            y0,
            y1,
            APRON_SEGMENTS + row,
            lift,
        );
        let h = (band.y1 - band.y0).max(1.5);
        tf.translation.x = band.center + lane * band.half_width;
        tf.translation.y = band.y0 + h * 0.5;
        let size = sp.custom_size.get_or_insert(Vec2::ONE);
        size.x = (7.0 * s).max(1.0) * (1.0 - lift) + 5.0 * lift;
        size.y = h * 0.6;
        let pos = sim.0.distance + proj.distances[row];
        let on = ((pos / 3.5).floor() as i64).rem_euclid(2) == 0;
        sp.color = if on { colors.dash } else { colors.asphalt_a };
    }
}

#[allow(clippy::type_complexity)]
fn sync_hazards(
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    scenery: Res<SceneryRun>,
    proj: Res<Proj>,
    road_visibility: Res<RoadVisibilityVisuals>,
    finish: Res<crate::FinishSequence>,
    mut pool: Query<(&HazardPart, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    if finish.active {
        for (_, _, _, mut visibility) in &mut pool {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let lp = sim.0.loop_pos();
    let helped = sim.0.rare_traffic_help(&tuning.0);
    let roadside_theme = backdrop::roadside_theme(*scenery, sim.0.environment_time);
    let prop_colors = roadside_prop_palette(
        game_core::environment::season_weights(sim.0.environment_time),
        game_core::environment::daylight(sim.0.environment_time),
        roadside_theme,
    );
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
                (game_core::HazardKind::Tree, 0) => match roadside_theme {
                    backdrop::RoadsideTheme::City => (
                        Color::srgb(0.12, 0.15, 0.22),
                        Vec2::new(7.0, 58.0),
                        29.0,
                        -2.55,
                    ),
                    backdrop::RoadsideTheme::Winter => (
                        Color::srgb(0.28, 0.22, 0.20),
                        Vec2::new(9.0, 50.0),
                        25.0,
                        -2.55,
                    ),
                    _ => (
                        Color::srgb(0.34, 0.18, 0.07),
                        Vec2::new(10.0, 44.0),
                        22.0,
                        -2.55,
                    ),
                },
                (game_core::HazardKind::Tree, 1) => match roadside_theme {
                    backdrop::RoadsideTheme::City => (
                        Color::srgb(0.16, 0.19, 0.27),
                        Vec2::new(30.0, 5.0),
                        55.0,
                        -2.45,
                    ),
                    backdrop::RoadsideTheme::Winter => {
                        (prop_colors.tree_dark, Vec2::new(42.0, 5.0), 38.0, -2.45)
                    }
                    _ => (prop_colors.tree_dark, Vec2::new(49.0, 34.0), 48.0, -2.45),
                },
                (game_core::HazardKind::Tree, 2) => match roadside_theme {
                    backdrop::RoadsideTheme::City => {
                        (prop_colors.tree_light, Vec2::new(17.0, 11.0), 51.0, -2.38)
                    }
                    backdrop::RoadsideTheme::Winter => {
                        (prop_colors.tree_light, Vec2::new(29.0, 4.0), 48.0, -2.42)
                    }
                    _ => (prop_colors.tree_light, Vec2::new(37.0, 28.0), 65.0, -2.42),
                },
                (game_core::HazardKind::Tree, _) => (
                    Color::srgba(0.02, 0.08, 0.03, 0.55),
                    Vec2::new(30.0, 6.0),
                    0.0,
                    -2.62,
                ),
                (game_core::HazardKind::Billboard, 0) => (
                    prop_colors.billboard_frame,
                    Vec2::new(44.0, 27.0),
                    30.0,
                    -2.46,
                ),
                (game_core::HazardKind::Billboard, 1) => (
                    prop_colors.billboard_face,
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
        if helped
            .iter()
            .flatten()
            .any(|traffic| traffic.hazard_index == hazard_index)
        {
            continue;
        }
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
    finish: Res<crate::FinishSequence>,
    mut parts: Query<(
        &NitroPickupPart,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    let mut parts = parts.iter_mut();
    if finish.active {
        for (_, _, _, mut visibility) in parts {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let Some((y, s, lat)) = proj.project(sim.0.nitro_pickup_ahead(), sim.0.road_x) else {
        for (_, _, _, mut vis) in parts {
            *vis = Visibility::Hidden;
        }
        return;
    };
    let x = (lat + sim.0.next_nitro_lane) * HALF_ROAD_PX * s;
    let item_scale = pickup_visual_scale(s);
    let pulse = 0.5 + 0.5 * (time.elapsed_secs() * 11.3).sin();
    for (part, mut tf, mut sprite, mut vis) in parts.by_ref() {
        let (color, size, dy, z) = match part.0 {
            0 => (
                Color::linear_rgba(0.06, 1.65, 2.50, 0.22 + pulse * 0.28),
                Vec2::new(
                    ((74.0 + pulse * 10.0) * item_scale).max(12.0),
                    ((84.0 + pulse * 12.0) * item_scale).max(14.0),
                ),
                0.0,
                -2.35,
            ),
            1 => (
                Color::linear_rgb(0.04, 1.30, 2.15),
                Vec2::new((32.0 * item_scale).max(5.0), (45.0 * item_scale).max(7.0)),
                0.0,
                -2.3,
            ),
            2 => (
                Color::linear_rgb(1.7, 2.1, 2.3),
                Vec2::new((17.0 * item_scale).max(5.0), (8.0 * item_scale).max(2.0)),
                23.0 * item_scale,
                -2.28,
            ),
            _ => (
                Color::srgb(1.0, 0.91, 0.42),
                Vec2::new((10.0 * item_scale).max(3.0), (24.0 * item_scale).max(7.0)),
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

#[allow(clippy::type_complexity)]
fn sync_jump_pickup(
    sim: Res<Sim>,
    proj: Res<Proj>,
    time: Res<Time>,
    finish: Res<crate::FinishSequence>,
    mut parts: Query<(
        &JumpPickupPart,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    let mut parts = parts.iter_mut();
    if finish.active {
        for (_, _, _, mut visibility) in parts {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let Some(ahead) = sim.0.jump_pickup_ahead() else {
        for (_, _, _, mut visibility) in parts {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let Some((y, scale, lateral)) = proj.project(ahead, sim.0.road_x) else {
        for (_, _, _, mut visibility) in parts {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let x = (lateral + sim.0.next_jump_lane) * HALF_ROAD_PX * scale;
    let item_scale = pickup_visual_scale(scale);
    let pulse = 0.5 + 0.5 * (time.elapsed_secs() * 8.7).sin();
    for (part, mut transform, mut sprite, mut visibility) in parts.by_ref() {
        let (color, size, dx, dy, z) = match part.0 {
            0 => (
                Color::linear_rgba(2.2, 0.05, 1.35, 0.34 + pulse * 0.38),
                Vec2::new(
                    ((90.0 + pulse * 18.0) * item_scale).max(12.0),
                    ((90.0 + pulse * 18.0) * item_scale).max(12.0),
                ),
                0.0,
                0.0,
                -2.30,
            ),
            1 => (
                Color::linear_rgb(1.55, 0.10, 0.82),
                Vec2::new((37.0 * item_scale).max(4.0), (42.0 * item_scale).max(5.0)),
                0.0,
                0.0,
                -2.25,
            ),
            2 => (
                Color::linear_rgb(2.0, 1.55, 2.0),
                Vec2::new(
                    ((30.0 + pulse * 12.0) * item_scale).max(3.0),
                    (6.0 * item_scale).max(1.0),
                ),
                0.0,
                0.0,
                -2.22,
            ),
            3 => (
                Color::srgb(0.28, 1.0, 0.78),
                Vec2::new((16.0 * item_scale).max(2.0), (8.0 * item_scale).max(1.0)),
                -19.0 * item_scale,
                5.0 * item_scale,
                -2.21,
            ),
            _ => (
                Color::srgb(0.28, 1.0, 0.78),
                Vec2::new((16.0 * item_scale).max(2.0), (8.0 * item_scale).max(1.0)),
                19.0 * item_scale,
                5.0 * item_scale,
                -2.21,
            ),
        };
        transform.translation = Vec3::new(x + dx, y + dy, z);
        transform.rotation = if part.0 == 2 {
            Quat::from_rotation_z(time.elapsed_secs() * 2.4)
        } else {
            Quat::IDENTITY
        };
        sprite.color = color;
        sprite.custom_size = Some(size);
        *visibility = Visibility::Visible;
    }
    for (_, _, _, mut visibility) in parts {
        *visibility = Visibility::Hidden;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct JumpPose {
    height: f32,
    sprite_spin: f32,
    scale: Vec3,
    shadow_scale: f32,
    shadow_alpha: f32,
}

fn jump_pose(progress: f32) -> JumpPose {
    if !(0.0..1.0).contains(&progress) {
        return JumpPose {
            height: 0.0,
            sprite_spin: 0.0,
            scale: Vec3::ONE,
            shadow_scale: 1.0,
            shadow_alpha: 0.28,
        };
    }
    let p = progress.clamp(0.0, 1.0);
    let height_unit = (std::f32::consts::PI * p).sin().max(0.0);
    let eased_spin = p * p * (3.0 - 2.0 * p);
    let launch_squash = ((0.09 - p).max(0.0) / 0.09).powi(2);
    let landing_squash = ((p - 0.90).max(0.0) / 0.10).powi(2);
    JumpPose {
        height: 104.0 * height_unit,
        sprite_spin: std::f32::consts::TAU * eased_spin,
        scale: Vec3::new(
            1.0 + 0.10 * (launch_squash + landing_squash),
            1.0 - 0.12 * (launch_squash + landing_squash),
            1.0,
        ),
        shadow_scale: 1.0 - 0.46 * height_unit,
        shadow_alpha: 0.28 * (1.0 - 0.64 * height_unit),
    }
}

fn cold_air_hop_pose(progress: f32) -> JumpPose {
    if !(0.0..1.0).contains(&progress) {
        return jump_pose(0.0);
    }
    let height_unit = (std::f32::consts::PI * progress).sin().max(0.0);
    JumpPose {
        height: 42.0 * height_unit,
        sprite_spin: 0.0,
        scale: Vec3::new(1.0 - 0.025 * height_unit, 1.0 + 0.045 * height_unit, 1.0),
        shadow_scale: 1.0 - 0.28 * height_unit,
        shadow_alpha: 0.28 * (1.0 - 0.48 * height_unit),
    }
}

fn launch_drop_pose(progress: f32) -> (f32, f32) {
    let descent = smoothstep((progress / 0.82).clamp(0.0, 1.0));
    let height = (VIEW_TOP + 82.0 - CAR_Y) * (1.0 - descent);
    let jet = 1.0 - smoothstep(((progress - 0.68) / 0.28).clamp(0.0, 1.0));
    (height, jet)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FinishPose {
    road_lift: f32,
    rear_scale_y: f32,
    top_scale_y: f32,
    climb: f32,
    car_y: f32,
    jet: f32,
}

fn finish_pose(progress: f32) -> FinishPose {
    let progress = progress.clamp(0.0, 1.0);
    let road_lift = smoothstep((progress / 0.50).clamp(0.0, 1.0));
    let morph = smoothstep(((progress - 0.38) / 0.22).clamp(0.0, 1.0));
    let climb = smoothstep(((progress - 0.56) / 0.40).clamp(0.0, 1.0));
    let rear_scale_y = (1.0 - morph * 2.0).clamp(0.0, 1.0);
    let top_scale_y = ((morph - 0.5) * 2.0).clamp(0.0, 1.0);
    let car_y = CAR_Y + (VIEW_TOP + 90.0 - CAR_Y) * climb;
    let jet = smoothstep(((progress - 0.54) / 0.08).clamp(0.0, 1.0))
        * (1.0 - smoothstep(((progress - 0.94) / 0.06).clamp(0.0, 1.0)));
    FinishPose {
        road_lift,
        rear_scale_y,
        top_scale_y,
        climb,
        car_y,
        jet,
    }
}

#[allow(clippy::type_complexity)]
fn sync_jump_car(
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    finish: Res<crate::FinishSequence>,
    launch: Res<crate::LaunchSequence>,
    mut car: Query<(&mut Transform, &mut Visibility), With<PlayerCarRoot>>,
    mut shadow: Query<
        (&mut Transform, &mut Sprite),
        (With<PlayerCarShadow>, Without<PlayerCarRoot>),
    >,
) {
    let pose = if sim.0.jump_active() {
        jump_pose(sim.0.jump_progress(&tuning.0))
    } else {
        cold_air_hop_pose(sim.0.cold_air_hop_progress(&tuning.0))
    };
    let finale = finish_pose(finish.progress());
    let (launch_height, _) = if launch.active {
        launch_drop_pose(launch.progress())
    } else {
        (0.0, 0.0)
    };
    if let Ok((mut transform, mut visibility)) = car.single_mut() {
        transform.translation = Vec3::new(0.0, CAR_Y + pose.height + launch_height, 0.5);
        // This is intentionally a simple 2D sprite spin in the screen plane.
        transform.rotation = Quat::from_rotation_z(pose.sprite_spin);
        transform.scale = Vec3::new(
            pose.scale.x,
            pose.scale.y * finale.rear_scale_y,
            pose.scale.z,
        );
        *visibility = if finish.active && finale.rear_scale_y <= 0.01 {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    if let Ok((mut transform, mut sprite)) = shadow.single_mut() {
        transform.scale = Vec3::new(pose.shadow_scale, pose.shadow_scale, 1.0);
        let altitude_fade = (1.0 - launch_height / 520.0).clamp(0.0, 1.0);
        sprite.color = Color::srgba(
            0.0,
            0.0,
            0.0,
            pose.shadow_alpha * finale.rear_scale_y * altitude_fade,
        );
    }
}

fn sync_finish_car(
    finish: Res<crate::FinishSequence>,
    mut car: Query<(&mut Transform, &mut Visibility), With<FinishCarRoot>>,
) {
    let Ok((mut transform, mut visibility)) = car.single_mut() else {
        return;
    };
    let pose = finish_pose(finish.progress());
    if !finish.active || pose.top_scale_y <= 0.01 {
        *visibility = Visibility::Hidden;
        return;
    }
    let flight_scale = 1.0 - pose.climb * 0.12;
    transform.translation = Vec3::new(0.0, pose.car_y, 0.62);
    transform.rotation = Quat::IDENTITY;
    transform.scale = Vec3::new(flight_scale, pose.top_scale_y * flight_scale, 1.0);
    *visibility = Visibility::Visible;
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TimeCarPose {
    position: Vec2,
    scale: f32,
    car_visible: bool,
    flash: f32,
}

fn time_car_pose(progress: f32) -> TimeCarPose {
    let progress = progress.clamp(0.0, 1.0);
    let travel = smoothstep((progress / 0.78).clamp(0.0, 1.0));
    TimeCarPose {
        position: Vec2::new(
            -62.0 + travel * 40.0 + (progress * std::f32::consts::TAU * 2.0).sin() * 5.0,
            -330.0 + travel * 418.0,
        ),
        scale: 1.18 + (0.18 - 1.18) * travel,
        car_visible: progress < 0.78,
        flash: (1.0 - ((progress - 0.82) / 0.18).abs()).clamp(0.0, 1.0),
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sync_time_travel(
    time: Res<Time>,
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    mut state: ResMut<TimeTravelVisuals>,
    mut car: Query<(&mut Transform, &mut Visibility), With<TimeCarRoot>>,
    mut trails: Query<
        (&TimeTrail, &mut Transform, &mut Sprite, &mut Visibility),
        (Without<TimeCarRoot>, Without<TimeFlash>),
    >,
    mut flash: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (With<TimeFlash>, Without<TimeCarRoot>, Without<TimeTrail>),
    >,
) {
    let running = matches!(sim.0.phase, Phase::Running);
    state.update(
        running,
        sim.0.paused,
        sim.0.distance,
        sim.0.speed * 3.6,
        sim.0.straightaway_overdrive_ready(&tuning.0),
        time.delta_secs(),
    );

    let active = running && state.active();
    let progress = if active {
        state.active_secs / TIME_TRAVEL_DURATION_SECS
    } else {
        1.0
    };
    let pose = time_car_pose(progress);
    if let Ok((mut transform, mut visibility)) = car.single_mut() {
        transform.translation = Vec3::new(
            pose.position.x,
            pose.position.y,
            if pose.position.y < CAR_Y { 0.46 } else { 0.78 },
        );
        transform.scale = Vec3::splat(pose.scale);
        *visibility = if active && pose.car_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (trail, mut transform, mut sprite, mut visibility) in &mut trails {
        if active && pose.car_visible {
            let index = trail.0 as f32;
            let side = if trail.0 % 2 == 0 { -1.0 } else { 1.0 };
            let flicker = 0.65 + 0.35 * (time.elapsed_secs() * 24.0 + index * 1.91).sin().abs();
            transform.translation = Vec3::new(
                pose.position.x + side * (7.0 + (trail.0 % 4) as f32 * 3.0) * pose.scale,
                pose.position.y - 8.0 - index * (5.5 + pose.scale * 5.0),
                0.72,
            );
            transform.rotation = Quat::from_rotation_z(side * (0.22 + index * 0.018));
            sprite.custom_size = Some(Vec2::new(
                (2.5 * pose.scale).max(1.5),
                (18.0 * pose.scale).max(5.0),
            ));
            sprite.color = Color::linear_rgba(0.16, 0.95, 2.2, 0.34 + flicker * 0.46);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
    if let Ok((mut transform, mut sprite, mut visibility)) = flash.single_mut() {
        let size = 34.0 + pose.flash * 180.0;
        transform.translation.x = pose.position.x;
        transform.translation.y = pose.position.y;
        transform.rotation = Quat::from_rotation_z(progress * 7.0);
        sprite.custom_size = Some(Vec2::splat(size));
        sprite.color = Color::linear_rgba(0.65, 1.25, 2.4, pose.flash * 0.92);
        *visibility = if active && pose.flash > 0.01 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn sync_player_rear_light(sim: Res<Sim>, mut light: Query<&mut Sprite, With<PlayerRearLight>>) {
    let daylight = game_core::environment::daylight(sim.0.environment_time);
    let night = (1.0 - daylight).clamp(0.0, 1.0);
    let Ok(mut sprite) = light.single_mut() else {
        return;
    };
    // The whole diffuser remains visible by day and crosses the HDR threshold
    // at night, so CRT bloom reads as one uninterrupted blade rather than two
    // conventional tail lamps.
    sprite.color = Color::linear_rgb(0.58 + night * 1.85, 0.008 + night * 0.018, 0.005);
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
    sim: Res<Sim>,
    visuals: Res<NitroVisuals>,
    launch: Res<crate::LaunchSequence>,
    finish: Res<crate::FinishSequence>,
    time: Res<Time>,
    mut jets: Query<(&NitroExhaust, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (launch_height, launch_jet) = if launch.active {
        launch_drop_pose(launch.progress())
    } else {
        (0.0, 0.0)
    };
    let finale = finish_pose(finish.progress());
    let intensity = visuals.intensity.max(launch_jet).max(finale.jet);
    if intensity <= 0.01 || sim.0.jump_active() {
        for (_, _, _, mut visibility) in &mut jets {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    for (jet, mut transform, mut sprite, mut visibility) in &mut jets {
        let shape = exhaust_shape(jet.layer, jet.side, time.elapsed_secs(), intensity);
        transform.translation.x = jet.side * 27.0;
        let finish_height = if finish.active {
            finale.car_y - CAR_Y
        } else {
            0.0
        };
        transform.translation.y = shape.center_y + launch_height + finish_height;
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
    // Anime-like boost streaks live around the player instead of stretching
    // from the car all the way to the horizon. Their widening and lengthening
    // toward the foreground still preserve the road's perspective.
    let scale = SPEED_LINE_FAR_SCALE + phase * (SPEED_LINE_NEAR_SCALE - SPEED_LINE_FAR_SCALE);
    let spread = 1.02 + (index % 4) as f32 * 0.09;
    let x = road_center + side * HALF_ROAD_PX * scale * spread;
    let jitter = (index % 7) as f32 - 3.0;
    let y = scale_to_y(scale) + jitter * 3.5;
    let size = Vec2::new(
        1.5 + intensity * 2.0,
        (16.0 + phase * 42.0) * (0.58 + intensity * 0.42),
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
        let row =
            (((1.0 - phase) * SPEED_LINE_FAR_ROW as f32).round() as usize).min(SPEED_LINE_FAR_ROW);
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
        let near_strength = 0.45 + phase * 0.55;
        sprite.color = Color::srgba(
            0.72,
            0.94,
            1.0,
            visuals.intensity * (0.12 + near_strength * 0.24),
        );
        *visibility = Visibility::Visible;
    }
}

#[derive(Debug, Clone, Copy)]
struct RoadsideDecorPart {
    color: Color,
    size: Vec2,
    dx: f32,
    dy: f32,
    rotation: f32,
}

fn roadside_decor_count(theme: backdrop::RoadsideTheme, season: usize) -> usize {
    let base = match theme {
        backdrop::RoadsideTheme::Forest => 24,
        backdrop::RoadsideTheme::Sakura => 22,
        backdrop::RoadsideTheme::City => 18,
        backdrop::RoadsideTheme::Winter => 12,
        backdrop::RoadsideTheme::OpenRoad => 16,
    };
    let seasonal_delta = match season % 4 {
        0 => 2, // spring growth
        1 => 0,
        2 => -2, // autumn thins out
        _ => -4, // sparse winter silhouettes
    };
    (base + seasonal_delta).clamp(2, ROADSIDE_DECOR_PROPS as i32) as usize
}

fn roadside_decor_part(
    theme: backdrop::RoadsideTheme,
    part: usize,
    night: f32,
    side: f32,
) -> RoadsideDecorPart {
    let branch_tilt = side * 0.42;
    match (theme, part) {
        (backdrop::RoadsideTheme::City, 0) => RoadsideDecorPart {
            color: Color::srgb(0.10, 0.13, 0.20),
            size: Vec2::new(7.0, 68.0),
            dx: 0.0,
            dy: 34.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::City, 1) => RoadsideDecorPart {
            color: Color::srgb(0.14, 0.18, 0.26),
            size: Vec2::new(34.0, 5.0),
            dx: -side * 13.0,
            dy: 65.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::City, _) => RoadsideDecorPart {
            color: Color::srgba(1.0, 0.82, 0.42, 0.20 + night * 0.80),
            size: Vec2::new(18.0, 12.0),
            dx: -side * 26.0,
            dy: 61.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::Winter, 0) => RoadsideDecorPart {
            color: Color::srgb(0.30, 0.25, 0.24),
            size: Vec2::new(9.0, 62.0),
            dx: 0.0,
            dy: 31.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::Winter, 1) => RoadsideDecorPart {
            color: Color::srgb(0.38, 0.31, 0.29),
            size: Vec2::new(44.0, 5.0),
            dx: 0.0,
            dy: 44.0,
            rotation: branch_tilt,
        },
        (backdrop::RoadsideTheme::Winter, _) => RoadsideDecorPart {
            color: Color::srgb(0.42, 0.35, 0.33),
            size: Vec2::new(34.0, 4.0),
            dx: 0.0,
            dy: 55.0,
            rotation: -branch_tilt,
        },
        (backdrop::RoadsideTheme::Sakura, 0) => RoadsideDecorPart {
            color: Color::srgb(0.34, 0.16, 0.14),
            size: Vec2::new(11.0, 58.0),
            dx: 0.0,
            dy: 29.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::Sakura, 1) => RoadsideDecorPart {
            color: Color::srgb(0.78, 0.30, 0.52),
            size: Vec2::new(68.0, 39.0),
            dx: -9.0,
            dy: 62.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::Sakura, _) => RoadsideDecorPart {
            color: Color::srgb(1.0, 0.62, 0.80),
            size: Vec2::new(49.0, 31.0),
            dx: 13.0,
            dy: 75.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::Forest, 0) => RoadsideDecorPart {
            color: Color::srgb(0.26, 0.14, 0.07),
            size: Vec2::new(12.0, 66.0),
            dx: 0.0,
            dy: 33.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::Forest, 1) => RoadsideDecorPart {
            color: Color::srgb(0.035, 0.23, 0.10),
            size: Vec2::new(70.0, 43.0),
            dx: 0.0,
            dy: 66.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::Forest, _) => RoadsideDecorPart {
            color: Color::srgb(0.09, 0.43, 0.17),
            size: Vec2::new(50.0, 31.0),
            dx: 8.0,
            dy: 82.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::OpenRoad, 0) => RoadsideDecorPart {
            color: Color::srgb(0.42, 0.28, 0.14),
            size: Vec2::new(11.0, 58.0),
            dx: 0.0,
            dy: 29.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::OpenRoad, 1) => RoadsideDecorPart {
            color: Color::srgb(0.10, 0.52, 0.20),
            size: Vec2::new(65.0, 18.0),
            dx: 0.0,
            dy: 57.0,
            rotation: 0.0,
        },
        (backdrop::RoadsideTheme::OpenRoad, _) => RoadsideDecorPart {
            color: Color::srgb(0.13, 0.60, 0.24),
            size: Vec2::new(38.0, 13.0),
            dx: 0.0,
            dy: 48.0,
            rotation: 0.0,
        },
    }
}

/// Decorative shoulder props ride the loop and mirror the active backdrop.
#[allow(clippy::type_complexity)]
fn sync_palms(
    sim: Res<Sim>,
    scenery: Res<SceneryRun>,
    proj: Res<Proj>,
    finish: Res<crate::FinishSequence>,
    mut pool: Query<
        (&mut Transform, &mut Visibility, &mut Sprite),
        (With<PalmPart>, Without<HazardPart>),
    >,
) {
    if finish.active {
        for (_, mut visibility, _) in &mut pool {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let lp = sim.0.loop_pos();
    let theme = backdrop::roadside_theme(*scenery, sim.0.environment_time);
    let season = game_core::environment::season_index(sim.0.environment_time).floor() as usize;
    let count = roadside_decor_count(theme, season);
    let night = 1.0 - game_core::environment::daylight(sim.0.environment_time);
    let mut spots = [None; ROADSIDE_DECOR_PROPS];
    let mut spot_count = 0;
    let pairs = count / 2;
    let visible_span = 42.0;
    let spacing = visible_span / pairs as f32;
    let scroll = lp.rem_euclid(spacing);
    for k in 0..pairs {
        for side_sign in [-1.0f32, 1.0] {
            let spread = 1.28 + (k % 6) as f32 * 0.28;
            let side = side_sign * spread;
            let side_stagger = if side < 0.0 { spacing * 0.48 } else { 0.0 };
            let ahead = 4.5 + (k as f32 * spacing + side_stagger - scroll).rem_euclid(visible_span);
            if let Some((y, s, lat)) = proj.project(ahead, sim.0.road_x) {
                let x = (lat + side) * HALF_ROAD_PX * s;
                spots[spot_count] = Some((x, y, s, side));
                spot_count += 1;
            }
        }
    }
    let mut parts = pool.iter_mut();
    for (x, y, s, side) in spots.into_iter().flatten() {
        for part_index in 0..ROADSIDE_DECOR_PARTS_PER_PROP {
            let Some((mut tf, mut vis, mut sp)) = parts.next() else {
                break;
            };
            let look = roadside_decor_part(theme, part_index, night, side);
            tf.translation = Vec3::new(
                x + look.dx * s,
                y + look.dy * s,
                -3.0 + part_index as f32 * 0.05,
            );
            tf.rotation = Quat::from_rotation_z(look.rotation);
            sp.color = look.color;
            sp.custom_size = Some(Vec2::new(
                (look.size.x * s).max(1.5),
                (look.size.y * s).max(1.5),
            ));
            *vis = Visibility::Visible;
        }
    }
    for (_, mut vis, _) in parts.by_ref() {
        *vis = Visibility::Hidden;
    }
}

fn sync_decor_billboards(
    sim: Res<Sim>,
    scenery: Res<SceneryRun>,
    proj: Res<Proj>,
    finish: Res<crate::FinishSequence>,
    mut signs: Query<(&DecorBillboard, &mut Transform, &mut Visibility)>,
) {
    if finish.active {
        for (_, _, mut visibility) in &mut signs {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let theme = backdrop::roadside_theme(*scenery, sim.0.environment_time);
    let count = match theme {
        backdrop::RoadsideTheme::City => 5,
        backdrop::RoadsideTheme::Forest | backdrop::RoadsideTheme::Sakura => 3,
        backdrop::RoadsideTheme::OpenRoad => 2,
        backdrop::RoadsideTheme::Winter => 1,
    };
    let loop_position = sim.0.loop_pos();
    for (sign, mut transform, mut visibility) in &mut signs {
        if sign.0 >= count {
            *visibility = Visibility::Hidden;
            continue;
        }
        let ahead = 7.0 + (sign.0 as f32 * 19.0 - loop_position).rem_euclid(76.0);
        let Some((y, scale, lateral)) = proj.project(ahead, sim.0.road_x) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let side = if sign.0.is_multiple_of(2) { -1.0 } else { 1.0 };
        let spread = 1.72 + (sign.0 % 3) as f32 * 0.30;
        transform.translation =
            Vec3::new((lateral + side * spread) * HALF_ROAD_PX * scale, y, -2.72);
        transform.scale = Vec3::splat(scale);
        *visibility = Visibility::Visible;
    }
}

fn sync_flash(sim: Res<Sim>, mut q: Query<&mut Sprite, With<FlashOverlay>>) {
    if let Ok(mut sp) = q.single_mut() {
        sp.color = Color::srgba(1.0, 1.0, 1.0, sim.0.flash.clamp(0.0, 1.0) * 0.85);
    }
}

#[allow(clippy::type_complexity)]
fn sync_jump_hud(
    sim: Res<Sim>,
    effects: Res<crate::VisualEffects>,
    time: Res<Time>,
    mut parts: Query<
        (&mut Sprite, Option<&JumpMeterFill>, Option<&JumpMeterHalo>),
        Or<(With<JumpMeterFill>, With<JumpMeterHalo>)>,
    >,
) {
    let ready_pulse = 0.5 + 0.5 * (time.elapsed_secs() * std::f32::consts::TAU * 1.35).sin();
    let pickup = (effects.jump_pickup_glow / crate::JUMP_PICKUP_GLOW_SECS).clamp(0.0, 1.0);
    for (mut sprite, fill, halo) in &mut parts {
        if fill.is_some() {
            sprite.color = if sim.0.jump_charge {
                Color::srgb(1.0, 0.24 + 0.20 * pickup, 0.70 + 0.24 * pickup)
            } else {
                Color::srgb(0.11, 0.12, 0.19)
            };
        } else if halo.is_some() {
            let alpha = if sim.0.jump_charge {
                0.10 + 0.20 * ready_pulse + 0.35 * pickup
            } else {
                0.0
            };
            sprite.color = Color::srgba(1.0, 0.18, 0.70, alpha.clamp(0.0, 0.72));
        }
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
        let volume_notice = format!(
            "{} {:03}%  |  [ ] music  - = effects",
            audio.level_notice,
            if audio.level_notice == "MUSIC" {
                audio.music_gain * 100.0
            } else {
                audio.effects_gain * 100.0
            }
        );
        let next = if audio.level_notice_secs > 0.0 {
            volume_notice.as_str()
        } else if audio.muted {
            "SOUND OFF | M to unmute"
        } else if sim.0.paused {
            "PAUSED | Start/P resume | Select/Esc restart"
        } else if view.mode == crate::toy_view::ViewMode::Toy {
            match &sim.0.phase {
                Phase::Ready => {
                    "ARROWS drive | Q low / E high | Y cold-air | R restart | T flat | L toy look"
                }
                Phase::Running => "Y cold-air | T flat view | L toy look",
                Phase::GameOver { .. } => "",
            }
        } else {
            match &sim.0.phase {
                Phase::Ready => {
                    "ARROWS steer | Q/E gear | Y cold-air | B/SPACE jump | R restart | T toy"
                }
                Phase::Running | Phase::GameOver { .. } => "",
            }
        };
        if text.as_str() != next {
            **text = next.to_owned();
        }
    }
}

fn sync_countdown_style(
    sim: Res<Sim>,
    time: Res<Time>,
    mut timer: Query<(&mut TextColor, &mut Transform), With<SessionText>>,
) {
    let Ok((mut color, mut transform)) = timer.single_mut() else {
        return;
    };
    let style = countdown_style(
        sim.0.session_remaining,
        time.elapsed_secs(),
        matches!(sim.0.phase, Phase::Running) && !sim.0.paused,
    );
    color.0 = style.color;
    transform.scale = Vec3::splat(style.scale);
}

fn pause_overlay_visible(paused: bool, playing: bool) -> bool {
    paused && playing
}

fn sync_pause_overlay(
    sim: Res<Sim>,
    end: Res<crate::EndSession>,
    mut root: Query<&mut Visibility, With<PauseOverlayRoot>>,
) {
    if !sim.is_changed() && !end.is_changed() {
        return;
    }
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = if pause_overlay_visible(
            sim.0.paused,
            end.0.mode == crate::session::EndMode::Playing,
        ) {
            Visibility::Visible
        } else {
            Visibility::Hidden
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
                crate::session::EndMode::Leaderboard => {
                    "START, R, or ENTER returns to run select".to_string()
                }
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

fn asphalt_palette(brightness: f32, rain: f32) -> (Color, Color, Color) {
    let wet = rain.clamp(0.0, 1.0);
    let sheen = wet * 0.32;
    let asphalt_a = Color::srgb(0.34, 0.35, 0.40).mix(&Color::srgb(0.38, 0.40, 0.47), sheen);
    let asphalt_b = Color::srgb(0.30, 0.31, 0.36).mix(&Color::srgb(0.28, 0.30, 0.37), sheen);
    let reflected_markings = brightness * (1.0 + wet * 0.08);
    (
        scale_color(asphalt_a, brightness),
        scale_color(asphalt_b, brightness),
        scale_color(Color::srgb(0.95, 0.95, 0.90), reflected_markings),
    )
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

fn road_visibility(daylight: f32, precipitation: f32) -> RoadVisibilityVisuals {
    let daylight = daylight.clamp(0.0, 1.0);
    let night = 1.0 - daylight;
    let rain = precipitation.clamp(0.0, 1.0);
    let night_headlights = ((0.55 - daylight) / 0.40).clamp(0.0, 1.0);
    RoadVisibilityVisuals {
        night,
        rain,
        // Keep the foreground readable and put the difficulty in the distant
        // gradient, rather than dimming the entire game screen uniformly.
        shade_alpha: (night * 0.08 + rain * 0.03).min(0.10),
        fog_alpha: (night * 0.68 + rain * 0.68).min(0.84),
        headlights: night_headlights.max(rain * 0.88),
    }
}

fn fog_band_geometry(index: usize) -> (f32, Vec2, f32) {
    let band_h = 435.0 / VISIBILITY_BANDS as f32;
    let t = index as f32 / (VISIBILITY_BANDS - 1) as f32;
    let y = -300.0 + band_h * (index as f32 + 0.5);
    (
        y,
        Vec2::new(960.0, band_h + 1.0),
        0.06 + smoothstep(t) * 0.94,
    )
}

fn projected_headlight_geometry(layer: usize, y0: f32, y1: f32) -> (Vec2, Vec2, f32) {
    let row_height = (y1 - y0).max(1.5);
    let center_y = y0 + row_height * 0.5;
    let road_depth = HORIZON_Y - CAR_Y;
    let fade_start_y = CAR_Y + road_depth * 0.32;
    let beam_end_y = CAR_Y + road_depth * 0.50;
    let distance = ((center_y - CAR_Y) / (beam_end_y - CAR_Y)).clamp(0.0, 1.0);
    let outer_width = 64.0 + distance * 250.0;
    let layer_width = if layer == 0 {
        outer_width
    } else {
        outer_width * 0.62
    };
    // The beam is rendered as deliberate pixel bands. Keep the bands thin at
    // the bumper, then grow them with projected distance so they describe a
    // ground-plane cone instead of a stack of equally heavy rectangles.
    let band_height = 2.5 + smoothstep(distance) * if layer == 0 { 13.5 } else { 10.0 };
    let near_fade = ((center_y - CAR_Y) / 55.0).clamp(0.0, 1.0);
    let far_fade = 1.0 - ((center_y - fade_start_y) / (beam_end_y - fade_start_y)).clamp(0.0, 1.0);
    let layer_strength = if layer == 0 { 0.28 } else { 0.72 };
    (
        Vec2::new(0.0, center_y),
        Vec2::new(layer_width.max(2.0), band_height),
        near_fade * far_fade * layer_strength,
    )
}

fn traffic_visibility(scale: f32, visibility: RoadVisibilityVisuals) -> (f32, f32) {
    let near = ((scale - S_FAR) / (0.42 - S_FAR)).clamp(0.0, 1.0);
    let distance = 1.0 - near;
    let visibility_loss = (visibility.night * 0.92 + visibility.rain * 0.86).min(0.96);
    let body_alpha = (1.0 - visibility_loss * distance).clamp(0.06, 1.0);
    let light_alpha = visibility.headlights * (0.58 + distance * 0.42);
    (body_alpha, light_alpha.clamp(0.0, 1.0))
}

fn traffic_car_color(variant: usize, alpha: f32) -> Color {
    let [r, g, b] = traffic_car_rgb(variant);
    Color::srgba(r, g, b, alpha)
}

fn traffic_car_rgb(variant: usize) -> [f32; 3] {
    match variant % 4 {
        0 => [0.93, 0.91, 0.85],
        1 => [0.90, 0.16, 0.12],
        2 => [0.96, 0.52, 0.07],
        _ => [0.22, 0.68, 0.25],
    }
}

#[derive(Clone, Copy)]
struct RoadsidePropPalette {
    tree_dark: Color,
    tree_light: Color,
    billboard_frame: Color,
    billboard_face: Color,
}

fn roadside_prop_palette(
    seasons: [f32; 4],
    daylight: f32,
    theme: backdrop::RoadsideTheme,
) -> RoadsidePropPalette {
    let (tree_dark, tree_light) = match theme {
        backdrop::RoadsideTheme::City => (
            Color::srgb(0.16, 0.19, 0.27),
            Color::srgba(
                1.0,
                0.78 + (1.0 - daylight.clamp(0.0, 1.0)) * 0.12,
                0.34,
                0.20 + (1.0 - daylight.clamp(0.0, 1.0)) * 0.80,
            ),
        ),
        backdrop::RoadsideTheme::Sakura => {
            (Color::srgb(0.70, 0.27, 0.45), Color::srgb(1.0, 0.62, 0.80))
        }
        backdrop::RoadsideTheme::Forest => (
            Color::srgb(0.045, 0.25, 0.11),
            Color::srgb(0.10, 0.48, 0.19),
        ),
        backdrop::RoadsideTheme::Winter => {
            (Color::srgb(0.34, 0.28, 0.27), Color::srgb(0.48, 0.40, 0.38))
        }
        backdrop::RoadsideTheme::OpenRoad => (
            blend4(
                [
                    Color::srgb(0.66, 0.28, 0.43),
                    Color::srgb(0.05, 0.30, 0.12),
                    Color::srgb(0.55, 0.20, 0.05),
                    Color::srgb(0.34, 0.28, 0.27),
                ],
                seasons,
            ),
            blend4(
                [
                    Color::srgb(1.0, 0.58, 0.76),
                    Color::srgb(0.10, 0.48, 0.19),
                    Color::srgb(0.94, 0.42, 0.08),
                    Color::srgb(0.48, 0.40, 0.38),
                ],
                seasons,
            ),
        ),
    };
    let night = 1.0 - daylight.clamp(0.0, 1.0);
    let winter = seasons[3];
    let billboard_frame = Color::srgb(0.95, 0.35, 0.12)
        .mix(&Color::srgb(1.0, 0.10, 0.68), night * 0.72)
        .mix(&Color::srgb(0.42, 0.72, 0.94), winter * 0.45);
    let billboard_face = Color::srgb(0.98, 0.90, 0.38)
        .mix(&Color::srgb(0.08, 0.92, 1.0), night * 0.70)
        .mix(&Color::srgb(0.86, 0.94, 1.0), winter * 0.35);
    RoadsidePropPalette {
        tree_dark,
        tree_light,
        billboard_frame,
        billboard_face,
    }
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
    (colors.asphalt_a, colors.asphalt_b, colors.dash) = asphalt_palette(bright, rain_visible);
    // rumble goes snow-pale in deep winter
    let rum_a = Color::srgb(0.85, 0.15, 0.12).mix(&Color::srgb(0.88, 0.9, 0.95), winter * 0.6);
    let rum_b = Color::srgb(0.95, 0.95, 0.92).mix(&Color::srgb(0.7, 0.75, 0.85), winter * 0.5);
    colors.rumble_red = scale_color(rum_a, bright);
    colors.rumble_white = scale_color(rum_b, bright);

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
) {
    let daylight = game_core::environment::daylight(sim.0.environment_time);
    let rain = weather_weights(sim.0.weather())[2];
    // Winter precipitation renders as snow instead of rain streaks, but its
    // distance haze remains just as important to gameplay visibility.
    *visuals = road_visibility(daylight, rain);

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
}

fn sync_projected_headlights(
    visuals: Res<RoadVisibilityVisuals>,
    proj: Res<Proj>,
    mut headlights: Query<(&HeadlightBand, &mut Transform, &mut Sprite)>,
) {
    for (band, mut transform, mut sprite) in &mut headlights {
        let y0 = proj.rows_y[band.row];
        let y1 = if band.row + 1 < ROWS {
            proj.rows_y[band.row + 1]
        } else {
            HORIZON_Y + 2.0
        };
        let (center, size, strength) = projected_headlight_geometry(band.layer, y0, y1);
        transform.translation.x = center.x;
        transform.translation.y = center.y;
        sprite.custom_size = Some(size);
        let (red, green, blue) = if band.layer == 0 {
            (1.0, 0.78, 0.42)
        } else {
            (1.0, 0.91, 0.68)
        };
        sprite.color = Color::srgba(red, green, blue, visuals.headlights * strength * 0.055);
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

fn speedometer_readout(displayed_mph: f32, time_travel_active: bool) -> String {
    if time_travel_active {
        "088 mph".to_string()
    } else {
        format!("{:03.0} mph", displayed_mph.min(999.0))
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sync_instruments(
    time: Res<Time>,
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    effects: Res<crate::VisualEffects>,
    time_travel: Res<TimeTravelVisuals>,
    mut visuals: ResMut<InstrumentVisuals>,
    mut needle: Query<
        (&mut Transform, &mut Sprite),
        (With<SpeedNeedle>, Without<SpeedTick>, Without<CooldownFill>),
    >,
    mut digital: Query<
        (&mut Text2d, &mut TextColor),
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
    // Deliberately retain the arcade dial's existing numbers while relabeling
    // them as mph for the 88-speed easter egg.
    let displayed_mph = sim.0.speed * 3.6;
    let angle = speedometer_angle(displayed_mph);
    let danger = speedometer_danger(displayed_mph, crate::nitro_effects_active(&sim.0));
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
    if let Ok((mut text, mut color)) = digital.single_mut() {
        if time_travel.active() {
            text.0 = speedometer_readout(displayed_mph, true);
            color.0 = if (time.elapsed_secs() * 9.0).fract() < 0.55 {
                Color::linear_rgb(2.2, 0.05, 0.03)
            } else {
                Color::srgb(1.0, 0.72, 0.32)
            };
        } else {
            text.0 = speedometer_readout(displayed_mph, false);
            color.0 = Color::srgb(0.95, 0.90, 0.70);
        }
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
    visuals.speed_kmh = displayed_mph;
    visuals.needle_angle = angle;
    visuals.overrev = sim.0.low_overrev(&tuning.0);
    visuals.cooldown = sim.0.nitro_cooldown;
    visuals.gauge_fill = gauge;
    visuals.recovery = recovery_name;
    visuals.cue = cue;
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sync_difficulty_overlay(
    end: Res<crate::EndSession>,
    intro: Res<crate::toy_view::StartupIntro>,
    leaderboard: Res<crate::LeaderboardState>,
    display: Res<crate::DisplaySettings>,
    mut root: Query<&mut Visibility, With<DifficultyOverlayRoot>>,
    mut cards: Query<(&DifficultyCard, &mut Sprite)>,
    mut texts: Query<
        (&DifficultyCardText, &mut TextColor),
        (Without<CrtMenuText>, Without<LargeMenuText>),
    >,
    mut board: Query<
        &mut Text2d,
        (
            With<DifficultyLeaderboard>,
            Without<CrtMenuText>,
            Without<LargeMenuText>,
        ),
    >,
    mut crt: Query<
        (&mut Text2d, &mut TextColor),
        (
            With<CrtMenuText>,
            Without<DifficultyLeaderboard>,
            Without<DifficultyCardText>,
            Without<LargeMenuText>,
        ),
    >,
    mut large: Query<
        (&mut Text2d, &mut TextColor),
        (
            With<LargeMenuText>,
            Without<CrtMenuText>,
            Without<DifficultyLeaderboard>,
            Without<DifficultyCardText>,
        ),
    >,
) {
    if !end.is_changed()
        && !intro.is_changed()
        && !leaderboard.is_changed()
        && !display.is_changed()
    {
        return;
    }
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = if end.0.mode == crate::session::EndMode::DifficultySelect && intro.complete()
        {
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
    let difficulty = game_core::Difficulty::ALL[end.0.difficulty_index];
    if let Ok(mut text) = board.single_mut() {
        text.0 = difficulty_menu_leaderboard_table(&leaderboard.entries, difficulty);
    }
    if let Ok((mut text, mut color)) = crt.single_mut() {
        let (label, tint) = if display.crt_enabled {
            ("ON", Color::srgb(0.55, 1.0, 0.74))
        } else {
            ("OFF", Color::srgb(1.0, 0.62, 0.38))
        };
        text.0 = format!("CRT DISPLAY: {label}   [ C / X ]");
        color.0 = tint;
    }
    if let Ok((mut text, mut color)) = large.single_mut() {
        let (label, tint) = if display.large_screen {
            ("ON", Color::srgb(0.55, 1.0, 0.74))
        } else {
            ("OFF", Color::srgb(1.0, 0.62, 0.38))
        };
        text.0 = format!("LARGE SCREEN: {label}   [ V / L3+R3 ]");
        color.0 = tint;
    }
}

fn difficulty_menu_leaderboard_table(
    entries: &[game_core::LeaderboardEntry],
    difficulty: game_core::Difficulty,
) -> String {
    let mut lines = vec![
        format!("LOCAL TOP 5 — {}", difficulty.label()),
        "#  TAG  SCORE   CYC PASS BUMP".to_string(),
    ];
    let mut ranked = game_core::leaderboard_for(entries, difficulty);
    for rank in 1..=5 {
        if let Some(entry) = ranked.next() {
            lines.push(format!(
                "{rank}  {}  {:06}   {:02}  {:03}  {:03}",
                entry.initials_str(),
                entry.result.score,
                entry.result.day_cycles,
                entry.result.cars_passed,
                entry.result.cars_bumped
            ));
        } else {
            lines.push(format!("{rank}  ---  ------   --  ---  ---"));
        }
    }
    lines.join("\n")
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
    app.add_systems(
        Update,
        (sync_nitro_visual_intensity, sync_time_travel)
            .chain()
            .in_set(ViewSet::Effects),
    );
    app.add_systems(
        Update,
        (
            backdrop::sync,
            shoulder::sync,
            ambient::sync,
            sync_palms,
            sync_hazards,
            sync_projected_headlights,
            sync_nitro_pickup,
            sync_jump_pickup,
            sync_jump_car,
            sync_player_rear_light,
            sync_nitro_exhaust,
            sync_nitro_speed_lines,
            sync_flash,
            sync_drum_overlays,
            sync_hud,
            sync_jump_hud,
            sync_countdown_style,
            sync_instruments,
            sync_end_overlay,
            sync_difficulty_overlay,
        )
            .in_set(ViewSet::Presentation),
    );
    app.add_systems(
        Update,
        (sync_decor_billboards, sync_finish_car, sync_pause_overlay).in_set(ViewSet::Presentation),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_overlay_only_appears_during_a_paused_run() {
        assert!(pause_overlay_visible(true, true));
        assert!(!pause_overlay_visible(false, true));
        assert!(!pause_overlay_visible(true, false));
    }

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
    fn constant_curvature_produces_an_arc_instead_of_a_shifted_straight() {
        let (heading_1, lateral_1) = advance_curve_projection(0.0, 0.0, 1.0, 1.0);
        let (heading_2, lateral_2) = advance_curve_projection(heading_1, lateral_1, 1.0, 1.0);
        let (heading_3, lateral_3) = advance_curve_projection(heading_2, lateral_2, 1.0, 1.0);

        assert_eq!((heading_1, lateral_1), (1.0, 0.5));
        assert_eq!((heading_2, lateral_2), (2.0, 2.0));
        assert_eq!((heading_3, lateral_3), (3.0, 4.5));
        assert!(lateral_3 - lateral_2 > lateral_2 - lateral_1);
    }

    #[test]
    fn opposing_curve_unwinds_heading_continuously() {
        let (heading, lateral) = advance_curve_projection(0.0, 0.0, 1.0, 2.0);
        let (heading, unwind_lateral) = advance_curve_projection(heading, lateral, -1.0, 2.0);

        assert_eq!(heading, 0.0);
        assert!(
            unwind_lateral > lateral,
            "the ribbon must not snap sideways"
        );
    }

    #[test]
    fn final_countdown_escalates_while_the_regular_timer_stays_steady() {
        let normal = countdown_style(10.01, 0.0, true);
        let paused = countdown_style(2.0, 0.1, false);
        assert_eq!(normal.scale, 1.0);
        assert_eq!(paused.scale, 1.0);

        let early_peak = countdown_style(9.0, 0.25 / 3.4, true);
        let late_peak = countdown_style(1.0, 0.25 / 6.6, true);
        let late_dim = countdown_style(1.0, 0.75 / 6.6, true);
        assert!(early_peak.scale > 1.0);
        assert!(late_peak.scale > early_peak.scale);
        assert!(late_peak.color.to_srgba().alpha > late_dim.color.to_srgba().alpha);
    }

    #[test]
    fn rain_adds_a_subtle_cool_sheen_to_the_road() {
        let (dry_a, dry_b, dry_dash) = asphalt_palette(1.0, 0.0);
        let (wet_a, wet_b, wet_dash) = asphalt_palette(1.0, 1.0);
        let dry_a = dry_a.to_linear();
        let dry_b = dry_b.to_linear();
        let wet_a = wet_a.to_linear();
        let wet_b = wet_b.to_linear();

        assert!(wet_a.blue - wet_b.blue > dry_a.blue - dry_b.blue);
        assert!(wet_a.blue / wet_a.red > dry_a.blue / dry_a.red);
        assert!(wet_dash.to_linear().red > dry_dash.to_linear().red);
    }

    #[test]
    fn roadside_props_echo_season_and_night_palettes() {
        let spring =
            roadside_prop_palette([1.0, 0.0, 0.0, 0.0], 1.0, backdrop::RoadsideTheme::Sakura);
        let summer =
            roadside_prop_palette([0.0, 1.0, 0.0, 0.0], 1.0, backdrop::RoadsideTheme::Forest);
        let winter =
            roadside_prop_palette([0.0, 0.0, 0.0, 1.0], 1.0, backdrop::RoadsideTheme::Winter);
        let neon = roadside_prop_palette([0.0, 1.0, 0.0, 0.0], 0.0, backdrop::RoadsideTheme::City);

        assert!(spring.tree_light.to_srgba().red > summer.tree_light.to_srgba().red);
        assert!(winter.tree_light.to_srgba().red < summer.tree_light.to_srgba().red + 0.4);
        assert!(neon.billboard_face.to_srgba().blue > summer.billboard_face.to_srgba().blue);
        assert_eq!(neon.tree_light.to_srgba().alpha, 1.0);
    }

    #[test]
    fn contextual_roadside_decor_is_larger_and_density_tracks_scenery() {
        assert_eq!(roadside_decor_count(backdrop::RoadsideTheme::Forest, 0), 24);
        assert_eq!(roadside_decor_count(backdrop::RoadsideTheme::Winter, 3), 8);
        assert!(
            roadside_decor_count(backdrop::RoadsideTheme::OpenRoad, 0)
                > roadside_decor_count(backdrop::RoadsideTheme::OpenRoad, 2)
        );
        assert!(
            roadside_decor_count(backdrop::RoadsideTheme::OpenRoad, 2)
                > roadside_decor_count(backdrop::RoadsideTheme::OpenRoad, 3)
        );
        let forest = roadside_decor_part(backdrop::RoadsideTheme::Forest, 1, 0.0, 1.0);
        let sakura = roadside_decor_part(backdrop::RoadsideTheme::Sakura, 1, 0.0, 1.0);
        let winter = roadside_decor_part(backdrop::RoadsideTheme::Winter, 1, 0.0, 1.0);
        let city_day = roadside_decor_part(backdrop::RoadsideTheme::City, 2, 0.0, 1.0);
        let city_night = roadside_decor_part(backdrop::RoadsideTheme::City, 2, 1.0, 1.0);
        assert!(forest.size.x >= 70.0 && sakura.size.x >= 68.0);
        assert!(winter.size.y <= 5.0, "winter branches must be leafless");
        assert!(city_night.color.to_srgba().alpha > city_day.color.to_srgba().alpha);
        let densest_theme = [
            backdrop::RoadsideTheme::Forest,
            backdrop::RoadsideTheme::Sakura,
            backdrop::RoadsideTheme::City,
            backdrop::RoadsideTheme::Winter,
            backdrop::RoadsideTheme::OpenRoad,
        ]
        .into_iter()
        .map(|theme| roadside_decor_count(theme, 1))
        .max()
        .unwrap();
        assert!(densest_theme <= ROADSIDE_DECOR_PROPS);
        assert_eq!(
            ROADSIDE_DECOR_PROPS * ROADSIDE_DECOR_PARTS_PER_PROP,
            72,
            "the startup pool must reserve every part of every prop"
        );
    }

    #[test]
    fn road_pickups_are_visually_larger_than_their_original_silhouettes() {
        let projection_scale = 0.37;
        assert!(pickup_visual_scale(projection_scale) >= projection_scale * 1.20);
    }

    #[test]
    fn speedometer_keeps_the_existing_numeric_sweep() {
        assert!((speedometer_angle(0.0) - 225.0_f32.to_radians()).abs() < 1e-5);
        assert!((speedometer_angle(60.0) - 90.0_f32.to_radians()).abs() < 1e-5);
        assert!((speedometer_angle(120.0) + 45.0_f32.to_radians()).abs() < 1e-5);
        assert!(speedometer_angle(0.0) > speedometer_angle(120.0));
        assert_eq!(speedometer_readout(87.4, false), "087 mph");
        assert_eq!(speedometer_readout(112.0, true), "088 mph");
    }

    #[test]
    fn time_machine_triggers_once_per_session_only_on_a_ready_straight() {
        let mut state = TimeTravelVisuals::default();
        state.update(true, false, 10.0, 89.0, false, 0.1);
        assert!(!state.active());
        state.update(true, false, 11.0, 87.9, true, 0.1);
        assert!(!state.active());
        state.update(true, false, 12.0, 89.0, true, 0.1);
        assert!(state.active());

        state.update(true, false, 20.0, 92.0, true, TIME_TRAVEL_DURATION_SECS);
        assert!(!state.active());
        state.update(true, false, 21.0, 95.0, true, 1.0);
        assert!(!state.active(), "the same session must not retrigger");

        state.update(false, false, 21.0, 0.0, false, 0.1);
        state.update(true, false, 0.0, 89.0, true, 0.1);
        assert!(state.active(), "a fresh session may trigger again");
    }

    #[test]
    fn time_machine_pass_vanishes_below_the_top_edge_in_a_flash() {
        let start = time_car_pose(0.0);
        let vanish = time_car_pose(0.78);
        let flash = time_car_pose(0.82);
        assert!(start.position.y < CAR_Y);
        assert!(!vanish.car_visible);
        assert!(vanish.position.y < VIEW_TOP);
        assert!(flash.flash > 0.95);
        assert!(flash.scale < start.scale);
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
    fn foreground_road_keeps_widening_toward_the_bottom_edge() {
        let at_row_zero = apron_road_half_width(1.0);
        let middle = apron_road_half_width(0.5);
        let bottom = apron_road_half_width(0.0);
        assert_eq!(at_row_zero, HALF_ROAD_PX);
        assert!(bottom > middle && middle > at_row_zero);
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
            "A/LB LOW | X/RB HIGH"
        );
        assert_eq!(normal_recovery_cue(Gear::Low, 10.0, 10.0), "SHIFT HIGH");
        assert_eq!(
            normal_recovery_cue(Gear::High, 24.0, 10.0),
            "A/LB LOW | X/RB HIGH"
        );
    }

    #[test]
    fn difficulty_menu_board_filters_and_fills_exactly_five_rows() {
        let entries = vec![
            game_core::LeaderboardEntry::new("EZY", game_core::Difficulty::Easy, 2, 12, 1).unwrap(),
            game_core::LeaderboardEntry::new("HRD", game_core::Difficulty::Hard, 8, 50, 0).unwrap(),
        ];
        let table = difficulty_menu_leaderboard_table(&entries, game_core::Difficulty::Easy);
        assert!(table.starts_with("LOCAL TOP 5 — EASY\n"));
        assert!(table.contains("1  EZY"));
        assert!(!table.contains("HRD"));
        assert_eq!(table.lines().count(), 7, "title + header + five ranks");
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
        assert!(storm.fog_alpha > day.fog_alpha);
        assert!(road_visibility(0.0, 1.0).fog_alpha > night.fog_alpha);
        assert_eq!(day.headlights, 0.0);
        assert_eq!(road_visibility(0.70, 0.0).headlights, 0.0);
        assert!(night.headlights > 0.9);
        assert!(storm.headlights > 0.8);
        assert!(night.shade_alpha <= 0.1 && storm.fog_alpha >= 0.65);
        let (_, _, foreground) = fog_band_geometry(0);
        let (_, _, distance) = fog_band_geometry(VISIBILITY_BANDS - 1);
        assert!(distance > foreground * 10.0);
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
    fn headlights_stay_straight_and_fade_by_midfield() {
        let (near_center, near, near_weight) = projected_headlight_geometry(0, -200.0, -180.0);
        let (middle_center, middle, middle_weight) = projected_headlight_geometry(0, -105.0, -85.0);
        let (_, _, far_weight) = projected_headlight_geometry(0, -45.0, -30.0);
        let (_, inner, _) = projected_headlight_geometry(1, -105.0, -85.0);

        assert!(middle.x > near.x);
        assert!(
            middle.y > near.y,
            "headlight bands must deepen into the cone"
        );
        assert!(inner.x < middle.x);
        assert!(middle_weight > near_weight);
        assert_eq!(far_weight, 0.0);
        assert_eq!(near_center.x, 0.0);
        assert_eq!(middle_center.x, 0.0);
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
        assert!(
            (0..4).all(|variant| {
                let [red, green, blue] = traffic_car_rgb(variant);
                blue < red.max(green)
            }),
            "traffic colors must not share the cold-air pickup's blue-dominant silhouette"
        );
    }

    #[test]
    fn pickup_glow_peaks_halfway_and_ends_cleanly() {
        assert!(nitro_pickup_pulse(crate::NITRO_PICKUP_GLOW_SECS) < 1e-6);
        assert!((nitro_pickup_pulse(crate::NITRO_PICKUP_GLOW_SECS * 0.5) - 1.0).abs() < 1e-6);
        assert!(nitro_pickup_pulse(0.0) < 1e-6);
    }

    #[test]
    fn jump_pose_lifts_spins_and_returns_to_rest() {
        let start = jump_pose(0.0);
        let apex = jump_pose(0.5);
        let end = jump_pose(1.0);
        assert_eq!(start.height, 0.0);
        assert!(apex.height > 100.0);
        assert!((apex.sprite_spin - std::f32::consts::PI).abs() < 1e-5);
        let quarter = jump_pose(0.326_351_82); // smoothstep(p) = 0.25
        let rotation = Quat::from_rotation_z(quarter.sprite_spin);
        assert!((rotation * Vec3::X - Vec3::Y).length() < 1e-4);
        assert!(apex.shadow_scale < start.shadow_scale);
        assert!(apex.shadow_alpha < start.shadow_alpha);
        assert_eq!(end.height, 0.0);
        assert_eq!(end.sprite_spin, 0.0);
        assert_eq!(end.scale, Vec3::ONE);
    }

    #[test]
    fn cold_air_crest_hop_is_shorter_and_does_not_barrel_roll() {
        let crest = cold_air_hop_pose(0.5);
        assert!(crest.height > 40.0);
        assert!(crest.height < jump_pose(0.5).height * 0.5);
        assert_eq!(crest.sprite_spin, 0.0);
        assert!(crest.shadow_scale < 1.0);
        assert_eq!(cold_air_hop_pose(1.0), jump_pose(0.0));
    }

    #[test]
    fn launch_drop_lands_before_its_twin_jets_taper_out() {
        let (start_height, start_jet) = launch_drop_pose(0.0);
        let (landing_height, landing_jet) = launch_drop_pose(0.82);
        let (end_height, end_jet) = launch_drop_pose(1.0);
        assert!(start_height > VIEW_TOP - CAR_Y);
        assert_eq!(start_jet, 1.0);
        assert_eq!(landing_height, 0.0);
        assert!(landing_jet > 0.0);
        assert_eq!(end_height, 0.0);
        assert_eq!(end_jet, 0.0);
    }

    #[test]
    fn smoothstep_clamps_and_eases_transition() {
        assert_eq!(smoothstep(-1.0), 0.0);
        assert_eq!(smoothstep(0.0), 0.0);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-6);
        assert_eq!(smoothstep(1.0), 1.0);
        assert_eq!(smoothstep(2.0), 1.0);
    }

    #[test]
    fn finish_road_becomes_constant_width_and_covers_the_screen() {
        let unchanged = finish_road_band(24.0, 80.0, -250.0, -220.0, 0, 0.0);
        assert_eq!(unchanged.center, 24.0);
        assert_eq!(unchanged.half_width, 80.0);
        assert_eq!(unchanged.y0, -250.0);
        assert_eq!(unchanged.y1, -220.0);

        let first = finish_road_band(24.0, 80.0, -250.0, -220.0, 0, 1.0);
        let second = finish_road_band(16.0, 62.0, -220.0, -190.0, 1, 1.0);
        let last = finish_road_band(-31.0, 22.0, 100.0, HORIZON_Y, FINISH_ROAD_BANDS - 1, 1.0);
        assert_eq!(first.center, 0.0);
        assert_eq!(first.half_width, FINISH_ROAD_HALF_WIDTH);
        assert_eq!(second.half_width, FINISH_ROAD_HALF_WIDTH);
        assert_eq!(first.y0, -VIEW_TOP);
        assert!((first.y1 - second.y0).abs() < f32::EPSILON);
        assert_eq!(last.y1, VIEW_TOP);
    }

    #[test]
    fn finish_sequence_lifts_morphs_then_thrusts_upward() {
        let start = finish_pose(0.0);
        let lifted = finish_pose(0.50);
        let flight = finish_pose(0.75);
        let end = finish_pose(1.0);

        assert_eq!(start.road_lift, 0.0);
        assert_eq!(start.rear_scale_y, 1.0);
        assert_eq!(start.top_scale_y, 0.0);
        assert_eq!(start.car_y, CAR_Y);
        assert_eq!(start.jet, 0.0);
        assert_eq!(lifted.road_lift, 1.0);
        assert!(lifted.rear_scale_y < 1.0);
        assert!(flight.top_scale_y > 0.99);
        assert!(flight.car_y > CAR_Y);
        assert!(flight.jet > 0.99);
        assert!(end.car_y > VIEW_TOP);
        assert_eq!(end.jet, 0.0);
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
    fn speed_lines_stay_near_the_car_and_scale_with_intensity() {
        let (far_left, _, left_angle) = speed_line_geometry(0, 0.0, 1.0, 0.0);
        let (far_right, _, right_angle) = speed_line_geometry(1, -0.618_034 / 1.46, 1.0, 0.0);
        assert!(far_left.x < 0.0 && far_right.x > 0.0);
        assert!(left_angle < 0.0 && right_angle > 0.0);
        assert!((left_angle + right_angle).abs() < 1e-6);

        for index in 0..SPEED_LINE_POOL {
            for frame in 0..30 {
                let (position, size, _) = speed_line_geometry(index, frame as f32 / 30.0, 1.0, 0.0);
                assert!(position.y < CAR_Y + 155.0, "line reached too far forward");
                assert!(position.y > -300.0, "line escaped below the screen");
                assert!(size.y <= 58.0, "streak is too long: {}", size.y);
            }
        }
        let (_, faint, _) = speed_line_geometry(3, 1.25, 0.1, 0.0);
        let (_, strong, _) = speed_line_geometry(3, 1.25, 1.0, 0.0);
        assert!(strong.y > faint.y);
    }
}
