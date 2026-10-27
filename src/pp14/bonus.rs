use super::tuning::{
    BONUS_BELOW_CUTOFF_MAX, BONUS_CUTOFF_RATIO, BONUS_MAX, BONUS_RANKS, BONUS_TOP_SIZE,
};

pub fn placement(score: i64, top: &[i64]) -> usize {
    top.iter().filter(|s| **s >= score).count() + 1
}

pub fn multiplier(score: i64, top: &[i64]) -> f64 {
    if top.len() < BONUS_TOP_SIZE || score <= 0 {
        return 1.0;
    }
    let cutoff = top.iter().copied().min().unwrap_or(0);
    if cutoff <= 0 {
        return 1.0;
    }
    let rank = placement(score, top);
    let bonus = if rank <= BONUS_TOP_SIZE {
        BONUS_RANKS
            .iter()
            .find(|(max_rank, _)| rank <= *max_rank)
            .map_or(0.0, |(_, b)| *b)
    } else {
        let ratio = score as f64 / cutoff as f64;
        if ratio <= BONUS_CUTOFF_RATIO {
            0.0
        } else {
            BONUS_BELOW_CUTOFF_MAX * (ratio - BONUS_CUTOFF_RATIO) / (1.0 - BONUS_CUTOFF_RATIO)
        }
    };
    1.0 + bonus.clamp(0.0, BONUS_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top() -> Vec<i64> {
        (0..50).map(|i| 2_000_000 - i * 20_000).collect()
    }

    #[test]
    fn no_bonus_without_a_full_top_50() {
        assert_eq!(multiplier(5_000_000, &[]), 1.0);
        assert_eq!(multiplier(5_000_000, &top()[..49]), 1.0);
    }

    #[test]
    fn no_bonus_at_or_below_75_percent_of_the_50th_score() {
        let t = top();
        let cut = *t.last().unwrap();
        assert_eq!(multiplier((cut as f64 * 0.75) as i64, &t), 1.0);
        assert_eq!(multiplier(10, &t), 1.0);
        assert!(multiplier((cut as f64 * 0.9) as i64, &t) > 1.0);
    }

    #[test]
    fn bonus_grows_with_placement_and_is_capped() {
        let t = top();
        let cut = *t.last().unwrap();
        let below = multiplier(cut, &t);
        let p50 = multiplier(cut + 1, &t);
        let p40 = multiplier(t[39] + 1, &t);
        let p20 = multiplier(t[19] + 1, &t);
        let p1 = multiplier(t[0] + 1, &t);
        assert!(1.0 < below && below < p50 && p50 < p40 && p40 < p20 && p20 < p1);
        assert!((p1 - 1.0 - BONUS_MAX).abs() < 1e-12);
        assert!(multiplier(i64::MAX, &t) <= 1.0 + BONUS_MAX + 1e-12);
    }
}
