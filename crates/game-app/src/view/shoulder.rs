//! Lightweight seasonal details projected along the road shoulders.

use bevy::prelude::*;

use crate::Sim;

use super::{Proj, WeatherVisuals, HALF_ROAD_PX};

const ACCENT_POOL: usize = 28;

#[derive(Component)]
pub(super) struct ShoulderAccent(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccentKind {
    Hidden,
    Blossom,
    AutumnLeaf,
    Snowbank,
    Puddle,
}

#[derive(Debug, Clone, Copy)]
struct AccentLook {
    kind: AccentKind,
    color: Color,
    size: Vec2,
    alpha: f32,
}

fn accent_look(seasons: [f32; 4], rain: f32) -> AccentLook {
    let spring = seasons[0];
    let autumn = seasons[2];
    let winter = seasons[3];
    let rain = rain.clamp(0.0, 1.0) * (1.0 - winter);
    if winter >= spring.max(autumn).max(rain) && winter > 0.08 {
        AccentLook {
            kind: AccentKind::Snowbank,
            color: Color::srgb(0.88, 0.93, 1.0),
            size: Vec2::new(48.0, 8.0),
            alpha: winter * 0.92,
        }
    } else if rain >= spring.max(autumn) && rain > 0.18 {
        AccentLook {
            kind: AccentKind::Puddle,
            color: Color::srgb(0.30, 0.55, 0.70),
            size: Vec2::new(42.0, 4.0),
            alpha: rain * 0.48,
        }
    } else if spring >= autumn && spring > 0.20 {
        AccentLook {
            kind: AccentKind::Blossom,
            color: Color::srgb(1.0, 0.55, 0.74),
            size: Vec2::new(18.0, 6.0),
            alpha: spring * 0.70,
        }
    } else if autumn > 0.20 {
        AccentLook {
            kind: AccentKind::AutumnLeaf,
            color: Color::srgb(0.94, 0.42, 0.08),
            size: Vec2::new(20.0, 5.0),
            alpha: autumn * 0.76,
        }
    } else {
        AccentLook {
            kind: AccentKind::Hidden,
            color: Color::NONE,
            size: Vec2::ZERO,
            alpha: 0.0,
        }
    }
}

pub(super) fn spawn(commands: &mut Commands) {
    for index in 0..ACCENT_POOL {
        commands.spawn((
            Sprite::from_color(Color::NONE, Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, -2.64),
            Visibility::Hidden,
            ShoulderAccent(index),
        ));
    }
}

pub(super) fn sync(
    sim: Res<Sim>,
    proj: Res<Proj>,
    weather: Res<WeatherVisuals>,
    time: Res<Time>,
    mut accents: Query<(
        &ShoulderAccent,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    let look = accent_look(
        game_core::environment::season_weights(sim.0.environment_time),
        weather.rain_visible,
    );
    if look.kind == AccentKind::Hidden {
        for (_, _, _, mut visibility) in &mut accents {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    let loop_position = sim.0.loop_pos();
    let loop_length = sim.0.road.length;
    for (accent, mut transform, mut sprite, mut visibility) in &mut accents {
        let index = accent.0;
        let road_position = (index as f32 + 0.37) / ACCENT_POOL as f32 * loop_length;
        let ahead = (road_position - loop_position).rem_euclid(loop_length);
        let Some((y, scale, lateral)) = proj.project(ahead, sim.0.road_x) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let side = if index.is_multiple_of(2) { -1.0 } else { 1.0 };
        transform.translation.x =
            (lateral + side * (1.08 + (index % 3) as f32 * 0.045)) * HALF_ROAD_PX * scale;
        transform.translation.y = y + look.size.y * scale * 0.35;
        transform.rotation = Quat::from_rotation_z(match look.kind {
            AccentKind::Blossom | AccentKind::AutumnLeaf => {
                side * (0.08 + (time.elapsed_secs() * 1.7 + index as f32).sin() * 0.04)
            }
            _ => 0.0,
        });
        sprite.custom_size = Some(Vec2::new(
            (look.size.x * scale).max(2.0),
            (look.size.y * scale).max(1.5),
        ));
        let glint = if look.kind == AccentKind::Puddle {
            0.72 + 0.28 * (time.elapsed_secs() * 2.4 + index as f32).sin().abs()
        } else {
            1.0
        };
        sprite.color = look.color.with_alpha(look.alpha * glint);
        *visibility = Visibility::Visible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shoulder_accents_make_seasons_and_rain_legible() {
        assert_eq!(
            accent_look([1.0, 0.0, 0.0, 0.0], 0.0).kind,
            AccentKind::Blossom
        );
        assert_eq!(
            accent_look([0.0, 1.0, 0.0, 0.0], 0.0).kind,
            AccentKind::Hidden
        );
        assert_eq!(
            accent_look([0.0, 0.0, 1.0, 0.0], 0.0).kind,
            AccentKind::AutumnLeaf
        );
        assert_eq!(
            accent_look([0.0, 0.0, 0.0, 1.0], 1.0).kind,
            AccentKind::Snowbank
        );
        assert_eq!(
            accent_look([0.0, 1.0, 0.0, 0.0], 1.0).kind,
            AccentKind::Puddle
        );
    }
}
