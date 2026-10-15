
use crate::console::TraceSend;
use base64::Engine;
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const API: &str = "https://discord.com/api/v10";

const EMOJIS: &[(&str, &[u8])] = &[
    ("oe_xh", include_bytes!("../../assets/discord_emojis/xh.png")),
    ("oe_x", include_bytes!("../../assets/discord_emojis/x.png")),
    ("oe_sh", include_bytes!("../../assets/discord_emojis/sh.png")),
    ("oe_s", include_bytes!("../../assets/discord_emojis/s.png")),
    ("oe_a", include_bytes!("../../assets/discord_emojis/a.png")),
    ("oe_b", include_bytes!("../../assets/discord_emojis/b.png")),
    ("oe_c", include_bytes!("../../assets/discord_emojis/c.png")),
    ("oe_d", include_bytes!("../../assets/discord_emojis/d.png")),
    ("oe_f", include_bytes!("../../assets/discord_emojis/f.png")),
    ("oe_miss", include_bytes!("../../assets/discord_emojis/miss.png")),
    ("oe_bpm", include_bytes!("../../assets/discord_emojis/bpm.png")),
];

fn registry() -> &'static Mutex<HashMap<String, String>> {
    static REG: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lookup(name: &str) -> Option<String> {
    registry().lock().ok().and_then(|m| m.get(name).cloned())
}

pub fn grade_key(grade: &str) -> &'static str {
    match grade {
        "XH" | "SSH" => "oe_xh",
        "X" | "SS" => "oe_x",
        "SH" => "oe_sh",
        "S" => "oe_s",
        "A" => "oe_a",
        "B" => "oe_b",
        "C" => "oe_c",
        "D" => "oe_d",
        _ => "oe_f",
    }
}

pub fn grade(grade: &str) -> String {
    lookup(grade_key(grade)).unwrap_or_else(|| format!("**{}**", grade))
}

pub fn miss() -> String {
    lookup("oe_miss").unwrap_or_else(|| "❌".to_string())
}

pub fn bpm() -> String {
    lookup("oe_bpm").unwrap_or_else(|| "◬".to_string())
}

pub fn parse_listing(listing: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(items) = listing["items"].as_array().or_else(|| listing.as_array()) else {
        return out;
    };
    for e in items {
        if let (Some(id), Some(name)) = (e["id"].as_str(), e["name"].as_str()) {
            let animated = e["animated"].as_bool().unwrap_or(false);
            out.insert(name.to_string(), format!("<{}:{}:{}>", if animated { "a" } else { "" }, name, id));
        }
    }
    out
}

pub async fn sync(http: &HttpClient, token: &str, app_id: &str) {
    let auth = format!("Bot {}", token);
    let url = format!("{}/applications/{}/emojis", API, app_id);

    let existing = match http.get(&url).header("Authorization", &auth).timeout(Duration::from_secs(15)).send_traced().await {
        Ok(r) if r.status().is_success() => parse_listing(&r.json::<Value>().await.unwrap_or(Value::Null)),
        Ok(r) => {
            crate::logger::warn(&format!("Discord bot: could not list emojis ({}); using text fallbacks", r.status()));
            return;
        }
        Err(e) => {
            crate::logger::warn(&format!("Discord bot: could not list emojis: {}", e));
            return;
        }
    };

    let mut created = 0;
    for (name, bytes) in EMOJIS {
        if let Some(m) = existing.get(*name) {
            if let Ok(mut reg) = registry().lock() {
                reg.insert(name.to_string(), m.clone());
            }
            continue;
        }
        let body = json!({
            "name": name,
            "image": format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)),
        });
        match http.post(&url).header("Authorization", &auth).timeout(Duration::from_secs(20)).json(&body).send_traced().await {
            Ok(r) if r.status().is_success() => {
                let v: Value = r.json().await.unwrap_or(Value::Null);
                if let Some(id) = v["id"].as_str() {
                    if let Ok(mut reg) = registry().lock() {
                        reg.insert(name.to_string(), format!("<:{}:{}>", name, id));
                    }
                    created += 1;
                }
            }
            Ok(r) => {
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                crate::logger::warn(&format!("Discord bot: emoji {} upload failed ({}): {}", name, status, text));
            }
            Err(e) => crate::logger::warn(&format!("Discord bot: emoji {} upload failed: {}", name, e)),
        }
    }
    if created > 0 {
        crate::logger::info(&format!("Discord bot: uploaded {} emojis", created));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_emoji_is_a_png_within_discords_limits() {
        for (name, bytes) in EMOJIS {
            assert!(name.len() >= 2 && name.len() <= 32 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'), "{}", name);
            assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "{} is not a PNG", name);
            assert!(bytes.len() < 256 * 1024, "{} too large", name);
        }
    }

    #[test]
    fn grades_map_to_emoji_names() {
        assert_eq!(grade_key("XH"), "oe_xh");
        assert_eq!(grade_key("SS"), "oe_x");
        assert_eq!(grade_key("SH"), "oe_sh");
        assert_eq!(grade_key("B"), "oe_b");
        assert_eq!(grade_key("F"), "oe_f");
        assert!(EMOJIS.iter().any(|(n, _)| *n == grade_key("D")));
    }

    #[test]
    fn listing_parses_both_response_shapes() {
        let wrapped = json!({"items": [{"id": "1", "name": "oe_b"}, {"id": "2", "name": "oe_x", "animated": true}]});
        let m = parse_listing(&wrapped);
        assert_eq!(m["oe_b"], "<:oe_b:1>");
        assert_eq!(m["oe_x"], "<a:oe_x:2>");
        assert_eq!(parse_listing(&json!([{"id": "9", "name": "oe_s"}]))["oe_s"], "<:oe_s:9>");
        assert!(parse_listing(&json!({})).is_empty());
    }

    #[test]
    fn fallbacks_are_readable_text() {
        assert_eq!(grade("ZZZ_unregistered"), "**ZZZ_unregistered**");
        assert_eq!(miss(), "❌");
    }
}
