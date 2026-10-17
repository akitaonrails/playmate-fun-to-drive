//! Environment cycles (PLAN: user tweak): a day/night layer (DAY_SECS)
//! nested inside a season layer (SEASON_SECS each; 3 day/night cycles per
//! season by default). Pure time math — deterministic and unit-testable.

/// Seconds for one full day/night cycle.
pub const DAY_SECS: f32 = 100.0;
/// Seconds per season.
pub const SEASON_SECS: f32 = 300.0;
/// Seconds for the full year (4 seasons).
pub const YEAR_SECS: f32 = SEASON_SECS * 4.0;

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
}
