use crate::console::TraceSend;
use crate::db;
use crate::packets;
use crate::server::response::Response;
use crate::state::AppState;
use crate::types::beatmap::Beatmap;
use crate::types::direct_response::DirectResponse;
use crate::types::leaderboard::*;
use crate::types::mods::Mods;
use crate::types::score::{LeaderboardEntry, Score};
use crate::utils;
use std::sync::Arc;
use tokio::sync::RwLock;

const DEFAULT_CHARTS: &str = concat!(
    "beatmapId:0|beatmapSetId:0|beatmapPlaycount:0|beatmapPasscount:0|approvedDate:0\n",
    "chartId:beatmap|chartUrl:https://osu.ppy.sh/b/0|chartName:Beatmap Ranking|rankBefore:|rankAfter:0|maxComboBefore:|maxComboAfter:0|accuracyBefore:|accuracyAfter:0|rankedScoreBefore:|rankedScoreAfter:0|ppBefore:|ppAfter:0|onlineScoreId:0\n",
    "chartId:overall|chartUrl:https://osu.ppy.sh/u/2|chartName:Overall Ranking|rankBefore:0|rankAfter:0|rankedScoreBefore:0|rankedScoreAfter:0|totalScoreBefore:0|totalScoreAfter:0|maxComboBefore:0|maxComboAfter:0|accuracyBefore:0|accuracyAfter:0|ppBefore:0|ppAfter:0|achievements-new:|onlineScoreId:0",
);

pub async fn handle(
    state: Arc<RwLock<AppState>>,
    sub_path: &str,
    params: &std::collections::HashMap<String, String>,
    method: &hyper::Method,
    headers: &hyper::HeaderMap,
    body: &[u8],
) -> Response {
    match sub_path {
        "/bancho_connect.php" => crate::handlers::cho::bancho_connect(state, params).await,
        "/osu-submit-modular-selector.php" | "/osu-submit-modular.php" => {
            utils::log(&format!("[submit] endpoint {}", sub_path));
            score_sub(state, method, headers, body).await
        }
        "/osu-submit.php" | "/osu-submit-new.php" => {
            utils::log(&format!("[submit] endpoint {}", sub_path));
            let mut resp = score_sub(state, method, headers, body).await;
            resp.body = plain_submit_reply(&resp.body);
            resp
        }
        "/osu-error.php" => {
            utils::log_error(&format!("[client-error] {}", client_error_summary(headers, body)));
            Response::empty()
        }
        "/check-updates.php" => Response::new(b"[]".to_vec()),
        "/release/update.php" => Response::new(b"0".to_vec()),
        "/release/update" | "/release/update2.php" | "/release/patches.php" | "/release/filter.txt" => Response::empty(),
        "/osu-getseasonal.php" => get_seasonal_bgs(state).await,
        "/osu-getreplay.php" => get_replay(state, params).await,
        "/osu-osz2-getscores.php" => osz2_leaderboard(state, params).await,
        "/osu-getscores6.php" | "/osu-getscores5.php" | "/osu-getscores4.php" => {
            utils::log(&format!("[leaderboard] endpoint {}", sub_path));
            legacy_leaderboard(state, params).await
        }
        "/rating/ingame-rate.php" | "/rating/ingame-rate2.php" | "/osu-rate.php" => {
            let voted = params.get("v").map_or(false, |v| !v.is_empty());
            let body = match (sub_path, voted) {
                (_, false) => "ok",
                ("/osu-rate.php", true) => "ok\n0.00",
                _ => "0.00",
            };
            utils::log(&format!("[rating] {} v={:?} -> {}", sub_path, params.get("v"), body.replace('\n', "\\n")));
            Response::new(body.as_bytes().to_vec())
        }
        "/osu-search.php" => direct_search(state, params).await,
        "/osu-getfriends.php" => get_friends(state, params).await,
        "/lastfm.php"
        | "/osu-session.php"
        | "/difficulty-rating"
        | "/osu-markasread.php"
        | "/osu-getbeatmapinfo.php"
        | "/osu-screenshot.php" => Response::empty(),
        other => {
            utils::log_success(&format!("[web] unhandled request: {}", other));
            Response::empty()
        }
    }
}

fn client_error_summary(headers: &hyper::HeaderMap, body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body).to_string();
    let content_type = headers.get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");
    let mut parts: Vec<String> = Vec::new();
    for field in ["exception", "stacktrace", "feedback", "version"] {
        let value = if content_type.starts_with("multipart/") {
            let marker = format!("name=\"{}\"", field);
            text.find(&marker).map(|i| {
                let rest = &text[i + marker.len()..];
                let rest = rest.trim_start_matches(['\r', '\n']);
                let end = rest.find("\r\n--").unwrap_or(rest.len());
                rest[..end].to_string()
            })
        } else {
            utils::parse_query_string(&text).get(field).cloned()
        };
        if let Some(v) = value {
            let v = v.replace(['\r', '\n'], " | ");
            if !v.trim().is_empty() {
                parts.push(format!("{}: {}", field, v.chars().take(1500).collect::<String>()));
            }
        }
    }
    if parts.is_empty() {
        format!("unrecognised report ({} bytes, {})", body.len(), content_type)
    } else {
        parts.join(" // ")
    }
}

fn plain_submit_reply(charts: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(charts);
    let overall = match text.lines().nth(1) {
        Some(l) if l.contains("chartId:") => l,
        _ => return charts.to_vec(),
    };
    let field = |name: &str| -> String {
        overall
            .split('|')
            .find_map(|kv| kv.strip_prefix(name).and_then(|r| r.strip_prefix(':')))
            .unwrap_or("")
            .to_string()
    };
    let rank = field("beatmapRankingAfter");
    let rank = if rank.is_empty() { "0".to_string() } else { rank };
    let mut lines = vec![rank, "0".to_string()];
    let ach = field("achievements");
    if !ach.is_empty() {
        lines.push(ach);
    }
    lines.join("\n").into_bytes()
}

async fn score_sub(state: Arc<RwLock<AppState>>, method: &hyper::Method, headers: &hyper::HeaderMap, body: &[u8]) -> Response {
    {
        let s = state.read().await;
        if s.player.is_none() {
            return Response::empty().with_status(hyper::StatusCode::NOT_FOUND);
        }
        if s.player.as_ref().map_or(false, |p| p.is_restricted) {
            return Response::new(b"error: ban\n".to_vec());
        }
    }

    if *method != hyper::Method::POST {
        let mut s = state.write().await;
        if let Some(ref mut p) = s.player {
            p.queue.extend_from_slice(&packets::notification("Score submission requires POST (-devserver localhost)."));
        }
        return Response::new(b"error: no".to_vec());
    }

    let content_type = headers.get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");

    let decoded = match crate::handlers::score_decode::parse_submission(content_type, body) {
        Ok(d) => d,
        Err(e) => {
            utils::log_error(&format!("Failed to parse/decrypt score submission: {}", e));
            return Response::new(b"error: no".to_vec());
        }
    };

    let state_for_build = state.clone();
    match crate::handlers::score_submit::process_native_submission(state, decoded).await {
        Ok(charts) => {
            let build = state_for_build.read().await.client_build;
            let charts = crate::handlers::score_submit::adapt_accuracy_for_build(charts, build);
            let charts = crate::handlers::score_submit::fill_empty_numbers_for_build(charts, build);
            utils::log(&format!("[submit] reply: {}", String::from_utf8_lossy(&charts).replace('\n', " // ")));
            Response::new(charts)
        }
        Err(e) => {
            utils::log_error(&format!("Failed to process score submission: {}", e));
            Response::new(b"error: no".to_vec())
        }
    }
}

async fn get_seasonal_bgs(state: Arc<RwLock<AppState>>) -> Response {
    let s = state.read().await;
    if s.config.seasonal_bgs.is_empty() {
        Response::new(b"[\"\"]".to_vec())
    } else {
        let json = serde_json::to_vec(&s.config.seasonal_bgs).unwrap_or(b"[\"\"]".to_vec());
        Response::new(json)
    }
}

async fn get_replay(state: Arc<RwLock<AppState>>, params: &std::collections::HashMap<String, String>) -> Response {
    let scoreid: i64 = real_score_id(params.get("c").and_then(|v| v.parse().ok()).unwrap_or(0));
    let mode: i32 = params.get("m").and_then(|v| v.parse().ok()).unwrap_or(0);

    if scoreid > 0 {
        let (api_key_opt, http) = {
            let s = state.read().await;
            (s.config.osu_api_key.clone(), s.http.clone())
        };
        if let Some(ref api_key) = api_key_opt {
            let url = format!("https://osu.ppy.sh/api/get_replay?k={}&s={}&m={}", api_key, scoreid, mode);
            if let Ok(resp) = http.get(&url).send_traced().await {
                if resp.status() == 200 {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(content) = json.get("content").and_then(|v| v.as_str()) {
                            if let Ok(frames) = utils::string_to_bytes(content) {
                                return Response::new(frames);
                            }
                        }
                    }
                }
            }
        }
        return Response::new(b"error: no".to_vec());
    }

    if scoreid < 0 {
        let real_id = scoreid.unsigned_abs() as i64;
        let s = state.read().await;
        let db_conn = s.db.lock().await;
        if let Ok(Some(score)) = db::get_score_by_id(&db_conn, real_id) {
            if let Some(ref frames_b64) = score.replay_frames {
                if let Ok(frames) = utils::string_to_bytes(frames_b64) {
                    return Response::new(frames);
                }
            }
        }
        return Response::new(b"error: no".to_vec());
    }

    Response::new(b"error: no".to_vec())
}

static SHORT_SCORE_IDS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<i64, i64>>> = std::sync::OnceLock::new();

pub(crate) fn shorten_score_id(id: i64) -> i64 {
    if id <= i32::MAX as i64 {
        return id;
    }
    let short = id % 2_000_000_000;
    if let Ok(mut m) = SHORT_SCORE_IDS.get_or_init(Default::default).lock() {
        m.insert(short, id);
    }
    short
}

fn real_score_id(id: i64) -> i64 {
    SHORT_SCORE_IDS
        .get()
        .and_then(|m| m.lock().ok().and_then(|m| m.get(&id).copied()))
        .unwrap_or(id)
}

fn request_v1_text(modern: &str) -> String {
    let mut lines: Vec<String> = modern.split('\n').map(|l| l.to_string()).collect();
    if let Some(first) = lines.first_mut() {
        let mut f: Vec<String> = first.split('|').map(|x| x.to_string()).collect();
        if let Some(code) = f.first().and_then(|v| v.parse::<i32>().ok()) {
            f[0] = match code {
                2 => 4,
                4 => 2,
                other => other,
            }
            .to_string();
        }
        if f.len() == 5 {
            f.push(String::new());
            f.push(String::new());
        }
        *first = f.join("|");
    }
    if lines.len() == 1 && lines[0].ends_with("|false") && !lines[0].starts_with('-') && !lines[0].starts_with('1') {
        lines[0] = format!("{}|0|0|0||", lines[0]);
        lines.extend(["0".to_string(), String::new(), "10.0".to_string()]);
    }
    if let Some(title) = lines.get_mut(2) {
        if let Some(rest) = title.strip_prefix("[bold:0,size:20]") {
            *title = rest.to_string();
        }
    }
    for (i, line) in lines.iter_mut().enumerate() {
        if i >= 5 && !line.is_empty() {
            let mut f: Vec<String> = line.split('|').map(|x| x.to_string()).collect();
            if f.len() >= 15 {
                if let Ok(id) = f[0].parse::<i64>() {
                    f[0] = shorten_score_id(id).to_string();
                }
                if let Ok(ts) = f[14].parse::<i64>() {
                    if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
                        f[14] = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                    }
                }
                *line = f.join("|");
            }
        } else if i == 4 && !line.is_empty() {
            let mut f: Vec<String> = line.split('|').map(|x| x.to_string()).collect();
            if f.len() >= 15 {
                if let Ok(id) = f[0].parse::<i64>() {
                    f[0] = shorten_score_id(id).to_string();
                }
                if let Ok(ts) = f[14].parse::<i64>() {
                    if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
                        f[14] = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                    }
                }
                *line = f.join("|");
            }
        }
    }
    lines.join("\n")
}

const OLD_CLIENT_STAR_RATING: &str = "9.4";

fn ppv1_star_cache() -> &'static std::sync::Mutex<std::collections::HashMap<String, String>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, String>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

async fn ppv1_star_text(state: &Arc<RwLock<AppState>>, md5: &str) -> Option<String> {
    if let Some(v) = ppv1_star_cache().lock().ok().and_then(|c| c.get(md5).cloned()) {
        return Some(v);
    }
    let (http, db_handle, config) = {
        let s = state.read().await;
        (s.http.clone(), s.db.clone(), s.config.clone())
    };
    let bmap = {
        let conn = db_handle.lock().await;
        db::get_beatmap_by_md5(&conn, md5).ok().flatten()
    }?;
    let content = utils::get_or_fetch_beatmap_content(&http, &db_handle, &config, &bmap).await?;
    let map = crate::pp_era::EraMap::parse(content.as_bytes())?.with_beatmap(&bmap);
    let stars = map.ppv1_stars();
    if stars <= 0.0 {
        return None;
    }
    let text = format!("{:.2}", stars);
    if let Ok(mut c) = ppv1_star_cache().lock() {
        c.insert(md5.to_string(), text.clone());
    }
    Some(text)
}

fn log_leaderboard_request(kind: &str, params: &std::collections::HashMap<String, String>, body: &[u8]) {
    let get = |k: &str| params.get(k).map(|v| v.as_str()).unwrap_or("-");
    let head = String::from_utf8_lossy(body).lines().next().unwrap_or("").chars().take(60).collect::<String>();
    utils::log(&format!(
        "[leaderboard:{}] c={} f={} i={} m={} v={} vv={} -> {} ({} bytes)",
        kind, get("c"), get("f"), get("i"), get("m"), get("v"), get("vv"), head, body.len()
    ));
}

async fn apply_star_mode(state: &Arc<RwLock<AppState>>, md5: Option<String>, mut out: String) -> String {
    let mode = state.read().await.old_star_rating;
    let value = match mode {
        crate::state::StarMode::Fixed => Some(OLD_CLIENT_STAR_RATING.to_string()),
        crate::state::StarMode::Modern => match md5 {
            Some(md5) => {
                let s = state.read().await;
                let db = s.db.lock().await;
                db::get_beatmap_by_md5(&db, &md5).ok().flatten().map(|b| b.difficultyrating).filter(|v| *v > 0.0).map(|v| format!("{:.2}", v))
            }
            None => None,
        },
        crate::state::StarMode::Ppv1 => match md5 {
            Some(md5) => ppv1_star_text(state, &md5).await,
            None => None,
        },
    };
    if let Some(value) = value {
        let mut lines: Vec<String> = out.split('\n').map(|l| l.to_string()).collect();
        if lines.len() >= 4 {
            lines[3] = value;
            out = lines.join("\n");
        }
    }
    out
}

async fn osz2_leaderboard(state: Arc<RwLock<AppState>>, params: &std::collections::HashMap<String, String>) -> Response {
    let version: i32 = match params.get("vv").and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None if params.contains_key("p") => 1,
        None => 4,
    };
    let resp = leaderboard(state.clone(), params).await;
    log_leaderboard_request("osz2", params, &resp.body);
    if version > 1 {
        return resp;
    }
    let text = String::from_utf8_lossy(&resp.body).to_string();
    let out = apply_star_mode(&state, params.get("c").cloned(), request_v1_text(&text)).await;
    Response::new(out.into_bytes())
}

fn legacy_leaderboard_text(modern: &str) -> String {
    let mut lines = modern.split('\n');
    let first = lines.next().unwrap_or("");
    let mut head = first.split('|');
    let status: i32 = head.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let rest: Vec<&str> = lines.collect();
    if rest.is_empty() {
        return status.to_string();
    }
    let mut out: Vec<String> = Vec::with_capacity(rest.len() + 1);
    out.push(status.to_string());
    for (i, line) in rest.iter().enumerate() {
        if i >= 3 && !line.is_empty() {
            let mut f: Vec<String> = line.split('|').map(|x| x.to_string()).collect();
            if let Some(id) = f.first().and_then(|v| v.parse::<i64>().ok()) {
                f[0] = shorten_score_id(id).to_string();
            }
            if f.len() >= 15 {
                if let Ok(ts) = f[14].parse::<i64>() {
                    if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
                        f[14] = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                    }
                }
                f.truncate(15);
            }
            out.push(f.join("|"));
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\n")
}

async fn legacy_leaderboard(state: Arc<RwLock<AppState>>, params: &std::collections::HashMap<String, String>) -> Response {
    let mut p = params.clone();
    p.remove("u");
    p.entry("v".to_string()).or_insert_with(|| "1".to_string());
    let resp = leaderboard(state.clone(), &p).await;
    log_leaderboard_request("legacy", params, &resp.body);
    let text = String::from_utf8_lossy(&resp.body).to_string();
    let out = apply_star_mode(&state, params.get("c").cloned(), legacy_leaderboard_text(&text)).await;
    for (i, line) in out.lines().take(6).enumerate() {
        utils::log(&format!("[leaderboard:legacy] reply line {}: {}", i, line));
    }
    Response::new(out.into_bytes())
}

async fn leaderboard(state: Arc<RwLock<AppState>>, params: &std::collections::HashMap<String, String>) -> Response {
    let username_from_params = params.get("us").cloned().or_else(|| params.get("u").cloned());
    if let Some(ref u) = username_from_params {
        if !u.is_empty() {
            let mut s = state.write().await;
            if s.player.is_none() && s.pending_login_name.is_none() {
                s.pending_login_name = Some(u.clone());
            }
        }
    }

    if state.read().await.player.is_none() {
        for _ in 0..5 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if state.read().await.player.is_some() {
                break;
            }
        }
    }

    let mods_val: u32 = params.get("mods").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mode: i32 = params.get("m").and_then(|v| v.parse().ok()).unwrap_or(0);
    let rank_type = LeaderboardTypes::from_i32(params.get("v").and_then(|v| v.parse().ok()).unwrap_or(1));
    let md5 = params.get("c").cloned().unwrap_or_default();
    let setid: i64 = params.get("i").and_then(|v| v.parse().ok()).unwrap_or(0);

    let valid_rank_types = [LeaderboardTypes::Local, LeaderboardTypes::Top, LeaderboardTypes::Mods, LeaderboardTypes::Friends, LeaderboardTypes::Country];
    if !(0..=3).contains(&mode) || !valid_rank_types.contains(&rank_type) {
        return Response::text("0|false");
    }

    let s_read = state.read().await;
    let player_name = s_read
        .player
        .as_ref()
        .map(|p| p.name.clone())
        .or_else(|| username_from_params.clone())
        .or_else(|| s_read.pending_login_name.clone())
        .or_else(|| s_read.config.osu_username.clone())
        .unwrap_or_default();
    let pp_leaderboard = s_read.config.pp_leaderboard;
    let show_pp_pb = s_read.config.show_pp_for_personal_best;
    let amount = s_read.config.amount_of_scores_on_lb;
    let prev_game_mode = s_read.player.as_ref().map(|p| p.mode).unwrap_or(mode as u8);
    drop(s_read);

    let target_game_mode = mode as u8;
    let mods = Mods::from_bits_truncate(mods_val);
    let mut stats_need_update = false;

    {
        let mut s = state.write().await;
        if let Some(ref mut p) = s.player {
            if p.mode != target_game_mode {
                p.mode = target_game_mode;
                stats_need_update = true;
            }
        }

        if mods.contains(Mods::RELAX) && s.mode != Some(Mods::RELAX) {
            s.mode = Some(Mods::RELAX);
            s.invalid_mods = Mods::INVALID_RELAX;
            stats_need_update = true;
            if let Some(ref mut p) = s.player {
                p.queue.extend_from_slice(&packets::notification("Switched to Relax mode."));
            }
        } else if mods.contains(Mods::AUTOPILOT) && s.mode != Some(Mods::AUTOPILOT) {
            s.mode = Some(Mods::AUTOPILOT);
            s.invalid_mods = Mods::INVALID_AUTOPILOT;
            stats_need_update = true;
            if let Some(ref mut p) = s.player {
                p.queue.extend_from_slice(&packets::notification("Switched to Autopilot mode."));
            }
        } else if !mods.intersects(Mods::RELAX | Mods::AUTOPILOT) && s.mode.is_some() {
            s.mode = None;
            s.invalid_mods = Mods::INVALID_STANDARD;
            stats_need_update = true;
            if let Some(ref mut p) = s.player {
                p.queue.extend_from_slice(&packets::notification("Switched to Standard mode."));
            }
        }
    }

    if !player_name.is_empty() && (stats_need_update || target_game_mode != prev_game_mode) {
        let s = state.read().await;
        let db_conn = s.db.lock().await;
        let scores = db::get_ranked_scores(&db_conn, &player_name).unwrap_or_default();
        let playcount = db::get_playcount(&db_conn, &player_name).unwrap_or(0);
        let filter_mod = s.mode;
        let api_key = s.config.osu_daily_api_key.clone();
        let http = s.http.clone();
        drop(db_conn);
        drop(s);

        let mut s_write = state.write().await;
        if let Some(ref mut p) = s_write.player {
            if !p.is_restricted {
                p.calculate_stats(&scores, filter_mod, playcount);
                let pp = p.pp_exact;
                let current_p_mode = p.mode;
                drop(s_write);

                let new_rank = match api_key {
                    Some(key) => tokio::time::timeout(std::time::Duration::from_secs(4), utils::get_rank_from_daily(&http, &key, pp, current_p_mode))
                        .await
                        .ok()
                        .flatten(),
                    None => None,
                };
                let mut s_write = state.write().await;
                if let Some(ref mut p) = s_write.player {
                    if !p.is_restricted {
                        if let Some(r) = new_rank {
                            p.rank = r;
                        }
                        p.enqueue_stats();
                    }
                }
            } else {
                p.rank = 0;
                p.pp = 0;
                p.bancho_privs = 0;
                p.enqueue_stats();
                drop(s_write);
            }
        }
    }

    let mut bmap = {
        let s = state.read().await;
        let db_conn = s.db.lock().await;
        db::get_beatmap_by_md5(&db_conn, &md5).unwrap_or(None)
    };

    if bmap.as_ref().map_or(false, |b| b.beatmap_id == 0 && b.approved == 0) {
        bmap = None;
    }

    if bmap.is_none() && !md5.is_empty() {
        let (api_key, http, db, config) = {
            let s = state.read().await;
            (s.config.osu_api_key.clone(), s.http.clone(), s.db.clone(), s.config.clone())
        };
        let file_hint = params.get("f").map(|s| s.as_str());

        if let Some(ref key) = api_key {
            if let Some(fetched) = utils::fetch_beatmap_from_api(&http, key, &[("h", md5.clone())]).await {
                let db_conn = db.lock().await;
                let _ = db::insert_beatmap(&db_conn, &fetched);
                bmap = Some(fetched);
            }
        }
        if bmap.is_none() {
            let sid = if setid > 0 { Some(setid) } else { None };
            for songs_dir in utils::resolve_songs_folders(&config) {
                if let Some((mut local_bmap, content)) = utils::find_and_parse_local_osu_file(&songs_dir, sid, None, Some(&md5), file_hint) {
                    if local_bmap.approved <= 0 || local_bmap.beatmap_id == 0 {
                        if let Some(ref key) = api_key {
                            let lookup_set = if local_bmap.beatmapset_id > 0 { local_bmap.beatmapset_id } else { setid };
                            if lookup_set > 0 {
                                let hint = if local_bmap.version.is_empty() { file_hint.map(|s| s.to_string()) } else { Some(local_bmap.version.clone()) };
                                if let Some(api) = utils::fetch_beatmap_from_api_with_hint(&http, key, &[("s", lookup_set.to_string())], hint.as_deref()).await {
                                    local_bmap.approved = api.approved;
                                    local_bmap.approved_date = api.approved_date.clone();
                                    if local_bmap.beatmapset_id == 0 {
                                        local_bmap.beatmapset_id = api.beatmapset_id;
                                    }
                                    if local_bmap.beatmap_id == 0 && api.version.eq_ignore_ascii_case(&local_bmap.version) {
                                        local_bmap.beatmap_id = api.beatmap_id;
                                    }
                                }
                            }
                        }
                    }
                    let db_conn = db.lock().await;
                    let _ = db::insert_beatmap(&db_conn, &local_bmap);
                    let _ = db::update_beatmap_file_content(&db_conn, &local_bmap.file_md5, &content);
                    bmap = Some(local_bmap);
                    break;
                }
            }
        }
        if bmap.is_none() && setid > 0 {
            if let Some(ref key) = api_key {
                if let Some(mut api) = utils::fetch_beatmap_from_api_with_hint(&http, key, &[("s", setid.to_string())], file_hint).await {
                    api.file_md5 = md5.clone();
                    let db_conn = db.lock().await;
                    let _ = db::insert_beatmap(&db_conn, &api);
                    bmap = Some(api);
                }
            }
        }
        if bmap.is_none() {
            let mut fallback = Beatmap::blank();
            fallback.file_md5 = md5.clone();
            fallback.beatmapset_id = if setid > 0 { setid } else { 0 };
            fallback.approved = 0;
            if let Some(hint) = file_hint {
                let (artist, title, creator, version) = utils::parse_osu_filename(hint);
                fallback.artist = artist;
                fallback.title = title;
                fallback.creator = creator;
                fallback.version = version;
            }
            let db_conn = db.lock().await;
            let _ = db::insert_beatmap(&db_conn, &fallback);
            bmap = Some(fallback);
        }
    }

    let Some(bmap) = bmap else {
        if !md5.is_empty() {
            let mut s_write = state.write().await;
            if let Some(ref mut p) = s_write.player {
                p.map_md5 = md5.clone();
                p.map_id = 0;
            }
            if s_write.last_np_map.as_ref().map(|b| &b.file_md5) != Some(&md5) {
                s_write.last_np_map = None;
            }
        }
        utils::log_error(&format!("unresolved map of setid: {}", setid));
        return Response::new(b"0|false".to_vec());
    };

    let mut lb = Leaderboard::new();
    lb.bmap = Some(bmap.clone());

    {
        let mut s_write = state.write().await;
        s_write.last_np_map = Some(bmap.clone());
        if let Some(ref mut p) = s_write.player {
            p.map_id = bmap.beatmap_id as i32;
            p.map_md5 = bmap.file_md5.clone();
            p.info_text = format!("{} - {} [{}]", bmap.artist, bmap.title, bmap.version);
        }
    }

    let (current_mode, http, db, config) = {
        let s = state.read().await;
        (
            s.mode,
            s.http.clone(),
            s.db.clone(),
            s.config.clone(),
        )
    };

    let status = api_to_server_status(bmap.approved);
    if !VALID_LB_STATUSES.contains(&status) {
        utils::log_success(&format!("handled map of setid: {}", setid));
        return Response::new(lb.as_binary(amount, show_pp_pb));
    }

    let is_global = rank_type == LeaderboardTypes::Top || rank_type == LeaderboardTypes::Mods || rank_type == LeaderboardTypes::Country;

    if is_global {
        let api_key_opt = config.osu_api_key.clone();
        let mods_filter = if rank_type == LeaderboardTypes::Mods { Some(mods_val) } else { None };
        let cache_key = (bmap.beatmap_id, mode, mods_filter, amount);

        let cached_scores = {
            let s = state.read().await;
            s.bancho_score_cache.get(&cache_key).and_then(|(instant, scores)| {
                if instant.elapsed() < std::time::Duration::from_secs(180) {
                    Some(scores.clone())
                } else {
                    None
                }
            })
        };

        let bancho_scores = if let Some(cs) = cached_scores {
            Some(cs)
        } else if let Some(ref api_key) = api_key_opt {
            let fetched = utils::fetch_scores_from_bancho(&http, api_key, bmap.beatmap_id, mode, mods_filter, amount).await;
            if let Some(ref sc) = fetched {
                let mut s_write = state.write().await;
                s_write.bancho_score_cache.insert(cache_key, (std::time::Instant::now(), sc.clone()));
                if s_write.bancho_score_cache.len() > 200 {
                    s_write.bancho_score_cache.retain(|_, (t, _)| t.elapsed() < std::time::Duration::from_secs(180));
                }
            }
            fetched
        } else {
            None
        };

        if let Some(mut b_scores) = bancho_scores {
            if pp_leaderboard || current_mode.is_some() {
                if let Some(content) = utils::get_or_fetch_beatmap_content(&http, &db, &config, &bmap).await {
                    if let Ok(parsed_map) = rosu_pp::Beatmap::from_bytes(content.as_bytes()) {
                        for sc in &mut b_scores {
                            utils::calculate_bancho_score_pp(&parsed_map, mode, sc);
                        }
                    }
                }
                b_scores.sort_by(|a, b| b.pp.unwrap_or(0.0).partial_cmp(&a.pp.unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
            } else {
                b_scores.sort_by_key(|a| std::cmp::Reverse(a.score.parse::<i64>().unwrap_or(0)));
            }

            for sc in b_scores {
                lb.scores.push(sc.as_leaderboard_entry(pp_leaderboard || current_mode.is_some()));
            }
        }
    } else if rank_type == LeaderboardTypes::Friends {
        let mut friend_entries: Vec<LeaderboardEntry> = Vec::new();

        let friend_records = {
            let db_conn = db.lock().await;
            db::get_friend_records(&db_conn, &player_name).unwrap_or_default()
        };

        {
            let db_conn = db.lock().await;
            let mut users_to_check = vec![player_name.clone()];
            for f in &friend_records {
                if !f.friend_name.is_empty() && !users_to_check.iter().any(|u| u.eq_ignore_ascii_case(&f.friend_name)) {
                    users_to_check.push(f.friend_name.clone());
                }
            }

            for u in users_to_check {
                if let Ok(scores) = db::get_scores_on_map(&db_conn, &u, &md5, mode) {
                    let filtered: Vec<&Score> = if let Some(fm) = current_mode {
                        scores.iter().filter(|sc| Mods::from_bits_truncate(sc.mods).intersects(fm)).collect()
                    } else {
                        scores.iter().filter(|sc| !Mods::from_bits_truncate(sc.mods).intersects(Mods::RELAX | Mods::AUTOPILOT)).collect()
                    };

                    for sc in filtered {
                        friend_entries.push(sc.as_leaderboard_entry(pp_leaderboard, false, current_mode));
                    }
                }
            }
        }

        if let Some(ref api_key) = config.osu_api_key {
            if bmap.beatmap_id > 0 {
                let parsed_map = if pp_leaderboard || current_mode.is_some() {
                    if let Some(content) = utils::get_or_fetch_beatmap_content(&http, &db, &config, &bmap).await {
                        rosu_pp::Beatmap::from_bytes(content.as_bytes()).ok()
                    } else {
                        None
                    }
                } else {
                    None
                };

                let mut tasks = tokio::task::JoinSet::new();
                for f in &friend_records {
                    if f.friend_id > 0 && f.friend_id != 3 && f.friend_id != 4 && f.friend_id != 2070907 {
                        let http = http.clone();
                        let api_key = api_key.clone();
                        let bmap_id = bmap.beatmap_id;
                        let fid = f.friend_id;
                        let fname = f.friend_name.clone();
                        tasks.spawn(async move {
                            let scores = utils::fetch_user_scores_from_bancho(&http, &api_key, bmap_id, fid, mode).await;
                            (fname, scores)
                        });
                    }
                }

                while let Some(res) = tasks.join_next().await {
                    if let Ok((fname, Some(mut b_scores))) = res {
                        for sc in &mut b_scores {
                            if sc.username.is_empty() && !fname.is_empty() {
                                sc.username = fname.clone();
                            }
                            if let Some(ref map) = parsed_map {
                                utils::calculate_bancho_score_pp(map, mode, sc);
                            }
                            friend_entries.push(sc.as_leaderboard_entry(pp_leaderboard || current_mode.is_some()));
                        }
                    }
                }
            }
        }

        friend_entries.sort_by_key(|a| std::cmp::Reverse(a.score));

        let mut seen_users = std::collections::HashSet::new();
        let mut deduped = Vec::new();
        for entry in friend_entries {
            if seen_users.insert(entry.username.to_lowercase()) {
                deduped.push(entry);
            }
        }

        lb.scores = deduped;
    } else if rank_type == LeaderboardTypes::Local {
        let scores = if !player_name.is_empty() {
            let db_conn = db.lock().await;
            db::get_scores_on_map(&db_conn, &player_name, &md5, mode).unwrap_or_default()
        } else {
            Vec::new()
        };

        let filtered: Vec<&Score> = if let Some(fm) = current_mode {
            scores.iter().filter(|sc| Mods::from_bits_truncate(sc.mods).intersects(fm)).collect()
        } else {
            scores.iter().filter(|sc| !Mods::from_bits_truncate(sc.mods).intersects(Mods::RELAX | Mods::AUTOPILOT)).collect()
        };

        let mut sorted = filtered;
        if pp_leaderboard || current_mode.is_some() {
            sorted.sort_by(|a, b| b.pp.unwrap_or(0.0).partial_cmp(&a.pp.unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
        } else {
            sorted.sort_by_key(|a| std::cmp::Reverse(a.score));
        }

        for sc in &sorted {
            lb.scores.push(sc.as_leaderboard_entry(pp_leaderboard, false, current_mode));
        }
    }

    let local_scores = if !player_name.is_empty() {
        let db_conn = db.lock().await;
        db::get_scores_on_map(&db_conn, &player_name, &md5, mode).unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut personal_candidates: Vec<&Score> = if let Some(fm) = current_mode {
        local_scores.iter().filter(|sc| Mods::from_bits_truncate(sc.mods).intersects(fm)).collect()
    } else {
        local_scores.iter().filter(|sc| !Mods::from_bits_truncate(sc.mods).intersects(Mods::RELAX | Mods::AUTOPILOT)).collect()
    };

    if rank_type == LeaderboardTypes::Mods {
        personal_candidates.retain(|sc| sc.mods == mods_val);
    }

    if pp_leaderboard || current_mode.is_some() {
        personal_candidates.sort_by(|a, b| b.pp.unwrap_or(0.0).partial_cmp(&a.pp.unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        personal_candidates.sort_by_key(|a| std::cmp::Reverse(a.score));
    }

    if let Some(best) = personal_candidates.first() {
        let personal_entry = best.as_leaderboard_entry(pp_leaderboard, show_pp_pb, current_mode);
        lb.personal_score = Some(personal_entry.clone());

        if rank_type == LeaderboardTypes::Top
            && current_mode.is_none()
            && best.submission_identity.as_deref().map_or(false, |s| s.starts_with("import:"))
        {
            let db_conn = db.lock().await;
            if let Some((rank, total)) = db::get_map_rank(&db_conn, &player_name, mode, &md5) {
                lb.personal_rank = Some(rank as i32);
                lb.total_scores = Some(total.max(rank) as i32);
            }
        }

        let entry_for_list = if pp_leaderboard || current_mode.is_some() { personal_entry } else { best.as_leaderboard_entry(false, false, current_mode) };

        if is_global {
            lb.scores.push(entry_for_list);
            lb.scores.sort_by_key(|a| std::cmp::Reverse(a.score));

            if let Some(pos) = lb.scores.iter().position(|s| s.score_id == lb.personal_score.as_ref().unwrap().score_id && s.username == player_name) {
                if pos >= 100 {
                    lb.scores.remove(pos);
                }
            }
        } else if rank_type == LeaderboardTypes::Friends
            && !lb.scores.iter().any(|s| s.username.eq_ignore_ascii_case(&player_name))
        {
            lb.scores.push(entry_for_list);
            lb.scores.sort_by_key(|a| std::cmp::Reverse(a.score));
        }
    }

    lb.scores.truncate(amount as usize);

    utils::log_success(&format!("handled map of setid: {}", setid));
    Response::new(lb.as_binary(amount, show_pp_pb))
}

async fn get_friends(state: Arc<RwLock<AppState>>, params: &std::collections::HashMap<String, String>) -> Response {
    let player_name = if let Some(u) = params.get("u") {
        utils::url_decode(u)
    } else {
        let s = state.read().await;
        s.player.as_ref().map(|p| p.name.clone()).unwrap_or_default()
    };

    let s = state.read().await;
    let db_conn = s.db.lock().await;
    let mut friend_ids = db::get_friend_ids(&db_conn, &player_name).unwrap_or_default();
    drop(db_conn);
    drop(s);

    if !friend_ids.contains(&3) {
        friend_ids.push(3);
    }
    if !friend_ids.contains(&4) {
        friend_ids.push(4);
    }

    let mut resp_str = String::new();
    for id in friend_ids {
        resp_str.push_str(&id.to_string());
        resp_str.push('\n');
    }
    Response::text(&resp_str)
}

#[derive(Debug, serde::Deserialize)]
struct CatboyBeatmap {
    #[serde(rename = "DiffName")]
    diff_name: Option<String>,
    #[serde(rename = "Mode")]
    mode: Option<i32>,
}

#[derive(Debug, serde::Deserialize)]
struct CatboyBeatmapSet {
    #[serde(rename = "SetID")]
    set_id: i64,
    #[serde(rename = "RankedStatus")]
    ranked_status: i32,
    #[serde(rename = "Artist")]
    artist: Option<String>,
    #[serde(rename = "Title")]
    title: Option<String>,
    #[serde(rename = "Creator")]
    creator: Option<String>,
    #[serde(rename = "LastUpdate")]
    last_update: Option<String>,
    #[serde(rename = "ChildrenBeatmaps")]
    children_beatmaps: Option<Vec<CatboyBeatmap>>,
}

async fn search_catboy(http: &reqwest::Client, query: &str, mode: i32, ranking_status: i32, page: i32) -> Option<Vec<u8>> {
    let entries = fetch_catboy_entries(http, query, mode, ranking_status, page).await?;
    let mut direct_resp = DirectResponse::new();
    direct_resp.entries = entries;
    Some(direct_resp.as_binary())
}

async fn fetch_catboy_entries(
    http: &reqwest::Client,
    query: &str,
    mode: i32,
    ranking_status: i32,
    page: i32,
) -> Option<Vec<crate::types::direct_response::DirectEntry>> {
    let mut url = reqwest::Url::parse("https://catboy.best/api/search").ok()?;
    {
        let mut pairs = url.query_pairs_mut();
        if !["Newest", "Top Rated", "Most Played"].contains(&query) && !query.is_empty() {
            pairs.append_pair("q", query);
        }
        if mode != -1 {
            pairs.append_pair("mode", &mode.to_string());
        }
        match ranking_status {
            0 | 7 => {
                pairs.append_pair("status", "1");
                pairs.append_pair("status", "2");
            }
            2 => {
                pairs.append_pair("status", "0");
                pairs.append_pair("status", "-1");
            }
            3 => {
                pairs.append_pair("status", "3");
            }
            5 => {
                pairs.append_pair("status", "-2");
            }
            8 => {
                pairs.append_pair("status", "4");
            }
            _ => {}
        }
        if page > 0 {
            pairs.append_pair("offset", &(page * 100).to_string());
        }
        pairs.append_pair("amount", "100");
    }

    let resp = http.get(url).send_traced().await.ok()?;
    if resp.status() != 200 {
        return None;
    }

    let sets: Vec<CatboyBeatmapSet> = resp.json().await.ok()?;
    let mut direct_resp = DirectResponse::new();

    for set in sets {
        let diffs: Vec<String> = set
            .children_beatmaps
            .unwrap_or_default()
            .into_iter()
            .map(|b| {
                let name = b.diff_name.unwrap_or_else(|| "Normal".to_string());
                let m = b.mode.unwrap_or(0);
                format!("{}@{}", name, m)
            })
            .collect();

        let last_updated = set
            .last_update
            .unwrap_or_else(|| "2020-01-01 00:00:00".to_string())
            .replace('T', " ")
            .trim_end_matches('Z')
            .to_string();

        let ranked = set.ranked_status;

        direct_resp.entries.push(crate::types::direct_response::DirectEntry {
            id: set.set_id,
            artist: set.artist.unwrap_or_default(),
            title: set.title.unwrap_or_default(),
            creator: set.creator.unwrap_or_default(),
            ranked,
            last_updated,
            diffs,
        });
    }

    Some(direct_resp.entries)
}

struct FilteredSearch {
    entries: Vec<crate::types::direct_response::DirectEntry>,
    next_page: i32,
    exhausted: bool,
    stamp: std::time::Instant,
}

const FILTER_PAGE_SIZE: usize = 100;
const FILTER_MAX_FETCHES: usize = 30;
const FILTER_PARALLEL: usize = 5;
const FILTER_CACHE_SECS: u64 = 600;

fn slice_filtered_page(
    entries: &[crate::types::direct_response::DirectEntry],
    page: usize,
    exhausted: bool,
) -> (Vec<crate::types::direct_response::DirectEntry>, bool) {
    let start = (page * FILTER_PAGE_SIZE).min(entries.len());
    let end = (start + FILTER_PAGE_SIZE).min(entries.len());
    let more = entries.len() > end || !exhausted;
    (entries[start..end].to_vec(), more)
}

async fn search_catboy_before_cutoff(
    http: &reqwest::Client,
    query: &str,
    mode: i32,
    ranking_status: i32,
    page: i32,
    cutoff: &str,
) -> Option<Vec<u8>> {
    static CACHE: std::sync::OnceLock<tokio::sync::Mutex<std::collections::HashMap<String, FilteredSearch>>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let key = format!("{}|{}|{}|{}", query, mode, ranking_status, cutoff);
    let page = page.max(0) as usize;
    let want = (page + 1) * FILTER_PAGE_SIZE;

    let mut guard = cache.lock().await;
    let fresh = || FilteredSearch { entries: Vec::new(), next_page: 0, exhausted: false, stamp: std::time::Instant::now() };
    let st = guard.entry(key).or_insert_with(fresh);
    if st.stamp.elapsed().as_secs() > FILTER_CACHE_SECS {
        *st = fresh();
    }

    let mut fetched = 0;
    while st.entries.len() < want && !st.exhausted && fetched < FILTER_MAX_FETCHES {
        let first = st.next_page;
        let results = futures_util::future::join_all((0..FILTER_PARALLEL).map(|i| fetch_catboy_entries(http, query, mode, ranking_status, first + i as i32))).await;
        let mut failed = false;
        for result in results {
            match result {
                None => {
                    failed = true;
                    break;
                }
                Some(batch) => {
                    if batch.len() < FILTER_PAGE_SIZE {
                        st.exhausted = true;
                    }
                    st.entries.extend(batch.into_iter().filter(|e| e.last_updated.get(..10).map_or(true, |d| d <= cutoff)));
                    st.next_page += 1;
                    fetched += 1;
                    if st.exhausted {
                        break;
                    }
                }
            }
        }
        if failed {
            if st.entries.is_empty() && fetched == 0 {
                return None;
            }
            break;
        }
    }

    let (slice, more) = slice_filtered_page(&st.entries, page, st.exhausted);
    let mut resp = DirectResponse::new();
    resp.entries = slice;
    resp.more = more;
    Some(resp.as_binary())
}

const OLD_CLIENT_MAP_CUTOFF_BUILD: i64 = 20131231;
const OLD_CLIENT_MAP_CUTOFF_DATE: &str = "2013-12-31";

fn direct_cutoff_for_build(build: Option<i64>) -> Option<&'static str> {
    match build {
        Some(b) if b > 0 && b <= OLD_CLIENT_MAP_CUTOFF_BUILD => Some(OLD_CLIENT_MAP_CUTOFF_DATE),
        _ => None,
    }
}

fn filter_direct_body(body: &[u8], cutoff: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(body);
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        return body.to_vec();
    };
    let Ok(count) = header.trim().parse::<usize>() else {
        return body.to_vec();
    };
    let kept: Vec<&str> = lines
        .filter(|line| match line.split('|').nth(6) {
            Some(date) if date.len() >= 10 && date.as_bytes()[0].is_ascii_digit() => &date[..10] <= cutoff,
            _ => true,
        })
        .collect();
    let new_header = if count >= 100 { "101".to_string() } else { kept.len().to_string() };
    let mut out = vec![new_header];
    out.extend(kept.iter().map(|l| l.to_string()));
    out.join("\n").into_bytes()
}

async fn direct_search(state: Arc<RwLock<AppState>>, params: &std::collections::HashMap<String, String>) -> Response {
    let query = params.get("q").map(|q| utils::url_decode(q)).unwrap_or_default();
    let mode: i32 = params.get("m").and_then(|v| v.parse().ok()).unwrap_or(-1);
    let ranking_status: i32 = params.get("r").and_then(|v| v.parse().ok()).unwrap_or(0);
    let page: i32 = params.get("p").and_then(|v| v.parse().ok()).unwrap_or(0);

    let (prefix, http, osu_username, osu_password, client_build) = {
        let s = state.read().await;
        (s.config.command_prefix.clone(), s.http.clone(), s.config.osu_username.clone(), s.config.osu_password.clone(), s.client_build)
    };
    let cutoff = direct_cutoff_for_build(client_build);

    if query.starts_with(&prefix) {
        let msg = "Commands have moved to in-game chat!\nType your command in #osu or PM BanchoBot/Tillerino (e.g. !help, /np, !r).";
        let resp = DirectResponse::from_str(msg, -2);
        return Response::new(resp.as_binary());
    }

    let mirror_body = match cutoff {
        Some(c) => search_catboy_before_cutoff(&http, &query, mode, ranking_status, page, c).await,
        None => search_catboy(&http, &query, mode, ranking_status, page).await,
    };
    if let Some(body) = mirror_body {
        utils::log_success(&format!("Catboy mirror: maps loaded for query: `{}`!", query));
        return Response::new(body);
    }

    if let (Some(username), Some(password)) = (osu_username, osu_password) {
        if let Ok(mut url) = reqwest::Url::parse("https://osu.ppy.sh/web/osu-search.php") {
            {
                let mut pairs = url.query_pairs_mut();
                pairs.append_pair("u", &username);
                pairs.append_pair("h", &password);
                if !["Newest", "Top Rated", "Most Played"].contains(&query.as_str()) && !query.is_empty() {
                    pairs.append_pair("q", &query);
                }
                if mode != -1 {
                    pairs.append_pair("m", &mode.to_string());
                }
                pairs.append_pair("r", &ranking_status.to_string());
                if page > 0 {
                    pairs.append_pair("p", &page.to_string());
                }
            }

            if let Ok(resp) = http.get(url).send_traced().await {
                if let Ok(body) = resp.bytes().await {
                    utils::log_success(&format!("Official osu!: maps loaded for query: `{}`!", query));
                    let body = match cutoff {
                        Some(c) => filter_direct_body(&body, c),
                        None => body.to_vec(),
                    };
                    return Response::new(body);
                }
            }
        }
    }

    utils::log_error("Failed to load maps from Catboy mirror and official fallback");
    Response::new(b"0".to_vec())
}

pub async fn handle_download(state: Arc<RwLock<AppState>>, setid: i64, no_video: bool) -> Response {
    if setid <= 0 {
        return Response::not_found();
    }

    let http = {
        let s = state.read().await;
        s.http.clone()
    };

    let catboy_url = if no_video {
        format!("https://catboy.best/d/{}n", setid)
    } else {
        format!("https://catboy.best/d/{}", setid)
    };

    let catboy_resp = http.get(&catboy_url).send_traced().await;
    if let Ok(resp) = catboy_resp {
        if resp.status() == hyper::StatusCode::OK {
            if let Ok(bytes) = resp.bytes().await {
                crate::logger::success(&format!("Direct download: downloaded set {} ({} bytes)", setid, bytes.len()));
                return Response::new(bytes.to_vec())
                    .with_header("Content-Type", "application/x-osu-beatmap-archive")
                    .with_header("Content-Disposition", &format!("attachment; filename=\"{}.osz\"", setid));
            }
        }
    }

    let nerinyan_url = format!("https://api.nerinyan.moe/d/{}?novideo={}", setid, if no_video { 1 } else { 0 });
    let nerinyan_resp = http.get(&nerinyan_url).send_traced().await;
    if let Ok(resp) = nerinyan_resp {
        if resp.status() == hyper::StatusCode::OK {
            if let Ok(bytes) = resp.bytes().await {
                crate::logger::success(&format!("Direct download (Nerinyan mirror): downloaded set {} ({} bytes)", setid, bytes.len()));
                return Response::new(bytes.to_vec())
                    .with_header("Content-Type", "application/x-osu-beatmap-archive")
                    .with_header("Content-Disposition", &format!("attachment; filename=\"{}.osz\"", setid));
            }
        }
    }

    crate::logger::error(&format!("Direct download failed for set {}", setid));
    Response::not_found()
}

pub async fn handle_thumbnail(state: Arc<RwLock<AppState>>, filename: &str) -> Response {
    let http = {
        let s = state.read().await;
        s.http.clone()
    };

    let b_url = format!("https://b.ppy.sh/thumb/{}", filename);
    if let Ok(resp) = http.get(&b_url).send_traced().await {
        if resp.status() == hyper::StatusCode::OK {
            if let Ok(bytes) = resp.bytes().await {
                return Response::image(bytes.to_vec(), "jpg");
            }
        }
    }

    let digits: String = filename.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        let assets_url = format!("https://assets.ppy.sh/beatmaps/{}/covers/card.jpg", digits);
        if let Ok(resp) = http.get(&assets_url).send_traced().await {
            if resp.status() == hyper::StatusCode::OK {
                if let Ok(bytes) = resp.bytes().await {
                    return Response::image(bytes.to_vec(), "jpg");
                }
            }
        }
    }

    Response::not_found()
}

pub async fn handle_preview(state: Arc<RwLock<AppState>>, filename: &str) -> Response {
    let http = {
        let s = state.read().await;
        s.http.clone()
    };

    let preview_url = format!("https://b.ppy.sh/preview/{}", filename);
    if let Ok(resp) = http.get(&preview_url).send_traced().await {
        if resp.status() == hyper::StatusCode::OK {
            if let Ok(bytes) = resp.bytes().await {
                return Response::new(bytes.to_vec()).with_header("Content-Type", "audio/mpeg");
            }
        }
    }

    Response::not_found()
}

pub async fn handle_asset(state: Arc<RwLock<AppState>>, sub_path: &str) -> Response {
    let http = {
        let s = state.read().await;
        s.http.clone()
    };

    let cleaned = sub_path.trim_start_matches('/');
    let asset_url = format!("https://assets.ppy.sh/{}", cleaned);
    if let Ok(resp) = http.get(&asset_url).send_traced().await {
        if resp.status() == hyper::StatusCode::OK {
            if let Ok(bytes) = resp.bytes().await {
                let ext = cleaned.rsplit('.').next().unwrap_or("jpg");
                return Response::image(bytes.to_vec(), ext);
            }
        }
    }

    Response::not_found()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use simple_rijndael::impls::RijndaelCbc;
    use simple_rijndael::paddings::Pkcs7Padding;

    #[tokio::test]
    async fn test_score_sub_post_flow() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "Alice").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.file_md5 = "0123456789abcdef0123456789abcdef".to_string();
        bmap.beatmap_id = 999;
        bmap.beatmapset_id = 888;
        bmap.approved = 1;
        bmap.file_content = Some("osu file format v14\n[General]\nMode: 0\n[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\nApproachRate:5\nSliderMultiplier:1.4\nSliderTickRate:1\n[TimingPoints]\n0,500,4,1,0,100,1,0\n[HitObjects]\n256,192,1000,1,0,0:0:0:0:\n".to_string());
        db::insert_beatmap(&conn, &bmap).unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        app_state.player = Some(crate::types::player::Player::new("Alice".to_string()));

        let shared_state = Arc::new(RwLock::new(app_state));

        let osuver = "20260912";
        let key = format!("osu!-scoreburgr---------{}", osuver).into_bytes();
        let iv = vec![42u8; 32];
        let plaintext = format!("{}:Alice:{}:1:0:0:0:0:0:10000:1:True:S:0:True:0:260912120000:20260912", bmap.file_md5, "checksum12345");

        let cipher = RijndaelCbc::<Pkcs7Padding>::new(&key, 32).unwrap();
        let encrypted = cipher.encrypt(&iv, plaintext.into_bytes()).unwrap();

        let b64 = base64::engine::general_purpose::STANDARD;
        let iv_b64 = b64.encode(&iv);
        let score_b64 = b64.encode(&encrypted);

        let body = format!(
            "--boundary\r\nContent-Disposition: form-data; name=\"osuver\"\r\n\r\n{}\r\n\
--boundary\r\nContent-Disposition: form-data; name=\"iv\"\r\n\r\n{}\r\n\
--boundary\r\nContent-Disposition: form-data; name=\"score\"\r\n\r\n{}\r\n\
--boundary\r\nContent-Disposition: form-data; name=\"score\"; filename=\"replay.osr\"\r\n\r\nfake_replay_frames\r\n\
--boundary--\r\n",
            osuver, iv_b64, score_b64
        );

        let mut headers = hyper::HeaderMap::new();
        headers.insert("content-type", hyper::header::HeaderValue::from_static("multipart/form-data; boundary=boundary"));

        let params = std::collections::HashMap::new();
        let resp = handle(shared_state.clone(), "/osu-submit-modular-selector.php", &params, &hyper::Method::POST, &headers, body.as_bytes()).await;

        let resp_str = String::from_utf8(resp.body).unwrap();
        assert!(resp_str.contains("beatmapId:999"));
        assert!(resp_str.contains("chartId:beatmap"));
        assert!(resp_str.contains("chartId:overall"));

        let s = shared_state.read().await;
        let p = s.player.as_ref().unwrap();
        assert_eq!(p.playcount, 1);

        let pkts = packets::split_packets(&p.queue);
        let notif_pkt = pkts.iter().find(|pkt| pkt.id == packets::PacketId::ChoNotification as u16).expect("Score submission notification");
        let mut r = packets::PacketReader::new(notif_pkt.payload);
        let msg = r.read_string().expect("notif message");
        assert!(msg.starts_with("Score submitted"), "Notification must be minimal score submitted format");
        assert!(!msg.contains('\n'), "Notification must be a clean single line");

        let recent_msg_pkt = pkts.iter().find(|pkt| pkt.id == packets::PacketId::ChoSendMessage as u16).expect("#recent message");
        let mut r = packets::PacketReader::new(recent_msg_pkt.payload);
        let _client = r.read_string().unwrap();
        let _content = r.read_string().unwrap();
        let target = r.read_string().unwrap();
        assert_eq!(target, "#recent");
    }

    #[tokio::test]
    async fn test_score_sub_failed_play_returns_charts_and_updates_stats() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "Alice").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.file_md5 = "0123456789abcdef0123456789abcdef".to_string();
        bmap.beatmap_id = 999;
        bmap.beatmapset_id = 888;
        bmap.approved = 1;
        bmap.file_content = Some("osu file format v14\n[General]\nMode: 0\n[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\nApproachRate:5\nSliderMultiplier:1.4\nSliderTickRate:1\n[TimingPoints]\n0,500,4,1,0,100,1,0\n[HitObjects]\n256,192,1000,1,0,0:0:0:0:\n".to_string());
        db::insert_beatmap(&conn, &bmap).unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        app_state.player = Some(crate::types::player::Player::new("Alice".to_string()));

        let shared_state = Arc::new(RwLock::new(app_state));

        let osuver = "20260912";
        let key = format!("osu!-scoreburgr---------{}", osuver).into_bytes();
        let iv = vec![42u8; 32];
        let plaintext = format!("{}:Alice:{}:1:0:0:0:0:0:10000:1:False:F:0:False:0:260912120000:20260912", bmap.file_md5, "checksum_fail");

        let cipher = RijndaelCbc::<Pkcs7Padding>::new(&key, 32).unwrap();
        let encrypted = cipher.encrypt(&iv, plaintext.into_bytes()).unwrap();

        let b64 = base64::engine::general_purpose::STANDARD;
        let iv_b64 = b64.encode(&iv);
        let score_b64 = b64.encode(&encrypted);

        let body = format!(
            "--boundary\r\nContent-Disposition: form-data; name=\"osuver\"\r\n\r\n{}\r\n\
--boundary\r\nContent-Disposition: form-data; name=\"iv\"\r\n\r\n{}\r\n\
--boundary\r\nContent-Disposition: form-data; name=\"score\"\r\n\r\n{}\r\n\
--boundary\r\nContent-Disposition: form-data; name=\"score\"; filename=\"replay.osr\"\r\n\r\nfake_replay_frames\r\n\
--boundary--\r\n",
            osuver, iv_b64, score_b64
        );

        let mut headers = hyper::HeaderMap::new();
        headers.insert("content-type", hyper::header::HeaderValue::from_static("multipart/form-data; boundary=boundary"));

        let params = std::collections::HashMap::new();
        let resp = handle(shared_state.clone(), "/osu-submit-modular-selector.php", &params, &hyper::Method::POST, &headers, body.as_bytes()).await;

        let resp_str = String::from_utf8(resp.body).unwrap();
        assert!(!resp_str.contains("error: no"), "Failed plays must not return error: no");
        assert!(resp_str.contains("chartId:beatmap"), "Response must contain beatmap chart");
        assert!(resp_str.contains("chartId:overall"), "Response must contain overall chart to conclude client submission");

        let s = shared_state.read().await;
        let p = s.player.as_ref().unwrap();
        assert_eq!(p.playcount, 1, "Failed play must still increment playcount");

        let pkts = packets::split_packets(&p.queue);
        assert!(!pkts.iter().any(|pkt| pkt.id == packets::PacketId::ChoNotification as u16), "Failed plays must NOT send notification popups to the player");
        assert!(pkts.iter().any(|pkt| pkt.id == packets::PacketId::ChoUserStats as u16), "Failed plays must queue ChoUserStats to update in-game HUD immediately");
        assert!(!pkts.iter().any(|pkt| pkt.id == packets::PacketId::ChoSendMessage as u16), "Failed plays must NOT send messages to #recent");
    }

    #[tokio::test]
    async fn test_leaderboard_local_flow() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "Bob").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.file_md5 = "fedcba9876543210fedcba9876543210".to_string();
        bmap.beatmap_id = 1234;
        bmap.beatmapset_id = 5678;
        bmap.approved = 1;
        bmap.title = "Test Song".to_string();
        bmap.artist = "Test Artist".to_string();
        db::insert_beatmap(&conn, &bmap).unwrap();

        let score = Score {
            mode: 0,
            md5: bmap.file_md5.clone(),
            name: "Bob".to_string(),
            n300: 300,
            n100: 0,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss: 0,
            score: 1_234_567,
            max_combo: 500,
            perfect: true,
            mods: 0,
            time: 1700000000,
            acc: Some(100.0),
            pp: Some(250.0),
            replay_md5: None,
            scoreid: Some(101),
            replay_frames: Some("fake_frames".to_string()),
            mods_str: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        db::insert_score(&conn, &score, "ranked").unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        app_state.player = Some(crate::types::player::Player::new("Bob".to_string()));
        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("c".to_string(), bmap.file_md5.clone());
        params.insert("m".to_string(), "0".to_string());
        params.insert("v".to_string(), "0".to_string());

        let headers = hyper::HeaderMap::new();
        let resp = handle(shared_state.clone(), "/osu-osz2-getscores.php", &params, &hyper::Method::GET, &headers, &[]).await;

        let resp_str = String::from_utf8(resp.body).unwrap();
        assert!(resp_str.starts_with("2|false|1234|5678|1\n"));
        assert!(resp_str.contains("Test Artist|Test Song"));
        assert!(resp_str.contains("Bob"));
    }

    #[tokio::test]
    async fn test_leaderboard_unranked_map() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "Charlie").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.file_md5 = "unranked_md5_hash_1234567890abcdef".to_string();
        bmap.beatmap_id = 4321;
        bmap.beatmapset_id = 8765;
        bmap.approved = 0;
        db::insert_beatmap(&conn, &bmap).unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        app_state.player = Some(crate::types::player::Player::new("Charlie".to_string()));
        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("c".to_string(), bmap.file_md5.clone());
        params.insert("m".to_string(), "0".to_string());
        params.insert("v".to_string(), "1".to_string());

        let headers = hyper::HeaderMap::new();
        let resp = handle(shared_state.clone(), "/osu-osz2-getscores.php", &params, &hyper::Method::GET, &headers, &[]).await;

        let resp_str = String::from_utf8(resp.body).unwrap();
        assert_eq!(resp_str, "0|false");
    }

    #[tokio::test]
    async fn test_leaderboard_selected_mods_filtering() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "Dave").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.file_md5 = "mods_test_md5_hash_1234567890abcdef".to_string();
        bmap.beatmap_id = 777;
        bmap.beatmapset_id = 888;
        bmap.approved = 1;
        db::insert_beatmap(&conn, &bmap).unwrap();

        let score_hd = Score {
            mode: 0,
            md5: bmap.file_md5.clone(),
            name: "Dave".to_string(),
            n300: 300,
            n100: 0,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss: 0,
            score: 1_000_000,
            max_combo: 500,
            perfect: true,
            mods: 8,
            time: 1700000000,
            acc: Some(100.0),
            pp: Some(300.0),
            replay_md5: None,
            scoreid: Some(201),
            replay_frames: Some("frames".to_string()),
            mods_str: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        db::insert_score(&conn, &score_hd, "ranked").unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        app_state.player = Some(crate::types::player::Player::new("Dave".to_string()));
        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params_hr = std::collections::HashMap::new();
        params_hr.insert("c".to_string(), bmap.file_md5.clone());
        params_hr.insert("m".to_string(), "0".to_string());
        params_hr.insert("v".to_string(), "2".to_string());
        params_hr.insert("mods".to_string(), "16".to_string());

        let headers = hyper::HeaderMap::new();
        let resp_hr = handle(shared_state.clone(), "/osu-osz2-getscores.php", &params_hr, &hyper::Method::GET, &headers, &[]).await;
        let str_hr = String::from_utf8(resp_hr.body).unwrap();
        assert!(str_hr.starts_with("2|false|777|888|0\n"));

        let mut params_hd = std::collections::HashMap::new();
        params_hd.insert("c".to_string(), bmap.file_md5.clone());
        params_hd.insert("m".to_string(), "0".to_string());
        params_hd.insert("v".to_string(), "2".to_string());
        params_hd.insert("mods".to_string(), "8".to_string());

        let resp_hd = handle(shared_state.clone(), "/osu-osz2-getscores.php", &params_hd, &hyper::Method::GET, &headers, &[]).await;
        let str_hd = String::from_utf8(resp_hd.body).unwrap();
        assert!(str_hd.starts_with("2|false|777|888|1\n"));
        assert!(str_hd.contains("Dave"));
    }

    #[tokio::test]
    async fn test_leaderboard_binary_generation_with_bancho_scores() {
        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.beatmap_id = 1001;
        bmap.beatmapset_id = 2002;
        bmap.approved = 1;
        bmap.title = "Freedom Dive".to_string();
        bmap.artist = "xi".to_string();

        let mut lb = Leaderboard::new();
        lb.bmap = Some(bmap);

        let json_bancho = serde_json::json!({
            "score_id": "999999",
            "username": "Cookiezi",
            "score": "9999999",
            "maxcombo": "2000",
            "count50": "0",
            "count100": "0",
            "count300": "1500",
            "countmiss": "0",
            "countkatu": "0",
            "countgeki": "0",
            "perfect": "1",
            "enabled_mods": "24",
            "user_id": "124",
            "date": "2023-01-01 00:00:00",
            "replay_available": "1",
            "pp": "800.0"
        });

        let b_score = crate::types::score::BanchoScore::from_json(&json_bancho).unwrap();
        let bancho_entry = b_score.as_leaderboard_entry(true);

        let local_entry = crate::types::score::LeaderboardEntry {
            score_id: -1,
            username: "LocalPlayer".to_string(),
            score: 750,
            maxcombo: 1800,
            count50: 0,
            count100: 5,
            count300: 1495,
            countmiss: 0,
            countkatu: 0,
            countgeki: 0,
            perfect: 1,
            enabled_mods: 0,
            user_id: 2,
            time: 1700000000,
            replay_available: 1,
        };

        lb.personal_score = Some(local_entry.clone());
        lb.scores.push(bancho_entry);
        lb.scores.push(local_entry);

        lb.scores.sort_by_key(|a| std::cmp::Reverse(a.score));

        let bin = lb.as_binary(100, false);
        let s = String::from_utf8(bin).unwrap();
        let lines: Vec<&str> = s.lines().collect();

        assert_eq!(lines[0], "2|false|1001|2002|2");
        assert_eq!(lines[1], "0");
        assert_eq!(lines[2], "[bold:0,size:20]xi|Freedom Dive");
        assert_eq!(lines[3], "10.0");
        assert!(lines[4].contains("LocalPlayer"));
        assert!(lines[4].contains("|2|"));
        assert!(lines[5].contains("Cookiezi"));
        assert!(lines[5].contains("|1|"));
        assert!(lines[6].contains("LocalPlayer"));
        assert!(lines[6].contains("|2|"));
    }

    #[tokio::test]
    async fn test_get_friends_endpoint() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "FriendTester").unwrap();
        db::add_friend(&conn, "FriendTester", 12345, "Buddy1").unwrap();
        db::add_friend(&conn, "FriendTester", 67890, "Buddy2").unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        app_state.player = Some(crate::types::player::Player::new("FriendTester".to_string()));
        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("u".to_string(), "FriendTester".to_string());

        let headers = hyper::HeaderMap::new();
        let resp = handle(shared_state, "/osu-getfriends.php", &params, &hyper::Method::GET, &headers, &[]).await;
        let body_str = String::from_utf8(resp.body).unwrap();
        assert!(body_str.contains("3\n"), "BanchoBot (3) must be in friends response");
        assert!(body_str.contains("4\n"), "Tillerino (4) must be in friends response");
        assert!(body_str.contains("12345\n"));
        assert!(body_str.contains("67890\n"));
    }

    #[tokio::test]
    async fn test_leaderboard_friends_flow() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "MainPlayer").unwrap();
        db::ensure_profile(&conn, "Bestie").unwrap();
        db::add_friend(&conn, "MainPlayer", 8888, "Bestie").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.beatmap_id = 1234;
        bmap.beatmapset_id = 5678;
        bmap.artist = "Camellia".to_string();
        bmap.title = "Ghost".to_string();
        bmap.version = "Insane".to_string();
        bmap.creator = "Mapper".to_string();
        bmap.file_md5 = "ghostmd5".to_string();
        bmap.diff_overall = 9.0;
        bmap.diff_approach = 9.0;
        bmap.diff_size = 4.0;
        bmap.diff_drain = 6.0;
        bmap.mode = 0;
        bmap.approved = 1;
        bmap.bpm = 220.0;
        db::insert_beatmap(&conn, &bmap).unwrap();

        let score_bestie = Score {
            mode: 0,
            md5: "ghostmd5".to_string(),
            name: "Bestie".to_string(),
            n300: 300,
            n100: 0,
            n50: 0,
            ngeki: 50,
            nkatu: 0,
            nmiss: 0,
            score: 5000000,
            max_combo: 600,
            perfect: true,
            mods: 0,
            time: 1700000000,
            acc: Some(100.0),
            pp: Some(350.0),
            replay_md5: None,
            scoreid: Some(101),
            replay_frames: None,
            mods_str: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        db::insert_score(&conn, &score_bestie, "ranked").unwrap();

        let score_main = Score {
            mode: 0,
            md5: "ghostmd5".to_string(),
            name: "MainPlayer".to_string(),
            n300: 290,
            n100: 10,
            n50: 0,
            ngeki: 40,
            nkatu: 0,
            nmiss: 0,
            score: 4500000,
            max_combo: 600,
            perfect: true,
            mods: 0,
            time: 1700000010,
            acc: Some(98.5),
            pp: Some(300.0),
            replay_md5: None,
            scoreid: Some(102),
            replay_frames: None,
            mods_str: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        db::insert_score(&conn, &score_main, "ranked").unwrap();

        let mut config = crate::types::config::Config::default();
        config.pp_leaderboard = true;
        let mut app_state = AppState::new(conn, config);
        let mut player = crate::types::player::Player::new("MainPlayer".to_string());
        player.userid = 2;
        app_state.player = Some(player);
        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("c".to_string(), "ghostmd5".to_string());
        params.insert("m".to_string(), "0".to_string());
        params.insert("v".to_string(), "3".to_string());

        let headers = hyper::HeaderMap::new();
        let resp = handle(shared_state, "/osu-osz2-getscores.php", &params, &hyper::Method::GET, &headers, &[]).await;
        assert_eq!(resp.status, hyper::StatusCode::OK);
        let body_str = String::from_utf8(resp.body).unwrap();
        assert!(!body_str.starts_with("0|false"));
        assert!(body_str.contains("Bestie"), "Friend score must appear on friend leaderboard");
        assert!(body_str.contains("MainPlayer"), "Player personal score must appear on friend leaderboard");
    }

    #[tokio::test]
    async fn test_direct_search_command_guidance() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();

        let config = crate::types::config::Config::default();
        let app_state = AppState::new(conn, config);
        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("q".to_string(), "!help".to_string());

        let headers = hyper::HeaderMap::new();
        let resp = handle(shared_state, "/osu-search.php", &params, &hyper::Method::GET, &headers, &[]).await;
        assert_eq!(resp.status, hyper::StatusCode::OK);
        assert!(!resp.body.is_empty());
    }

    #[tokio::test]
    async fn test_handle_download_catboy() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let config = crate::types::config::Config::default();
        let app_state = AppState::new(conn, config);
        let shared_state = Arc::new(RwLock::new(app_state));

        let resp = handle_download(shared_state.clone(), 0, false).await;
        assert_eq!(resp.status, hyper::StatusCode::NOT_FOUND);

        let resp_thumb = handle_thumbnail(shared_state, "2620610l.jpg").await;
        assert!(resp_thumb.status == hyper::StatusCode::OK || resp_thumb.status == hyper::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_direct_response_status_reporting() {
        use crate::types::direct_response::{DirectEntry, DirectResponse};

        let mut resp = DirectResponse::new();
        resp.entries.push(DirectEntry {
            id: 100,
            artist: "Test Artist".to_string(),
            title: "Test Title".to_string(),
            creator: "Mapper".to_string(),
            ranked: 1,
            last_updated: "2026-09-19 12:00:00".to_string(),
            diffs: vec!["Normal@0".to_string()],
        });
        resp.entries.push(DirectEntry {
            id: 101,
            artist: "Test Artist".to_string(),
            title: "Pending Title".to_string(),
            creator: "Mapper".to_string(),
            ranked: 0,
            last_updated: "2026-09-19 12:00:00".to_string(),
            diffs: vec!["Hard@0".to_string()],
        });

        let binary = resp.as_binary();
        let text = String::from_utf8(binary).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "2");

        let fields_ranked: Vec<&str> = lines[1].split('|').collect();
        assert_eq!(fields_ranked[0], "100.osz");
        assert_eq!(fields_ranked[4], "1", "Ranked beatmap must have status 1 in osu!direct line");

        let fields_pending: Vec<&str> = lines[2].split('|').collect();
        assert_eq!(fields_pending[0], "101.osz");
        assert_eq!(fields_pending[4], "0", "Pending beatmap must have status 0 in osu!direct line");
    }

    #[test]
    fn test_direct_from_plays_status() {
        use crate::types::direct_response::{DirectResponse, PlayEntry};

        let play = PlayEntry {
            bmap_title: "Title".to_string(),
            bmap_version: "Insane".to_string(),
            bmap_artist: "Artist".to_string(),
            bmap_creator: "Creator".to_string(),
            bmap_setid: 500,
            bmap_approved: 1,
            bmap_max_combo: 1000,
            pp: 300.0,
            acc: 99.5,
            mods_str: "HDDT".to_string(),
            max_combo: 1000,
            nmiss: 0,
            n300: 500,
            n100: 5,
            n50: 0,
            ngeki: 100,
            nkatu: 10,
            mods: 72,
            mode: 0,
            time_str: "2026-09-19 12:00:00".to_string(),
        };

        let resp = DirectResponse::from_plays(&[play]);
        assert_eq!(resp.entries[0].ranked, 1, "PlayEntry ranked field must be 1 for Ranked maps");
    }

    #[tokio::test]
    async fn test_search_catboy_ranked_filtering() {
        let http = reqwest::Client::new();
        if let Some(body) = search_catboy(&http, "Newest", -1, 0, 0).await {
            let text = String::from_utf8_lossy(&body);
            let lines: Vec<&str> = text.lines().collect();
            let count: usize = lines[0].parse().unwrap_or(0);
            assert!(count > 0, "Catboy mirror must return maps when Ranked (r=0) is selected");
            if lines.len() > 1 {
                let fields: Vec<&str> = lines[1].split('|').collect();
                let status: i32 = fields[4].parse().unwrap_or(-99);
                assert!(status == 1 || status == 2, "Ranked maps must have status 1 or 2, got {}", status);
            }
        }

        if let Some(body) = search_catboy(&http, "Newest", -1, 2, 0).await {
            let text = String::from_utf8_lossy(&body);
            let lines: Vec<&str> = text.lines().collect();
            let count: usize = lines[0].parse().unwrap_or(0);
            assert!(count > 0, "Catboy mirror must return maps when Pending (r=2) is selected");
            if lines.len() > 1 {
                let fields: Vec<&str> = lines[1].split('|').collect();
                let status: i32 = fields[4].parse().unwrap_or(-99);
                assert!(status == 0 || status == -1, "Pending maps must have status 0 or -1, got {}", status);
            }
        }
    }

    #[tokio::test]
    async fn test_leaderboard_pre_login_instant_load() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.beatmap_id = 9991;
        bmap.beatmapset_id = 9992;
        bmap.artist = "PreLoginArtist".to_string();
        bmap.title = "PreLoginTitle".to_string();
        bmap.version = "Normal".to_string();
        bmap.file_md5 = "prelogin_md5".to_string();
        bmap.approved = 1;
        db::insert_beatmap(&conn, &bmap).unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        app_state.player = None;
        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("c".to_string(), "prelogin_md5".to_string());
        params.insert("m".to_string(), "0".to_string());
        params.insert("v".to_string(), "1".to_string());
        params.insert("us".to_string(), "PreLoginUser".to_string());

        let headers = hyper::HeaderMap::new();
        let resp = handle(shared_state.clone(), "/osu-osz2-getscores.php", &params, &hyper::Method::GET, &headers, &[]).await;

        assert_eq!(resp.status, hyper::StatusCode::OK);
        let body_str = String::from_utf8(resp.body).unwrap();
        assert!(!body_str.starts_with("0|false"));
        assert!(body_str.contains("PreLoginArtist"));
        assert!(body_str.contains("PreLoginTitle"));

        let s = shared_state.read().await;
        assert_eq!(s.pending_login_name.as_deref(), Some("PreLoginUser"));
    }

    #[tokio::test]
    async fn test_leaderboard_bancho_score_cache() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.beatmap_id = 8881;
        bmap.beatmapset_id = 8882;
        bmap.artist = "CachedArtist".to_string();
        bmap.title = "CachedTitle".to_string();
        bmap.version = "Hard".to_string();
        bmap.file_md5 = "cached_md5".to_string();
        bmap.approved = 1;
        db::insert_beatmap(&conn, &bmap).unwrap();

        let mut config = crate::types::config::Config::default();
        config.amount_of_scores_on_lb = 50;
        let mut app_state = AppState::new(conn, config);

        let dummy_cached_score = crate::types::score::BanchoScore {
            score_id: "77777".to_string(),
            username: "CacheHitUser".to_string(),
            score: "999999".to_string(),
            maxcombo: "500".to_string(),
            count50: "0".to_string(),
            count100: "0".to_string(),
            count300: "300".to_string(),
            countmiss: "0".to_string(),
            countkatu: "0".to_string(),
            countgeki: "50".to_string(),
            perfect: "1".to_string(),
            enabled_mods: "0".to_string(),
            user_id: "555".to_string(),
            time: 1700000000,
            replay_available: "0".to_string(),
            pp: Some(400.0),
        };
        app_state.bancho_score_cache.insert(
            (8881, 0, None, 50),
            (std::time::Instant::now(), vec![dummy_cached_score]),
        );
        app_state.player = Some(crate::types::player::Player::new("TestPlayer".to_string()));

        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("c".to_string(), "cached_md5".to_string());
        params.insert("m".to_string(), "0".to_string());
        params.insert("v".to_string(), "1".to_string());

        let headers = hyper::HeaderMap::new();
        let resp = handle(shared_state, "/osu-osz2-getscores.php", &params, &hyper::Method::GET, &headers, &[]).await;

        assert_eq!(resp.status, hyper::StatusCode::OK);
        let body_str = String::from_utf8(resp.body).unwrap();
        assert!(body_str.contains("CacheHitUser"), "Leaderboard must include cached score");
    }

    #[tokio::test]
    async fn test_leaderboard_while_logged_in_does_not_pollute_pending_login_name() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "LoggedInUser").unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        app_state.player = Some(crate::types::player::Player::new("LoggedInUser".to_string()));
        app_state.pending_login_name = None;
        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("c".to_string(), "any_md5".to_string());
        params.insert("m".to_string(), "0".to_string());
        params.insert("v".to_string(), "1".to_string());
        params.insert("us".to_string(), "LoggedInUser".to_string());

        let headers = hyper::HeaderMap::new();
        let _ = handle(shared_state.clone(), "/osu-osz2-getscores.php", &params, &hyper::Method::GET, &headers, &[]).await;

        let s = shared_state.read().await;
        assert!(s.pending_login_name.is_none(), "Leaderboard queries while a player is active must NEVER pollute pending_login_name");
    }

    #[tokio::test]
    async fn test_score_sub_while_restricted_returns_error_ban() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "RestrictedSubUser").unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        let mut player = crate::types::player::Player::new("RestrictedSubUser".to_string());
        player.is_restricted = true;
        app_state.player = Some(player);

        let shared_state = Arc::new(RwLock::new(app_state));

        let headers = hyper::HeaderMap::new();
        let params = std::collections::HashMap::new();
        let resp = handle(
            shared_state,
            "/osu-submit-modular-selector.php",
            &params,
            &hyper::Method::POST,
            &headers,
            b"test_body",
        ).await;

        assert_eq!(resp.status, hyper::StatusCode::OK);
        assert_eq!(resp.body, b"error: ban\n");
    }

    #[tokio::test]
    async fn test_leaderboard_while_restricted_shows_personal_score() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "RestrictedLBUser").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.file_md5 = "0123456789abcdef0123456789abcdef".to_string();
        bmap.beatmap_id = 999999;
        bmap.beatmapset_id = 888888;
        bmap.approved = 1;
        db::insert_beatmap(&conn, &bmap).unwrap();

        let sample_score = Score {
            mode: 0,
            md5: bmap.file_md5.clone(),
            name: "RestrictedLBUser".to_string(),
            n300: 300,
            n100: 0,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss: 0,
            score: 500_000,
            max_combo: 100,
            perfect: true,
            mods: 0,
            time: 1700000000,
            acc: Some(100.0),
            pp: Some(150.0),
            replay_md5: None,
            scoreid: Some(999),
            replay_frames: None,
            mods_str: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        db::insert_score(&conn, &sample_score, "ranked").unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        let mut player = crate::types::player::Player::new("RestrictedLBUser".to_string());
        player.is_restricted = true;
        app_state.player = Some(player);

        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("c".to_string(), "0123456789abcdef0123456789abcdef".to_string());
        params.insert("m".to_string(), "0".to_string());
        params.insert("v".to_string(), "1".to_string());
        params.insert("u".to_string(), "RestrictedLBUser".to_string());

        let headers = hyper::HeaderMap::new();
        let resp = handle(
            shared_state,
            "/osu-osz2-getscores.php",
            &params,
            &hyper::Method::GET,
            &headers,
            b"",
        ).await;

        let body_str = String::from_utf8_lossy(&resp.body);
        let lines: Vec<&str> = body_str.lines().collect();
        assert!(lines.len() >= 5, "Response must include header and personal score lines");
        let personal_score_line = lines[4];
        assert!(personal_score_line.contains("RestrictedLBUser"), "Personal best banner must be shown even when restricted");
    }

    #[tokio::test]
    async fn test_get_replay_local_flow() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "ReplayUser").unwrap();

        let raw_frames = b"raw_replay_frame_data_for_test";
        let b64_frames = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, raw_frames);

        let score = Score {
            mode: 0,
            md5: "somereplaymd5".to_string(),
            name: "ReplayUser".to_string(),
            n300: 300,
            n100: 0,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss: 0,
            score: 1_000_000,
            max_combo: 500,
            perfect: true,
            mods: 0,
            time: 1700000000,
            acc: Some(100.0),
            pp: Some(250.0),
            replay_md5: None,
            scoreid: Some(42),
            replay_frames: Some(b64_frames),
            mods_str: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        db::insert_score(&conn, &score, "ranked").unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        app_state.player = Some(crate::types::player::Player::new("ReplayUser".to_string()));
        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("c".to_string(), "-1".to_string());
        params.insert("m".to_string(), "0".to_string());

        let headers = hyper::HeaderMap::new();
        let resp = handle(shared_state.clone(), "/osu-getreplay.php", &params, &hyper::Method::GET, &headers, &[]).await;

        assert_eq!(resp.status, hyper::StatusCode::OK);
        assert_eq!(resp.body, raw_frames);

        let mut params_pos = std::collections::HashMap::new();
        params_pos.insert("c".to_string(), "1".to_string());
        params_pos.insert("m".to_string(), "0".to_string());
        let resp_pos = handle(shared_state, "/osu-getreplay.php", &params_pos, &hyper::Method::GET, &headers, &[]).await;
        assert_eq!(resp_pos.status, hyper::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_leaderboard_concurrent_write_lock_acquisition() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "LockTester").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.beatmap_id = 5555;
        bmap.beatmapset_id = 6666;
        bmap.approved = 1;
        bmap.file_md5 = "lock_test_md5".to_string();
        db::insert_beatmap(&conn, &bmap).unwrap();

        let config = crate::types::config::Config::default();
        let mut app_state = AppState::new(conn, config);
        app_state.player = Some(crate::types::player::Player::new("LockTester".to_string()));
        let shared_state = Arc::new(RwLock::new(app_state));

        let state_clone1 = shared_state.clone();
        let handle1 = tokio::spawn(async move {
            let mut params = std::collections::HashMap::new();
            params.insert("c".to_string(), "lock_test_md5".to_string());
            params.insert("m".to_string(), "0".to_string());
            params.insert("v".to_string(), "1".to_string());
            let headers = hyper::HeaderMap::new();
            handle(state_clone1, "/osu-osz2-getscores.php", &params, &hyper::Method::GET, &headers, &[]).await
        });

        let state_clone2 = shared_state.clone();
        let handle2 = tokio::spawn(async move {
            let mut s = state_clone2.write().await;
            if let Some(ref mut p) = s.player {
                p.action = 2;
            }
        });

        let (res1, res2) = tokio::join!(handle1, handle2);
        assert!(res1.is_ok());
        assert!(res2.is_ok());
    }

    #[tokio::test]
    async fn test_leaderboard_show_pp_pb_isolation() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "PBUser").unwrap();

        let mut bmap = crate::types::beatmap::Beatmap::blank();
        bmap.beatmap_id = 4444;
        bmap.beatmapset_id = 7777;
        bmap.approved = 1;
        bmap.file_md5 = "pb_isolation_md5".to_string();
        db::insert_beatmap(&conn, &bmap).unwrap();

        let score = Score {
            mode: 0,
            md5: "pb_isolation_md5".to_string(),
            name: "PBUser".to_string(),
            n300: 300,
            n100: 0,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss: 0,
            score: 12_345_678,
            max_combo: 500,
            perfect: true,
            mods: 0,
            time: 1700000000,
            acc: Some(100.0),
            pp: Some(450.0),
            replay_md5: None,
            scoreid: Some(99),
            replay_frames: None,
            mods_str: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        };
        db::insert_score(&conn, &score, "ranked").unwrap();

        let mut config = crate::types::config::Config::default();
        config.pp_leaderboard = false;
        config.show_pp_for_personal_best = true;

        let mut app_state = AppState::new(conn, config);
        app_state.player = Some(crate::types::player::Player::new("PBUser".to_string()));
        let shared_state = Arc::new(RwLock::new(app_state));

        let mut params = std::collections::HashMap::new();
        params.insert("c".to_string(), "pb_isolation_md5".to_string());
        params.insert("m".to_string(), "0".to_string());
        params.insert("v".to_string(), "0".to_string());

        let headers = hyper::HeaderMap::new();
        let resp = handle(shared_state.clone(), "/osu-osz2-getscores.php", &params, &hyper::Method::GET, &headers, &[]).await;
        let body_str = String::from_utf8(resp.body).unwrap();
        let lines: Vec<&str> = body_str.lines().collect();

        assert!(lines.len() >= 6, "Must have header, offset, title, rating, personal score, and leaderboard score");
        let pb_line = lines[4];
        let lb_line = lines[5];

        assert!(pb_line.contains("|450|"), "Personal best banner must display PP: got {}", pb_line);

        assert!(lb_line.contains("|12345678|"), "Leaderboard list entry must display score, NOT PP: got {}", lb_line);
        assert!(!lb_line.contains("|450|"), "Leaderboard list entry must NOT leak show_pp_pb: got {}", lb_line);
    }
}

#[cfg(test)]
mod legacy_lb_tests {
    #[test]
    fn filtered_pages_are_sliced_and_report_more_results() {
        let entry = |i: i64| crate::types::direct_response::DirectEntry { id: i, artist: String::new(), title: String::new(), creator: String::new(), ranked: 1, last_updated: "2012-01-01 00:00:00".into(), diffs: vec![] };
        let entries: Vec<_> = (0..130).map(entry).collect();
        let (first, more) = super::slice_filtered_page(&entries, 0, true);
        assert_eq!((first.len(), more), (100, true));
        let (second, more) = super::slice_filtered_page(&entries, 1, true);
        assert_eq!((second.len(), more), (30, false));
        let (second, more) = super::slice_filtered_page(&entries, 1, false);
        assert_eq!((second.len(), more), (30, true));
        let (beyond, more) = super::slice_filtered_page(&entries, 5, true);
        assert_eq!((beyond.len(), more), (0, false));
    }

    #[test]
    fn old_clients_only_see_maps_updated_by_the_end_of_2013() {
        assert_eq!(super::direct_cutoff_for_build(Some(20130604)), Some("2013-12-31"));
        assert_eq!(super::direct_cutoff_for_build(Some(1704)), Some("2013-12-31"));
        assert_eq!(super::direct_cutoff_for_build(Some(20140401)), None);
        assert_eq!(super::direct_cutoff_for_build(None), None);
        let body = b"3\n1.osz|a|t|c|1|10.0|2012-05-01 10:00:00|1|0\n2.osz|a|t|c|1|10.0|2013-12-31 23:59:59|2|0\n3.osz|a|t|c|1|10.0|2014-01-01 00:00:00|3|0";
        let out = String::from_utf8(super::filter_direct_body(body, "2013-12-31")).unwrap();
        assert_eq!(out, "2\n1.osz|a|t|c|1|10.0|2012-05-01 10:00:00|1|0\n2.osz|a|t|c|1|10.0|2013-12-31 23:59:59|2|0");
        let full: String = std::iter::once("100".to_string()).chain((0..100).map(|i| format!("{i}.osz|a|t|c|1|10.0|2020-01-01 00:00:00|{i}|0"))).collect::<Vec<_>>().join("\n");
        let paged = String::from_utf8(super::filter_direct_body(full.as_bytes(), "2013-12-31")).unwrap();
        assert_eq!(paged, "101");
    }

    #[test]
    fn plain_submit_reply_is_rank_then_next_rank_then_achievements() {
        let charts = b"beatmapId:1|approvedDate:0\nchartId:overall|achievements:a.png b.png|rankBefore:5|beatmapRankingBefore:0|beatmapRankingAfter:3";
        assert_eq!(super::plain_submit_reply(charts), b"3\n0\na.png b.png".to_vec());
        assert_eq!(super::plain_submit_reply(b"error: no"), b"error: no".to_vec());
    }

    use super::legacy_leaderboard_text;

    #[test]
    fn legacy_format_drops_header_fields_and_replay_flag() {
        let modern = "2|false|5|6|1\n0\nA|B\n10.0\n\n1|name|100|10|0|0|10|0|0|0|1|0|9|1|1700000000|1";
        let out = legacy_leaderboard_text(modern);
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines[0], "2");
        assert_eq!(lines[2], "A|B");
        assert_eq!(lines[5], "1|name|100|10|0|0|10|0|0|0|1|0|9|1|2023-11-14 22:13:20");
    }

    #[test]
    fn legacy_short_replies_are_just_the_status() {
        assert_eq!(legacy_leaderboard_text("-1|false"), "-1");
        assert_eq!(legacy_leaderboard_text("1|true"), "1");
    }

    #[test]
    fn request_v1_uses_old_status_numbers_and_date_strings() {
        use super::request_v1_text;
        let modern = "2|false|5|6|1\n0\n[bold:0,size:20]A|B\n10.0\n\n1|name|100|10|0|0|10|0|0|0|1|0|9|1|1700000000|1\n4260861200|x|1|1|0|0|1|0|0|0|1|0|9|2|1700000000|1";
        let out = request_v1_text(modern);
        assert_eq!(out.split('\n').nth(2).unwrap(), "A|B");
        let big = out.split('\n').nth(6).unwrap().split('|').next().unwrap().parse::<i64>().unwrap();
        assert!(big <= i32::MAX as i64 && super::real_score_id(big) == 4260861200);
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines[0], "4|false|5|6|1||");
        assert_eq!(lines[5], "1|name|100|10|0|0|10|0|0|0|1|0|9|1|2023-11-14 22:13:20|1");
        assert_eq!(request_v1_text("-1|false"), "-1|false");
        assert!(request_v1_text("0|false").starts_with("0|false|0|0|0||\n0\n"));
    }
}
