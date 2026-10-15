
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub const CHANNELS: [&str; 4] = ["packets", "requests", "internal", "web"];
const CAP: usize = 600;

#[derive(Clone, Serialize)]
pub struct Entry {
    pub id: u64,
    pub ts: i64,
    pub level: &'static str,
    pub msg: String,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static BUFFERS: OnceLock<Vec<Mutex<VecDeque<Entry>>>> = OnceLock::new();
static STARTED: OnceLock<Instant> = OnceLock::new();
static STARTED_AT: OnceLock<i64> = OnceLock::new();

fn buffers() -> &'static Vec<Mutex<VecDeque<Entry>>> {
    BUFFERS.get_or_init(|| CHANNELS.iter().map(|_| Mutex::new(VecDeque::with_capacity(CAP))).collect())
}

pub fn init() {
    STARTED.get_or_init(Instant::now);
    STARTED_AT.get_or_init(|| chrono::Utc::now().timestamp());
}

pub fn uptime_secs() -> u64 {
    STARTED.get_or_init(Instant::now).elapsed().as_secs()
}

pub fn started_at() -> i64 {
    *STARTED_AT.get_or_init(|| chrono::Utc::now().timestamp())
}

pub fn push(channel: &str, level: &'static str, msg: impl Into<String>) {
    let Some(idx) = CHANNELS.iter().position(|c| *c == channel) else { return };
    let msg: String = msg.into();
    let entry = Entry { id: NEXT_ID.fetch_add(1, Ordering::Relaxed), ts: chrono::Utc::now().timestamp_millis(), level, msg: redact_text(&msg) };
    if let Ok(mut q) = buffers()[idx].lock() {
        if q.len() >= CAP {
            q.pop_front();
        }
        q.push_back(entry);
    }
}

pub fn since(channel: &str, after: u64) -> Vec<Entry> {
    let Some(idx) = CHANNELS.iter().position(|c| *c == channel) else { return Vec::new() };
    buffers()[idx].lock().map(|q| q.iter().filter(|e| e.id > after).cloned().collect()).unwrap_or_default()
}

pub fn clear(channel: &str) {
    if let Some(idx) = CHANNELS.iter().position(|c| *c == channel) {
        if let Ok(mut q) = buffers()[idx].lock() {
            q.clear();
        }
    }
}

pub fn packet_name(id: u16) -> String {
    let n = match id {
        0 => "OsuChangeAction",
        1 => "OsuSendPublicMessage",
        2 => "OsuLogout",
        3 => "OsuRequestStatusUpdate",
        4 => "OsuPing",
        5 => "ChoUserId",
        7 => "ChoSendMessage",
        8 => "ChoPong",
        11 => "ChoUserStats",
        12 => "ChoUserLogout",
        19 => "ChoVersionUpdate",
        24 => "ChoNotification",
        25 => "OsuSendPrivateMessage",
        63 => "OsuChannelJoin",
        64 => "ChoChannelJoinSuccess",
        65 => "ChoChannelInfo",
        71 => "ChoPrivileges",
        72 => "ChoFriendsList",
        73 => "OsuFriendAdd",
        74 => "OsuFriendRemove",
        75 => "ChoProtocolVersion",
        76 => "ChoMainMenuIcon",
        78 => "OsuChannelPart",
        83 => "ChoUserPresence",
        85 => "OsuUserStatsRequest",
        86 => "ChoRestart",
        89 => "ChoChannelInfoEnd",
        94 => "ChoUserSilenced",
        96 => "ChoUserPresenceBundle",
        97 => "OsuUserPresenceRequest",
        98 => "OsuUserPresenceRequestAll",
        _ => return format!("packet {}", id),
    };
    format!("{} ({})", n, id)
}

pub fn log_packets(dir: &str, buf: &[u8]) {
    let mut pos = 0;
    while buf.len() >= pos + 7 {
        let id = u16::from_le_bytes([buf[pos], buf[pos + 1]]);
        let len = u32::from_le_bytes([buf[pos + 3], buf[pos + 4], buf[pos + 5], buf[pos + 6]]) as usize;
        if buf.len() < pos + 7 + len {
            break;
        }
        push("packets", if dir == "IN" { "in" } else { "out" }, format!("{:<3} {} - {} B", dir, packet_name(id), len));
        pos += 7 + len;
    }
}

const SECRET_KEYS: [&str; 14] = ["k", "key", "api_key", "apikey", "token", "h", "hash", "p", "pw", "pass", "password", "secret", "client_secret", "access_token"];

pub fn redact_text(text: &str) -> String {
    if !text.contains('=') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut skip_to = 0usize;
    for (i, c) in text.char_indices() {
        if i < skip_to {
            continue;
        }
        out.push(c);
        if c == '?' || c == '&' {
            let rest = &text[i + 1..];
            if let Some(eq) = rest.find('=') {
                let key = &rest[..eq];
                if SECRET_KEYS.contains(&key.to_lowercase().as_str()) {
                    let vstart = eq + 1;
                    let vend = rest[vstart..].find(|ch: char| matches!(ch, '&' | ')' | '"' | '\'' | ' ' | '\n')).map(|n| vstart + n).unwrap_or(rest.len());
                    out.push_str(key);
                    out.push_str("=***");
                    skip_to = i + 1 + vend;
                }
            }
        }
    }
    out
}

pub fn redact_url(url: &str) -> String {
    redact_text(url)
}

pub trait TraceSend {
    fn send_traced(self) -> impl std::future::Future<Output = Result<reqwest::Response, reqwest::Error>> + Send;
}

impl TraceSend for reqwest::RequestBuilder {
    fn send_traced(self) -> impl std::future::Future<Output = Result<reqwest::Response, reqwest::Error>> + Send {
        async move {
            let started = Instant::now();
            let label = self.try_clone().and_then(|b| b.build().ok()).map(|r| format!("{} {}", r.method(), redact_url(r.url().as_str())));
            let res = self.send().await;
            if let Some(label) = label {
                let ms = started.elapsed().as_millis();
                match &res {
                    Ok(r) => push("requests", if r.status().is_success() { "ok" } else { "warn" }, format!("{} -> {} ({} ms)", label, r.status().as_u16(), ms)),
                    Err(e) => push("requests", "error", format!("{} -> failed: {} ({} ms)", label, e, ms)),
                }
            }
            res
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secret_query_values() {
        assert_eq!(redact_url("https://osu.ppy.sh/api/get_user?k=abc123&u=Tester&type=string"), "https://osu.ppy.sh/api/get_user?k=***&u=Tester&type=string");
        assert_eq!(redact_text("error sending request for url (https://x.y/z?a=1&token=SECRET)"), "error sending request for url (https://x.y/z?a=1&token=***)");
        assert_eq!(redact_text("no secrets = here"), "no secrets = here");
        assert_eq!(redact_text("café ?k=1 ünï"), "café ?k=*** ünï");
    }
}
