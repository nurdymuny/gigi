//! `imagine_halo` — populate boundary halo records for a chart so
//! sharded CURVATURE becomes partition-invariant per T12.
//!
//! T12 validation:
//! [`theory/imagine/validation/t12_halo_partition_invariance.py`].
//! Result: same 60-record dataset partitioned into n_charts ∈ {2, 4, 8},
//! with halos populated, produces **exactly identical** aggregate
//! k_sum across all three partitions — zero residual, not first-order
//! bounded slack.
//!
//! This Rust implementation mirrors the Python algorithm directly:
//! for each record in the target chart, find which records from
//! OTHER charts are in its k-nearest-neighbors (over the union of
//! all candidates), and project those into halo entries.

use crate::imagine::config::HaloConfig;
use crate::imagine::provenance::{ImaginedProvenance, ImaginedRecord};

/// Compute the halo for a target chart given its records and the
/// records of other charts (intra-bundle hash-sharded case — the
/// "imagined" projection is identity since both halves of the bundle
/// share the same coordinate system).
///
/// For cross-atlas joins (Phase F), this primitive would compose
/// with a bridge transition. Phase 1 ships the identity-projection
/// intra-bundle path used by Phase D.
///
/// Returns one `ImaginedRecord` per record that participates in some
/// target-chart record's k-NN. Each carries `ImaginedProvenance::Halo`.
///
/// Per T12: when `imagine_halo` is called for every chart and the
/// per-chart k-NN computation uses (chart records ∪ halo records)
/// as candidates, the aggregate K_sum across charts equals the
/// direct single-shard K_sum exactly.
pub fn imagine_halo(
    target_chart_id: u32,
    source_chart_id: u32,
    target_chart_records: &[(String, Vec<f64>)],
    other_chart_records: &[(String, Vec<f64>)],
    config: &HaloConfig,
) -> Vec<ImaginedRecord> {
    if other_chart_records.is_empty() || target_chart_records.is_empty() {
        return Vec::new();
    }
    let k = config.k_neighbors.max(1);
    // Combined candidate pool — same as T12's procedure
    let mut all_candidates: Vec<(String, &Vec<f64>)> = Vec::with_capacity(
        target_chart_records.len() + other_chart_records.len(),
    );
    for (id, coords) in target_chart_records {
        all_candidates.push((id.clone(), coords));
    }
    for (id, coords) in other_chart_records {
        all_candidates.push((id.clone(), coords));
    }
    let other_ids: std::collections::HashSet<&String> = other_chart_records
        .iter()
        .map(|(id, _)| id)
        .collect();

    let mut halo_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (tid, tcoords) in target_chart_records {
        // Find k smallest squared distances from tcoords to other candidates
        let mut dists: Vec<(f64, &String)> = all_candidates
            .iter()
            .filter(|(id, _)| id != tid)
            .map(|(id, c)| {
                let d2: f64 = c.iter()
                    .zip(tcoords.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum();
                (d2, id)
            })
            .collect();
        // Partial sort: get the k smallest
        let k_eff = k.min(dists.len());
        let n_dists = dists.len();
        if n_dists > 0 && k_eff > 0 {
            let pivot_idx = k_eff.saturating_sub(1).min(n_dists - 1);
            dists.select_nth_unstable_by(
                pivot_idx,
                |a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal),
            );
        }
        // The first k_eff entries are the k smallest (in unsorted order)
        for (_d2, id) in dists.into_iter().take(k_eff) {
            if other_ids.contains(id) {
                halo_ids.insert(id.clone());
            }
        }
    }

    let mut halo: Vec<ImaginedRecord> = Vec::new();
    for (id, coords) in other_chart_records {
        if !halo_ids.contains(id) {
            continue;
        }
        // Enforce max_halo_records ceiling
        if halo.len() >= config.max_halo_records {
            break;
        }
        halo.push(ImaginedRecord {
            coords: coords.clone(),
            // Halo records inherit K from substrate at the projected
            // point. Phase 1 places a placeholder 0.0; consumer
            // computes the real value when needed.
            local_k: 0.0,
            accumulated_holonomy: 0.0,
            provenance: ImaginedProvenance::Halo {
                source_chart: source_chart_id,
                target_chart: target_chart_id,
                seed_record_id: id.clone(),
                // Intra-bundle identity projection — Lipschitz = 1.0.
                transition_lipschitz: 1.0,
            },
        });
    }
    halo
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic substrate matching T12's setup: 3D points on a noisy
    /// sphere, split into two charts by index parity.
    fn make_dataset() -> Vec<(String, Vec<f64>)> {
        let mut records = Vec::new();
        for i in 0..40_usize {
            let t = (i as f64) * 0.157;
            records.push((
                format!("rec_{}", i),
                vec![t.cos(), t.sin(), (t * 1.7).sin() * 0.4],
            ));
        }
        records
    }

    #[test]
    fn halo_contains_records_from_other_chart() {
        let all = make_dataset();
        let target: Vec<_> = all.iter().filter(|(id, _)| {
            id.trim_start_matches("rec_").parse::<u32>().unwrap() % 2 == 0
        }).cloned().collect();
        let other: Vec<_> = all.iter().filter(|(id, _)| {
            id.trim_start_matches("rec_").parse::<u32>().unwrap() % 2 == 1
        }).cloned().collect();
        let config = HaloConfig { max_halo_records: 64, k_neighbors: 8 };
        let halo = imagine_halo(0, 1, &target, &other, &config);

        assert!(!halo.is_empty(), "halo must be non-empty when both charts have records");
        for h in &halo {
            assert!(matches!(h.provenance, ImaginedProvenance::Halo { source_chart: 1, target_chart: 0, .. }),
                    "halo record has wrong provenance");
        }
    }

    #[test]
    fn halo_respects_max_halo_records_cap() {
        let all = make_dataset();
        let target: Vec<_> = all.iter().take(5).cloned().collect();
        let other: Vec<_> = all.iter().skip(5).cloned().collect();
        let config = HaloConfig { max_halo_records: 3, k_neighbors: 20 };
        let halo = imagine_halo(0, 1, &target, &other, &config);
        assert!(halo.len() <= 3, "halo exceeded max_halo_records ceiling");
    }

    #[test]
    fn halo_is_empty_when_no_other_records() {
        let target = vec![("a".to_string(), vec![0.0, 0.0])];
        let other: Vec<(String, Vec<f64>)> = Vec::new();
        let config = HaloConfig::default();
        let halo = imagine_halo(0, 1, &target, &other, &config);
        assert!(halo.is_empty());
    }

    #[test]
    fn halo_records_have_correct_provenance_kind() {
        let all = make_dataset();
        let target: Vec<_> = all.iter().take(10).cloned().collect();
        let other: Vec<_> = all.iter().skip(10).cloned().collect();
        let config = HaloConfig::default();
        let halo = imagine_halo(2, 5, &target, &other, &config);
        for h in &halo {
            assert_eq!(h.provenance_kind(), "halo");
        }
    }

    /// Synthetic K function matching T12's Python: K(record_i) =
    /// mean of k smallest squared distances to other points in the
    /// candidate set.
    fn synthetic_k(target: &[f64], candidates: &[(String, Vec<f64>)], k: usize) -> f64 {
        let mut dists: Vec<f64> = candidates
            .iter()
            .map(|(_, c)| {
                c.iter()
                    .zip(target.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
            })
            .filter(|&d| d > 1e-15) // exclude self
            .collect();
        if dists.is_empty() {
            return 0.0;
        }
        let k_eff = k.min(dists.len());
        let n_dists = dists.len();
        let pivot_idx = k_eff.saturating_sub(1).min(n_dists - 1);
        dists.select_nth_unstable_by(
            pivot_idx,
            |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
        );
        let sum: f64 = dists.iter().take(k_eff).sum();
        sum / (k_eff as f64)
    }

    fn hash_partition_records(
        records: &[(String, Vec<f64>)],
        n_charts: u32,
    ) -> std::collections::HashMap<u32, Vec<(String, Vec<f64>)>> {
        let mut out: std::collections::HashMap<u32, Vec<(String, Vec<f64>)>> =
            std::collections::HashMap::new();
        for (i, r) in records.iter().enumerate() {
            let chart = (i as u32) % n_charts;
            out.entry(chart).or_default().push(r.clone());
        }
        out
    }

    /// Rust integration test mirroring T12 Python validation:
    /// the same dataset partitioned into 2, 4, 8 charts must produce
    /// identical synthetic-K aggregates when halos are populated via
    /// `imagine_halo`.
    ///
    /// This is the Rust-side proof that the IMAGINE pivot solves the
    /// Phase D fragmentation finding — the encrypt-style gauge-
    /// equivariance holds in Rust as well as in Python.
    #[test]
    fn halos_make_synthetic_k_sum_partition_invariant() {
        let records = make_dataset(); // 40 records on a noisy ring
        let k = 8;
        let config = HaloConfig {
            max_halo_records: 256,
            k_neighbors: k,
        };

        // Baseline: aggregate over the full dataset
        let baseline: f64 = records
            .iter()
            .map(|(_, coords)| synthetic_k(coords, &records, k))
            .sum();

        // For each partition count, compute the per-chart aggregate
        // using chart_records UNION halo.
        let mut partition_aggregates: Vec<(u32, f64)> = Vec::new();
        for &n_charts in &[2u32, 4, 8] {
            let partition = hash_partition_records(&records, n_charts);
            let mut aggregate = 0.0_f64;
            for (chart_id, chart_records) in &partition {
                // Other records = everything not in this chart
                let chart_ids: std::collections::HashSet<&String> =
                    chart_records.iter().map(|(id, _)| id).collect();
                let other_records: Vec<(String, Vec<f64>)> = records
                    .iter()
                    .filter(|(id, _)| !chart_ids.contains(id))
                    .cloned()
                    .collect();
                let halo = imagine_halo(
                    *chart_id,
                    99,
                    chart_records,
                    &other_records,
                    &config,
                );
                // Construct the candidate set = chart_records + halo
                let mut candidates: Vec<(String, Vec<f64>)> = chart_records.clone();
                for h in &halo {
                    if let ImaginedProvenance::Halo { seed_record_id, .. } = &h.provenance {
                        candidates.push((seed_record_id.clone(), h.coords.clone()));
                    }
                }
                // Sum synthetic-K over the chart's OWN records using
                // chart + halo as candidates
                for (_, target_coords) in chart_records {
                    aggregate += synthetic_k(target_coords, &candidates, k);
                }
            }
            partition_aggregates.push((n_charts, aggregate));
        }

        // All three partitions must match the baseline to floating-
        // point precision -- the T12 result reproduced in Rust.
        for &(n_charts, agg) in &partition_aggregates {
            assert!(
                (agg - baseline).abs() < 1e-9,
                "n_charts={}: aggregate {} differs from baseline {} (delta={:.2e})",
                n_charts, agg, baseline, (agg - baseline).abs()
            );
        }
        // And to each other (trivially follows but worth asserting)
        let spread = partition_aggregates
            .iter()
            .map(|(_, a)| *a)
            .fold(f64::NEG_INFINITY, f64::max)
            - partition_aggregates
                .iter()
                .map(|(_, a)| *a)
                .fold(f64::INFINITY, f64::min);
        assert!(
            spread < 1e-9,
            "cross-partition spread {:.2e} exceeds 1e-9 -- gauge-equivariance broken",
            spread,
        );
    }
}
