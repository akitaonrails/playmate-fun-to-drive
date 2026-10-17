//! game-core — pure, deterministic game logic for the Fun-to-Drive recreation.
//!
//! No I/O, no window, no engine types: everything here is unit-testable with
//! plain `cargo test` (PLAN.md M5+). The presentation layer (game-app) only
//! consumes the structs and step functions exposed from this crate.

pub mod dynamics;
pub mod environment;
pub mod game;
pub mod input;
pub mod rng;
pub mod road;
pub mod tuning;

pub use dynamics::BodyDynamics;
pub use environment::{
    daylight, season_weights, WeatherKind, WeatherSchedule, WeatherTransition, DAY_SECS,
    SEASON_SECS, WEATHER_FADE_SECS, WEATHER_SLOT_SECS, YEAR_SECS,
};
pub use game::{
    rank_leaderboard, score_for, EndReason, Game, LeaderboardEntry, Phase, SessionResult,
    StepEvents, LEADERBOARD_CAPACITY,
};
pub use input::{Gear, InputState, KeyboardSteering};
pub use rng::Rng;
pub use road::{Hazard, HazardKind, RoadLoop};
pub use tuning::Tuning;

/// Crate version stamp (used by the app's HUD and smoke tests).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semverish() {
        assert!(version().split('.').count() >= 2);
    }
}
