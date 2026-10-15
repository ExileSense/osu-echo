
use crate::console::TraceSend;
use crate::db;
use crate::types::beatmap::Beatmap;
use crate::types::mods::Mods;
use crate::types::score::Score;
use rusqlite::Connection;
use serde_json::Value;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

const API: &str = "https://osu.ppy.sh/api/v2";
const TOKEN_URL: &str = "https://osu.ppy.sh/oauth/token";
const MODE_NAMES: [&str; 4] = ["osu", "taiko", "fruits", "mania"];

pub type ApiErr = (u16, String);

fn dir() -> PathBuf {
    std::env::current_dir().unwrap_or_default().join(".data")
}
pub fn creds_file() -> PathBuf {
    dir().join("osu_oauth.json")
}

pub fn init() {
    let d = dir();
    let _ = std::fs::create_dir_all(&d);
    let ex = d.join("osu_oauth.example.json");
    if !ex.exists() {
        let _ = std::fs::write(
            ex,
            "{\r\n  \"client_id\": 12345,\r\n  \"client_secret\": \"paste-your-client-secret-here\"\r\n}\r\n",
        );
    }
}

struct Creds {
    id: u64,
    secret: String,
}

fn load_creds() -> Result<Creds, ApiErr> {
    let path = creds_file();
    let missing = || {
        (
            400u16,
            format!(
                "osu! API credentials are not set up. Create an OAuth application at https://osu.ppy.sh/home/account/edit (OAuth section), \
                 then save its Client ID and Client Secret in {} as {{\"client_id\": 123, \"client_secret\": \"...\"}} (see osu_oauth.example.json beside it).",
                path.display()
            ),
        )
    };
    let text = std::fs::read_to_string(&path).map_err(|_| missing())?;
    let v: Value = serde_json::from_str(&text).map_err(|_| (400u16, format!("{} is not valid JSON.", path.display())))?;
    let id = match v.get("client_id") {
        Some(Value::Number(n)) => n.as_u64(),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    };
    let secret = v.get("client_secret").and_then(|s| s.as_str()).unwrap_or("").trim().to_string();
    match id {
        Some(id) if id > 0 && !secret.is_empty() && !secret.starts_with("paste-") && id != 12345 => Ok(Creds { id, secret }),
        _ => Err(missing()),
    }
}

static BONUS: RwLock<Option<HashMap<(String, i32), f64>>> = RwLock::new(None);

pub fn bonus(name: &str, mode: i32) -> Option<f64> {
    BONUS.read().ok()?.as_ref()?.get(&(name.to_string(), mode)).copied()
}
pub fn load_bonuses(conn: &Connection) {
    let rows = db::all_pp_bonus(conn).unwrap_or_default();
    if let Ok(mut w) = BONUS.write() {
        *w = Some(rows.into_iter().map(|(n, m, b)| ((n, m), b)).collect());
    }
}
pub fn set_bonus(conn: &Connection, name: &str, mode: i32, value: f64) {
    if db::set_pp_bonus(conn, name, mode, value).is_ok() {
        if let Ok(mut w) = BONUS.write() {
            w.get_or_insert_with(HashMap::new).insert((name.to_string(), mode), value);
        }
    }
}
pub fn clear_bonus(name: &str) {
    if let Ok(mut w) = BONUS.write() {
        if let Some(m) = w.as_mut() {
            m.retain(|(n, _), _| n != name);
        }
    }
}
pub fn rename_bonus(old: &str, new: &str) {
    if let Ok(mut w) = BONUS.write() {
        if let Some(m) = w.as_mut() {
            let moved: Vec<(i32, f64)> = m.iter().filter(|((n, _), _)| n == old).map(|((_, md), b)| (*md, *b)).collect();
            m.retain(|(n, _), _| n != old);
            for (md, b) in moved {
                m.insert((new.to_string(), md), b);
            }
        }
    }
}

pub fn weighted_pp(items: &[Converted]) -> f64 {
    let mut best: HashMap<&str, f64> = HashMap::new();
    for c in items {
        if let Some(pp) = c.score.pp {
            let e = best.entry(c.score.md5.as_str()).or_insert(0.0);
            if pp > *e {
                *e = pp;
            }
        }
    }
    let mut v: Vec<f64> = best.into_values().collect();
    v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    v.iter().enumerate().map(|(i, p)| p * 0.95_f64.powi(i as i32)).sum()
}

static TOKEN: Mutex<Option<(String, Instant)>> = Mutex::new(None);

async fn token(http: &reqwest::Client, force: bool) -> Result<String, ApiErr> {
    if !force {
        if let Some((t, exp)) = TOKEN.lock().unwrap().as_ref() {
            if Instant::now() < *exp {
                return Ok(t.clone());
            }
        }
    }
    let c = load_creds()?;
    let resp = http
        .post(TOKEN_URL)
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": c.id,
            "client_secret": c.secret,
            "grant_type": "client_credentials",
            "scope": "public",
        }))
        .timeout(Duration::from_secs(15))
        .send_traced()
        .await
        .map_err(|e| (502u16, format!("Could not reach osu.ppy.sh: {e}")))?;
    if !resp.status().is_success() {
        return Err((
            401,
            format!("osu! rejected the Client ID/Secret in {} (status {}). Check them at your OAuth application page.", creds_file().display(), resp.status()),
        ));
    }
    let v: Value = resp.json().await.map_err(|e| (502u16, format!("Unexpected token response: {e}")))?;
    let t = v.get("access_token").and_then(|t| t.as_str()).ok_or((502u16, "No access token in response.".to_string()))?.to_string();
    let ttl = v.get("expires_in").and_then(|x| x.as_u64()).unwrap_or(3600).saturating_sub(120).max(60);
    *TOKEN.lock().unwrap() = Some((t.clone(), Instant::now() + Duration::from_secs(ttl)));
    Ok(t)
}

static NEXT_SLOT: Mutex<Option<Instant>> = Mutex::new(None);

async fn pace() {
    let wait = {
        let mut g = NEXT_SLOT.lock().unwrap();
        let now = Instant::now();
        let slot = match *g {
            Some(t) if t > now => t,
            _ => now,
        };
        *g = Some(slot + Duration::from_millis(1100));
        slot.saturating_duration_since(now)
    };
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
}

pub async fn get(http: &reqwest::Client, path: &str, query: &[(&str, String)]) -> Result<Value, ApiErr> {
    let mut force = false;
    let mut limited = false;
    for _ in 0..3 {
        let tok = token(http, force).await?;
        pace().await;
        let resp = http
            .get(format!("{API}{path}"))
            .query(query)
            .header("Accept", "application/json")
            .bearer_auth(tok)
            .timeout(Duration::from_secs(20))
            .send_traced()
            .await
            .map_err(|e| (502u16, format!("Could not reach osu.ppy.sh: {e}")))?;
        match resp.status().as_u16() {
            200 => return resp.json().await.map_err(|e| (502u16, format!("Unexpected response from osu!: {e}"))),
            401 if !force => {
                force = true;
                continue;
            }
            404 => return Err((404, "Not found on osu.ppy.sh.".to_string())),
            429 if !limited => {
                limited = true;
                tokio::time::sleep(Duration::from_secs(20)).await;
                continue;
            }
            429 => return Err((429, "osu! rate limit hit; wait a minute and try again.".to_string())),
            s => return Err((502, format!("osu! API returned status {s}."))),
        }
    }
    Err((401, "osu! rejected the access token.".to_string()))
}

pub async fn user(http: &reqwest::Client, query: &str) -> Result<Value, ApiErr> {
    let q = query.trim();
    let key = if !q.is_empty() && q.chars().all(|c| c.is_ascii_digit()) { "id" } else { "username" };
    let enc = percent_encoding::utf8_percent_encode(q, percent_encoding::NON_ALPHANUMERIC).to_string();
    get(http, &format!("/users/{enc}/osu"), &[("key", key.to_string())])
        .await
        .map_err(|e| if e.0 == 404 { (404, format!("User '{q}' was not found on osu.ppy.sh.")) } else { e })
}

pub struct Converted {
    pub score: Score,
    pub ext_id: i64,
    pub beatmap_id: i64,
    pub beatmap: Beatmap,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn i(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_i64()).unwrap_or(0)
}
fn f(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0)
}
fn stat(st: &Value, classic: &str, lazer: &str) -> i32 {
    st.get(classic).and_then(|x| x.as_i64()).or_else(|| st.get(lazer).and_then(|x| x.as_i64())).unwrap_or(0) as i32
}
fn date(v: &Value, k: &str) -> String {
    s(v, k).replace('T', " ").trim_end_matches('Z').split('+').next().unwrap_or("").to_string()
}

fn status_code(st: &str) -> i32 {
    match st {
        "ranked" => 1,
        "approved" => 2,
        "qualified" => 3,
        "loved" => 4,
        _ => 0,
    }
}

pub fn mods_from_json(v: Option<&Value>) -> Mods {
    let mut text = String::new();
    if let Some(Value::Array(a)) = v {
        for m in a {
            let ac = match m {
                Value::String(s) => s.as_str(),
                Value::Object(o) => o.get("acronym").and_then(|x| x.as_str()).unwrap_or(""),
                _ => "",
            };
            text.push_str(ac);
        }
    }
    let mut m = Mods::from_short_str(&text);
    if m.contains(Mods::NIGHTCORE) {
        m |= Mods::DOUBLETIME;
    }
    m
}

pub fn convert(sc: &Value, player: &str) -> Option<Converted> {
    let ext_id = sc.get("id").and_then(|x| x.as_i64()).or_else(|| sc.get("best_id").and_then(|x| x.as_i64()))?;
    let bm = sc.get("beatmap")?;
    let set = sc.get("beatmapset").cloned().unwrap_or(Value::Null);
    let beatmap_id = i(bm, "id");
    if beatmap_id <= 0 {
        return None;
    }
    let mode = sc.get("mode_int").and_then(|x| x.as_i64()).map(|x| x as i32).or_else(|| {
        sc.get("mode").and_then(|m| m.as_str()).and_then(|m| MODE_NAMES.iter().position(|n| *n == m)).map(|p| p as i32)
    })?;
    let st = sc.get("statistics").cloned().unwrap_or(Value::Null);
    let time = chrono::DateTime::parse_from_rfc3339(&s(sc, "created_at")).map(|d| d.timestamp()).unwrap_or(0);
    let md5 = s(bm, "checksum");
    let mods = mods_from_json(sc.get("mods"));
    let total = sc
        .get("score")
        .and_then(|x| x.as_i64())
        .or_else(|| sc.get("legacy_total_score").and_then(|x| x.as_i64()))
        .or_else(|| sc.get("total_score").and_then(|x| x.as_i64()))
        .unwrap_or(0);

    let score = Score {
        mode,
        md5: md5.clone(),
        name: player.to_string(),
        n300: stat(&st, "count_300", "great"),
        n100: stat(&st, "count_100", "ok"),
        n50: stat(&st, "count_50", "meh"),
        ngeki: stat(&st, "count_geki", "perfect"),
        nkatu: stat(&st, "count_katu", "good"),
        nmiss: stat(&st, "count_miss", "miss"),
        score: total,
        max_combo: i(sc, "max_combo") as i32,
        perfect: sc.get("perfect").and_then(|x| x.as_bool()).unwrap_or(false),
        mods: mods.bits(),
        time,
        acc: Some(f(sc, "accuracy") * 100.0),
        pp: sc.get("pp").and_then(|x| x.as_f64()),
        replay_md5: None,
        scoreid: None,
        replay_frames: None,
        mods_str: Some(mods.short_name()),
        additional_mods: None,
        submission_checksum: None,
        submission_identity: Some(format!("import:{ext_id}")),
    };

    let mut b = Beatmap::blank();
    b.file_md5 = md5;
    b.beatmap_id = beatmap_id;
    b.beatmapset_id = i(bm, "beatmapset_id").max(i(&set, "id"));
    b.approved = status_code(&s(bm, "status"));
    b.total_length = i(bm, "total_length") as i32;
    b.hit_length = i(bm, "hit_length") as i32;
    b.version = s(bm, "version");
    b.diff_size = f(bm, "cs");
    b.diff_overall = f(bm, "accuracy");
    b.diff_approach = f(bm, "ar");
    b.diff_drain = f(bm, "drain");
    b.mode = i(bm, "mode_int") as i32;
    if sc.get("mode_int").is_none() && bm.get("mode_int").is_none() {
        b.mode = mode;
    }
    b.count_normal = i(bm, "count_circles") as i32;
    b.count_slider = i(bm, "count_sliders") as i32;
    b.count_spinner = i(bm, "count_spinners") as i32;
    b.submit_date = date(&set, "submitted_date");
    let ranked = date(&set, "ranked_date");
    b.approved_date = if ranked.is_empty() { None } else { Some(ranked) };
    b.last_update = {
        let d = date(bm, "last_updated");
        if d.is_empty() { date(&set, "last_updated") } else { d }
    };
    b.artist = s(&set, "artist");
    b.artist_unicode = Some(s(&set, "artist_unicode")).filter(|x| !x.is_empty());
    b.title = s(&set, "title");
    b.title_unicode = Some(s(&set, "title_unicode")).filter(|x| !x.is_empty());
    b.creator = s(&set, "creator");
    b.creator_id = i(&set, "user_id");
    b.bpm = f(bm, "bpm").max(f(&set, "bpm"));
    b.source = s(&set, "source");
    b.favourite_count = i(&set, "favourite_count") as i32;
    b.storyboard = set.get("storyboard").and_then(|x| x.as_bool()).unwrap_or(false) as i32;
    b.video = set.get("video").and_then(|x| x.as_bool()).unwrap_or(false) as i32;
    b.playcount = i(bm, "playcount") as i32;
    b.passcount = i(bm, "passcount") as i32;
    b.max_combo = i(bm, "max_combo") as i32;
    b.difficultyrating = f(bm, "difficulty_rating");
    Some(Converted { score, ext_id, beatmap_id, beatmap: b })
}

#[derive(Debug, Default, serde::Serialize)]
pub struct Summary {
    pub imported: usize,
    pub skipped_existing: usize,
    pub skipped_unusable: usize,
    pub per_mode: Vec<(String, usize)>,
}

pub fn store(conn: &Connection, player: &str, items: &[Converted]) -> (usize, usize, usize) {
    let _ = db::ensure_profile(conn, player);
    let (mut imported, mut existing, mut unusable) = (0, 0, 0);
    for it in items {
        if it.score.md5.len() != 32 {
            unusable += 1;
            continue;
        }
        let ident = it.score.submission_identity.clone().unwrap_or_default();
        if let Ok(Some(id)) = db::check_duplicate_score(conn, None, None, Some(&ident)) {
            if let Some(pp) = it.score.pp {
                let _ = db::update_score_pp_and_acc(conn, id, pp, it.score.acc.unwrap_or(0.0));
            }
            existing += 1;
            continue;
        }
        if matches!(db::get_beatmap_by_md5(conn, &it.score.md5), Ok(None) | Err(_)) {
            let _ = db::insert_beatmap(conn, &it.beatmap);
        }
        let status = match it.beatmap.approved {
            1 => "ranked",
            2 => "approved",
            3 => "qualified",
            4 => "loved",
            _ if it.score.pp.is_some() => "ranked",
            _ => "unranked",
        };
        if db::insert_score(conn, &it.score, status).is_ok() {
            imported += 1;
        } else {
            unusable += 1;
        }
    }
    (imported, existing, unusable)
}

async fn fill_checksums(http: &reqwest::Client, items: &mut [Converted]) {
    let ids: Vec<i64> = items.iter().filter(|c| c.score.md5.len() != 32).map(|c| c.beatmap_id).collect();
    for chunk in ids.chunks(50) {
        let q: Vec<(&str, String)> = chunk.iter().map(|id| ("ids[]", id.to_string())).collect();
        if let Ok(v) = get(http, "/beatmaps", &q).await {
            if let Some(arr) = v.get("beatmaps").and_then(|a| a.as_array()) {
                for b in arr {
                    let (id, sum) = (i(b, "id"), s(b, "checksum"));
                    if sum.len() == 32 {
                        for c in items.iter_mut().filter(|c| c.beatmap_id == id) {
                            c.score.md5 = sum.clone();
                            c.beatmap.file_md5 = sum.clone();
                        }
                    }
                }
            }
        }
    }
}

pub async fn map_ranks(http: &reqwest::Client, uid: i64, mode: i32, maps: &[(String, i64)]) -> Result<Vec<(String, Option<i64>, Option<i64>)>, ApiErr> {
    let name = MODE_NAMES.get(mode as usize).copied().unwrap_or("osu");
    let mut passes: HashMap<i64, i64> = HashMap::new();
    for chunk in maps.chunks(50) {
        let q: Vec<(&str, String)> = chunk.iter().map(|(_, id)| ("ids[]", id.to_string())).collect();
        if let Ok(v) = get(http, "/beatmaps", &q).await {
            if let Some(arr) = v.get("beatmaps").and_then(|a| a.as_array()) {
                for b in arr {
                    let pc = i(b, "passcount");
                    if pc > 0 {
                        passes.insert(i(b, "id"), pc);
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    for (md5, id) in maps {
        let rank = match get(http, &format!("/beatmaps/{id}/scores/users/{uid}"), &[("mode", name.to_string())]).await {
            Ok(v) => v.get("position").and_then(|p| p.as_i64()).filter(|p| *p > 0),
            Err((404, _)) => None,
            Err(e) => return Err(e),
        };
        out.push((md5.clone(), rank, passes.get(id).copied()));
    }
    Ok(out)
}

pub struct ModeFetch {
    pub mode: i32,
    pub raw: usize,
    pub items: Vec<Converted>,
    pub official_pp: Option<f64>,
    pub firsts: Vec<Converted>,
    pub stats: Option<Value>,
}

pub async fn fetch(http: &reqwest::Client, query: &str, player: &str, modes: &[i32], limit: usize) -> Result<(String, Vec<ModeFetch>), ApiErr> {
    let q = query.trim();
    let key = if !q.is_empty() && q.chars().all(|c| c.is_ascii_digit()) { "id" } else { "username" };
    let enc = percent_encoding::utf8_percent_encode(q, percent_encoding::NON_ALPHANUMERIC).to_string();
    let user = get(http, &format!("/users/{enc}/osu"), &[("key", key.to_string())])
        .await
        .map_err(|e| if e.0 == 404 { (404, format!("User '{q}' was not found on osu.ppy.sh.")) } else { e })?;
    let uid = i(&user, "id");
    let uname = s(&user, "username");
    if uid <= 0 {
        return Err((404, format!("User '{q}' was not found on osu.ppy.sh.")));
    }
    let mut out = Vec::new();
    for &m in modes {
        let name = MODE_NAMES.get(m as usize).copied().unwrap_or("osu");
        let mut items: Vec<Converted> = Vec::new();
        let mut offset = 0usize;
        let mut raw = 0usize;
        while offset < limit {
            let page = (limit - offset).min(100);
            let v = get(
                http,
                &format!("/users/{uid}/scores/best"),
                &[("mode", name.to_string()), ("limit", page.to_string()), ("offset", offset.to_string())],
            )
            .await?;
            let arr = match v.as_array() {
                Some(a) => a.clone(),
                None => {
                    let t: String = v.to_string().chars().take(200).collect();
                    return Err((502, format!("Unexpected reply from osu! for {name} scores: {t}")));
                }
            };
            let n = arr.len();
            raw += n;
            items.extend(arr.iter().filter_map(|x| convert(x, player)));
            if n < page {
                break;
            }
            offset += page;
        }
        fill_checksums(http, &mut items).await;
        let stats: Option<Value> = match get(http, &format!("/users/{uid}/{name}"), &[("key", "id".to_string())]).await {
            Ok(u) => u.get("statistics").cloned().filter(|s| s.is_object()),
            Err(_) => None,
        };
        let official_pp = stats.as_ref().and_then(|s| s.get("pp")).and_then(|p| p.as_f64()).filter(|p| *p > 0.0);
        let mut firsts: Vec<Converted> = Vec::new();
        let mut off = 0usize;
        while off < limit {
            let page = (limit - off).min(100);
            let q = [("mode", name.to_string()), ("limit", page.to_string()), ("offset", off.to_string())];
            let Ok(v) = get(http, &format!("/users/{uid}/scores/firsts"), &q).await else { break };
            let Some(arr) = v.as_array() else { break };
            let n = arr.len();
            firsts.extend(arr.iter().filter_map(|x| convert(x, player)));
            if n < page {
                break;
            }
            off += page;
        }
        fill_checksums(http, &mut firsts).await;
        out.push(ModeFetch { mode: m, raw, items, official_pp, firsts, stats });
    }
    Ok((uname, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Value {
        serde_json::json!({
            "id": 4242, "accuracy": 0.9853, "created_at": "2024-03-05T12:34:56Z", "max_combo": 512,
            "mode": "osu", "mode_int": 0, "mods": ["HD", "NC"], "perfect": false, "pp": 301.5, "rank": "A", "score": 12345678,
            "statistics": {"count_300": 400, "count_100": 9, "count_50": 1, "count_geki": 80, "count_katu": 5, "count_miss": 2},
            "beatmap": {"id": 77, "beatmapset_id": 55, "version": "Insane", "checksum": "0123456789abcdef0123456789abcdef", "status": "ranked",
                        "mode_int": 0, "total_length": 120, "hit_length": 110, "cs": 4.0, "ar": 9.0, "accuracy": 8.0, "drain": 5.0, "bpm": 180.0,
                        "count_circles": 300, "count_sliders": 100, "count_spinners": 2, "difficulty_rating": 5.5},
            "beatmapset": {"id": 55, "artist": "Artist", "title": "Title", "creator": "Mapper", "user_id": 9, "submitted_date": "2020-01-02T03:04:05+00:00", "ranked_date": "2020-02-01T00:00:00+00:00"}
        })
    }

    #[test]
    fn converts_score_and_beatmap() {
        let c = convert(&sample(), "me").unwrap();
        assert_eq!(c.score.mode, 0);
        assert_eq!(c.score.n300, 400);
        assert_eq!(c.score.nmiss, 2);
        assert_eq!(c.score.time, 1709642096);
        assert!((c.score.acc.unwrap() - 98.53).abs() < 1e-9);
        assert_eq!(c.score.pp, Some(301.5));
        assert!(Mods::from_bits_truncate(c.score.mods).contains(Mods::NIGHTCORE | Mods::DOUBLETIME | Mods::HIDDEN));
        assert_eq!(c.score.submission_identity.as_deref(), Some("import:4242"));
        assert_eq!(c.beatmap.file_md5, "0123456789abcdef0123456789abcdef");
        assert_eq!(c.beatmap.approved, 1);
        assert_eq!(c.beatmap.artist, "Artist");
        assert_eq!(c.beatmap.submit_date, "2020-01-02 03:04:05");
    }

    #[test]
    fn reimport_restores_official_pp_on_existing_scores() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        let c = convert(&sample(), "me").unwrap();
        let (imported, _, _) = store(&conn, "me", std::slice::from_ref(&c));
        assert_eq!(imported, 1);
        let id = db::check_duplicate_score(&conn, None, None, Some("import:4242")).unwrap().unwrap();
        db::update_score_pp_and_acc(&conn, id, 111.0, 90.0).unwrap();
        let (imported, existing, _) = store(&conn, "me", std::slice::from_ref(&c));
        assert_eq!((imported, existing), (0, 1));
        let scores = db::get_all_scores(&conn, "me").unwrap();
        assert_eq!(scores[0].pp, Some(301.5));
    }

    #[test]
    fn lazer_style_statistics_are_read() {
        let mut v = sample();
        v["statistics"] = serde_json::json!({"great": 300, "ok": 7, "meh": 2, "miss": 1});
        let c = convert(&v, "me").unwrap();
        assert_eq!((c.score.n300, c.score.n100, c.score.n50, c.score.nmiss), (300, 7, 2, 1));
    }

    #[test]
    fn lazer_style_mod_objects_and_unknown_mods() {
        let m = mods_from_json(Some(&serde_json::json!([{"acronym": "DT"}, {"acronym": "CL"}, "HR"])));
        assert!(m.contains(Mods::DOUBLETIME | Mods::HARDROCK));
    }

    #[test]
    fn map_ranks_are_stored_and_top_maps_listed() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        store(&conn, "me", &[convert(&sample(), "me").unwrap()]);
        let maps = db::top_play_maps(&conn, "me", 0, 0, 10);
        assert_eq!(maps, vec![("0123456789abcdef0123456789abcdef".to_string(), 77)]);
        assert!(db::top_play_maps(&conn, "me", 0, 1, 10).is_empty());
        let md5 = &maps[0].0;
        assert_eq!(db::get_map_rank(&conn, "me", 0, md5), None);
        db::set_map_rank(&conn, "me", 0, md5, 186, 15600);
        assert_eq!(db::get_map_rank(&conn, "me", 0, md5), Some((186, 15600)));
        db::rename_profile(&conn, "me", "you").ok();
    }

    #[test]
    fn store_is_idempotent_and_skips_missing_checksum() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        let mut no_sum = sample();
        no_sum["id"] = serde_json::json!(5);
        no_sum["beatmap"]["checksum"] = serde_json::json!(null);
        let items = vec![convert(&sample(), "me").unwrap(), convert(&no_sum, "me").unwrap()];
        assert_eq!(store(&conn, "me", &items), (1, 0, 1));
        assert_eq!(store(&conn, "me", &items), (0, 1, 1));
        let scores = db::get_ranked_scores(&conn, "me").unwrap();
        assert_eq!(scores.len(), 1);
        assert!(db::get_beatmap_by_md5(&conn, "0123456789abcdef0123456789abcdef").unwrap().is_some());
    }

    #[test]
    fn weighting_counts_every_play_and_uses_imported_bonus() {
        let items: Vec<Converted> = (0..150)
            .map(|k| {
                let mut v = sample();
                v["id"] = serde_json::json!(1000 + k);
                v["pp"] = serde_json::json!(100.0);
                v["beatmap"]["id"] = serde_json::json!(5000 + k);
                v["beatmap"]["checksum"] = serde_json::json!(format!("{:032x}", k + 1));
                convert(&v, "bonus-test").unwrap()
            })
            .collect();
        let expected: f64 = (0..150).map(|i| 100.0 * 0.95_f64.powi(i)).sum();
        assert!((weighted_pp(&items) - expected).abs() < 1e-6);

        let scores: Vec<Score> = items.iter().map(|c| c.score.clone()).collect();
        let mut p = crate::types::player::Player::new("bonus-test".to_string());
        p.calculate_stats(&scores, None, 10);
        let default_bonus = 416.6667 * (1.0 - 0.9994_f64.powi(150));
        assert_eq!(p.pp, (expected + default_bonus).round() as i32);

        let conn = Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        set_bonus(&conn, "bonus-test", 0, 300.0);
        p.calculate_stats(&scores, None, 10);
        assert_eq!(p.pp, (expected + 300.0).round() as i32);
        assert!((p.pp_exact - (expected + 300.0)).abs() < 1e-3, "exact pp must keep the decimals");
        clear_bonus("bonus-test");
    }

    #[test]
    fn first_places_are_stored_and_replaced_and_loved_maps_are_not_ranked() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        let mut v = sample();
        v["id"] = serde_json::json!(9);
        v["pp"] = serde_json::json!(null);
        v["beatmap"]["status"] = serde_json::json!("loved");
        let first = convert(&v, "fp").unwrap();
        assert_eq!(store(&conn, "fp", std::slice::from_ref(&first)), (1, 0, 0));
        assert!(db::get_ranked_scores(&conn, "fp").unwrap().is_empty());
        let md5 = first.score.md5.clone();
        db::replace_first_places(&conn, "fp", 0, &[md5.clone()]).unwrap();
        assert!(db::get_first_place_md5s(&conn, "fp", 0).contains(&md5));
        assert!(db::get_first_place_md5s(&conn, "fp", 1).is_empty());
        db::replace_first_places(&conn, "fp", 0, &[]).unwrap();
        assert!(db::get_first_place_md5s(&conn, "fp", 0).is_empty());
    }
}
