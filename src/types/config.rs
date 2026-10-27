use md5::Digest;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuIcon {
    pub image_link: Option<String>,
    pub click_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paths {
    pub osu_path: Option<String>,
    pub songs: Option<String>,
    pub replay: Option<String>,
    pub screenshots: Option<String>,
    #[serde(default)]
    pub extra_songs: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub paths: Paths,
    pub pp_leaderboard: bool,
    pub ping_user_when_recent_score: bool,
    #[serde(default = "default_true")]
    pub enable_recent_channel: bool,
    pub menu_icon: MenuIcon,
    pub command_prefix: String,
    pub show_pp_for_personal_best: bool,
    pub amount_of_scores_on_lb: i32,
    pub auto_update: bool,
    pub disable_funorange_maps: bool,
    pub osu_api_key: Option<String>,
    pub osu_daily_api_key: Option<String>,
    #[serde(default)]
    pub osu_client_id: Option<String>,
    #[serde(default)]
    pub osu_client_secret: Option<String>,
    pub seasonal_bgs: Vec<String>,
    pub osu_username: Option<String>,
    pub osu_password: Option<String>,
    pub country: Option<String>,
    #[serde(default)]
    pub discord_webhook_url: Option<String>,
    #[serde(default)]
    pub discord_webhook_min_pp: Option<f64>,
    #[serde(default)]
    pub discord_bot_token: Option<String>,
    #[serde(default)]
    pub discord_application_id: Option<String>,
    #[serde(default)]
    pub discord_guild_id: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            paths: Paths { osu_path: None, songs: None, replay: None, screenshots: None, extra_songs: vec![] },
            pp_leaderboard: false,
            ping_user_when_recent_score: false,
            enable_recent_channel: true,
            menu_icon: MenuIcon { image_link: None, click_link: None },
            command_prefix: "!".to_string(),
            show_pp_for_personal_best: false,
            amount_of_scores_on_lb: 50,
            auto_update: true,
            disable_funorange_maps: false,
            osu_api_key: None,
            osu_daily_api_key: None,
            osu_client_id: None,
            osu_client_secret: None,
            seasonal_bgs: vec![],
            osu_username: None,
            osu_password: None,
            country: None,
            discord_webhook_url: None,
            discord_webhook_min_pp: None,
            discord_bot_token: None,
            discord_application_id: None,
            discord_guild_id: None,
        }
    }
}

impl Config {
    pub fn songs_folder(&self) -> Option<PathBuf> {
        self.paths.songs.as_ref().map(PathBuf::from).or_else(|| self.paths.osu_path.as_ref().map(|osu| PathBuf::from(osu).join("Songs")))
    }

    pub fn extra_songs_folders(&self) -> Vec<PathBuf> {
        self.paths.extra_songs.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).map(PathBuf::from).collect()
    }

    pub fn replay_folder(&self) -> Option<PathBuf> {
        self.paths.replay.as_ref().map(PathBuf::from).or_else(|| self.paths.osu_path.as_ref().map(|osu| PathBuf::from(osu).join("Replays")))
    }

    pub fn screenshots_folder(&self) -> Option<PathBuf> {
        self.paths.screenshots.as_ref().map(PathBuf::from).or_else(|| self.paths.osu_path.as_ref().map(|osu| PathBuf::from(osu).join("Screenshots")))
    }

    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("OSU_USERNAME") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.osu_username = Some(trimmed.to_string());
            }
        }

        if let Ok(val) = std::env::var("OSU_PASSWORD_HASH") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.osu_password = Some(trimmed.to_string());
            }
        } else if let Ok(raw_pw) = std::env::var("OSU_PASSWORD") {
            let trimmed = raw_pw.trim();
            if !trimmed.is_empty() {
                let hash = format!("{:x}", md5::Md5::digest(trimmed.as_bytes()));
                self.osu_password = Some(hash);
            }
        }

        if let Ok(val) = std::env::var("OSU_API_KEY") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.osu_api_key = Some(trimmed.to_string());
            }
        }

        if let Ok(val) = std::env::var("OSU_DAILY_API_KEY") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.osu_daily_api_key = Some(trimmed.to_string());
            }
        }

        if let Ok(val) = std::env::var("OSU_CLIENT_ID") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.osu_client_id = Some(trimmed.to_string());
            }
        }

        if let Ok(val) = std::env::var("OSU_CLIENT_SECRET") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.osu_client_secret = Some(trimmed.to_string());
            }
        }

        if let Ok(val) = std::env::var("PP_LEADERBOARD") {
            let trimmed = val.trim().to_lowercase();
            if trimmed == "true" || trimmed == "1" || trimmed == "yes" {
                self.pp_leaderboard = true;
            } else if trimmed == "false" || trimmed == "0" || trimmed == "no" {
                self.pp_leaderboard = false;
            }
        }

        if let Ok(val) = std::env::var("SHOW_PP_FOR_PERSONAL_BEST") {
            let trimmed = val.trim().to_lowercase();
            if trimmed == "true" || trimmed == "1" || trimmed == "yes" {
                self.show_pp_for_personal_best = true;
            } else if trimmed == "false" || trimmed == "0" || trimmed == "no" {
                self.show_pp_for_personal_best = false;
            }
        }

        if let Ok(val) = std::env::var("AMOUNT_OF_SCORES_ON_LB") {
            if let Ok(amount) = val.trim().parse::<i32>() {
                self.amount_of_scores_on_lb = amount.clamp(1, 100);
            }
        }

        if let Ok(val) = std::env::var("ENABLE_RECENT_CHANNEL").or_else(|_| std::env::var("RECENT_CHANNEL")) {
            let trimmed = val.trim().to_lowercase();
            if trimmed == "true" || trimmed == "1" || trimmed == "yes" {
                self.enable_recent_channel = true;
            } else if trimmed == "false" || trimmed == "0" || trimmed == "no" {
                self.enable_recent_channel = false;
            }
        }

        if let Ok(val) = std::env::var("COUNTRY") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.country = Some(trimmed.to_uppercase());
            }
        }

        if let Ok(val) = std::env::var("DISCORD_WEBHOOK_URL") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.discord_webhook_url = Some(trimmed.to_string());
            }
        }

        if let Ok(val) = std::env::var("DISCORD_WEBHOOK_MIN_PP") {
            if let Ok(min_pp) = val.trim().parse::<f64>() {
                self.discord_webhook_min_pp = Some(min_pp);
            }
        }

        if let Ok(val) = std::env::var("DISCORD_BOT_TOKEN") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.discord_bot_token = Some(trimmed.to_string());
            }
        }

        if let Ok(val) = std::env::var("DISCORD_APPLICATION_ID") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.discord_application_id = Some(trimmed.to_string());
            }
        }

        if let Ok(val) = std::env::var("DISCORD_GUILD_ID") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.discord_guild_id = Some(trimmed.to_string());
            }
        }
    }
}

pub fn detect_osu_path() -> Option<PathBuf> {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let candidate = PathBuf::from(local_app_data).join("osu!");
        if candidate.exists() && candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

pub fn write_dotenv_country(country: &str) -> std::io::Result<()> {
    write_dotenv_full(None, None, Some(country), None, None, None, None, None, None, None, None, None, None)
}

pub fn write_dotenv(
    username: Option<&str>,
    password_hash: Option<&str>,
    api_key: Option<&str>,
    daily_key: Option<&str>,
    pp_leaderboard: Option<bool>,
    show_pp_for_personal_best: Option<bool>,
    amount_of_scores: Option<i32>,
) -> std::io::Result<()> {
    write_dotenv_full(
        None,
        None,
        None,
        username,
        password_hash,
        api_key,
        daily_key,
        None,
        None,
        pp_leaderboard,
        show_pp_for_personal_best,
        amount_of_scores,
        None,
    )
}

pub fn write_dotenv_full(
    host: Option<&str>,
    port: Option<u16>,
    country: Option<&str>,
    username: Option<&str>,
    password_hash: Option<&str>,
    api_key: Option<&str>,
    daily_key: Option<&str>,
    client_id: Option<&str>,
    client_secret: Option<&str>,
    pp_leaderboard: Option<bool>,
    show_pp_for_personal_best: Option<bool>,
    amount_of_scores: Option<i32>,
    enable_recent_channel: Option<bool>,
) -> std::io::Result<()> {
    let env_path = Path::new(".env");
    let mut map = std::collections::BTreeMap::new();

    if env_path.exists() {
        if let Ok(content) = std::fs::read_to_string(env_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.is_empty() {
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once('=') {
                    map.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
    }

    if let Some(h) = host {
        map.insert("SERVER_HOST".to_string(), h.to_string());
    }
    if let Some(p) = port {
        map.insert("SERVER_PORT".to_string(), p.to_string());
    }
    if let Some(c) = country {
        map.insert("COUNTRY".to_string(), c.to_uppercase());
    }
    if let Some(u) = username {
        map.insert("OSU_USERNAME".to_string(), u.to_string());
    }
    if let Some(h) = password_hash {
        map.insert("OSU_PASSWORD_HASH".to_string(), h.to_string());
    }
    if let Some(k) = api_key {
        map.insert("OSU_API_KEY".to_string(), k.to_string());
        std::env::set_var("OSU_API_KEY", k);
    }
    if let Some(d) = daily_key {
        map.insert("OSU_DAILY_API_KEY".to_string(), d.to_string());
        std::env::set_var("OSU_DAILY_API_KEY", d);
    }
    if let Some(cid) = client_id {
        map.insert("OSU_CLIENT_ID".to_string(), cid.to_string());
        std::env::set_var("OSU_CLIENT_ID", cid);
    }
    if let Some(cs) = client_secret {
        map.insert("OSU_CLIENT_SECRET".to_string(), cs.to_string());
        std::env::set_var("OSU_CLIENT_SECRET", cs);
    }
    if let Some(pp) = pp_leaderboard {
        map.insert("PP_LEADERBOARD".to_string(), pp.to_string());
    }
    if let Some(pb) = show_pp_for_personal_best {
        map.insert("SHOW_PP_FOR_PERSONAL_BEST".to_string(), pb.to_string());
    }
    if let Some(amt) = amount_of_scores {
        map.insert("AMOUNT_OF_SCORES_ON_LB".to_string(), amt.to_string());
    }
    if let Some(recent) = enable_recent_channel {
        map.insert("ENABLE_RECENT_CHANNEL".to_string(), recent.to_string());
    }

    map.entry("SERVER_HOST".to_string()).or_insert_with(|| "127.0.0.1".to_string());
    map.entry("SERVER_PORT".to_string()).or_insert_with(|| "5000".to_string());

    let mut out = String::from("# osu-echo - Environment Configuration\n");
    out.push_str(&format!("SERVER_HOST={}\n", map.get("SERVER_HOST").unwrap()));
    out.push_str(&format!("SERVER_PORT={}\n\n", map.get("SERVER_PORT").unwrap()));

    out.push_str("# Player Profile Settings\n");
    out.push_str(&format!("COUNTRY={}\n\n", map.get("COUNTRY").cloned().unwrap_or_default()));

    out.push_str("# Essential API Keys\n");
    out.push_str("# (Stored strictly locally in this file; never sent to or shared with any remote server)\n");
    out.push_str("# Legacy v1 API key for online beatmap leaderboards: https://old.ppy.sh/p/api/\n");
    out.push_str("# (Scroll down to 'Legacy API' -> 'New Legacy API Key' -> Name: localhost, URL: https://osu-echo.local)\n");
    out.push_str(&format!("OSU_API_KEY={}\n", map.get("OSU_API_KEY").cloned().unwrap_or_default()));
    out.push_str("# osudaily API key for real-time global rank calculation: https://osudaily.net/api.php\n");
    out.push_str(&format!("OSU_DAILY_API_KEY={}\n\n", map.get("OSU_DAILY_API_KEY").cloned().unwrap_or_default()));

    out.push_str("# Official osu! API OAuth application credentials for country rankings (https://osu.ppy.sh/home/account/edit#oauth)\n");
    out.push_str(&format!("OSU_CLIENT_ID={}\n", map.get("OSU_CLIENT_ID").cloned().unwrap_or_default()));
    out.push_str(&format!("OSU_CLIENT_SECRET={}\n\n", map.get("OSU_CLIENT_SECRET").cloned().unwrap_or_default()));

    out.push_str("# Optional osu! Account Credentials (only needed for '!friend sync')\n");
    out.push_str("# (Stored strictly locally; password is MD5 hashed and never stored in plaintext)\n");
    out.push_str(&format!("OSU_USERNAME={}\n", map.get("OSU_USERNAME").cloned().unwrap_or_default()));
    out.push_str(&format!("OSU_PASSWORD_HASH={}\n\n", map.get("OSU_PASSWORD_HASH").cloned().unwrap_or_default()));

    out.push_str("# Leaderboard Settings\n");
    out.push_str("# Show PP instead of Score on beatmap leaderboards\n");
    out.push_str(&format!("PP_LEADERBOARD={}\n", map.get("PP_LEADERBOARD").cloned().unwrap_or_else(|| "false".to_string())));
    out.push_str("# Show PP in the personal best score panel\n");
    out.push_str(&format!("SHOW_PP_FOR_PERSONAL_BEST={}\n", map.get("SHOW_PP_FOR_PERSONAL_BEST").cloned().unwrap_or_else(|| "false".to_string())));
    out.push_str("# Number of scores shown on leaderboards (1-100)\n");
    out.push_str(&format!("AMOUNT_OF_SCORES_ON_LB={}\n\n", map.get("AMOUNT_OF_SCORES_ON_LB").cloned().unwrap_or_else(|| "50".to_string())));

    out.push_str("# Chat Feed Settings\n");
    out.push_str(&format!("ENABLE_RECENT_CHANNEL={}\n\n", map.get("ENABLE_RECENT_CHANNEL").cloned().unwrap_or_else(|| "true".to_string())));

    out.push_str("# Discord Webhook Integration\n");
    out.push_str("# (Posts submitted scores directly to a Discord channel via webhook)\n");
    out.push_str(&format!("DISCORD_WEBHOOK_URL={}\n", map.get("DISCORD_WEBHOOK_URL").cloned().unwrap_or_default()));
    out.push_str(&format!("DISCORD_WEBHOOK_MIN_PP={}\n", map.get("DISCORD_WEBHOOK_MIN_PP").cloned().unwrap_or_default()));

    out.push_str("\n# Discord Bot (slash commands: /rs /top /profile)\n");
    out.push_str("# Leave DISCORD_BOT_TOKEN empty to keep the bot off.\n");
    out.push_str("# DISCORD_GUILD_ID is optional: set it to register commands to one server instantly.\n");
    out.push_str(&format!("DISCORD_BOT_TOKEN={}\n", map.get("DISCORD_BOT_TOKEN").cloned().unwrap_or_default()));
    out.push_str(&format!("DISCORD_APPLICATION_ID={}\n", map.get("DISCORD_APPLICATION_ID").cloned().unwrap_or_default()));
    out.push_str(&format!("DISCORD_GUILD_ID={}\n", map.get("DISCORD_GUILD_ID").cloned().unwrap_or_default()));

    let site_theme = std::env::var("SITE_THEME").ok().or_else(|| map.get("SITE_THEME").cloned()).unwrap_or_default();
    if !site_theme.trim().is_empty() {
        out.push_str("\n# Default site theme (modern, 2019, next, 2016)\n");
        out.push_str(&format!("SITE_THEME={}\n", site_theme.trim()));
    }

    std::fs::write(env_path, out)?;
    Ok(())
}

pub fn update_env_file(pairs: &[(&str, String)]) -> std::io::Result<()> {
    let env_path = Path::new(".env");
    let content = std::fs::read_to_string(env_path).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    for (key, val) in pairs {
        let val = val.replace(['\r', '\n'], "");
        let mut found = false;
        for line in lines.iter_mut() {
            let t = line.trim_start();
            if t.starts_with('#') {
                continue;
            }
            if let Some((k, _)) = t.split_once('=') {
                if k.trim() == *key {
                    *line = format!("{}={}", key, val);
                    found = true;
                    break;
                }
            }
        }
        if !found {
            lines.push(format!("{}={}", key, val));
        }
        if val.is_empty() {
            std::env::remove_var(key);
        } else {
            std::env::set_var(key, &val);
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    std::fs::write(env_path, out)
}

pub fn install_local_trust(data_dir: &Path) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let hosts = crate::server::tls::setup_windows_hosts().map_err(|e| format!("Hosts file update failed: {}", e));
        let cert = crate::server::tls::get_or_create_certificates(data_dir)
            .map_err(|e| format!("Certificate generation failed: {}", e))
            .and_then(|paths| crate::server::tls::install_windows_trust(&paths.cert_der).map_err(|e| format!("Certificate trust failed: {}", e)));
        if !crate::server::tls::hosts_ready() {
            return Err("The hosts file could not be updated (administrator permission is needed). Close osu-echo and start it as administrator.".to_string());
        }
        hosts?;
        cert?;
        Ok("Certificate trusted and hosts file updated. If the website does not load, reload the page with CTRL + F5.".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = data_dir;
        Err("Certificate/hosts installation is only available on Windows.".to_string())
    }
}

pub fn open_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    let _ = r;
}

pub fn ensure_data_folders(data_dir: &Path) {
    for sub in ["", "avatars", "covers", "ladder"] {
        let _ = std::fs::create_dir_all(data_dir.join(sub));
    }
}

pub fn fresh_config() -> Config {
    let mut c = Config::default();
    if let Some(p) = detect_osu_path() {
        c.paths.osu_path = Some(p.to_string_lossy().to_string());
    }
    c
}

pub fn parse_port_input(input: &str, default_val: u16) -> Option<u16> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Some(default_val);
    }
    match trimmed.parse::<u16>() {
        Ok(port) if port > 0 => Some(port),
        _ => None,
    }
}

pub fn parse_number_input(input: &str, default_val: i32, min: i32, max: i32) -> Option<i32> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Some(default_val);
    }
    match trimmed.parse::<i32>() {
        Ok(n) if n >= min && n <= max => Some(n),
        _ => None,
    }
}

pub fn parse_country_code(input: &str) -> Option<String> {
    let trimmed = input.trim().to_uppercase();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.len() == 2 && trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
        Some(trimmed)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_helpers() {
        assert_eq!(parse_port_input("", 5000), Some(5000));
        assert_eq!(parse_port_input("8080", 5000), Some(8080));
        assert_eq!(parse_port_input("0", 5000), None);
        assert_eq!(parse_port_input("invalid", 5000), None);

        assert_eq!(parse_number_input("", 50, 1, 100), Some(50));
        assert_eq!(parse_number_input("25", 50, 1, 100), Some(25));
        assert_eq!(parse_number_input("0", 50, 1, 100), None);
        assert_eq!(parse_number_input("150", 50, 1, 100), None);

        assert_eq!(parse_country_code(""), None);
        assert_eq!(parse_country_code("us"), Some("US".to_string()));
        assert_eq!(parse_country_code("DE"), Some("DE".to_string()));
        assert_eq!(parse_country_code("USA"), None);
        assert_eq!(parse_country_code("12"), None);
    }

    #[test]
    fn test_paths_resolution() {
        let mut config = Config::default();
        config.paths.osu_path = Some("C:\\osu".to_string());
        assert_eq!(config.songs_folder(), Some(PathBuf::from("C:\\osu\\Songs")));
        assert_eq!(config.replay_folder(), Some(PathBuf::from("C:\\osu\\Replays")));
        assert_eq!(config.screenshots_folder(), Some(PathBuf::from("C:\\osu\\Screenshots")));

        config.paths.songs = Some("D:\\CustomSongs".to_string());
        assert_eq!(config.songs_folder(), Some(PathBuf::from("D:\\CustomSongs")));
        assert_eq!(config.replay_folder(), Some(PathBuf::from("C:\\osu\\Replays")));
    }

    #[test]
    fn test_env_overrides_and_password_hashing() {
        std::env::set_var("OSU_USERNAME", "testuser");
        std::env::set_var("OSU_PASSWORD", "secret123");
        std::env::set_var("OSU_API_KEY", "apikey_xyz");
        std::env::set_var("OSU_DAILY_API_KEY", "dailykey_123");
        std::env::set_var("OSU_CLIENT_ID", "12345");
        std::env::set_var("OSU_CLIENT_SECRET", "supersecret");
        std::env::set_var("PP_LEADERBOARD", "true");
        std::env::set_var("SHOW_PP_FOR_PERSONAL_BEST", "true");
        std::env::set_var("AMOUNT_OF_SCORES_ON_LB", "75");
        std::env::set_var("ENABLE_RECENT_CHANNEL", "false");

        let mut config = Config::default();
        assert!(!config.pp_leaderboard);
        assert!(!config.show_pp_for_personal_best);
        assert!(config.enable_recent_channel);

        config.apply_env_overrides();

        assert_eq!(config.osu_username.as_deref(), Some("testuser"));
        assert_eq!(config.osu_api_key.as_deref(), Some("apikey_xyz"));
        assert_eq!(config.osu_daily_api_key.as_deref(), Some("dailykey_123"));
        assert_eq!(config.osu_client_id.as_deref(), Some("12345"));
        assert_eq!(config.osu_client_secret.as_deref(), Some("supersecret"));
        assert!(config.pp_leaderboard);
        assert!(config.show_pp_for_personal_best);
        assert_eq!(config.amount_of_scores_on_lb, 75);
        assert!(!config.enable_recent_channel);

        let expected_hash = format!("{:x}", md5::Md5::digest(b"secret123"));
        assert_eq!(config.osu_password.as_deref(), Some(expected_hash.as_str()));

        std::env::remove_var("OSU_USERNAME");
        std::env::remove_var("OSU_PASSWORD");
        std::env::remove_var("OSU_API_KEY");
        std::env::remove_var("OSU_DAILY_API_KEY");
        std::env::remove_var("OSU_CLIENT_ID");
        std::env::remove_var("OSU_CLIENT_SECRET");
        std::env::remove_var("PP_LEADERBOARD");
        std::env::remove_var("SHOW_PP_FOR_PERSONAL_BEST");
        std::env::remove_var("AMOUNT_OF_SCORES_ON_LB");
        std::env::remove_var("ENABLE_RECENT_CHANNEL");
    }
}
