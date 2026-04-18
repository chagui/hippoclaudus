//! Distribution / percentile computation for session analytics.
//!
//! Uses the nearest-rank method (no interpolation). Good enough for
//! human-facing stats; swap for linear interpolation if anyone ever uses
//! these values for SLO math.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Distribution {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

impl Distribution {
    pub const EMPTY: Distribution = Distribution {
        count: 0,
        min: 0.0,
        max: 0.0,
        mean: 0.0,
        p50: 0.0,
        p90: 0.0,
        p95: 0.0,
        p99: 0.0,
    };
}

/// Compute a Distribution from an unsorted slice of samples.
pub fn compute_distribution(samples: &[f64]) -> Distribution {
    if samples.is_empty() {
        return Distribution::EMPTY;
    }

    let mut sorted: Vec<f64> = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let count = sorted.len();
    let sum: f64 = sorted.iter().sum();
    Distribution {
        count,
        min: sorted[0],
        max: sorted[count - 1],
        mean: sum / count as f64,
        p50: nearest_rank(&sorted, 0.50),
        p90: nearest_rank(&sorted, 0.90),
        p95: nearest_rank(&sorted, 0.95),
        p99: nearest_rank(&sorted, 0.99),
    }
}

/// Nearest-rank percentile on an already-sorted slice.
/// Rank = ceil(p * n), 1-indexed. For p=0 we return the min.
fn nearest_rank(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if p <= 0.0 {
        return sorted[0];
    }
    if p >= 1.0 {
        return sorted[n - 1];
    }
    let rank = (p * n as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(n - 1);
    sorted[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_zero_distribution() {
        let d = compute_distribution(&[]);
        assert_eq!(d.count, 0);
        assert_eq!(d.p50, 0.0);
        assert_eq!(d.p99, 0.0);
    }

    #[test]
    fn single_element_percentiles_all_equal() {
        let d = compute_distribution(&[42.0]);
        assert_eq!(d.count, 1);
        assert_eq!(d.min, 42.0);
        assert_eq!(d.max, 42.0);
        assert_eq!(d.mean, 42.0);
        assert_eq!(d.p50, 42.0);
        assert_eq!(d.p99, 42.0);
    }

    #[test]
    fn all_equal_samples() {
        let d = compute_distribution(&[7.0; 10]);
        assert_eq!(d.min, 7.0);
        assert_eq!(d.max, 7.0);
        assert_eq!(d.p50, 7.0);
        assert_eq!(d.p99, 7.0);
        assert_eq!(d.mean, 7.0);
    }

    #[test]
    fn percentiles_monotone_on_1_to_100() {
        let samples: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let d = compute_distribution(&samples);
        assert_eq!(d.count, 100);
        assert_eq!(d.min, 1.0);
        assert_eq!(d.max, 100.0);
        // Nearest-rank: p50 rank = ceil(0.5*100) = 50 -> sorted[49] = 50.0
        assert_eq!(d.p50, 50.0);
        // p90 rank = 90 -> sorted[89] = 90.0
        assert_eq!(d.p90, 90.0);
        assert_eq!(d.p95, 95.0);
        assert_eq!(d.p99, 99.0);
        assert!(d.p50 <= d.p90 && d.p90 <= d.p95 && d.p95 <= d.p99);
    }

    #[test]
    fn unsorted_input_handled() {
        let d = compute_distribution(&[5.0, 1.0, 3.0, 4.0, 2.0]);
        assert_eq!(d.min, 1.0);
        assert_eq!(d.max, 5.0);
        assert_eq!(d.mean, 3.0);
        // p50 rank = ceil(0.5*5) = 3 -> sorted[2] = 3.0
        assert_eq!(d.p50, 3.0);
    }

    #[test]
    fn percentiles_always_monotone_invariant() {
        // Randomish integer samples — check the monotone invariant always holds.
        let samples: Vec<f64> = vec![
            10.0, 3.0, 77.0, 1.0, 2.0, 2.0, 5.0, 9.0, 14.0, 21.0, 100.0, 50.0, 30.0,
        ];
        let d = compute_distribution(&samples);
        assert!(d.min <= d.p50);
        assert!(d.p50 <= d.p90);
        assert!(d.p90 <= d.p95);
        assert!(d.p95 <= d.p99);
        assert!(d.p99 <= d.max);
    }
}
