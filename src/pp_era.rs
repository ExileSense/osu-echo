
use rosu_pp_older as old;
use rosu_pp_v4::{model::mode::GameMode, Beatmap, GameMods};

pub const LIVE: &str = "live";

pub struct Era {
    pub id: &'static str,
    pub label: &'static str,
}

pub const ERAS: [Era; 13] = [
    Era { id: "may14", label: "May 2014 - July 2014" },
    Era { id: "july14", label: "July 2014 - February 2015" },
    Era { id: "feb15", label: "February 2015 - April 2015" },
    Era { id: "apr15", label: "April 2015 - May 2018" },
    Era { id: "may18", label: "May 2018 - February 2019" },
    Era { id: "feb19", label: "February 2019 - January 2021" },
    Era { id: "jan21", label: "January 2021 - July 2021" },
    Era { id: "jul21", label: "July 2021 - November 2021" },
    Era { id: "nov21", label: "November 2021 - September 2022" },
    Era { id: "sep22", label: "September 2022 - October 2024" },
    Era { id: "oct24", label: "October 2024 - March 2025" },
    Era { id: "mar25", label: "March 2025 - October 2025" },
    Era { id: LIVE, label: "October 2025 - Now (live)" },
];

pub fn is_known(id: &str) -> bool {
    ERAS.iter().any(|e| e.id == id)
}

pub fn stored_ids() -> impl Iterator<Item = &'static str> {
    ERAS.iter().map(|e| e.id).filter(|id| *id != LIVE)
}

pub fn parse(input: &str) -> Option<&'static str> {
    let t = input.trim().to_lowercase();
    if t == "now" || t == "current" {
        return Some(LIVE);
    }
    if let Some(e) = ERAS.iter().find(|e| e.id == t) {
        return Some(e.id);
    }
    let year: i32 = t.parse().ok()?;
    let year = if (0..100).contains(&year) { 2000 + year } else { year };
    Some(match year {
        ..=2014 => "may14",
        2015 => "apr15",
        2016..=2017 => "apr15",
        2018 => "may18",
        2019..=2020 => "feb19",
        2021 => "jul21",
        2022 => "nov21",
        2023 => "sep22",
        2024 => "oct24",
        2025 => "mar25",
        _ => LIVE,
    })
}

pub fn end_date(id: &str) -> Option<&'static str> {
    Some(match id {
        "may14" => "2014-06-30",
        "july14" => "2015-01-31",
        "feb15" => "2015-03-31",
        "apr15" => "2018-04-30",
        "may18" => "2019-01-31",
        "feb19" => "2020-12-31",
        "jan21" => "2021-06-30",
        "jul21" => "2021-10-31",
        "nov21" => "2022-08-31",
        "sep22" => "2024-09-30",
        "oct24" => "2025-02-28",
        "mar25" => "2025-09-30",
        _ => return None,
    })
}

pub const CLIENT_ERA_MAX_BUILD: i64 = 20201231;

pub fn era_for_build(build: i64) -> Option<&'static str> {
    if build <= 0 || build > CLIENT_ERA_MAX_BUILD {
        return None;
    }
    stored_ids().find(|id| {
        end_date(id)
            .and_then(|d| d.replace('-', "").parse::<i64>().ok())
            .map_or(false, |end| build <= end)
    })
}

#[derive(Clone, Debug, Default)]
pub struct EraScore {
    pub mode: u8,
    pub mods: u32,
    pub n300: u32,
    pub n100: u32,
    pub n50: u32,
    pub ngeki: u32,
    pub nkatu: u32,
    pub nmiss: u32,
    pub max_combo: u32,
    pub score: u32,
    pub accuracy: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Calc {
    Osu(&'static str),
    Taiko(&'static str),
    Catch(&'static str),
    Mania(&'static str),
    Live,
}

fn calculator(mode: u8, era: &str) -> Calc {
    if era == LIVE {
        return Calc::Live;
    }
    match mode {
        1 => Calc::Taiko(match era {
            "may14" | "july14" | "feb15" | "apr15" | "may18" | "feb19" => "ppv1",
            "jan21" | "jul21" | "nov21" => "2020",
            "sep22" => "2022",
            "oct24" => "2024",
            "mar25" => "2025",
            _ => return Calc::Live,
        }),
        2 => Calc::Catch(match era {
            "may14" | "july14" | "feb15" | "apr15" | "may18" | "feb19" => "ppv1",
            "jan21" | "jul21" | "nov21" | "sep22" => "2022",
            "oct24" | "mar25" => "2024",
            _ => return Calc::Live,
        }),
        3 => Calc::Mania(match era {
            "may14" | "july14" | "feb15" | "apr15" => "ppv1",
            "may18" | "feb19" | "jan21" | "jul21" | "nov21" => "2018",
            "sep22" => "2022",
            _ => return Calc::Live,
        }),
        _ => Calc::Osu(match era {
            "may14" | "july14" | "feb15" | "apr15" | "may18" | "feb19" | "jan21" | "jul21"
            | "nov21" | "sep22" | "oct24" | "mar25" => ERAS
                .iter()
                .find(|e| e.id == era)
                .map(|e| e.id)
                .unwrap_or("may14"),
            _ => return Calc::Live,
        }),
    }
}

pub struct EraMap {
    map: Beatmap,
}

impl EraMap {
    pub fn parse(content: &[u8]) -> Option<Self> {
        Beatmap::from_bytes(content).ok().map(|map| Self { map })
    }
}

fn game_mode(mode: u8) -> GameMode {
    match mode {
        1 => GameMode::Taiko,
        2 => GameMode::Catch,
        3 => GameMode::Mania,
        _ => GameMode::Osu,
    }
}

pub fn calculate(map: &EraMap, era: &str, score: &EraScore) -> Option<f64> {
    let calc = calculator(score.mode, era);
    if calc == Calc::Live {
        return None;
    }

    let mods = GameMods::from(score.mods);
    let converted = map.map.convert_ref(game_mode(score.mode), &mods).ok()?;
    let m: &Beatmap = &converted;

    let pp = match calc {
        Calc::Osu(id) => osu(m, id, score)?,
        Calc::Taiko(id) => taiko(m, id, score)?,
        Calc::Catch(id) => catch(m, id, score)?,
        Calc::Mania(id) => mania(m, id, score)?,
        Calc::Live => return None,
    };

    pp.is_finite().then_some(pp.max(0.0))
}

macro_rules! osu_classic {
    ($module:ident, $map:expr, $s:expr) => {{
        let max = old::$module::OsuPP::new($map).mods($s.mods).calculate();
        old::$module::OsuPP::new($map)
            .mods($s.mods)
            .attributes(max.difficulty)
            .n300($s.n300)
            .n100($s.n100)
            .n50($s.n50)
            .misses($s.nmiss)
            .combo($s.max_combo)
            .calculate()
            .pp as f64
    }};
}

fn osu(map: &Beatmap, id: &str, s: &EraScore) -> Option<f64> {
    Some(match id {
        "may14" => osu_classic!(osu_2014_may, map, s),
        "july14" => osu_classic!(osu_2014_july, map, s),
        "feb15" => osu_classic!(osu_2015_february, map, s),
        "apr15" => osu_classic!(osu_2015_april, map, s),
        "may18" => osu_classic!(osu_2018, map, s),
        "feb19" => osu_classic!(osu_2019, map, s),
        "jan21" => osu_classic!(osu_2021_january, map, s),
        "jul21" => osu_classic!(osu_2021_july, map, s),
        "nov21" => osu_classic!(osu_2021_november, map, s),
        "sep22" => osu_classic!(osu_2022, map, s),
        "oct24" => {
            let max = old::osu_2024::OsuPP::new(map).mods(s.mods).lazer(false).calculate();
            old::osu_2024::OsuPP::new(map)
                .mods(s.mods)
                .lazer(false)
                .attributes(max.difficulty)
                .n300(s.n300)
                .n100(s.n100)
                .n50(s.n50)
                .misses(s.nmiss)
                .combo(s.max_combo)
                .calculate()
                .pp
        }
        "mar25" => {
            let max = old::osu_2025::OsuPP::new(map)
                .mods(s.mods)
                .lazer(false)
                .calculate()
                .ok()?;
            old::osu_2025::OsuPP::new(map)
                .mods(s.mods)
                .lazer(false)
                .attributes(max.difficulty)
                .n300(s.n300)
                .n100(s.n100)
                .n50(s.n50)
                .misses(s.nmiss)
                .combo(s.max_combo)
                .calculate()
                .ok()?
                .pp
        }
        _ => return None,
    })
}

macro_rules! taiko_classic {
    ($module:ident, $map:expr, $s:expr) => {{
        let max = old::$module::TaikoPP::new($map).mods($s.mods).calculate();
        old::$module::TaikoPP::new($map)
            .mods($s.mods)
            .attributes(max.difficulty)
            .n300($s.n300)
            .n100($s.n100)
            .misses($s.nmiss)
            .combo($s.max_combo)
            .calculate()
            .pp as f64
    }};
}

fn taiko(map: &Beatmap, id: &str, s: &EraScore) -> Option<f64> {
    Some(match id {
        "ppv1" => taiko_classic!(taiko_ppv1, map, s),
        "2020" => taiko_classic!(taiko_2020, map, s),
        "2022" => taiko_classic!(taiko_2022, map, s),
        "2024" => taiko_classic!(taiko_2024, map, s),
        "2025" => {
            let max = old::taiko_2025::TaikoPP::new(map).mods(s.mods).calculate().ok()?;
            old::taiko_2025::TaikoPP::new(map)
                .mods(s.mods)
                .attributes(max.difficulty)
                .n300(s.n300)
                .n100(s.n100)
                .misses(s.nmiss)
                .combo(s.max_combo)
                .calculate()
                .ok()?
                .pp
        }
        _ => return None,
    })
}

macro_rules! catch_classic {
    ($module:ident, $map:expr, $s:expr) => {{
        let max = old::$module::FruitsPP::new($map).mods($s.mods).calculate();
        old::$module::FruitsPP::new($map)
            .mods($s.mods)
            .attributes(max.difficulty)
            .fruits($s.n300)
            .droplets($s.n100)
            .tiny_droplets($s.n50)
            .tiny_droplet_misses($s.nkatu)
            .misses($s.nmiss)
            .combo($s.max_combo)
            .calculate()
            .pp as f64
    }};
}

fn catch(map: &Beatmap, id: &str, s: &EraScore) -> Option<f64> {
    Some(match id {
        "ppv1" => catch_classic!(fruits_ppv1, map, s),
        "2022" => catch_classic!(fruits_2022, map, s),
        "2024" => {
            let max = old::fruits_2024::FruitsPP::new(map).mods(s.mods).calculate().ok()?;
            old::fruits_2024::FruitsPP::new(map)
                .mods(s.mods)
                .attributes(max.difficulty)
                .fruits(s.n300)
                .droplets(s.n100)
                .tiny_droplets(s.n50)
                .tiny_droplet_misses(s.nkatu)
                .misses(s.nmiss)
                .combo(s.max_combo)
                .calculate()
                .ok()?
                .pp
        }
        _ => return None,
    })
}

fn mania(map: &Beatmap, id: &str, s: &EraScore) -> Option<f64> {
    Some(match id {
        "ppv1" => {
            let max = old::mania_ppv1::ManiaPP::new(map).mods(s.mods).calculate();
            old::mania_ppv1::ManiaPP::new(map)
                .mods(s.mods)
                .attributes(max)
                .score(s.score)
                .accuracy(s.accuracy as f32)
                .calculate()
                .pp as f64
        }
        "2018" => {
            let max = old::mania_2018::ManiaPP::new(map).mods(s.mods).calculate();
            old::mania_2018::ManiaPP::new(map)
                .mods(s.mods)
                .attributes(max)
                .score(s.score)
                .calculate()
                .pp as f64
        }
        "2022" => {
            let max = old::mania_2022::ManiaPP::new(map).mods(s.mods).calculate();
            old::mania_2022::ManiaPP::new(map)
                .mods(s.mods)
                .attributes(max.difficulty)
                .n320(s.ngeki)
                .n300(s.n300)
                .n200(s.nkatu)
                .n100(s.n100)
                .n50(s.n50)
                .misses(s.nmiss)
                .calculate()
                .pp as f64
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> EraMap {
        let mut s = String::from("osu file format v14\n\n[General]\nAudioFilename: a.mp3\nMode: 0\n\n[Metadata]\nTitle:T\nArtist:A\nCreator:C\nVersion:V\n\n[Difficulty]\nHPDrainRate:5\nCircleSize:4\nOverallDifficulty:8\nApproachRate:9\nSliderMultiplier:1.4\nSliderTickRate:1\n\n[TimingPoints]\n0,350,4,2,0,50,1,0\n\n[HitObjects]\n");
        for i in 0..200 {
            let x = 100 + (i * 37) % 300;
            let y = 80 + (i * 53) % 220;
            s.push_str(&format!("{x},{y},{},1,0,0:0:0:0:\n", 1000 + i * 175));
        }
        EraMap::parse(s.as_bytes()).unwrap()
    }

    fn play(n300: u32, nmiss: u32) -> EraScore {
        EraScore { mode: 0, n300, nmiss, max_combo: n300, score: 500000, accuracy: 100.0, ..Default::default() }
    }

    #[test]
    fn every_stored_osu_era_gives_a_positive_finite_pp() {
        let m = map();
        for id in stored_ids() {
            let pp = calculate(&m, id, &play(200, 0)).unwrap_or_else(|| panic!("no pp for {id}"));
            assert!(pp > 0.0 && pp.is_finite(), "{id}: {pp}");
            let worse = calculate(&m, id, &play(190, 10)).unwrap();
            assert!(worse < pp, "{id}: misses should lower pp ({worse} vs {pp})");
        }
    }

    #[test]
    fn era_end_dates_exist_for_every_stored_era_and_increase() {
        let mut prev = "";
        for id in stored_ids() {
            let d = end_date(id).unwrap_or_else(|| panic!("no end date for {id}"));
            assert!(d > prev, "{id}: {d} not after {prev}");
            prev = d;
        }
        assert!(end_date(LIVE).is_none());
    }

    #[test]
    fn client_builds_map_to_the_system_of_their_time_and_modern_clients_stay_live() {
        assert_eq!(era_for_build(20130604), Some("may14"));
        assert_eq!(era_for_build(20140401), Some("may14"));
        assert_eq!(era_for_build(20140715), Some("july14"));
        assert_eq!(era_for_build(20150310), Some("feb15"));
        assert_eq!(era_for_build(20160101), Some("apr15"));
        assert_eq!(era_for_build(20181201), Some("may18"));
        assert_eq!(era_for_build(20200101), Some("feb19"));
        assert_eq!(era_for_build(20210101), None);
        assert_eq!(era_for_build(20250101), None);
        assert_eq!(era_for_build(20260601), None);
        assert_eq!(era_for_build(0), None);
    }

    #[test]
    fn live_is_not_calculated_here_and_parse_accepts_years() {
        assert!(calculate(&map(), LIVE, &play(200, 0)).is_none());
        assert_eq!(parse("2014"), Some("may14"));
        assert_eq!(parse("2016"), Some("apr15"));
        assert_eq!(parse("25"), Some("mar25"));
        assert_eq!(parse("2026"), Some(LIVE));
        assert_eq!(parse("nov21"), Some("nov21"));
        assert_eq!(parse("banana"), None);
    }
}
