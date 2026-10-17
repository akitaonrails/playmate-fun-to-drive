//! Rare, deterministic sky easter eggs.
//!
//! The module exposes only installation and synchronization; scheduling,
//! eligibility, pooling, and animation details stay local and unit-tested.

use bevy::prelude::*;
use game_core::Phase;

use crate::Sim;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AmbientKind {
    Birds,
    ShootingStar,
    Hurricane,
    Ufo,
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
        (AmbientKind::Hurricane, 6),
        (AmbientKind::Ufo, 3),
    ] {
        for index in 0..count {
            commands.spawn((
                Sprite::from_color(Color::WHITE, Vec2::ONE),
                Transform::from_xyz(0.0, 0.0, -4.62),
                Visibility::Hidden,
                AmbientPart { kind, index },
            ));
        }
    }
}

fn kind_for_slot(slot: u64, salt: u64) -> Option<AmbientKind> {
    let mut hash = slot ^ salt.rotate_left(17) ^ 0xd1b5_4a32_d192_ed03;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    match hash % 32 {
        0 => Some(AmbientKind::Ufo),
        1 => Some(AmbientKind::Hurricane),
        2..=4 => Some(AmbientKind::ShootingStar),
        5..=9 => Some(AmbientKind::Birds),
        _ => None,
    }
}

fn event_at(t: f32, daylight: f32, salt: u64) -> Option<(AmbientKind, f32)> {
    const SLOT_SECS: f32 = 20.0;
    const EVENT_SECS: f32 = 5.0;
    let slot = (t.max(0.0) / SLOT_SECS).floor() as u64;
    let progress = (t - slot as f32 * SLOT_SECS) / EVENT_SECS;
    if !(0.0..1.0).contains(&progress) {
        return None;
    }
    let kind = kind_for_slot(slot, salt)?;
    match kind {
        AmbientKind::Birds if daylight > 0.62 => Some((kind, progress)),
        AmbientKind::ShootingStar | AmbientKind::Ufo if daylight < 0.30 => Some((kind, progress)),
        AmbientKind::Hurricane => Some((kind, progress)),
        _ => None,
    }
}

pub(super) fn sync(
    sim: Res<Sim>,
    time: Res<Time>,
    mut parts: Query<(&AmbientPart, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let daylight = game_core::environment::daylight(sim.0.environment_time);
    let salt = sim
        .0
        .road
        .hazards
        .first()
        .map_or(0, |hazard| u64::from(hazard.s.to_bits()));
    let event = matches!(sim.0.phase, Phase::Running)
        .then(|| event_at(sim.0.environment_time, daylight, salt))
        .flatten();

    for (part, mut tf, mut sprite, mut visibility) in &mut parts {
        let Some((kind, progress)) = event.filter(|(kind, _)| *kind == part.kind) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = Visibility::Visible;
        let fade = (progress * 8.0).min(1.0) * ((1.0 - progress) * 8.0).min(1.0);
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
            AmbientKind::Ufo => {
                animate_ufo(part.index, progress, fade, &time, &mut tf, &mut sprite)
            }
        }
    }
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
    let swirl = (time.elapsed_secs() * 3.0 + i * 1.35).sin() * (10.0 - i);
    tf.translation.x = -570.0 + 1140.0 * progress + swirl;
    tf.translation.y = 265.0 - i * 19.0;
    tf.rotation = Quat::from_rotation_z((i - 2.5) * 0.025);
    sprite.custom_size = Some(Vec2::new((112.0 - i * 17.0).max(18.0), 8.0));
    sprite.color = Color::srgba(0.22, 0.27, 0.34, fade * (0.36 + i * 0.035));
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
    tf.translation.x = 550.0 - 1100.0 * progress;
    tf.translation.y = 258.0 + bob + index as f32 * 4.0;
    tf.rotation = Quat::IDENTITY;
    let pulse = 0.55 + 0.45 * (time.elapsed_secs() * 9.0).sin().abs();
    let (size, color) = match index {
        0 => (Vec2::new(62.0, 8.0), Color::srgba(0.45, 0.56, 0.64, fade)),
        1 => (
            Vec2::new(26.0, 11.0),
            Color::srgba(0.35, 0.82, 0.92, fade * 0.8),
        ),
        _ => (
            Vec2::new(38.0, 4.0),
            Color::srgba(0.35, 1.0, 0.72, fade * pulse),
        ),
    };
    sprite.custom_size = Some(size);
    sprite.color = color;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rare_events_respect_day_and_night() {
        let salt = 1234;
        let ufo_slot = (0..1024)
            .find(|slot| kind_for_slot(*slot, salt) == Some(AmbientKind::Ufo))
            .expect("schedule should contain a UFO");
        let ufo_time = ufo_slot as f32 * 20.0 + 1.0;
        assert_eq!(event_at(ufo_time, 1.0, salt), None);
        assert!(matches!(
            event_at(ufo_time, 0.0, salt),
            Some((AmbientKind::Ufo, _))
        ));
        assert_eq!(event_at(ufo_time + 6.0, 0.0, salt), None);

        let birds_slot = (0..1024)
            .find(|slot| kind_for_slot(*slot, salt) == Some(AmbientKind::Birds))
            .expect("schedule should contain birds");
        let birds_time = birds_slot as f32 * 20.0 + 1.0;
        assert!(event_at(birds_time, 1.0, salt).is_some());
        assert_eq!(event_at(birds_time, 0.0, salt), None);
    }
}
