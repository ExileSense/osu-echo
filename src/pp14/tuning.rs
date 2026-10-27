pub const SPEED_SKILL_MULTIPLIER: f32 = 1400.0;
pub const SPEED_STRAIN_DECAY_BASE: f32 = 0.3;
pub const AIM_SKILL_MULTIPLIER: f32 = 26.25;
pub const AIM_STRAIN_DECAY_BASE: f32 = 0.15;
pub const DECAY_WEIGHT: f32 = 0.9;

pub const SPEED_STRAIN_PP_DIVISOR: f32 = 0.045;
pub const AIM_STRAIN_PP_DIVISOR: f32 = 0.0445;

pub const AIM_PP_SCALE: f32 = 1.27;
pub const SPEED_PP_SCALE: f32 = 0.5;
pub const ACC_PP_SCALE: f32 = 0.8;
pub const ACC_EXPONENT_LOW_OD: f32 = 50.0;
pub const ACC_EXPONENT_HIGH_OD: f32 = 34.0;
pub const ACC_EXPONENT_LOW_OD_AT: f64 = 8.0;
pub const ACC_EXPONENT_HIGH_OD_AT: f64 = 10.0;
pub const ACC_BASE: f32 = 2.738;
pub const AIM_ACC_EXPONENT: f32 = 8.0;
pub const SPEED_ACC_EXPONENT: f32 = 8.0;
pub const ACC_KNEE: f32 = 0.95;
pub const AIM_ACC_EXPONENT_BELOW_KNEE: f32 = 6.0;
pub const SPEED_ACC_EXPONENT_BELOW_KNEE: f32 = 6.0;
pub const TOTAL_PP_MULTIPLIER: f32 = 1.1;

pub const BONUS_TOP_SIZE: usize = 50;
pub const BONUS_CUTOFF_RATIO: f64 = 0.75;
pub const BONUS_BELOW_CUTOFF_MAX: f64 = 0.03;
pub const BONUS_RANKS: [(usize, f64); 6] = [
    (1, 0.30),
    (5, 0.25),
    (10, 0.20),
    (29, 0.15),
    (49, 0.10),
    (50, 0.05),
];
pub const BONUS_MAX: f64 = 0.30;
