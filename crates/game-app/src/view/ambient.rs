//! Rare, deterministic sky easter eggs.
//!
//! The module exposes only installation and synchronization; scheduling,
//! eligibility, pooling, and animation details stay local and unit-tested.

use bevy::prelude::*;
use game_core::Phase;

use super::{Proj, HALF_ROAD_PX, TRAFFIC_CAR_WIDTH_PX};
use crate::{Sim, TuningRes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AmbientKind {
    Birds,
    ShootingStar,
    Hurricane,
    HurricaneLeaves,
    HurricaneTree,
    SakuraPetals,
    Ufo,
    HelpedCar,
}

#[derive(Component)]
pub(super) struct AmbientPart {
    kind: AmbientKind,
    index: usize,
}

pub(super) fn spawn(commands: &mut Commands) {
    for (kind, count) in [
        (AmbientKind::Birds, 10),
        (AmbientKind::ShootingStar, 2),
        (AmbientKind::Hurricane, 10),
        (AmbientKind::HurricaneLeaves, 14),
        (AmbientKind::HurricaneTree, 6),
        (AmbientKind::SakuraPetals, 18),
        (AmbientKind::Ufo, 4),
        (AmbientKind::HelpedCar, 2),
    ] {
        for index in 0..count {
            let depth = ambient_depth(kind);
            commands.spawn((
                Sprite::from_color(Color::WHITE, Vec2::ONE),
                Transform::from_xyz(0.0, 0.0, depth),
                Visibility::Hidden,
                AmbientPart { kind, index },
            ));
        }
    }
}

fn ambient_depth(kind: AmbientKind) -> f32 {
    match kind {
        // The funnel crosses the projected road, so it must share the effects
        // stack to remain visible; debris and abducted cars stay just above it.
        AmbientKind::Hurricane => 0.56,
        AmbientKind::HurricaneLeaves
        | AmbientKind::HurricaneTree
        | AmbientKind::SakuraPetals
        | AmbientKind::HelpedCar => 0.62,
        _ => -4.62,
    }
}

fn ambient_kind(kind: game_core::RareEventKind) -> AmbientKind {
    match kind {
        game_core::RareEventKind::Birds => AmbientKind::Birds,
        game_core::RareEventKind::ShootingStar => AmbientKind::ShootingStar,
        game_core::RareEventKind::Hurricane => AmbientKind::Hurricane,
        game_core::RareEventKind::SakuraPetals => AmbientKind::SakuraPetals,
        game_core::RareEventKind::Ufo => AmbientKind::Ufo,
    }
}

fn participates(part: AmbientKind, event: AmbientKind) -> bool {
    part == event
        || (part == AmbientKind::HurricaneLeaves && event == AmbientKind::Hurricane)
        || (part == AmbientKind::HurricaneTree && event == AmbientKind::Hurricane)
        || (part == AmbientKind::HelpedCar
            && matches!(event, AmbientKind::Hurricane | AmbientKind::Ufo))
}

pub(super) fn sync(
    sim: Res<Sim>,
    tuning: Res<TuningRes>,
    proj: Res<Proj>,
    time: Res<Time>,
    mut parts: Query<(&AmbientPart, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let seasons = game_core::environment::season_weights(sim.0.environment_time);
    let helped = sim.0.rare_traffic_help(&tuning.0);
    let event = matches!(sim.0.phase, Phase::Running)
        .then(|| sim.0.rare_event())
        .flatten();

    for (part, mut tf, mut sprite, mut visibility) in &mut parts {
        let Some(event) = event.filter(|event| participates(part.kind, ambient_kind(event.kind)))
        else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let kind = ambient_kind(event.kind);
        let progress = event.progress;
        *visibility = Visibility::Visible;
        let fade = (progress * 8.0).min(1.0) * ((1.0 - progress) * 8.0).min(1.0);
        if part.kind == AmbientKind::HurricaneLeaves {
            animate_hurricane_leaves(
                part.index,
                progress,
                fade,
                &time,
                seasons,
                &mut tf,
                &mut sprite,
            );
            continue;
        }
        if part.kind == AmbientKind::HurricaneTree {
            animate_hurricane_tree(part.index, progress, fade, &mut tf, &mut sprite);
            continue;
        }
        if part.kind == AmbientKind::HelpedCar {
            let Some(traffic) = helped.get(part.index).copied().flatten() else {
                *visibility = Visibility::Hidden;
                continue;
            };
            animate_helped_car(kind, traffic, progress, fade, &proj, &mut tf, &mut sprite);
            continue;
        }
        match kind {
            AmbientKind::Birds => {
                animate_birds(part.index, progress, fade, &time, &mut tf, &mut sprite)
            }
            AmbientKind::ShootingStar => {
                animate_shooting_star(part.index, progress, fade, &mut tf, &mut sprite)
            }
            AmbientKind::Hurricane => {
                animate_hurricane(part.index, progress, fade, &time, &mut tf, &mut sprite)
            }
            AmbientKind::HurricaneLeaves => unreachable!("debris accompanies the hurricane event"),
            AmbientKind::HurricaneTree => unreachable!("uprooted trees accompany the hurricane"),
            AmbientKind::SakuraPetals => {
                animate_sakura_petals(part.index, progress, fade, &time, &mut tf, &mut sprite)
            }
            AmbientKind::Ufo => {
                animate_ufo(part.index, progress, fade, &time, &mut tf, &mut sprite)
            }
            AmbientKind::HelpedCar => unreachable!("handled before event dispatch"),
        }
    }
}

fn animate_hurricane_tree(
    index: usize,
    progress: f32,
    fade: f32,
    tf: &mut Transform,
    sprite: &mut Sprite,
) {
    let tree = index / 3;
    let part = index % 3;
    let side = if tree == 0 { -1.0 } else { 1.0 };
    let pull = smoothstep(((progress - 0.10) / 0.68).clamp(0.0, 1.0));
    let storm_x = -570.0 + 1140.0 * progress;
    let start_x = side * (425.0 + tree as f32 * 18.0);
    let start_y = -18.0 + tree as f32 * 34.0;
    tf.translation.x = start_x + (storm_x - start_x) * pull;
    tf.translation.y = start_y
        + (115.0 - start_y) * pull
        + (progress * std::f32::consts::TAU * 3.0 + tree as f32).sin() * 24.0 * pull;
    tf.rotation = Quat::from_rotation_z(side * pull * std::f32::consts::TAU * 1.4);
    let (offset, size, color) = match part {
        0 => (
            Vec2::new(0.0, 17.0),
            Vec2::new(9.0, 38.0),
            Color::srgba(0.32, 0.17, 0.07, fade),
        ),
        1 => (
            Vec2::new(-10.0, 39.0),
            Vec2::new(42.0, 28.0),
            Color::srgba(0.06, 0.29, 0.12, fade * 0.92),
        ),
        _ => (
            Vec2::new(12.0, 48.0),
            Vec2::new(31.0, 23.0),
            Color::srgba(0.12, 0.46, 0.18, fade * 0.88),
        ),
    };
    tf.translation.x += offset.x;
    tf.translation.y += offset.y;
    sprite.custom_size = Some(size);
    sprite.color = color;
}

fn animate_birds(
    index: usize,
    progress: f32,
    fade: f32,
    time: &Time,
    tf: &mut Transform,
    sprite: &mut Sprite,
) {
    let bird = index / 2;
    let wing = index % 2;
    let formation_x = (bird % 3) as f32 * 27.0 - (bird / 3) as f32 * 14.0;
    let formation_y = (bird / 3) as f32 * 18.0 - (bird % 3) as f32 * 5.0;
    let flap = (time.elapsed_secs() * 10.0 + bird as f32).sin() * 0.22;
    tf.translation.x = -540.0 + 1080.0 * progress + formation_x;
    tf.translation.y = 235.0 + formation_y + (progress * std::f32::consts::TAU).sin() * 8.0;
    tf.rotation = Quat::from_rotation_z(if wing == 0 { 0.42 + flap } else { -0.42 - flap });
    sprite.custom_size = Some(Vec2::new(12.0, 2.0));
    sprite.color = Color::srgba(0.05, 0.08, 0.12, fade * 0.82);
}

fn animate_shooting_star(
    index: usize,
    progress: f32,
    fade: f32,
    tf: &mut Transform,
    sprite: &mut Sprite,
) {
    tf.translation.x = -410.0 + 780.0 * progress - index as f32 * 16.0;
    tf.translation.y = 292.0 - 125.0 * progress + index as f32 * 7.0;
    tf.rotation = Quat::from_rotation_z(-0.16);
    sprite.custom_size = Some(if index == 0 {
        Vec2::new(54.0, 2.0)
    } else {
        Vec2::new(8.0, 5.0)
    });
    sprite.color = Color::srgba(0.82, 0.94, 1.0, fade * (0.62 + index as f32 * 0.3));
}

fn animate_hurricane(
    index: usize,
    progress: f32,
    fade: f32,
    time: &Time,
    tf: &mut Transform,
    sprite: &mut Sprite,
) {
    let i = index as f32;
    let swirl = (time.elapsed_secs() * 4.2 + i * 1.35).sin() * (18.0 - i * 1.15);
    tf.translation.x = -570.0 + 1140.0 * progress + swirl;
    tf.translation.y = 188.0 - i * 15.0;
    tf.rotation = Quat::from_rotation_z(
        (time.elapsed_secs() * 3.1 + i * 0.72).sin() * 0.075 + (i - 4.5) * 0.008,
    );
    sprite.custom_size = Some(Vec2::new((188.0 - i * 16.0).max(40.0), 13.0 - i * 0.55));
    let pulse = 0.90 + (time.elapsed_secs() * 5.0 + i).sin() * 0.10;
    let shade = if index.is_multiple_of(2) { 1.0 } else { 0.72 };
    sprite.color = Color::srgba(
        0.32 * shade * pulse,
        0.37 * shade * pulse,
        0.45 * shade * pulse,
        fade * (0.68 + i * 0.018),
    );
}

fn animate_sakura_petals(
    index: usize,
    progress: f32,
    fade: f32,
    time: &Time,
    tf: &mut Transform,
    sprite: &mut Sprite,
) {
    let phase = index as f32 * 1.73;
    tf.translation.x = -540.0 + 1080.0 * progress + (index % 6) as f32 * 18.0;
    tf.translation.y = 280.0 - progress * 170.0
        + (index * 47 % 125) as f32
        + (time.elapsed_secs() * 5.0 + phase).sin() * 18.0;
    tf.rotation = Quat::from_rotation_z(time.elapsed_secs() * 3.8 + phase);
    sprite.custom_size = Some(Vec2::new(7.0 + (index % 3) as f32 * 2.0, 3.0));
    sprite.color = if index.is_multiple_of(3) {
        Color::srgba(1.0, 0.76, 0.86, fade * 0.74)
    } else {
        Color::srgba(1.0, 0.48, 0.68, fade * 0.68)
    };
}

#[allow(clippy::too_many_arguments)]
fn animate_hurricane_leaves(
    index: usize,
    progress: f32,
    fade: f32,
    time: &Time,
    seasons: [f32; 4],
    tf: &mut Transform,
    sprite: &mut Sprite,
) {
    let phase = index as f32 * 2.17;
    let swirl = (time.elapsed_secs() * 7.0 + phase).sin();
    tf.translation.x = -560.0 + 1120.0 * progress + (index % 5) as f32 * 24.0;
    tf.translation.y = 70.0 + (index * 53 % 235) as f32 + swirl * (28.0 + (index % 4) as f32 * 7.0);
    tf.rotation = Quat::from_rotation_z(time.elapsed_secs() * (5.0 + index as f32 * 0.12));
    sprite.custom_size = Some(Vec2::new(9.0 + (index % 4) as f32 * 2.0, 4.0));
    let color = if seasons[3] > 0.55 {
        Color::srgba(0.78, 0.84, 0.88, fade * 0.72)
    } else if seasons[2] > 0.35 {
        Color::srgba(0.88, 0.46, 0.10, fade * 0.78)
    } else {
        Color::srgba(0.34, 0.58, 0.16, fade * 0.74)
    };
    sprite.color = color;
}

fn animate_ufo(
    index: usize,
    progress: f32,
    fade: f32,
    time: &Time,
    tf: &mut Transform,
    sprite: &mut Sprite,
) {
    let bob = (time.elapsed_secs() * 4.2).sin() * 4.0;
    let path = ufo_path(progress);
    tf.translation.x = path.x;
    tf.translation.y = path.y + bob + index.min(2) as f32 * 4.0;
    tf.rotation = Quat::IDENTITY;
    let pulse = 0.55 + 0.45 * (time.elapsed_secs() * 9.0).sin().abs();
    let (size, color) = match index {
        0 => (Vec2::new(62.0, 8.0), Color::srgba(0.45, 0.56, 0.64, fade)),
        1 => (
            Vec2::new(26.0, 11.0),
            Color::srgba(0.35, 0.82, 0.92, fade * 0.8),
        ),
        2 => (
            Vec2::new(38.0, 4.0),
            Color::srgba(0.35, 1.0, 0.72, fade * pulse),
        ),
        _ => {
            let beam = ((progress - 0.22) / 0.10).clamp(0.0, 1.0)
                * ((0.75 - progress) / 0.12).clamp(0.0, 1.0);
            tf.translation.y = 105.0;
            (
                Vec2::new(28.0 + pulse * 8.0, 178.0),
                Color::srgba(0.34, 1.0, 0.76, fade * beam * 0.22),
            )
        }
    };
    sprite.custom_size = Some(size);
    sprite.color = color;
}

fn ufo_path(progress: f32) -> Vec2 {
    let (x, departure_lift) = if progress < 0.22 {
        (550.0 * (1.0 - smoothstep(progress / 0.22)), 0.0)
    } else if progress < 0.75 {
        (0.0, 0.0)
    } else {
        let depart = smoothstep((progress - 0.75) / 0.25);
        (-550.0 * depart, 130.0 * depart)
    };
    Vec2::new(x, 205.0 + departure_lift)
}

fn animate_helped_car(
    kind: AmbientKind,
    traffic: game_core::RareTrafficHelp,
    progress: f32,
    fade: f32,
    proj: &Proj,
    tf: &mut Transform,
    sprite: &mut Sprite,
) {
    let Some((start_y, scale, lat)) = proj.project(traffic.start_ahead, 0.0) else {
        return;
    };
    let start_x = (lat + traffic.start_x) * HALF_ROAD_PX * scale;
    let pull = match kind {
        AmbientKind::Hurricane => smoothstep(((progress - 0.12) / 0.58).clamp(0.0, 1.0)),
        // The saucer completes its approach before the tractor beam lifts.
        AmbientKind::Ufo => smoothstep(((progress - 0.26) / 0.42).clamp(0.0, 1.0)),
        _ => unreachable!("only gameplay-help events carry cars"),
    };
    match kind {
        AmbientKind::Hurricane => {
            let target_x = -570.0 + 1140.0 * progress;
            tf.translation.x = start_x + (target_x - start_x) * pull;
            tf.translation.y = start_y
                + (105.0 - start_y) * pull
                + (progress * std::f32::consts::TAU * 4.0).sin() * 18.0 * pull;
            tf.rotation = Quat::from_rotation_z(progress * std::f32::consts::TAU * 3.0 * pull);
        }
        AmbientKind::Ufo => {
            tf.translation.x = start_x * (1.0 - pull);
            tf.translation.y = start_y + (125.0 - start_y) * pull;
            tf.rotation = Quat::from_rotation_z(progress * 0.22);
        }
        _ => unreachable!("only gameplay-help events carry cars"),
    }
    tf.translation.z = 0.68;
    sprite.custom_size = Some(Vec2::new(
        (TRAFFIC_CAR_WIDTH_PX * scale).max(10.0) * (1.0 - pull * 0.28),
        (TRAFFIC_CAR_WIDTH_PX * 0.42 * scale).max(5.0) * (1.0 - pull * 0.28),
    ));
    sprite.color = Color::srgba(0.92, 0.20, 0.24, fade * (1.0 - pull * 0.35));
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_event_kinds_map_to_every_visual_kind() {
        assert_eq!(
            ambient_kind(game_core::RareEventKind::Ufo),
            AmbientKind::Ufo
        );
        assert_eq!(
            ambient_kind(game_core::RareEventKind::Hurricane),
            AmbientKind::Hurricane
        );
    }

    #[test]
    fn petals_are_occasional_spring_events_and_hurricane_debris_is_linked() {
        assert!(participates(
            AmbientKind::HurricaneLeaves,
            AmbientKind::Hurricane
        ));
        assert!(participates(
            AmbientKind::HurricaneTree,
            AmbientKind::Hurricane
        ));
        assert!(!participates(
            AmbientKind::HurricaneLeaves,
            AmbientKind::Birds
        ));
        assert!(participates(AmbientKind::HelpedCar, AmbientKind::Ufo));
        assert!(participates(AmbientKind::HelpedCar, AmbientKind::Hurricane));
    }

    #[test]
    fn ufo_approaches_hovers_over_the_road_then_departs() {
        assert!(ufo_path(0.0).x > 500.0);
        assert_eq!(ufo_path(0.30).x, 0.0);
        assert_eq!(ufo_path(0.70).x, 0.0);
        assert!(ufo_path(0.90).x < 0.0);
        assert!(ufo_path(0.90).y > ufo_path(0.50).y);
        assert_eq!(ufo_path(0.50).y, 205.0);
    }

    #[test]
    fn rare_helpers_converge_on_the_middle_distance_road() {
        let ufo = ufo_path(0.50);
        assert!(ufo.y < 220.0);
        let mut tf = Transform::default();
        let mut sprite = Sprite::default();
        animate_hurricane(9, 0.5, 1.0, &Time::default(), &mut tf, &mut sprite);
        assert!(tf.translation.y <= 55.0);
    }

    #[test]
    fn hurricane_is_a_large_tapered_foreground_funnel_below_its_debris() {
        assert!(ambient_depth(AmbientKind::Hurricane) > 0.0);
        assert!(
            ambient_depth(AmbientKind::Hurricane) < ambient_depth(AmbientKind::HurricaneLeaves)
        );

        let mut top = Transform::default();
        let mut bottom = Transform::default();
        let mut top_sprite = Sprite::default();
        let mut bottom_sprite = Sprite::default();
        animate_hurricane(0, 0.5, 1.0, &Time::default(), &mut top, &mut top_sprite);
        animate_hurricane(
            9,
            0.5,
            1.0,
            &Time::default(),
            &mut bottom,
            &mut bottom_sprite,
        );
        let top_size = top_sprite.custom_size.expect("top funnel band");
        let bottom_size = bottom_sprite.custom_size.expect("bottom funnel band");
        assert!(top.translation.y - bottom.translation.y > 120.0);
        assert!(top_size.x > bottom_size.x * 4.0);
        assert!(top_size.y > bottom_size.y);
    }

    #[test]
    fn hurricane_uproots_trees_from_both_grass_sides() {
        let mut left = Transform::default();
        let mut right = Transform::default();
        let mut sprite = Sprite::default();
        animate_hurricane_tree(0, 0.0, 1.0, &mut left, &mut sprite);
        animate_hurricane_tree(3, 0.0, 1.0, &mut right, &mut sprite);
        assert!(left.translation.x < -400.0);
        assert!(right.translation.x > 400.0);
        animate_hurricane_tree(0, 0.60, 1.0, &mut left, &mut sprite);
        assert!(left.rotation != Quat::IDENTITY);
    }
}
