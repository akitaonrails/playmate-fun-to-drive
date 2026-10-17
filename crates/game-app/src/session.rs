use game_core::{rank_leaderboard, Difficulty, LeaderboardEntry, SessionResult};
use serde::{Deserialize, Serialize};

pub const STORAGE_KEY: &str = "fun-to-drive.leaderboard.v2";
pub const LEGACY_STORAGE_KEY: &str = "fun-to-drive.leaderboard.v1";
const STORAGE_VERSION: u8 = 2;
// These are deliberately generous ceilings for a run capped at 600 seconds.
// Current maximum tuning cannot approach 100 cycles or 100k printed-traffic
// encounters; the bounds reject corrupt/hostile numeric payloads without
// invalidating any plausible legitimate result.
const MAX_STORED_CYCLES: u64 = 100;
const MAX_STORED_TRAFFIC: u64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EndMode {
    #[default]
    DifficultySelect,
    Playing,
    NameEntry,
    Leaderboard,
}

impl EndMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DifficultySelect => "difficulty_select",
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

#[derive(Debug, Clone)]
pub struct EndFlow {
    pub mode: EndMode,
    pub initials: Initials,
    pub difficulty_index: usize,
}

impl Default for EndFlow {
    fn default() -> Self {
        Self {
            mode: EndMode::DifficultySelect,
            initials: Initials::default(),
            difficulty_index: 1,
        }
    }
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
            result.difficulty,
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
struct StoredBoardV1 {
    version: u8,
    entries: Vec<StoredEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredBoardV2 {
    version: u8,
    boards: StoredBoards,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredBoards {
    easy: Vec<StoredEntry>,
    medium: Vec<StoredEntry>,
    hard: Vec<StoredEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEntry {
    initials: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    difficulty: Option<String>,
    day_cycles: u64,
    cars_passed: u64,
    cars_bumped: u64,
}

pub fn decode_board(raw: &str) -> Vec<LeaderboardEntry> {
    decode_board_checked(raw).unwrap_or_default()
}

fn decode_board_checked(raw: &str) -> Option<Vec<LeaderboardEntry>> {
    if let Ok(stored) = serde_json::from_str::<StoredBoardV2>(raw) {
        if stored.version != STORAGE_VERSION {
            return None;
        }
        let total =
            stored.boards.easy.len() + stored.boards.medium.len() + stored.boards.hard.len();
        if total > game_core::LEADERBOARD_CAPACITY {
            return None;
        }
        let mut entries = Vec::with_capacity(total);
        if !decode_entries(stored.boards.easy, Difficulty::Easy, &mut entries)
            || !decode_entries(stored.boards.medium, Difficulty::Medium, &mut entries)
            || !decode_entries(stored.boards.hard, Difficulty::Hard, &mut entries)
        {
            return None;
        }
        rank_leaderboard(&mut entries);
        return Some(entries);
    }
    let Ok(stored) = serde_json::from_str::<StoredBoardV1>(raw) else {
        return None;
    };
    if stored.version != 1 || stored.entries.len() > 100 {
        return None;
    }
    let mut entries = Vec::with_capacity(stored.entries.len().min(10));
    for entry in stored.entries {
        if entry.day_cycles > MAX_STORED_CYCLES
            || entry.cars_passed > MAX_STORED_TRAFFIC
            || entry.cars_bumped > MAX_STORED_TRAFFIC
        {
            return None;
        }
        let valid = LeaderboardEntry::new(
            &entry.initials,
            Difficulty::Hard,
            entry.day_cycles,
            entry.cars_passed,
            entry.cars_bumped,
        )?;
        entries.push(valid);
    }
    rank_leaderboard(&mut entries);
    Some(entries)
}

/// Resolve startup storage and signal an actual v1 -> v2 write only when the
/// legacy payload is fully valid. Presence of v2 is authoritative, even when
/// corrupt, so stale legacy data cannot silently replace it.
pub fn load_board_sources(
    v2: Option<&str>,
    legacy_v1: Option<&str>,
) -> (Vec<LeaderboardEntry>, Option<String>) {
    if let Some(raw) = v2 {
        return (decode_board(raw), None);
    }
    let Some(entries) = legacy_v1.and_then(decode_board_checked) else {
        return (Vec::new(), None);
    };
    let migrated = encode_board(&entries);
    (entries, Some(migrated))
}

fn decode_entries(
    stored: Vec<StoredEntry>,
    difficulty: Difficulty,
    entries: &mut Vec<LeaderboardEntry>,
) -> bool {
    if stored.len() > game_core::LEADERBOARD_CAPACITY_PER_DIFFICULTY {
        return false;
    }
    for entry in stored {
        if entry.day_cycles > MAX_STORED_CYCLES
            || entry.cars_passed > MAX_STORED_TRAFFIC
            || entry.cars_bumped > MAX_STORED_TRAFFIC
        {
            return false;
        }
        let Some(valid) = LeaderboardEntry::new(
            &entry.initials,
            difficulty,
            entry.day_cycles,
            entry.cars_passed,
            entry.cars_bumped,
        ) else {
            return false;
        };
        entries.push(valid);
    }
    true
}

pub fn encode_board(entries: &[LeaderboardEntry]) -> String {
    let mut boards = StoredBoards::default();
    for entry in entries.iter().take(game_core::LEADERBOARD_CAPACITY) {
        let stored = StoredEntry {
            initials: entry.initials_str().to_owned(),
            difficulty: None,
            day_cycles: entry.result.day_cycles,
            cars_passed: entry.result.cars_passed,
            cars_bumped: entry.result.cars_bumped,
        };
        match entry.result.difficulty {
            Difficulty::Easy => boards.easy.push(stored),
            Difficulty::Medium => boards.medium.push(stored),
            Difficulty::Hard => boards.hard.push(stored),
        }
    }
    let board = StoredBoardV2 {
        version: STORAGE_VERSION,
        boards,
    };
    serde_json::to_string(&board).expect("leaderboard schema is serializable")
}

pub fn entries_json(entries: &[LeaderboardEntry], difficulty: Difficulty, limit: usize) -> String {
    let rows: Vec<_> = entries
        .iter()
        .filter(|entry| entry.result.difficulty == difficulty)
        .take(limit)
        .map(|entry| {
            serde_json::json!({
                "initials": entry.initials_str(),
                "difficulty": difficulty_name(entry.result.difficulty),
                "day_cycles": entry.result.day_cycles,
                "cars_passed": entry.result.cars_passed,
                "cars_bumped": entry.result.cars_bumped,
                "score": entry.result.score,
            })
        })
        .collect();
    serde_json::to_string(&rows).expect("leaderboard rows are serializable")
}

pub const fn difficulty_name(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "easy",
        Difficulty::Medium => "medium",
        Difficulty::Hard => "hard",
    }
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
        let result = SessionResult::new(Difficulty::Hard, 2, 12, 1);
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
        let mut entries = vec![
            LeaderboardEntry::new("AAA", Difficulty::Medium, 3, 20, 2).unwrap(),
            LeaderboardEntry::new("BBB", Difficulty::Easy, 1, 8, 0).unwrap(),
            LeaderboardEntry::new("CCC", Difficulty::Hard, 5, 30, 1).unwrap(),
        ];
        rank_leaderboard(&mut entries);
        let raw = encode_board(&entries);
        let decoded = decode_board(&raw);
        assert_eq!(decoded, entries);
        assert_eq!(
            game_core::leaderboard_for(&decoded, Difficulty::Easy).count(),
            1
        );
        assert_eq!(
            game_core::leaderboard_for(&decoded, Difficulty::Medium).count(),
            1
        );
        assert_eq!(
            game_core::leaderboard_for(&decoded, Difficulty::Hard).count(),
            1
        );
        let medium = game_core::leaderboard_for(&decoded, Difficulty::Medium)
            .next()
            .unwrap();
        assert_eq!(medium.result.score, game_core::score_for(3, 20, 2));
        let legacy = decode_board(
            r#"{"version":1,"entries":[{"initials":"OLD","day_cycles":1,"cars_passed":2,"cars_bumped":3}]}"#,
        );
        assert_eq!(legacy[0].result.difficulty, Difficulty::Hard);
        assert!(decode_board("not json").is_empty());
        assert!(decode_board(r#"{"version":1,"entries":[{"initials":"bad","day_cycles":1,"cars_passed":2,"cars_bumped":3}]}"#).is_empty());
        assert!(decode_board(r#"{"version":2,"entries":[]}"#).is_empty());
        assert!(decode_board(
            r#"{"version":1,"entries":[{"initials":"MAX","day_cycles":18446744073709551615,"cars_passed":18446744073709551615,"cars_bumped":18446744073709551615}]}"#
        )
        .is_empty());
    }

    #[test]
    fn startup_migration_writes_only_valid_legacy_data() {
        let legacy = r#"{"version":1,"entries":[{"initials":"OLD","day_cycles":1,"cars_passed":2,"cars_bumped":0}]}"#;
        let (entries, migrated) = load_board_sources(None, Some(legacy));
        assert_eq!(entries.len(), 1);
        let migrated = migrated.expect("valid v1 needs an immediate v2 write");
        assert!(migrated.contains("\"version\":2"));
        assert_eq!(decode_board(&migrated), entries);

        let (entries, migration) = load_board_sources(None, Some("corrupt"));
        assert!(entries.is_empty());
        assert!(migration.is_none(), "corrupt v1 must not overwrite storage");

        let valid_v2 = encode_board(&[]);
        let (_, migration) = load_board_sources(Some(&valid_v2), Some(legacy));
        assert!(migration.is_none(), "present v2 is authoritative");
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
