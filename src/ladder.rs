
use crate::console::TraceSend;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

pub const DATASET_URL: &str = "https://osu-api-bridge.ameo.dev/analysis/dataset?mode=0";
static BUNDLED: &[u8] = include_bytes!("../assets/ladder/osu.dataset");

pub struct Ladder {
    start_day: i64,
    days: usize,
    ranks: Vec<f64>,
    pp: Vec<u32>,
}

fn ymd_to_day(date: &str) -> Option<i64> {
    let d = chrono::NaiveDate::parse_from_str(date.get(..10)?, "%Y-%m-%d").ok()?;
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1)?;
    Some((d - epoch).num_days())
}

fn day_to_ymd(day: i64) -> String {
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    (epoch + chrono::Duration::days(day)).format("%Y-%m-%d").to_string()
}

impl Ladder {
    pub fn parse(bytes: &[u8]) -> Result<Ladder, String> {
        if bytes.len() < 32 {
            return Err("file too small".into());
        }
        let start_ts = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let days = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        if days == 0 || days > 100_000 {
            return Err("implausible day count".into());
        }
        if &bytes[16..20] != b"pco!" {
            return Err("unexpected layout (no pco segment at offset 16)".into());
        }
        let ranks_u: Vec<u32> = pco::standalone::simple_decompress(&bytes[16..]).map_err(|e| format!("ranks: {e}"))?;
        let n = ranks_u.len();
        if n < 2 {
            return Err("rank axis too short".into());
        }
        let mut found = None;
        let mut i = 20;
        while i + 4 <= bytes.len() {
            if &bytes[i..i + 4] == b"pco!" {
                if let Ok(v) = pco::standalone::simple_decompress::<u32>(&bytes[i..]) {
                    if v.len() == days * n {
                        found = Some(v);
                        break;
                    }
                }
            }
            i += 1;
        }
        let pp = found.ok_or("pp table not found")?;
        Ok(Ladder {
            start_day: (start_ts / 86_400) as i64,
            days,
            ranks: ranks_u.into_iter().map(|r| r as f64).collect(),
            pp,
        })
    }

    pub fn first_date(&self) -> String {
        day_to_ymd(self.start_day)
    }
    pub fn last_date(&self) -> String {
        day_to_ymd(self.start_day + self.days as i64 - 1)
    }

    pub fn day_index(&self, date: &str) -> Option<usize> {
        let d = ymd_to_day(date)?;
        Some((d - self.start_day).clamp(0, self.days as i64 - 1) as usize)
    }

    fn row(&self, day: usize) -> &[u32] {
        let n = self.ranks.len();
        &self.pp[day * n..(day + 1) * n]
    }

    pub fn pp_at_rank(&self, day: usize, rank: f64) -> Option<f64> {
        let day = day.min(self.days - 1);
        let row = self.row(day);
        let n = self.ranks.len();
        if rank <= self.ranks[0] {
            return Some(row[0] as f64 / 100.0);
        }
        if rank >= self.ranks[n - 1] {
            return Some(row[n - 1] as f64 / 100.0);
        }
        let i = self.ranks.partition_point(|&r| r < rank);
        let (r0, r1) = (self.ranks[i - 1], self.ranks[i]);
        let (p0, p1) = (row[i - 1] as f64 / 100.0, row[i] as f64 / 100.0);
        let t = (rank.ln() - r0.ln()) / (r1.ln() - r0.ln());
        Some(p0 + t * (p1 - p0))
    }

    pub fn rank_at_pp(&self, day: usize, pp: f64) -> Option<i32> {
        if pp <= 0.0 {
            return None;
        }
        let day = day.min(self.days - 1);
        let n = self.ranks.len();
        let mut mono: Vec<f64> = Vec::with_capacity(n);
        let mut cur = f64::MAX;
        for &v in self.row(day) {
            cur = cur.min(v as f64 / 100.0);
            mono.push(cur);
        }
        if pp >= mono[0] {
            return Some(1);
        }
        if pp < mono[n - 1] {
            return Some(self.ranks[n - 1] as i32);
        }
        let i = mono.partition_point(|&t| t > pp);
        if i == 0 {
            return Some(1);
        }
        let (hi, lo) = (mono[i - 1], mono[i]);
        let (r0, r1) = (self.ranks[i - 1], self.ranks[i]);
        if (hi - lo).abs() < f64::EPSILON {
            return Some(r1 as i32);
        }
        let t = (hi - pp) / (hi - lo);
        Some((r0.ln() + t * (r1.ln() - r0.ln())).exp().round().max(1.0) as i32)
    }

    pub fn rank_at_pp_on(&self, date: &str, pp: f64) -> Option<i32> {
        self.rank_at_pp(self.day_index(date)?, pp)
    }
    pub fn pp_at_rank_on(&self, date: &str, rank: f64) -> Option<f64> {
        self.pp_at_rank(self.day_index(date)?, rank)
    }
}

fn dir() -> PathBuf {
    std::env::current_dir().unwrap_or_default().join(".data").join("ladder")
}
fn file() -> PathBuf {
    dir().join("osu.dataset")
}

struct Cache {
    mtime: Option<SystemTime>,
    ladder: Option<Arc<Ladder>>,
    from_file: bool,
}
static CACHE: RwLock<Cache> = RwLock::new(Cache { mtime: None, ladder: None, from_file: false });

pub fn era_date(era: &str) -> Option<String> {
    match era {
        "2017" => return Some("2017-11-15".into()),
        "2019" => return Some("2019-02-25".into()),
        _ => {}
    }
    let year: i32 = era.parse().ok()?;
    if era.len() != 4 {
        return None;
    }
    let l = get()?;
    let first: i32 = l.first_date()[..4].parse().ok()?;
    let last_date = l.last_date();
    let last: i32 = last_date[..4].parse().ok()?;
    if year < first || year > last {
        return None;
    }
    let mid = format!("{year}-07-01");
    Some(if mid > last_date { last_date } else { mid })
}

pub fn valid_era(era: &str) -> bool {
    era_date(era).is_some()
}

fn month_label(date: &str) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let m: usize = date.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(7);
    format!("{} {}", M[(m.clamp(1, 12)) - 1], &date[..4])
}

pub fn era_list() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    if let Some(l) = get() {
        let first: i32 = l.first_date()[..4].parse().unwrap_or(2014);
        let last: i32 = l.last_date()[..4].parse().unwrap_or(first);
        for y in first..=last {
            let id = y.to_string();
            if let Some(d) = era_date(&id) {
                out.push((id.clone(), format!("{} ({})", id, month_label(&d)), d));
            }
        }
    } else {
        for id in ["2017", "2019"] {
            let d = era_date(id).unwrap();
            out.push((id.to_string(), format!("{} ({})", id, month_label(&d)), d));
        }
    }
    out
}

static ACTIVE_ERA: RwLock<Option<String>> = RwLock::new(None);

fn era_file() -> PathBuf {
    dir().join("active_era.txt")
}

pub fn active_era() -> Option<String> {
    ACTIVE_ERA.read().ok().and_then(|e| e.clone())
}

pub fn set_active_era(era: Option<&str>) {
    let era = era.filter(|e| valid_era(e)).map(|e| e.to_string());
    if let Ok(mut w) = ACTIVE_ERA.write() {
        *w = era.clone();
    }
    let _ = std::fs::write(era_file(), era.unwrap_or_else(|| "latest".to_string()));
}

pub fn init() {
    let d = dir();
    let _ = std::fs::create_dir_all(&d);
    let f = file();
    if !f.exists() {
        let _ = std::fs::write(&f, BUNDLED);
    }
    if let Ok(t) = std::fs::read_to_string(era_file()) {
        let t = t.trim().to_string();
        if t.len() == 4 && t.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(mut w) = ACTIVE_ERA.write() {
                *w = Some(t);
            }
        }
    }
    let readme = d.join("README.txt");
    if !readme.exists() {
        let _ = std::fs::write(
            &readme,
            "osu-echo rank ladder dataset\r\n\
             ============================\r\n\
             osu.dataset    pp->rank history for osu!standard (osu!track / Ameobea format), one row per day.\r\n\
             osu.dataset.prev   previous copy, kept when a refresh replaces the file.\r\n\
             \r\n\
             * To swap datasets: replace osu.dataset (no restart needed, it is reloaded when it changes).\r\n\
             * To refresh manually: download " ,
        );
        let _ = std::fs::OpenOptions::new().append(true).open(&readme).and_then(|mut h| {
            use std::io::Write;
            write!(
                h,
                "{DATASET_URL}\r\n  and save it over osu.dataset (or use the Refresh button in the site footer).\r\n\
                 * Auto refresh: the exe checks about every 6 hours and downloads a new copy when the file is more\r\n\
                   than 2 days old (one request). To turn it off, create an empty file called auto_refresh.off here.\r\n\
                 * If osu.dataset is missing or invalid, the copy bundled in the exe is used.\r\n"
            )
        });
    }
}

pub fn get() -> Option<Arc<Ladder>> {
    let mtime = std::fs::metadata(file()).and_then(|m| m.modified()).ok();
    {
        let c = CACHE.read().ok()?;
        if c.ladder.is_some() && c.mtime == mtime {
            return c.ladder.clone();
        }
    }
    let (ladder, from_file) = match std::fs::read(file()).ok().and_then(|b| Ladder::parse(&b).ok()) {
        Some(l) => (Some(Arc::new(l)), true),
        None => (Ladder::parse(BUNDLED).ok().map(Arc::new), false),
    };
    let mut c = CACHE.write().ok()?;
    c.mtime = mtime;
    c.ladder = ladder.clone();
    c.from_file = from_file;
    ladder
}

pub struct Info {
    pub first_date: String,
    pub last_date: String,
    pub source: &'static str,
    pub path: String,
    pub auto_refresh: bool,
}

pub fn info() -> Option<Info> {
    let l = get()?;
    let from_file = CACHE.read().ok().map(|c| c.from_file).unwrap_or(false);
    Some(Info {
        first_date: l.first_date(),
        last_date: l.last_date(),
        source: if from_file { "file" } else { "bundled" },
        path: file().display().to_string(),
        auto_refresh: !dir().join("auto_refresh.off").exists(),
    })
}

pub async fn refresh(http: &reqwest::Client) -> Result<String, String> {
    let resp = http
        .get(DATASET_URL)
        .header("User-Agent", "osu-echo-local (personal use; at most one request/day)")
        .timeout(std::time::Duration::from_secs(60))
        .send_traced()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("server answered {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("download failed: {e}"))?;
    let parsed = Ladder::parse(&bytes).map_err(|e| format!("downloaded file is not a valid dataset ({e})"))?;
    let last = parsed.last_date();
    let d = dir();
    let _ = std::fs::create_dir_all(&d);
    let tmp = d.join("osu.dataset.tmp");
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    if file().exists() {
        let _ = std::fs::copy(file(), d.join("osu.dataset.prev"));
    }
    std::fs::rename(&tmp, file()).map_err(|e| e.to_string())?;
    Ok(last)
}

pub fn spawn_refresher(http: reqwest::Client) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        loop {
            if !dir().join("auto_refresh.off").exists() {
                let stale = match get() {
                    Some(l) => {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        match (ymd_to_day(&today), ymd_to_day(&l.last_date())) {
                            (Some(t), Some(d)) => t - d > 2,
                            _ => true,
                        }
                    }
                    None => true,
                };
                if stale {
                    match refresh(&http).await {
                        Ok(last) => crate::logger::success(&format!("Rank ladder dataset refreshed (data through {last})")),
                        Err(e) => crate::logger::warn(&format!("Rank ladder refresh skipped: {e}")),
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn era_list_covers_every_year() {
        let eras = era_list();
        assert_eq!(eras.first().unwrap().0, "2014");
        assert!(eras.iter().any(|e| e.0 == "2016" && e.2 == "2016-07-01"));
        assert!(eras.iter().any(|e| e.0 == "2017" && e.2 == "2017-11-15"));
        assert!(valid_era("2026") || !valid_era("2099"));
        assert!(!valid_era("2013") && !valid_era("latest"));
    }

    #[test]
    fn bundled_parses_and_looks_up() {
        let l = Ladder::parse(BUNDLED).expect("bundled dataset parses");
        assert_eq!(l.first_date(), "2014-05-27");
        let pp = l.pp_at_rank_on("2016-08-26", 9264.0).unwrap();
        assert!((pp - 4397.29).abs() < 1.0, "pp {pp}");
        let r = l.rank_at_pp_on("2016-08-26", pp).unwrap();
        assert!((r - 9264).abs() < 50, "rank {r}");
        assert!(l.rank_at_pp_on("2019-02-25", 6000.0).unwrap() < l.rank_at_pp_on("2017-11-15", 6000.0).unwrap() * 3);
    }
}
