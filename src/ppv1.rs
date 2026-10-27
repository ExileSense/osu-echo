#[derive(Clone, Debug, Default)]
pub struct MapMeta {
    pub drain_secs: f64,
    pub bpm: f64,
    pub hp: f64,
    pub od: f64,
    pub cs: f64,
    pub n_normal: f64,
    pub n_slider: f64,
    pub n_spinner: f64,
    pub slider_multiplier: f64,
}

pub fn eyup_stars(m: &MapMeta) -> f64 {
    if m.drain_secs <= 0.0 || m.bpm <= 0.0 {
        return 0.0;
    }
    let total_objects = m.n_normal + m.n_slider * 2.0 + m.n_spinner * 3.0;
    if total_objects <= 0.0 {
        return 0.0;
    }
    let note_density = total_objects / m.drain_secs;
    let mut difficulty = m.hp + m.od + m.cs;
    if m.n_slider / total_objects >= 0.1 {
        let bpm_factor = (m.bpm / 60.0) * m.slider_multiplier;
        difficulty = (difficulty + ((bpm_factor - 1.5) * 2.5).clamp(0.0, 10.0)) * 0.75;
    }
    let stars = if difficulty > 21.0 {
        (difficulty.min(30.0) / 3.0 * 4.0 + (20.0 - 0.032 * (note_density - 5.0).powi(4)).min(20.0)) / 10.0
    } else if note_density >= 2.5 {
        (difficulty.min(18.0) / 18.0 * 10.0
            + (40.0 - 40.0 / 5f64.powf(3.5) * (note_density.min(5.0) - 5.0).powi(4)).min(40.0))
            / 10.0
    } else if note_density < 1.0 {
        (difficulty.min(18.0) / 18.0 * 10.0) / 10.0 + 0.25
    } else {
        (difficulty.min(18.0) / 18.0 * 10.0 + (25.0 * (note_density - 1.0)).min(40.0)) / 10.0
    };
    stars.clamp(0.0, 5.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> MapMeta {
        MapMeta {
            drain_secs: 120.0,
            bpm: 180.0,
            hp: 6.0,
            od: 8.0,
            cs: 4.0,
            n_normal: 250.0,
            n_slider: 100.0,
            n_spinner: 2.0,
            slider_multiplier: 1.6,
        }
    }

    #[test]
    fn eyup_stars_follow_the_reference_formula() {
        let s = eyup_stars(&meta());
        assert!((s - 4.970_325_052_4).abs() < 1e-9, "{s}");
        let mut dense = meta();
        dense.n_normal = 900.0;
        dense.n_slider = 0.0;
        dense.n_spinner = 0.0;
        dense.drain_secs = 100.0;
        assert_eq!(eyup_stars(&dense), 5.0);
        let mut slow = meta();
        slow.n_normal = 40.0;
        slow.n_slider = 0.0;
        slow.n_spinner = 0.0;
        assert!((eyup_stars(&slow) - 1.25).abs() < 1e-9);
        assert_eq!(eyup_stars(&MapMeta::default()), 0.0);
        let mut insane = meta();
        insane.hp = 10.0;
        insane.od = 10.0;
        insane.cs = 10.0;
        insane.n_slider = 0.0;
        assert_eq!(eyup_stars(&insane), 5.0);
    }
}
