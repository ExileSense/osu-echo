use hyper::StatusCode;
use rust_embed::RustEmbed;
use std::path::PathBuf;
use crate::server::response::Response;

#[derive(RustEmbed)]
#[folder = "website/static/"]
pub struct EmbeddedAssets;

pub fn clean_path(path: &str) -> String {
    let path = path.trim_start_matches('/').replace('\\', "/");
    let mut parts = Vec::new();
    for seg in path.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            return String::new();
        }
        parts.push(seg);
    }
    parts.join("/")
}

pub fn get_asset(path: &str) -> Option<(Vec<u8>, String)> {
    let clean = clean_path(path);
    if clean.is_empty() {
        return None;
    }

    let local_path = PathBuf::from("website/static").join(&clean);
    if let Ok(canon_base) = std::fs::canonicalize("website/static") {
        if let Ok(canon_file) = std::fs::canonicalize(&local_path) {
            if canon_file.starts_with(&canon_base) && canon_file.is_file() {
                if let Ok(data) = std::fs::read(&canon_file) {
                    let mime = mime_guess::from_path(&clean).first_or_octet_stream().to_string();
                    return Some((data, mime));
                }
            }
        }
    }

    if let Some(file) = EmbeddedAssets::get(&clean) {
        let mime = mime_guess::from_path(&clean).first_or_octet_stream().to_string();
        return Some((file.data.into_owned(), mime));
    }

    None
}

pub fn serve_asset(path: &str) -> Response {
    let clean = clean_path(path);
    if let Some((data, mime)) = get_asset(&clean) {
        let is_html = clean == "index.html" || mime.starts_with("text/html");
        let mut resp = Response::new(data)
            .with_header("Content-Type", if is_html { "text/html; charset=utf-8" } else { &mime })
            .with_header("X-Content-Type-Options", "nosniff");

        if is_html || clean == "profile.js" || clean == "profile.css" || clean == "home.js" || clean == "home.css" {
            resp = resp
                .with_header("Cache-Control", "no-cache");
            if is_html {
                resp = resp.with_header(
                    "Content-Security-Policy",
                    "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data:; font-src 'self' https://osu.ppy.sh; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
                );
            }
        } else {
            resp = resp.with_header("Cache-Control", "public, max-age=86400");
        }
        resp
    } else {
        Response::empty().with_status(StatusCode::NOT_FOUND)
    }
}

fn valid_theme_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn cookie_value(cookie_header: Option<&str>, name: &str) -> Option<String> {
    cookie_header?
        .split(';')
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v.trim().to_string())
}

fn theme_exists(id: &str) -> bool {
    id == "modern" || (valid_theme_id(id) && get_asset(&format!("themes/{}/index.html", id)).is_some())
}

pub fn list_themes() -> Vec<(String, String, String)> {
    let mut ids: Vec<String> = Vec::new();
    for path in EmbeddedAssets::iter() {
        if let Some(rest) = path.strip_prefix("themes/") {
            if let Some((id, file)) = rest.split_once('/') {
                if file == "index.html" && valid_theme_id(id) {
                    ids.push(id.to_string());
                }
            }
        }
    }
    if let Ok(dir) = std::fs::read_dir("website/static/themes") {
        for entry in dir.flatten() {
            let id = entry.file_name().to_string_lossy().to_string();
            if valid_theme_id(&id) && entry.path().join("index.html").is_file() {
                ids.push(id);
            }
        }
    }
    ids.sort();
    ids.dedup();
    let mut out = vec![("modern".to_string(), "Modern (default)".to_string(), "The current osu!web style".to_string())];
    for id in ids {
        let meta = get_asset(&format!("themes/{}/theme.json", id))
            .and_then(|(data, _)| serde_json::from_slice::<serde_json::Value>(&data).ok());
        let name = meta.as_ref().and_then(|m| m["name"].as_str()).unwrap_or(&id).to_string();
        let desc = meta.as_ref().and_then(|m| m["description"].as_str()).unwrap_or("").to_string();
        out.push((id, name, desc));
    }
    out
}

pub fn active_theme(cookie_header: Option<&str>) -> String {
    if let Some(c) = cookie_value(cookie_header, "site_theme") {
        let c = c.to_lowercase();
        if theme_exists(&c) {
            return c;
        }
    }
    let env = std::env::var("SITE_THEME").unwrap_or_default().trim().to_lowercase();
    if theme_exists(&env) { env } else { "modern".to_string() }
}

pub fn serve_themes(cookie_header: Option<&str>) -> Response {
    let themes: Vec<serde_json::Value> = list_themes()
        .into_iter()
        .map(|(id, name, description)| serde_json::json!({"id": id, "name": name, "description": description}))
        .collect();
    Response::json(&serde_json::json!({"themes": themes, "current": active_theme(cookie_header)}))
        .with_header("Cache-Control", "no-store")
}

pub fn serve_index(cookie_header: Option<&str>) -> Response {
    let theme = active_theme(cookie_header);
    if theme != "modern" {
        let resp = serve_asset(&format!("themes/{}/index.html", theme));
        if resp.status.as_u16() != 404 {
            return resp.with_header("Vary", "Cookie");
        }
    }
    serve_asset("index.html").with_header("Vary", "Cookie")
}

pub fn serve_themed(cookie_header: Option<&str>, file: &str) -> Response {
    let theme = active_theme(cookie_header);
    if theme != "modern" {
        let resp = serve_asset(&format!("themes/{}/{}", theme, file));
        if resp.status.as_u16() != 404 {
            return resp.with_header("Vary", "Cookie");
        }
    }
    serve_asset(file).with_header("Vary", "Cookie")
}

pub fn serve_home(cookie_header: Option<&str>) -> Response {
    let theme = active_theme(cookie_header);
    if theme != "modern" {
        let resp = serve_asset(&format!("themes/{}/home.html", theme));
        if resp.status.as_u16() != 404 {
            return resp.with_header("Vary", "Cookie");
        }
    }
    serve_asset("home.html").with_header("Vary", "Cookie")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_path_security() {
        assert_eq!(clean_path("index.html"), "index.html");
        assert_eq!(clean_path("/vendor/osu-web.css"), "vendor/osu-web.css");
        assert_eq!(clean_path("vendor\\osu-web.css"), "vendor/osu-web.css");
        assert_eq!(clean_path("../Cargo.toml"), "");
        assert_eq!(clean_path("vendor/../../Cargo.toml"), "");
        assert_eq!(clean_path(""), "");
    }

    #[test]
    fn test_embedded_assets_exist() {
        assert!(EmbeddedAssets::get("index.html").is_some());
        assert!(EmbeddedAssets::get("home.html").is_some());
        assert!(EmbeddedAssets::get("profile.css").is_some());
        assert!(EmbeddedAssets::get("profile.js").is_some());
        assert!(EmbeddedAssets::get("vendor/osu-web.css").is_some());
        assert!(EmbeddedAssets::get("vendor/avatar-guest@2x.01495bc4.png").is_some());
    }

    #[test]
    fn test_serve_asset_response() {
        let resp = serve_index(None);
        assert_eq!(resp.status, StatusCode::OK);
        assert!(resp.headers.iter().any(|(k, v)| k == "Content-Type" && v.contains("text/html")));
        assert!(resp.headers.iter().any(|(k, v)| k == "X-Content-Type-Options" && v == "nosniff"));

        let not_found = serve_asset("nonexistent_file.xyz");
        assert_eq!(not_found.status, StatusCode::NOT_FOUND);

        let traversal = serve_asset("../Cargo.toml");
        assert_eq!(traversal.status, StatusCode::NOT_FOUND);
    }
}
