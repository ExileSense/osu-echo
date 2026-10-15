use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const TTL: Duration = Duration::from_secs(300);

type Cache = Mutex<HashMap<i32, (Instant, Value)>>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

pub fn normalize_user(u: &Value) -> Option<(i32, Value)> {
    let id = u.get("id").and_then(|x| x.as_i64())? as i32;
    let cover = u
        .get("cover")
        .and_then(|c| c.get("custom_url").and_then(|x| x.as_str()).filter(|x| !x.is_empty()).or_else(|| c.get("url").and_then(|x| x.as_str())))
        .unwrap_or("")
        .to_string();
    let team = u.get("team").filter(|t| t.is_object()).map(|t| {
        json!({"name": s(t, "name"), "short": s(t, "short_name"), "flag": s(t, "flag_url")})
    });
    let groups: Vec<Value> = u
        .get("groups")
        .and_then(|g| g.as_array())
        .map(|a| {
            a.iter()
                .filter(|g| g.get("has_listing").and_then(|x| x.as_bool()).unwrap_or(true))
                .map(|g| json!({"short": s(g, "short_name"), "colour": s(g, "colour"), "name": s(g, "name")}))
                .collect()
        })
        .unwrap_or_default();
    Some((
        id,
        json!({
            "username": s(u, "username"),
            "country": s(u, "country_code"),
            "cover": cover,
            "online": u.get("is_online").and_then(|x| x.as_bool()),
            "last_visit": s(u, "last_visit"),
            "support": u.get("support_level").and_then(|x| x.as_i64()).unwrap_or(if u.get("is_supporter").and_then(|x| x.as_bool()).unwrap_or(false) { 1 } else { 0 }),
            "team": team,
            "groups": groups,
        }),
    ))
}

pub async fn enrich(http: &reqwest::Client, ids: &[i32]) -> HashMap<i32, Value> {
    let mut out = HashMap::new();
    let mut missing = Vec::new();
    if let Ok(c) = cache().lock() {
        for id in ids {
            match c.get(id) {
                Some((at, v)) if at.elapsed() < TTL => {
                    out.insert(*id, v.clone());
                }
                _ => missing.push(*id),
            }
        }
    } else {
        missing = ids.to_vec();
    }
    for chunk in missing.chunks(50) {
        let q: Vec<(&str, String)> = chunk.iter().map(|i| ("ids[]", i.to_string())).collect();
        match crate::osu_api::get(http, "/users", &q).await {
            Ok(v) => {
                let list = v.get("users").and_then(|u| u.as_array()).cloned().unwrap_or_default();
                if let Ok(mut c) = cache().lock() {
                    for u in &list {
                        if let Some((id, n)) = normalize_user(u) {
                            c.insert(id, (Instant::now(), n.clone()));
                            out.insert(id, n);
                        }
                    }
                } else {
                    for u in &list {
                        if let Some((id, n)) = normalize_user(u) {
                            out.insert(id, n);
                        }
                    }
                }
            }
            Err((code, msg)) => {
                crate::logger::warn(&format!("friends page: osu! API lookup failed ({code}): {msg}"));
                if let Ok(c) = cache().lock() {
                    for id in chunk {
                        if let Some((_, v)) = c.get(id) {
                            out.insert(*id, v.clone());
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_api_user() {
        let u = json!({"id": 5, "username": "Bob", "country_code": "PL", "is_online": true, "last_visit": "2026-10-11T10:00:00+00:00",
            "cover": {"custom_url": null, "url": "https://x/c.jpg"}, "support_level": 3,
            "team": {"name": "T", "short_name": "TT", "flag_url": "https://x/f.png"},
            "groups": [{"short_name": "GMT", "colour": "#0f0", "name": "Global Moderators", "has_listing": true}, {"short_name": "X", "has_listing": false}]});
        let (id, n) = normalize_user(&u).unwrap();
        assert_eq!(id, 5);
        assert_eq!(n["cover"], "https://x/c.jpg");
        assert_eq!(n["online"], true);
        assert_eq!(n["support"], 3);
        assert_eq!(n["team"]["short"], "TT");
        assert_eq!(n["groups"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn missing_fields_are_tolerated() {
        let (_, n) = normalize_user(&json!({"id": 9})).unwrap();
        assert!(n["online"].is_null());
        assert_eq!(n["support"], 0);
        assert!(normalize_user(&json!({})).is_none());
    }
}
