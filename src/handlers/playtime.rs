
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const MAX_SEGMENT_SECS: i64 = 4 * 3600;

#[derive(Default)]
struct Track {
    playing: Option<(i64, u8)>,
    last_seen: i64,
}

fn tracks() -> &'static Mutex<HashMap<String, Track>> {
    static T: OnceLock<Mutex<HashMap<String, Track>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn is_playing_action(action: u8) -> bool {
    matches!(action, 2 | 12)
}

fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

fn close(conn: &Connection, player: &str, start: i64, mode: u8, end: i64) {
    let secs = (end - start).clamp(0, MAX_SEGMENT_SECS);
    if secs > 0 {
        match crate::db::add_playtime(conn, player, mode as i32, secs) {
            Ok(()) => crate::utils::log(&format!("Play time: {} +{}s (mode {})", player, secs, mode)),
            Err(e) => crate::utils::log(&format!("Play time: could not save {}'s time: {}", player, e)),
        }
    }
}

pub fn touch(player: &str) {
    touch_at(player, now_secs());
}

fn touch_at(player: &str, now: i64) {
    if let Ok(mut t) = tracks().lock() {
        t.entry(player.to_string()).or_default().last_seen = now;
    }
}

pub fn on_action(conn: &Connection, player: &str, action: u8, mode: u8) {
    on_action_at(conn, player, action, mode, now_secs());
}

fn on_action_at(conn: &Connection, player: &str, action: u8, mode: u8, now: i64) {
    let open = {
        let Ok(mut t) = tracks().lock() else { return };
        let track = t.entry(player.to_string()).or_default();
        track.last_seen = now;
        let open = track.playing.take();
        if is_playing_action(action) {
            track.playing = Some((now, mode));
        }
        open
    };
    if let Some((start, m)) = open {
        close(conn, player, start, m, now);
    }
}

pub fn finish(conn: &Connection, player: &str) {
    let open = {
        let Ok(mut t) = tracks().lock() else { return };
        t.remove(player).and_then(|track| track.playing.map(|(start, mode)| (start, mode, track.last_seen)))
    };
    if let Some((start, mode, end)) = open {
        close(conn, player, start, mode, end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn playing_then_idle_adds_the_elapsed_time() {
        let conn = db();
        on_action_at(&conn, "PtA", 2, 0, 1_000);
        on_action_at(&conn, "PtA", 9, 0, 1_095);
        assert_eq!(crate::db::get_playtime(&conn, "PtA", 0).unwrap(), 95);
        on_action_at(&conn, "PtA", 0, 0, 5_000);
        assert_eq!(crate::db::get_playtime(&conn, "PtA", 0).unwrap(), 95);
    }

    #[test]
    fn retrying_a_map_keeps_the_total_continuous() {
        let conn = db();
        on_action_at(&conn, "PtB", 2, 0, 100);
        on_action_at(&conn, "PtB", 2, 0, 130);
        on_action_at(&conn, "PtB", 0, 0, 160);
        assert_eq!(crate::db::get_playtime(&conn, "PtB", 0).unwrap(), 60);
    }

    #[test]
    fn pausing_stops_the_clock_and_resuming_restarts_it() {
        let conn = db();
        on_action_at(&conn, "PtC", 2, 0, 0);
        on_action_at(&conn, "PtC", 10, 0, 40);
        on_action_at(&conn, "PtC", 2, 0, 400);
        on_action_at(&conn, "PtC", 0, 0, 430);
        assert_eq!(crate::db::get_playtime(&conn, "PtC", 0).unwrap(), 70);
    }

    #[test]
    fn multiplayer_counts_and_modes_are_kept_apart() {
        let conn = db();
        on_action_at(&conn, "PtD", 12, 0, 0);
        on_action_at(&conn, "PtD", 5, 0, 120);
        on_action_at(&conn, "PtD", 2, 3, 200);
        on_action_at(&conn, "PtD", 0, 3, 230);
        assert_eq!(crate::db::get_playtime(&conn, "PtD", 0).unwrap(), 120);
        assert_eq!(crate::db::get_playtime(&conn, "PtD", 3).unwrap(), 30);
        assert_eq!(crate::db::get_playtime(&conn, "PtD", 1).unwrap(), 0);
    }

    #[test]
    fn a_vanished_client_is_closed_at_its_last_packet() {
        let conn = db();
        on_action_at(&conn, "PtE", 2, 0, 1_000);
        touch_at("PtE", 1_050);
        touch_at("PtE", 1_080);
        finish(&conn, "PtE");
        assert_eq!(crate::db::get_playtime(&conn, "PtE", 0).unwrap(), 80);
        finish(&conn, "PtE");
        assert_eq!(crate::db::get_playtime(&conn, "PtE", 0).unwrap(), 80);
    }

    #[test]
    fn one_stretch_is_capped() {
        let conn = db();
        on_action_at(&conn, "PtF", 2, 0, 0);
        on_action_at(&conn, "PtF", 0, 0, 10 * 3600);
        assert_eq!(crate::db::get_playtime(&conn, "PtF", 0).unwrap(), MAX_SEGMENT_SECS);
    }

    #[test]
    fn playing_statuses() {
        assert!(is_playing_action(2));
        assert!(is_playing_action(12));
        for a in [0u8, 1, 3, 4, 5, 6, 8, 9, 10, 11, 13] {
            assert!(!is_playing_action(a), "{}", a);
        }
    }
}
