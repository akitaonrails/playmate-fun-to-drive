//! Deterministic scheduling for rare ambient events and their gameplay help.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RareEventKind {
    Birds,
    ShootingStar,
    Hurricane,
    SakuraPetals,
    Ufo,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RareEvent {
    pub kind: RareEventKind,
    pub slot: u64,
    pub progress: f32,
}

pub const SLOT_SECS: f32 = 20.0;
pub const EVENT_SECS: f32 = 5.0;

pub fn kind_for_slot(slot: u64, salt: u64) -> Option<RareEventKind> {
    let mut hash = slot ^ salt.rotate_left(17) ^ 0xd1b5_4a32_d192_ed03;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    match hash % 32 {
        0 => Some(RareEventKind::Ufo),
        1 => Some(RareEventKind::Hurricane),
        2..=4 => Some(RareEventKind::ShootingStar),
        5..=9 => Some(RareEventKind::Birds),
        10..=13 => Some(RareEventKind::SakuraPetals),
        _ => None,
    }
}

pub fn event_at(environment_time: f32, salt: u64) -> Option<RareEvent> {
    let time = environment_time.max(0.0);
    let slot = (time / SLOT_SECS).floor() as u64;
    let progress = (time - slot as f32 * SLOT_SECS) / EVENT_SECS;
    if !(0.0..1.0).contains(&progress) {
        return None;
    }
    let kind = kind_for_slot(slot, salt)?;
    let daylight = crate::environment::daylight(time);
    let spring = crate::environment::season_weights(time)[0];
    let eligible = match kind {
        RareEventKind::Birds => daylight > 0.62,
        RareEventKind::ShootingStar | RareEventKind::Ufo => daylight < 0.30,
        RareEventKind::Hurricane => true,
        RareEventKind::SakuraPetals => daylight > 0.45 && spring > 0.55,
    };
    eligible.then_some(RareEvent {
        kind,
        slot,
        progress,
    })
}

pub fn selection_hash(slot: u64, seed: u64, hazard_index: usize) -> u64 {
    let mut value = slot
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(seed.rotate_left(21))
        .wrapping_add((hazard_index as u64).wrapping_mul(0x94d0_49bb_1331_11eb));
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^ (value >> 27)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ufo_is_night_only_and_events_end_after_five_seconds() {
        let salt = 1234;
        let slot = (0..4096)
            .find(|slot| {
                kind_for_slot(*slot, salt) == Some(RareEventKind::Ufo)
                    && crate::environment::daylight(*slot as f32 * SLOT_SECS + 1.0) < 0.30
            })
            .expect("schedule should contain a nighttime UFO");
        let time = slot as f32 * SLOT_SECS + 1.0;
        assert_eq!(
            event_at(time, salt).map(|event| event.kind),
            Some(RareEventKind::Ufo)
        );
        assert_eq!(event_at(time + EVENT_SECS, salt), None);
    }

    #[test]
    fn selection_is_stable_but_varies_by_car() {
        assert_eq!(selection_hash(9, 7, 2), selection_hash(9, 7, 2));
        assert_ne!(selection_hash(9, 7, 2), selection_hash(9, 7, 3));
    }
}
