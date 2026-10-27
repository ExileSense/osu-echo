use rosu_pp_v4::model::control_point::{DifficultyPoint, TimingPoint};

pub trait Mods {
    fn nf(&self) -> bool;
    fn ez(&self) -> bool;
    fn hd(&self) -> bool;
    fn hr(&self) -> bool;
    fn ht(&self) -> bool;
    fn dt(&self) -> bool;
    fn fl(&self) -> bool;
    fn so(&self) -> bool;
}

impl Mods for u32 {
    fn nf(&self) -> bool {
        *self & (1 << 0) != 0
    }
    fn ez(&self) -> bool {
        *self & (1 << 1) != 0
    }
    fn hd(&self) -> bool {
        *self & (1 << 3) != 0
    }
    fn hr(&self) -> bool {
        *self & (1 << 4) != 0
    }
    fn dt(&self) -> bool {
        *self & (1 << 6) != 0
    }
    fn ht(&self) -> bool {
        *self & (1 << 8) != 0
    }
    fn fl(&self) -> bool {
        *self & (1 << 10) != 0
    }
    fn so(&self) -> bool {
        *self & (1 << 12) != 0
    }
}

pub fn timing_point_at(points: &[TimingPoint], time: f64) -> Option<&TimingPoint> {
    let i = points
        .binary_search_by(|probe| probe.time.total_cmp(&time))
        .unwrap_or_else(|i| i.saturating_sub(1));

    points.get(i)
}

pub fn difficulty_point_at(points: &[DifficultyPoint], time: f64) -> Option<&DifficultyPoint> {
    points
        .binary_search_by(|probe| probe.time.total_cmp(&time))
        .map_or_else(|i| i.checked_sub(1), Some)
        .map(|i| &points[i])
}
