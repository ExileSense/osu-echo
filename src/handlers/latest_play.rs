
use crate::types::beatmap::Beatmap;
use crate::types::score::Score;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

const HISTORY: usize = 100;

struct Entry {
    latest: LatestPlay,
    recent_md5: VecDeque<String>,
}

#[derive(Debug, Clone)]
pub struct LatestPlay {
    pub player_name: String,
    pub score: Score,
    pub beatmap: Option<Beatmap>,
    pub passed: bool,
    pub rank_after: Option<i32>,
    pub recorded_at: i64,
}

fn store() -> &'static Mutex<HashMap<String, Entry>> {
    static STORE: OnceLock<Mutex<HashMap<String, Entry>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

pub fn record(player_name: &str, score: &Score, beatmap: Option<&Beatmap>, passed: bool, rank_after: Option<i32>) {
    let play = LatestPlay {
        player_name: player_name.to_string(),
        score: score.clone(),
        beatmap: beatmap.cloned(),
        passed,
        rank_after,
        recorded_at: chrono::Utc::now().timestamp(),
    };
    if let Ok(mut map) = store().lock() {
        let entry = map.entry(key(player_name)).or_insert_with(|| Entry { latest: play.clone(), recent_md5: VecDeque::new() });
        entry.recent_md5.push_front(play.score.md5.clone());
        entry.recent_md5.truncate(HISTORY);
        entry.latest = play;
    }
}

pub fn try_number(player_name: &str) -> usize {
    let Ok(map) = store().lock() else { return 1 };
    let Some(entry) = map.get(&key(player_name)) else { return 1 };
    let Some(first) = entry.recent_md5.front() else { return 1 };
    entry.recent_md5.iter().take_while(|m| *m == first).count().max(1)
}

pub fn get(player_name: &str) -> Option<LatestPlay> {
    store().lock().ok().and_then(|map| map.get(&key(player_name)).map(|e| e.latest.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank_score(name: &str, pts: i64) -> Score {
        let mut s: Score = serde_json::from_value(serde_json::json!({
            "mode": 0, "md5": "abc", "name": name, "n300": 1, "n100": 0, "n50": 0,
            "ngeki": 0, "nkatu": 0, "nmiss": 0, "score": pts, "max_combo": 1,
            "perfect": false, "mods": 0, "time": 0, "acc": null, "pp": null,
            "replay_md5": null, "scoreid": null, "mods_str": null,
            "additional_mods": null, "submission_checksum": null, "submission_identity": null
        }))
        .unwrap();
        s.name = name.to_string();
        s
    }

    #[test]
    fn newer_play_replaces_older_and_lookup_ignores_case() {
        let name = "LatestPlayTester";
        assert!(get(name).is_none());
        record(name, &blank_score(name, 100), None, true, Some(3));
        record(name, &blank_score(name, 200), None, false, None);
        let got = get("latestplaytester").expect("recorded");
        assert_eq!(got.score.score, 200);
        assert!(!got.passed);
        assert!(got.rank_after.is_none());
    }

    #[test]
    fn try_number_counts_consecutive_plays_on_the_same_map() {
        let name = "TryCounter";
        assert_eq!(try_number(name), 1);
        let mut a = blank_score(name, 1);
        a.md5 = "map-a".into();
        let mut b = blank_score(name, 2);
        b.md5 = "map-b".into();
        record(name, &a, None, false, None);
        record(name, &a, None, false, None);
        assert_eq!(try_number(name), 2);
        record(name, &b, None, true, None);
        assert_eq!(try_number(name), 1);
        record(name, &b, None, false, None);
        record(name, &b, None, false, None);
        assert_eq!(try_number(name), 3);
    }

    #[test]
    fn players_are_independent() {
        record("LpA", &blank_score("LpA", 1), None, true, None);
        record("LpB", &blank_score("LpB", 2), None, true, None);
        assert_eq!(get("lpa").unwrap().score.score, 1);
        assert_eq!(get("LPB").unwrap().score.score, 2);
    }
}
