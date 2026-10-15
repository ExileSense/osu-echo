use crate::db;
use crate::types::beatmap::Beatmap;
use crate::types::mods::Mods;
use crate::types::score::Score;
use rusqlite::Connection;
use std::collections::HashSet;

pub fn evaluate_score_submission(
    score: &Score,
    bmap: &Beatmap,
    parsed_map: Option<&rosu_pp::Beatmap>,
    playcount: i32,
    mode_total_hits: i64,
    existing_medals: &HashSet<i32>,
    passed: bool,
) -> Vec<i32> {
    let mut newly_earned = Vec::new();
    let mut check_and_award = |id: i32| {
        if !existing_medals.contains(&id) && !newly_earned.contains(&id) {
            newly_earned.push(id);
        }
    };

    let playcount_milestones = [
        (5_000, 20),
        (15_000, 21),
        (25_000, 22),
        (50_000, 28),
    ];
    for &(threshold, medal_id) in &playcount_milestones {
        if playcount >= threshold {
            check_and_award(medal_id);
        }
    }

    let hit_milestones: &[(i64, i32)] = match score.mode {
        1 => &[
            (30_000, 31),
            (300_000, 32),
            (3_000_000, 33),
            (30_000_000, 291),
        ],
        2 => &[
            (20_000, 13),
            (200_000, 23),
            (2_000_000, 24),
            (20_000_000, 292),
        ],
        3 => &[
            (40_000, 46),
            (400_000, 47),
            (4_000_000, 48),
            (40_000_000, 293),
        ],
        _ => &[],
    };
    for &(threshold, medal_id) in hit_milestones {
        if mode_total_hits >= threshold {
            check_and_award(medal_id);
        }
    }

    if !passed {
        return newly_earned;
    }

    if bmap.approved != 1 && bmap.approved != 2 {
        return newly_earned;
    }

    let score_mods = Mods::from_bits_truncate(score.mods);
    let is_unranked_mods = score_mods.intersects(Mods::RELAX | Mods::AUTOPILOT | Mods::CINEMA);

    if is_unranked_mods {
        return newly_earned;
    }

    if score.mode == 0 {
        let combo_milestones = [
            (500, 1),
            (750, 3),
            (1000, 4),
            (2000, 5),
        ];
        for &(threshold, medal_id) in &combo_milestones {
            if score.max_combo >= threshold {
                check_and_award(medal_id);
            }
        }
    }

    let game_mode = match score.mode {
        0 => rosu_pp::model::mode::GameMode::Osu,
        1 => rosu_pp::model::mode::GameMode::Taiko,
        2 => rosu_pp::model::mode::GameMode::Catch,
        3 => rosu_pp::model::mode::GameMode::Mania,
        _ => rosu_pp::model::mode::GameMode::Osu,
    };
    let stars = if score.mods == 0 && bmap.difficultyrating > 0.0 {
        bmap.difficultyrating
    } else if let Some(parsed) = parsed_map {
        rosu_pp::Performance::new(parsed)
            .mode_or_ignore(game_mode)
            .mods(score.mods)
            .calculate()
            .difficulty_attributes()
            .stars()
    } else {
        bmap.difficultyrating
    };

    let pass_table: &[(f64, f64, i32)] = match score.mode {
        0 => &[
            (1.0, 2.0, 55), (2.0, 3.0, 56), (3.0, 4.0, 57), (4.0, 5.0, 58), (5.0, 6.0, 59),
            (6.0, 7.0, 60), (7.0, 8.0, 61), (8.0, 9.0, 62), (9.0, 10.0, 242), (10.0, f64::MAX, 244),
        ],
        1 => &[
            (1.0, 2.0, 71), (2.0, 3.0, 72), (3.0, 4.0, 73), (4.0, 5.0, 74), (5.0, 6.0, 75),
            (6.0, 7.0, 76), (7.0, 8.0, 77), (8.0, f64::MAX, 78),
        ],
        2 => &[
            (1.0, 2.0, 79), (2.0, 3.0, 80), (3.0, 4.0, 81), (4.0, 5.0, 82), (5.0, 6.0, 83),
            (6.0, 7.0, 84), (7.0, 8.0, 85), (8.0, f64::MAX, 86),
        ],
        3 => &[
            (1.0, 2.0, 87), (2.0, 3.0, 88), (3.0, 4.0, 89), (4.0, 5.0, 90), (5.0, 6.0, 91),
            (6.0, 7.0, 92), (7.0, 8.0, 93), (8.0, f64::MAX, 94),
        ],
        _ => &[],
    };
    for &(min_stars, max_stars, medal_id) in pass_table {
        if stars >= min_stars && stars < max_stars {
            check_and_award(medal_id);
        }
    }

    let is_fc = score.nmiss == 0 && match score.mode {
        1 | 3 => true,
        _ => score.perfect || bmap.max_combo == 0 || score.max_combo >= bmap.max_combo - 1,
    };

    if is_fc {
        let fc_table: &[(f64, f64, i32)] = match score.mode {
            0 => &[
                (1.0, 2.0, 63), (2.0, 3.0, 64), (3.0, 4.0, 65), (4.0, 5.0, 66), (5.0, 6.0, 67),
                (6.0, 7.0, 68), (7.0, 8.0, 69), (8.0, 9.0, 70), (9.0, 10.0, 243), (10.0, f64::MAX, 245),
            ],
            1 => &[
                (1.0, 2.0, 95), (2.0, 3.0, 96), (3.0, 4.0, 97), (4.0, 5.0, 98), (5.0, 6.0, 99),
                (6.0, 7.0, 100), (7.0, 8.0, 101), (8.0, f64::MAX, 102),
            ],
            2 => &[
                (1.0, 2.0, 103), (2.0, 3.0, 104), (3.0, 4.0, 105), (4.0, 5.0, 106), (5.0, 6.0, 107),
                (6.0, 7.0, 108), (7.0, 8.0, 109), (8.0, f64::MAX, 110),
            ],
            3 => &[
                (1.0, 2.0, 111), (2.0, 3.0, 112), (3.0, 4.0, 113), (4.0, 5.0, 114), (5.0, 6.0, 115),
                (6.0, 7.0, 116), (7.0, 8.0, 117), (8.0, f64::MAX, 118),
            ],
            _ => &[],
        };
        for &(min_stars, max_stars, medal_id) in fc_table {
            if stars >= min_stars && stars < max_stars {
                check_and_award(medal_id);
            }
        }
    }

    if score.mode == 0 {
        if score_mods == Mods::SUDDENDEATH {
            check_and_award(119);
        } else if score_mods == Mods::PERFECT {
            check_and_award(120);
        } else if score_mods == Mods::HARDROCK {
            check_and_award(121);
        } else if score_mods == Mods::DOUBLETIME {
            check_and_award(122);
        } else if score_mods == Mods::NIGHTCORE || score_mods == (Mods::NIGHTCORE | Mods::DOUBLETIME) {
            check_and_award(123);
        } else if score_mods == Mods::HIDDEN {
            check_and_award(124);
        } else if score_mods == Mods::FLASHLIGHT {
            check_and_award(125);
        } else if score_mods == Mods::EASY {
            check_and_award(126);
        } else if score_mods == Mods::NOFAIL {
            check_and_award(127);
        } else if score_mods == Mods::HALFTIME {
            check_and_award(128);
        } else if score_mods == Mods::SPUNOUT {
            check_and_award(131);
        }
    }

    let acc = score.acc.unwrap_or(100.0);
    let map_title_lower = bmap.title.to_lowercase();
    let map_artist_lower = bmap.artist.to_lowercase();

    let s_str = score.score.to_string();
    if s_str.len() >= 6 && s_str.chars().all(|c| c == s_str.chars().next().unwrap()) {
        check_and_award(41);
    }

    if bmap.hit_length >= 600 && is_fc {
        check_and_award(44);
    }

    if acc > 0.0 && acc < 70.0 && !score_mods.intersects(Mods::NOFAIL | Mods::EASY) {
        check_and_award(40);
    }

    if acc > 0.0 && acc < 75.0 && !score_mods.intersects(Mods::NOFAIL | Mods::EASY) {
        check_and_award(38);
    }

    if acc > 0.0 && acc < 65.0 && score_mods.intersects(Mods::HIDDEN | Mods::HARDROCK | Mods::DOUBLETIME | Mods::NIGHTCORE | Mods::FLASHLIGHT) && !score_mods.intersects(Mods::NOFAIL | Mods::EASY) {
        check_and_award(177);
    }

    if bmap.approved == 2 && acc >= 90.0 {
        check_and_award(39);
    }

    if acc > 0.0 && acc < 70.0 && score_mods.contains(Mods::FLASHLIGHT) {
        check_and_award(178);
    }

    if bmap.diff_approach >= 9.0 && score_mods.contains(Mods::HIDDEN | Mods::FLASHLIGHT) {
        check_and_award(135);
    }

    if score_mods.contains(Mods::HIDDEN | Mods::HALFTIME) {
        check_and_award(136);
    }

    if score.score % 1_000 == 727 || score.max_combo == 727 || score.score == 727 || (score.pp.unwrap_or(0.0) - 727.0).abs() < 0.5 {
        check_and_award(287);
    }

    if map_title_lower.contains("one life left to live") && score_mods.intersects(Mods::SUDDENDEATH | Mods::PERFECT) {
        check_and_award(220);
    }

    if is_fc && stars >= 5.0 && score.n300 > 0 && score.n300 % 20 == 0 {
        check_and_award(222);
    }

    if map_title_lower.contains("fright march") && score_mods.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE) {
        check_and_award(175);
    }

    if map_title_lower.contains("glitch") && map_artist_lower.contains("lukhash") && score_mods.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE) {
        check_and_award(194);
    }

    if map_title_lower.contains("silence") && score_mods.contains(Mods::HIDDEN) {
        check_and_award(197);
    }

    if score_mods.contains(Mods::HIDDEN | Mods::FLASHLIGHT) {
        check_and_award(139);
    }

    if score_mods.contains(Mods::NIGHTCORE | Mods::FLASHLIGHT) {
        check_and_award(144);
    }

    if is_fc && score_mods.contains(Mods::HIDDEN | Mods::NOFAIL) {
        check_and_award(147);
    }

    if is_fc && score_mods.contains(Mods::NOFAIL) {
        check_and_award(138);
    }

    if is_fc && (map_title_lower == "eos" || map_title_lower.starts_with("eos ") || map_title_lower.ends_with(" eos")) {
        check_and_award(150);
    }

    if map_title_lower.contains("evanescent") && score_mods.contains(Mods::HALFTIME | Mods::HIDDEN) {
        check_and_award(153);
    }

    if is_fc && map_title_lower.contains("house with legs") && score_mods.contains(Mods::HIDDEN | Mods::HARDROCK) && score_mods.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE) {
        check_and_award(154);
    }

    if is_fc && stars >= 4.0 && map_artist_lower.contains("the flashbulb") {
        check_and_award(218);
    }

    if is_fc && stars >= 4.0 && map_title_lower.contains("chandelier") {
        check_and_award(219);
    }

    if map_title_lower.contains("exit this earth's atomosphere") {
        check_and_award(221);
    }

    if (map_title_lower.contains("alexithymia") || map_title_lower.contains("lupinus")) && acc >= 90.0 {
        check_and_award(225);
    }

    if map_title_lower.contains("scarlet rose") && score_mods.contains(Mods::EASY) && acc >= 92.0 {
        check_and_award(278);
    }

    if map_title_lower.contains("rpg") && (map_artist_lower.contains("yooh") || bmap.version.to_lowercase().contains("divinity")) {
        check_and_award(280);
    }

    if map_title_lower.contains("feral") && acc >= 98.0 {
        check_and_award(281);
    }

    if score_mods.contains(Mods::HALFTIME) && bmap.hit_length >= 300 {
        check_and_award(134);
    }

    if score_mods.contains(Mods::FLASHLIGHT) && score.max_combo >= 500 {
        check_and_award(176);
    }

    if is_fc && stars >= 3.0 && bmap.count_normal > 0 && bmap.count_normal == bmap.count_slider {
        check_and_award(276);
    }

    if map_title_lower.contains("daydream caf") || map_title_lower.contains("gochuumon") || map_artist_lower.contains("petit rabbit") {
        check_and_award(6);
    }

    if is_fc && bmap.diff_approach >= 10.0 && bmap.diff_overall >= 10.0 && bmap.diff_drain >= 10.0 {
        check_and_award(143);
    }

    if is_fc && map_title_lower.contains("classic pursuit") && score_mods.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE) && stars >= 2.5 {
        check_and_award(170);
    }

    if is_fc && score_mods.contains(Mods::HIDDEN | Mods::FLASHLIGHT) && stars >= 4.0 {
        check_and_award(172);
    }

    if is_fc && score_mods.contains(Mods::HALFTIME) && stars >= 6.0 {
        check_and_award(285);
    }

    if is_fc && map_title_lower.contains("arrogance") && map_artist_lower.contains("onumi") && score_mods.contains(Mods::HIDDEN | Mods::FLASHLIGHT) {
        check_and_award(217);
    }

    if map_title_lower.contains("mephisto") && map_artist_lower.contains("leaf") && score_mods.contains(Mods::FLASHLIGHT) {
        check_and_award(286);
    }

    if map_title_lower.contains("the solace of oblivion") && acc >= 70.0 && !score_mods.contains(Mods::NOFAIL) {
        check_and_award(173);
    }

    if bmap.hit_length >= 450 {
        check_and_award(132);
    }

    if bmap.hit_length >= 450 && is_fc {
        check_and_award(133);
    }

    if is_fc && stars >= 3.0 && score_mods.contains(Mods::SUDDENDEATH | Mods::HARDROCK) {
        check_and_award(140);
    }

    if score_mods.contains(Mods::FLASHLIGHT) && score.max_combo < 200 && bmap.max_combo >= 200 {
        check_and_award(141);
    }

    if is_fc && stars >= 4.0 && score_mods.contains(Mods::EASY) {
        check_and_award(142);
    }

    let is_dthr = score_mods.contains(Mods::HARDROCK) && score_mods.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE);
    let bpm = bmap.bpm * if score_mods.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE) { 1.5 } else { 1.0 };
    let effective_ar = if is_dthr { bmap.diff_approach * 1.4 } else { bmap.diff_approach };
    let effective_od = if is_dthr { bmap.diff_overall * 1.4 } else { bmap.diff_overall };
    let effective_hp = if is_dthr { bmap.diff_drain * 1.4 } else { bmap.diff_drain };
    let high_stats_cleared = (bmap.diff_approach >= 11.0 || effective_ar >= 10.5)
        && (bmap.diff_overall >= 11.0 || effective_od >= 10.5)
        && (bmap.diff_drain >= 11.0 || effective_hp >= 10.5)
        && (bpm >= 260.0 || bmap.bpm >= 260.0);

    if high_stats_cleared {
        check_and_award(145);
    }

    if is_fc && high_stats_cleared {
        check_and_award(146);
    }

    if score.nmiss == 1 && bmap.max_combo > 0 && score.max_combo >= bmap.max_combo - 2 {
        check_and_award(148);
    }

    if is_fc && stars >= 3.0 && score_mods.contains(Mods::EASY | Mods::HIDDEN | Mods::FLASHLIGHT) {
        check_and_award(149);
    }

    if is_fc && acc >= 99.99 && stars >= 3.0 && score_mods.contains(Mods::HALFTIME) && score_mods.intersects(Mods::PERFECT | Mods::SUDDENDEATH) {
        check_and_award(151);
    }

    if is_fc && bmap.hit_length > 0 && bmap.hit_length <= 30 && score_mods.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE) {
        check_and_award(152);
    }

    if is_fc && acc >= 99.99 && stars >= 3.0 && score_mods.contains(Mods::EASY) && score_mods.intersects(Mods::PERFECT | Mods::SUDDENDEATH) {
        check_and_award(157);
    }

    let effective_cs = if score_mods.contains(Mods::HARDROCK) { bmap.diff_size * 1.3 } else { bmap.diff_size };
    if is_fc && score_mods.contains(Mods::HARDROCK) && effective_cs >= 7.8 {
        check_and_award(158);
    }

    if score.n300 >= 15 && score.n300 == score.n100 && score.n100 == score.n50 {
        check_and_award(159);
    }

    if stars >= 4.0 && is_fc && score_mods.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE) && score_mods.intersects(Mods::PERFECT | Mods::SUDDENDEATH) {
        check_and_award(160);
    }

    if score.max_combo == 1337 && stars >= 3.5 && bmap.diff_approach >= 8.0 && bmap.diff_overall >= 8.0 && bmap.max_combo >= 1500 {
        check_and_award(161);
    }

    if score.n50 == 50 {
        check_and_award(168);
    }

    if score.mode == 3 && score.max_combo >= 100 {
        check_and_award(54);
    }

    if is_fc && bmap.diff_approach >= 10.0 && bmap.diff_overall >= 10.0 && bmap.diff_drain >= 10.0 {
        check_and_award(352);
    }

    if bmap.hit_length >= 600 || (bmap.hit_length > 0 && bmap.hit_length <= 60) {
        check_and_award(268);
    }

    if map_title_lower.contains("paraparamax i") && !score_mods.contains(Mods::NOFAIL) {
        check_and_award(17);
    }

    if map_title_lower.contains("pika girl") && map_artist_lower.contains("s3rl") && acc >= 95.0 && score.max_combo == 151 {
        check_and_award(171);
    }

    if map_title_lower.contains("moonlight sonata") && map_artist_lower.contains("cysmix") && score_mods.contains(Mods::HIDDEN | Mods::HARDROCK) && score_mods.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE) {
        check_and_award(174);
    }

    if map_title_lower == "sky" && map_artist_lower.contains("james portland") && is_fc && acc >= 99.99 {
        check_and_award(192);
    }

    if (map_title_lower.contains("t&j") || map_title_lower.contains("t & j")) && map_artist_lower.contains("cranky") && acc >= 80.0 && !score_mods.intersects(Mods::NOFAIL | Mods::EASY) {
        check_and_award(193);
    }

    if map_title_lower.contains("relucent") && map_artist_lower.contains("culprate") && is_fc {
        check_and_award(195);
    }

    if (map_title_lower.contains("phonetic") || map_title_lower.contains("journey")) && map_artist_lower.contains("left") && score_mods.contains(Mods::HIDDEN | Mods::FLASHLIGHT) {
        check_and_award(196);
    }

    if map_title_lower.contains("hanaarashi") && map_artist_lower.contains("cranky") && score_mods.contains(Mods::HIDDEN) {
        check_and_award(200);
    }

    if map_title_lower.contains("impulse") && (map_artist_lower.contains("culprate") || map_artist_lower.contains("au5")) {
        check_and_award(201);
    }

    if map_title_lower.contains("cherry blossoms explode") || (map_artist_lower.contains("sakuraburst") && map_title_lower.contains("dying horizon")) {
        check_and_award(204);
    }

    if map_title_lower.contains("waterflow") && map_artist_lower.contains("tieff") && score_mods.intersects(Mods::DOUBLETIME | Mods::NIGHTCORE) && acc >= 80.0 {
        check_and_award(216);
    }

    if map_title_lower.contains("didj pvc") && map_artist_lower.contains("the flashbulb") && score_mods.contains(Mods::EASY | Mods::HALFTIME) {
        check_and_award(223);
    }

    if map_artist_lower == "sooo" || map_artist_lower.contains("sooo") {
        check_and_award(224);
    }

    if (map_title_lower.contains("the deceit") || map_title_lower.contains("the violation")) && map_artist_lower.contains("fleshgod apocalypse") {
        check_and_award(297);
    }

    if is_fc && map_title_lower.contains("ghost assassin") && map_artist_lower.contains("veela") && score_mods.contains(Mods::HIDDEN) && score_mods.intersects(Mods::SUDDENDEATH | Mods::PERFECT) {
        check_and_award(298);
    }

    if map_title_lower.contains("perfect metal") {
        check_and_award(347);
    }

    newly_earned
}

pub fn retroactive_eval_profile(conn: &Connection, player_name: &str) -> rusqlite::Result<Vec<i32>> {
    let existing_medals: HashSet<i32> = db::get_user_medals(conn, player_name)?
        .into_iter()
        .map(|m| m.medal_id)
        .collect();

    let playcount = db::get_playcount(conn, player_name).unwrap_or(0);
    let mut newly_earned = Vec::new();
    let mut check_and_award = |id: i32| {
        if !existing_medals.contains(&id) && !newly_earned.contains(&id) {
            newly_earned.push(id);
        }
    };

    let playcount_milestones = [
        (5_000, 20),
        (15_000, 21),
        (25_000, 22),
        (50_000, 28),
    ];
    for &(threshold, medal_id) in &playcount_milestones {
        if playcount >= threshold {
            check_and_award(medal_id);
        }
    }

    let taiko_hits = db::get_mode_total_hits(conn, player_name, 1).unwrap_or(0);
    let catch_hits = db::get_mode_total_hits(conn, player_name, 2).unwrap_or(0);
    let mania_hits = db::get_mode_total_hits(conn, player_name, 3).unwrap_or(0);

    let mode_hit_milestones = [
        (taiko_hits, &[(30_000, 31), (300_000, 32), (3_000_000, 33), (30_000_000, 291)][..]),
        (catch_hits, &[(20_000, 13), (200_000, 23), (2_000_000, 24), (20_000_000, 292)][..]),
        (mania_hits, &[(40_000, 46), (400_000, 47), (4_000_000, 48), (40_000_000, 293)][..]),
    ];
    for &(total, list) in &mode_hit_milestones {
        for &(thresh, id) in list {
            if total >= thresh {
                check_and_award(id);
            }
        }
    }

    let all_scores = db::get_all_scores(conn, player_name)?;
    for score in &all_scores {
        let bmap = db::get_beatmap_by_md5(conn, &score.md5)?
            .unwrap_or_else(Beatmap::blank);

        if bmap.approved != 1 && bmap.approved != 2 {
            continue;
        }

        let parsed_map = bmap.file_content.as_deref().and_then(|c| rosu_pp::Beatmap::from_bytes(c.as_bytes()).ok());
        let mode_hits = match score.mode {
            1 => taiko_hits,
            2 => catch_hits,
            3 => mania_hits,
            _ => 0,
        };

        let awarded = evaluate_score_submission(
            score,
            &bmap,
            parsed_map.as_ref(),
            playcount,
            mode_hits,
            &existing_medals,
            true,
        );

        for id in awarded {
            check_and_award(id);
        }
    }

    let s_rank_maps: HashSet<&str> = all_scores
        .iter()
        .filter(|s| s.nmiss == 0 && s.acc.unwrap_or(0.0) >= 95.0)
        .map(|s| s.md5.as_str())
        .collect();
    if s_rank_maps.len() >= 5 {
        check_and_award(15);
    }

    let scores_mode_0 = all_scores.iter().filter(|s| s.mode == 0).count();
    let scores_mode_1 = all_scores.iter().filter(|s| s.mode == 1).count();
    let scores_mode_2 = all_scores.iter().filter(|s| s.mode == 2).count();
    let scores_mode_3 = all_scores.iter().filter(|s| s.mode == 3).count();
    if scores_mode_0 >= 5_000 && scores_mode_1 >= 5_000 && scores_mode_2 >= 5_000 && scores_mode_3 >= 5_000 {
        check_and_award(45);
    }

    if !newly_earned.is_empty() {
        let now = chrono::Utc::now().timestamp();
        let batch: Vec<(i32, i64)> = newly_earned.iter().map(|&id| (id, now)).collect();
        db::add_user_medals_batch(conn, player_name, &batch)?;
    }

    Ok(newly_earned)
}

pub fn evaluate_rank_medals(rank: i32, existing: &HashSet<i32>) -> Vec<i32> {
    let mut newly_earned = Vec::new();
    if rank <= 0 || rank == 9999999 {
        return newly_earned;
    }
    let milestones = [
        (50_000, 50),
        (10_000, 51),
        (5_000, 52),
        (1_000, 53),
    ];
    for &(thresh, id) in &milestones {
        if rank <= thresh && !existing.contains(&id) {
            newly_earned.push(id);
        }
    }
    newly_earned
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_score(max_combo: i32, nmiss: i32, perfect: bool, mods: u32, score_val: i64) -> Score {
        Score {
            mode: 0,
            md5: "test_md5".to_string(),
            name: "Alice".to_string(),
            n300: 300,
            n100: 0,
            n50: 0,
            ngeki: 0,
            nkatu: 0,
            nmiss,
            score: score_val,
            max_combo,
            perfect,
            mods,
            time: 1700000000,
            acc: Some(100.0),
            pp: Some(100.0),
            replay_md5: None,
            scoreid: None,
            replay_frames: None,
            mods_str: None,
            additional_mods: None,
            submission_checksum: None,
            submission_identity: None,
        }
    }

    fn make_test_bmap(stars: f64) -> Beatmap {
        let mut b = Beatmap::blank();
        b.approved = 1;
        b.difficultyrating = stars;
        b.max_combo = 500;
        b
    }

    #[test]
    fn test_medal_combo_evaluation() {
        let bmap = make_test_bmap(0.5);
        let existing = HashSet::new();

        let s499 = make_test_score(499, 0, true, 0, 500000);
        let medals = evaluate_score_submission(&s499, &bmap, None, 10, 0, &existing, true);
        assert!(!medals.contains(&1));

        let s500 = make_test_score(500, 0, true, 0, 500000);
        let medals = evaluate_score_submission(&s500, &bmap, None, 10, 0, &existing, true);
        assert!(medals.contains(&1));
        assert!(!medals.contains(&3));

        let s1000 = make_test_score(1000, 0, true, 0, 1000000);
        let medals = evaluate_score_submission(&s1000, &bmap, None, 10, 0, &existing, true);
        assert!(medals.contains(&1));
        assert!(medals.contains(&3));
        assert!(medals.contains(&4));
        assert!(!medals.contains(&5));

        let mut existing_with_some = HashSet::new();
        existing_with_some.insert(1);
        existing_with_some.insert(3);
        let medals = evaluate_score_submission(&s1000, &bmap, None, 10, 0, &existing_with_some, true);
        assert_eq!(medals, vec![4]);
    }

    #[test]
    fn test_medal_star_pass_and_fc_evaluation() {
        let bmap = make_test_bmap(5.25);
        let existing = HashSet::new();

        let pass_score = make_test_score(350, 1, false, 0, 600000);
        let medals = evaluate_score_submission(&pass_score, &bmap, None, 10, 0, &existing, true);
        assert!(!medals.contains(&55));
        assert!(medals.contains(&59));
        assert!(!medals.contains(&60));
        assert!(!medals.contains(&63));
        assert!(!medals.contains(&67));

        let fc_score = make_test_score(500, 0, true, 0, 800000);
        let medals = evaluate_score_submission(&fc_score, &bmap, None, 10, 0, &existing, true);
        assert!(!medals.contains(&55));
        assert!(!medals.contains(&63));
        assert!(medals.contains(&59));
        assert!(medals.contains(&67));
        assert!(!medals.contains(&68));
    }

    #[test]
    fn test_unranked_map_awards_no_score_medals() {
        let mut bmap = make_test_bmap(5.0);
        bmap.approved = 0;
        let existing = HashSet::new();

        let fc_score = make_test_score(500, 0, true, 0, 800000);
        let medals = evaluate_score_submission(&fc_score, &bmap, None, 10, 0, &existing, true);
        assert!(medals.is_empty(), "Unranked maps must never award score medals");
    }

    #[test]
    fn test_failed_score_awards_no_score_medals() {
        let bmap = make_test_bmap(5.0);
        let existing = HashSet::new();

        let fail_score = make_test_score(500, 1, false, 0, 800000);
        let medals = evaluate_score_submission(&fail_score, &bmap, None, 10, 0, &existing, false);
        assert!(medals.is_empty(), "Failed scores must never award score medals");
    }

    #[test]
    fn test_medal_mod_introductions() {
        let bmap = make_test_bmap(3.0);
        let existing = HashSet::new();

        let hr = Mods::HARDROCK.bits();
        let s_hr = make_test_score(100, 0, true, hr, 100000);
        let medals_hr = evaluate_score_submission(&s_hr, &bmap, None, 10, 0, &existing, true);
        assert!(medals_hr.contains(&121));

        let hd = Mods::HIDDEN.bits();
        let s_hd = make_test_score(100, 0, true, hd, 100000);
        let medals_hd = evaluate_score_submission(&s_hd, &bmap, None, 10, 0, &existing, true);
        assert!(medals_hd.contains(&124));

        let hdhr = (Mods::HIDDEN | Mods::HARDROCK).bits();
        let s_hdhr = make_test_score(100, 0, true, hdhr, 100000);
        let medals_hdhr = evaluate_score_submission(&s_hdhr, &bmap, None, 10, 0, &existing, true);
        assert!(!medals_hdhr.contains(&121), "Combinations must not award single mod medals");
        assert!(!medals_hdhr.contains(&124), "Combinations must not award single mod medals");

        let dt = Mods::DOUBLETIME.bits();
        let s_dt = make_test_score(100, 0, true, dt, 100000);
        let medals_dt = evaluate_score_submission(&s_dt, &bmap, None, 10, 0, &existing, true);
        assert!(medals_dt.contains(&122));

        let rx = (Mods::DOUBLETIME | Mods::RELAX).bits();
        let s_rx = make_test_score(100, 0, true, rx, 100000);
        let medals_rx = evaluate_score_submission(&s_rx, &bmap, None, 10, 0, &existing, true);
        assert!(medals_rx.is_empty());
    }

    #[test]
    fn test_medal_secrets_jackpot_and_consolation() {
        let bmap = make_test_bmap(2.5);
        let existing = HashSet::new();

        let jackpot_score = make_test_score(100, 0, true, 0, 777_777);
        let medals = evaluate_score_submission(&jackpot_score, &bmap, None, 10, 0, &existing, true);
        assert!(medals.contains(&41));

        let mut consol_score = make_test_score(100, 5, false, 0, 300000);
        consol_score.acc = Some(72.0);
        let medals_consol = evaluate_score_submission(&consol_score, &bmap, None, 10, 0, &existing, true);
        assert!(medals_consol.contains(&38));
    }

    #[test]
    fn test_retroactive_eval_profile() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_db(&conn).unwrap();
        db::ensure_profile(&conn, "RetroTester").unwrap();

        for _ in 0..5005 {
            db::increment_playcount(&conn, "RetroTester").unwrap();
        }

        let mut bmap = make_test_bmap(4.5);
        bmap.file_md5 = "retro_map_md5".to_string();
        bmap.max_combo = 600;
        db::insert_beatmap(&conn, &bmap).unwrap();

        let mut sc = make_test_score(550, 0, true, Mods::HARDROCK.bits(), 750000);
        sc.name = "RetroTester".to_string();
        sc.md5 = "retro_map_md5".to_string();
        db::insert_score(&conn, &sc, "ranked").unwrap();

        let new_medals = retroactive_eval_profile(&conn, "RetroTester").unwrap();

        assert!(new_medals.contains(&20));
        assert!(new_medals.contains(&1));
        assert!(!new_medals.contains(&55));
        assert!(new_medals.contains(&58));
        assert!(!new_medals.contains(&63));
        assert!(new_medals.contains(&66));
        assert!(new_medals.contains(&121));

        let rerun = retroactive_eval_profile(&conn, "RetroTester").unwrap();
        assert!(rerun.is_empty());

        let count = db::get_user_medal_count(&conn, "RetroTester").unwrap();
        assert_eq!(count, new_medals.len());
    }

    #[test]
    fn test_multimode_passes_and_secrets() {
        let bmap = make_test_bmap(4.2);
        let existing = HashSet::new();

        let mut s_taiko = make_test_score(200, 0, true, 0, 400000);
        s_taiko.mode = 1;
        let medals_taiko = evaluate_score_submission(&s_taiko, &bmap, None, 10, 35000, &existing, true);
        assert!(!medals_taiko.contains(&71));
        assert!(medals_taiko.contains(&74));
        assert!(!medals_taiko.contains(&75));
        assert!(!medals_taiko.contains(&95));
        assert!(medals_taiko.contains(&98));
        assert!(medals_taiko.contains(&31));

        let mut s_mania = make_test_score(150, 5, false, 0, 200000);
        s_mania.mode = 3;
        s_mania.acc = Some(65.0);
        let medals_mania = evaluate_score_submission(&s_mania, &bmap, None, 10, 50000, &existing, true);
        assert!(!medals_mania.contains(&87));
        assert!(medals_mania.contains(&90));
        assert!(!medals_mania.contains(&111));
        assert!(medals_mania.contains(&40));
        assert!(medals_mania.contains(&46));

        let mut s_wysi = make_test_score(727, 0, true, 0, 727000);
        s_wysi.pp = Some(727.0);
        let medals_wysi = evaluate_score_submission(&s_wysi, &bmap, None, 10, 0, &existing, true);
        assert!(medals_wysi.contains(&287));

        let bmap_nat20 = make_test_bmap(5.2);
        let mut s_nat20 = make_test_score(100, 0, true, 0, 50000);
        s_nat20.n300 = 40;
        let medals_nat20 = evaluate_score_submission(&s_nat20, &bmap_nat20, None, 10, 0, &existing, true);
        assert!(medals_nat20.contains(&222));

        let mut bmap_balanced = make_test_bmap(3.5);
        bmap_balanced.count_normal = 100;
        bmap_balanced.count_slider = 100;
        let s_balanced = make_test_score(200, 0, true, 0, 100000);
        let medals_dc = evaluate_score_submission(&s_balanced, &bmap_balanced, None, 10, 0, &existing, true);
        assert!(medals_dc.contains(&276));

        let hd_fl = (Mods::HIDDEN | Mods::FLASHLIGHT).bits();
        let s_hdfl = make_test_score(100, 0, true, hd_fl, 100000);
        let medals_hdfl = evaluate_score_submission(&s_hdfl, &bmap, None, 10, 0, &existing, true);
        assert!(medals_hdfl.contains(&139));

        let nc_fl = (Mods::NIGHTCORE | Mods::FLASHLIGHT).bits();
        let s_ncfl = make_test_score(100, 0, true, nc_fl, 100000);
        let medals_ncfl = evaluate_score_submission(&s_ncfl, &bmap, None, 10, 0, &existing, true);
        assert!(medals_ncfl.contains(&144));

        let rank_medals = evaluate_rank_medals(450, &existing);
        assert!(rank_medals.contains(&50));
        assert!(rank_medals.contains(&51));
        assert!(rank_medals.contains(&52));
        assert!(rank_medals.contains(&53));

        let mut bmap_bunny = make_test_bmap(2.0);
        bmap_bunny.title = "Daydream cafe (TV Size)".to_string();
        let s_bunny = make_test_score(100, 0, true, 0, 100000);
        let medals_bunny = evaluate_score_submission(&s_bunny, &bmap_bunny, None, 10, 0, &existing, true);
        assert!(medals_bunny.contains(&6));

        let hd_ht = (Mods::HIDDEN | Mods::HALFTIME).bits();
        let s_hdht = make_test_score(100, 0, true, hd_ht, 100000);
        let medals_hdht = evaluate_score_submission(&s_hdht, &bmap, None, 10, 0, &existing, true);
        assert!(medals_hdht.contains(&136));
    }

    #[test]
    fn test_new_important_medals() {
        let existing = HashSet::new();

        let mut bmap_long = make_test_bmap(3.0);
        bmap_long.hit_length = 460;
        let s_long_pass = make_test_score(300, 1, false, 0, 500000);
        let m_long_pass = evaluate_score_submission(&s_long_pass, &bmap_long, None, 10, 0, &existing, true);
        assert!(m_long_pass.contains(&132));
        assert!(!m_long_pass.contains(&133));

        let s_long_fc = make_test_score(500, 0, true, 0, 800000);
        let m_long_fc = evaluate_score_submission(&s_long_fc, &bmap_long, None, 10, 0, &existing, true);
        assert!(m_long_fc.contains(&132));
        assert!(m_long_fc.contains(&133));

        let sd_hr = (Mods::SUDDENDEATH | Mods::HARDROCK).bits();
        let s_sdhr = make_test_score(500, 0, true, sd_hr, 800000);
        let m_sdhr = evaluate_score_submission(&s_sdhr, &bmap_long, None, 10, 0, &existing, true);
        assert!(m_sdhr.contains(&140));

        let bmap_4star = make_test_bmap(4.2);
        let s_ez = make_test_score(500, 0, true, Mods::EASY.bits(), 500000);
        let m_ez = evaluate_score_submission(&s_ez, &bmap_4star, None, 10, 0, &existing, true);
        assert!(m_ez.contains(&142));

        let mut s_eq = make_test_score(100, 0, false, 0, 100000);
        s_eq.n300 = 25;
        s_eq.n100 = 25;
        s_eq.n50 = 25;
        let m_eq = evaluate_score_submission(&s_eq, &bmap_4star, None, 10, 0, &existing, true);
        assert!(m_eq.contains(&159));

        let mut s_50 = make_test_score(100, 0, false, 0, 100000);
        s_50.n50 = 50;
        let m_50 = evaluate_score_submission(&s_50, &bmap_4star, None, 10, 0, &existing, true);
        assert!(m_50.contains(&168));

        let mut s_mania_100 = make_test_score(120, 0, false, 0, 200000);
        s_mania_100.mode = 3;
        let m_mania = evaluate_score_submission(&s_mania_100, &bmap_4star, None, 10, 0, &existing, true);
        assert!(m_mania.contains(&54));

        let mut bmap_pika = make_test_bmap(3.0);
        bmap_pika.title = "Pika Girl".to_string();
        bmap_pika.artist = "S3RL".to_string();
        let mut s_pika = make_test_score(151, 0, false, 0, 300000);
        s_pika.acc = Some(96.5);
        let m_pika = evaluate_score_submission(&s_pika, &bmap_pika, None, 10, 0, &existing, true);
        assert!(m_pika.contains(&171));

        let mut bmap_coils = make_test_bmap(7.0);
        bmap_coils.title = "The Violation".to_string();
        bmap_coils.artist = "Fleshgod Apocalypse".to_string();
        let s_coils = make_test_score(200, 10, false, 0, 500000);
        let m_coils = evaluate_score_submission(&s_coils, &bmap_coils, None, 10, 0, &existing, true);
        assert!(m_coils.contains(&297));

        let mut bmap_sonata = make_test_bmap(2.5);
        bmap_sonata.title = "Moonlight Sonata".to_string();
        bmap_sonata.artist = "cYsmix".to_string();
        let hd_hr_dt = (Mods::HIDDEN | Mods::HARDROCK | Mods::DOUBLETIME).bits();
        let s_sonata = make_test_score(200, 0, true, hd_hr_dt, 500000);
        let m_sonata = evaluate_score_submission(&s_sonata, &bmap_sonata, None, 10, 0, &existing, true);
        assert!(m_sonata.contains(&174));
    }
}

