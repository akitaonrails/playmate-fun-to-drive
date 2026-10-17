use game_core::{rank_leaderboard, LeaderboardEntry, SessionResult};
use serde::{Deserialize, Serialize};

pub const STORAGE_KEY: &str = "fun-to-drive.leaderboard.v1";
const STORAGE_VERSION: u8 = 1;
// These are deliberately generous ceilings for a run capped at 600 seconds.
// Current maximum tuning cannot approach 100 cycles or 100k printed-traffic
// encounters; the bounds reject corrupt/hostile numeric payloads without
// invalidating any plausible legitimate result.
const MAX_STORED_CYCLES: u64 = 100;
const MAX_STORED_TRAFFIC: u64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EndMode {
    #[default]
    Playing,
    NameEntry,
    Leaderboard,
}

impl EndMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Playing => "playing",
            Self::NameEntry => "name_entry",
            Self::Leaderboard => "leaderboard",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Initials {
    bytes: [u8; 3],
    len: usize,
}

impl Initials {
    pub fn push(&mut self, letter: char) -> bool {
        if self.len >= self.bytes.len() || !letter.is_ascii_alphabetic() {
            return false;
        }
        self.bytes[self.len] = letter.to_ascii_uppercase() as u8;
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> bool {
        if self.len == 0 {
            return false;
        }
        self.len -= 1;
        self.bytes[self.len] = 0;
        true
    }

    pub const fn is_complete(&self) -> bool {
        self.len == 3
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).expect("initials contain ASCII only")
    }

    pub fn cell(&self, index: usize) -> char {
        self.bytes
            .get(index)
            .copied()
            .filter(|byte| *byte != 0)
            .map(char::from)
            .unwrap_or('_')
    }
}

#[derive(Debug, Clone, Default)]
pub struct EndFlow {
    pub mode: EndMode,
    pub initials: Initials,
}

impl EndFlow {
    pub fn begin_entry(&mut self) {
        self.mode = EndMode::NameEntry;
        self.initials = Initials::default();
    }

    pub fn submit(&mut self, result: SessionResult) -> Option<LeaderboardEntry> {
        if self.mode != EndMode::NameEntry || !self.initials.is_complete() {
            return None;
        }
        let entry = LeaderboardEntry::new(
            self.initials.as_str(),
            result.day_cycles,
            result.cars_passed,
            result.cars_bumped,
        )?;
        self.mode = EndMode::Leaderboard;
        Some(entry)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredBoard {
    version: u8,
    entries: Vec<StoredEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEntry {
    initials: String,
    day_cycles: u64,
    cars_passed: u64,
    cars_bumped: u64,
}

pub fn decode_board(raw: &str) -> Vec<LeaderboardEntry> {
    let Ok(stored) = serde_json::from_str::<StoredBoard>(raw) else {
        return Vec::new();
    };
    if stored.version != STORAGE_VERSION || stored.entries.len() > 100 {
        return Vec::new();
    }
    let mut entries = Vec::with_capacity(stored.entries.len().min(10));
    for entry in stored.entries {
        if entry.day_cycles > MAX_STORED_CYCLES
            || entry.cars_passed > MAX_STORED_TRAFFIC
            || entry.cars_bumped > MAX_STORED_TRAFFIC
        {
            return Vec::new();
        }
        let Some(valid) = LeaderboardEntry::new(
            &entry.initials,
            entry.day_cycles,
            entry.cars_passed,
            entry.cars_bumped,
        ) else {
            return Vec::new();
        };
        entries.push(valid);
    }
    rank_leaderboard(&mut entries);
    entries
}

pub fn encode_board(entries: &[LeaderboardEntry]) -> String {
    let board = StoredBoard {
        version: STORAGE_VERSION,
        entries: entries
            .iter()
            .take(game_core::LEADERBOARD_CAPACITY)
            .map(|entry| StoredEntry {
                initials: entry.initials_str().to_owned(),
                day_cycles: entry.result.day_cycles,
                cars_passed: entry.result.cars_passed,
                cars_bumped: entry.result.cars_bumped,
            })
            .collect(),
    };
    serde_json::to_string(&board).expect("leaderboard schema is serializable")
}

pub fn entries_json(entries: &[LeaderboardEntry], limit: usize) -> String {
    let rows: Vec<_> = entries
        .iter()
        .take(limit)
        .map(|entry| {
            serde_json::json!({
                "initials": entry.initials_str(),
                "day_cycles": entry.result.day_cycles,
                "cars_passed": entry.result.cars_passed,
                "cars_bumped": entry.result.cars_bumped,
                "score": entry.result.score,
            })
        })
        .collect();
    serde_json::to_string(&rows).expect("leaderboard rows are serializable")
}

pub fn validated_session_secs(raw: &str) -> Option<f32> {
    let value = raw.parse::<f32>().ok()?;
    (value.is_finite() && (1.0..=600.0).contains(&value)).then_some(value)
}

pub fn qa_mode_and_duration(raw: Option<&str>) -> (bool, Option<f32>) {
    (raw.is_some(), raw.and_then(validated_session_secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_accept_three_ascii_letters_and_backspace() {
        let mut initials = Initials::default();
        assert!(initials.push('a'));
        assert!(initials.push('Z'));
        assert!(!initials.push('7'));
        assert!(initials.push('q'));
        assert!(!initials.push('X'));
        assert_eq!(initials.as_str(), "AZQ");
        assert!(initials.pop());
        assert_eq!(initials.as_str(), "AZ");
        assert_eq!(initials.cell(2), '_');
    }

    #[test]
    fn submit_requires_complete_name_and_happens_once() {
        let result = SessionResult::new(2, 12, 1);
        let mut flow = EndFlow::default();
        flow.begin_entry();
        flow.initials.push('A');
        flow.initials.push('B');
        assert!(flow.submit(result).is_none());
        flow.initials.push('C');
        let entry = flow.submit(result).unwrap();
        assert_eq!(entry.initials_str(), "ABC");
        assert_eq!(flow.mode, EndMode::Leaderboard);
        assert!(flow.submit(result).is_none());
    }

    #[test]
    fn storage_round_trip_recomputes_score_and_rejects_corruption() {
        let entries = vec![LeaderboardEntry::new("AAA", 3, 20, 2).unwrap()];
        let raw = encode_board(&entries);
        let decoded = decode_board(&raw);
        assert_eq!(decoded, entries);
        assert_eq!(decoded[0].result.score, game_core::score_for(3, 20, 2));
        assert!(decode_board("not json").is_empty());
        assert!(decode_board(r#"{"version":1,"entries":[{"initials":"bad","day_cycles":1,"cars_passed":2,"cars_bumped":3}]}"#).is_empty());
        assert!(decode_board(r#"{"version":2,"entries":[]}"#).is_empty());
        assert!(decode_board(
            r#"{"version":1,"entries":[{"initials":"MAX","day_cycles":18446744073709551615,"cars_passed":18446744073709551615,"cars_bumped":18446744073709551615}]}"#
        )
        .is_empty());
    }

    #[test]
    fn qa_duration_is_finite_and_narrowly_bounded() {
        assert_eq!(validated_session_secs("2.5"), Some(2.5));
        for raw in ["0", "601", "NaN", "inf", "nope"] {
            assert_eq!(validated_session_secs(raw), None);
        }
        assert_eq!(qa_mode_and_duration(None), (false, None));
        assert_eq!(qa_mode_and_duration(Some("2")), (true, Some(2.0)));
        assert_eq!(qa_mode_and_duration(Some("invalid")), (true, None));
    }
}
