//! Query engine — sheaf evaluation, query algebra, confidence annotation.
//!
//! Implements §8: Query Engine (Layer 2).

use crate::types::Record;

/// Query result annotated with geometric metadata (§8.2).
#[derive(Debug)]
pub struct QueryResult {
    pub records: Vec<Record>,
    /// 1/(1+K) — from local curvature.
    pub confidence: f64,
    /// K at query region.
    pub curvature: f64,
    /// C = τ/K — Davis capacity.
    pub capacity: f64,
    /// Average deviation norm across results.
    pub deviation_norm: f64,
}

/// Recall and deviation from the double cover (Def 6.1).
///
/// S = |correct returned| / |total correct|
/// d = √(1 - S)
/// S + d² = 1 (Theorem 6.1)
pub fn recall_deviation(returned: usize, total_correct: usize) -> (f64, f64) {
    if total_correct == 0 {
        return if returned == 0 { (1.0, 0.0) } else { (0.0, 1.0) };
    }
    let s = returned.min(total_correct) as f64 / total_correct as f64;
    let d = (1.0 - s).max(0.0).sqrt();
    (s, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD-6.1 / TDD-6.2: Exact query → S=1, d=0.
    #[test]
    fn tdd_6_1_exact_recall() {
        let (s, d) = recall_deviation(10, 10);
        assert!((s - 1.0).abs() < 1e-15);
        assert!(d.abs() < 1e-15);
    }

    /// TDD-6.3: S + d² = 1.
    #[test]
    fn tdd_6_3_double_cover() {
        for returned in [0, 3, 7, 10] {
            let (s, d) = recall_deviation(returned, 10);
            assert!((s + d * d - 1.0).abs() < 1e-14, "S + d² ≠ 1 for {returned}/10");
        }
    }

    /// TDD-6.5: d = √(1-S).
    #[test]
    fn tdd_6_5_deviation_identity() {
        let (s, d) = recall_deviation(6, 10);
        let expected_d = (1.0 - s).sqrt();
        assert!((d - expected_d).abs() < 1e-14);
    }
}
