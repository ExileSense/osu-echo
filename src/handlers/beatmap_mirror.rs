use crate::console::TraceSend;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const TTL: Duration = Duration::from_secs(600);
pub const PAGE_SIZE: usize = 50;

type Cache = Mutex<HashMap<String, (Instant, Value)>>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached(key: &str, fresh_only: bool) -> Option<Value> {
    let c = cache().lock().ok()?;
    let (at, v) = c.get(key)?;
    if fresh_only && at.elapsed() > TTL {
        return None;
    }
    Some(v.clone())
}

fn put(key: &str, v: &Value) {
    if let Ok(mut c) = cache().lock() {
        if c.len() > 400 {
            c.clear();
        }
        c.insert(key.to_string(), (Instant::now(), v.clone()));
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchParams {
    pub q: String,
    pub mode: Option<i32>,
    pub status: String,
    pub sort: String,
    pub page: usize,
    pub nsfw: bool,
}

fn get_s(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(x) = v.get(*k) {
            if let Some(s) = x.as_str() {
                return s.to_string();
            }
            if x.is_number() {
                return x.to_string();
            }
        }
    }
    String::new()
}

fn get_f(v: &Value, keys: &[&str]) -> f64 {
    for k in keys {
        if let Some(x) = v.get(*k) {
            if let Some(f) = x.as_f64() {
                return f;
            }
            if let Some(s) = x.as_str() {
                if let Ok(f) = s.parse() {
                    return f;
                }
            }
        }
    }
    0.0
}

fn get_i(v: &Value, keys: &[&str]) -> i64 {
    get_f(v, keys) as i64
}

fn get_b(v: &Value, keys: &[&str]) -> bool {
    for k in keys {
        if let Some(x) = v.get(*k) {
            if let Some(b) = x.as_bool() {
                return b;
            }
            if let Some(n) = x.as_i64() {
                return n != 0;
            }
        }
    }
    false
}

pub fn status_name(v: &Value) -> &'static str {
    let from_int = |n: i64| match n {
        1 => "ranked",
        2 => "approved",
        3 => "qualified",
        4 => "loved",
        0 => "pending",
        -1 => "wip",
        _ => "graveyard",
    };
    for k in ["status", "RankedStatus", "ranked"] {
        if let Some(x) = v.get(k) {
            if let Some(s) = x.as_str() {
                return match s.to_ascii_lowercase().as_str() {
                    "ranked" => "ranked",
                    "approved" => "approved",
                    "qualified" => "qualified",
                    "loved" => "loved",
                    "pending" => "pending",
                    "wip" => "wip",
                    _ => "graveyard",
                };
            }
            if let Some(n) = x.as_i64() {
                return from_int(n);
            }
        }
    }
    "graveyard"
}

fn normalize_map(b: &Value, set_id: i64) -> Value {
    let mode = if b.get("mode_int").is_some() || b.get("Mode").is_some() {
        get_i(b, &["mode_int", "Mode"])
    } else {
        match b.get("mode").and_then(|m| m.as_str()) {
            Some("taiko") => 1,
            Some("fruits") => 2,
            Some("mania") => 3,
            _ => 0,
        }
    };
    json!({
        "id": get_i(b, &["id", "BeatmapID"]),
        "set_id": set_id,
        "version": get_s(b, &["version", "DiffName"]),
        "mode": mode,
        "stars": get_f(b, &["difficulty_rating", "DifficultyRating"]),
        "cs": get_f(b, &["cs", "CS", "DiffCircleSize"]),
        "ar": get_f(b, &["ar", "AR", "DiffApproach"]),
        "od": get_f(b, &["accuracy", "od", "OD", "DiffOverall"]),
        "hp": get_f(b, &["drain", "hp", "HP", "DiffDrain"]),
        "bpm": get_f(b, &["bpm", "BPM"]),
        "length": get_i(b, &["total_length", "TotalLength"]),
        "hit_length": get_i(b, &["hit_length", "HitLength"]),
        "max_combo": get_i(b, &["max_combo", "MaxCombo"]),
        "circles": get_i(b, &["count_circles"]),
        "sliders": get_i(b, &["count_sliders"]),
        "spinners": get_i(b, &["count_spinners"]),
        "playcount": get_i(b, &["playcount", "Playcount"]),
        "passcount": get_i(b, &["passcount", "Passcount"]),
        "checksum": get_s(b, &["checksum", "FileMD5"]),
    })
}

pub fn normalize_set(s: &Value) -> Option<Value> {
    let id = get_i(s, &["id", "SetID", "beatmapset_id"]);
    if id <= 0 {
        return None;
    }
    let maps_v = s
        .get("beatmaps")
        .or_else(|| s.get("ChildrenBeatmaps"))
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();
    let mut maps: Vec<Value> = maps_v.iter().map(|b| normalize_map(b, id)).collect();
    maps.sort_by(|a, b| {
        let ma = a["mode"].as_i64().unwrap_or(0);
        let mb = b["mode"].as_i64().unwrap_or(0);
        ma.cmp(&mb).then(
            a["stars"]
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&b["stars"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    let desc = match s.get("description") {
        Some(d) if d.is_object() => get_s(d, &["description"]),
        Some(d) => d.as_str().unwrap_or("").to_string(),
        None => String::new(),
    };
    let named = |keys: &[&str]| -> String {
        for k in keys {
            match s.get(*k) {
                Some(o) if o.is_object() => return get_s(o, &["name"]),
                Some(o) if o.is_string() => return o.as_str().unwrap_or("").to_string(),
                _ => {}
            }
        }
        String::new()
    };
    let ratings: Vec<i64> = s
        .get("ratings")
        .and_then(|r| r.as_array())
        .map(|a| a.iter().map(|x| x.as_i64().unwrap_or(0)).collect())
        .or_else(|| {
            s.get("ratings_string").and_then(|r| r.as_str()).map(|r| {
                r.split(',').map(|x| x.trim().parse::<i64>().unwrap_or(0)).collect()
            })
        })
        .unwrap_or_default();
    let bpm = {
        let b = get_f(s, &["bpm", "BPM"]);
        if b > 0.0 { b } else { maps.iter().filter_map(|m| m["bpm"].as_f64()).find(|b| *b > 0.0).unwrap_or(0.0) }
    };
    Some(json!({
        "id": id,
        "title": get_s(s, &["title", "Title"]),
        "title_unicode": get_s(s, &["title_unicode"]),
        "artist": get_s(s, &["artist", "Artist"]),
        "artist_unicode": get_s(s, &["artist_unicode"]),
        "creator": get_s(s, &["creator", "Creator"]),
        "user_id": get_i(s, &["user_id", "CreatorID"]),
        "status": status_name(s),
        "ranked_date": get_s(s, &["ranked_date", "ApprovedDate"]),
        "submitted_date": get_s(s, &["submitted_date"]),
        "last_updated": get_s(s, &["last_updated", "LastUpdate"]),
        "favourites": get_i(s, &["favourite_count", "Favourites"]),
        "plays": get_i(s, &["play_count"]),
        "source": get_s(s, &["source", "Source"]),
        "tags": get_s(s, &["tags", "Tags"]),
        "genre": named(&["genre", "Genre"]),
        "language": named(&["language", "Language"]),
        "video": get_b(s, &["video", "HasVideo"]),
        "storyboard": get_b(s, &["storyboard"]),
        "nsfw": get_b(s, &["nsfw"]),
        "bpm": bpm,
        "description": desc,
        "ratings": ratings,
        "beatmaps": maps,
    }))
}

fn normalize_list(v: &Value) -> Vec<Value> {
    let arr = if let Some(a) = v.as_array() {
        a.clone()
    } else {
        v.get("beatmapsets").and_then(|a| a.as_array()).cloned().unwrap_or_default()
    };
    arr.iter().filter_map(normalize_set).collect()
}

fn nerinyan_status(st: &str) -> &'static str {
    match st {
        "ranked" => "ranked",
        "qualified" => "qualified",
        "loved" => "loved",
        "pending" => "pending",
        "graveyard" => "graveyard",
        "any" => "all",
        _ => "ranked,approved,loved,qualified",
    }
}

fn nerinyan_sort(sort: &str) -> &'static str {
    match sort {
        "title_asc" => "title_asc",
        "title_desc" => "title_desc",
        "artist_asc" => "artist_asc",
        "artist_desc" => "artist_desc",
        "plays_desc" => "plays_desc",
        "plays_asc" => "plays_asc",
        "favourites_desc" => "favourites_desc",
        "favourites_asc" => "favourites_asc",
        "updated_desc" => "updated_desc",
        "updated_asc" => "updated_asc",
        "ranked_asc" => "ranked_asc",
        _ => "ranked_desc",
    }
}

async fn search_nerinyan(http: &reqwest::Client, p: &SearchParams) -> Result<Vec<Value>, String> {
    let mut q: Vec<(&str, String)> = vec![
        ("q", p.q.clone()),
        ("s", nerinyan_status(&p.status).to_string()),
        ("sort", nerinyan_sort(&p.sort).to_string()),
        ("p", p.page.to_string()),
        ("ps", PAGE_SIZE.to_string()),
        ("nsfw", p.nsfw.to_string()),
    ];
    if let Some(m) = p.mode {
        q.push(("m", m.to_string()));
    }
    let r = http
        .get("https://api.nerinyan.moe/search")
        .query(&q)
        .timeout(Duration::from_secs(15))
        .send_traced()
        .await
        .map_err(|e| format!("nerinyan: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("nerinyan: HTTP {}", r.status()));
    }
    let v: Value = r.json().await.map_err(|e| format!("nerinyan: {e}"))?;
    Ok(normalize_list(&v))
}

async fn search_catboy(http: &reqwest::Client, p: &SearchParams) -> Result<Vec<Value>, String> {
    let mut url = reqwest::Url::parse("https://catboy.best/api/search").map_err(|e| e.to_string())?;
    {
        let mut pairs = url.query_pairs_mut();
        if !p.q.is_empty() {
            pairs.append_pair("q", &p.q);
        }
        if let Some(m) = p.mode {
            pairs.append_pair("mode", &m.to_string());
        }
        let statuses: &[&str] = match p.status.as_str() {
            "ranked" => &["1", "2"],
            "qualified" => &["3"],
            "loved" => &["4"],
            "pending" => &["0", "-1"],
            "graveyard" => &["-2"],
            "any" => &[],
            _ => &["1", "2", "4", "3"],
        };
        for s in statuses {
            pairs.append_pair("status", s);
        }
        if p.page > 0 {
            pairs.append_pair("offset", &(p.page * PAGE_SIZE).to_string());
        }
        pairs.append_pair("amount", &PAGE_SIZE.to_string());
    }
    let r = http.get(url).timeout(Duration::from_secs(15)).send_traced().await.map_err(|e| format!("catboy: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("catboy: HTTP {}", r.status()));
    }
    let v: Value = r.json().await.map_err(|e| format!("catboy: {e}"))?;
    Ok(normalize_list(&v))
}

pub async fn search(http: &reqwest::Client, p: &SearchParams) -> Result<Value, String> {
    let key = format!("s|{}|{:?}|{}|{}|{}|{}", p.q, p.mode, p.status, p.sort, p.page, p.nsfw);
    if let Some(v) = cached(&key, true) {
        return Ok(v);
    }
    let mut errs = Vec::new();
    for (name, res) in [("nerinyan", search_nerinyan(http, p).await), ("catboy", search_catboy(http, p).await)] {
        match res {
            Ok(sets) => {
                let out = json!({ "sets": sets, "page": p.page, "has_more": sets.len() >= PAGE_SIZE, "source": name });
                put(&key, &out);
                return Ok(out);
            }
            Err(e) => errs.push(e),
        }
    }
    cached(&key, false).ok_or_else(|| errs.join("; "))
}

async fn get_json(http: &reqwest::Client, url: &str) -> Result<Value, String> {
    let r = http.get(url).timeout(Duration::from_secs(15)).send_traced().await.map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status()));
    }
    r.json().await.map_err(|e| e.to_string())
}

pub async fn set_id_of_beatmap(http: &reqwest::Client, bid: i64) -> Option<i64> {
    for url in [format!("https://catboy.best/api/v2/b/{bid}"), format!("https://api.nerinyan.moe/search/b/{bid}")] {
        if let Ok(v) = get_json(http, &url).await {
            let v = if v.is_array() { v[0].clone() } else { v };
            if let Some(i) = v["beatmapset_id"].as_i64().or_else(|| v["beatmapset"]["id"].as_i64()).or_else(|| v["set_id"].as_i64()) {
                if i > 0 {
                    return Some(i);
                }
            }
        }
    }
    None
}

pub async fn get_set(http: &reqwest::Client, id: i64) -> Result<Value, String> {
    let key = format!("set|{id}");
    if let Some(v) = cached(&key, true) {
        return Ok(v);
    }
    let mut errs = Vec::new();
    let urls = [
        ("nerinyan", format!("https://api.nerinyan.moe/search/s/{id}")),
        ("catboy", format!("https://catboy.best/api/v2/s/{id}")),
        ("catboy", format!("https://catboy.best/api/s/{id}")),
    ];
    for (name, url) in urls {
        match get_json(http, &url).await {
            Ok(v) => {
                if let Some(set) = normalize_set(&v) {
                    let out = json!({ "set": set, "source": name });
                    put(&key, &out);
                    return Ok(out);
                }
                errs.push(format!("{name}: unrecognised response"));
            }
            Err(e) => errs.push(format!("{name}: {e}")),
        }
    }
    match crate::osu_api::get(http, &format!("/beatmapsets/{id}"), &[]).await {
        Ok(v) => {
            if let Some(set) = normalize_set(&v) {
                let out = json!({ "set": set, "source": "osu!" });
                put(&key, &out);
                return Ok(out);
            }
        }
        Err((_, e)) => errs.push(format!("osu!: {e}")),
    }
    cached(&key, false).ok_or_else(|| errs.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v2_style_set() {
        let v = json!({"id":12,"title":"T","artist":"A","creator":"C","status":"ranked","favourite_count":5,"play_count":9,
            "genre":{"id":1,"name":"Rock"},"language":{"id":2,"name":"English"},"ratings":[0,1,2],
            "beatmaps":[{"id":2,"version":"Hard","mode_int":0,"difficulty_rating":4.5,"cs":4,"ar":9,"accuracy":8,"drain":6,"bpm":180,"total_length":100,"checksum":"abc"},
                        {"id":1,"version":"Easy","mode_int":0,"difficulty_rating":1.5}]});
        let s = normalize_set(&v).unwrap();
        assert_eq!(s["id"], 12);
        assert_eq!(s["status"], "ranked");
        assert_eq!(s["genre"], "Rock");
        assert_eq!(s["beatmaps"][0]["version"], "Easy");
        assert_eq!(s["beatmaps"][1]["od"], 8.0);
        assert_eq!(s["bpm"], 180.0);
    }

    #[test]
    fn cheesegull_style_set() {
        let v = json!({"SetID":77,"Title":"T","Artist":"A","Creator":"C","RankedStatus":4,"Favourites":3,
            "ChildrenBeatmaps":[{"BeatmapID":9,"DiffName":"X","Mode":3,"DifficultyRating":5.0,"BPM":200.0,"OD":7.0}]});
        let s = normalize_set(&v).unwrap();
        assert_eq!(s["id"], 77);
        assert_eq!(s["status"], "loved");
        assert_eq!(s["beatmaps"][0]["mode"], 3);
        assert_eq!(s["bpm"], 200.0);
        assert_eq!(s["favourites"], 3);
    }

    #[test]
    fn status_from_ints_and_junk() {
        assert_eq!(status_name(&json!({"ranked": 1})), "ranked");
        assert_eq!(status_name(&json!({"ranked": -1})), "wip");
        assert_eq!(status_name(&json!({"status": "Qualified"})), "qualified");
        assert_eq!(status_name(&json!({})), "graveyard");
    }

    #[test]
    fn list_skips_invalid_and_accepts_wrapper() {
        let v = json!({"beatmapsets":[{"id":1,"title":"a"},{"title":"no id"}]});
        assert_eq!(normalize_list(&v).len(), 1);
        assert_eq!(normalize_list(&json!([{"id":3}])).len(), 1);
    }
}
