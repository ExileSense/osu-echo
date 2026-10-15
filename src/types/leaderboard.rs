use crate::types::beatmap::Beatmap;
use crate::types::score::LeaderboardEntry;

pub const NOTSUBMITTED: i32 = -1;
pub const PENDING: i32 = 0;
pub const UPDATEAVAILABLE: i32 = 1;
pub const RANKED: i32 = 2;
pub const APPROVED: i32 = 3;
pub const QUALIFIED: i32 = 4;
pub const LOVED: i32 = 5;

pub const VALID_LB_STATUSES: [i32; 4] = [LOVED, QUALIFIED, RANKED, APPROVED];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardTypes {
    Local = 0,
    Top = 1,
    Mods = 2,
    Friends = 3,
    Country = 4,
}

impl LeaderboardTypes {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::Local,
            1 => Self::Top,
            2 => Self::Mods,
            3 => Self::Friends,
            4 => Self::Country,
            _ => Self::Top,
        }
    }
}

pub fn api_to_server_status(approved: i32) -> i32 {
    match approved {
        4 => LOVED,
        3 => QUALIFIED,
        2 => APPROVED,
        1 => RANKED,
        -2..=0 => PENDING,
        _ => PENDING,
    }
}

pub fn status_to_db_key(approved: i32) -> &'static str {
    match approved {
        1 => "ranked",
        2 => "approved",
        3 => "qualified",
        4 => "loved",
        _ => "unranked",
    }
}

pub struct Leaderboard {
    pub scores: Vec<LeaderboardEntry>,
    pub bmap: Option<Beatmap>,
    pub personal_score: Option<LeaderboardEntry>,
    pub personal_rank: Option<i32>,
    pub total_scores: Option<i32>,
}

impl Leaderboard {
    pub fn new() -> Self {
        Self { scores: Vec::new(), bmap: None, personal_score: None, personal_rank: None, total_scores: None }
    }

    pub fn as_binary(&self, amount_of_scores: i32, _show_pp_pb: bool) -> Vec<u8> {
        let Some(bmap) = &self.bmap else {
            return b"0|false".to_vec();
        };

        let r = api_to_server_status(bmap.approved);
        if !VALID_LB_STATUSES.contains(&r) {
            return format!("{}|false", r).into_bytes();
        }

        let artist = bmap.artist_unicode.as_deref().unwrap_or(&bmap.artist);
        let title = bmap.title_unicode.as_deref().unwrap_or(&bmap.title);

        let base = format!("{}|false|{}|{}|{}\n0\n[bold:0,size:20]{}|{}\n10.0\n", r, bmap.beatmap_id, bmap.beatmapset_id, self.total_scores.map(|n| n.max(self.scores.len() as i32)).unwrap_or(self.scores.len() as i32), artist, title,);

        let mut buffer = base.into_bytes();

        if let Some(ps) = &self.personal_score {
            let num = self.personal_rank.unwrap_or_else(|| self
                .scores
                .iter()
                .position(|s| if s.score_id != 0 && ps.score_id != 0 { s.score_id == ps.score_id } else { s.username.eq_ignore_ascii_case(&ps.username) })
                .map(|i| i as i32 + 1)
                .unwrap_or(amount_of_scores + 1));
            buffer.extend_from_slice(ps.format(num).as_bytes());
            buffer.push(b'\n');
        } else {
            buffer.push(b'\n');
        }

        let len_scores = self.scores.len();
        for (idx, s) in self.scores.iter().enumerate() {
            buffer.extend_from_slice(s.format(idx as i32 + 1).as_bytes());
            if idx + 1 != len_scores {
                buffer.push(b'\n');
            }
        }

        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_mappings() {
        let db_keys = [(1, "ranked"), (2, "approved"), (3, "qualified"), (4, "loved"), (0, "unranked"), (-1, "unranked"), (-2, "unranked"), (99, "unranked")];
        for (approved, expected) in db_keys {
            assert_eq!(status_to_db_key(approved), expected);
        }

        let api_statuses = [(1, RANKED), (2, APPROVED), (3, QUALIFIED), (4, LOVED), (0, PENDING), (-2, PENDING)];
        for (approved, expected) in api_statuses {
            assert_eq!(api_to_server_status(approved), expected);
        }
    }

    #[test]
    fn personal_rank_and_total_override_list_position() {
        let mut lb = Leaderboard::new();
        let mut b = Beatmap::blank();
        b.approved = 1;
        b.beatmap_id = 5;
        lb.bmap = Some(b);
        let mk = |u: &str, id: i64| LeaderboardEntry {
            score_id: id, username: u.to_string(), score: 1, maxcombo: 1, count50: 0, count100: 0, count300: 1, countmiss: 0,
            countkatu: 0, countgeki: 0, perfect: 0, enabled_mods: 0, user_id: 0, time: 0, replay_available: 0,
        };
        lb.scores = vec![mk("a", 1), mk("b", 2)];
        lb.personal_score = Some(mk("me", 99));
        let plain = String::from_utf8(lb.as_binary(50, false)).unwrap();
        assert!(plain.starts_with("2|false|5|0|2\n"), "{plain}");
        lb.personal_rank = Some(186);
        lb.total_scores = Some(15600);
        let bin = String::from_utf8(lb.as_binary(50, false)).unwrap();
        assert!(bin.starts_with("2|false|5|0|15600\n"), "{bin}");
        assert!(bin.lines().nth(4).unwrap().starts_with("99|me|"), "{bin}");
        assert!(bin.lines().nth(4).unwrap().contains("|186|"), "{bin}");
        lb.total_scores = Some(1);
        assert!(String::from_utf8(lb.as_binary(50, false)).unwrap().starts_with("2|false|5|0|2\n"));
    }
}
