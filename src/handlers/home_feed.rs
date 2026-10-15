use crate::console::TraceSend;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const TTL: Duration = Duration::from_secs(600);
const MENU_URL: &str = "https://assets.ppy.sh/menu-content.json";

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
        c.insert(key.to_string(), (Instant::now(), v.clone()));
    }
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn mode_name(m: i64) -> &'static str {
    match m {
        1 => "taiko",
        2 => "fruits",
        3 => "mania",
        _ => "osu",
    }
}

fn mode_param(m: &str) -> &'static str {
    match m {
        "taiko" => "1",
        "fruits" | "catch" => "2",
        "mania" => "3",
        _ => "0",
    }
}

pub fn parse_news(v: &Value) -> Value {
    let posts = v.get("news_posts").and_then(|p| p.as_array()).cloned().unwrap_or_default();
    let out: Vec<Value> = posts
        .iter()
        .filter(|p| !s(p, "slug").is_empty())
        .take(12)
        .map(|p| {
            json!({
                "slug": s(p, "slug"),
                "title": s(p, "title"),
                "image": s(p, "first_image"),
                "date": s(p, "published_at"),
                "preview": s(p, "preview"),
            })
        })
        .collect();
    json!({ "posts": out })
}

pub fn parse_sets(v: &Value, limit: usize, with_modes: bool) -> Value {
    let sets = v.get("beatmapsets").and_then(|p| p.as_array()).cloned().unwrap_or_default();
    let out: Vec<Value> = sets
        .iter()
        .take(limit)
        .map(|b| {
            let mut modes: Vec<&str> = Vec::new();
            if with_modes {
                if let Some(bms) = b.get("beatmaps").and_then(|x| x.as_array()) {
                    for bm in bms {
                        let m = mode_name(bm.get("mode_int").and_then(|x| x.as_i64()).unwrap_or(0));
                        if !modes.contains(&m) {
                            modes.push(m);
                        }
                    }
                }
            }
            json!({
                "id": b.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
                "title": s(b, "title"),
                "artist": s(b, "artist"),
                "creator": s(b, "creator"),
                "cover": b.get("covers").map(|c| s(c, "cover")).unwrap_or_default(),
                "cover2x": b.get("covers").map(|c| s(c, "cover@2x")).unwrap_or_default(),
                "favourites": b.get("favourite_count").and_then(|x| x.as_i64()).unwrap_or(0),
                "plays": b.get("play_count").and_then(|x| x.as_i64()).unwrap_or(0),
                "ranked": s(b, "ranked_date"),
                "modes": modes,
            })
        })
        .collect();
    json!({ "sets": out })
}

pub fn parse_menu(v: &Value) -> Value {
    let arr = v.get("images").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    let out: Vec<Value> = arr
        .iter()
        .filter(|i| !s(i, "image").is_empty())
        .take(8)
        .map(|i| json!({ "image": s(i, "image"), "url": s(i, "url"), "name": s(i, "name") }))
        .collect();
    json!({ "images": out })
}

pub fn parse_daily(v: &Value) -> Value {
    let rooms = v.as_array().cloned().unwrap_or_default();
    for r in rooms {
        let item = r
            .get("current_playlist_item")
            .cloned()
            .or_else(|| r.get("playlist").and_then(|p| p.as_array()).and_then(|p| p.last()).cloned());
        let Some(item) = item else { continue };
        let Some(bm) = item.get("beatmap") else { continue };
        let set = bm.get("beatmapset").cloned().unwrap_or(Value::Null);
        if set.is_null() {
            continue;
        }
        let mut one = parse_sets(&json!({ "beatmapsets": [set] }), 1, false);
        if let Some(first) = one.get_mut("sets").and_then(|x| x.as_array_mut()).and_then(|x| x.first_mut()) {
            first["ends"] = json!(s(&r, "ends_at"));
            first["room"] = r.get("id").cloned().unwrap_or(Value::Null);
            return json!({ "set": first.clone() });
        }
    }
    json!({ "set": Value::Null })
}

async fn fetch(http: &reqwest::Client, part: &str, mode: &str) -> Result<Value, (u16, String)> {
    use crate::osu_api::get;
    match part {
        "news" => get(http, "/news", &[]).await.map(|v| parse_news(&v)),
        "new" => get(http, "/beatmapsets/search", &[("sort", "ranked_desc".into()), ("s", "ranked".into())])
            .await
            .map(|v| parse_sets(&v, 5, true)),
        "popular" => {
            let d = chrono::Utc::now() - chrono::Duration::days(30);
            let base = [
                ("sort", "favourites_desc".to_string()),
                ("s", "ranked".to_string()),
                ("m", mode_param(mode).to_string()),
            ];
            let mut q = base.to_vec();
            q.push(("q", format!("ranked>{}", d.format("%Y-%m-%d"))));
            let mut out = get(http, "/beatmapsets/search", &q).await.map(|v| parse_sets(&v, 5, false));
            if !matches!(&out, Ok(o) if o["sets"].as_array().map(|a| !a.is_empty()).unwrap_or(false)) {
                if let Ok(v) = get(http, "/beatmapsets/search", &base).await {
                    out = Ok(parse_sets(&v, 5, false));
                }
            }
            out
        }
        "daily" => get(
            http,
            "/rooms",
            &[("mode", "active".into()), ("category", "daily_challenge".into()), ("type_group", "playlists".into())],
        )
        .await
        .map(|v| parse_daily(&v)),
        "menu" => {
            let r = http
                .get(MENU_URL)
                .timeout(Duration::from_secs(10))
                .send_traced()
                .await
                .map_err(|e| (502u16, format!("menu content: {e}")))?;
            if !r.status().is_success() {
                return Err((502, format!("menu content status {}", r.status())));
            }
            let v: Value = r.json().await.map_err(|e| (502u16, format!("menu content: {e}")))?;
            Ok(parse_menu(&v))
        }
        _ => Err((404, "Unknown part".into())),
    }
}

pub async fn get_part(http: &reqwest::Client, part: &str, mode: &str) -> Result<Value, (u16, String)> {
    let mode = if part == "popular" { mode } else { "" };
    let key = format!("{part}:{mode}");
    if let Some(v) = cached(&key, true) {
        return Ok(v);
    }
    match fetch(http, part, mode).await {
        Ok(v) => {
            put(&key, &v);
            Ok(v)
        }
        Err(e) => cached(&key, false).ok_or(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn news_is_normalised() {
        let v = json!({"news_posts":[{"slug":"a","title":"T","first_image":"i","published_at":"2026-01-02T00:00:00Z","preview":"p"},{"title":"no slug"}]});
        let o = parse_news(&v);
        assert_eq!(o["posts"].as_array().unwrap().len(), 1);
        assert_eq!(o["posts"][0]["title"], "T");
    }

    #[test]
    fn sets_collect_modes() {
        let v = json!({"beatmapsets":[{"id":5,"title":"t","artist":"a","creator":"c","covers":{"cover":"x","cover@2x":"y"},
            "beatmaps":[{"mode_int":0},{"mode_int":3},{"mode_int":0}]}]});
        let o = parse_sets(&v, 5, true);
        assert_eq!(o["sets"][0]["modes"], json!(["osu", "mania"]));
        assert_eq!(o["sets"][0]["cover2x"], "y");
    }

    #[test]
    fn daily_room_is_read() {
        let v = json!([{"id":9,"ends_at":"2026-10-12T00:00:00Z","current_playlist_item":{"beatmap":{"beatmapset":{"id":1,"title":"t","artist":"a","creator":"c","covers":{"cover":"x"}}}}}]);
        let o = parse_daily(&v);
        assert_eq!(o["set"]["id"], 1);
        assert_eq!(o["set"]["room"], 9);
        assert!(parse_daily(&json!([]))["set"].is_null());
    }

    #[test]
    fn menu_skips_empty() {
        let o = parse_menu(&json!({"images":[{"image":"a","url":"u"},{"url":"z"}]}));
        assert_eq!(o["images"].as_array().unwrap().len(), 1);
    }
}
