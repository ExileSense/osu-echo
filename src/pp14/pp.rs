use rosu_pp_v4::Beatmap;

use super::tuning::{
    ACC_BASE, ACC_KNEE, AIM_ACC_EXPONENT_BELOW_KNEE, SPEED_ACC_EXPONENT_BELOW_KNEE, ACC_EXPONENT_HIGH_OD, ACC_EXPONENT_HIGH_OD_AT, ACC_EXPONENT_LOW_OD,
    ACC_EXPONENT_LOW_OD_AT, ACC_PP_SCALE, AIM_ACC_EXPONENT, AIM_PP_SCALE, AIM_STRAIN_PP_DIVISOR,
    SPEED_ACC_EXPONENT, SPEED_PP_SCALE, SPEED_STRAIN_PP_DIVISOR,
    TOTAL_PP_MULTIPLIER,
};
use super::util::Mods;

use super::stars::{stars, OsuDifficultyAttributes, OsuPerformanceAttributes};
#[derive(Clone, Debug)]
pub struct OsuPP<'m> {
    map: &'m Beatmap,
    attributes: Option<OsuDifficultyAttributes>,
    mods: u32,
    combo: Option<u32>,
    acc: Option<f32>,
    n300: Option<u32>,
    n100: Option<u32>,
    n50: Option<u32>,
    n_misses: u32,
    bonus: f32,
}

impl<'m> OsuPP<'m> {
    #[inline]
    pub fn new(map: &'m Beatmap) -> Self {
        Self {
            map,
            attributes: None,
            mods: 0,
            combo: None,
            acc: None,
            n300: None,
            n100: None,
            n50: None,
            n_misses: 0,
            bonus: 1.0,
        }
    }
    #[inline]
    pub fn attributes(mut self, attributes: impl OsuAttributeProvider) -> Self {
        if let Some(attributes) = attributes.attributes() {
            self.attributes.replace(attributes);
        }

        self
    }
    #[inline]
    pub fn mods(mut self, mods: u32) -> Self {
        self.mods = mods;

        self
    }
    #[inline]
    pub fn combo(mut self, combo: u32) -> Self {
        self.combo.replace(combo);

        self
    }
    #[inline]
    pub fn n300(mut self, n300: u32) -> Self {
        self.n300.replace(n300);

        self
    }
    #[inline]
    pub fn n100(mut self, n100: u32) -> Self {
        self.n100.replace(n100);

        self
    }
    #[inline]
    pub fn n50(mut self, n50: u32) -> Self {
        self.n50.replace(n50);

        self
    }
    #[inline]
    pub fn bonus(mut self, multiplier: f64) -> Self {
        self.bonus = multiplier as f32;

        self
    }

    pub fn misses(mut self, n_misses: u32) -> Self {
        self.n_misses = n_misses;

        self
    }
    pub fn accuracy(mut self, acc: f32) -> Self {
        let n_objects = self.map.hit_objects.len() as u32;

        let acc = acc / 100.0;

        if self.n100.or(self.n50).is_some() {
            let mut n100 = self.n100.unwrap_or(0);
            let mut n50 = self.n50.unwrap_or(0);

            let placed_points = 2 * n100 + n50 + self.n_misses;
            let missing_objects = n_objects - n100 - n50 - self.n_misses;
            let missing_points =
                ((6.0 * acc * n_objects as f32).round() as u32).saturating_sub(placed_points);

            let mut n300 = missing_objects.min(missing_points / 6);
            n50 += missing_objects - n300;

            if let Some(orig_n50) = self.n50.filter(|_| self.n100.is_none()) {
                let difference = n50 - orig_n50;
                let n = n300.min(difference / 4);

                n300 -= n;
                n100 += 5 * n;
                n50 -= 4 * n;
            }

            self.n300.replace(n300);
            self.n100.replace(n100);
            self.n50.replace(n50);
        } else {
            let misses = self.n_misses.min(n_objects);
            let target_total = (acc * n_objects as f32 * 6.0).round() as u32;
            let delta = target_total - (n_objects - misses);

            let mut n300 = delta / 5;
            let mut n100 = delta % 5;
            let mut n50 = n_objects - n300 - n100 - misses;

            let n = n300.min(n50 / 4);
            n300 -= n;
            n100 += 5 * n;
            n50 -= 4 * n;

            self.n300.replace(n300);
            self.n100.replace(n100);
            self.n50.replace(n50);
        }

        let acc = (6 * self.n300.unwrap() + 2 * self.n100.unwrap() + self.n50.unwrap()) as f32
            / (6 * n_objects) as f32;

        self.acc.replace(acc);

        self
    }

    fn assert_hitresults(&mut self) {
        if self.acc.is_none() {
            let n_objects = self.map.hit_objects.len() as u32;

            let remaining = n_objects
                .saturating_sub(self.n300.unwrap_or(0))
                .saturating_sub(self.n100.unwrap_or(0))
                .saturating_sub(self.n50.unwrap_or(0))
                .saturating_sub(self.n_misses);

            if remaining > 0 {
                if self.n300.is_none() {
                    self.n300.replace(remaining);
                    self.n100.get_or_insert(0);
                    self.n50.get_or_insert(0);
                } else if self.n100.is_none() {
                    self.n100.replace(remaining);
                    self.n50.get_or_insert(0);
                } else if self.n50.is_none() {
                    self.n50.replace(remaining);
                } else if let Some(ref mut n300) = self.n300 {
                    *n300 += remaining;
                }
            } else {
                self.n300.get_or_insert(0);
                self.n100.get_or_insert(0);
                self.n50.get_or_insert(0);
            }

            let numerator = self.n50.unwrap() + self.n100.unwrap() * 2 + self.n300.unwrap() * 6;
            self.acc.replace(numerator as f32 / n_objects as f32 / 6.0);
        }
    }
    pub fn calculate(mut self) -> OsuPerformanceAttributes {
        if self.attributes.is_none() {
            let attributes = stars(self.map, self.mods);
            self.attributes = Some(attributes);
        }

        self.assert_hitresults();

        let total_hits = self.total_hits() as f32;
        let mut multiplier = TOTAL_PP_MULTIPLIER;

        if self.mods.nf() {
            multiplier *= 0.9;
        }

        if self.mods.so() {
            multiplier *= 0.95;
        }

        let aim_value = self.compute_aim_value(total_hits) * AIM_PP_SCALE;
        let speed_value = self.compute_speed_value(total_hits) * SPEED_PP_SCALE;
        let acc_value = self.compute_accuracy_value() * ACC_PP_SCALE;

        let pp = (aim_value.powf(1.1) + speed_value.powf(1.1) + acc_value.powf(1.1))
            .powf(1.0 / 1.1)
            * multiplier
            * self.bonus;

        OsuPerformanceAttributes {
            difficulty: self.attributes.unwrap(),
            pp_acc: acc_value as f64,
            pp_aim: aim_value as f64,
            pp_flashlight: 0.0,
            pp_speed: speed_value as f64,
            pp: pp as f64,
        }
    }

    fn compute_aim_value(&self, total_hits: f32) -> f32 {
        let attributes = self.attributes.as_ref().unwrap();

        let mut aim_value =
            (5.0 * (attributes.aim_strain as f32 / AIM_STRAIN_PP_DIVISOR).max(1.0) - 4.0).powi(3) / 100_000.0;

        aim_value *= 1.0 + 0.1 * (total_hits / 1500.0).min(1.0);

        aim_value *= 0.97_f32.powi(self.n_misses as i32);

        if let Some(combo) = self.combo.filter(|_| attributes.max_combo > 0) {
            aim_value *= ((combo as f32 / attributes.max_combo as f32).powf(0.8)).min(1.0);
        }

        let mut ar_factor = 1.0;
        if attributes.ar > 10.00 {
            ar_factor += 0.30 * (attributes.ar - 10.00);
        } else if attributes.ar < 8.0 {
            if self.mods.hd() {
                ar_factor += 0.02 * (8.0 - attributes.ar);
            } else {
                ar_factor += 0.01 * (8.0 - attributes.ar);
            }
        }
        aim_value *= ar_factor as f32;

        if self.mods.hd() {
            aim_value *= 1.18;
        }

        if self.mods.fl() {
            aim_value *= 1.50;
        }

        aim_value *= acc_factor(self.acc.unwrap(), AIM_ACC_EXPONENT, AIM_ACC_EXPONENT_BELOW_KNEE);
        aim_value *= 0.98 + attributes.od as f32 * attributes.od as f32 / 2500.0;

        aim_value
    }

    fn compute_speed_value(&self, total_hits: f32) -> f32 {
        let attributes = self.attributes.as_ref().unwrap();

        let mut speed_value =
            (5.0 * (attributes.speed_strain as f32 / SPEED_STRAIN_PP_DIVISOR).max(1.0) - 4.0).powi(3) / 100_000.0;

        speed_value *= 1.0 + 0.1 * (total_hits / 1500.0).min(1.0);

        speed_value *= 0.97_f32.powi(self.n_misses as i32);

        if let Some(combo) = self.combo.filter(|_| attributes.max_combo > 0) {
            speed_value *= ((combo as f32 / attributes.max_combo as f32).powf(0.8)).min(1.0);
        }

        speed_value *= acc_factor(self.acc.unwrap(), SPEED_ACC_EXPONENT, SPEED_ACC_EXPONENT_BELOW_KNEE);
        speed_value *= 0.98 + attributes.od as f32 * attributes.od as f32 / 2500.0;

        speed_value
    }

    fn compute_accuracy_value(&self) -> f32 {
        let attributes = self.attributes.as_ref().unwrap();
        let n_circles = attributes.n_circles as f32;
        let better_acc_percentage = (n_circles > 0.0) as u8 as f32 * self.acc.unwrap();

        let mut acc_value =
            1.52163_f32.powf(attributes.od as f32) * better_acc_percentage.powf(acc_exponent(attributes.od)) * ACC_BASE;

        acc_value *= ((n_circles / 1000.0).powf(0.3)).min(1.15);

        if self.mods.hd() {
            acc_value *= 1.02;
        }

        if self.mods.fl() {
            acc_value *= 1.02;
        }

        acc_value
    }

    fn total_hits(&self) -> u32 {
        let n_objects = self.map.hit_objects.len() as u32;

        (self.n300.unwrap_or(0) + self.n100.unwrap_or(0) + self.n50.unwrap_or(0) + self.n_misses)
            .min(n_objects)
    }
}

pub trait OsuAttributeProvider {
    fn attributes(self) -> Option<OsuDifficultyAttributes>;
}

impl OsuAttributeProvider for OsuDifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<OsuDifficultyAttributes> {
        Some(self)
    }
}

impl OsuAttributeProvider for OsuPerformanceAttributes {
    #[inline]
    fn attributes(self) -> Option<OsuDifficultyAttributes> {
        Some(self.difficulty)
    }
}

fn acc_exponent(od: f64) -> f32 {
    let t = ((od - ACC_EXPONENT_LOW_OD_AT) / (ACC_EXPONENT_HIGH_OD_AT - ACC_EXPONENT_LOW_OD_AT))
        .clamp(0.0, 1.0) as f32;

    ACC_EXPONENT_LOW_OD + (ACC_EXPONENT_HIGH_OD - ACC_EXPONENT_LOW_OD) * t
}

fn acc_factor(acc: f32, above: f32, below: f32) -> f32 {
    if acc >= ACC_KNEE {
        acc.powf(above)
    } else {
        ACC_KNEE.powf(above) * (acc / ACC_KNEE).powf(below)
    }
}
