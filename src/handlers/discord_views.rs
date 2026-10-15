
use crate::db;
use crate::handlers::discord_bot::{author_json, avatar_url, context, message, player_info, resolve_player, Ctx, PlayerInfo};
use crate::handlers::discord_emoji;
use crate::handlers::website::ProfileResponse;
use crate::handlers::webhook::format_number;
use crate::state::SharedState;
use crate::types::beatmap::Beatmap;
use crate::types::mods::Mods;
use crate::types::score::Score;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const COLOR: u32 = 0x2e9e5a;
const PER_PAGE: usize = 10;
const MAX_TOP: usize = 100;

fn trim_num(v: f64) -> String {
    let s = format!("{:.2}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn escape_md(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '[' | ']' | '*' | '_' | '~' | '`' | '|' | '>' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

pub fn short_title(artist: &str, title: &str, version: &str) -> String {
    const LIMIT: usize = 45;
    let ver = version.chars().count() + 3;
    let full = format!("{} - {}", artist, title);
    if full.chars().count() + ver <= LIMIT {
        return format!("{} [{}]", full, version);
    }
    if title.chars().count() + ver <= LIMIT {
        return format!("{} [{}]", title, version);
    }
    let keep = (LIMIT - 6).saturating_sub(ver).max(8);
    let cut: String = title.chars().take(keep).collect();
    format!("{}... [{}]", cut.trim_end(), version)
}

fn hours_text(secs: i64) -> String {
    format!("{} hrs", format_number(secs.max(0) / 3600))
}

pub fn ago(secs: i64) -> String {
    let days = (secs.max(0)) / 86_400;
    let years = days / 365;
    if years >= 1 {
        return format!("{} year{} ago", years, if years == 1 { "" } else { "s" });
    }
    let months = days / 30;
    if months >= 1 {
        return format!("{} month{} ago", months, if months == 1 { "" } else { "s" });
    }
    if days >= 1 {
        return format!("{} day{} ago", days, if days == 1 { "" } else { "s" });
    }
    "today".to_string()
}

pub fn score_level(total: i64) -> String {
    let formula = |n: f64| 5000.0 / 3.0 * (4.0 * n.powi(3) - 3.0 * n.powi(2) - n) + 1.25 * 1.8f64.powf(n - 60.0);
    let (level, lower, upper): (i64, f64, f64);
    if total >= 26_931_190_827 {
        let l = 100 + (total - 26_931_190_827) / 99_999_999_999;
        let lo = 26_931_190_827.0 + (l - 100) as f64 * 99_999_999_999.0;
        level = l;
        lower = lo;
        upper = lo + 99_999_999_999.0;
    } else {
        let (mut l, mut lo) = (1i64, 0.0f64);
        let mut up = 0.0;
        while l < 100 {
            up = lo + (formula((l + 1) as f64) - formula(l as f64)).round();
            if (total as f64) < up {
                break;
            }
            lo = up;
            l += 1;
        }
        level = l;
        lower = lo;
        upper = up;
    }
    let progress = (((total as f64 - lower) / (upper - lower)) * 100.0).floor().clamp(0.0, 100.0) as i64;
    format!("{}.{:02}", level, progress)
}

pub struct TopRow {
    pub rank: usize,
    pub score: Score,
    pub beatmap: Option<Beatmap>,
    pub stars: Option<f64>,
}

fn mods_change_stars(mods: u32) -> bool {
    Mods::from_bits_truncate(mods).intersects(Mods::EASY | Mods::HARDROCK | Mods::DOUBLETIME | Mods::NIGHTCORE | Mods::HALFTIME | Mods::FLASHLIGHT)
}

fn stars_cache() -> &'static Mutex<HashMap<(String, u32), f64>> {
    static C: OnceLock<Mutex<HashMap<(String, u32), f64>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn stars_for(ctx: &Ctx, bmap: &Beatmap, mods: u32) -> Option<f64> {
    let stored = Some(bmap.difficultyrating).filter(|s| *s > 0.0);
    if !mods_change_stars(mods) {
        return stored;
    }
    let key = (bmap.file_md5.clone(), mods);
    if let Some(v) = stars_cache().lock().ok().and_then(|c| c.get(&key).copied()) {
        return Some(v);
    }
    let calc = async {
        let content = crate::utils::get_or_fetch_beatmap_content(&ctx.http, &ctx.db, &ctx.config, bmap).await?;
        let parsed = rosu_pp::Beatmap::from_bytes(content.as_bytes()).ok()?;
        Some(rosu_pp::Difficulty::new().mods(mods).calculate(&parsed).stars())
    };
    match tokio::time::timeout(Duration::from_secs(8), calc).await {
        Ok(Some(v)) => {
            if let Ok(mut c) = stars_cache().lock() {
                c.insert(key, v);
            }
            Some(v)
        }
        _ => stored,
    }
}

fn mods_text(score: &Score) -> String {
    format!("+{}", Mods::from_bits_truncate(score.mods).short_name())
}

fn grade_of(score: &Score) -> &'static str {
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
}

pub fn build_top_embed(who: &PlayerInfo, avatar: Option<&str>, rows: &[TopRow], page: usize, pages: usize) -> Value {
    let mut lines = Vec::new();
    for row in rows {
        let s = &row.score;
        let heading = match &row.beatmap {
            Some(b) => {
                let title = escape_md(&short_title(&b.artist, &b.title, &b.version));
                let stars = row.stars.map(|v| format!(" [{}★]", trim_num(v))).unwrap_or_default();
                if b.beatmap_id > 0 {
                    format!("**#{}** [{}](https://osu.ppy.sh/b/{}){}", row.rank, title, b.beatmap_id, stars)
                } else {
                    format!("**#{}** {}{}", row.rank, title, stars)
                }
            }
            None => format!("**#{}** Unknown beatmap ({})", row.rank, s.md5.chars().take(8).collect::<String>()),
        };
        let max_combo = row.beatmap.as_ref().map(|b| b.max_combo).filter(|m| *m > 0);
        let combo = match max_combo {
            Some(m) => format!("[**{}x**/{}x]", s.max_combo, m),
            None => format!("[**{}x**]", s.max_combo),
        };
        lines.push(heading);
        lines.push(format!(
            "{} **{}pp** ({}%) {} **{}** <t:{}:R>",
            discord_emoji::grade(grade_of(s)),
            trim_num(s.pp.unwrap_or(0.0)),
            trim_num(s.acc.unwrap_or(0.0)),
            combo,
            mods_text(s),
            s.time.max(0)
        ));
    }

    let mut embed = json!({
        "author": author_json(who),
        "description": lines.join("\n"),
        "color": COLOR,
        "footer": { "text": format!("Page {}/{} • Mode: osu!", page + 1, pages.max(1)) },
    });
    if let Some(a) = avatar {
        embed["thumbnail"] = json!({ "url": a });
    }
    embed
}

pub fn top_buttons(player: &str, page: usize, pages: usize) -> Value {
    let last = pages.saturating_sub(1);
    let btn = |role: &str, emoji: &str, target: usize, disabled: bool| {
        json!({
            "type": 2,
            "style": 3,
            "emoji": { "name": emoji },
            "custom_id": format!("top|{}|{}|{}", role, target, player),
            "disabled": disabled
        })
    };
    json!([{
        "type": 1,
        "components": [
            btn("first", "⏮️", 0, page == 0),
            btn("prev", "◀️", page.saturating_sub(1), page == 0),
            btn("next", "▶️", (page + 1).min(last), page >= last),
            btn("last", "⏭️", last, page >= last),
        ]
    }])
}

pub async fn cmd_top(state: &SharedState, username: Option<String>, page: usize) -> Value {
    let name = match resolve_player(state, username).await {
        Ok(n) => n,
        Err(e) => return message(&e),
    };
    let ctx = context(state).await;

    let all: Vec<Score> = {
        let conn = ctx.db.lock().await;
        let scores = db::get_ranked_scores(&conn, &name).unwrap_or_default();
        let mut seen = std::collections::HashSet::new();
        scores.into_iter().filter(|s| s.mode == 0).filter(|s| seen.insert(s.md5.clone())).take(MAX_TOP).collect()
    };
    if all.is_empty() {
        return message(&format!("**{}** has no top plays yet.", name));
    }
    let pages = (all.len() + PER_PAGE - 1) / PER_PAGE;
    let page = page.min(pages - 1);
    let slice: Vec<(usize, Score)> = all.into_iter().enumerate().skip(page * PER_PAGE).take(PER_PAGE).map(|(i, s)| (i + 1, s)).collect();

    let maps: Vec<Option<Beatmap>> = {
        let conn = ctx.db.lock().await;
        slice.iter().map(|(_, s)| db::get_beatmap_by_md5(&conn, &s.md5).ok().flatten()).collect()
    };
    let stars = futures_util::future::join_all(slice.iter().zip(maps.iter()).map(|((_, s), b)| {
        let ctx = &ctx;
        async move {
            match b {
                Some(b) => stars_for(ctx, b, s.mods).await,
                None => None,
            }
        }
    }))
    .await;

    let rows: Vec<TopRow> = slice
        .into_iter()
        .zip(maps)
        .zip(stars)
        .map(|(((rank, score), beatmap), stars)| TopRow { rank, score, beatmap, stars })
        .collect();

    let (who, avatar) = tokio::join!(player_info(&ctx, &name), avatar_url(&ctx, &name));
    json!({
        "embeds": [build_top_embed(&who, avatar.as_deref(), &rows, page, pages)],
        "components": top_buttons(&name, page, pages)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileView {
    Compact,
    Stats,
}

fn view_key(v: ProfileView) -> &'static str {
    match v {
        ProfileView::Compact => "compact",
        ProfileView::Stats => "stats",
    }
}

fn profile_select(player: &str, view: ProfileView) -> Value {
    json!([{
        "type": 1,
        "components": [{
            "type": 3,
            "custom_id": format!("prof|{}", player),
            "options": [
                { "label": "Compact", "value": "compact", "description": "Compact user statistics", "default": view == ProfileView::Compact },
                { "label": "User Statistics", "value": "stats", "description": "Extended user statistics", "default": view == ProfileView::Stats }
            ]
        }]
    }])
}

fn joined_footer(p: &ProfileResponse, now: i64) -> Option<String> {
    let jd = p.details.join_date.trim();
    if jd.len() >= 7 {
        let (y, m) = (jd.get(0..4)?, jd.get(5..7)?);
        let date = chrono::NaiveDate::from_ymd_opt(y.parse().ok()?, m.parse().ok()?, 1)?;
        let ts = date.and_hms_opt(0, 0, 0)?.and_utc().timestamp();
        return Some(format!("Joined osu! {}-{} ({})", y, m, ago(now - ts)));
    }
    let ts = p.first_play.filter(|t| *t > 0)?;
    let dt = chrono::DateTime::from_timestamp(ts, 0)?;
    Some(format!("Joined osu! {} ({})", dt.format("%Y-%m-%d %H:%M:%S"), ago(now - ts)))
}

fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    let d = chrono::NaiveDate::parse_from_str(s.get(0..10)?, "%Y-%m-%d").ok()?;
    use chrono::Datelike;
    Some((d.year(), d.month(), d.day()))
}

pub fn build_compact_embed(who: &PlayerInfo, avatar: Option<&str>, p: &ProfileResponse, now: i64) -> Value {
    let mut lines = vec![
        format!("Accuracy: `{:.2}%` • Level: `{}`", p.acc, score_level(p.total_score)),
        if p.play_time_secs >= 60 {
            format!("Playcount: `{}` (`{}`)", format_number(p.playcount as i64), hours_text(p.play_time_secs))
        } else {
            format!("Playcount: `{}`", format_number(p.playcount as i64))
        },
        format!("Medals: `{}`", p.medals.len()),
    ];
    if let Some(h) = &p.rank_highest {
        if let Some((y, m, d)) = parse_ymd(&h.updated_at) {
            lines.push(format!("Peak rank: `#{}` ({}/{}/{})", format_number(h.rank as i64), m, d, y));
        } else {
            lines.push(format!("Peak rank: `#{}`", format_number(h.rank as i64)));
        }
    }
    let mut embed = json!({
        "author": author_json(who),
        "description": lines.join("\n"),
        "color": COLOR,
    });
    if let Some(a) = avatar {
        embed["thumbnail"] = json!({ "url": a });
    }
    if let Some(f) = joined_footer(p, now) {
        embed["footer"] = json!({ "text": f });
    }
    embed
}

pub fn build_stats_embed(who: &PlayerInfo, avatar: Option<&str>, p: &ProfileResponse, bonus_pp: Option<f64>, now: i64) -> Value {
    let g = |k: &str| p.grades.get(k).copied().unwrap_or(0);
    let mut fields: Vec<(String, String, bool)> = Vec::new();
    let mut add = |name: &str, value: String| fields.push((name.to_string(), value, true));

    if let Some(h) = &p.rank_highest {
        let when = h.updated_at.get(2..7).map(|s| s.replace('-', "/")).unwrap_or_default();
        add("Peak rank", format!("#{} ('{})", format_number(h.rank as i64), when));
    }
    if let Some(top) = p.top.first() {
        add("Top score PP", format!("{:.2}pp", top.pp));
    }
    add("Level", score_level(p.total_score));
    add("Total score", format_number(p.total_score));
    add("Total hits", format_number(p.total_hits));
    if let Some(b) = bonus_pp {
        add("Bonus PP", format!("{:.2}pp", b));
    }
    add("Ranked score", format_number(p.ranked_score));
    if p.playcount > 0 {
        add("Hits per play", format!("{:.2}", p.total_hits as f64 / p.playcount as f64));
    }
    add("Accuracy", format!("{:.2}%", p.acc));
    add("Max combo", format_number(p.max_combo as i64));
    add("Medals", p.medals.len().to_string());
    add("First places", p.first_places.len().to_string());
    if p.play_time_secs >= 60 {
        add("Play count / time", format!("{} / {}", format_number(p.playcount as i64), hours_text(p.play_time_secs)));
    } else {
        add("Play count", format_number(p.playcount as i64));
    }
    add(
        "Combined grades",
        format!("{} {} {} {}", discord_emoji::grade("X"), g("XH") + g("X"), discord_emoji::grade("S"), g("SH") + g("S")),
    );

    let mut json_fields: Vec<Value> = fields.into_iter().map(|(n, v, inline)| json!({ "name": n, "value": v, "inline": inline })).collect();
    json_fields.push(json!({
        "name": "Grades",
        "value": format!(
            "{} {} {} {} {} {} {} {} {} {}",
            discord_emoji::grade("XH"), g("XH"),
            discord_emoji::grade("X"), g("X"),
            discord_emoji::grade("SH"), g("SH"),
            discord_emoji::grade("S"), g("S"),
            discord_emoji::grade("A"), g("A")
        ),
        "inline": false
    }));

    let mut embed = json!({
        "author": author_json(who),
        "description": "__User statistics:__",
        "color": COLOR,
        "fields": json_fields,
    });
    if let Some(a) = avatar {
        embed["thumbnail"] = json!({ "url": a });
    }
    if let Some(f) = joined_footer(p, now) {
        embed["footer"] = json!({ "text": f });
    }
    embed
}

pub async fn cmd_profile(state: &SharedState, username: Option<String>, view: ProfileView) -> Value {
    let name = match resolve_player(state, username).await {
        Ok(n) => n,
        Err(e) => return message(&e),
    };
    let ctx = context(state).await;

    let mut params = HashMap::new();
    params.insert("name".to_string(), name.clone());
    params.insert("mode".to_string(), "osu".to_string());
    let resp = match tokio::time::timeout(Duration::from_secs(30), crate::handlers::website::handle_profile(state.clone(), &params)).await {
        Ok(r) => r,
        Err(_) => return message("Loading that profile took too long, try again in a moment."),
    };
    let profile: ProfileResponse = match serde_json::from_slice(&resp.body) {
        Ok(p) if resp.status.is_success() => p,
        _ => return message(&format!("Couldn't load the profile of **{}**.", name)),
    };

    let (who, avatar) = tokio::join!(player_info(&ctx, &name), avatar_url(&ctx, &name));
    let now = chrono::Utc::now().timestamp();
    let embed = match view {
        ProfileView::Compact => build_compact_embed(&who, avatar.as_deref(), &profile, now),
        ProfileView::Stats => build_stats_embed(&who, avatar.as_deref(), &profile, crate::osu_api::bonus(&name, 0), now),
    };
    json!({ "embeds": [embed], "components": profile_select(&name, view) })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Component {
    Top { player: String, page: usize },
    Profile { player: String, view: ProfileView },
}

pub fn parse_component(custom_id: &str, values: &[String]) -> Option<Component> {
    let mut parts = custom_id.splitn(4, '|');
    match parts.next()? {
        "top" => {
            let _role = parts.next()?;
            let page = parts.next()?.parse().ok()?;
            let player = parts.next()?.to_string();
            Some(Component::Top { player, page })
        }
        "prof" => {
            let player = parts.next()?.to_string();
            let view = match values.first().map(|s| s.as_str()) {
                Some("stats") => ProfileView::Stats,
                _ => ProfileView::Compact,
            };
            Some(Component::Profile { player, view })
        }
        _ => None,
    }
}

pub async fn run_component(state: &SharedState, action: Component) -> Value {
    match action {
        Component::Top { player, page } => cmd_top(state, Some(player), page).await,
        Component::Profile { player, view } => cmd_profile(state, Some(player), view).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who() -> PlayerInfo {
        PlayerInfo { name: "Rohulk".into(), pp: 13236.2, country: "RO".into(), global_rank: Some(1416), country_rank: Some(15) }
    }

    fn score(pp: f64, acc: f64, combo: i32, mods: u32) -> Score {
        serde_json::from_value(json!({
            "mode": 0, "md5": "0123456789abcdef", "name": "Rohulk", "n300": 1900, "n100": 50, "n50": 0,
            "ngeki": 0, "nkatu": 0, "nmiss": 0, "score": 1000, "max_combo": combo,
            "perfect": true, "mods": mods, "time": 1_600_000_000, "acc": acc, "pp": pp,
            "replay_md5": null, "scoreid": null, "mods_str": null,
            "additional_mods": null, "submission_checksum": null, "submission_identity": null
        }))
        .unwrap()
    }

    fn bmap(artist: &str, title: &str, version: &str, id: i64, max_combo: i32) -> Beatmap {
        let mut b = Beatmap::blank();
        b.artist = artist.into();
        b.title = title.into();
        b.version = version.into();
        b.beatmap_id = id;
        b.max_combo = max_combo;
        b
    }

    #[test]
    fn level_matches_bathbots_value_for_the_same_total_score() {
        assert_eq!(score_level(660_389_373_708), "106.33");
        assert_eq!(score_level(0), "1.00");
        assert_eq!(score_level(29_999), "1.99");
        assert_eq!(score_level(30_000), "2.00");
    }

    #[test]
    fn titles_are_shortened_like_bathbots_list() {
        assert_eq!(short_title("xi", "Akasha", "Aether"), "xi - Akasha [Aether]");
        assert_eq!(short_title("Avenged Sevenfold", "Save Me", "Nightmare"), "Avenged Sevenfold - Save Me [Nightmare]");
        assert_eq!(short_title("A Very Long Artist Name Here", "To The Terminus", "Skystar's Extra"), "To The Terminus [Skystar's Extra]");
        let t = short_title("Artist", "Pastel Subliminal & Synesthesia Extended Mix", "Stargazer");
        assert!(t.starts_with("Pastel Subliminal & Synesth..."), "{}", t);
        assert!(t.ends_with("[Stargazer]"));
    }

    #[test]
    fn markdown_in_titles_is_escaped() {
        assert_eq!(escape_md("a [b] *c*"), "a \\[b\\] \\*c\\*");
    }

    #[test]
    fn top_embed_has_two_lines_per_play_and_a_page_footer() {
        let rows = vec![
            TopRow { rank: 1, score: score(690.73, 99.76, 1959, 0), beatmap: Some(bmap("xi", "Akasha", "Aether", 55, 1959)), stars: Some(7.5) },
            TopRow { rank: 2, score: score(660.3, 99.5, 1500, Mods::HIDDEN.bits() | Mods::HARDROCK.bits()), beatmap: None, stars: None },
        ];
        let e = build_top_embed(&who(), Some("https://a.ppy.sh/1"), &rows, 0, 20);
        let d = e["description"].as_str().unwrap();
        let l: Vec<&str> = d.lines().collect();
        assert_eq!(l.len(), 4);
        assert_eq!(l[0], "**#1** [xi - Akasha [Aether]](https://osu.ppy.sh/b/55) [7.5★]".replace("[Aether]](", "\\[Aether\\]](").replace("Akasha [", "Akasha \\["));
        assert!(l[1].contains("**690.73pp** (99.76%) [**1959x**/1959x] **+NM** <t:1600000000:R>"), "{}", l[1]);
        assert!(l[2].starts_with("**#2** Unknown beatmap"));
        assert!(l[3].contains("**660.3pp** (99.5%) [**1500x**]"));
        assert_eq!(e["footer"]["text"], "Page 1/20 • Mode: osu!");
        assert_eq!(e["thumbnail"]["url"], "https://a.ppy.sh/1");
        assert_eq!(e["author"]["name"], "Rohulk: 13,236.20pp (#1,416 RO15)");
    }

    #[test]
    fn buttons_have_unique_ids_and_disable_at_the_edges() {
        let row = &top_buttons("Rohulk", 0, 20)[0]["components"];
        let ids: Vec<&str> = row.as_array().unwrap().iter().map(|b| b["custom_id"].as_str().unwrap()).collect();
        let mut dedup = ids.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(dedup.len(), 4, "ids must be unique: {:?}", ids);
        assert_eq!(row[0]["disabled"], true);
        assert_eq!(row[1]["disabled"], true);
        assert_eq!(row[2]["disabled"], false);
        let mid = &top_buttons("Rohulk", 5, 20)[0]["components"];
        assert_eq!(mid[1]["custom_id"], "top|prev|4|Rohulk");
        assert_eq!(mid[2]["custom_id"], "top|next|6|Rohulk");
        assert_eq!(mid[3]["custom_id"], "top|last|19|Rohulk");
        let end = &top_buttons("Rohulk", 19, 20)[0]["components"];
        assert_eq!(end[2]["disabled"], true);
        assert_eq!(end[3]["disabled"], true);
        let one = &top_buttons("Rohulk", 0, 1)[0]["components"];
        assert!(one.as_array().unwrap().iter().all(|b| b["disabled"] == true));
    }

    #[test]
    fn components_round_trip() {
        assert_eq!(parse_component("top|next|6|Rohulk", &[]), Some(Component::Top { player: "Rohulk".into(), page: 6 }));
        assert_eq!(
            parse_component("top|first|0|Some Name", &[]),
            Some(Component::Top { player: "Some Name".into(), page: 0 })
        );
        assert_eq!(
            parse_component("prof|Rohulk", &["stats".to_string()]),
            Some(Component::Profile { player: "Rohulk".into(), view: ProfileView::Stats })
        );
        assert_eq!(
            parse_component("prof|Rohulk", &["compact".to_string()]),
            Some(Component::Profile { player: "Rohulk".into(), view: ProfileView::Compact })
        );
        assert_eq!(parse_component("nonsense", &[]), None);
        assert_eq!(parse_component("top|next|x|Rohulk", &[]), None);
    }

    fn profile() -> ProfileResponse {
        serde_json::from_value(json!({
            "name": "Rohulk", "mode": "osu", "pp": 13236, "acc": 99.38, "playcount": 68392,
            "total_score": 660_389_373_708i64, "ranked_score": 158_668_328_193i64, "total_hits": 48_019_717i64,
            "max_combo": 9032,
            "grades": {"XH": 95, "X": 8, "SH": 1998, "S": 667, "A": 4572},
            "active": false, "details": {"join_date": "2013-08"},
            "country_rank": 15, "global_rank": 1416, "rank_history": [],
            "top": [{"id": "1", "pp": 690.73, "acc": 99.76, "score": 1, "max_combo": 1, "time": 1, "n300": 1, "n100": 0, "n50": 0, "nmiss": 0,
                     "grade": "S", "mods": "HDHR", "beatmap": {"title": "t", "artist": "a", "version": "v", "creator": "c", "beatmap_id": 1, "beatmapset_id": 1}}],
            "recent": [], "last_play": null, "performance_history": [], "play_history": [], "most_played": [], "recent_24h": [],
            "rank_highest": {"rank": 8, "updated_at": "2017-08-28"}
        }))
        .expect("profile json")
    }

    #[test]
    fn compact_profile_follows_the_bathbot_layout() {
        let now = 1_791_700_000;
        let e = build_compact_embed(&who(), None, &profile(), now);
        let d = e["description"].as_str().unwrap();
        assert_eq!(
            d,
            "Accuracy: `99.38%` • Level: `106.33`\nPlaycount: `68,392`\nMedals: `0`\nPeak rank: `#8` (8/28/2017)"
        );
        assert_eq!(e["footer"]["text"], "Joined osu! 2013-08 (13 years ago)");
        assert!(e.get("thumbnail").is_none());
    }

    #[test]
    fn stats_profile_lists_the_extended_fields() {
        let e = build_stats_embed(&who(), Some("https://a.ppy.sh/1"), &profile(), Some(413.89), 1_791_700_000);
        assert_eq!(e["description"], "__User statistics:__");
        let fields = e["fields"].as_array().unwrap();
        let get = |n: &str| fields.iter().find(|f| f["name"] == n).map(|f| f["value"].as_str().unwrap().to_string());
        assert_eq!(get("Peak rank").unwrap(), "#8 ('17/08)");
        assert_eq!(get("Top score PP").unwrap(), "690.73pp");
        assert_eq!(get("Level").unwrap(), "106.33");
        assert_eq!(get("Total score").unwrap(), "660,389,373,708");
        assert_eq!(get("Bonus PP").unwrap(), "413.89pp");
        assert_eq!(get("Hits per play").unwrap(), "702.12");
        assert_eq!(get("Accuracy").unwrap(), "99.38%");
        assert!(get("Combined grades").unwrap().contains("103") && get("Combined grades").unwrap().contains("2665"));
        assert!(get("Grades").unwrap().contains("4572"));
        assert_eq!(fields.last().unwrap()["name"], "Grades");
        assert_eq!(e["thumbnail"]["url"], "https://a.ppy.sh/1");
    }

    #[test]
    fn play_time_shows_in_both_profile_views_once_tracked() {
        let mut p = profile();
        p.play_time_secs = 2131 * 3600 + 900;
        let c = build_compact_embed(&who(), None, &p, 1_791_700_000);
        assert!(c["description"].as_str().unwrap().contains("Playcount: `68,392` (`2,131 hrs`)"));
        let s = build_stats_embed(&who(), None, &p, None, 1_791_700_000);
        let f = s["fields"].as_array().unwrap();
        assert!(f.iter().any(|x| x["name"] == "Play count / time" && x["value"] == "68,392 / 2,131 hrs"));
        p.play_time_secs = 30;
        assert!(!build_compact_embed(&who(), None, &p, 1).to_string().contains("hrs"));
    }

    #[test]
    fn profile_select_marks_the_current_view() {
        let s = &profile_select("Rohulk", ProfileView::Stats)[0]["components"][0];
        assert_eq!(s["custom_id"], "prof|Rohulk");
        assert_eq!(s["options"][0]["default"], false);
        assert_eq!(s["options"][1]["default"], true);
        assert_eq!(view_key(ProfileView::Compact), "compact");
    }

    #[test]
    fn relative_time_text() {
        assert_eq!(ago(0), "today");
        assert_eq!(ago(3 * 86_400), "3 days ago");
        assert_eq!(ago(65 * 86_400), "2 months ago");
        assert_eq!(ago(13 * 365 * 86_400 + 5), "13 years ago");
        assert_eq!(ago(365 * 86_400), "1 year ago");
    }
}
