
use crate::db;
use crate::pp_era::{self, EraMap, EraScore};
use crate::state::SharedState;
use crate::types::score::Score;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

static FILLING: Mutex<Option<HashSet<String>>> = Mutex::new(None);

pub fn to_era_score(s: &Score) -> EraScore {
    EraScore {
        mode: s.mode.clamp(0, 3) as u8,
        mods: s.mods,
        n300: s.n300.max(0) as u32,
        n100: s.n100.max(0) as u32,
        n50: s.n50.max(0) as u32,
        ngeki: s.ngeki.max(0) as u32,
        nkatu: s.nkatu.max(0) as u32,
        nmiss: s.nmiss.max(0) as u32,
        max_combo: s.max_combo.max(0) as u32,
        score: s.score.clamp(0, u32::MAX as i64) as u32,
        accuracy: crate::utils::calculate_accuracy(
            s.mode as u8,
            s.n300,
            s.n100,
            s.n50,
            s.ngeki,
            s.nkatu,
            s.nmiss,
        ),
    }
}

fn calc_guarded(map: &EraMap, era: &str, score: &EraScore) -> Option<f64> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| pp_era::calculate(map, era, score)))
        .ok()
        .flatten()
}

pub fn calculate_for(map: &EraMap, era: &str, score: &Score) -> Option<f64> {
    calc_guarded(map, era, &to_era_score(score))
}

pub fn store_all_eras(conn: &rusqlite::Connection, score_id: i64, map: &EraMap, score: &Score) {
    if score.mode != 0 {
        return;
    }
    let es = to_era_score(score);
    for era in pp_era::stored_ids() {
        if let Some(pp) = calc_guarded(map, era, &es) {
            let _ = db::upsert_era_pp(conn, score_id, era, pp);
        }
    }
}

pub struct FillReport {
    pub scores: usize,
    pub calculated: usize,
    pub no_map: usize,
}

pub async fn fill(state: &SharedState, player: &str, eras: &[&str], force: bool, fetch: bool) -> FillReport {
    let (scores, http, config, db_handle) = {
        let s = state.read().await;
        let conn = s.db.lock().await;
        let sc: Vec<Score> = db::get_all_scores(&conn, player)
            .unwrap_or_default()
            .into_iter()
            .filter(|x| x.mode == 0)
            .collect();
        (sc, s.http.clone(), s.config.clone(), s.db.clone())
    };

    let mut existing: HashMap<&str, HashMap<i64, f64>> = HashMap::new();
    {
        let conn = db_handle.lock().await;
        for era in eras {
            existing.insert(*era, db::get_era_pp(&conn, player, era).unwrap_or_default());
        }
    }

    let mut report = FillReport { scores: scores.len(), calculated: 0, no_map: 0 };
    let mut maps: HashMap<String, Option<EraMap>> = HashMap::new();

    for score in &scores {
        let Some(id) = score.scoreid else { continue };
        let missing: Vec<&str> = eras
            .iter()
            .copied()
            .filter(|e| force || !existing.get(e).map_or(false, |m| m.contains_key(&id)))
            .collect();
        if missing.is_empty() {
            continue;
        }

        if !maps.contains_key(&score.md5) {
            let bmap = {
                let conn = db_handle.lock().await;
                db::get_beatmap_by_md5(&conn, &score.md5).unwrap_or(None)
            };
            let mut content: Option<String> = bmap.as_ref().and_then(|b| b.file_content.clone());
            if content.is_none() && fetch {
                if let Some(ref b) = bmap {
                    content = crate::utils::get_or_fetch_beatmap_content(&http, &db_handle, &config, b).await;
                }
                if content.is_none() {
                    if let Some(songs_dir) = crate::utils::resolve_songs_folder(&config) {
                        if let Some((local_bmap, local_content)) =
                            crate::utils::find_and_parse_local_osu_file(&songs_dir, None, None, Some(&score.md5), None)
                        {
                            let conn = db_handle.lock().await;
                            let _ = db::insert_beatmap(&conn, &local_bmap);
                            let _ = db::update_beatmap_file_content(&conn, &local_bmap.file_md5, &local_content);
                            content = Some(local_content);
                        }
                    }
                }
                if content.is_none() {
                    if let Some(ref api_key) = config.osu_api_key {
                        if let Some(api_bmap) =
                            crate::utils::fetch_beatmap_from_api(&http, api_key, &[("h", score.md5.clone())]).await
                        {
                            {
                                let conn = db_handle.lock().await;
                                let _ = db::insert_beatmap(&conn, &api_bmap);
                            }
                            content = crate::utils::get_or_fetch_beatmap_content(&http, &db_handle, &config, &api_bmap).await;
                        }
                    }
                }
            }
            maps.insert(score.md5.clone(), content.and_then(|c| EraMap::parse(c.as_bytes())));
        }

        let Some(Some(map)) = maps.get(&score.md5) else {
            report.no_map += 1;
            continue;
        };

        let es = to_era_score(score);
        let conn = db_handle.lock().await;
        for era in missing {
            if let Some(pp) = calc_guarded(map, era, &es) {
                let _ = db::upsert_era_pp(&conn, id, era, pp);
                report.calculated += 1;
            }
        }
    }
    report
}

pub fn count_missing(conn: &rusqlite::Connection, player: &str, era: &str) -> usize {
    let have = db::get_era_pp(conn, player, era).unwrap_or_default();
    db::get_all_scores(conn, player)
        .unwrap_or_default()
        .iter()
        .filter(|s| s.mode == 0 && s.scoreid.map_or(false, |id| !have.contains_key(&id)))
        .count()
}

pub fn spawn_background_fill(state: SharedState, player: String) {
    {
        let mut g = FILLING.lock().unwrap();
        let set = g.get_or_insert_with(HashSet::new);
        if !set.insert(player.clone()) {
            return;
        }
    }
    tokio::spawn(async move {
        let eras: Vec<&str> = pp_era::stored_ids().collect();
        let _ = fill(&state, &player, &eras, false, true).await;
        if let Some(set) = FILLING.lock().unwrap().as_mut() {
            set.remove(&player);
        }
    });
}

pub fn is_filling(player: &str) -> bool {
    FILLING.lock().unwrap().as_ref().map_or(false, |s| s.contains(player))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_all_eras_writes_a_row_per_era_and_reads_back_per_player() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "Alice").unwrap();

        let mut osu = String::from("osu file format v14\n\n[General]\nMode: 0\n\n[Metadata]\nTitle:T\nArtist:A\nCreator:C\nVersion:V\n\n[Difficulty]\nHPDrainRate:5\nCircleSize:4\nOverallDifficulty:8\nApproachRate:9\nSliderMultiplier:1.4\nSliderTickRate:1\n\n[TimingPoints]\n0,350,4,2,0,50,1,0\n\n[HitObjects]\n");
        for i in 0..100 {
            osu.push_str(&format!("{},{},{},1,0,0:0:0:0:\n", 100 + (i * 37) % 300, 80 + (i * 53) % 220, 1000 + i * 200));
        }
        let map = EraMap::parse(osu.as_bytes()).unwrap();

        let score = Score {
            mode: 0,
            md5: "m".into(),
            name: "Alice".into(),
            n300: 100,
            n100: 0,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss: 0,
            max_combo: 100,
            perfect: true,
            mods: 0,
            time: 1,
            score: 400000,
            acc: Some(100.0),
            pp: Some(1.0),
            replay_md5: None,
            scoreid: None,
            replay_frames: None,
            mods_str: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        let id = db::insert_score(&conn, &score, "ranked").unwrap();
        store_all_eras(&conn, id, &map, &score);

        for era in pp_era::stored_ids() {
            let rows = db::get_era_pp(&conn, "Alice", era).unwrap();
            assert!(rows.get(&id).copied().unwrap_or(0.0) > 0.0, "{era} missing");
        }
        assert!(db::get_era_pp(&conn, "Bob", "may14").unwrap().is_empty());
        assert_eq!(count_missing(&conn, "Alice", "may14"), 0);
    }
}
