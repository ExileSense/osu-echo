
use crate::console::{self, TraceSend};
use crate::db;
use crate::server::response::Response;
use crate::state::SharedState;
use hyper::{Method, StatusCode};
use rusqlite::params;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

fn ok(v: Value) -> Response {
    Response::json(&v).with_header("Cache-Control", "no-store")
}

fn err(status: StatusCode, msg: &str) -> Response {
    Response::json(&json!({ "error": msg })).with_status(status).with_header("Cache-Control", "no-store")
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn mask(v: &Option<String>) -> Value {
    match v.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => json!({ "set": false, "preview": "" }),
        Some(s) => {
            let tail: String = s.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
            let preview = if s.chars().count() > 8 { format!("••••••••{}", tail) } else { "••••••••".to_string() };
            json!({ "set": true, "preview": preview })
        }
    }
}

pub async fn handle(state: SharedState, action: &str, params: &HashMap<String, String>, method: &Method, headers: &hyper::HeaderMap, body: &[u8]) -> Response {
    if *method != Method::GET && !headers.contains_key("x-admin") {
        return err(StatusCode::FORBIDDEN, "Missing admin header. Reload the admin page and try again.");
    }
    match (method, action) {
        (&Method::GET, "admin/stats") => stats(state).await,
        (&Method::GET, "admin/users") => users(state).await,
        (&Method::POST, "admin/users/update") => user_update(state, body).await,
        (&Method::POST, "admin/users/delete") => user_delete(state, body).await,
        (&Method::GET, "admin/maps") => maps(state).await,
        (&Method::POST, "admin/maps/set") => map_set(state, body).await,
        (&Method::GET, "admin/keys") => keys(state).await,
        (&Method::POST, "admin/keys") => keys_save(state, body).await,
        (&Method::POST, "admin/ladder/refresh") => ladder_refresh(state).await,
        (&Method::GET, "admin/console") => console_feed(params),
        (&Method::POST, "admin/console/clear") => {
            if let Some(c) = params.get("chan") {
                console::clear(c);
            }
            ok(json!({ "ok": true }))
        }
        (&Method::GET, "admin/other") => other().await,
        (&Method::POST, "admin/other/theme") => set_theme(body),
        (&Method::GET, "admin/settings") => settings(state).await,
        (&Method::POST, "admin/settings") => settings_save(state, body).await,
        (&Method::GET, "admin/setup") => setup_get(state).await,
        (&Method::POST, "admin/setup") => setup_post(state, body).await,
        (&Method::POST, "admin/keys/test") => keys_test(state).await,
        (&Method::POST, "admin/users/create") => user_create(state, body).await,
        (&Method::POST, "admin/system/trust-cert") => trust_cert().await,
        _ => err(StatusCode::NOT_FOUND, "Unknown admin action."),
    }
}

async fn stats(state: SharedState) -> Response {
    let st = state.read().await;
    let conn = st.db.lock().await;
    let users: i64 = conn.query_row("SELECT count(*) FROM profiles", [], |r| r.get(0)).unwrap_or(0);
    let since = now() - 86_400;
    let count: i64 = conn.query_row("SELECT count(*) FROM scores WHERE time >= ?1", params![since], |r| r.get(0)).unwrap_or(0);
    let mut recent = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT s.id, s.player_name, s.mode, s.pp, s.score, s.time, b.artist, b.title, b.version, b.beatmap_id, b.beatmapset_id
         FROM scores s LEFT JOIN beatmaps b ON b.file_md5 = s.md5
         WHERE s.time >= ?1 ORDER BY s.time DESC LIMIT 30",
    ) {
        let rows = stmt.query_map(params![since], |r| {
            Ok(json!({
                "id": r.get::<_, i64>(0)?, "player": r.get::<_, String>(1)?, "mode": r.get::<_, i64>(2)?,
                "pp": r.get::<_, Option<f64>>(3)?, "score": r.get::<_, i64>(4)?, "time": r.get::<_, i64>(5)?,
                "artist": r.get::<_, Option<String>>(6)?, "title": r.get::<_, Option<String>>(7)?, "version": r.get::<_, Option<String>>(8)?,
                "beatmap_id": r.get::<_, Option<i64>>(9)?, "set_id": r.get::<_, Option<i64>>(10)?,
            }))
        });
        if let Ok(rows) = rows {
            recent = rows.filter_map(|r| r.ok()).collect();
        }
    }
    ok(json!({ "users": users, "uptime": console::uptime_secs(), "started_at": console::started_at(), "scores_24h": count, "recent": recent }))
}

fn flag_code(byte: i64, text: &str) -> String {
    let t = text.trim().to_uppercase();
    if t.len() == 2 {
        return t;
    }
    let c = crate::utils::country_byte_to_code(byte as u8);
    if c == "Unknown" { String::new() } else { c.to_string() }
}

async fn users(state: SharedState) -> Response {
    let st = state.read().await;
    let conn = st.db.lock().await;
    let mut list = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT p.name, p.pp, p.playcount, p.country, COALESCE(pd.country, ''),
                (SELECT count(*) FROM scores s WHERE s.player_name = p.name)
         FROM profiles p LEFT JOIN profile_details pd ON pd.player_name = p.name ORDER BY p.name COLLATE NOCASE",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| {
            let byte: i64 = r.get(3)?;
            let text: String = r.get(4)?;
            Ok(json!({ "name": r.get::<_, String>(0)?, "pp": r.get::<_, f64>(1)?, "playcount": r.get::<_, i64>(2)?, "country": flag_code(byte, &text), "scores": r.get::<_, i64>(5)? }))
        }) {
            list = rows.filter_map(|r| r.ok()).collect();
        }
    }
    let online = st.player.as_ref().map(|p| p.name.clone());
    ok(json!({ "users": list, "online": online }))
}

#[derive(Deserialize)]
struct UserUpdate {
    name: String,
    new_name: Option<String>,
    country: Option<String>,
}

async fn user_update(state: SharedState, body: &[u8]) -> Response {
    let p: UserUpdate = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("Invalid request: {}", e)),
    };
    let old = p.name.trim().to_string();
    let mut new_name = old.clone();

    {
        let st = state.read().await;
        let conn = st.db.lock().await;
        if !db::profile_exists(&conn, &old).unwrap_or(false) {
            return err(StatusCode::NOT_FOUND, "That user does not exist.");
        }
        if let Some(n) = p.new_name.as_deref() {
            let n = n.trim();
            if n != old {
                let valid = match crate::handlers::website::validate_username(n) {
                    Ok(v) => v,
                    Err(e) => return err(StatusCode::BAD_REQUEST, e),
                };
                if db::profile_exists(&conn, &valid).unwrap_or(false) {
                    return err(StatusCode::BAD_REQUEST, &format!("The username '{}' is already taken.", valid));
                }
                if let Err(e) = db::rename_profile(&conn, &old, &valid) {
                    return err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Rename failed: {}", e));
                }
                new_name = valid;
            }
        }
        if let Some(c) = p.country.as_deref() {
            let code = c.trim().to_uppercase();
            let byte = if code.is_empty() { 0 } else { crate::utils::country_code_to_byte(&code) };
            if !code.is_empty() && byte == 0 {
                return err(StatusCode::BAD_REQUEST, "Unknown country code. Use a two-letter code such as GB, US or JP.");
            }
            let _ = db::save_profile_country(&conn, &new_name, byte);
            let _ = conn.execute("INSERT OR IGNORE INTO profile_details (player_name) VALUES (?1)", params![new_name]);
            let _ = conn.execute("UPDATE profile_details SET country = ?1 WHERE player_name = ?2", params![code, new_name]);
        }
    }

    let mut st = state.write().await;
    if new_name != old {
        for s in st.web_sessions.values_mut() {
            if s.username.as_deref() == Some(&old) {
                s.username = Some(new_name.clone());
            }
        }
        if st.pending_login_name.as_deref() == Some(&old) {
            st.pending_login_name = Some(new_name.clone());
        }
        if st.config.osu_username.as_deref() == Some(&old) {
            st.config.osu_username = Some(new_name.clone());
        }
    }
    let country_byte = p.country.as_deref().map(|c| crate::utils::country_code_to_byte(c.trim()));
    if let Some(pl) = st.player.as_mut() {
        if pl.name == old || pl.name == new_name {
            pl.name = new_name.clone();
            if let Some(b) = country_byte {
                pl.country = b;
            }
            let (pres, stats) = (crate::packets::user_presence(pl), crate::packets::user_stats(pl));
            pl.queue.extend_from_slice(&pres);
            pl.queue.extend_from_slice(&stats);
        }
    }
    crate::logger::info(&format!("[admin] updated user {} (now '{}')", old, new_name));
    ok(json!({ "ok": true, "name": new_name }))
}

#[derive(Deserialize)]
struct NameOnly {
    name: String,
}

async fn user_delete(state: SharedState, body: &[u8]) -> Response {
    let p: NameOnly = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("Invalid request: {}", e)),
    };
    let name = p.name.trim().to_string();
    {
        let st = state.read().await;
        if st.player.as_ref().map(|pl| pl.name == name).unwrap_or(false) {
            return err(StatusCode::CONFLICT, "This user is connected in-game right now. Log out of osu! first.");
        }
        let conn = st.db.lock().await;
        if !db::profile_exists(&conn, &name).unwrap_or(false) {
            return err(StatusCode::NOT_FOUND, "That user does not exist.");
        }
        let _ = conn.execute("DELETE FROM score_era_pp WHERE score_id IN (SELECT id FROM scores WHERE player_name = ?1)", params![name]);
        if let Err(e) = db::wipe_profile(&conn, &name) {
            return err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Could not delete scores: {}", e));
        }
        for t in ["avatars", "covers", "friends", "profile_details", "user_medals", "discord_links"] {
            let _ = conn.execute(&format!("DELETE FROM {} WHERE player_name = ?1", t), params![name]);
        }
        if let Err(e) = conn.execute("DELETE FROM profiles WHERE name = ?1", params![name]) {
            return err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Could not delete the profile: {}", e));
        }
    }
    let mut st = state.write().await;
    st.web_sessions.retain(|_, s| s.username.as_deref() != Some(&name));
    if st.pending_login_name.as_deref() == Some(&name) {
        st.pending_login_name = None;
    }
    crate::logger::info(&format!("[admin] deleted user {}", name));
    ok(json!({ "ok": true }))
}

async fn maps(state: SharedState) -> Response {
    let st = state.read().await;
    let conn = st.db.lock().await;
    let mut list = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT c.beatmapset_id, MIN(c.beatmap_id), MIN(c.file_md5), c.status, COUNT(*), MAX(c.updated_at),
                COALESCE(MIN(b.artist), ''), COALESCE(MIN(b.title), ''), COALESCE(MIN(b.creator), '')
         FROM custom_maps c LEFT JOIN beatmaps b ON b.file_md5 = c.file_md5
         GROUP BY CASE WHEN c.beatmapset_id > 0 THEN c.beatmapset_id ELSE -c.rowid END, c.status
         ORDER BY MAX(c.updated_at) DESC",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| {
            let status: i64 = r.get(3)?;
            Ok(json!({
                "set_id": r.get::<_, i64>(0)?, "beatmap_id": r.get::<_, i64>(1)?, "md5": r.get::<_, String>(2)?,
                "status": if status == 4 { "loved" } else { "ranked" }, "maps": r.get::<_, i64>(4)?, "updated": r.get::<_, i64>(5)?,
                "artist": r.get::<_, String>(6)?, "title": r.get::<_, String>(7)?, "creator": r.get::<_, String>(8)?,
            }))
        }) {
            list = rows.filter_map(|r| r.ok()).collect();
        }
    }
    ok(json!({ "maps": list }))
}

#[derive(Deserialize)]
struct MapSet {
    #[serde(default)]
    set_id: i64,
    #[serde(default)]
    beatmap_id: i64,
    #[serde(default)]
    md5: String,
    status: String,
}

async fn map_set(state: SharedState, body: &[u8]) -> Response {
    let p: MapSet = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("Invalid request: {}", e)),
    };
    let status = match p.status.as_str() {
        "ranked" => 1,
        "loved" => 4,
        "unranked" => 0,
        _ => return err(StatusCode::BAD_REQUEST, "Status must be ranked, loved or unranked."),
    };
    if p.set_id <= 0 && p.beatmap_id <= 0 && p.md5.is_empty() {
        return err(StatusCode::BAD_REQUEST, "Enter a beatmap set ID or beatmap ID.");
    }
    let st = state.read().await;
    let conn = st.db.lock().await;
    let mut set_id = p.set_id;
    if set_id <= 0 && p.beatmap_id > 0 {
        if let Ok(Some(b)) = db::get_beatmap_by_id(&conn, p.beatmap_id) {
            set_id = b.beatmapset_id;
        }
    }
    let changed = if set_id > 0 {
        db::set_beatmapset_status(&conn, set_id, status).unwrap_or(0)
    } else if p.beatmap_id > 0 {
        db::set_beatmap_status(&conn, p.beatmap_id, status).unwrap_or(0)
    } else {
        db::set_beatmap_status_by_md5(&conn, &p.md5, status).unwrap_or(0)
    };
    if changed == 0 {
        return err(StatusCode::NOT_FOUND, "That map is not in the local database yet. Open or select it in osu! once, then try again.");
    }
    crate::logger::info(&format!("[admin] set map status to {} (set {}, map {})", p.status, set_id, p.beatmap_id));
    ok(json!({ "ok": true, "changed": changed }))
}

async fn keys(state: SharedState) -> Response {
    let st = state.read().await;
    let c = &st.config;
    let ladder = crate::ladder::info().map(|i| json!({ "first_date": i.first_date, "last_date": i.last_date, "source": i.source, "auto_refresh": i.auto_refresh }));
    ok(json!({
        "v1": mask(&c.osu_api_key),
        "daily": mask(&c.osu_daily_api_key),
        "client_id": c.osu_client_id.clone().unwrap_or_default(),
        "client_secret": mask(&c.osu_client_secret),
        "ladder": ladder,
    }))
}

#[derive(Deserialize, Default)]
struct KeysPayload {
    v1: Option<String>,
    daily: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    #[serde(default)]
    clear: Vec<String>,
}

async fn keys_save(state: SharedState, body: &[u8]) -> Response {
    let p: KeysPayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("Invalid request: {}", e)),
    };
    let clean = |v: &Option<String>| v.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    let clear = |n: &str| p.clear.iter().any(|c| c == n);

    let mut st = state.write().await;
    let mut v1 = clean(&p.v1);
    let mut daily = clean(&p.daily);
    let mut cid = clean(&p.client_id);
    let mut csec = clean(&p.client_secret);
    if clear("v1") {
        v1 = Some(String::new());
    }
    if clear("daily") {
        daily = Some(String::new());
    }
    if clear("v2") {
        cid = Some(String::new());
        csec = Some(String::new());
    }

    if let Err(e) = crate::types::config::write_dotenv_full(None, None, None, None, None, v1.as_deref(), daily.as_deref(), cid.as_deref(), csec.as_deref(), None, None, None, None) {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Could not write .env: {}", e));
    }
    let opt = |s: String| if s.is_empty() { None } else { Some(s) };
    if let Some(v) = v1 {
        st.config.osu_api_key = opt(v);
    }
    if let Some(v) = daily {
        st.config.osu_daily_api_key = opt(v);
    }
    if let Some(v) = cid {
        st.config.osu_client_id = opt(v);
    }
    if let Some(v) = csec {
        st.config.osu_client_secret = opt(v);
    }

    let path = crate::osu_api::creds_file();
    if clear("v2") {
        let _ = std::fs::remove_file(&path);
    } else if let (Some(id), Some(sec)) = (st.config.osu_client_id.clone(), st.config.osu_client_secret.clone()) {
        if let Ok(idn) = id.trim().parse::<u64>() {
            if p.client_id.is_some() || p.client_secret.is_some() {
                let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
                let _ = std::fs::write(&path, serde_json::to_string_pretty(&json!({ "client_id": idn, "client_secret": sec })).unwrap_or_default());
            }
        }
    }
    {
        let conn = st.db.lock().await;
        let _ = db::save_config(&conn, &st.config);
    }
    crate::logger::info("[admin] API keys updated");
    drop(st);
    keys(state).await
}

async fn ladder_refresh(state: SharedState) -> Response {
    let http = state.read().await.http.clone();
    match crate::ladder::refresh(&http).await {
        Ok(last) => {
            crate::logger::success(&format!("[admin] rank ladder dataset refreshed (data up to {})", last));
            ok(json!({ "ok": true, "last_date": last }))
        }
        Err(e) => err(StatusCode::BAD_GATEWAY, &format!("Refresh failed: {}", e)),
    }
}

fn console_feed(params: &HashMap<String, String>) -> Response {
    let after = params.get("after").and_then(|a| a.parse::<u64>().ok()).unwrap_or(0);
    let mut out = serde_json::Map::new();
    for chan in console::CHANNELS {
        if params.get("chan").map(|c| c == chan).unwrap_or(true) {
            out.insert(chan.to_string(), json!(console::since(chan, after)));
        }
    }
    ok(Value::Object(out))
}

async fn other() -> Response {
    let themes: Vec<Value> = crate::server::assets::list_themes().into_iter().map(|(id, name, desc)| json!({ "id": id, "name": name, "description": desc })).collect();
    ok(json!({ "theme": crate::server::assets::active_theme(None), "themes": themes }))
}

#[derive(Deserialize)]
struct ThemePayload {
    theme: String,
}

fn set_theme(body: &[u8]) -> Response {
    let p: ThemePayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("Invalid request: {}", e)),
    };
    let id = p.theme.trim().to_lowercase();
    if !crate::server::assets::list_themes().iter().any(|(t, _, _)| *t == id) {
        return err(StatusCode::BAD_REQUEST, "Unknown theme.");
    }
    std::env::set_var("SITE_THEME", &id);
    if let Err(e) = crate::types::config::write_dotenv_full(None, None, None, None, None, None, None, None, None, None, None, None, None) {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Could not write .env: {}", e));
    }
    crate::logger::info(&format!("[admin] default site theme is now {}", id));
    ok(json!({ "ok": true, "theme": id }))
}

async fn setup_get(state: SharedState) -> Response {
    let st = state.read().await;
    let conn = st.db.lock().await;
    let v = db::get_setup_state(&conn).unwrap_or_else(|| json!({ "active": false, "step": 1 }));
    let users: i64 = conn.query_row("SELECT COUNT(*) FROM profiles", [], |r| r.get(0)).unwrap_or(0);
    let port = std::env::var("SERVER_PORT").ok().and_then(|p| p.parse::<u16>().ok()).unwrap_or(5000);
    ok(json!({
        "active": v.get("active").and_then(|a| a.as_bool()).unwrap_or(false),
        "step": v.get("step").and_then(|a| a.as_i64()).unwrap_or(1),
        "users": users,
        "port": port,
        "windows": cfg!(target_os = "windows"),
        "hosts_ready": crate::server::tls::hosts_ready(),
    }))
}

#[derive(Deserialize, Default)]
struct SetupPayload {
    step: Option<i64>,
    action: Option<String>,
}

async fn setup_post(state: SharedState, body: &[u8]) -> Response {
    let p: SetupPayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("Invalid request: {}", e)),
    };
    {
        let st = state.read().await;
        let conn = st.db.lock().await;
        let cur = db::get_setup_state(&conn).and_then(|v| v.get("step").and_then(|s| s.as_i64())).unwrap_or(1);
        let res = match p.action.as_deref() {
            Some("finish") => db::set_setup_state(&conn, false, 4),
            Some("restart") => db::set_setup_state(&conn, true, 1),
            _ => db::set_setup_state(&conn, true, p.step.unwrap_or(cur).clamp(1, 4)),
        };
        if let Err(e) = res {
            return err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Could not save setup state: {}", e));
        }
    }
    setup_get(state).await
}

async fn user_create(state: SharedState, body: &[u8]) -> Response {
    #[derive(Deserialize)]
    struct P {
        name: String,
    }
    let p: P = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("Invalid request: {}", e)),
    };
    let name = match crate::handlers::website::validate_username(&p.name) {
        Ok(n) => n,
        Err(e) => return err(StatusCode::BAD_REQUEST, e),
    };
    let st = state.read().await;
    let conn = st.db.lock().await;
    if let Err(e) = db::ensure_profile(&conn, &name) {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Could not create account: {}", e));
    }
    crate::logger::info(&format!("[admin] account '{}' created", name));
    ok(json!({ "ok": true, "name": name }))
}

async fn trust_cert() -> Response {
    let data_dir = std::env::current_dir().unwrap_or_default().join(".data");
    match tokio::task::spawn_blocking(move || crate::types::config::install_local_trust(&data_dir)).await {
        Ok(Ok(m)) => ok(json!({ "ok": true, "message": m })),
        Ok(Err(e)) => err(StatusCode::BAD_REQUEST, &e),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn keys_test(state: SharedState) -> Response {
    let (http, v1, daily, cid, csec) = {
        let st = state.read().await;
        (st.http.clone(), st.config.osu_api_key.clone(), st.config.osu_daily_api_key.clone(), st.config.osu_client_id.clone(), st.config.osu_client_secret.clone())
    };
    let timeout = std::time::Duration::from_secs(10);
    let nz = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    let r_v1 = match nz(v1) {
        None => json!({ "state": "unset", "message": "Not set" }),
        Some(k) => match http.get(format!("https://osu.ppy.sh/api/get_user?k={}&u=peppy", k)).timeout(timeout).send_traced().await {
            Err(_) => json!({ "state": "error", "message": "Could not reach osu.ppy.sh" }),
            Ok(r) => match r.json::<Value>().await {
                Ok(Value::Array(a)) if !a.is_empty() => json!({ "state": "ok", "message": "Key works" }),
                _ => json!({ "state": "fail", "message": "The server rejected this key" }),
            },
        },
    };

    let r_v2 = match (nz(cid), nz(csec)) {
        (Some(id), Some(sec)) => match id.parse::<u64>() {
            Err(_) => json!({ "state": "fail", "message": "Client ID must be a number" }),
            Ok(id) => {
                let body = json!({ "client_id": id, "client_secret": sec, "grant_type": "client_credentials", "scope": "public" });
                match http.post("https://osu.ppy.sh/oauth/token").json(&body).timeout(timeout).send_traced().await {
                    Err(_) => json!({ "state": "error", "message": "Could not reach osu.ppy.sh" }),
                    Ok(r) => match r.json::<Value>().await {
                        Ok(v) if v.get("access_token").is_some() => json!({ "state": "ok", "message": "Credentials work" }),
                        _ => json!({ "state": "fail", "message": "The server rejected these credentials" }),
                    },
                }
            }
        },
        (None, None) => json!({ "state": "unset", "message": "Not set" }),
        _ => json!({ "state": "fail", "message": "Both Client ID and Client Secret are needed" }),
    };

    let r_daily = match nz(daily) {
        None => json!({ "state": "unset", "message": "Not set" }),
        Some(k) => match http.get(format!("https://osudaily.net/api/pp.php?k={}&t=pp&v=100&m=0", k)).timeout(timeout).send_traced().await {
            Err(_) => json!({ "state": "error", "message": "Could not reach osudaily.net" }),
            Ok(r) => match r.json::<Value>().await {
                Ok(v) if v.get("error").is_none() && v.get("rank").is_some() => json!({ "state": "ok", "message": "Key works" }),
                _ => json!({ "state": "fail", "message": "The server rejected this key" }),
            },
        },
    };
    ok(json!({ "v1": r_v1, "v2": r_v2, "daily": r_daily }))
}

async fn settings(state: SharedState) -> Response {
    let st = state.read().await;
    let c = &st.config;
    let s = |o: &Option<String>| o.clone().unwrap_or_default();
    ok(json!({
        "pp_leaderboard": c.pp_leaderboard,
        "show_pp_for_personal_best": c.show_pp_for_personal_best,
        "amount_of_scores_on_lb": c.amount_of_scores_on_lb,
        "enable_recent_channel": c.enable_recent_channel,
        "ping_user_when_recent_score": c.ping_user_when_recent_score,
        "command_prefix": c.command_prefix,
        "disable_funorange_maps": c.disable_funorange_maps,
        "auto_update": c.auto_update,
        "osu_path": s(&c.paths.osu_path),
        "songs": s(&c.paths.songs),
        "replay": s(&c.paths.replay),
        "screenshots": s(&c.paths.screenshots),
        "country": s(&c.country),
        "osu_username": s(&c.osu_username),
        "osu_password_set": c.osu_password.as_deref().map(|p| !p.is_empty()).unwrap_or(false),
        "discord_webhook_url": s(&c.discord_webhook_url),
        "discord_webhook_min_pp": c.discord_webhook_min_pp,
        "discord_bot_token": mask(&c.discord_bot_token),
        "discord_application_id": s(&c.discord_application_id),
        "discord_guild_id": s(&c.discord_guild_id),
        "host": std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        "port": std::env::var("SERVER_PORT").ok().and_then(|p| p.parse::<u16>().ok()).unwrap_or(5000),
        "windows": cfg!(target_os = "windows"),
    }))
}

#[derive(Deserialize, Default)]
struct SettingsPayload {
    pp_leaderboard: Option<bool>,
    show_pp_for_personal_best: Option<bool>,
    amount_of_scores_on_lb: Option<i32>,
    enable_recent_channel: Option<bool>,
    ping_user_when_recent_score: Option<bool>,
    command_prefix: Option<String>,
    disable_funorange_maps: Option<bool>,
    auto_update: Option<bool>,
    osu_path: Option<String>,
    songs: Option<String>,
    replay: Option<String>,
    screenshots: Option<String>,
    country: Option<String>,
    osu_username: Option<String>,
    osu_password: Option<String>,
    discord_webhook_url: Option<String>,
    discord_webhook_min_pp: Option<String>,
    discord_bot_token: Option<String>,
    discord_application_id: Option<String>,
    discord_guild_id: Option<String>,
    host: Option<String>,
    port: Option<String>,
}

async fn settings_save(state: SharedState, body: &[u8]) -> Response {
    use md5::Digest;
    let p: SettingsPayload = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => return err(StatusCode::BAD_REQUEST, &format!("Invalid request: {}", e)),
    };
    if let Some(n) = p.amount_of_scores_on_lb {
        if !(1..=100).contains(&n) {
            return err(StatusCode::BAD_REQUEST, "Leaderboard size must be between 1 and 100.");
        }
    }
    if let Some(pref) = &p.command_prefix {
        let t = pref.trim();
        if t.is_empty() || t.chars().count() > 3 || t.chars().any(|c| c.is_whitespace()) {
            return err(StatusCode::BAD_REQUEST, "Command prefix must be 1–3 characters without spaces.");
        }
    }
    let country = match p.country.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(c) => match crate::types::config::parse_country_code(c) {
            Some(code) => Some(code),
            None => return err(StatusCode::BAD_REQUEST, "Country must be a 2-letter code like US, DE or JP."),
        },
    };
    let min_pp = match p.discord_webhook_min_pp.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(v) => match v.parse::<f64>() {
            Ok(n) if n >= 0.0 => Some(n),
            _ => return err(StatusCode::BAD_REQUEST, "Minimum pp must be a number."),
        },
    };
    if let Some(port) = &p.port {
        if crate::types::config::parse_port_input(port, 5000).is_none() {
            return err(StatusCode::BAD_REQUEST, "Port must be 1–65535.");
        }
    }
    if let Some(h) = &p.host {
        if h.trim().parse::<std::net::IpAddr>().is_err() {
            return err(StatusCode::BAD_REQUEST, "Host must be an IP address like 127.0.0.1.");
        }
    }

    let opt = |s: &str| {
        let t = s.trim();
        if t.is_empty() { None } else { Some(t.to_string()) }
    };
    let mut env: Vec<(&str, String)> = Vec::new();
    let mut st = state.write().await;
    let mut restart = false;

    if let Some(v) = p.pp_leaderboard { st.config.pp_leaderboard = v; env.push(("PP_LEADERBOARD", v.to_string())); }
    if let Some(v) = p.show_pp_for_personal_best { st.config.show_pp_for_personal_best = v; env.push(("SHOW_PP_FOR_PERSONAL_BEST", v.to_string())); }
    if let Some(v) = p.amount_of_scores_on_lb { st.config.amount_of_scores_on_lb = v; env.push(("AMOUNT_OF_SCORES_ON_LB", v.to_string())); }
    if let Some(v) = p.enable_recent_channel { st.config.enable_recent_channel = v; env.push(("ENABLE_RECENT_CHANNEL", v.to_string())); }
    if let Some(v) = p.ping_user_when_recent_score { st.config.ping_user_when_recent_score = v; }
    if let Some(v) = &p.command_prefix { st.config.command_prefix = v.trim().to_string(); }
    if let Some(v) = p.disable_funorange_maps { st.config.disable_funorange_maps = v; }
    if let Some(v) = p.auto_update { st.config.auto_update = v; }

    if let Some(v) = &p.osu_path { st.config.paths.osu_path = opt(v); restart = true; }
    if let Some(v) = &p.songs { st.config.paths.songs = opt(v); restart = true; }
    if let Some(v) = &p.replay { st.config.paths.replay = opt(v); restart = true; }
    if let Some(v) = &p.screenshots { st.config.paths.screenshots = opt(v); restart = true; }

    if p.country.is_some() {
        st.config.country = country.clone();
        env.push(("COUNTRY", country.unwrap_or_default()));
    }
    if let Some(v) = &p.osu_username {
        st.config.osu_username = opt(v);
        env.push(("OSU_USERNAME", v.trim().to_string()));
    }
    if let Some(v) = &p.osu_password {
        let t = v.trim();
        let hash = if t.is_empty() { String::new() } else { format!("{:x}", md5::Md5::digest(t.as_bytes())) };
        st.config.osu_password = opt(&hash);
        env.push(("OSU_PASSWORD_HASH", hash));
    }
    if let Some(v) = &p.discord_webhook_url {
        st.config.discord_webhook_url = opt(v);
        env.push(("DISCORD_WEBHOOK_URL", v.trim().to_string()));
    }
    if p.discord_webhook_min_pp.is_some() {
        st.config.discord_webhook_min_pp = min_pp;
        env.push(("DISCORD_WEBHOOK_MIN_PP", min_pp.map(|n| n.to_string()).unwrap_or_default()));
    }
    if let Some(v) = &p.discord_bot_token {
        st.config.discord_bot_token = opt(v);
        env.push(("DISCORD_BOT_TOKEN", v.trim().to_string()));
        restart = true;
    }
    if let Some(v) = &p.discord_application_id { st.config.discord_application_id = opt(v); env.push(("DISCORD_APPLICATION_ID", v.trim().to_string())); restart = true; }
    if let Some(v) = &p.discord_guild_id { st.config.discord_guild_id = opt(v); env.push(("DISCORD_GUILD_ID", v.trim().to_string())); restart = true; }
    if let Some(v) = &p.host { env.push(("SERVER_HOST", v.trim().to_string())); restart = true; }
    if let Some(v) = &p.port { env.push(("SERVER_PORT", v.trim().to_string())); restart = true; }

    if let Err(e) = crate::types::config::update_env_file(&env) {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Could not write .env: {}", e));
    }
    {
        let conn = st.db.lock().await;
        let _ = db::save_config(&conn, &st.config);
    }
    crate::logger::info("[admin] settings updated");
    ok(json!({ "ok": true, "restart_needed": restart }))
}
