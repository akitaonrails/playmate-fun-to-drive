//! Deterministic pixel-art horizon scenery.
//!
//! Each scene is a small pool of flat rectangles. Runtime only changes color,
//! alpha, and visibility, so seasonal variety adds no asset loads or spawning
//! churn during play.

use bevy::prelude::*;

use crate::{SceneryRun, Sim};

use super::{WeatherVisuals, HORIZON_Y};

const SCENE_COUNT: usize = 8;
const SCENE_BLEND_FRACTION: f32 = 0.12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum BackdropKind {
    Mountains,
    City,
    Neon,
    Forest,
    SnowMountains,
    Sakura,
    Tokyo,
    Paris,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RoadsideTheme {
    OpenRoad,
    City,
    Forest,
    Sakura,
    Winter,
}

#[derive(Component)]
pub(super) struct BackdropPixel {
    kind: BackdropKind,
    base: Color,
    glow: bool,
    phase: f32,
}

fn hash(seed: u64, day: u64, salt: u64) -> u64 {
    let mut value = seed
        .wrapping_add(day.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(salt.wrapping_mul(0xD1B5_4A32_D192_ED03));
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value.wrapping_mul(0x94D0_49BB_1331_11EB) ^ (value >> 31)
}

fn featured_scene(scenery: SceneryRun, day: u64, night: bool) -> BackdropKind {
    let season = (day / 3 % 4) as usize;
    let city_themes = [
        BackdropKind::Tokyo,
        BackdropKind::Paris,
        BackdropKind::Neon,
        BackdropKind::City,
    ];
    // A run owns one city identity. Serial rotation guarantees that adjacent
    // fresh runs differ, while entropy randomizes where the rotation starts.
    let city = city_themes[(hash(scenery.entropy, 0, 19).wrapping_add(scenery.serial)
        % city_themes.len() as u64) as usize];
    // Always reveal that identity at the opening dawn; later day boundaries
    // may introduce season-appropriate nature before returning to the same city.
    if day == 0 {
        return city;
    }
    let roll = hash(
        scenery
            .entropy
            .wrapping_add(scenery.serial.wrapping_mul(0xA076_1D64_78BD_642F)),
        day,
        season as u64 + u64::from(night) * 11,
    ) % 100;
    match (season, night, roll) {
        (0, false, 0..=44) => BackdropKind::Sakura,
        (0, false, 45..=59) => BackdropKind::Mountains,
        (1, false, 0..=24) => BackdropKind::Forest,
        (1, false, 25..=34) => BackdropKind::Mountains,
        (2, false, 0..=34) => BackdropKind::Forest,
        (2, false, 35..=49) => BackdropKind::Mountains,
        (3, false, 0..=64) => BackdropKind::SnowMountains,
        (3, true, 0..=34) => BackdropKind::SnowMountains,
        (_, true, 0..=9) => BackdropKind::Mountains,
        _ => city,
    }
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn scene_weights(scenery: SceneryRun, environment_time: f32) -> [f32; SCENE_COUNT] {
    let day_length = game_core::environment::DAY_SECS;
    let day = (environment_time / day_length).floor().max(0.0) as u64;
    let phase = environment_time.rem_euclid(day_length) / day_length;
    let previous_day = day.saturating_sub(1);
    let scene_blend = smoothstep(phase / SCENE_BLEND_FRACTION);
    let daylight = game_core::environment::daylight(environment_time);
    let day_mix = smoothstep((daylight - 0.18) / 0.52);
    let night_mix = 1.0 - day_mix;
    let mut weights = [0.0; SCENE_COUNT];

    for (night, light_weight) in [(false, day_mix), (true, night_mix)] {
        let previous = featured_scene(scenery, previous_day, night) as usize;
        let current = featured_scene(scenery, day, night) as usize;
        weights[previous] += light_weight * (1.0 - scene_blend);
        weights[current] += light_weight * scene_blend;
    }
    weights
}

pub(super) fn roadside_theme(scenery: SceneryRun, environment_time: f32) -> RoadsideTheme {
    let weights = scene_weights(scenery, environment_time);
    let dominant = weights
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(right.1))
        .map_or(BackdropKind::Mountains, |(index, _)| match index {
            0 => BackdropKind::Mountains,
            1 => BackdropKind::City,
            2 => BackdropKind::Neon,
            3 => BackdropKind::Forest,
            4 => BackdropKind::SnowMountains,
            5 => BackdropKind::Sakura,
            6 => BackdropKind::Tokyo,
            _ => BackdropKind::Paris,
        });
    match dominant {
        BackdropKind::City | BackdropKind::Neon | BackdropKind::Tokyo | BackdropKind::Paris => {
            RoadsideTheme::City
        }
        BackdropKind::Forest => RoadsideTheme::Forest,
        BackdropKind::SnowMountains => RoadsideTheme::Winter,
        BackdropKind::Sakura => RoadsideTheme::Sakura,
        BackdropKind::Mountains => RoadsideTheme::OpenRoad,
    }
}

#[allow(clippy::too_many_arguments)]
fn pixel(
    commands: &mut Commands,
    kind: BackdropKind,
    color: Color,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    depth: f32,
    glow: bool,
) {
    let z = if kind == BackdropKind::Neon {
        0.115 + depth * 0.002
    } else {
        -4.68 + depth * 0.01
    };
    commands.spawn((
        Sprite::from_color(color.with_alpha(0.0), Vec2::new(width, height)),
        Transform::from_xyz(x.round(), y.round(), z),
        Visibility::Hidden,
        BackdropPixel {
            kind,
            base: color,
            glow,
            phase: (x * 0.037 + y * 0.019).rem_euclid(std::f32::consts::TAU),
        },
    ));
}

fn mountain_height(x: f32, peaks: &[(f32, f32, f32)]) -> f32 {
    peaks
        .iter()
        .map(|(center, height, slope)| (height - (x - center).abs() * slope).max(12.0))
        .fold(12.0, f32::max)
}

fn spawn_mountains(commands: &mut Commands, kind: BackdropKind, snowy: bool) {
    let peaks = [
        (-330.0, 76.0, 0.24),
        (-70.0, 116.0, 0.30),
        (280.0, 88.0, 0.25),
    ];
    let back_color = if snowy {
        Color::srgb(0.48, 0.60, 0.75)
    } else {
        Color::srgb(0.20, 0.30, 0.47)
    };
    let front_color = if snowy {
        Color::srgb(0.34, 0.45, 0.61)
    } else {
        Color::srgb(0.12, 0.22, 0.35)
    };
    for layer in 0..2 {
        let width = if layer == 0 { 40.0 } else { 32.0 };
        let count = (960.0 / width) as usize + 1;
        for index in 0..count {
            let x = -480.0 + width * (index as f32 + 0.5);
            let offset = if layer == 0 { 42.0 } else { 0.0 };
            let height = mountain_height(x + offset, &peaks) * (0.72 + layer as f32 * 0.28);
            let color = if layer == 0 { back_color } else { front_color };
            pixel(
                commands,
                kind,
                color,
                x,
                HORIZON_Y + height * 0.5,
                width + 1.0,
                height,
                layer as f32,
                false,
            );
            if snowy && layer == 1 && height > 54.0 && index.is_multiple_of(2) {
                let cap = (height * 0.22).clamp(10.0, 22.0);
                pixel(
                    commands,
                    kind,
                    Color::srgb(0.90, 0.94, 1.0),
                    x,
                    HORIZON_Y + height - cap * 0.5,
                    width + 1.0,
                    cap,
                    1.5,
                    false,
                );
            }
        }
    }
}

fn spawn_city(commands: &mut Commands, neon: bool) {
    let kind = if neon {
        BackdropKind::Neon
    } else {
        BackdropKind::City
    };
    for index in 0..16 {
        let width = 42.0 + (index * 17 % 25) as f32;
        let x = -455.0 + index as f32 * 61.0;
        let height = 34.0 + (index * 31 % 82) as f32;
        let body = if neon {
            Color::srgb(0.025, 0.035, 0.09)
        } else {
            Color::srgb(0.10 + (index % 3) as f32 * 0.025, 0.14, 0.22)
        };
        pixel(
            commands,
            kind,
            body,
            x,
            HORIZON_Y + height * 0.5,
            width,
            height,
            1.0,
            false,
        );
        let light = if neon {
            match index % 3 {
                0 => Color::srgb(1.0, 0.12, 0.66),
                1 => Color::srgb(0.08, 0.92, 1.0),
                _ => Color::srgb(1.0, 0.72, 0.08),
            }
        } else {
            Color::srgb(1.0, 0.76, 0.28)
        };
        pixel(
            commands,
            kind,
            light,
            x,
            HORIZON_Y + height * 0.62,
            width - 9.0,
            if neon { 5.0 } else { 3.0 },
            2.0,
            neon,
        );
        if neon && index % 2 == 0 {
            pixel(
                commands,
                kind,
                light,
                x + width * 0.34,
                HORIZON_Y + height * 0.80,
                5.0,
                14.0,
                2.2,
                true,
            );
        }
    }
}

fn spawn_tokyo(commands: &mut Commands) {
    let kind = BackdropKind::Tokyo;
    // Dense low/mid-rise blocks leave the vanishing point readable while
    // vertical signs and the red-white lattice landmark establish identity.
    for index in 0..15 {
        let width = 38.0 + (index * 13 % 22) as f32;
        let x = -452.0 + index as f32 * 64.0;
        let height = 42.0 + (index * 29 % 78) as f32;
        let body = match index % 3 {
            0 => Color::srgb(0.08, 0.13, 0.22),
            1 => Color::srgb(0.12, 0.19, 0.29),
            _ => Color::srgb(0.17, 0.24, 0.32),
        };
        pixel(
            commands,
            kind,
            body,
            x,
            HORIZON_Y + height * 0.5,
            width,
            height,
            1.0,
            false,
        );
        for row in 0..3 {
            let light = match (index + row) % 3 {
                0 => Color::srgb(0.08, 0.86, 0.94),
                1 => Color::srgb(0.95, 0.20, 0.62),
                _ => Color::srgb(1.0, 0.68, 0.18),
            };
            pixel(
                commands,
                kind,
                light,
                x - width * 0.16 + row as f32 * width * 0.16,
                HORIZON_Y + 18.0 + row as f32 * 15.0,
                5.0,
                5.0,
                2.0,
                true,
            );
        }
        if index % 3 == 1 {
            pixel(
                commands,
                kind,
                Color::srgb(0.08, 0.86, 0.94),
                x + width * 0.36,
                HORIZON_Y + height * 0.64,
                5.0,
                19.0,
                2.3,
                true,
            );
        }
    }
    // Elevated rail/bay strip.
    pixel(
        commands,
        kind,
        Color::srgb(0.12, 0.20, 0.29),
        0.0,
        HORIZON_Y + 13.0,
        960.0,
        7.0,
        1.8,
        false,
    );
    for index in 0..5 {
        let width = 58.0 - index as f32 * 9.0;
        let height = 15.0;
        pixel(
            commands,
            kind,
            if index % 2 == 0 {
                Color::srgb(0.88, 0.12, 0.16)
            } else {
                Color::srgb(0.88, 0.88, 0.80)
            },
            -252.0,
            HORIZON_Y + 27.0 + index as f32 * height,
            width,
            height + 1.0,
            2.5,
            index % 2 == 0,
        );
    }
    pixel(
        commands,
        kind,
        Color::srgb(0.92, 0.18, 0.20),
        -252.0,
        HORIZON_Y + 110.0,
        5.0,
        34.0,
        2.6,
        true,
    );
}

fn spawn_paris(commands: &mut Commands) {
    let kind = BackdropKind::Paris;
    // Low continuous Haussmann-like blocks and zinc roofs contrast with the
    // narrow Tokyo towers without requiring textures or curved geometry.
    for index in 0..14 {
        let width = 66.0 + (index % 3) as f32 * 5.0;
        let x = -455.0 + index as f32 * 70.0;
        let height = 43.0 + (index * 7 % 18) as f32;
        pixel(
            commands,
            kind,
            if index % 2 == 0 {
                Color::srgb(0.62, 0.54, 0.40)
            } else {
                Color::srgb(0.72, 0.63, 0.47)
            },
            x,
            HORIZON_Y + height * 0.5,
            width,
            height,
            1.0,
            false,
        );
        pixel(
            commands,
            kind,
            Color::srgb(0.25, 0.31, 0.39),
            x,
            HORIZON_Y + height + 7.0,
            width + 3.0,
            14.0,
            1.4,
            false,
        );
        for column in [-1.0, 0.0, 1.0] {
            pixel(
                commands,
                kind,
                Color::srgb(1.0, 0.70, 0.34),
                x + column * 17.0,
                HORIZON_Y + 24.0 + (index % 2) as f32 * 8.0,
                5.0,
                7.0,
                2.0,
                true,
            );
        }
    }
    // Simplified Eiffel-like lattice silhouette, offset from the road center.
    for (x, rotation_side) in [(-286.0, -1.0), (-240.0, 1.0)] {
        for level in 0..5 {
            pixel(
                commands,
                kind,
                Color::srgb(0.30, 0.23, 0.19),
                x + rotation_side * level as f32 * 4.0,
                HORIZON_Y + 14.0 + level as f32 * 14.0,
                (9.0 - level as f32).max(4.0),
                17.0,
                2.3,
                false,
            );
        }
    }
    for (level, width) in [58.0, 45.0, 32.0, 20.0].into_iter().enumerate() {
        pixel(
            commands,
            kind,
            Color::srgb(0.95, 0.62, 0.28),
            -263.0,
            HORIZON_Y + 20.0 + level as f32 * 19.0,
            width,
            5.0,
            2.5,
            true,
        );
    }
    pixel(
        commands,
        kind,
        Color::srgb(0.36, 0.29, 0.22),
        -263.0,
        HORIZON_Y + 104.0,
        5.0,
        45.0,
        2.4,
        false,
    );
}

fn spawn_forest(commands: &mut Commands) {
    for index in 0..20 {
        let x = -468.0 + index as f32 * 49.0;
        let height = 34.0 + (index * 19 % 45) as f32;
        pixel(
            commands,
            BackdropKind::Forest,
            Color::srgb(0.20, 0.12, 0.08),
            x,
            HORIZON_Y + height * 0.25,
            6.0,
            height * 0.5,
            1.0,
            false,
        );
        pixel(
            commands,
            BackdropKind::Forest,
            if index % 3 == 0 {
                Color::srgb(0.16, 0.34, 0.18)
            } else {
                Color::srgb(0.09, 0.27, 0.16)
            },
            x,
            HORIZON_Y + height * 0.68,
            25.0 + (index % 4) as f32 * 4.0,
            height * 0.64,
            1.5,
            false,
        );
    }
}

fn spawn_sakura(commands: &mut Commands) {
    for index in 0..15 {
        let x = -455.0 + index as f32 * 65.0;
        let height = 34.0 + (index * 23 % 38) as f32;
        pixel(
            commands,
            BackdropKind::Sakura,
            Color::srgb(0.30, 0.16, 0.16),
            x,
            HORIZON_Y + height * 0.28,
            6.0,
            height * 0.56,
            1.0,
            false,
        );
        for (offset, width, color) in [
            (-10.0, 30.0, Color::srgb(0.96, 0.48, 0.68)),
            (11.0, 34.0, Color::srgb(1.0, 0.64, 0.78)),
        ] {
            pixel(
                commands,
                BackdropKind::Sakura,
                color,
                x + offset,
                HORIZON_Y + height * 0.78 + offset.abs() * 0.18,
                width,
                20.0 + (index % 3) as f32 * 4.0,
                1.5,
                false,
            );
        }
    }
}

pub(super) fn spawn(commands: &mut Commands) {
    spawn_mountains(commands, BackdropKind::Mountains, false);
    spawn_city(commands, false);
    spawn_city(commands, true);
    spawn_forest(commands);
    spawn_mountains(commands, BackdropKind::SnowMountains, true);
    spawn_sakura(commands);
    spawn_tokyo(commands);
    spawn_paris(commands);
}

pub(super) fn sync(
    sim: Res<Sim>,
    scenery: Res<SceneryRun>,
    weather: Res<WeatherVisuals>,
    time: Res<Time>,
    mut pixels: Query<(&BackdropPixel, &mut Sprite, &mut Visibility)>,
) {
    let daylight = game_core::environment::daylight(sim.0.environment_time);
    let seasons = game_core::environment::season_weights(sim.0.environment_time);
    let weights = scene_weights(*scenery, sim.0.environment_time);
    let weather_visibility = 1.0 - weather.cloudy * 0.12 - weather.rain_visible * 0.28;
    for (pixel, mut sprite, mut visibility) in &mut pixels {
        let weight = weights[pixel.kind as usize] * weather_visibility;
        if weight < 0.003 {
            *visibility = Visibility::Hidden;
            continue;
        }

        *visibility = Visibility::Visible;
        let twinkle = if pixel.glow {
            0.78 + 0.22 * (time.elapsed_secs() * 3.2 + pixel.phase).sin().abs()
        } else {
            1.0
        };
        let brightness = match pixel.kind {
            BackdropKind::Neon => 0.88 + (1.0 - daylight) * 0.32,
            BackdropKind::SnowMountains => 0.42 + daylight * 0.58,
            _ => 0.25 + daylight * 0.75,
        };
        let brightness = if pixel.glow {
            brightness.max(0.30 + (1.0 - daylight) * 0.82 + weather.rain_visible * 0.12)
        } else {
            brightness
        };
        let base = pixel.base.to_linear();
        // Gentle seasonal grading keeps persistent city geometry coherent:
        // spring is rosier, autumn warmer, and winter cooler/desaturated.
        let red = 1.0 + seasons[0] * 0.05 + seasons[2] * 0.12 - seasons[3] * 0.06;
        let green = 1.0 + seasons[1] * 0.05 + seasons[2] * 0.03 - seasons[3] * 0.04;
        let blue = 1.0 + seasons[3] * 0.16 + seasons[0] * 0.03;
        sprite.color = Color::LinearRgba(LinearRgba::new(
            base.red * brightness * twinkle * red,
            base.green * brightness * twinkle * green,
            base.blue * brightness * twinkle * blue,
            weight.clamp(0.0, 1.0),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SCENERY: SceneryRun = SceneryRun {
        entropy: 42,
        serial: 0,
    };

    #[test]
    fn scene_weights_are_normalized_and_continuous_at_day_boundaries() {
        for time in [0.0, 25.0, 75.0, 99.99, 100.01, 305.0, 925.0] {
            let sum: f32 = scene_weights(TEST_SCENERY, time).iter().sum();
            assert!((sum - 1.0).abs() < 1e-5);
        }
        let before = scene_weights(TEST_SCENERY, game_core::environment::DAY_SECS - 0.01);
        let after = scene_weights(TEST_SCENERY, game_core::environment::DAY_SECS + 0.01);
        let delta: f32 = before
            .iter()
            .zip(after)
            .map(|(left, right)| (left - right).abs())
            .sum();
        assert!(delta < 0.02, "daily scenery must crossfade without popping");
    }

    #[test]
    fn scenery_selection_respects_season_and_night_vocabulary() {
        for day in 0..3 {
            assert!(matches!(
                featured_scene(TEST_SCENERY, day, false),
                BackdropKind::Sakura
                    | BackdropKind::Mountains
                    | BackdropKind::Tokyo
                    | BackdropKind::Paris
                    | BackdropKind::City
                    | BackdropKind::Neon
            ));
        }
        for day in 9..12 {
            assert!(matches!(
                featured_scene(TEST_SCENERY, day, false),
                BackdropKind::SnowMountains
                    | BackdropKind::Tokyo
                    | BackdropKind::Paris
                    | BackdropKind::City
                    | BackdropKind::Neon
            ));
            assert!(matches!(
                featured_scene(TEST_SCENERY, day, true),
                BackdropKind::SnowMountains
                    | BackdropKind::Neon
                    | BackdropKind::Tokyo
                    | BackdropKind::Paris
                    | BackdropKind::City
            ));
        }
        for day in 3..9 {
            assert!(!matches!(
                featured_scene(TEST_SCENERY, day, true),
                BackdropKind::Sakura | BackdropKind::Forest
            ));
        }
    }

    #[test]
    fn adjacent_fresh_runs_do_not_repeat_the_opening_scenery() {
        for serial in 0..16 {
            let first = SceneryRun {
                entropy: 777,
                serial,
            };
            let next = SceneryRun {
                serial: serial + 1,
                ..first
            };
            assert_ne!(
                featured_scene(first, 0, false),
                featured_scene(next, 0, false)
            );
        }
    }

    #[test]
    fn opening_city_identity_is_stable_across_day_and_night() {
        for serial in 0..8 {
            let scenery = SceneryRun {
                entropy: 456,
                serial,
            };
            assert_eq!(
                featured_scene(scenery, 0, false),
                featured_scene(scenery, 0, true)
            );
        }
    }

    #[test]
    fn tokyo_and_paris_are_reachable_opening_scenes() {
        let mut seen_tokyo = false;
        let mut seen_paris = false;
        for serial in 0..16 {
            let scenery = SceneryRun {
                entropy: 123,
                serial,
            };
            seen_tokyo |= featured_scene(scenery, 0, false) == BackdropKind::Tokyo;
            seen_paris |= featured_scene(scenery, 0, false) == BackdropKind::Paris;
        }
        assert!(seen_tokyo && seen_paris);
    }

    #[test]
    fn roadside_theme_follows_the_dominant_backdrop() {
        for serial in 0..4 {
            let scenery = SceneryRun {
                entropy: 42,
                serial,
            };
            assert_eq!(roadside_theme(scenery, 50.0), RoadsideTheme::City);
        }

        let mut saw_forest = false;
        let mut saw_sakura = false;
        let mut saw_winter = false;
        for entropy in 0..256 {
            let scenery = SceneryRun { entropy, serial: 0 };
            saw_sakura |= roadside_theme(scenery, 150.0) == RoadsideTheme::Sakura;
            saw_forest |= roadside_theme(scenery, 350.0) == RoadsideTheme::Forest;
            saw_winter |= roadside_theme(scenery, 1_050.0) == RoadsideTheme::Winter;
        }
        assert!(saw_sakura && saw_forest && saw_winter);
    }
}
