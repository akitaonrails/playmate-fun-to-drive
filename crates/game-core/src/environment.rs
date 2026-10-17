//! Environment cycles (PLAN: user tweak): a day/night layer (DAY_SECS)
//! nested inside a season layer (SEASON_SECS each; 3 day/night cycles per
//! season by default). Pure time math — deterministic and unit-testable.

/// Seconds for one full day/night cycle.
pub const DAY_SECS: f32 = 100.0;
/// Seconds per season.
pub const SEASON_SECS: f32 = 300.0;
/// Seconds for the full year (4 seasons).
pub const YEAR_SECS: f32 = SEASON_SECS * 4.0;
pub const WEATHER_SLOT_SECS: f32 = 30.0;
pub const WEATHER_FADE_SECS: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WeatherKind {
    Clear,
    Cloudy,
    Rain,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherTransition {
    pub from: WeatherKind,
    pub to: WeatherKind,
    /// Crossfade progress in [0, 1]. The first two seconds of a slot blend
    /// from the preceding slot, after which the new weather is fully active.
    pub blend: f32,
    pub slot: u64,
}

/// Stateless, index-derived weather schedule. Each aligned three-slot bag is
/// a permutation of all weather kinds and adjacent slots never repeat,
/// including across bag boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeatherSchedule {
    seed: u64,
}

impl WeatherSchedule {
    pub const fn new(seed: u64) -> Self {
        Self {
            seed: seed ^ 0xA076_1D64_78BD_642F,
        }
    }

    fn bag(&self, bag_index: u64) -> [WeatherKind; 3] {
        // This six-permutation cycle is arranged so the last item of every
        // bag differs from the first item of the next, in either direction.
        // Seed chooses an offset and traversal direction, providing a
        // dedicated weather stream without iterative history or road RNG.
        const BAGS: [[WeatherKind; 3]; 6] = [
            [WeatherKind::Clear, WeatherKind::Cloudy, WeatherKind::Rain],
            [WeatherKind::Clear, WeatherKind::Rain, WeatherKind::Cloudy],
            [WeatherKind::Rain, WeatherKind::Clear, WeatherKind::Cloudy],
            [WeatherKind::Rain, WeatherKind::Cloudy, WeatherKind::Clear],
            [WeatherKind::Cloudy, WeatherKind::Rain, WeatherKind::Clear],
            [WeatherKind::Cloudy, WeatherKind::Clear, WeatherKind::Rain],
        ];
        let offset = (self.seed % BAGS.len() as u64) as i64;
        let direction = if self.seed & 1 == 0 { 1 } else { -1 };
        let bag_mod = (bag_index % BAGS.len() as u64) as i64;
        let index = (offset + direction * bag_mod).rem_euclid(BAGS.len() as i64);
        BAGS[index as usize]
    }

    pub fn kind_at_slot(&self, slot: u64) -> WeatherKind {
        self.bag(slot / 3)[slot as usize % 3]
    }

    pub fn at(&self, environment_time: f32) -> WeatherTransition {
        let time = environment_time.max(0.0);
        let slot = (time / WEATHER_SLOT_SECS).floor() as u64;
        let to = self.kind_at_slot(slot);
        let previous = if slot == 0 {
            to
        } else {
            self.kind_at_slot(slot - 1)
        };
        let slot_time = time - slot as f32 * WEATHER_SLOT_SECS;
        let blend = if previous == to {
            1.0
        } else {
            (slot_time / WEATHER_FADE_SECS).clamp(0.0, 1.0)
        };
        WeatherTransition {
            // Once the fade completes, current and next intentionally agree:
            // consumers do not need to retain a stale previous state.
            from: if blend >= 1.0 { to } else { previous },
            to,
            blend,
            slot,
        }
    }
}

/// Daylight factor in [0, 1]: 1 = full day, 0 = deep night. Smooth sine so
/// dawn/dusk ease in and out over the cycle.
pub fn daylight(t: f32) -> f32 {
    let phase = (t.rem_euclid(DAY_SECS)) / DAY_SECS; // 0..1
                                                     // day peaks at phase 0.25, night bottoms at phase 0.75
    ((phase - 0.25) * std::f32::consts::TAU).cos() * 0.5 + 0.5
}

/// Season index in [0, 4): 0 spring, 1 summer, 2 autumn, 3 winter.
pub fn season_index(t: f32) -> f32 {
    (t.rem_euclid(YEAR_SECS)) / SEASON_SECS
}

/// Blend weights for the four seasons. Transitions smear across the last
/// 20% of each season so weather eases in rather than snapping.
pub fn season_weights(t: f32) -> [f32; 4] {
    let idx = season_index(t);
    let base = idx.floor();
    let frac = idx - base;
    let blend = if frac > 0.8 { (frac - 0.8) / 0.2 } else { 0.0 };
    let mut w = [0.0f32; 4];
    let cur = base as usize % 4;
    let next = (cur + 1) % 4;
    w[cur] = 1.0 - blend;
    w[next] = blend;
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daylight_bounds_and_cycle() {
        for t in [0.0, 13.7, 55.0, 99.9, 250.0] {
            let d = daylight(t);
            assert!((0.0..=1.0).contains(&d));
        }
        // mid-day high, mid-night low
        assert!(daylight(DAY_SECS * 0.25) > 0.9);
        assert!(daylight(DAY_SECS * 0.75) < 0.1);
        // periodic
        assert!((daylight(3.3) - daylight(3.3 + DAY_SECS)).abs() < 1e-4);
    }

    #[test]
    fn three_day_cycles_per_season() {
        let season_span = SEASON_SECS;
        assert!((season_span / DAY_SECS - 3.0).abs() < 1e-3);
    }

    #[test]
    fn season_weights_sane() {
        let w = season_weights(10.0); // spring start
        assert!(w[0] > 0.99 && w[1] < 0.01);
        let w = season_weights(SEASON_SECS * 0.9); // mid-blend into summer
        assert!((w[0] - 0.5).abs() < 0.01 && (w[1] - 0.5).abs() < 0.01);
        let w = season_weights(SEASON_SECS * 1.5); // mid summer
        assert!(w[1] > 0.99);
        // near year end, winter is blending into spring (frac 0.98 -> blend 0.92)
        let w = season_weights(YEAR_SECS - 5.0);
        assert!(w[0] > 0.9 && w[3] < 0.1);
        // half a minute before the end, the blend straddles both
        let w = season_weights(YEAR_SECS - 30.0); // frac 0.9
        assert!((w[3] - 0.5).abs() < 0.01 && (w[0] - 0.5).abs() < 0.01);
        for t in [0.0, 700.0, 1199.0] {
            let w = season_weights(t);
            let sum: f32 = w.iter().sum();
            assert!((sum - 1.0).abs() < 1e-4);
            assert!(w.iter().all(|x| *x >= 0.0));
        }
    }

    #[test]
    fn each_weather_bag_contains_all_states_without_boundary_repeats() {
        for seed in [0, 1, 42, u64::MAX] {
            let schedule = WeatherSchedule::new(seed);
            let mut previous = None;
            for bag in 0..20 {
                let mut kinds = [
                    schedule.kind_at_slot(bag * 3),
                    schedule.kind_at_slot(bag * 3 + 1),
                    schedule.kind_at_slot(bag * 3 + 2),
                ];
                kinds.sort();
                assert_eq!(
                    kinds,
                    [WeatherKind::Clear, WeatherKind::Cloudy, WeatherKind::Rain]
                );
                if let Some(last) = previous {
                    assert_ne!(last, schedule.kind_at_slot(bag * 3));
                }
                previous = Some(schedule.kind_at_slot(bag * 3 + 2));
            }
        }
    }

    #[test]
    fn weather_is_deterministic_and_time_indexed() {
        let a = WeatherSchedule::new(99);
        let b = WeatherSchedule::new(99);
        for t in [0.0, 1.5, 30.0, 31.0, 89.999, 900.0] {
            assert_eq!(a.at(t), b.at(t));
        }
        let transition = a.at(30.5);
        assert_eq!(transition.slot, 1);
        assert!((transition.blend - 0.25).abs() < 1e-6);
        assert_ne!(transition.from, transition.to);
        let settled = a.at(32.0);
        assert_eq!(settled.blend, 1.0);
        assert_eq!(settled.from, settled.to);
    }
}
