//! Deterministic pixel-art horizon scenery.
//!
//! Each scene is a small pool of flat rectangles. Runtime only changes color,
//! alpha, and visibility, so seasonal variety adds no asset loads or spawning
//! churn during play.

use bevy::prelude::*;

use crate::Sim;

use super::{WeatherVisuals, HORIZON_Y};

const SCENE_COUNT: usize = 6;
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

fn featured_scene(seed: u64, day: u64, night: bool) -> BackdropKind {
    let season = (day / 3 % 4) as usize;
    let roll = hash(seed, day, season as u64 + u64::from(night) * 11) % 100;
    match (season, night, roll) {
        (0, false, 0..=69) => BackdropKind::Sakura,
        (0, false, 70..=89) => BackdropKind::Mountains,
        (0, false, _) => BackdropKind::City,
        (1, false, 0..=44) => BackdropKind::Forest,
        (1, false, 45..=84) => BackdropKind::City,
        (1, false, _) => BackdropKind::Mountains,
        (2, false, 0..=64) => BackdropKind::Forest,
        (2, false, 65..=89) => BackdropKind::Mountains,
        (2, false, _) => BackdropKind::City,
        (3, false, 0..=89) => BackdropKind::SnowMountains,
        (3, false, _) => BackdropKind::City,
        (3, true, 0..=59) => BackdropKind::SnowMountains,
        (_, true, 0..=69) => BackdropKind::Neon,
        (_, true, 70..=89) => BackdropKind::City,
        (_, true, _) => BackdropKind::Mountains,
        _ => BackdropKind::Mountains,
    }
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn scene_weights(seed: u64, environment_time: f32) -> [f32; SCENE_COUNT] {
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
        let previous = featured_scene(seed, previous_day, night) as usize;
        let current = featured_scene(seed, day, night) as usize;
        weights[previous] += light_weight * (1.0 - scene_blend);
        weights[current] += light_weight * scene_blend;
    }
    weights
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
}

pub(super) fn sync(
    sim: Res<Sim>,
    weather: Res<WeatherVisuals>,
    time: Res<Time>,
    mut pixels: Query<(&BackdropPixel, &mut Sprite, &mut Visibility)>,
) {
    let daylight = game_core::environment::daylight(sim.0.environment_time);
    let weights = scene_weights(sim.0.seed, sim.0.environment_time);
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
        let base = pixel.base.to_linear();
        sprite.color = Color::LinearRgba(LinearRgba::new(
            base.red * brightness * twinkle,
            base.green * brightness * twinkle,
            base.blue * brightness * twinkle,
            weight.clamp(0.0, 1.0),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_weights_are_normalized_and_continuous_at_day_boundaries() {
        for time in [0.0, 25.0, 75.0, 99.99, 100.01, 305.0, 925.0] {
            let sum: f32 = scene_weights(42, time).iter().sum();
            assert!((sum - 1.0).abs() < 1e-5);
        }
        let before = scene_weights(42, game_core::environment::DAY_SECS - 0.01);
        let after = scene_weights(42, game_core::environment::DAY_SECS + 0.01);
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
                featured_scene(9, day, false),
                BackdropKind::Sakura | BackdropKind::Mountains | BackdropKind::City
            ));
        }
        for day in 9..12 {
            assert!(matches!(
                featured_scene(9, day, false),
                BackdropKind::SnowMountains | BackdropKind::City
            ));
            assert!(matches!(
                featured_scene(9, day, true),
                BackdropKind::SnowMountains
                    | BackdropKind::Neon
                    | BackdropKind::City
                    | BackdropKind::Mountains
            ));
        }
        for day in 3..9 {
            assert!(!matches!(
                featured_scene(9, day, true),
                BackdropKind::Sakura | BackdropKind::Forest
            ));
        }
    }
}
