use crate::console::TraceSend;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use base64::Engine;
use hyper::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as TokioMutex;

use crate::db;
use crate::server::assets::EmbeddedAssets;
use crate::server::response::Response;
use crate::state::{SharedState, WebSession};
use crate::types::mods::Mods;
use crate::types::profile::ProfileDetails;
use crate::utils;

pub const SESSION_COOKIE_NAME: &str = "los_session";
pub const SESSION_AGE_SECS: u64 = 86400;

#[derive(Deserialize)]
pub struct SettingsPayload {
    pub username: Option<String>,
    #[serde(default)]
    pub reset_avatar: bool,
    pub details: Option<ProfileDetails>,
    pub avatar: Option<String>,
    #[serde(default)]
    pub reset_cover: bool,
    pub cover: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SessionResponse {
    pub csrf: String,
    pub user: Option<String>,
    pub profiles: Vec<String>,
    pub active: Option<String>,
    pub default: Option<String>,
    #[serde(default)]
    pub country: String,
}

#[derive(Deserialize)]
pub struct LoginPayload {
    #[serde(default)]
    pub username: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SuccessResponse {
    pub ok: bool,
}

#[derive(Deserialize)]
pub struct DeleteScorePayload {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: i64,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct ImportOfficialPayload {
    pub query: String,
    #[serde(default)]
    pub apply: bool,
    #[serde(default = "default_true")]
    pub import_avatar: bool,
    #[serde(default = "default_true")]
    pub import_cover: bool,
    #[serde(default = "default_true")]
    pub import_bio: bool,
    #[serde(default = "default_true")]
    pub import_details: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ImportOfficialResponse {
    pub ok: bool,
    pub official_username: String,
    pub official_id: i64,
    pub details: ProfileDetails,
    pub avatar_data_url: Option<String>,
    pub avatar_updated: bool,
    #[serde(default)]
    pub cover_updated: bool,
    #[serde(default)]
    pub cover_data_url: Option<String>,
    pub details_updated: bool,
    pub message: String,
}

fn deserialize_id<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct IdVisitor;
    impl<'de> serde::de::Visitor<'de> for IdVisitor {
        type Value = i64;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or string containing an integer")
        }
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
            Ok(v)
        }
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
            Ok(v as i64)
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.parse::<i64>().map_err(serde::de::Error::custom)
        }
    }
    deserializer.deserialize_any(IdVisitor)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicBeatmap {
    pub title: String,
    pub artist: String,
    pub version: String,
    pub creator: String,
    pub beatmap_id: i64,
    pub beatmapset_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicScore {
    pub id: String,
    pub pp: f64,
    pub acc: f64,
    pub score: i64,
    pub max_combo: i32,
    pub time: i64,
    pub n300: i32,
    pub n100: i32,
    pub n50: i32,
    pub nmiss: i32,
    pub grade: String,
    pub mods: String,
    pub beatmap: PublicBeatmap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MostPlayedEntry {
    pub count: i32,
    pub beatmap: PublicBeatmap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResponse {
    pub name: String,
    pub mode: String,
    pub pp: i32,
    #[serde(default)]
    pub pp_exact: f64,
    pub acc: f64,
    pub playcount: i32,
    #[serde(default)]
    pub play_time_secs: i64,
    pub total_score: i64,
    pub ranked_score: i64,
    pub total_hits: i64,
    pub max_combo: i32,
    #[serde(default)]
    pub replays_watched: i64,
    #[serde(default)]
    pub official_stats: bool,
    pub grades: HashMap<String, i32>,
    pub active: bool,
    pub details: ProfileDetails,
    pub country_rank: Option<i32>,
    pub global_rank: Option<i32>,
    pub rank_history: Vec<(String, i32)>,
    pub top: Vec<PublicScore>,
    #[serde(default)]
    pub first_places: Vec<PublicScore>,
    pub recent: Vec<PublicScore>,
    pub last_play: Option<i64>,
    #[serde(default)]
    pub first_play: Option<i64>,
    pub performance_history: Vec<(String, f64)>,
    pub play_history: Vec<(String, i32)>,
    pub most_played: Vec<MostPlayedEntry>,
    pub recent_24h: Vec<PublicScore>,
    #[serde(default)]
    pub medals: Vec<crate::types::medal::UserMedalDisplay>,
    #[serde(default)]
    pub rank_highest: Option<crate::db::RankHighest>,
    #[serde(default)]
    pub has_cover: bool,
    #[serde(default)]
    pub rank_era: String,
    #[serde(default)]
    pub rank_era_date: Option<String>,
    #[serde(default)]
    pub pp_system: String,
    #[serde(default)]
    pub pp_system_missing: usize,
    #[serde(default)]
    pub pp_system_filling: bool,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn generate_token() -> String {
    let mut buf = [0u8; 32];
    let _ = getrandom::getrandom(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

pub fn generate_hex_token(nbytes: usize) -> String {
    let mut buf = vec![0u8; nbytes];
    let _ = getrandom::getrandom(&mut buf);
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn validate_image_format_and_dimensions(bytes: &[u8]) -> Result<&'static str, &'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        if bytes.len() < 24 {
            return Err("Invalid or truncated PNG image.");
        }
        if &bytes[12..16] != b"IHDR" {
            return Err("Invalid PNG IHDR header.");
        }
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err("Image dimensions must not exceed 4096x4096 pixels.");
        }
        return Ok("png");
    }

    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        let mut idx = 2;
        let mut found_dim = false;
        while idx + 4 < bytes.len() {
            if bytes[idx] != 0xFF {
                idx += 1;
                continue;
            }
            while idx < bytes.len() && bytes[idx] == 0xFF {
                idx += 1;
            }
            if idx >= bytes.len() {
                break;
            }
            let marker = bytes[idx];
            idx += 1;
            if marker == 0xD9 || marker == 0xDA {
                break;
            }
            if (0xD0..=0xD7).contains(&marker) || marker == 0x01 || marker == 0x00 {
                continue;
            }
            if idx + 2 > bytes.len() {
                break;
            }
            let seg_len = u16::from_be_bytes([bytes[idx], bytes[idx + 1]]) as usize;
            if seg_len < 2 || idx + seg_len > bytes.len() {
                break;
            }
            let is_sof = matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF);
            if is_sof && seg_len >= 7 {
                let height = u16::from_be_bytes([bytes[idx + 3], bytes[idx + 4]]) as u32;
                let width = u16::from_be_bytes([bytes[idx + 5], bytes[idx + 6]]) as u32;
                if width == 0 || height == 0 || width > 4096 || height > 4096 {
                    return Err("Image dimensions must not exceed 4096x4096 pixels.");
                }
                found_dim = true;
                break;
            }
            idx += seg_len;
        }
        if !found_dim && bytes.len() < 24 {
            return Err("Invalid or truncated JPEG image.");
        }
        return Ok("jpg");
    }

    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        if bytes.len() < 10 {
            return Err("Invalid or truncated GIF image.");
        }
        let width = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
        let height = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err("Image dimensions must not exceed 4096x4096 pixels.");
        }
        return Ok("gif");
    }

    if bytes.len() >= 16 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        if &bytes[12..16] == b"VP8 " && bytes.len() >= 30 {
            let width = (u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3FFF) as u32;
            let height = (u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3FFF) as u32;
            if width > 4096 || height > 4096 {
                return Err("Image dimensions must not exceed 4096x4096 pixels.");
            }
        } else if &bytes[12..16] == b"VP8L" && bytes.len() >= 25 && bytes[20] == 0x2F {
            let width = 1 + (((bytes[22] as u32 & 0x3F) << 8) | bytes[21] as u32);
            let height = 1 + (((bytes[24] as u32 & 0xF) << 10) | ((bytes[23] as u32) << 2) | ((bytes[22] as u32 & 0xC0) >> 6));
            if width > 4096 || height > 4096 {
                return Err("Image dimensions must not exceed 4096x4096 pixels.");
            }
        } else if &bytes[12..16] == b"VP8X" && bytes.len() >= 30 {
            let width = 1 + (bytes[24] as u32 | ((bytes[25] as u32) << 8) | ((bytes[26] as u32) << 16));
            let height = 1 + (bytes[27] as u32 | ((bytes[28] as u32) << 8) | ((bytes[29] as u32) << 16));
            if width > 4096 || height > 4096 {
                return Err("Image dimensions must not exceed 4096x4096 pixels.");
            }
        }
        return Ok("webp");
    }

    Err("Choose a PNG, JPEG, GIF or WebP up to 2 MB.")
}

pub fn parse_and_validate_avatar(data_str: &str) -> Result<(&'static str, Vec<u8>), &'static str> {
    parse_and_validate_image(data_str, 2 * 1024 * 1024)
}

pub fn parse_and_validate_image(data_str: &str, max_bytes: usize) -> Result<(&'static str, Vec<u8>), &'static str> {
    let raw_b64 = if let Some((_prefix, payload)) = data_str.split_once("base64,") {
        payload
    } else if data_str.starts_with("data:") {
        return Err("Invalid data URL format");
    } else {
        data_str
    };

    let cleaned: String = raw_b64.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if cleaned.is_empty() {
        return Err("Choose an image to upload.");
    }
    if cleaned.len() > max_bytes / 3 * 4 + 8 {
        return Err("Image is too large.");
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(cleaned.as_bytes())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(cleaned.as_bytes()))
        .map_err(|_| "Failed to decode base64 image data.")?;

    if bytes.is_empty() {
        return Err("Choose an image to upload.");
    }
    if bytes.len() > max_bytes {
        return Err("Image is too large.");
    }

    let ext = validate_image_format_and_dimensions(&bytes)?;
    Ok((ext, bytes))
}

pub fn extract_cookie<'a>(cookie_header: &'a str, cookie_name: &str) -> Option<&'a str> {
    for pair in cookie_header.split(';') {
        let trimmed = pair.trim();
        if let Some((k, v)) = trimmed.split_once('=') {
            if k.trim() == cookie_name {
                return Some(v.trim());
            }
        }
    }
    None
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn decode_html_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            while let Some(&next_c) = chars.peek() {
                if next_c == ';' || entity.len() >= 10 || next_c.is_whitespace() {
                    break;
                }
                entity.push(chars.next().unwrap());
            }
            let has_semicolon = if chars.peek() == Some(&';') {
                chars.next();
                true
            } else {
                false
            };
            match entity.as_str() {
                "quot" if has_semicolon => out.push('"'),
                "amp" if has_semicolon => out.push('&'),
                "lt" if has_semicolon => out.push('<'),
                "gt" if has_semicolon => out.push('>'),
                "apos" | "#039" | "#39" if has_semicolon => out.push('\''),
                _ => {
                    out.push('&');
                    out.push_str(&entity);
                    if has_semicolon {
                        out.push(';');
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[derive(Debug, Clone, Default)]
pub struct OfficialUserParsed {
    pub id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub cover_url: Option<String>,
    pub country_code: Option<String>,
    pub location: Option<String>,
    pub playstyle: Vec<String>,
    pub raw_bio: Option<String>,
    pub interests: Option<String>,
    pub occupation: Option<String>,
    pub twitter: Option<String>,
    pub website: Option<String>,
    pub discord: Option<String>,
    pub join_month: Option<String>,
    pub rank_history: Vec<i64>,
}

pub fn parse_official_userpage_html(html: &str) -> Result<OfficialUserParsed, String> {
    let marker_double = "data-initial-data=\"";
    let marker_single = "data-initial-data='";

    let (raw_data, _quote) = if let Some(pos) = html.find(marker_double) {
        let start = pos + marker_double.len();
        let end = html[start..]
            .find('"')
            .ok_or_else(|| "Malformed HTML: unterminated data-initial-data attribute".to_string())?;
        (&html[start..start + end], '"')
    } else if let Some(pos) = html.find(marker_single) {
        let start = pos + marker_single.len();
        let end = html[start..]
            .find('\'')
            .ok_or_else(|| "Malformed HTML: unterminated data-initial-data attribute".to_string())?;
        (&html[start..start + end], '\'')
    } else {
        return Err("Could not locate profile initial data in official osu! page".to_string());
    };

    let decoded = decode_html_entities(raw_data);
    let root: serde_json::Value = serde_json::from_str(&decoded)
        .map_err(|e| format!("Failed to parse profile JSON: {}", e))?;

    let user = if root.get("user").is_some() {
        &root["user"]
    } else {
        &root
    };
    parse_official_user_value(user)
}

pub fn parse_official_user_value(user: &serde_json::Value) -> Result<OfficialUserParsed, String> {
    let id = user.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let username = user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if username.is_empty() {
        return Err("Official user profile did not contain a valid username".to_string());
    }

    let avatar_url = user.get("avatar_url").and_then(|v| v.as_str()).map(|s| s.to_string());
    let cover_url = user
        .get("cover")
        .and_then(|c| c.get("custom_url").and_then(|v| v.as_str()).or_else(|| c.get("url").and_then(|v| v.as_str())))
        .or_else(|| user.get("cover_url").and_then(|v| v.as_str()))
        .filter(|u| !u.is_empty())
        .map(|s| s.to_string());
    let country_code = user.get("country_code").and_then(|v| v.as_str()).map(|s| s.to_uppercase());
    let location = user.get("location").and_then(|v| v.as_str()).map(|s| s.to_string());

    let mut playstyle = Vec::new();
    if let Some(arr) = user.get("playstyle").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                let normalized = match s.to_ascii_lowercase().as_str() {
                    "mouse" => "Mouse",
                    "keyboard" => "Keyboard",
                    "tablet" => "Tablet",
                    "touch" | "touchscreen" => "Touchscreen",
                    "controller" => "Controller",
                    _ => continue,
                };
                if !playstyle.contains(&normalized.to_string()) {
                    playstyle.push(normalized.to_string());
                }
            }
        }
    }

    let raw_bio = user
        .get("page")
        .and_then(|p| p.get("raw"))
        .and_then(|r| r.as_str())
        .map(|s| s.to_string());

    let text = |k: &str| user.get(k).and_then(|v| v.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let join_month = user
        .get("join_date")
        .and_then(|v| v.as_str())
        .filter(|d| d.len() >= 7 && d.as_bytes()[4] == b'-')
        .map(|d| d[..7].to_string());

    Ok(OfficialUserParsed {
        id,
        username,
        avatar_url,
        cover_url,
        country_code,
        location,
        playstyle,
        raw_bio,
        interests: text("interests"),
        occupation: text("occupation"),
        twitter: text("twitter"),
        website: text("website"),
        discord: text("discord"),
        join_month,
        rank_history: user
            .get("rank_history")
            .and_then(|h| h.get("data"))
            .and_then(|d| d.as_array())
            .map(|a| a.iter().map(|v| v.as_i64().unwrap_or(0)).collect())
            .unwrap_or_default(),
    })
}

pub fn official_rank_rows(today: chrono::NaiveDate, ranks: &[i64]) -> Vec<(String, i32)> {
    let n = ranks.len() as i64;
    ranks
        .iter()
        .enumerate()
        .filter(|(_, r)| **r > 0 && **r <= i32::MAX as i64)
        .map(|(i, r)| {
            let date = today - chrono::Duration::days(n - 1 - i as i64);
            (date.format("%Y-%m-%d").to_string(), *r as i32)
        })
        .collect()
}

pub fn can_write(
    session: Option<&WebSession>,
    headers: &hyper::HeaderMap,
) -> bool {
    let session = match session {
        Some(s) => s,
        None => return false,
    };

    let host = headers.get("host").and_then(|v| v.to_str().ok()).unwrap_or("");
    let host_domain = host.split(':').next().unwrap_or("").to_lowercase();

    let origin_header = headers.get("origin").and_then(|v| v.to_str().ok());
    let referer_header = headers.get("referer").and_then(|v| v.to_str().ok());

    let origin_target = origin_header.or(referer_header);
    let origin_matches = match origin_target {
        Some(target) => {
            let without_proto = target.split("://").nth(1).unwrap_or(target);
            let domain_part = without_proto.split('/').next().unwrap_or("");
            let domain_name = domain_part.split(':').next().unwrap_or("").to_lowercase();
            domain_name == host_domain
                || (domain_name == "localhost" && host_domain == "127.0.0.1")
                || (domain_name == "127.0.0.1" && host_domain == "localhost")
                || (domain_name.ends_with(".localhost") && host_domain.ends_with(".localhost"))
                || (domain_name.ends_with(".localhost") && (host_domain == "localhost" || host_domain == "127.0.0.1"))
                || ((domain_name == "localhost" || domain_name == "127.0.0.1") && host_domain.ends_with(".localhost"))
        }
        None => false,
    };

    if !origin_matches {
        return false;
    }

    let csrf_header = headers.get("x-csrf-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    if csrf_header.is_empty() {
        return false;
    }

    constant_time_eq(csrf_header.as_bytes(), session.csrf_token.as_bytes())
}

pub fn validate_username(name: &str) -> Result<String, &'static str> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err("Enter a username of 1–64 characters without slashes or control characters.");
    }
    if trimmed.chars().any(|c| (c as u32) < 32 || c == '/' || c == '\\') {
        return Err("Enter a username of 1–64 characters without slashes or control characters.");
    }
    Ok(trimmed.to_string())
}

fn json_ok<T: Serialize + ?Sized>(data: &T) -> Response {
    let val = serde_json::to_value(data).unwrap_or(serde_json::Value::Null);
    Response::json(&val).with_header("Cache-Control", "no-store")
}

fn json_error(status: StatusCode, message: &str) -> Response {
    let val = serde_json::json!({ "error": message });
    Response::json(&val).with_status(status).with_header("Cache-Control", "no-store")
}

const PP_RANK_ERAS_JSON: &str = include_str!("../../assets/pp_rank_eras.json");

struct PpRankEras {
    ranks: Vec<f64>,
    eras: HashMap<String, (String, Vec<f64>)>,
}

fn pp_rank_eras() -> &'static PpRankEras {
    static ERAS: std::sync::OnceLock<PpRankEras> = std::sync::OnceLock::new();
    ERAS.get_or_init(|| {
        let v: serde_json::Value = serde_json::from_str(PP_RANK_ERAS_JSON).unwrap_or(serde_json::Value::Null);
        let ranks: Vec<f64> = v
            .get("ranks")
            .and_then(|r| r.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_f64()).collect())
            .unwrap_or_default();
        let mut eras = HashMap::new();
        if let Some(obj) = v.get("eras").and_then(|e| e.as_object()) {
            for (key, era) in obj {
                let date = era.get("date").and_then(|d| d.as_str()).unwrap_or("").to_string();
                let pp: Vec<f64> = era
                    .get("pp")
                    .and_then(|p| p.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_f64()).collect())
                    .unwrap_or_default();
                if pp.len() == ranks.len() && !pp.is_empty() {
                    eras.insert(key.clone(), (date, pp));
                }
            }
        }
        PpRankEras { ranks, eras }
    })
}

pub fn era_rank(era: &str, pp: f64) -> Option<i32> {
    if pp > 0.0 {
        if let (Some(l), Some(d)) = (crate::ladder::get(), crate::ladder::era_date(era)) {
            if let Some(r) = l.rank_at_pp_on(&d, pp) {
                return Some(r);
            }
        }
    }
    era_rank_json(era, pp)
}

fn era_rank_json(era: &str, pp: f64) -> Option<i32> {
    let data = pp_rank_eras();
    let (_, table) = data.eras.get(era)?;
    if pp <= 0.0 {
        return None;
    }
    let n = table.len();
    if pp >= table[0] {
        return Some(1);
    }
    if pp < table[n - 1] {
        return Some(data.ranks[n - 1] as i32);
    }
    let i = table.partition_point(|&t| t > pp);
    if i == 0 {
        return Some(1);
    }
    let (hi_pp, lo_pp) = (table[i - 1], table[i]);
    let (hi_rank, lo_rank) = (data.ranks[i - 1], data.ranks[i]);
    if (hi_pp - lo_pp).abs() < f64::EPSILON {
        return Some(lo_rank as i32);
    }
    let t = (hi_pp - pp) / (hi_pp - lo_pp);
    let rank = (hi_rank.ln() + t * (lo_rank.ln() - hi_rank.ln())).exp();
    Some(rank.round().max(1.0) as i32)
}

pub fn era_date(era: &str) -> Option<String> {
    crate::ladder::era_date(era).or_else(|| pp_rank_eras().eras.get(era).map(|(d, _)| d.clone()))
}

static DAILY_CACHE: TokioMutex<Option<HashMap<i64, (Instant, Option<i32>)>>> =
    TokioMutex::const_new(None);
static DAILY_LAST_REQUEST: TokioMutex<Option<Instant>> = TokioMutex::const_new(None);

pub async fn get_daily_rank(
    http: &reqwest::Client,
    api_key_opt: Option<&str>,
    pp: f64,
) -> Option<i32> {
    let api_key = api_key_opt?;
    if api_key.trim().is_empty() || pp <= 0.0 {
        return None;
    }

    let pp_key = (pp * 100.0).round() as i64;
    let mut cache_guard = DAILY_CACHE.lock().await;
    let cache = cache_guard.get_or_insert_with(HashMap::new);

    let now = Instant::now();
    if let Some((cached_time, cached_rank)) = cache.get(&pp_key) {
        if now.duration_since(*cached_time).as_secs() < 300 {
            return *cached_rank;
        }
    }

    let mut last_req_guard = DAILY_LAST_REQUEST.lock().await;
    if let Some(last_time) = *last_req_guard {
        let elapsed = now.duration_since(last_time);
        if elapsed.as_millis() < 1100 {
            tokio::time::sleep(std::time::Duration::from_millis(1100 - elapsed.as_millis() as u64)).await;
        }
    }
    *last_req_guard = Some(Instant::now());
    drop(last_req_guard);

    let mut rank: Option<i32> = None;
    for value in [format!("{:.2}", pp), format!("{}", pp.round() as i64)] {
        let url = format!("https://osudaily.net/api/pp.php?k={}&t=pp&v={}&m=0", api_key, value);
        if let Ok(resp) = http.get(&url).timeout(std::time::Duration::from_secs(8)).send_traced().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(r) = json.get("rank").and_then(|v| v.as_i64()) {
                        if r > 0 {
                            rank = Some(r as i32);
                        }
                    }
                }
            }
        }
        if rank.is_some() || value == format!("{}", pp.round() as i64) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }

    cache.insert(pp_key, (Instant::now(), rank));
    rank
}

static COUNTRY_RANK_CACHE: TokioMutex<Option<HashMap<(String, i64), (Instant, Option<i32>)>>> =
    TokioMutex::const_new(None);
static OAUTH_TOKEN: TokioMutex<Option<(String, Instant)>> = TokioMutex::const_new(None);

async fn fetch_oauth_token(
    http: &reqwest::Client,
    client_id: u64,
    client_secret: &str,
    token_guard: &mut tokio::sync::MutexGuard<'_, Option<(String, Instant)>>,
) -> Option<String> {
    let payload = serde_json::json!({
        "client_id": client_id,
        "client_secret": client_secret,
        "grant_type": "client_credentials",
        "scope": "public"
    });

    let resp = http
        .post("https://osu.ppy.sh/oauth/token")
        .header("Accept", "application/json")
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send_traced()
        .await
        .ok()?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        crate::logger::warn(&format!(
            "osu! OAuth token request failed (HTTP {}): {}",
            status, body
        ));
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    let access_token = json.get("access_token")?.as_str()?.to_string();
    let expires_in = json.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(86400);
    let valid_duration = std::time::Duration::from_secs(expires_in.saturating_sub(60).max(60));

    **token_guard = Some((access_token.clone(), Instant::now() + valid_duration));
    Some(access_token)
}

async fn fetch_country_placement(
    http: &reqwest::Client,
    token: &str,
    country: &str,
    pp: f64,
) -> Option<i32> {
    let fetch_page = |page: i64| {
        let http = http.clone();
        let auth = format!("Bearer {}", token);
        let country = country.to_string();
        async move {
            let resp = http
                .get("https://osu.ppy.sh/api/v2/rankings/osu/performance")
                .query(&[("country", &country), ("cursor[page]", &page.to_string())])
                .header("Authorization", auth)
                .timeout(std::time::Duration::from_secs(10))
                .send_traced()
                .await
                .ok()?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                crate::logger::warn(&format!(
                    "osu! API rankings request failed (HTTP {}): {}",
                    status, text
                ));
                return None;
            }
            resp.json::<serde_json::Value>().await.ok()
        }
    };

    let first = fetch_page(1).await?;
    let ranking = first.get("ranking")?.as_array()?;
    if ranking.is_empty() {
        return None;
    }
    let size = ranking.len() as i64;
    if size == 0 {
        return None;
    }
    let total = first.get("total")?.as_i64()?;
    let pp_f = pp;

    let mut low = 1;
    let mut high = (total + size - 1) / size;
    let mut candidate: Option<(i64, Vec<serde_json::Value>)> = None;

    for _ in 0..16 {
        if low > high {
            break;
        }
        let mid = (low + high) / 2;
        let data = if mid == 1 {
            Some(first.clone())
        } else {
            fetch_page(mid).await
        };
        let entries = match data.as_ref().and_then(|d| d.get("ranking")).and_then(|r| r.as_array()) {
            Some(e) if !e.is_empty() => e,
            _ => break,
        };

        let last_pp = entries.last().and_then(|e| e.get("pp")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        if last_pp <= pp_f {
            candidate = Some((mid, entries.clone()));
            high = mid - 1;
        } else {
            low = mid + 1;
        }
    }

    if let Some((candidate_page, entries)) = candidate {
        for (idx, entry) in entries.iter().enumerate() {
            let entry_pp = entry.get("pp").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if entry_pp <= pp_f {
                let calculated_rank = ((candidate_page - 1) * size + idx as i64 + 1) as i32;
                let rank = entry
                    .get("country_rank")
                    .and_then(|v| v.as_i64())
                    .map(|r| r as i32)
                    .unwrap_or(calculated_rank);
                return Some(rank);
            }
        }
    }

    if total > 0 && total < 10000 {
        return Some((total + 1) as i32);
    }

    None
}

pub async fn get_country_rank(
    http: &reqwest::Client,
    country: &str,
    pp: f64,
    creds: Option<(u64, &str)>,
) -> Option<i32> {
    let country_trimmed = country.trim();
    if country_trimmed.is_empty() || pp <= 0.0 {
        return None;
    }

    let (client_id, client_secret) = if let Some((id, sec)) = creds {
        (id, sec.trim().trim_matches('"').trim_matches('\'').to_string())
    } else {
        let client_id_str = std::env::var("OSU_CLIENT_ID").ok()?;
        let client_id: u64 = client_id_str
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .parse()
            .ok()?;
        let client_secret = std::env::var("OSU_CLIENT_SECRET").ok()?;
        let client_secret = client_secret.trim().trim_matches('"').trim_matches('\'').to_string();
        (client_id, client_secret)
    };
    if client_secret.trim().is_empty() {
        return None;
    }

    let country_upper = country_trimmed.to_uppercase();
    let cache_key = (country_upper.clone(), (pp * 100.0).round() as i64);

    {
        let mut guard = COUNTRY_RANK_CACHE.lock().await;
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some((cached_time, rank)) = cache.get(&cache_key) {
            let ttl = if rank.is_some() { 900 } else { 10 };
            if cached_time.elapsed().as_secs() < ttl {
                return *rank;
            }
        }
    }

    let token = {
        let mut token_guard = OAUTH_TOKEN.lock().await;
        let now = Instant::now();
        if let Some((tok, expires_at)) = &*token_guard {
            if *expires_at > now {
                tok.clone()
            } else {
                fetch_oauth_token(http, client_id, &client_secret, &mut token_guard).await?
            }
        } else {
            fetch_oauth_token(http, client_id, &client_secret, &mut token_guard).await?
        }
    };

    let rank = fetch_country_placement(http, &token, &country_upper, pp).await;

    {
        let mut guard = COUNTRY_RANK_CACHE.lock().await;
        let cache = guard.get_or_insert_with(HashMap::new);
        cache.insert(cache_key, (Instant::now(), rank));
    }

    rank
}

fn score_grade(s: &db::ScoreWithBeatmap) -> &'static str {
    let g = crate::utils::get_grade(
        s.mode as u8,
        s.n300,
        s.n100,
        s.n50,
        s.ngeki,
        s.nkatu,
        s.nmiss,
        s.mods,
        s.acc,
    );
    match g {
        "SSH" => "XH",
        "SS" => "X",
        other => other,
    }
}

fn to_public_score(s: &db::ScoreWithBeatmap) -> PublicScore {
    let mods_str = if let Some(ref ms) = s.mods_str {
        if !ms.is_empty() {
            ms.clone()
        } else {
            Mods::from_bits_truncate(s.mods).short_name()
        }
    } else {
        Mods::from_bits_truncate(s.mods).short_name()
    };

    PublicScore {
        id: s.id.to_string(),
        pp: (s.pp * 100.0).round() / 100.0,
        acc: (s.acc * 100.0).round() / 100.0,
        score: s.score,
        max_combo: s.max_combo,
        time: s.time,
        n300: s.n300,
        n100: s.n100,
        n50: s.n50,
        nmiss: s.nmiss,
        grade: score_grade(s).to_string(),
        mods: mods_str,
        beatmap: PublicBeatmap {
            title: s.title.clone().unwrap_or_else(|| "Unknown beatmap".to_string()),
            artist: s.artist.clone().unwrap_or_else(|| "unknown artist".to_string()),
            version: s.version.clone().unwrap_or_else(|| "Unknown difficulty".to_string()),
            creator: s.creator.clone().unwrap_or_default(),
            beatmap_id: s.beatmap_id.unwrap_or(0),
            beatmapset_id: s.beatmapset_id.unwrap_or(0),
        },
    }
}

pub async fn handle(
    state: SharedState,
    action: &str,
    params: &HashMap<String, String>,
    method: &Method,
    headers: &hyper::HeaderMap,
    body: &[u8],
) -> Response {
    match (method, action) {
        (&Method::GET, "session") => handle_session(state, headers).await,
        (&Method::GET, "profile") => handle_profile(state, params).await,
        (&Method::GET, "avatar") => handle_avatar(state, params).await,
        (&Method::GET, "cover") => handle_cover(state, params).await,
        (&Method::GET, "medals") => handle_medals_catalog().await,
        (&Method::GET, "ladder_info") => handle_ladder_info(),
        (&Method::GET, "home_feed") => handle_home_feed(state, params).await,
        (&Method::GET, "beatmaps") => handle_beatmaps(state, params).await,
        (&Method::GET, "friends") => handle_friends(state, headers).await,
        (&Method::GET, "beatmapset") => handle_beatmapset(state, params).await,
        (&Method::GET, "beatmap_scores") => handle_beatmap_scores(state, params, headers).await,
        (&Method::GET, "score") => handle_score(state, params).await,
        (&Method::POST, "ladder_era") => handle_ladder_era(state, headers, body).await,
        (&Method::POST, "ladder_refresh") => handle_ladder_refresh(state, headers).await,
        (&Method::POST, "login") => handle_login(state, headers, body).await,
        (&Method::POST, "logout") => handle_logout(state, headers).await,
        (&Method::POST, "settings") => handle_settings(state, headers, body).await,
        (&Method::POST, "friends/remove") => handle_friend_remove(state, headers, body).await,
        (&Method::POST, "scores/delete") => handle_delete_score(state, headers, body).await,
        (&Method::POST, "import_official") => handle_import_official(state, headers, body).await,
        (&Method::POST, "import_scores") => handle_import_scores(state, headers, body).await,
        (&Method::POST, "import_ranks") => handle_import_ranks(state, headers, body).await,
        _ => json_error(StatusCode::NOT_FOUND, "Not found"),
    }
}

async fn session_user(state: &SharedState, headers: &hyper::HeaderMap) -> Option<(String, WebSession)> {
    let cookie_val = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME))?
        .to_string();
    let now = now_secs();
    let guard = state.read().await;
    let session = guard.web_sessions.get(&cookie_val).filter(|s| s.expires > now)?.clone();
    let user = session.username.clone().filter(|u| !u.is_empty())?;
    Some((user, session))
}

fn is_bot_friend(id: i32) -> bool {
    id <= 2 || id == crate::handlers::banchobot::BOT_ID || crate::handlers::tillerino::is_tillerino_id(id)
}

async fn handle_friends(state: SharedState, headers: &hyper::HeaderMap) -> Response {
    let Some((user, _)) = session_user(&state, headers).await else {
        return json_error(StatusCode::UNAUTHORIZED, "Sign in to see your friends.");
    };
    let (http, records, locals) = {
        let guard = state.read().await;
        let conn = guard.db.lock().await;
        let recs = db::get_friend_records(&conn, &user).unwrap_or_default();
        let locals: std::collections::HashSet<String> = recs
            .iter()
            .filter(|r| !r.friend_name.is_empty() && db::profile_exists(&conn, &r.friend_name).unwrap_or(false))
            .map(|r| r.friend_name.to_ascii_lowercase())
            .collect();
        (guard.http.clone(), recs, locals)
    };
    let records: Vec<_> = records.into_iter().filter(|r| !is_bot_friend(r.friend_id)).collect();
    let ids: Vec<i32> = records.iter().map(|r| r.friend_id).collect();
    let extra = crate::handlers::friends::enrich(&http, &ids).await;
    let enriched = !extra.is_empty();
    let friends: Vec<serde_json::Value> = records
        .iter()
        .map(|r| {
            let e = extra.get(&r.friend_id);
            let name = e
                .and_then(|e| e["username"].as_str())
                .filter(|n| !n.is_empty())
                .unwrap_or(&r.friend_name)
                .to_string();
            let country = e
                .and_then(|e| e["country"].as_str())
                .filter(|c| !c.is_empty())
                .map(|c| c.to_string())
                .unwrap_or_else(|| {
                    let c = crate::utils::country_byte_to_code(r.country);
                    if c == "Unknown" { String::new() } else { c.to_string() }
                });
            serde_json::json!({
                "id": r.friend_id, "name": name, "country": country, "rank": r.rank, "pp": r.pp,
                "local": locals.contains(&name.to_ascii_lowercase()),
                "online": e.and_then(|e| e["online"].as_bool()),
                "last_visit": e.and_then(|e| e["last_visit"].as_str()).unwrap_or(""),
                "cover": e.and_then(|e| e["cover"].as_str()).unwrap_or(""),
                "support": e.and_then(|e| e["support"].as_i64()).unwrap_or(0),
                "team": e.map(|e| e["team"].clone()).unwrap_or(serde_json::Value::Null),
                "groups": e.map(|e| e["groups"].clone()).unwrap_or_else(|| serde_json::json!([])),
            })
        })
        .collect();
    json_ok(&serde_json::json!({ "user": user, "friends": friends, "enriched": enriched }))
}

#[derive(serde::Deserialize)]
struct FriendRemovePayload {
    id: i32,
}

async fn handle_friend_remove(state: SharedState, headers: &hyper::HeaderMap, body: &[u8]) -> Response {
    let Some((user, session)) = session_user(&state, headers).await else {
        return json_error(StatusCode::UNAUTHORIZED, "Sign in to manage your friends.");
    };
    if !can_write(Some(&session), headers) {
        return json_error(StatusCode::FORBIDDEN, "Refresh the page and try again.");
    }
    let payload: FriendRemovePayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, &format!("Invalid request body: {}", e)),
    };
    if is_bot_friend(payload.id) {
        return json_error(StatusCode::BAD_REQUEST, "Invalid friend.");
    }
    let mut ids = {
        let guard = state.read().await;
        let conn = guard.db.lock().await;
        if let Err(e) = db::remove_friend(&conn, &user, payload.id) {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("Database error: {}", e));
        }
        db::get_friend_ids(&conn, &user).unwrap_or_default()
    };
    ids.retain(|&id| id > 2);
    for bot in [crate::handlers::banchobot::BOT_ID, crate::handlers::tillerino::BOT_ID] {
        if !ids.contains(&bot) {
            ids.push(bot);
        }
    }
    {
        let mut guard = state.write().await;
        if let Some(ref mut p) = guard.player {
            if p.name.eq_ignore_ascii_case(&user) {
                p.queue.extend_from_slice(&crate::packets::friends_list(&ids));
                p.queue.extend_from_slice(&crate::packets::logout(payload.id));
            }
        }
    }
    json_ok(&serde_json::json!({ "ok": true }))
}

async fn handle_beatmaps(state: SharedState, params: &HashMap<String, String>) -> Response {
    use crate::handlers::beatmap_mirror as bm;
    let sp = bm::SearchParams {
        q: params.get("q").map(|s| s.trim().chars().take(200).collect()).unwrap_or_default(),
        mode: params.get("m").and_then(|m| m.parse::<i32>().ok()).filter(|m| (0..=3).contains(m)),
        status: params.get("s").cloned().unwrap_or_else(|| "leaderboard".into()),
        sort: params.get("sort").cloned().unwrap_or_else(|| "ranked_desc".into()),
        page: params.get("page").and_then(|p| p.parse().ok()).unwrap_or(0).min(200),
        nsfw: params.get("nsfw").map(|v| v == "1" || v == "true").unwrap_or(false),
    };
    let http = state.read().await.http.clone();
    match bm::search(&http, &sp).await {
        Ok(v) => json_ok(&v),
        Err(e) => json_error(StatusCode::BAD_GATEWAY, &format!("No beatmap mirror answered ({e})")),
    }
}

async fn handle_beatmapset(state: SharedState, params: &HashMap<String, String>) -> Response {
    let mut id: i64 = params.get("id").and_then(|i| i.parse().ok()).unwrap_or(0);
    let http = state.read().await.http.clone();
    if id <= 0 {
        let bid: i64 = params.get("bid").and_then(|i| i.parse().ok()).unwrap_or(0);
        if bid > 0 {
            {
                let db_handle = state.read().await.db.clone();
                let conn = db_handle.lock().await;
                id = conn
                    .query_row("SELECT beatmapset_id FROM beatmaps WHERE beatmap_id = ?1 AND beatmapset_id > 0 LIMIT 1", [bid], |r| r.get::<_, i64>(0))
                    .unwrap_or(0);
            }
            if id <= 0 {
                id = crate::handlers::beatmap_mirror::set_id_of_beatmap(&http, bid).await.unwrap_or(0);
            }
        }
    }
    if id <= 0 {
        return json_error(StatusCode::BAD_REQUEST, "Missing beatmapset id.");
    }
    match crate::handlers::beatmap_mirror::get_set(&http, id).await {
        Ok(v) => json_ok(&v),
        Err(e) => json_error(StatusCode::BAD_GATEWAY, &format!("No beatmap mirror answered ({e})")),
    }
}

fn mod_tokens(s: &str) -> Vec<String> {
    let cleaned: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_uppercase();
    let mut v: Vec<String> = if s.contains(',') {
        s.split(',').map(|t| t.trim().trim_start_matches('+').to_ascii_uppercase()).filter(|t| !t.is_empty()).collect()
    } else {
        cleaned.as_bytes().chunks(2).map(|c| String::from_utf8_lossy(c).to_string()).collect()
    };
    v.retain(|t| t != "NM" && t != "NONE");
    if v.iter().any(|t| t == "NC") {
        v.retain(|t| t != "DT");
    }
    if v.iter().any(|t| t == "PF") {
        v.retain(|t| t != "SD");
    }
    v.sort();
    v.dedup();
    v
}

async fn handle_beatmap_scores(state: SharedState, params: &HashMap<String, String>, headers: &hyper::HeaderMap) -> Response {
    let md5 = params.get("md5").cloned().unwrap_or_default();
    let id: i64 = params.get("id").and_then(|i| i.parse().ok()).unwrap_or(0);
    let mode: i32 = params.get("m").and_then(|m| m.parse().ok()).unwrap_or(0);
    let kind = params.get("type").map(|s| s.as_str()).unwrap_or("global");
    let mods_filter: Option<Vec<String>> = params.get("mods").filter(|m| !m.is_empty()).map(|m| mod_tokens(m));
    if md5.is_empty() && id <= 0 {
        return json_error(StatusCode::BAD_REQUEST, "Missing beatmap.");
    }
    let viewer = session_user(&state, headers).await.map(|(u, _)| u);
    let db_handle = state.read().await.db.clone();
    let conn = db_handle.lock().await;
    let mut rows = db::get_map_leaderboard(&conn, &md5, id, mode, 10_000).unwrap_or_default();
    if let Some(want) = &mods_filter {
        rows.retain(|r| &mod_tokens(r.mods_str.as_deref().unwrap_or("")) == want);
    }
    let mut requires_login = false;
    match kind {
        "country" | "friend" if viewer.is_none() => {
            requires_login = true;
            rows.clear();
        }
        "country" => {
            let mine = viewer.as_deref().and_then(|v| db::get_profile_details(&conn, v).ok()).map(|d| d.country).unwrap_or_default();
            rows.retain(|r| !mine.is_empty() && db::get_profile_details(&conn, &r.player).map(|d| d.country).unwrap_or_default().eq_ignore_ascii_case(&mine));
        }
        "friend" => {
            let me = viewer.clone().unwrap_or_default();
            let mut names: std::collections::HashSet<String> =
                db::get_friends(&conn, &me).unwrap_or_default().into_iter().map(|(_, n)| n.to_ascii_lowercase()).filter(|n| !n.is_empty()).collect();
            names.insert(me.to_ascii_lowercase());
            rows.retain(|r| names.contains(&r.player.to_ascii_lowercase()));
        }
        _ => {}
    }
    let friend_names: std::collections::HashSet<String> = match &viewer {
        Some(v) => db::get_friends(&conn, v).unwrap_or_default().into_iter().map(|(_, n)| n.to_ascii_lowercase()).collect(),
        None => Default::default(),
    };
    let to_json = |conn: &rusqlite::Connection, r: &db::MapLeaderboardRow, pos: usize| {
        let grade = crate::utils::get_grade(r.mode as u8, r.n300, r.n100, r.n50, r.ngeki, r.nkatu, r.nmiss, r.mods, r.acc);
        let grade = match grade {
            "SSH" => "XH",
            "SS" => "X",
            other => other,
        };
        let country = db::get_profile_details(conn, &r.player).map(|d| d.country).unwrap_or_default();
        serde_json::json!({
            "pos": pos, "player": r.player, "country": country, "grade": grade, "score": r.score, "acc": r.acc,
            "combo": r.max_combo, "n300": r.n300, "n100": r.n100, "n50": r.n50, "miss": r.nmiss,
            "geki": r.ngeki, "katu": r.nkatu, "pp": r.pp, "mods": r.mods_str.clone().unwrap_or_default(),
            "time": r.time, "id": r.id,
            "friend": friend_names.contains(&r.player.to_ascii_lowercase()),
            "self": viewer.as_deref().map(|v| v.eq_ignore_ascii_case(&r.player)).unwrap_or(false),
        })
    };
    let out: Vec<serde_json::Value> = rows.iter().take(50).enumerate().map(|(i, r)| to_json(&conn, r, i + 1)).collect();
    let me = viewer
        .as_deref()
        .and_then(|v| rows.iter().position(|r| r.player.eq_ignore_ascii_case(v)))
        .map(|i| to_json(&conn, &rows[i], i + 1));
    json_ok(&serde_json::json!({ "scores": out, "me": me, "total": rows.len(), "type": kind, "requires_login": requires_login,
        "viewer_country": viewer.as_deref().and_then(|v| db::get_profile_details(&conn, v).ok()).map(|d| d.country).unwrap_or_default() }))
}

async fn handle_score(state: SharedState, params: &HashMap<String, String>) -> Response {
    let id: i64 = params.get("id").and_then(|i| i.trim().parse().ok()).unwrap_or(0);
    if id <= 0 {
        return json_error(StatusCode::BAD_REQUEST, "Missing score id.");
    }
    let db_handle = state.read().await.db.clone();
    let conn = db_handle.lock().await;
    let Some((player, s)) = db::get_score_with_player(&conn, id).ok().flatten() else {
        return json_error(StatusCode::NOT_FOUND, "This score does not exist on this server.");
    };
    let p = to_public_score(&s);
    let country = db::get_profile_details(&conn, &player).map(|d| d.country).unwrap_or_default();
    let rank = db::get_map_leaderboard(&conn, &s.md5, s.beatmap_id.unwrap_or(0), s.mode, 10_000)
        .ok()
        .and_then(|rows| rows.iter().position(|r| r.id == s.id))
        .map(|i| i + 1);
    json_ok(&serde_json::json!({
        "id": s.id, "mode": s.mode, "player": player, "country": country,
        "grade": p.grade, "score": s.score, "acc": p.acc, "combo": s.max_combo, "perfect": s.perfect,
        "n300": s.n300, "n100": s.n100, "n50": s.n50, "miss": s.nmiss, "geki": s.ngeki, "katu": s.nkatu,
        "pp": p.pp, "mods": p.mods, "time": s.time, "rank": rank, "has_replay": s.replay_md5.is_some(),
        "beatmap": p.beatmap, "md5": s.md5,
    }))
}

async fn handle_home_feed(state: SharedState, params: &HashMap<String, String>) -> Response {
    let part = params.get("part").map(|s| s.as_str()).unwrap_or("");
    let mode = params.get("mode").map(|s| s.as_str()).unwrap_or("osu");
    let http = {
        let s = state.read().await;
        s.http.clone()
    };
    match crate::handlers::home_feed::get_part(&http, part, mode).await {
        Ok(v) => json_ok(&v),
        Err((code, msg)) => json_error(if code == 404 { StatusCode::NOT_FOUND } else { StatusCode::BAD_GATEWAY }, &msg),
    }
}

fn handle_ladder_info() -> Response {
    match crate::ladder::info() {
        Some(i) => json_ok(&serde_json::json!({
            "available": true,
            "first_date": i.first_date,
            "last_date": i.last_date,
            "source": i.source,
            "path": i.path,
            "auto_refresh": i.auto_refresh,
            "era": crate::ladder::active_era().unwrap_or_else(|| "latest".to_string()),
            "eras": crate::ladder::era_list().into_iter().map(|(id, label, date)| serde_json::json!({"id": id, "label": label, "date": date})).collect::<Vec<_>>(),
        })),
        None => json_ok(&serde_json::json!({ "available": false })),
    }
    .with_header("Cache-Control", "no-cache")
}

async fn handle_ladder_era(state: SharedState, headers: &hyper::HeaderMap, body: &[u8]) -> Response {
    let now = now_secs();
    let token = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME))
        .map(|t| t.to_string());
    let session = {
        let st = state.read().await;
        token.and_then(|t| st.web_sessions.get(&t).filter(|s| s.expires > now).cloned())
    };
    let session = match session {
        Some(s) => s,
        None => return json_error(StatusCode::UNAUTHORIZED, "Sign in to change the rank ladder."),
    };
    if !can_write(Some(&session), headers) {
        return json_error(StatusCode::FORBIDDEN, "Refresh the page and try again.");
    }
    let v: serde_json::Value = serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
    let era = v.get("era").and_then(|e| e.as_str()).unwrap_or("latest");
    if era != "latest" && !crate::ladder::valid_era(era) {
        return json_error(StatusCode::BAD_REQUEST, "Unknown rank ladder.");
    }
    crate::ladder::set_active_era(if era == "latest" { None } else { Some(era) });
    let (pp, mode, key, http) = {
        let s = state.read().await;
        match s.player.as_ref() {
            Some(p) => (p.pp, p.mode, s.config.osu_daily_api_key.clone(), s.http.clone()),
            None => return json_ok(&serde_json::json!({ "ok": true, "era": era })),
        }
    };
    let rank = if era != "latest" {
        crate::utils::get_rank_from_daily(&http, "", pp, mode).await
    } else if let Some(k) = key {
        crate::utils::get_rank_from_daily(&http, &k, pp, mode).await
    } else {
        None
    };
    if let Some(r) = rank {
        let mut s = state.write().await;
        if let Some(ref mut p) = s.player {
            p.rank = r;
            let pres = crate::packets::user_presence(p);
            p.queue.extend_from_slice(&pres);
            p.enqueue_stats();
        }
    }
    json_ok(&serde_json::json!({ "ok": true, "era": era }))
}

static LADDER_LAST_REFRESH: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);

async fn handle_ladder_refresh(state: SharedState, headers: &hyper::HeaderMap) -> Response {
    let now = now_secs();
    let token = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME))
        .map(|t| t.to_string());
    let session = {
        let st = state.read().await;
        token.and_then(|t| st.web_sessions.get(&t).filter(|s| s.expires > now).cloned())
    };
    let session = match session {
        Some(s) => s,
        None => return json_error(StatusCode::UNAUTHORIZED, "Sign in to refresh the rank ladder."),
    };
    if !can_write(Some(&session), headers) {
        return json_error(StatusCode::FORBIDDEN, "Refresh the page and try again.");
    }
    {
        let mut last = LADDER_LAST_REFRESH.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed() < std::time::Duration::from_secs(600) {
                return json_error(StatusCode::TOO_MANY_REQUESTS, "Refreshed recently; try again in a few minutes.");
            }
        }
        *last = Some(Instant::now());
    }
    let http = state.read().await.http.clone();
    match crate::ladder::refresh(&http).await {
        Ok(last) => json_ok(&serde_json::json!({ "ok": true, "last_date": last })),
        Err(e) => json_error(StatusCode::BAD_GATEWAY, &e),
    }
}

async fn handle_medals_catalog() -> Response {
    json_ok(crate::types::medal::get_all_medals())
        .with_header("Cache-Control", "public, max-age=86400")
}

fn weighted_total(
    scores: &[&db::ScoreWithBeatmap],
    pp_of: impl Fn(&db::ScoreWithBeatmap) -> f64,
    bonus: Option<f64>,
    cutoff: i64,
) -> f64 {
    let ranked: Vec<&&db::ScoreWithBeatmap> = scores
        .iter()
        .filter(|s| (s.bmap_status == "ranked" || s.bmap_status == "approved") && s.time <= cutoff)
        .collect();
    let mut best: HashMap<&str, f64> = HashMap::new();
    for s in &ranked {
        let e = best.entry(s.md5.as_str()).or_insert(0.0);
        let pp = pp_of(s);
        if pp > *e {
            *e = pp;
        }
    }
    let mut pps: Vec<f64> = best.into_values().collect();
    pps.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let mut total: f64 = pps.iter().enumerate().map(|(i, pp)| pp * 0.95_f64.powi(i as i32)).sum();
    if !ranked.is_empty() {
        total += bonus.unwrap_or_else(|| 416.6667 * (1.0 - 0.9994_f64.powi(ranked.len() as i32)));
    }
    total
}

fn era_rank_history(
    mode_scores: &[&db::ScoreWithBeatmap],
    ladder_date: &str,
    today_pp: f64,
    today_rank: Option<i32>,
) -> Vec<(String, i32)> {
    let Some(ladder) = crate::ladder::get() else {
        return Vec::new();
    };
    let today = chrono::Utc::now().date_naive();
    let mut out = Vec::new();
    for back in (0..90).rev() {
        let day = today - chrono::Duration::days(back);
        let date = day.format("%Y-%m-%d").to_string();
        let rank = if back == 0 {
            today_rank
        } else {
            let end = day.and_hms_opt(23, 59, 59).map(|d| d.and_utc().timestamp()).unwrap_or(i64::MAX);
            let pp = weighted_total(mode_scores, |s| s.pp, None, end);
            if pp > 0.0 { ladder.rank_at_pp_on(ladder_date, pp) } else { None }
        };
        if let Some(r) = rank {
            out.push((date, r));
        }
    }
    let _ = today_pp;
    out
}

pub async fn handle_profile(state: SharedState, params: &HashMap<String, String>) -> Response {
    let name = match params.get("name") {
        Some(n) if !n.trim().is_empty() => n.trim(),
        _ => return json_error(StatusCode::BAD_REQUEST, "Profile username is required."),
    };

    let state_guard = state.read().await;
    let conn = state_guard.db.lock().await;

    if !db::profile_exists(&conn, name).unwrap_or(false) {
        return json_error(
            StatusCode::NOT_FOUND,
            "This local profile does not exist. Sign in with its username to create it.",
        );
    }

    let mut details = db::get_profile_details(&conn, name).unwrap_or_default();
    let has_cover = db::get_cover(&conn, name).ok().flatten().is_some();
    let mut scores = db::get_player_scores_with_beatmaps(&conn, name).unwrap_or_default();

    let pp_system: &'static str = params
        .get("ppsys")
        .and_then(|e| crate::pp_era::parse(e))
        .unwrap_or(crate::pp_era::LIVE);
    let mut pp_system_missing = 0usize;
    let live_scores: Option<Vec<db::ScoreWithBeatmap>> = (pp_system != crate::pp_era::LIVE).then(|| scores.clone());
    if pp_system != crate::pp_era::LIVE {
        let have = db::get_era_pp(&conn, name, pp_system).unwrap_or_default();
        scores.retain_mut(|s| {
            if s.mode != 0 {
                return true;
            }
            match have.get(&s.id) {
                Some(pp) => {
                    s.pp = *pp;
                    true
                }
                None => {
                    pp_system_missing += 1;
                    false
                }
            }
        });
    }
    let first_sets: Vec<std::collections::HashSet<String>> = (0..4).map(|m| db::get_first_place_md5s(&conn, name, m)).collect();

    let mode_str = params.get("mode").map(|s| s.as_str()).unwrap_or("osu");
    if !["osu", "taiko", "fruits", "mania", "vn", "rx", "ap"].contains(&mode_str) {
        return json_error(StatusCode::BAD_REQUEST, "Unknown mode.");
    }

    let is_active = state_guard.player.as_ref().map_or(false, |p| p.name == name);
    let http_client = state_guard.http.clone();
    let daily_key = state_guard.config.osu_daily_api_key.clone();
    let oauth_creds = match (
        &state_guard.config.osu_client_id,
        &state_guard.config.osu_client_secret,
    ) {
        (Some(cid), Some(sec)) => cid
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .parse::<u64>()
            .ok()
            .map(|id| (id, sec.trim().trim_matches('"').trim_matches('\'').to_string())),
        _ => None,
    };

    let effective_country = if !details.country.trim().is_empty() {
        details.country.trim().to_uppercase()
    } else if let Some(ref c) = state_guard.config.country {
        c.trim().to_uppercase()
    } else if let Ok(byte) = db::get_profile_country(&conn, name) {
        if byte != 0 {
            crate::utils::country_byte_to_code(byte).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    if details.country.is_empty() && !effective_country.is_empty() {
        details.country = effective_country.clone();
    }

    drop(conn);
    drop(state_guard);

    if pp_system_missing > 0 {
        crate::handlers::era_pp::spawn_background_fill(state.clone(), name.to_string());
    }

    let (rx_bit, ap_bit) = (
        Mods::RELAX.bits() as u32,
        Mods::AUTOPILOT.bits() as u32,
    );

    let target_ruleset: Option<i32> = match mode_str {
        "taiko" | "1" => Some(1),
        "fruits" | "catch" | "2" => Some(2),
        "mania" | "3" => Some(3),
        "osu" | "0" | "vn" | "rx" | "ap" => Some(0),
        _ => None,
    };

    let mode_scores: Vec<&db::ScoreWithBeatmap> = scores
        .iter()
        .filter(|s| {
            if let Some(ruleset) = target_ruleset {
                if s.mode != ruleset {
                    return false;
                }
            }
            match mode_str {
                "rx" => (s.mods & rx_bit) != 0,
                "ap" => (s.mods & ap_bit) != 0,
                _ => (s.mods & (rx_bit | ap_bit)) == 0,
            }
        })
        .collect();

    let mut ranked: Vec<&&db::ScoreWithBeatmap> = mode_scores
        .iter()
        .filter(|s| s.bmap_status == "ranked" || s.bmap_status == "approved")
        .collect();
    ranked.sort_by(|a, b| b.pp.partial_cmp(&a.pp).unwrap_or(std::cmp::Ordering::Equal));

    let mut seen_md5 = std::collections::HashSet::new();
    let mut top_scores: Vec<&&db::ScoreWithBeatmap> = Vec::new();
    for s in &ranked {
        if seen_md5.insert(&s.md5) {
            top_scores.push(s);
        }
    }

    let mut weighted_pp = 0.0;
    let mut weighted_acc = 0.0;
    let mut weight_sum = 0.0;
    for (i, s) in top_scores.iter().enumerate() {
        let weight = 0.95_f64.powi(i as i32);
        weighted_pp += s.pp * weight;
        if i < 100 {
            weighted_acc += s.acc * weight;
            weight_sum += weight;
        }
    }
    if !ranked.is_empty() {
        let imported_bonus = match (mode_str, target_ruleset) {
            ("rx", _) | ("ap", _) => None,
            (_, Some(m)) => crate::osu_api::bonus(&name, m),
            _ => None,
        };
        weighted_pp += imported_bonus.unwrap_or_else(|| 416.6667 * (1.0 - 0.9994_f64.powi(ranked.len() as i32)));
    }
    let calculated_pp = weighted_pp.round() as i32;
    let calculated_pp_exact = (weighted_pp * 10000.0).round() / 10000.0;
    let calculated_acc = if weight_sum > 0.0 { weighted_acc / weight_sum } else { 0.0 };

    let mut grade_counts: HashMap<String, i32> = [
        ("XH".to_string(), 0),
        ("X".to_string(), 0),
        ("SH".to_string(), 0),
        ("S".to_string(), 0),
        ("A".to_string(), 0),
    ].into_iter().collect();

    let mut seen_for_grades = std::collections::HashSet::new();
    for s in &ranked {
        if seen_for_grades.insert(&s.md5) {
            let g = score_grade(s);
            if let Some(cnt) = grade_counts.get_mut(g) {
                *cnt += 1;
            }
        }
    }

    let mut best_scores_per_map: HashMap<&str, i64> = HashMap::new();
    for s in &ranked {
        let entry = best_scores_per_map.entry(&s.md5).or_insert(0);
        if s.score > *entry {
            *entry = s.score;
        }
    }
    let mut ranked_score: i64 = best_scores_per_map.values().sum();
    let mut total_score: i64 = mode_scores.iter().map(|s| s.score).sum();
    let mut total_hits: i64 = mode_scores.iter().map(|s| (s.n300 + s.n100 + s.n50) as i64).sum();
    let mut max_combo: i32 = mode_scores.iter().map(|s| s.max_combo).max().unwrap_or(0);
    let mut playcount = mode_scores.len() as i32;

    let recent_public: Vec<PublicScore> = mode_scores.iter().take(100).map(|s| to_public_score(s)).collect();
    let first_public: Vec<PublicScore> = match (mode_str, target_ruleset) {
        ("rx", _) | ("ap", _) => Vec::new(),
        (_, Some(m)) => {
            let set = &first_sets[(m as usize) % 4];
            let mut best: HashMap<&str, &db::ScoreWithBeatmap> = HashMap::new();
            for s in mode_scores.iter().filter(|s| set.contains(&s.md5)) {
                let e = best.entry(s.md5.as_str()).or_insert(*s);
                if s.score > e.score {
                    *e = *s;
                }
            }
            let mut v: Vec<&db::ScoreWithBeatmap> = best.into_values().collect();
            v.sort_by(|a, b| b.time.cmp(&a.time));
            v.into_iter().take(200).map(|s| to_public_score(s)).collect()
        }
        _ => Vec::new(),
    };
    let top_public: Vec<PublicScore> = top_scores.iter().take(200).map(|s| to_public_score(s)).collect();

    let now_ts = now_secs() as i64;
    let recent_24h_public: Vec<PublicScore> = mode_scores
        .iter()
        .filter(|s| s.time >= now_ts - 86400)
        .take(100)
        .map(|s| to_public_score(s))
        .collect();

    let last_play = mode_scores.first().map(|s| s.time);
    let first_play = scores.iter().map(|s| s.time).filter(|&t| t > 0).min();

    let mut map_counts: HashMap<&str, (i32, &db::ScoreWithBeatmap)> = HashMap::new();
    for s in &mode_scores {
        let entry = map_counts.entry(&s.md5).or_insert((0, s));
        entry.0 += 1;
    }
    let mut most_played: Vec<MostPlayedEntry> = map_counts
        .into_iter()
        .map(|(_, (count, rep))| MostPlayedEntry {
            count,
            beatmap: to_public_score(rep).beatmap,
        })
        .collect();
    most_played.sort_by(|a, b| b.count.cmp(&a.count));
    most_played.truncate(100);

    let mut month_counts: HashMap<String, i32> = HashMap::new();
    for s in &mode_scores {
        if s.time > 0 {
            if let Some(dt) = chrono::DateTime::from_timestamp(s.time, 0) {
                let month = dt.format("%Y-%m").to_string();
                *month_counts.entry(month).or_insert(0) += 1;
            }
        }
    }
    let mut play_history: Vec<(String, i32)> = month_counts.into_iter().collect();
    play_history.sort_by(|a, b| a.0.cmp(&b.0));

    let mut day_peaks: HashMap<String, f64> = HashMap::new();
    for s in &mode_scores {
        if s.time > 0 && s.pp > 0.0 {
            if let Some(dt) = chrono::DateTime::from_timestamp(s.time, 0) {
                let day = dt.format("%Y-%m-%d").to_string();
                let entry = day_peaks.entry(day).or_insert(0.0);
                if s.pp > *entry {
                    *entry = s.pp;
                }
            }
        }
    }
    let mut perf_history: Vec<(String, f64)> = day_peaks.into_iter().collect();
    perf_history.sort_by(|a, b| a.0.cmp(&b.0));
    if perf_history.len() > 90 {
        perf_history = perf_history.split_off(perf_history.len() - 90);
    }

    let mode_id = match mode_str {
        "rx" => 100,
        "ap" => 200,
        "taiko" => 1,
        "fruits" => 2,
        "mania" => 3,
        _ => 0,
    };

    let era = params
        .get("era")
        .map(|e| e.as_str())
        .filter(|e| crate::ladder::valid_era(e) && (mode_str == "vn" || mode_str == "osu"))
        .map(|e| e.to_string());

    let pp_active = pp_system != crate::pp_era::LIVE && target_ruleset == Some(0);
    let follow_system = pp_active && params.get("ppfollow").map_or(true, |v| v != "0");
    let rank_pp: f64 = if pp_active && !follow_system {
        let live = live_scores.as_deref().unwrap_or(&[]);
        let live_mode: Vec<&db::ScoreWithBeatmap> = live
            .iter()
            .filter(|s| s.mode == 0)
            .filter(|s| match mode_str {
                "rx" => (s.mods & rx_bit) != 0,
                "ap" => (s.mods & ap_bit) != 0,
                _ => (s.mods & (rx_bit | ap_bit)) == 0,
            })
            .collect();
        let bonus = match mode_str {
            "rx" | "ap" => None,
            _ => crate::osu_api::bonus(&name, 0),
        };
        weighted_total(&live_mode, |s| s.pp, bonus, i64::MAX)
    } else {
        calculated_pp_exact
    };
    let system_end_date: Option<String> = if follow_system {
        crate::pp_era::end_date(pp_system).and_then(|d| {
            let l = crate::ladder::get()?;
            let (first, last) = (l.first_date(), l.last_date());
            let d = d.to_string();
            Some(if d < first { first } else if d > last { last } else { d })
        })
    } else {
        None
    };

    let global_rank = if follow_system {
        system_end_date.as_deref().and_then(|d| {
            if calculated_pp > 0 {
                crate::ladder::get()?.rank_at_pp_on(d, calculated_pp_exact)
            } else {
                None
            }
        })
    } else if let Some(ref e) = era {
        era_rank(e, rank_pp)
    } else {
        get_daily_rank(&http_client, daily_key.as_deref(), rank_pp).await
    };
    let country_rank = if !pp_active && era.is_none() && (mode_str == "vn" || mode_str == "osu" || mode_str == "0") && !effective_country.is_empty() && calculated_pp > 0 {
        get_country_rank(
            &http_client,
            &effective_country,
            calculated_pp_exact,
            oauth_creds.as_ref().map(|(id, sec)| (*id, sec.as_str())),
        )
        .await
    } else {
        None
    };

    let state_guard = state.read().await;
    let conn = state_guard.db.lock().await;
    let play_time_secs = db::get_playtime(
        &conn,
        name,
        match mode_str {
            "taiko" => 1,
            "fruits" => 2,
            "mania" => 3,
            _ => 0,
        },
    )
    .unwrap_or(0);
    let mut replays_watched: i64 = 0;
    let mut official_stats = false;
    let mut play_time_secs = play_time_secs;
    if mode_id < 100 {
        if let Some(o) = db::get_official_stats(&conn, name, mode_id) {
            official_stats = true;
            let fresh: Vec<&&db::ScoreWithBeatmap> = mode_scores.iter().filter(|s| s.time > o.imported_at).collect();
            playcount = (o.play_count + fresh.len() as i64).clamp(0, i32::MAX as i64) as i32;
            total_score = o.total_score + fresh.iter().map(|s| s.score).sum::<i64>();
            total_hits = o.total_hits + fresh.iter().map(|s| (s.n300 + s.n100 + s.n50) as i64).sum::<i64>();
            max_combo = max_combo.max(o.max_combo as i32);
            ranked_score = ranked_score.max(o.ranked_score);
            replays_watched = o.replays_watched;
            play_time_secs = o.play_time + (play_time_secs - o.local_playtime_at_import).max(0);
            for (k, v) in [("XH", o.grade_xh), ("X", o.grade_x), ("SH", o.grade_sh), ("S", o.grade_s), ("A", o.grade_a)] {
                if let Some(c) = grade_counts.get_mut(k) {
                    *c = (*c).max(v as i32);
                }
            }
        }
    }
    if era.is_none() && !pp_active {
        if let Some(rank) = global_rank {
            if calculated_pp > 0 {
                let _ = db::record_rank_snapshot_pp(&conn, name, mode_id, rank, calculated_pp_exact);
            } else {
                let _ = db::record_rank_snapshot(&conn, name, mode_id, rank);
            }
        }
    }
    let (rank_history, mut rank_highest) = if follow_system {
        let series = match system_end_date.as_deref() {
            Some(d) => era_rank_history(&mode_scores, d, calculated_pp_exact, global_rank),
            None => Vec::new(),
        };
        let highest = series
            .iter()
            .min_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
            .map(|(d, r)| crate::db::RankHighest { rank: *r, updated_at: d.clone() });
        (series, highest)
    } else if let Some(ref e) = era {
        let raw = db::get_rank_history_pp(&conn, name, mode_id, 90).unwrap_or_default();
        let ladder = crate::ladder::get();
        let mut out: Vec<(String, i32)> = Vec::new();
        for (date, rank, pp) in raw {
            let pp = pp.or_else(|| ladder.as_ref().and_then(|l| l.pp_at_rank_on(&date, rank as f64)));
            if let Some(r) = pp.and_then(|p| era_rank(e, p)) {
                out.push((date, r));
            }
        }
        let highest = out
            .iter()
            .min_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
            .map(|(d, r)| crate::db::RankHighest { rank: *r, updated_at: d.clone() });
        (out, highest)
    } else {
        (
            db::get_rank_history(&conn, name, mode_id, 90).unwrap_or_default(),
            db::get_highest_rank(&conn, name, mode_id).unwrap_or(None),
        )
    };
    if let (Some(current_rank), true) = (global_rank, era.is_none()) {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        if let Some(highest) = &mut rank_highest {
            if current_rank < highest.rank {
                highest.rank = current_rank;
                highest.updated_at = today;
            }
        } else {
            rank_highest = Some(crate::db::RankHighest {
                rank: current_rank,
                updated_at: today,
            });
        }
    }
    let mut rank_history = rank_history;
    if let (Some(ref _e), Some(cur)) = (&era, global_rank) {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        match rank_history.last_mut() {
            Some(last) if last.0 == today => last.1 = cur,
            _ => rank_history.push((today.clone(), cur)),
        }
        match &mut rank_highest {
            Some(h) if cur < h.rank => {
                h.rank = cur;
                h.updated_at = today;
            }
            None => rank_highest = Some(crate::db::RankHighest { rank: cur, updated_at: today }),
            _ => {}
        }
    }
    let user_medals = db::get_user_medals(&conn, name).unwrap_or_default();
    let medals_display: Vec<crate::types::medal::UserMedalDisplay> = user_medals
        .into_iter()
        .filter_map(|rec| {
            crate::types::medal::get_medal_by_id(rec.medal_id).map(|def| crate::types::medal::UserMedalDisplay {
                id: def.id,
                name: def.name.clone(),
                description: def.description.clone(),
                category: def.category.as_str().to_string(),
                icon_url: def.icon_url.clone(),
                achieved_at: rec.achieved_at,
            })
        })
        .collect();
    drop(conn);
    drop(state_guard);

    json_ok(&ProfileResponse {
        name: name.to_string(),
        mode: mode_str.to_string(),
        pp: calculated_pp,
        pp_exact: calculated_pp_exact,
        acc: (calculated_acc * 100.0).round() / 100.0,
        playcount,
        play_time_secs,
        total_score,
        ranked_score,
        total_hits,
        max_combo,
        replays_watched,
        official_stats,
        grades: grade_counts,
        active: is_active,
        details,
        country_rank,
        global_rank,
        rank_history,
        top: top_public,
        first_places: first_public,
        recent: recent_public,
        last_play,
        first_play,
        performance_history: perf_history,
        play_history,
        most_played,
        recent_24h: recent_24h_public,
        medals: medals_display,
        rank_highest,
        has_cover,
        rank_era: era.clone().unwrap_or_else(|| "latest".to_string()),
        rank_era_date: era.as_deref().and_then(era_date),
        pp_system: pp_system.to_string(),
        pp_system_missing,
        pp_system_filling: crate::handlers::era_pp::is_filling(name),
    })
}

async fn handle_session(state: SharedState, headers: &hyper::HeaderMap) -> Response {
    let now = now_secs();
    let cookie_val = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME));

    let mut state_guard = state.write().await;

    state_guard.web_sessions.retain(|_, s| s.expires > now);

    let (token, csrf_token, username, set_cookie) = match cookie_val.and_then(|t| state_guard.web_sessions.get(t).map(|s| (t.to_string(), s.clone()))) {
        Some((t, session)) => (t, session.csrf_token, session.username, false),
        None => {
            let new_token = generate_token();
            let new_csrf = generate_token();
            let new_session = WebSession {
                csrf_token: new_csrf.clone(),
                username: None,
                expires: now + SESSION_AGE_SECS,
            };
            state_guard.web_sessions.insert(new_token.clone(), new_session);
            (new_token, new_csrf, None, true)
        }
    };

    let active = state_guard.player.as_ref().map(|p| p.name.clone());

    let (profiles, default, country) = {
        let conn = state_guard.db.lock().await;
        let p_list = db::list_profiles(&conn).unwrap_or_default();
        let def = db::get_default_profile(&conn).unwrap_or_default();
        let country = username.as_deref().and_then(|u| db::get_profile_details(&conn, u).ok()).map(|d| d.country).unwrap_or_default();
        (p_list, def, country)
    };

    drop(state_guard);

    let mut resp = json_ok(&SessionResponse {
        csrf: csrf_token,
        user: username,
        profiles,
        active,
        default,
        country,
    });

    if set_cookie {
        let cookie_str = format!(
            "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
            SESSION_COOKIE_NAME, token, SESSION_AGE_SECS
        );
        resp = resp.with_header("Set-Cookie", &cookie_str);
    }

    resp
}

async fn handle_login(state: SharedState, headers: &hyper::HeaderMap, body: &[u8]) -> Response {
    let now = now_secs();
    let cookie_val = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME));

    let token = match cookie_val {
        Some(t) => t.to_string(),
        None => return json_error(StatusCode::FORBIDDEN, "Refresh the page and try signing in again."),
    };

    let mut state_guard = state.write().await;
    state_guard.web_sessions.retain(|_, s| s.expires > now);

    let can_write_ok = {
        let session = state_guard.web_sessions.get(&token);
        can_write(session, headers)
    };

    if !can_write_ok {
        return json_error(StatusCode::FORBIDDEN, "Refresh the page and try signing in again.");
    }

    let payload: LoginPayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(_) => return json_error(StatusCode::BAD_REQUEST, "Invalid request payload."),
    };

    let username = match validate_username(&payload.username) {
        Ok(name) => name,
        Err(err) => return json_error(StatusCode::BAD_REQUEST, err),
    };

    {
        let conn = state_guard.db.lock().await;
        if let Err(e) = db::ensure_profile(&conn, &username) {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("Database error: {}", e));
        }
    }

    if let Some(session) = state_guard.web_sessions.get_mut(&token) {
        session.username = Some(username.clone());
    }

    json_ok(&LoginResponse { name: username })
}

async fn handle_logout(state: SharedState, headers: &hyper::HeaderMap) -> Response {
    let now = now_secs();
    let cookie_val = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME));

    let token = match cookie_val {
        Some(t) => t.to_string(),
        None => return json_error(StatusCode::FORBIDDEN, "Refresh the page and try again."),
    };

    let mut state_guard = state.write().await;
    state_guard.web_sessions.retain(|_, s| s.expires > now);

    let can_write_ok = {
        let session = state_guard.web_sessions.get(&token);
        can_write(session, headers)
    };

    if !can_write_ok {
        return json_error(StatusCode::FORBIDDEN, "Refresh the page and try again.");
    }

    if let Some(session) = state_guard.web_sessions.get_mut(&token) {
        session.username = None;
    }

    json_ok(&SuccessResponse { ok: true })
}

async fn serve_guest_avatar(state: &SharedState) -> Response {
    if let Some(asset) = EmbeddedAssets::get("vendor/avatar-guest@2x.01495bc4.png") {
        return Response::image(asset.data.to_vec(), "png")
            .with_header("Cache-Control", "public, max-age=86400");
    }

    let state_read = state.read().await;
    if !state_read.default_avatar.is_empty() {
        return Response::image(state_read.default_avatar.clone(), "png")
            .with_header("Cache-Control", "public, max-age=86400");
    }

    Response::not_found()
}

async fn handle_avatar(state: SharedState, params: &HashMap<String, String>) -> Response {
    let name = match params.get("name") {
        Some(n) if !n.trim().is_empty() => n.trim(),
        _ => return serve_guest_avatar(&state).await,
    };

    let state_read = state.read().await;
    let conn = state_read.db.lock().await;
    let avatar_url = db::get_avatar(&conn, name).unwrap_or(None);
    drop(conn);
    drop(state_read);

    let Some(pfp) = avatar_url else {
        return serve_guest_avatar(&state).await;
    };

    if let Some(path) = utils::is_path(&pfp) {
        if let Ok(bytes) = tokio::fs::read(&path).await {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
            return Response::image(bytes, ext)
                .with_header("Cache-Control", "public, max-age=86400");
        }
    } else if pfp.starts_with("http://") || pfp.starts_with("https://") {
        let state_read = state.read().await;
        let client = state_read.http.clone();
        drop(state_read);

        if let Ok(resp) = client.get(&pfp).send_traced().await {
            if resp.status().is_success() {
                if let Ok(bytes) = resp.bytes().await {
                    let ext = if pfp.ends_with(".jpg") || pfp.ends_with(".jpeg") {
                        "jpg"
                    } else if pfp.ends_with(".gif") {
                        "gif"
                    } else if pfp.ends_with(".webp") {
                        "webp"
                    } else {
                        "png"
                    };
                    return Response::image(bytes.to_vec(), ext)
                        .with_header("Cache-Control", "public, max-age=86400");
                }
            }
        }
    }

    serve_guest_avatar(&state).await
}

async fn handle_cover(state: SharedState, params: &HashMap<String, String>) -> Response {
    let name = match params.get("name") {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => return json_error(StatusCode::NOT_FOUND, "No cover"),
    };

    let state_read = state.read().await;
    let conn = state_read.db.lock().await;
    let cover = db::get_cover(&conn, &name).unwrap_or(None);
    drop(conn);
    drop(state_read);

    if let Some(path) = cover.as_deref().and_then(utils::is_path) {
        if let Ok(bytes) = tokio::fs::read(&path).await {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
            return Response::image(bytes, ext).with_header("Cache-Control", "no-cache");
        }
    }
    json_error(StatusCode::NOT_FOUND, "No cover")
}

async fn handle_settings(
    state: SharedState,
    headers: &hyper::HeaderMap,
    body: &[u8],
) -> Response {
    let now = now_secs();
    let cookie_val = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME));

    let token = match cookie_val {
        Some(t) => t.to_string(),
        None => return json_error(StatusCode::UNAUTHORIZED, "Sign in to update your profile."),
    };

    let state_read = state.read().await;
    let session = match state_read.web_sessions.get(&token) {
        Some(s) if s.expires > now => s.clone(),
        _ => return json_error(StatusCode::UNAUTHORIZED, "Session expired. Please sign in again."),
    };
    drop(state_read);

    if !can_write(Some(&session), headers) {
        return json_error(StatusCode::FORBIDDEN, "Refresh the page and try again.");
    }

    let current_user = match session.username {
        Some(ref u) if !u.is_empty() => u.clone(),
        _ => return json_error(StatusCode::UNAUTHORIZED, "Sign in to update your profile."),
    };

    let payload: SettingsPayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, &format!("Invalid settings JSON: {}", e)),
    };

    if let Some(ref details) = payload.details {
        if let Err(err_msg) = details.validate() {
            return json_error(StatusCode::BAD_REQUEST, err_msg);
        }
    }

    let target_username = if let Some(ref new_name_raw) = payload.username {
        let trimmed = new_name_raw.trim();
        if trimmed != current_user {
            let valid_name = match validate_username(trimmed) {
                Ok(n) => n,
                Err(e) => return json_error(StatusCode::BAD_REQUEST, e),
            };

            let s_read = state.read().await;
            let conn = s_read.db.lock().await;
            let exists = db::profile_exists(&conn, &valid_name).unwrap_or(false);
            if exists {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    &format!("The username '{}' is already taken.", valid_name),
                );
            }
            if let Err(e) = db::rename_profile(&conn, &current_user, &valid_name) {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to rename profile: {}", e),
                );
            }
            drop(conn);
            drop(s_read);
            valid_name
        } else {
            current_user.clone()
        }
    } else {
        current_user.clone()
    };

    let mut old_avatar_to_delete: Option<PathBuf> = None;
    if payload.reset_avatar {
        let s_read = state.read().await;
        let conn = s_read.db.lock().await;
        if let Ok(Some(old_url)) = db::get_avatar(&conn, &target_username) {
            if let Some(p) = utils::is_path(&old_url) {
                old_avatar_to_delete = Some(p);
            }
        }
        let _ = db::clear_avatar(&conn, &target_username);
        drop(conn);
        drop(s_read);
    } else if let Some(ref avatar_data_str) = payload.avatar {
        let (ext, img_bytes) = match parse_and_validate_avatar(avatar_data_str) {
            Ok(res) => res,
            Err(err_msg) => return json_error(StatusCode::BAD_REQUEST, err_msg),
        };

        let avatar_dir = Path::new(".data").join("avatars");
        let _ = tokio::fs::create_dir_all(&avatar_dir).await;
        let token_hex = generate_hex_token(16);
        let filename = format!("{}.{}", token_hex, ext);
        let file_path = avatar_dir.join(&filename);

        if let Err(e) = tokio::fs::write(&file_path, &img_bytes).await {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to save avatar image: {}", e),
            );
        }

        let s_read = state.read().await;
        let conn = s_read.db.lock().await;
        if let Ok(Some(old_url)) = db::get_avatar(&conn, &target_username) {
            if let Some(p) = utils::is_path(&old_url) {
                old_avatar_to_delete = Some(p);
            }
        }
        let stored_path = file_path.to_string_lossy().to_string();
        let _ = db::set_avatar(&conn, &target_username, &stored_path);
        drop(conn);
        drop(s_read);
    }

    if let Some(old_path) = old_avatar_to_delete {
        let p_str = old_path.to_string_lossy();
        if p_str.contains(".data") && p_str.contains("avatars") {
            let _ = tokio::fs::remove_file(old_path).await;
        }
    }

    let mut old_cover_to_delete: Option<PathBuf> = None;
    if payload.reset_cover {
        let s_read = state.read().await;
        let conn = s_read.db.lock().await;
        if let Ok(Some(old_url)) = db::get_cover(&conn, &target_username) {
            if let Some(p) = utils::is_path(&old_url) {
                old_cover_to_delete = Some(p);
            }
        }
        let _ = db::clear_cover(&conn, &target_username);
        drop(conn);
        drop(s_read);
    } else if let Some(ref cover_data_str) = payload.cover {
        let (ext, img_bytes) = match parse_and_validate_image(cover_data_str, 6 * 1024 * 1024) {
            Ok(res) => res,
            Err(err_msg) => return json_error(StatusCode::BAD_REQUEST, err_msg),
        };

        let cover_dir = Path::new(".data").join("covers");
        let _ = tokio::fs::create_dir_all(&cover_dir).await;
        let filename = format!("{}.{}", generate_hex_token(16), ext);
        let file_path = cover_dir.join(&filename);

        if let Err(e) = tokio::fs::write(&file_path, &img_bytes).await {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to save cover image: {}", e),
            );
        }

        let s_read = state.read().await;
        let conn = s_read.db.lock().await;
        if let Ok(Some(old_url)) = db::get_cover(&conn, &target_username) {
            if let Some(p) = utils::is_path(&old_url) {
                old_cover_to_delete = Some(p);
            }
        }
        let stored_path = file_path.to_string_lossy().to_string();
        let _ = db::set_cover(&conn, &target_username, &stored_path);
        drop(conn);
        drop(s_read);
    }

    if let Some(old_path) = old_cover_to_delete {
        let p_str = old_path.to_string_lossy();
        if p_str.contains(".data") && p_str.contains("covers") {
            let _ = tokio::fs::remove_file(old_path).await;
        }
    }

    let mut country_byte: u8 = 0;
    if let Some(mut details) = payload.details {
        details.country = details.country.trim().to_uppercase();
        if !details.country.is_empty() {
            country_byte = crate::utils::country_code_to_byte(&details.country);
        }
        let s_read = state.read().await;
        let conn = s_read.db.lock().await;
        if let Err(e) = db::save_profile_details(&conn, &target_username, &details) {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to save profile details: {}", e),
            );
        }
        drop(conn);
        drop(s_read);
    }

    let mut s_write = state.write().await;
    if target_username != current_user {
        for s in s_write.web_sessions.values_mut() {
            if s.username.as_deref() == Some(&current_user) {
                s.username = Some(target_username.clone());
            }
        }
        if s_write.pending_login_name.as_deref() == Some(&current_user) {
            s_write.pending_login_name = Some(target_username.clone());
        }
        if s_write.config.osu_username.as_deref() == Some(&current_user) {
            s_write.config.osu_username = Some(target_username.clone());
        }
    }

    if let Some(ref mut p) = s_write.player {
        if p.name == current_user || p.name == target_username {
            if target_username != current_user {
                p.name = target_username.clone();
            }
            if country_byte > 0 {
                p.country = country_byte;
            }
            p.queue.extend_from_slice(&crate::packets::user_presence(p));
            p.queue.extend_from_slice(&crate::packets::user_stats(p));
        }
    }
    drop(s_write);

    json_ok(&LoginResponse { name: target_username })
}

async fn handle_delete_score(
    state: SharedState,
    headers: &hyper::HeaderMap,
    body: &[u8],
) -> Response {
    let now = now_secs();
    let cookie_val = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME));

    let token = match cookie_val {
        Some(t) => t.to_string(),
        None => return json_error(StatusCode::UNAUTHORIZED, "Sign in to manage your scores."),
    };

    let state_read = state.read().await;
    let session = match state_read.web_sessions.get(&token) {
        Some(s) if s.expires > now => s.clone(),
        _ => return json_error(StatusCode::UNAUTHORIZED, "Session expired. Please sign in again."),
    };
    drop(state_read);

    if !can_write(Some(&session), headers) {
        return json_error(StatusCode::FORBIDDEN, "Refresh the page and try again.");
    }

    let current_user = match session.username {
        Some(ref u) if !u.is_empty() => u.clone(),
        _ => return json_error(StatusCode::UNAUTHORIZED, "Sign in to manage your scores."),
    };

    let payload: DeleteScorePayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, &format!("Invalid request body: {}", e)),
    };

    {
        let state_guard = state.read().await;
        let conn = state_guard.db.lock().await;
        let deleted = match db::delete_score_by_id(&conn, &current_user, payload.id) {
            Ok(d) => d,
            Err(e) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Database error deleting score: {}", e),
                );
            }
        };

        if !deleted {
            return json_error(
                StatusCode::NOT_FOUND,
                "Score not found or not owned by your current profile.",
            );
        }

        let _ = db::decrement_playcount(&conn, &current_user);
    }

    let (_recalc_count, new_pp, new_acc) =
        crate::handlers::banchobot::recalculate_profile(&state, &current_user).await;

    let mut s_write = state.write().await;
    if let Some(ref mut p) = s_write.player {
        if p.name == current_user {
            p.pp = new_pp;
            p.acc = new_acc;
            p.queue.extend_from_slice(&crate::packets::user_stats(p));
            p.queue.extend_from_slice(&crate::packets::user_presence(p));
        }
    }
    drop(s_write);

    json_ok(&SuccessResponse { ok: true })
}

#[derive(Deserialize)]
struct ImportScoresPayload {
    query: String,
    #[serde(default)]
    mode: String,
    #[serde(default)]
    limit: usize,
}

#[derive(Deserialize)]
struct ImportRanksPayload {
    query: String,
    mode: String,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    count: i64,
}

async fn handle_import_ranks(state: SharedState, headers: &hyper::HeaderMap, body: &[u8]) -> Response {
    let now = now_secs();
    let token = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME))
        .map(|t| t.to_string());
    let session = {
        let st = state.read().await;
        token.and_then(|t| st.web_sessions.get(&t).filter(|s| s.expires > now).cloned())
    };
    let Some(session) = session else { return json_error(StatusCode::UNAUTHORIZED, "Sign in to import scores.") };
    if !can_write(Some(&session), headers) {
        return json_error(StatusCode::FORBIDDEN, "Refresh the page and try again.");
    }
    let player = match session.username {
        Some(ref u) if !u.is_empty() => u.clone(),
        _ => return json_error(StatusCode::UNAUTHORIZED, "Sign in to import scores."),
    };
    let payload: ImportRanksPayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, &format!("Invalid request body: {}", e)),
    };
    let mode: i32 = match payload.mode.as_str() {
        "osu" | "" => 0,
        "taiko" => 1,
        "fruits" | "catch" => 2,
        "mania" => 3,
        _ => return json_error(StatusCode::BAD_REQUEST, "Unknown game mode."),
    };
    let offset = payload.offset.clamp(0, 100);
    let count = payload.count.clamp(1, 25).min(100 - offset);
    if count <= 0 {
        return json_ok(&serde_json::json!({ "ok": true, "checked": 0, "ranked": 0, "next_offset": serde_json::Value::Null }));
    }
    let (http, db_handle) = {
        let st = state.read().await;
        (st.http.clone(), st.db.clone())
    };
    let maps = {
        let conn = db_handle.lock().await;
        db::top_play_maps(&conn, &player, mode, offset, count)
    };
    let user = match crate::osu_api::user(&http, &payload.query).await {
        Ok(u) => u,
        Err((code, msg)) => return json_error(StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_GATEWAY), &msg),
    };
    let uid = user.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let results = match crate::osu_api::map_ranks(&http, uid, mode, &maps).await {
        Ok(r) => r,
        Err((code, msg)) => return json_error(StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_GATEWAY), &msg),
    };
    let mut ranked = 0;
    {
        let conn = db_handle.lock().await;
        for (md5, rank, pass) in &results {
            if let Some(r) = rank {
                db::set_map_rank(&conn, &player, mode, md5, *r, pass.unwrap_or(0).max(*r));
                ranked += 1;
            }
        }
    }
    let next = if (maps.len() as i64) < count || offset + count >= 100 { serde_json::Value::Null } else { serde_json::json!(offset + count) };
    json_ok(&serde_json::json!({ "ok": true, "checked": maps.len(), "ranked": ranked, "next_offset": next }))
}

async fn handle_import_scores(state: SharedState, headers: &hyper::HeaderMap, body: &[u8]) -> Response {
    let now = now_secs();
    let token = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME))
        .map(|t| t.to_string());
    let session = {
        let st = state.read().await;
        token.and_then(|t| st.web_sessions.get(&t).filter(|s| s.expires > now).cloned())
    };
    let session = match session {
        Some(s) => s,
        None => return json_error(StatusCode::UNAUTHORIZED, "Sign in to import scores."),
    };
    if !can_write(Some(&session), headers) {
        return json_error(StatusCode::FORBIDDEN, "Refresh the page and try again.");
    }
    let player = match session.username {
        Some(ref u) if !u.is_empty() => u.clone(),
        _ => return json_error(StatusCode::UNAUTHORIZED, "Sign in to import scores."),
    };
    let payload: ImportScoresPayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, &format!("Invalid request body: {}", e)),
    };
    let query = payload.query.trim().to_string();
    if query.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "Enter an official osu! username or user ID.");
    }
    let modes: Vec<i32> = match payload.mode.as_str() {
        "osu" | "" => vec![0],
        "taiko" => vec![1],
        "fruits" | "catch" => vec![2],
        "mania" => vec![3],
        "all" => vec![0, 1, 2, 3],
        _ => return json_error(StatusCode::BAD_REQUEST, "Unknown game mode."),
    };
    let limit = if payload.limit >= 200 { 200 } else { 100 };

    let (http, db_handle) = {
        let st = state.read().await;
        (st.http.clone(), st.db.clone())
    };
    let (official_name, fetched) = match crate::osu_api::fetch(&http, &query, &player, &modes, limit).await {
        Ok(r) => r,
        Err((code, msg)) => {
            let status = StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_GATEWAY);
            return json_error(status, &msg);
        }
    };

    let mut summary = crate::osu_api::Summary::default();
    let mut fetched_report: Vec<serde_json::Value> = Vec::new();
    let mut stats_modes = 0usize;
    {
        let conn = db_handle.lock().await;
        for mf in &fetched {
            let (imp, ex, un) = crate::osu_api::store(&conn, &player, &mf.items);
            if let Some(official) = mf.official_pp {
                let bonus = (official - crate::osu_api::weighted_pp(&mf.items)).max(0.0);
                crate::osu_api::set_bonus(&conn, &player, mf.mode, bonus);
            }
            if let Some(st) = &mf.stats {
                let g = |k: &str| st.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
                let gc = |k: &str| st.get("grade_counts").and_then(|c| c.get(k)).and_then(|v| v.as_i64()).unwrap_or(0);
                if g("play_count") > 0 || g("total_score") > 0 {
                    let o = db::OfficialStats {
                        play_count: g("play_count"), play_time: g("play_time"), total_hits: g("total_hits"), max_combo: g("maximum_combo"),
                        replays_watched: g("replays_watched_by_others"), ranked_score: g("ranked_score"), total_score: g("total_score"),
                        grade_xh: gc("ssh"), grade_x: gc("ss"), grade_sh: gc("sh"), grade_s: gc("s"), grade_a: gc("a"),
                        imported_at: now_secs() as i64,
                        local_playtime_at_import: db::get_playtime(&conn, &player, mf.mode).unwrap_or(0),
                    };
                    if db::set_official_stats(&conn, &player, mf.mode, &o).is_ok() {
                        stats_modes += 1;
                    }
                }
            }
            let unreadable = mf.raw.saturating_sub(mf.items.len());
            summary.imported += imp;
            summary.skipped_existing += ex;
            summary.skipped_unusable += un + unreadable;
            let label = ["osu!", "Taiko", "Catch", "Mania"][mf.mode as usize % 4];
            summary.per_mode.push((label.to_string(), imp));
            let (fimp, _, _) = crate::osu_api::store(&conn, &player, &mf.firsts);
            let first_md5s: Vec<String> = mf.firsts.iter().filter(|c| c.score.md5.len() == 32).map(|c| c.score.md5.clone()).collect();
            let _ = db::replace_first_places(&conn, &player, mf.mode, &first_md5s);
            summary.imported += fimp;
            fetched_report.push(serde_json::json!({ "mode": label, "returned": mf.raw, "imported": imp, "official_pp": mf.official_pp, "firsts": first_md5s.len() }));
        }
    }

    let (ranked, playcount) = {
        let conn = db_handle.lock().await;
        (
            db::get_ranked_scores(&conn, &player).unwrap_or_default(),
            db::get_playcount(&conn, &player).unwrap_or(0),
        )
    };
    let (pp, acc) = {
        let mut st = state.write().await;
        let filter_mod = st.mode;
        match st.player.as_mut() {
            Some(p) if p.name == player => {
                p.calculate_stats(&ranked, filter_mod, playcount);
                p.enqueue_stats();
                (p.pp, p.acc)
            }
            _ => {
                let mut dummy = crate::types::player::Player::new(player.clone());
                dummy.calculate_stats(&ranked, filter_mod, playcount);
                (dummy.pp, dummy.acc)
            }
        }
    };
    {
        let conn = db_handle.lock().await;
        let _ = db::update_profile_stats(&conn, &player, pp as f64, acc);
    }

    json_ok(&serde_json::json!({
        "ok": true,
        "official_name": official_name,
        "imported": summary.imported,
        "skipped_existing": summary.skipped_existing,
        "skipped_unusable": summary.skipped_unusable,
        "per_mode": summary.per_mode,
        "fetched": fetched_report,
        "stats_imported": stats_modes,
    }))
}

async fn handle_import_official(
    state: SharedState,
    headers: &hyper::HeaderMap,
    body: &[u8],
) -> Response {
    let now = now_secs();
    let cookie_val = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| extract_cookie(c, SESSION_COOKIE_NAME));

    let token = match cookie_val {
        Some(t) => t.to_string(),
        None => return json_error(StatusCode::UNAUTHORIZED, "Sign in to import profile details."),
    };

    let state_read = state.read().await;
    let session = match state_read.web_sessions.get(&token) {
        Some(s) if s.expires > now => s.clone(),
        _ => return json_error(StatusCode::UNAUTHORIZED, "Sign in to import profile details."),
    };
    drop(state_read);

    if !can_write(Some(&session), headers) {
        return json_error(StatusCode::FORBIDDEN, "Refresh the page and try again.");
    }

    let current_user = match session.username {
        Some(ref u) if !u.is_empty() => u.clone(),
        _ => return json_error(StatusCode::UNAUTHORIZED, "Sign in to import profile details."),
    };

    let payload: ImportOfficialPayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, &format!("Invalid request body: {}", e)),
    };

    let query = payload.query.trim();
    if query.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "Please provide an official osu! username or user ID.");
    }

    let http_client = {
        let s = state.read().await;
        s.http.clone()
    };

    let parsed = match crate::osu_api::user(&http_client, query).await {
        Ok(v) => match parse_official_user_value(&v) {
            Ok(p) => p,
            Err(err) => return json_error(StatusCode::UNPROCESSABLE_ENTITY, &err),
        },
        Err((404, msg)) => return json_error(StatusCode::NOT_FOUND, &msg),
        Err(_) => {
            let encoded_query = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC).to_string();
            let url = format!("https://osu.ppy.sh/users/{}", encoded_query);
            let res = match http_client
                .get(&url)
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                .header("Accept-Language", "en-US,en;q=0.9")
                .timeout(std::time::Duration::from_secs(12))
                .send_traced()
                .await
            {
                Ok(r) => r,
                Err(e) => return json_error(StatusCode::BAD_GATEWAY, &format!("Failed to reach osu.ppy.sh: {}", e)),
            };
            if res.status() == StatusCode::NOT_FOUND {
                return json_error(StatusCode::NOT_FOUND, &format!("User '{}' not found on osu.ppy.sh.", query));
            }
            if !res.status().is_success() {
                return json_error(StatusCode::BAD_GATEWAY, &format!("osu.ppy.sh returned status {}.", res.status()));
            }
            let html = match res.text().await {
                Ok(t) => t,
                Err(e) => return json_error(StatusCode::BAD_GATEWAY, &format!("Failed to read response from osu.ppy.sh: {}", e)),
            };
            match parse_official_userpage_html(&html) {
                Ok(p) => p,
                Err(err) => return json_error(StatusCode::UNPROCESSABLE_ENTITY, &err),
            }
        }
    };

    let mut current_details = {
        let s = state.read().await;
        let conn = s.db.lock().await;
        db::get_profile_details(&conn, &current_user).unwrap_or_default()
    };

    let mut details_updated = false;
    if payload.import_bio {
        if let Some(bio) = parsed.raw_bio {
            current_details.about = safe_truncate(&bio, 5000).to_string();
            details_updated = true;
        }
    }

    if payload.import_details {
        if let Some(ref loc) = parsed.location {
            current_details.location = safe_truncate(loc, 100).to_string();
            details_updated = true;
        }
        if let Some(ref cc) = parsed.country_code {
            if crate::utils::country_code_to_byte(cc) != 0 {
                current_details.country = cc.clone();
                details_updated = true;
            }
        }
        if !parsed.playstyle.is_empty() {
            current_details.devices = parsed.playstyle.clone();
            details_updated = true;
        }
        for (src, dst, max) in [
            (&parsed.interests, &mut current_details.interests, 100),
            (&parsed.occupation, &mut current_details.occupation, 100),
            (&parsed.twitter, &mut current_details.twitter, 50),
            (&parsed.website, &mut current_details.website, 200),
            (&parsed.discord, &mut current_details.discord, 50),
        ] {
            if let Some(v) = src {
                *dst = safe_truncate(v, max).to_string();
                details_updated = true;
            }
        }
        if let Some(ref jm) = parsed.join_month {
            current_details.join_date = jm.clone();
            details_updated = true;
        }
    }

    let mut avatar_data_url: Option<String> = None;
    let mut avatar_bytes_opt: Option<(Vec<u8>, &'static str)> = None;

    if payload.import_avatar {
        if let Some(ref av_url) = parsed.avatar_url {
            let final_av_url = if av_url.starts_with("//") {
                format!("https:{}", av_url)
            } else if av_url.starts_with('/') {
                format!("https://osu.ppy.sh{}", av_url)
            } else {
                av_url.clone()
            };

            if !final_av_url.contains("avatar-guest") {
                if let Ok(av_res) = http_client
                    .get(&final_av_url)
                    .timeout(std::time::Duration::from_secs(10))
                    .send_traced()
                    .await
                {
                    if av_res.status().is_success() {
                        if let Ok(bytes) = av_res.bytes().await {
                            if let Ok(ext) = validate_image_format_and_dimensions(&bytes) {
                                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                let data_url = format!("data:image/{};base64,{}", ext, b64);
                                avatar_data_url = Some(data_url);
                                avatar_bytes_opt = Some((bytes.to_vec(), ext));
                            }
                        }
                    }
                }
            }
        }
    }

    let mut cover_bytes_opt: Option<(Vec<u8>, &'static str)> = None;
    let mut cover_data_url: Option<String> = None;
    if payload.import_cover {
        if let Some(ref cv_url) = parsed.cover_url {
            let final_cv_url = if cv_url.starts_with("//") {
                format!("https:{}", cv_url)
            } else if cv_url.starts_with('/') {
                format!("https://osu.ppy.sh{}", cv_url)
            } else {
                cv_url.clone()
            };
            if let Ok(cv_res) = http_client
                .get(&final_cv_url)
                .timeout(std::time::Duration::from_secs(15))
                .send_traced()
                .await
            {
                if cv_res.status().is_success() {
                    if let Ok(bytes) = cv_res.bytes().await {
                        if bytes.len() <= 6 * 1024 * 1024 {
                            if let Ok(ext) = validate_image_format_and_dimensions(&bytes) {
                                if !payload.apply {
                                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                    cover_data_url = Some(format!("data:image/{};base64,{}", ext, b64));
                                }
                                cover_bytes_opt = Some((bytes.to_vec(), ext));
                            }
                        }
                    }
                }
            }
        }
    }

    if !parsed.rank_history.is_empty() {
        let today = chrono::Utc::now().date_naive();
        let rows = official_rank_rows(today, &parsed.rank_history);
        let s_read = state.read().await;
        let conn = s_read.db.lock().await;
        let _ = db::import_rank_history(&conn, &current_user, 0, &rows);
    }

    let mut avatar_updated = false;
    let mut cover_updated = false;
    if payload.apply {
        let s_read = state.read().await;
        let conn = s_read.db.lock().await;

        if details_updated {
            if let Err(e) = db::save_profile_details(&conn, &current_user, &current_details) {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to save profile details: {}", e),
                );
            }
        }

        let mut old_avatar_to_delete: Option<PathBuf> = None;
        if let Some((img_bytes, ext)) = avatar_bytes_opt {
            let avatar_dir = Path::new(".data").join("avatars");
            let _ = tokio::fs::create_dir_all(&avatar_dir).await;
            let token_hex = generate_hex_token(16);
            let filename = format!("{}.{}", token_hex, ext);
            let file_path = avatar_dir.join(&filename);

            if let Ok(()) = tokio::fs::write(&file_path, &img_bytes).await {
                if let Ok(Some(old_url)) = db::get_avatar(&conn, &current_user) {
                    if let Some(p) = utils::is_path(&old_url) {
                        old_avatar_to_delete = Some(p);
                    }
                }
                let stored_path = file_path.to_string_lossy().to_string();
                let _ = db::set_avatar(&conn, &current_user, &stored_path);
                avatar_updated = true;
            }
        }

        let mut old_cover_to_delete: Option<PathBuf> = None;
        if let Some((img_bytes, ext)) = cover_bytes_opt {
            let cover_dir = Path::new(".data").join("covers");
            let _ = tokio::fs::create_dir_all(&cover_dir).await;
            let filename = format!("{}.{}", generate_hex_token(16), ext);
            let file_path = cover_dir.join(&filename);
            if let Ok(()) = tokio::fs::write(&file_path, &img_bytes).await {
                if let Ok(Some(old_url)) = db::get_cover(&conn, &current_user) {
                    if let Some(p) = utils::is_path(&old_url) {
                        old_cover_to_delete = Some(p);
                    }
                }
                let stored_path = file_path.to_string_lossy().to_string();
                let _ = db::set_cover(&conn, &current_user, &stored_path);
                cover_updated = true;
            }
        }

        let country_byte = if !current_details.country.is_empty() {
            crate::utils::country_code_to_byte(&current_details.country)
        } else {
            0
        };
        if country_byte != 0 {
            let _ = conn.execute(
                "UPDATE profiles SET country = ?1 WHERE name = ?2",
                rusqlite::params![country_byte, current_user],
            );
        }

        drop(conn);
        drop(s_read);

        if let Some(old_path) = old_avatar_to_delete {
            let p_str = old_path.to_string_lossy();
            if p_str.contains(".data") && p_str.contains("avatars") {
                let _ = tokio::fs::remove_file(old_path).await;
            }
        }
        if let Some(old_path) = old_cover_to_delete {
            let p_str = old_path.to_string_lossy();
            if p_str.contains(".data") && p_str.contains("covers") {
                let _ = tokio::fs::remove_file(old_path).await;
            }
        }

        let mut s_write = state.write().await;
        if let Some(ref mut player) = s_write.player {
            if player.name == current_user {
                if country_byte != 0 {
                    player.country = country_byte;
                }
                player.queue.extend_from_slice(&crate::packets::user_presence(player));
                player.queue.extend_from_slice(&crate::packets::user_stats(player));
            }
        }
        drop(s_write);
    }

    let message = if payload.apply {
        format!(
            "Successfully imported and applied official profile for '{}' (#{id})",
            parsed.username,
            id = parsed.id
        )
    } else {
        format!(
            "Successfully fetched official profile for '{}' (#{id})",
            parsed.username,
            id = parsed.id
        )
    };

    json_ok(&ImportOfficialResponse {
        ok: true,
        official_username: parsed.username,
        official_id: parsed.id,
        details: current_details,
        avatar_data_url,
        avatar_updated,
        cover_updated,
        cover_data_url,
        details_updated,
        message,
    })
}

#[cfg(test)]
mod tests {
    fn sb(id: i64, md5: &str, pp: f64, time: i64) -> db::ScoreWithBeatmap {
        db::ScoreWithBeatmap {
            id, mode: 0, md5: md5.into(), n300: 1, n100: 0, n50: 0, ngeki: 0, nkatu: 0, nmiss: 0,
            score: 1, max_combo: 1, perfect: true, mods: 0, time, acc: 100.0, pp, replay_md5: None,
            mods_str: None, bmap_status: "ranked".into(), title: None, artist: None, version: None,
            creator: None, beatmap_id: None, beatmapset_id: None,
        }
    }

    #[test]
    fn era_rank_history_translates_pp_onto_the_chosen_ladder_day() {
        crate::ladder::init();
        let now = chrono::Utc::now().timestamp();
        let a = sb(1, "m1", 3000.0, now - 40 * 86400);
        let b = sb(2, "m2", 3000.0, now - 5 * 86400);
        let all = vec![&a, &b];
        let h = era_rank_history(&all, "2019-01-31", 6000.0, Some(1234));
        assert!(!h.is_empty());
        assert_eq!(h.last().unwrap().1, 1234);
        let first = h.first().unwrap().1;
        let before_second = h.iter().find(|(d, _)| *d == (chrono::Utc::now().date_naive() - chrono::Duration::days(10)).format("%Y-%m-%d").to_string()).unwrap().1;
        let after_second = h.iter().rev().nth(1).unwrap().1;
        assert_eq!(first, before_second);
        assert!(after_second < before_second, "more pp must mean a better (lower) rank: {after_second} vs {before_second}");
    }

    #[test]
    fn weighted_total_uses_best_per_map_cutoff_and_bonus() {
        let a = sb(1, "m1", 100.0, 1000);
        let b = sb(2, "m1", 80.0, 2000);
        let c = sb(3, "m2", 50.0, 3000);
        let all = vec![&a, &b, &c];
        let t = weighted_total(&all, |s| s.pp, Some(10.0), i64::MAX);
        assert!((t - (100.0 + 47.5 + 10.0)).abs() < 1e-9, "{t}");
        let t = weighted_total(&all, |s| s.pp, Some(0.0), 2500);
        assert!((t - 100.0).abs() < 1e-9, "{t}");
        assert_eq!(weighted_total(&all, |s| s.pp, None, 10), 0.0);
        let t = weighted_total(&all, |s| s.pp, None, i64::MAX);
        assert!(t > 147.5 && t < 147.5 + 1.0, "{t}");
    }

    use super::*;
    use crate::state::AppState;
    use hyper::header::HeaderValue;
    use hyper::HeaderMap;

    #[test]
    fn test_era_rank_lookup() {
        assert_eq!(era_rank("2017", 1_000_000.0), Some(1));
        assert_eq!(era_rank("2017", 0.0), None);
        assert_eq!(era_rank("nope", 5000.0), None);
        assert_eq!(era_date("2019").as_deref(), Some("2019-02-25"));
        let r17 = era_rank("2017", 6000.0).unwrap();
        let r19 = era_rank("2019", 6000.0).unwrap();
        assert!(r17 > 0 && r17 < r19, "{r17} vs {r19}");
        assert!(era_rank("2019", 6500.0).unwrap() <= era_rank("2019", 6000.0).unwrap());
    }

    #[test]
    fn test_extract_cookie() {
        let header = "other=123; los_session=my_secure_token; theme=dark";
        assert_eq!(extract_cookie(header, "los_session"), Some("my_secure_token"));
        assert_eq!(extract_cookie(header, "other"), Some("123"));
        assert_eq!(extract_cookie(header, "theme"), Some("dark"));
        assert_eq!(extract_cookie(header, "missing"), None);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"token123", b"token123"));
        assert!(!constant_time_eq(b"token123", b"token124"));
        assert!(!constant_time_eq(b"token123", b"token12"));
    }

    #[test]
    fn test_validate_username() {
        assert_eq!(validate_username("Alice").unwrap(), "Alice");
        assert_eq!(validate_username("  Bob_123  ").unwrap(), "Bob_123");
        assert!(validate_username("").is_err());
        assert!(validate_username("a".repeat(65).as_str()).is_err());
        assert!(validate_username("user/name").is_err());
        assert!(validate_username("user\\name").is_err());
        assert!(validate_username("user\x00name").is_err());
    }

    #[test]
    fn test_can_write_verification() {
        let session = WebSession {
            csrf_token: "csrf_secret_123".to_string(),
            username: Some("Alice".to_string()),
            expires: 9999999999,
        };

        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("127.0.0.1:5000"));
        headers.insert("origin", HeaderValue::from_static("http://127.0.0.1:5000"));
        headers.insert("x-csrf-token", HeaderValue::from_static("csrf_secret_123"));

        assert!(can_write(Some(&session), &headers));

        let mut bad_csrf = headers.clone();
        bad_csrf.insert("x-csrf-token", HeaderValue::from_static("wrong_token"));
        assert!(!can_write(Some(&session), &bad_csrf));

        let mut bad_origin = headers.clone();
        bad_origin.insert("origin", HeaderValue::from_static("https://evil.com"));
        assert!(!can_write(Some(&session), &bad_origin));

        assert!(!can_write(None, &headers));
    }

    #[tokio::test]
    async fn test_session_bootstrap_login_and_logout_flow() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        let app_state = AppState::new(conn, crate::types::config::Config::default());
        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(app_state));

        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("127.0.0.1:5000"));
        let resp = handle_session(shared_state.clone(), &headers).await;
        assert_eq!(resp.status, StatusCode::OK);

        let cookie_header = resp.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("set-cookie")).expect("Set-Cookie header missing");
        let cookie_val = extract_cookie(&cookie_header.1, SESSION_COOKIE_NAME).expect("Session token in cookie");

        let body: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
        let csrf = body["csrf"].as_str().unwrap().to_string();
        assert!(body["user"].is_null());

        headers.insert("cookie", HeaderValue::from_str(&format!("{}={}", SESSION_COOKIE_NAME, cookie_val)).unwrap());
        let resp2 = handle_session(shared_state.clone(), &headers).await;
        assert_eq!(resp2.status, StatusCode::OK);
        let body2: serde_json::Value = serde_json::from_slice(&resp2.body).unwrap();
        assert_eq!(body2["csrf"].as_str().unwrap(), csrf);
        assert!(!resp2.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("set-cookie")));

        headers.insert("origin", HeaderValue::from_static("http://127.0.0.1:5000"));
        let bad_login = handle_login(shared_state.clone(), &headers, b"{\"username\":\"Alice\"}").await;
        assert_eq!(bad_login.status, StatusCode::FORBIDDEN);

        headers.insert("x-csrf-token", HeaderValue::from_str(&csrf).unwrap());
        let login_resp = handle_login(shared_state.clone(), &headers, b"{\"username\":\"Alice\"}").await;
        assert_eq!(login_resp.status, StatusCode::OK);
        let login_body: serde_json::Value = serde_json::from_slice(&login_resp.body).unwrap();
        assert_eq!(login_body["name"], "Alice");

        let resp3 = handle_session(shared_state.clone(), &headers).await;
        let body3: serde_json::Value = serde_json::from_slice(&resp3.body).unwrap();
        assert_eq!(body3["user"], "Alice");
        assert!(body3["profiles"].as_array().unwrap().iter().any(|p| p == "Alice"));

        let logout_resp = handle_logout(shared_state.clone(), &headers).await;
        assert_eq!(logout_resp.status, StatusCode::OK);

        let resp4 = handle_session(shared_state.clone(), &headers).await;
        let body4: serde_json::Value = serde_json::from_slice(&resp4.body).unwrap();
        assert!(body4["user"].is_null());
    }

    #[tokio::test]
    async fn test_session_expiration_pruning() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        let mut app_state = AppState::new(conn, crate::types::config::Config::default());

        let expired_token = "expired_token_123";
        app_state.web_sessions.insert(expired_token.to_string(), WebSession {
            csrf_token: "old_csrf".to_string(),
            username: Some("OldUser".to_string()),
            expires: 100,
        });

        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(app_state));

        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("127.0.0.1:5000"));
        headers.insert("cookie", HeaderValue::from_str(&format!("{}={}", SESSION_COOKIE_NAME, expired_token)).unwrap());

        let resp = handle_session(shared_state.clone(), &headers).await;
        assert_eq!(resp.status, StatusCode::OK);

        let cookie_header = resp.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("set-cookie")).expect("New cookie header");
        let new_token = extract_cookie(&cookie_header.1, SESSION_COOKIE_NAME).unwrap();
        assert_ne!(new_token, expired_token);

        let state_guard = shared_state.read().await;
        assert!(!state_guard.web_sessions.contains_key(expired_token));
    }

    #[tokio::test]
    async fn test_handle_profile_not_found_and_invalid_mode() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        let app_state = AppState::new(conn, crate::types::config::Config::default());
        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(app_state));

        let params_empty = HashMap::new();
        let resp_empty = handle_profile(shared_state.clone(), &params_empty).await;
        assert_eq!(resp_empty.status, StatusCode::BAD_REQUEST);

        let mut params = HashMap::new();
        params.insert("name".to_string(), "GhostUser".to_string());
        let resp_nf = handle_profile(shared_state.clone(), &params).await;
        assert_eq!(resp_nf.status, StatusCode::NOT_FOUND);

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        crate::db::ensure_profile(&conn, "Alice").unwrap();
        let app_state = AppState::new(conn, crate::types::config::Config::default());
        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(app_state));

        params.insert("name".to_string(), "Alice".to_string());
        params.insert("mode".to_string(), "invalid_mode".to_string());
        let resp_bad_mode = handle_profile(shared_state.clone(), &params).await;
        assert_eq!(resp_bad_mode.status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_profile_data_and_mode_filtering() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        crate::db::ensure_profile(&conn, "Alice").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.file_md5 = "map_1".to_string();
        bmap.title = "Song One".to_string();
        bmap.artist = "Artist A".to_string();
        bmap.version = "Hard".to_string();
        bmap.creator = "Mapper A".to_string();
        bmap.beatmap_id = 1001;
        bmap.beatmapset_id = 2001;
        crate::db::insert_beatmap(&conn, &bmap).unwrap();

        let mut bmap2 = crate::types::beatmap::Beatmap::blank();
        bmap2.file_md5 = "map_2".to_string();
        bmap2.title = "Song Two".to_string();
        bmap2.artist = "Artist B".to_string();
        bmap2.version = "Insane".to_string();
        bmap2.creator = "Mapper B".to_string();
        bmap2.beatmap_id = 1002;
        bmap2.beatmapset_id = 2002;
        crate::db::insert_beatmap(&conn, &bmap2).unwrap();

        let s1 = crate::types::score::Score {
            mode: 0,
            md5: "map_1".to_string(),
            name: "Alice".to_string(),
            n300: 300,
            n100: 0,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss: 0,
            score: 1000000,
            max_combo: 500,
            perfect: true,
            mods: 0,
            time: 1700000000,
            acc: Some(100.0),
            pp: Some(300.0),
            replay_md5: None,
            replay_frames: None,
            mods_str: None,
            scoreid: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        crate::db::insert_score(&conn, &s1, "ranked").unwrap();

        let s2 = crate::types::score::Score {
            mode: 0,
            md5: "map_2".to_string(),
            name: "Alice".to_string(),
            n300: 300,
            n100: 0,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss: 0,
            score: 1050000,
            max_combo: 500,
            perfect: true,
            mods: 8,
            time: 1700086400,
            acc: Some(100.0),
            pp: Some(200.0),
            replay_md5: None,
            replay_frames: None,
            mods_str: Some("HD".to_string()),
            scoreid: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        crate::db::insert_score(&conn, &s2, "ranked").unwrap();

        let s_rx = crate::types::score::Score {
            mode: 0,
            md5: "map_1".to_string(),
            name: "Alice".to_string(),
            n300: 290,
            n100: 10,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss: 0,
            score: 950000,
            max_combo: 480,
            perfect: false,
            mods: 128,
            time: 1700172800,
            acc: Some(98.5),
            pp: Some(400.0),
            replay_md5: None,
            replay_frames: None,
            mods_str: Some("RX".to_string()),
            scoreid: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        crate::db::insert_score(&conn, &s_rx, "ranked").unwrap();
        crate::db::add_user_medal(&conn, "Alice", 1, 1700000000).unwrap();

        let app_state = AppState::new(conn, crate::types::config::Config::default());
        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(app_state));

        let mut params_vn = HashMap::new();
        params_vn.insert("name".to_string(), "Alice".to_string());
        params_vn.insert("mode".to_string(), "vn".to_string());
        let resp_vn = handle_profile(shared_state.clone(), &params_vn).await;
        assert_eq!(resp_vn.status, StatusCode::OK);

        let data: ProfileResponse = serde_json::from_slice(&resp_vn.body).unwrap();
        assert_eq!(data.name, "Alice");
        assert_eq!(data.mode, "vn");
        assert_eq!(data.playcount, 2);
        assert!(data.pp > 0);
        assert_eq!(data.top.len(), 2);
        assert_eq!(data.top[0].beatmap.title, "Song One");
        assert_eq!(data.top[0].grade, "X");
        assert_eq!(data.top[1].beatmap.title, "Song Two");
        assert_eq!(data.top[1].grade, "XH");
        assert_eq!(*data.grades.get("X").unwrap(), 1);
        assert_eq!(*data.grades.get("XH").unwrap(), 1);
        assert_eq!(data.ranked_score, 2050000);
        assert_eq!(data.total_score, 2050000);
        assert_eq!(data.most_played.len(), 2);
        assert_eq!(data.medals.len(), 1);
        assert_eq!(data.medals[0].id, 1);
        assert_eq!(data.medals[0].name, "500 Combo");
        assert_eq!(data.medals[0].category, "Skill & Dedication");

        let mut params_rx = HashMap::new();
        params_rx.insert("name".to_string(), "Alice".to_string());
        params_rx.insert("mode".to_string(), "rx".to_string());
        let resp_rx = handle_profile(shared_state.clone(), &params_rx).await;
        assert_eq!(resp_rx.status, StatusCode::OK);

        let data_rx: ProfileResponse = serde_json::from_slice(&resp_rx.body).unwrap();
        assert_eq!(data_rx.playcount, 1);
        assert_eq!(data_rx.top.len(), 1);
        assert_eq!(data_rx.top[0].mods, "RX");

        let mut params_osu = HashMap::new();
        params_osu.insert("name".to_string(), "Alice".to_string());
        params_osu.insert("mode".to_string(), "osu".to_string());
        let resp_osu = handle_profile(shared_state.clone(), &params_osu).await;
        assert_eq!(resp_osu.status, StatusCode::OK);
        let data_osu: ProfileResponse = serde_json::from_slice(&resp_osu.body).unwrap();
        assert_eq!(data_osu.mode, "osu");
        assert_eq!(data_osu.playcount, 2);

        let mut params_taiko = HashMap::new();
        params_taiko.insert("name".to_string(), "Alice".to_string());
        params_taiko.insert("mode".to_string(), "taiko".to_string());
        let resp_taiko = handle_profile(shared_state.clone(), &params_taiko).await;
        assert_eq!(resp_taiko.status, StatusCode::OK);
        let data_taiko: ProfileResponse = serde_json::from_slice(&resp_taiko.body).unwrap();
        assert_eq!(data_taiko.mode, "taiko");
        assert_eq!(data_taiko.playcount, 0);
        assert_eq!(data_taiko.top.len(), 0);
    }

    #[test]
    fn test_image_validation_formats() {
        let valid_png = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
            0x00, 0x00, 0x00, 0x0D,
            b'I', b'H', b'D', b'R',
            0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00,
        ];
        assert_eq!(validate_image_format_and_dimensions(&valid_png).unwrap(), "png");

        let oversized_png = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
            0x00, 0x00, 0x00, 0x0D,
            b'I', b'H', b'D', b'R',
            0x00, 0x00, 0x10, 0x01,
            0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00,
        ];
        assert!(validate_image_format_and_dimensions(&oversized_png).is_err());

        let mut valid_gif = vec![b'G', b'I', b'F', b'8', b'9', b'a'];
        valid_gif.extend_from_slice(&100u16.to_le_bytes());
        valid_gif.extend_from_slice(&200u16.to_le_bytes());
        valid_gif.push(0x80);
        assert_eq!(validate_image_format_and_dimensions(&valid_gif).unwrap(), "gif");

        assert!(validate_image_format_and_dimensions(b"NOT_AN_IMAGE").is_err());
    }

    #[test]
    fn test_parse_and_validate_avatar_data_url() {
        let valid_png = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
            0x00, 0x00, 0x00, 0x0D,
            b'I', b'H', b'D', b'R',
            0x00, 0x00, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x02,
            0x08, 0x06, 0x00, 0x00, 0x00,
        ];
        let b64 = base64::engine::general_purpose::STANDARD.encode(&valid_png);
        let data_url = format!("data:image/png;base64,{}", b64);

        let (ext, bytes) = parse_and_validate_avatar(&data_url).unwrap();
        assert_eq!(ext, "png");
        assert_eq!(bytes, valid_png);

        assert!(parse_and_validate_avatar("data:image/png;base64,???bad_base64???").is_err());
    }

    #[tokio::test]
    async fn test_avatar_route_fallback_and_custom() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        crate::db::ensure_profile(&conn, "Bob").unwrap();

        let app_state = AppState::new(conn, crate::types::config::Config::default());
        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(app_state));

        let mut params = HashMap::new();
        params.insert("name".to_string(), "Bob".to_string());
        let resp = handle_avatar(shared_state.clone(), &params).await;
        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.header("content-type").unwrap(), "image/png");

        let temp_avatar = std::path::Path::new(".data").join("avatars").join("test_avatar.png");
        let _ = std::fs::create_dir_all(temp_avatar.parent().unwrap());
        std::fs::write(&temp_avatar, b"\x89PNG\r\n\x1a\ntest").unwrap();

        {
            let s = shared_state.read().await;
            let conn = s.db.lock().await;
            crate::db::set_avatar(&conn, "Bob", temp_avatar.to_str().unwrap()).unwrap();
        }

        let resp_custom = handle_avatar(shared_state.clone(), &params).await;
        assert_eq!(resp_custom.status, StatusCode::OK);
        assert_eq!(resp_custom.body, b"\x89PNG\r\n\x1a\ntest");

        let _ = std::fs::remove_file(&temp_avatar);
    }

    #[tokio::test]
    async fn test_settings_route_rename_details_and_avatar() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        crate::db::ensure_profile(&conn, "OldName").unwrap();

        let mut app_state = AppState::new(conn, crate::types::config::Config::default());
        let mut player = crate::types::player::Player::new("OldName".to_string());
        player.country = 0;
        app_state.player = Some(player);

        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(app_state));

        let headers_session = HeaderMap::new();
        let s_resp = handle_session(shared_state.clone(), &headers_session).await;
        let s_data: SessionResponse = serde_json::from_slice(&s_resp.body).unwrap();
        let set_cookie = s_resp.header("set-cookie").unwrap();
        let session_token = extract_cookie(set_cookie, SESSION_COOKIE_NAME).unwrap().to_string();

        let mut login_headers = HeaderMap::new();
        login_headers.insert("cookie", HeaderValue::from_str(&format!("los_session={}", session_token)).unwrap());
        login_headers.insert("origin", HeaderValue::from_static("http://127.0.0.1:5000"));
        login_headers.insert("host", HeaderValue::from_static("127.0.0.1:5000"));
        login_headers.insert("x-csrf-token", HeaderValue::from_str(&s_data.csrf).unwrap());
        let login_body = serde_json::json!({ "username": "OldName" }).to_string();
        let l_resp = handle_login(shared_state.clone(), &login_headers, login_body.as_bytes()).await;
        assert_eq!(l_resp.status, StatusCode::OK);

        let valid_png = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
            0x00, 0x00, 0x00, 0x0D,
            b'I', b'H', b'D', b'R',
            0x00, 0x00, 0x00, 0x10,
            0x00, 0x00, 0x00, 0x10,
            0x08, 0x06, 0x00, 0x00, 0x00,
        ];
        let avatar_b64 = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&valid_png));

        let settings_body = serde_json::json!({
            "username": "NewName",
            "reset_avatar": false,
            "avatar": avatar_b64,
            "details": {
                "country": "DE",
                "location": "Berlin",
                "about": "New bio",
                "devices": ["Tablet", "Keyboard"],
                "section_order": ["me", "top-ranks", "historical", "beatmaps", "medals", "recent-activity"]
            }
        }).to_string();

        let settings_resp = handle_settings(shared_state.clone(), &login_headers, settings_body.as_bytes()).await;
        assert_eq!(settings_resp.status, StatusCode::OK);
        let ret: LoginResponse = serde_json::from_slice(&settings_resp.body).unwrap();
        assert_eq!(ret.name, "NewName");

        {
            let s = shared_state.read().await;
            let conn = s.db.lock().await;
            assert!(!crate::db::profile_exists(&conn, "OldName").unwrap());
            assert!(crate::db::profile_exists(&conn, "NewName").unwrap());
            assert_eq!(crate::db::get_profile_country(&conn, "NewName").unwrap(), 56);
            let det = crate::db::get_profile_details(&conn, "NewName").unwrap();
            assert_eq!(det.location, "Berlin");
            assert_eq!(det.about, "New bio");
            assert_eq!(det.country, "DE");

            let av = crate::db::get_avatar(&conn, "NewName").unwrap();
            assert!(av.is_some());
            let av_path = av.unwrap();
            assert!(std::path::Path::new(&av_path).exists());
            let _ = std::fs::remove_file(&av_path);
        }

        {
            let s = shared_state.read().await;
            let p = s.player.as_ref().unwrap();
            assert_eq!(p.name, "NewName");
            assert_eq!(p.country, 56);
            assert!(!p.queue.is_empty(), "Packets must be queued for in-game client");
        }
    }

    #[tokio::test]
    async fn test_delete_score_route_and_stats_recalculation() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        crate::db::ensure_profile(&conn, "ScoreUser").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.file_md5 = "map_del_1".to_string();
        bmap.title = "Delete Me".to_string();
        bmap.artist = "Artist".to_string();
        bmap.version = "Insane".to_string();
        bmap.beatmap_id = 9991;
        bmap.beatmapset_id = 9992;
        crate::db::insert_beatmap(&conn, &bmap).unwrap();

        let s1 = crate::types::score::Score {
            mode: 0,
            md5: "map_del_1".to_string(),
            name: "ScoreUser".to_string(),
            n300: 300,
            n100: 0,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss: 0,
            score: 1000000,
            max_combo: 500,
            perfect: true,
            mods: 0,
            time: 1700000000,
            acc: Some(100.0),
            pp: Some(250.0),
            replay_md5: None,
            replay_frames: None,
            mods_str: None,
            scoreid: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        let s1_id = crate::db::insert_score(&conn, &s1, "ranked").unwrap();
        crate::db::increment_playcount(&conn, "ScoreUser").unwrap();

        let mut app_state = AppState::new(conn, crate::types::config::Config::default());
        let player = crate::types::player::Player::new("ScoreUser".to_string());
        app_state.player = Some(player);

        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(app_state));

        let s_resp = handle_session(shared_state.clone(), &HeaderMap::new()).await;
        let s_data: SessionResponse = serde_json::from_slice(&s_resp.body).unwrap();
        let set_cookie = s_resp.header("set-cookie").unwrap();
        let session_token = extract_cookie(set_cookie, SESSION_COOKIE_NAME).unwrap().to_string();

        let mut login_headers = HeaderMap::new();
        login_headers.insert("cookie", HeaderValue::from_str(&format!("los_session={}", session_token)).unwrap());
        login_headers.insert("origin", HeaderValue::from_static("http://127.0.0.1:5000"));
        login_headers.insert("host", HeaderValue::from_static("127.0.0.1:5000"));
        login_headers.insert("x-csrf-token", HeaderValue::from_str(&s_data.csrf).unwrap());
        let login_body = serde_json::json!({ "username": "ScoreUser" }).to_string();
        let l_resp = handle_login(shared_state.clone(), &login_headers, login_body.as_bytes()).await;
        assert_eq!(l_resp.status, StatusCode::OK);

        let del_body_nf = serde_json::json!({ "id": 999999 }).to_string();
        let resp_nf = handle_delete_score(shared_state.clone(), &login_headers, del_body_nf.as_bytes()).await;
        assert_eq!(resp_nf.status, StatusCode::NOT_FOUND);

        let del_body = serde_json::json!({ "id": s1_id.to_string() }).to_string();
        let resp_del = handle_delete_score(shared_state.clone(), &login_headers, del_body.as_bytes()).await;
        assert_eq!(resp_del.status, StatusCode::OK);

        {
            let s = shared_state.read().await;
            let conn = s.db.lock().await;
            let remaining = crate::db::get_player_scores_with_beatmaps(&conn, "ScoreUser").unwrap();
            assert!(remaining.is_empty(), "Score should be deleted from scores table");

            let archived_count: i32 = conn.query_row(
                "SELECT COUNT(*) FROM deleted_scores WHERE player_name = 'ScoreUser' AND original_id = ?1",
                rusqlite::params![s1_id],
                |r| r.get(0),
            ).unwrap();
            assert_eq!(archived_count, 1, "Score should be archived in deleted_scores table");
        }

        {
            let s = shared_state.read().await;
            let p = s.player.as_ref().unwrap();
            assert!(!p.queue.is_empty(), "Packets must be queued for in-game stats update");
        }
    }

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(decode_html_entities(""), "");
        assert_eq!(
            decode_html_entities("&quot;hello&quot; &amp; &lt;world&gt; &#039;test&#39; &apos;123&apos;"),
            "\"hello\" & <world> 'test' '123'"
        );
        assert_eq!(
            decode_html_entities("normal text without entities"),
            "normal text without entities"
        );
        assert_eq!(decode_html_entities("trailing & and &unknown; entity"), "trailing & and &unknown; entity");
    }

    #[test]
    fn test_safe_truncate() {
        let ascii = "hello world";
        assert_eq!(safe_truncate(ascii, 5), "hello");
        assert_eq!(safe_truncate(ascii, 20), "hello world");

        let unicode = "こんにちは世界";
        assert_eq!(safe_truncate(unicode, 6), "こん");
        assert_eq!(safe_truncate(unicode, 5), "こ");
    }

    #[test]
    fn official_rank_history_is_dated_back_from_today_and_imported_without_overwriting() {
        let user = serde_json::json!({"id": 1, "username": "peppy", "rank_history": {"mode": "osu", "data": [300, 0, 250, 200]}});
        let parsed = parse_official_user_value(&user).unwrap();
        assert_eq!(parsed.rank_history, vec![300, 0, 250, 200]);

        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
        let rows = official_rank_rows(today, &parsed.rank_history);
        assert_eq!(rows, vec![("2026-10-04".to_string(), 300), ("2026-10-06".to_string(), 250), ("2026-10-07".to_string(), 200)]);

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::record_rank_snapshot_with_date(&conn, "me", 0, "2026-10-07", 123).unwrap();
        let added = db::import_rank_history(&conn, "me", 0, &rows).unwrap();
        assert_eq!(added, 2);
        let hist = db::get_rank_history(&conn, "me", 0, 90).unwrap();
        assert_eq!(hist.len(), 3);
        assert!(hist.iter().any(|(d, r)| d == "2026-10-07" && *r == 123));
    }

    #[test]
    fn test_parse_official_userpage_html() {
        let mock_html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>peppy</title></head>
            <body>
                <div class="js-react--profile-page" data-initial-data="{&quot;user&quot;:{&quot;id&quot;:2,&quot;username&quot;:&quot;peppy&quot;,&quot;country_code&quot;:&quot;AU&quot;,&quot;location&quot;:&quot;Melbourne&quot;,&quot;playstyle&quot;:[&quot;mouse&quot;,&quot;keyboard&quot;,&quot;touch&quot;],&quot;avatar_url&quot;:&quot;https://a.ppy.sh/2?1234&quot;,&quot;page&quot;:{&quot;raw&quot;:&quot;[b]Hi there[/b]&quot;}}}"></div>
            </body>
            </html>
        "#;

        let parsed = parse_official_userpage_html(mock_html).expect("Should parse valid profile HTML");
        assert_eq!(parsed.id, 2);
        assert_eq!(parsed.username, "peppy");
        assert_eq!(parsed.country_code.as_deref(), Some("AU"));
        assert_eq!(parsed.location.as_deref(), Some("Melbourne"));
        assert_eq!(parsed.playstyle, vec!["Mouse", "Keyboard", "Touchscreen"]);
        assert_eq!(parsed.raw_bio.as_deref(), Some("[b]Hi there[/b]"));
        assert_eq!(parsed.avatar_url.as_deref(), Some("https://a.ppy.sh/2?1234"));
    }

    #[test]
    fn test_parse_official_api_user_extras() {
        let v = serde_json::json!({
            "id": 7, "username": "exile sense", "avatar_url": "https://a.ppy.sh/7", "country_code": "gb",
            "interests": "rhythm ", "occupation": "", "twitter": "handle", "website": "https://e.x", "discord": "me#1",
            "join_date": "2019-04-04T10:00:00+00:00", "playstyle": ["mouse", "keyboard"],
            "cover": {"url": "https://assets.ppy.sh/c.jpg"}, "page": {"raw": "hi"}
        });
        let p = parse_official_user_value(&v).unwrap();
        assert_eq!(p.interests.as_deref(), Some("rhythm"));
        assert_eq!(p.occupation, None);
        assert_eq!(p.twitter.as_deref(), Some("handle"));
        assert_eq!(p.join_month.as_deref(), Some("2019-04"));
        assert_eq!(p.playstyle, vec!["Mouse", "Keyboard"]);
        assert_eq!(p.country_code.as_deref(), Some("GB"));
    }

    #[test]
    fn test_parse_official_userpage_html_errors() {
        let html_no_data = "<html><body>No data here</body></html>";
        assert!(parse_official_userpage_html(html_no_data).is_err());

        let html_bad_json = "<div data-initial-data=\"not valid json\"></div>";
        assert!(parse_official_userpage_html(html_bad_json).is_err());

        let html_no_user = "<div data-initial-data=\"{&quot;user&quot;:{&quot;id&quot;:1}}\"></div>";
        assert!(parse_official_userpage_html(html_no_user).is_err());
    }

    #[tokio::test]
    async fn test_import_official_route_session_and_csrf() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        crate::db::ensure_profile(&conn, "ImportUser").unwrap();

        let app_state = AppState::new(conn, crate::types::config::Config::default());
        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(app_state));

        let empty_headers = HeaderMap::new();
        let payload = serde_json::json!({ "query": "peppy" }).to_string();
        let resp_unauth = handle_import_official(shared_state.clone(), &empty_headers, payload.as_bytes()).await;
        assert_eq!(resp_unauth.status, StatusCode::UNAUTHORIZED);

        let s_resp = handle_session(shared_state.clone(), &HeaderMap::new()).await;
        let s_data: SessionResponse = serde_json::from_slice(&s_resp.body).unwrap();
        let set_cookie = s_resp.header("set-cookie").unwrap();
        let session_token = extract_cookie(set_cookie, SESSION_COOKIE_NAME).unwrap().to_string();

        let mut login_headers = HeaderMap::new();
        login_headers.insert("cookie", HeaderValue::from_str(&format!("los_session={}", session_token)).unwrap());
        login_headers.insert("origin", HeaderValue::from_static("http://127.0.0.1:5000"));
        login_headers.insert("host", HeaderValue::from_static("127.0.0.1:5000"));
        login_headers.insert("x-csrf-token", HeaderValue::from_str(&s_data.csrf).unwrap());

        let login_body = serde_json::json!({ "username": "ImportUser" }).to_string();
        let l_resp = handle_login(shared_state.clone(), &login_headers, login_body.as_bytes()).await;
        assert_eq!(l_resp.status, StatusCode::OK);

        let mut bad_csrf_headers = login_headers.clone();
        bad_csrf_headers.insert("x-csrf-token", HeaderValue::from_static("bad-token"));
        let resp_bad_csrf = handle_import_official(shared_state.clone(), &bad_csrf_headers, payload.as_bytes()).await;
        assert_eq!(resp_bad_csrf.status, StatusCode::FORBIDDEN);

        let empty_query_body = serde_json::json!({ "query": "   " }).to_string();
        let resp_bad_query = handle_import_official(shared_state.clone(), &login_headers, empty_query_body.as_bytes()).await;
        assert_eq!(resp_bad_query.status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_country_rank_empty_and_fallback() {
        let http = reqwest::Client::new();
        assert_eq!(get_country_rank(&http, "", 1000.0, None).await, None);
        assert_eq!(get_country_rank(&http, "   ", 1000.0, None).await, None);
        assert_eq!(get_country_rank(&http, "DE", 0.0, None).await, None);
        assert_eq!(get_country_rank(&http, "DE", -50.0, None).await, None);
        assert_eq!(get_country_rank(&http, "DE", 1000.0, Some((123, ""))).await, None);
        assert_eq!(get_country_rank(&http, "DE", 1000.0, Some((123, "   "))).await, None);
    }

    #[tokio::test]
    async fn test_handle_medals_catalog() {
        let resp = handle_medals_catalog().await;
        assert_eq!(resp.status, StatusCode::OK);
        let medals: Vec<crate::types::medal::MedalDefinition> = serde_json::from_slice(&resp.body).expect("valid json catalog");
        assert_eq!(medals.len(), 352);
    }

    #[tokio::test]
    async fn test_profile_effective_country_fallback() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        crate::db::ensure_profile(&conn, "CountryFallbackUser").unwrap();

        let mut config = crate::types::config::Config::default();
        config.country = Some("DE".to_string());
        let app_state = AppState::new(conn, config);
        let shared_state = std::sync::Arc::new(tokio::sync::RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("name".to_string(), "CountryFallbackUser".to_string());
        params.insert("mode".to_string(), "osu".to_string());

        let resp = handle_profile(shared_state, &params).await;
        assert_eq!(resp.status, StatusCode::OK);
        let data: ProfileResponse = serde_json::from_slice(&resp.body).unwrap();
        assert_eq!(data.details.country, "DE");
    }
}

