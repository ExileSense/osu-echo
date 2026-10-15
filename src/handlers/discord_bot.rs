
use crate::console::TraceSend;
use crate::db;
use crate::handlers::latest_play::{self, LatestPlay};
use crate::state::SharedState;
use crate::types::beatmap::Beatmap;
use crate::types::mods::Mods;
use crate::types::score::Score;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const API: &str = "https://discord.com/api/v10";
const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

const COLOR_FAIL: u32 = 0xef4444;

fn is_fatal_close(code: u16) -> bool {
    matches!(code, 4004 | 4010 | 4011 | 4012 | 4013 | 4014)
}

fn clears_session(code: u16) -> bool {
    matches!(code, 4007 | 4009)
}

#[derive(Default)]
struct Session {
    session_id: Option<String>,
    resume_url: Option<String>,
    seq: Option<u64>,
    app_id: Option<String>,
}

enum SessionEnd {
    Reconnect,
    Fatal(String),
}

pub async fn run(state: SharedState) {
    let (token, config_app_id, guild_id, http) = {
        let s = state.read().await;
        (
            s.config.discord_bot_token.clone(),
            s.config.discord_application_id.clone(),
            s.config.discord_guild_id.clone(),
            s.http.clone(),
        )
    };
    let Some(token) = token.filter(|t| !t.trim().is_empty()) else {
        return;
    };

    crate::logger::info("Discord bot: starting");
    let mut session = Session { app_id: config_app_id, ..Default::default() };
    let mut backoff = 5u64;

    loop {
        let mut reached_ready = false;
        match run_session(&state, &http, &token, guild_id.as_deref(), &mut session, &mut reached_ready).await {
            Ok(SessionEnd::Fatal(msg)) => {
                crate::logger::warn(&format!("Discord bot: stopped ({})", msg));
                return;
            }
            Ok(SessionEnd::Reconnect) => {}
            Err(e) => crate::logger::warn(&format!("Discord bot: connection error: {}", e)),
        }
        if reached_ready {
            backoff = 5;
        }
        tokio::time::sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(60);
    }
}

async fn run_session(
    state: &SharedState,
    http: &HttpClient,
    token: &str,
    guild_id: Option<&str>,
    session: &mut Session,
    reached_ready: &mut bool,
) -> Result<SessionEnd, String> {
    let url = match (&session.resume_url, &session.session_id) {
        (Some(u), Some(_)) => format!("{}/?v=10&encoding=json", u.trim_end_matches('/')),
        _ => GATEWAY_URL.to_string(),
    };
    let (ws, _) = connect_async(url.as_str()).await.map_err(|e| e.to_string())?;
    let (mut write, mut read) = ws.split();

    let interval_ms = loop {
        match read.next().await {
            Some(Ok(Message::Text(t))) => {
                let v: Value = serde_json::from_str(t.as_str()).map_err(|e| e.to_string())?;
                if v["op"] == 10 {
                    break v["d"]["heartbeat_interval"].as_u64().unwrap_or(41_250);
                }
            }
            Some(Ok(_)) => {}
            Some(Err(e)) => return Err(e.to_string()),
            None => return Err("closed before hello".to_string()),
        }
    };

    let first = match (&session.session_id, session.seq) {
        (Some(id), Some(seq)) => json!({"op": 6, "d": {"token": token, "session_id": id, "seq": seq}}),
        _ => json!({
            "op": 2,
            "d": {
                "token": token,
                "intents": 0,
                "properties": {"os": "windows", "browser": "osu-echo", "device": "osu-echo"}
            }
        }),
    };
    write.send(Message::Text(first.to_string().into())).await.map_err(|e| e.to_string())?;

    let start = tokio::time::Instant::now() + Duration::from_millis(interval_ms / 2);
    let mut hb = tokio::time::interval_at(start, Duration::from_millis(interval_ms));
    let mut acked = true;

    loop {
        tokio::select! {
            _ = hb.tick() => {
                if !acked {
                    return Err("heartbeat not acknowledged".to_string());
                }
                acked = false;
                let beat = json!({"op": 1, "d": session.seq});
                write.send(Message::Text(beat.to_string().into())).await.map_err(|e| e.to_string())?;
            }
            msg = read.next() => {
                let msg = match msg {
                    None => return Ok(SessionEnd::Reconnect),
                    Some(Err(e)) => return Err(e.to_string()),
                    Some(Ok(m)) => m,
                };
                match msg {
                    Message::Close(frame) => {
                        let code = frame.map(|f| u16::from(f.code)).unwrap_or(0);
                        if is_fatal_close(code) {
                            return Ok(SessionEnd::Fatal(format!("Discord closed the connection with code {} - check the bot token", code)));
                        }
                        if clears_session(code) {
                            *session = Session { app_id: session.app_id.clone(), ..Default::default() };
                        }
                        return Ok(SessionEnd::Reconnect);
                    }
                    Message::Text(t) => {
                        let Ok(v) = serde_json::from_str::<Value>(t.as_str()) else { continue };
                        if let Some(s) = v["s"].as_u64() {
                            session.seq = Some(s);
                        }
                        match v["op"].as_u64().unwrap_or(99) {
                            0 => match v["t"].as_str().unwrap_or("") {
                                "READY" => {
                                    *reached_ready = true;
                                    session.session_id = v["d"]["session_id"].as_str().map(String::from);
                                    session.resume_url = v["d"]["resume_gateway_url"].as_str().map(String::from);
                                    if session.app_id.is_none() {
                                        session.app_id = v["d"]["application"]["id"].as_str().map(String::from);
                                    }
                                    crate::logger::info("Discord bot: connected");
                                    if let Some(app_id) = session.app_id.clone() {
                                        let http = http.clone();
                                        let token = token.to_string();
                                        let guild = guild_id.map(String::from);
                                        tokio::spawn(async move {
                                            register_commands(&http, &token, &app_id, guild.as_deref()).await;
                                            crate::handlers::discord_emoji::sync(&http, &token, &app_id).await;
                                        });
                                    }
                                }
                                "RESUMED" => *reached_ready = true,
                                "INTERACTION_CREATE" => {
                                    let state = state.clone();
                                    let http = http.clone();
                                    let interaction = v["d"].clone();
                                    tokio::spawn(async move {
                                        handle_interaction(state, http, interaction).await;
                                    });
                                }
                                _ => {}
                            },
                            1 => {
                                let beat = json!({"op": 1, "d": session.seq});
                                write.send(Message::Text(beat.to_string().into())).await.map_err(|e| e.to_string())?;
                            }
                            7 => return Ok(SessionEnd::Reconnect),
                            9 => {
                                if !v["d"].as_bool().unwrap_or(false) {
                                    *session = Session { app_id: session.app_id.clone(), ..Default::default() };
                                }
                                tokio::time::sleep(Duration::from_secs(3)).await;
                                return Ok(SessionEnd::Reconnect);
                            }
                            11 => acked = true,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn username_option(description: &str) -> Value {
    json!([{ "type": 3, "name": "username", "description": description, "required": false }])
}

pub fn command_definitions() -> Value {
    let mut defs = json!([
        {
            "name": "rs",
            "description": "Show a player's most recent play (passed or failed)",
            "options": username_option("Player name (defaults to you)")
        },
        {
            "name": "recent",
            "description": "Show a player's most recent play (same as /rs)",
            "options": username_option("Player name (defaults to you)")
        },
        {
            "name": "link",
            "description": "Link your Discord account to a profile on this server",
            "options": [{ "type": 3, "name": "username", "description": "Your profile name", "required": true }]
        },
        {
            "name": "top",
            "description": "Show a player's top plays",
            "options": username_option("Player name (defaults to you)")
        },
        {
            "name": "profile",
            "description": "Show a player's profile",
            "options": username_option("Player name (defaults to you)")
        }
    ]);
    if let Some(list) = defs.as_array_mut() {
        for cmd in list {
            cmd["integration_types"] = json!([0, 1]);
            cmd["contexts"] = json!([0, 1, 2]);
        }
    }
    defs
}

async fn register_commands(http: &HttpClient, token: &str, app_id: &str, guild_id: Option<&str>) {
    let url = match guild_id {
        Some(g) => format!("{}/applications/{}/guilds/{}/commands", API, app_id, g),
        None => format!("{}/applications/{}/commands", API, app_id),
    };
    match http
        .put(&url)
        .header("Authorization", format!("Bot {}", token))
        .timeout(Duration::from_secs(15))
        .json(&command_definitions())
        .send_traced()
        .await
    {
        Ok(r) if r.status().is_success() => crate::logger::info("Discord bot: slash commands registered"),
        Ok(r) => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            crate::logger::warn(&format!("Discord bot: command registration failed ({}): {}", status, body));
        }
        Err(e) => crate::logger::warn(&format!("Discord bot: command registration failed: {}", e)),
    }
}

pub fn username_arg(interaction: &Value) -> Option<String> {
    interaction["data"]["options"]
        .as_array()?
        .iter()
        .find(|o| o["name"] == "username")
        .and_then(|o| o["value"].as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

async fn post_callback(http: &HttpClient, id: &str, token: &str, kind: u8, ephemeral: bool) -> Result<(), String> {
    let body = if ephemeral { json!({"type": kind, "data": {"flags": 64}}) } else { json!({"type": kind}) };
    http.post(format!("{}/interactions/{}/{}/callback", API, id, token))
        .timeout(Duration::from_secs(10))
        .json(&body)
        .send_traced()
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

async fn edit_original(http: &HttpClient, app_id: &str, token: &str, body: &Value, what: &str) {
    let result = http
        .patch(format!("{}/webhooks/{}/{}/messages/@original", API, app_id, token))
        .timeout(Duration::from_secs(15))
        .json(body)
        .send_traced()
        .await;
    match result {
        Ok(r) if r.status().is_success() => {}
        Ok(r) => {
            let status = r.status();
            let text = r.text().await.unwrap_or_default();
            crate::logger::warn(&format!("Discord bot: reply to {} failed ({}): {}", what, status, text));
        }
        Err(e) => crate::logger::warn(&format!("Discord bot: reply to {} failed: {}", what, e)),
    }
}

async fn handle_interaction(state: SharedState, http: HttpClient, interaction: Value) {
    let (Some(id), Some(token), Some(app_id)) = (
        interaction["id"].as_str(),
        interaction["token"].as_str(),
        interaction["application_id"].as_str(),
    ) else {
        return;
    };

    match interaction["type"].as_u64() {
        Some(2) => {
            let name = interaction["data"]["name"].as_str().unwrap_or("").to_string();
            let username = username_arg(&interaction);
            let user_id = discord_user_id(&interaction);
            let private = name == "link";

            if let Err(e) = post_callback(&http, id, token, 5, private).await {
                crate::logger::warn(&format!("Discord bot: could not acknowledge /{}: {}", name, e));
                return;
            }
            let username = match (username, name.as_str()) {
                (None, "rs" | "recent" | "top" | "profile") => linked_profile(&state, user_id.as_deref()).await,
                (given, _) => given,
            };
            let body = match name.as_str() {
                "link" => cmd_link(&state, user_id.as_deref(), username).await,
                "rs" | "recent" => cmd_recent(&state, username).await,
                "top" => crate::handlers::discord_views::cmd_top(&state, username, 0).await,
                "profile" => crate::handlers::discord_views::cmd_profile(&state, username, crate::handlers::discord_views::ProfileView::Compact).await,
                _ => message("That command isn't available."),
            };
            edit_original(&http, app_id, token, &body, &format!("/{}", name)).await;
        }
        Some(3) => {
            let custom_id = interaction["data"]["custom_id"].as_str().unwrap_or("").to_string();
            let values: Vec<String> = interaction["data"]["values"]
                .as_array()
                .map(|v| v.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let Some(action) = crate::handlers::discord_views::parse_component(&custom_id, &values) else {
                return;
            };
            if let Err(e) = post_callback(&http, id, token, 6, false).await {
                crate::logger::warn(&format!("Discord bot: could not acknowledge a component: {}", e));
                return;
            }
            let body = crate::handlers::discord_views::run_component(&state, action).await;
            edit_original(&http, app_id, token, &body, "a component").await;
        }
        _ => {}
    }
}

pub fn discord_user_id(interaction: &Value) -> Option<String> {
    interaction["member"]["user"]["id"]
        .as_str()
        .or_else(|| interaction["user"]["id"].as_str())
        .map(String::from)
}

async fn linked_profile(state: &SharedState, user_id: Option<&str>) -> Option<String> {
    let uid = user_id?;
    let s = state.read().await;
    let conn = s.db.lock().await;
    db::get_discord_link(&conn, uid).ok().flatten()
}

async fn cmd_link(state: &SharedState, user_id: Option<&str>, username: Option<String>) -> Value {
    let Some(uid) = user_id else {
        return message("I couldn't tell which Discord account ran this command.");
    };
    let Some(requested) = username else {
        return message("Tell me which profile to link, for example `/link username:YourName`.");
    };
    let s = state.read().await;
    let conn = s.db.lock().await;
    let profiles = db::list_profiles(&conn).unwrap_or_default();
    let Some(name) = profiles.into_iter().find(|p| p.eq_ignore_ascii_case(&requested)) else {
        return message(&format!("No player named **{}** on this server.", requested));
    };
    match db::set_discord_link(&conn, uid, &name) {
        Ok(()) => message(&format!(
            "Linked your Discord account to **{}**. `/rs`, `/top` and `/profile` now use that profile when you leave the username out.",
            name
        )),
        Err(_) => message("Couldn't save the link, try again."),
    }
}

pub(crate) fn message(text: &str) -> Value {
    json!({ "content": text })
}

pub(crate) async fn resolve_player(state: &SharedState, requested: Option<String>) -> Result<String, String> {
    let s = state.read().await;
    let conn = s.db.lock().await;
    let profiles = db::list_profiles(&conn).map_err(|_| "Database error.".to_string())?;
    if let Some(req) = requested {
        return profiles
            .into_iter()
            .find(|p| p.eq_ignore_ascii_case(&req))
            .ok_or_else(|| format!("No player named **{}** on this server.", req));
    }
    if let Some(p) = s.player.as_ref() {
        return Ok(p.name.clone());
    }
    match db::get_default_profile(&conn) {
        Ok(Some(name)) => Ok(name),
        _ => Err("No players on this server yet.".to_string()),
    }
}

pub(crate) struct Ctx {
    pub http: HttpClient,
    pub db: std::sync::Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    pub config: crate::types::config::Config,
}

pub(crate) async fn context(state: &SharedState) -> Ctx {
    let s = state.read().await;
    Ctx { http: s.http.clone(), db: s.db.clone(), config: s.config.clone() }
}

pub(crate) async fn player_info(ctx: &Ctx, name: &str) -> PlayerInfo {
    let (pp, country_byte) = {
        let conn = ctx.db.lock().await;
        (profile_pp(&conn, name).unwrap_or(0.0), db::get_profile_country(&conn, name).unwrap_or(0))
    };
    let mut country = crate::utils::country_byte_to_code(country_byte).to_string();
    if country == "Unknown" {
        country = ctx.config.country.clone().unwrap_or_default();
    }

    let daily_key = ctx.config.osu_daily_api_key.clone();
    let creds = match (&ctx.config.osu_client_id, &ctx.config.osu_client_secret) {
        (Some(cid), Some(sec)) => cid
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .parse::<u64>()
            .ok()
            .map(|id| (id, sec.trim().trim_matches('"').trim_matches('\'').to_string())),
        _ => None,
    };
    let pp_i = pp.round() as i32;
    let ranks = async {
        if pp_i <= 0 {
            return (None, None);
        }
        let global = crate::handlers::website::get_daily_rank(&ctx.http, daily_key.as_deref(), pp_i as f64);
        let country_rank = async {
            if country.is_empty() {
                None
            } else {
                crate::handlers::website::get_country_rank(&ctx.http, &country, pp_i as f64, creds.as_ref().map(|(id, sec)| (*id, sec.as_str()))).await
            }
        };
        tokio::join!(global, country_rank)
    };
    let (global_rank, country_rank) = tokio::time::timeout(Duration::from_secs(8), ranks).await.unwrap_or((None, None));
    PlayerInfo { name: name.to_string(), pp, country: country.clone(), global_rank, country_rank }
}

pub(crate) async fn avatar_url(ctx: &Ctx, name: &str) -> Option<String> {
    let conn = ctx.db.lock().await;
    db::get_avatar(&conn, name).ok().flatten().filter(|u| u.starts_with("https://") || u.starts_with("http://"))
}

pub(crate) fn author_json(who: &PlayerInfo) -> Value {
    let mut author_name = who.name.clone();
    if who.pp > 0.0 {
        author_name.push_str(&format!(": {}pp", fmt_pp(who.pp)));
        let mut ranks = Vec::new();
        if let Some(g) = who.global_rank {
            ranks.push(format!("#{}", crate::handlers::webhook::format_number(g as i64)));
        }
        if let Some(c) = who.country_rank {
            ranks.push(format!("{}{}", who.country.to_uppercase(), c));
        }
        if !ranks.is_empty() {
            author_name.push_str(&format!(" ({})", ranks.join(" ")));
        }
    }
    let mut author = json!({ "name": author_name });
    if who.country.len() == 2 {
        author["icon_url"] = json!(format!("https://flagcdn.com/w80/{}.png", who.country.to_lowercase()));
    }
    author
}

async fn cmd_recent(state: &SharedState, username: Option<String>) -> Value {
    let name = match resolve_player(state, username).await {
        Ok(n) => n,
        Err(e) => return message(&e),
    };

    let latest = latest_play::get(&name);
    let try_n = if latest.is_some() { latest_play::try_number(&name) } else { 1 };
    let ctx = context(state).await;

    let play = match latest {
        Some(p) => Some(p),
        None => {
            let conn = ctx.db.lock().await;
            db::get_all_scores(&conn, &name).ok().and_then(|scores| scores.into_iter().next()).map(|score| {
                let beatmap = db::get_beatmap_by_md5(&conn, &score.md5).ok().flatten();
                LatestPlay { player_name: name.clone(), recorded_at: score.time, score, beatmap, passed: true, rank_after: None }
            })
        }
    };
    let Some(play) = play else {
        return message(&format!("**{}** has no recent plays.", name));
    };

    let calc = async {
        let bmap = play.beatmap.as_ref()?;
        let content = crate::utils::get_or_fetch_beatmap_content(&ctx.http, &ctx.db, &ctx.config, bmap).await?;
        let parsed = rosu_pp::Beatmap::from_bytes(content.as_bytes()).ok()?;
        Some(calc_map(&parsed, &play.score, bmap))
    };
    let (who, calc) = tokio::join!(player_info(&ctx, &name), async {
        tokio::time::timeout(Duration::from_secs(8), calc).await.unwrap_or(None)
    });

    json!({
        "content": format!("Try #{}", try_n),
        "embeds": [build_play_embed(&play, &who, calc.as_ref())]
    })
}

pub(crate) fn profile_pp(conn: &rusqlite::Connection, name: &str) -> Option<f64> {
    conn.query_row("SELECT pp FROM profiles WHERE name = ?1", rusqlite::params![name], |r| r.get::<_, f64>(0)).ok()
}

pub struct PlayerInfo {
    pub name: String,
    pub pp: f64,
    pub country: String,
    pub global_rank: Option<i32>,
    pub country_rank: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct MapCalc {
    pub stars: f64,
    pub cs: f64,
    pub ar: f64,
    pub od: f64,
    pub hp: f64,
    pub bpm: f64,
    pub length_secs: i64,
    pub max_pp: f64,
    pub if_fc_pp: Option<f64>,
    pub max_combo: i32,
}

pub(crate) fn clock_rate(mods: u32) -> f64 {
    let m = Mods::from_bits_truncate(mods);
    if m.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE) {
        1.5
    } else if m.contains(Mods::HALFTIME) {
        0.75
    } else {
        1.0
    }
}

pub fn mod_adjusted_stats(cs: f64, ar: f64, od: f64, hp: f64, mods: u32) -> (f64, f64, f64, f64) {
    let m = Mods::from_bits_truncate(mods);
    let (mut cs, mut ar, mut od, mut hp) = (cs, ar, od, hp);
    if m.contains(Mods::EASY) {
        cs *= 0.5;
        ar *= 0.5;
        od *= 0.5;
        hp *= 0.5;
    }
    if m.contains(Mods::HARDROCK) {
        cs = (cs * 1.3).min(10.0);
        ar = (ar * 1.4).min(10.0);
        od = (od * 1.4).min(10.0);
        hp = (hp * 1.4).min(10.0);
    }
    let rate = clock_rate(mods);
    if (rate - 1.0).abs() > f64::EPSILON {
        let ms = if ar <= 5.0 { 1800.0 - 120.0 * ar } else { 1200.0 - 150.0 * (ar - 5.0) } / rate;
        ar = if ms > 1200.0 { (1800.0 - ms) / 120.0 } else { 5.0 + (1200.0 - ms) / 150.0 };
        let window = (80.0 - 6.0 * od) / rate;
        od = (80.0 - window) / 6.0;
    }
    (cs, ar, od, hp)
}

pub(crate) fn game_mode(mode: i32) -> rosu_pp::model::mode::GameMode {
    match mode {
        1 => rosu_pp::model::mode::GameMode::Taiko,
        2 => rosu_pp::model::mode::GameMode::Catch,
        3 => rosu_pp::model::mode::GameMode::Mania,
        _ => rosu_pp::model::mode::GameMode::Osu,
    }
}

fn calc_map(parsed: &rosu_pp::Beatmap, score: &Score, bmap: &Beatmap) -> MapCalc {
    let mode = game_mode(score.mode);
    let max = rosu_pp::Performance::new(parsed).mode_or_ignore(mode).mods(score.mods).lazer(false).calculate();
    let attrs = max.difficulty_attributes();
    let stars = attrs.stars();
    let max_combo = attrs.max_combo() as i32;
    let max_pp = max.pp();

    let if_fc_pp = if score.mode == 0 && score.nmiss > 0 {
        Some(
            rosu_pp::Performance::new(parsed)
                .mode_or_ignore(mode)
                .mods(score.mods)
                .lazer(false)
                .n300((score.n300 + score.nmiss).max(0) as u32)
                .n100(score.n100.max(0) as u32)
                .n50(score.n50.max(0) as u32)
                .misses(0)
                .calculate()
                .pp(),
        )
    } else {
        None
    };

    let rate = clock_rate(score.mods);
    let (cs, ar, od, hp) = mod_adjusted_stats(bmap.diff_size, bmap.diff_approach, bmap.diff_overall, bmap.diff_drain, score.mods);
    let base_len = if bmap.hit_length > 0 { bmap.hit_length } else { bmap.total_length };
    MapCalc {
        stars,
        cs,
        ar,
        od,
        hp,
        bpm: bmap.bpm * rate,
        length_secs: (base_len as f64 / rate).round() as i64,
        max_pp,
        if_fc_pp,
        max_combo,
    }
}

fn trim_num(v: f64) -> String {
    let s = format!("{:.2}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub(crate) fn fmt_pp(v: f64) -> String {
    let s = format!("{:.2}", v.max(0.0));
    let (int, frac) = s.split_once('.').unwrap_or((&s, "00"));
    format!("{}.{}", crate::handlers::webhook::format_number(int.parse::<i64>().unwrap_or(0)), frac)
}

fn status_name(approved: i32) -> &'static str {
    match approved {
        1 => "Ranked",
        2 => "Approved",
        3 => "Qualified",
        4 => "Loved",
        -1 => "WIP",
        -2 => "Graveyard",
        _ => "Pending",
    }
}

fn hits_text(score: &Score) -> String {
    if score.nmiss > 0 {
        return format!("{} {}", score.nmiss, crate::handlers::discord_emoji::miss());
    }
    match score.mode {
        1 => format!("{}/{}", score.n300, score.n100),
        2 => format!("{}/{}/{}", score.n300, score.n100, score.n50),
        3 => format!("{}/{}/{}/{}/{}", score.ngeki, score.n300, score.nkatu, score.n100, score.n50),
        _ => format!("{}/{}/{}", score.n300, score.n100, score.n50),
    }
}

pub fn build_play_embed(play: &LatestPlay, who: &PlayerInfo, calc: Option<&MapCalc>) -> Value {
    let score: &Score = &play.score;
    let bmap = play.beatmap.as_ref();

    let grade = if play.passed {
        crate::utils::get_grade(
            score.mode as u8,
            score.n300,
            score.n100,
            score.n50,
            score.ngeki,
            score.nkatu,
            score.nmiss,
            score.mods,
            score.acc.unwrap_or(0.0),
        )
    } else {
        "F"
    };

    let title = match bmap {
        Some(b) => {
            let stars = calc.map(|c| c.stars).filter(|s| *s > 0.0).or(Some(b.difficultyrating).filter(|s| *s > 0.0));
            let star_txt = stars.map(|s| format!(" [{:.2}★]", s)).unwrap_or_default();
            format!("{} - {} [{}]{}", b.artist, b.title, b.version, star_txt)
        }
        None => format!("Unknown beatmap ({})", score.md5.chars().take(8).collect::<String>()),
    };
    let url = bmap.and_then(|b| {
        if b.beatmap_id > 0 {
            Some(format!("https://osu.ppy.sh/b/{}", b.beatmap_id))
        } else if b.beatmapset_id > 0 {
            Some(format!("https://osu.ppy.sh/s/{}", b.beatmapset_id))
        } else {
            None
        }
    });

    let author = author_json(who);

    let mods_txt = format!("+{}", Mods::from_bits_truncate(score.mods).short_name());
    let mut lines = Vec::new();
    const GAP: &str = " \u{a0}";
    lines.push(format!(
        "{} **{}**{}**{}**{}**{:.2}%**{}<t:{}:R>",
        crate::handlers::discord_emoji::grade(grade),
        mods_txt,
        GAP,
        crate::handlers::webhook::format_number(score.score),
        GAP,
        score.acc.unwrap_or(0.0),
        GAP,
        play.recorded_at.max(0)
    ));

    let mut line2 = Vec::new();
    if play.passed {
        if let Some(pp) = score.pp.filter(|p| *p > 0.0) {
            match calc {
                Some(c) => {
                    line2.push(format!("**{:.2}**/{:.2}PP", pp, c.max_pp));
                    if let Some(fc) = c.if_fc_pp {
                        line2.push(format!("~~({:.2}pp)~~", fc));
                    }
                }
                None => line2.push(format!("**{:.2}pp**", pp)),
            }
        }
    }
    match calc.map(|c| c.max_combo).filter(|m| *m > 0).or(bmap.map(|b| b.max_combo).filter(|m| *m > 0)) {
        Some(max) => line2.push(format!("**{}x**/{}x", score.max_combo, max)),
        None => line2.push(format!("**{}x**", score.max_combo)),
    }
    line2.push(hits_text(score));
    lines.push(line2.join(" • "));

    if let Some(c) = calc {
        let mut parts = vec![format!("`{:02}:{:02}`", c.length_secs / 60, c.length_secs % 60)];
        if score.mode == 0 {
            parts.push(format!("`CS: {} AR: {} OD: {} HP: {}`", trim_num(c.cs), trim_num(c.ar), trim_num(c.od), trim_num(c.hp)));
        }
        if c.bpm > 0.0 {
            parts.push(format!("{} **{}**", crate::handlers::discord_emoji::bpm(), c.bpm.round() as i64));
        }
        lines.push(parts.join(" • "));
    }

    let mut embed = json!({
        "title": title,
        "color": if play.passed { crate::handlers::webhook::grade_color(grade) } else { COLOR_FAIL },
        "author": author,
        "description": lines.join("\n"),
    });
    if let Some(u) = url {
        embed["url"] = json!(u);
    }
    if let Some(b) = bmap {
        if b.beatmapset_id > 0 {
            embed["thumbnail"] = json!({ "url": format!("https://b.ppy.sh/thumb/{}l.jpg", b.beatmapset_id) });
        }
        if !b.creator.is_empty() {
            let date = b
                .approved_date
                .as_deref()
                .filter(|d| d.len() >= 16)
                .map(|d| format!(" {} UTC", &d[..16]))
                .unwrap_or_default();
            let mut footer = json!({ "text": format!("Mapset by {} • {}{}", b.creator, status_name(b.approved), date) });
            if b.creator_id > 0 {
                footer["icon_url"] = json!(format!("https://a.ppy.sh/{}", b.creator_id));
            }
            embed["footer"] = footer;
        }
    }
    embed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(passed_pp: Option<f64>) -> Score {
        serde_json::from_value(json!({
            "mode": 0, "md5": "0123456789abcdef", "name": "Tester", "n300": 100, "n100": 2, "n50": 0,
            "ngeki": 0, "nkatu": 0, "nmiss": 1, "score": 123456, "max_combo": 200,
            "perfect": false, "mods": 0, "time": 1_700_000_000, "acc": 98.1, "pp": passed_pp,
            "replay_md5": null, "scoreid": null, "mods_str": null,
            "additional_mods": null, "submission_checksum": null, "submission_identity": null
        }))
        .unwrap()
    }

    fn play(passed: bool, with_map: bool) -> LatestPlay {
        let beatmap = with_map.then(|| {
            let mut b = Beatmap::blank();
            b.artist = "Artist".into();
            b.title = "Song".into();
            b.version = "Insane".into();
            b.beatmap_id = 5;
            b.beatmapset_id = 9;
            b
        });
        LatestPlay {
            player_name: "Tester".into(),
            score: score(Some(123.4)),
            beatmap,
            passed,
            rank_after: Some(1),
            recorded_at: 1_700_000_000,
        }
    }

    fn who() -> PlayerInfo {
        PlayerInfo { name: "Tester".into(), pp: 13236.2, country: "RO".into(), global_rank: Some(1416), country_rank: Some(15) }
    }

    fn calc() -> MapCalc {
        MapCalc {
            stars: 8.06,
            cs: 4.0,
            ar: 10.33,
            od: 9.11,
            hp: 7.0,
            bpm: 480.0,
            length_secs: 97,
            max_pp: 640.76,
            if_fc_pp: Some(446.99),
            max_combo: 1441,
        }
    }

    #[test]
    fn passed_play_embed_follows_the_bathbot_layout() {
        let e = build_play_embed(&play(true, true), &who(), Some(&calc()));
        assert_eq!(e["author"]["name"], "Tester: 13,236.20pp (#1,416 RO15)");
        assert_eq!(e["author"]["icon_url"], "https://flagcdn.com/w80/ro.png");
        assert_eq!(e["title"], "Artist - Song [Insane] [8.06★]");
        let d = e["description"].as_str().unwrap();
        let lines: Vec<&str> = d.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("**"), "grade first: {}", lines[0]);
        assert!(lines[0].contains("**+NM**") && lines[0].contains("**123,456**") && lines[0].contains("**98.10%**"));
        assert!(lines[0].contains(" \u{a0}"), "gaps survive Discord's whitespace collapsing");
        assert!(lines[0].contains("<t:1700000000:R>"));
        assert_eq!(lines[1], "**123.40**/640.76PP • ~~(446.99pp)~~ • **200x**/1441x • 1 ❌");
        assert_eq!(lines[2], "`01:37` • `CS: 4 AR: 10.33 OD: 9.11 HP: 7` • ◬ **480**");
    }

    #[test]
    fn embed_without_calc_or_ranks_still_reads_well() {
        let mut p = who();
        p.global_rank = None;
        p.country_rank = None;
        let e = build_play_embed(&play(true, true), &p, None);
        assert_eq!(e["author"]["name"], "Tester: 13,236.20pp");
        assert_eq!(e["description"].as_str().unwrap().lines().count(), 2);
        let mut nobody = who();
        nobody.pp = 0.0;
        assert_eq!(build_play_embed(&play(true, true), &nobody, None)["author"]["name"], "Tester");
    }

    #[test]
    fn failed_play_embed_is_red_with_f_grade() {
        let e = build_play_embed(&play(false, true), &who(), Some(&calc()));
        assert_eq!(e["color"], COLOR_FAIL);
        let first = e["description"].as_str().unwrap().lines().next().unwrap().to_string();
        assert!(first.starts_with("**F**"), "{}", first);
        assert!(!e["description"].as_str().unwrap().contains("PP"), "no pp for fails");
    }

    #[test]
    fn unknown_map_still_builds_an_embed() {
        let e = build_play_embed(&play(false, false), &who(), None);
        assert!(e["title"].as_str().unwrap().contains("01234567"));
        assert!(e.get("footer").is_none());
    }

    #[test]
    fn footer_names_mapper_status_and_date() {
        let mut p = play(true, true);
        if let Some(b) = p.beatmap.as_mut() {
            b.creator = "val0108".into();
            b.creator_id = 77;
            b.approved = 1;
            b.approved_date = Some("2012-01-27 03:58:00".into());
        }
        let e = build_play_embed(&p, &who(), None);
        assert_eq!(e["footer"]["text"], "Mapset by val0108 • Ranked 2012-01-27 03:58 UTC");
        assert_eq!(e["footer"]["icon_url"], "https://a.ppy.sh/77");
    }

    #[test]
    fn mod_stats_match_known_values() {
        let dt = Mods::DOUBLETIME.bits();
        let (cs, ar, od, hp) = mod_adjusted_stats(4.0, 9.0, 7.0, 7.0, dt);
        assert_eq!(trim_num(cs), "4");
        assert_eq!(trim_num(ar), "10.33");
        assert_eq!(trim_num(od), "9.11");
        assert_eq!(trim_num(hp), "7");
        let (cs, ar, od, hp) = mod_adjusted_stats(4.0, 8.0, 8.0, 5.0, Mods::HARDROCK.bits());
        assert_eq!((trim_num(cs), trim_num(ar), trim_num(od), trim_num(hp)), ("5.2".into(), "10".into(), "10".into(), "7".into()));
        let (cs, ar, _, _) = mod_adjusted_stats(4.0, 8.0, 8.0, 5.0, Mods::EASY.bits());
        assert_eq!((trim_num(cs), trim_num(ar)), ("2".into(), "4".into()));
        assert_eq!(mod_adjusted_stats(4.0, 9.0, 7.0, 7.0, 0), (4.0, 9.0, 7.0, 7.0));
    }

    #[test]
    fn username_option_is_read_and_trimmed() {
        let i = json!({"data": {"name": "rs", "options": [{"name": "username", "type": 3, "value": "  Foo  "}]}});
        assert_eq!(username_arg(&i), Some("Foo".to_string()));
        assert_eq!(username_arg(&json!({"data": {"name": "rs"}})), None);
        let blank = json!({"data": {"options": [{"name": "username", "value": "  "}]}});
        assert_eq!(username_arg(&blank), None);
    }

    #[test]
    fn command_definitions_list_all_commands() {
        let defs = command_definitions();
        let names: Vec<&str> = defs.as_array().unwrap().iter().filter_map(|c| c["name"].as_str()).collect();
        assert_eq!(names, vec!["rs", "recent", "link", "top", "profile"]);
        for c in defs.as_array().unwrap() {
            assert_eq!(c["contexts"], json!([0, 1, 2]), "usable in DMs and group chats");
            assert_eq!(c["integration_types"], json!([0, 1]), "user-installable");
        }
    }

    #[test]
    fn user_id_comes_from_member_or_dm_user() {
        let in_server = json!({"member": {"user": {"id": "111"}}});
        let in_dm = json!({"user": {"id": "222"}});
        assert_eq!(discord_user_id(&in_server), Some("111".to_string()));
        assert_eq!(discord_user_id(&in_dm), Some("222".to_string()));
        assert_eq!(discord_user_id(&json!({})), None);
    }

    #[test]
    fn close_code_classification() {
        assert!(is_fatal_close(4004));
        assert!(!is_fatal_close(4000));
        assert!(clears_session(4009));
        assert!(!clears_session(4000));
    }

    #[tokio::test]
    async fn gateway_handshake_resume_heartbeat_and_fatal_close() {
        use std::sync::Arc;
        use tokio::net::TcpListener;
        use tokio::sync::RwLock;
        use tokio_tungstenite::{accept_async, tungstenite::protocol::{frame::coding::CloseCode, CloseFrame}};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = tokio::spawn(async move {
            let (sock, _) = listener.accept().await.unwrap();
            let ws = accept_async(sock).await.unwrap();
            let (mut w, mut r) = ws.split();
            w.send(Message::Text(json!({"op": 10, "d": {"heartbeat_interval": 100}}).to_string().into())).await.unwrap();

            let first: Value = match r.next().await.unwrap().unwrap() {
                Message::Text(t) => serde_json::from_str(t.as_str()).unwrap(),
                other => panic!("unexpected {:?}", other),
            };
            assert_eq!(first["op"], 6);
            assert_eq!(first["d"]["session_id"], "sess");
            assert_eq!(first["d"]["seq"], 5);

            w.send(Message::Text(json!({"op": 0, "t": "RESUMED", "s": 6, "d": {}}).to_string().into())).await.unwrap();

            let beat: Value = match r.next().await.unwrap().unwrap() {
                Message::Text(t) => serde_json::from_str(t.as_str()).unwrap(),
                other => panic!("unexpected {:?}", other),
            };
            assert_eq!(beat["op"], 1);
            assert_eq!(beat["d"], 6);
            w.send(Message::Text(json!({"op": 11}).to_string().into())).await.unwrap();

            w.send(Message::Close(Some(CloseFrame { code: CloseCode::from(4004), reason: "bad token".into() }))).await.unwrap();
        });

        let state: SharedState = Arc::new(RwLock::new(crate::state::AppState::new(
            rusqlite::Connection::open_in_memory().unwrap(),
            crate::types::config::Config::default(),
        )));
        let http = HttpClient::new();
        let mut session = Session {
            session_id: Some("sess".into()),
            resume_url: Some(format!("ws://127.0.0.1:{}", port)),
            seq: Some(5),
            app_id: None,
        };
        let mut ready = false;
        let end = tokio::time::timeout(
            Duration::from_secs(10),
            run_session(&state, &http, "token", None, &mut session, &mut ready),
        )
        .await
        .expect("session finished")
        .expect("no transport error");
        assert!(matches!(end, SessionEnd::Fatal(_)));
        assert!(ready);
        assert_eq!(session.seq, Some(6));
        server.await.unwrap();
    }

    #[test]
    fn calc_map_gives_max_and_if_fc_pp() {
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 0\n\n[Difficulty]\nHPDrainRate:5\nCircleSize:4\nOverallDifficulty:8\nApproachRate:9\nSliderMultiplier:1.4\nSliderTickRate:1\n\n[TimingPoints]\n0,500,4,2,0,60,1,0\n\n[HitObjects]\n",
        );
        for i in 0..120 {
            let x = 64 + (i % 8) * 50;
            let y = 64 + ((i / 8) % 5) * 50;
            osu.push_str(&format!("{},{},{},1,0\n", x, y, 1000 + i * 250));
        }
        let parsed = rosu_pp::Beatmap::from_bytes(osu.as_bytes()).expect("valid map");
        let mut s = score(Some(50.0));
        s.n300 = 110;
        s.n100 = 5;
        s.n50 = 0;
        s.nmiss = 5;
        let mut b = Beatmap::blank();
        b.diff_size = 4.0;
        b.diff_approach = 9.0;
        b.diff_overall = 8.0;
        b.diff_drain = 5.0;
        b.bpm = 120.0;
        b.hit_length = 150;
        let c = calc_map(&parsed, &s, &b);
        assert!(c.stars > 0.0);
        assert!(c.max_pp > 0.0);
        assert_eq!(c.max_combo, 120);
        let fc = c.if_fc_pp.expect("misses present -> if-FC pp");
        assert!(fc > 0.0 && fc <= c.max_pp);
        assert_eq!(c.length_secs, 150);
        assert_eq!(c.bpm, 120.0);
        s.mods = Mods::DOUBLETIME.bits();
        let dt = calc_map(&parsed, &s, &b);
        assert_eq!(dt.length_secs, 100);
        assert_eq!(dt.bpm, 180.0);
        assert!(dt.max_pp > c.max_pp);
    }
}
