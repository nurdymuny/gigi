//! Substrate-side Laplacian extractor for sharded SPECTRAL.
//!
//! Closes the end-to-end loop on T7's distributed Lanczos: a caller
//! supplies a [`ShardedBundle`] and gets `λ_1(L)` back, with no manual
//! block extraction. This is the bundle-aware entry point that
//! composes:
//!
//! 1. Numeric fiber-field extraction (k-NN graph construction)
//! 2. Combinatorial Laplacian `L = D - A`
//! 3. 2-way Fiedler bisection for the (S, T) partition
//! 4. Block split `(A_S, A_T, B)`
//! 5. Distributed Lanczos
//!
//! For Fiedler-partitioned bundles with exactly 2 charts, step 3 is a
//! no-op — the existing chart assignment IS the bisection.

use crate::sharded::execution::ShardedExecError;
use crate::sharded::fiedler::{extract_coords, fiedler_vector, knn_adjacency, laplacian};
use crate::sharded::sharded_bundle::ShardedBundle;
use crate::sharded::spectral::{
    distributed_lanczos, DistributedLanczosConfig, DistributedLanczosResult,
};
use crate::types::Record;

/// Laplacian sharded into the three blocks distributed Lanczos needs.
#[derive(Clone, Debug)]
pub struct LaplacianBlocks {
    /// `A_S = L[S, S]`
    pub a_s: Vec<Vec<f64>>,
    /// `A_T = L[T, T]`
    pub a_t: Vec<Vec<f64>>,
    /// `B = L[S, T]`
    pub b: Vec<Vec<f64>>,
    pub size_s: usize,
}

/// Errors from substrate-side Laplacian extraction.
#[derive(Debug, Clone, PartialEq)]
pub enum LaplacianExtractError {
    /// The bundle has too few records to support k-NN graph construction.
    /// Need at least `k_neighbors + 1` records.
    TooFewRecords { n_records: usize, min_required: usize },
    /// At least one record lacked any numeric / vector fiber field for
    /// coordinate extraction.
    NoNumericFields,
    /// The Fiedler bisection produced an empty partition. Happens when
    /// the connectivity graph has no meaningful split (e.g., disconnected
    /// components).
    EmptyPartition,
}

/// Build the combinatorial k-NN Laplacian for all records in a sharded
/// bundle. Returns the full L matrix and the ordered Vec<Record>
/// matching its row/column indices.
///
/// Record iteration is in `ShardedBundle::records()` order (chart-id-
/// ordered for determinism; within a chart, the inner store's order).
pub fn build_bundle_laplacian(
    bundle: &ShardedBundle,
    k_neighbors: usize,
) -> Result<(Vec<Vec<f64>>, Vec<Record>), LaplacianExtractError> {
    // Collect records in deterministic order
    let records: Vec<Record> = bundle.records().collect();
    if records.is_empty() {
        return Err(LaplacianExtractError::TooFewRecords {
            n_records: 0,
            min_required: k_neighbors + 1,
        });
    }
    if records.len() < k_neighbors + 1 {
        return Err(LaplacianExtractError::TooFewRecords {
            n_records: records.len(),
            min_required: k_neighbors + 1,
        });
    }

    // Need a schema to extract fiber coords; pull from the first chart.
    let schema = bundle
        .chart_store(crate::sharded::atlas::ChartId(0))
        .map(|s| s.schema.clone());
    // Fallback: pull from any chart that exists
    let schema = match schema {
        Some(s) => s,
        None => {
            // No charts at all — shouldn't happen, but handle gracefully
            return Err(LaplacianExtractError::NoNumericFields);
        }
    };

    // Extract coords for every record
    let coords: Vec<Vec<f64>> = records
        .iter()
        .filter_map(|r| extract_coords(r, &schema))
        .collect();
    if coords.len() != records.len() {
        return Err(LaplacianExtractError::NoNumericFields);
    }

    let adj = knn_adjacency(&coords, k_neighbors);
    let l = laplacian(&adj);
    Ok((l, records))
}

/// Compute a 2-way Fiedler bisection of a Laplacian L. Returns a
/// boolean vector where `in_s[i] == true` means record i is in
/// partition S (sign-of-Fiedler-vector below median).
pub fn two_way_fiedler_partition_from_laplacian(
    l: &[Vec<f64>],
    max_iterations: usize,
) -> Result<Vec<bool>, LaplacianExtractError> {
    let n = l.len();
    if n < 2 {
        return Err(LaplacianExtractError::EmptyPartition);
    }
    let fv = fiedler_vector(l, max_iterations);
    if fv.iter().all(|x| x.abs() < 1e-12) {
        return Err(LaplacianExtractError::EmptyPartition);
    }
    let mut sorted_fv = fv.clone();
    sorted_fv.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted_fv[n / 2];
    let in_s: Vec<bool> = fv.iter().map(|&v| v < median).collect();
    let n_s = in_s.iter().filter(|&&b| b).count();
    if n_s == 0 || n_s == n {
        return Err(LaplacianExtractError::EmptyPartition);
    }
    Ok(in_s)
}

/// Split a full Laplacian into the three blocks distributed Lanczos
/// consumes, given a boolean partition vector.
pub fn split_laplacian_two_way(l: &[Vec<f64>], in_s: &[bool]) -> LaplacianBlocks {
    let n = l.len();
    debug_assert_eq!(in_s.len(), n);
    let s_indices: Vec<usize> = (0..n).filter(|&i| in_s[i]).collect();
    let t_indices: Vec<usize> = (0..n).filter(|&i| !in_s[i]).collect();
    let size_s = s_indices.len();

    let a_s: Vec<Vec<f64>> = s_indices
        .iter()
        .map(|&i| s_indices.iter().map(|&j| l[i][j]).collect())
        .collect();
    let a_t: Vec<Vec<f64>> = t_indices
        .iter()
        .map(|&i| t_indices.iter().map(|&j| l[i][j]).collect())
        .collect();
    let b: Vec<Vec<f64>> = s_indices
        .iter()
        .map(|&i| t_indices.iter().map(|&j| l[i][j]).collect())
        .collect();
    LaplacianBlocks {
        a_s,
        a_t,
        b,
        size_s,
    }
}

/// End-to-end sharded λ_1 from a bundle.
///
/// 1. Build the k-NN Laplacian from the bundle's records.
/// 2. Compute a 2-way Fiedler bisection (for partition S, T).
/// 3. Split L into blocks (A_S, A_T, B).
/// 4. Run distributed Lanczos.
///
/// This is the canonical entry point for `λ_1(L_bundle)` against any
/// sharded bundle — hash-partitioned, Fiedler-partitioned, or trivial.
/// The Fiedler bisection at step 2 ensures the partition respects the
/// graph structure, which is what T7's distributed Lanczos expects
/// for fast convergence on naturally-clustered substrates.
pub fn shard_lambda_1_from_bundle(
    bundle: &ShardedBundle,
    k_neighbors: usize,
    config: &DistributedLanczosConfig,
) -> Result<DistributedLanczosResult, ShardedExecError> {
    let (l, _records) =
        build_bundle_laplacian(bundle, k_neighbors).map_err(|e| match e {
            LaplacianExtractError::TooFewRecords {
                n_records,
                min_required,
            } => ShardedExecError::NotImplementedYet {
                phase: Box::leak(
                    format!(
                        "TooFewRecords (n={}, min={}) — supply a bundle with ≥ k+1 records",
                        n_records, min_required
                    )
                    .into_boxed_str(),
                ),
            },
            LaplacianExtractError::NoNumericFields => ShardedExecError::NotImplementedYet {
                phase: "NoNumericFields — bundle's fiber must include at least one Float/Integer/Vector field",
            },
            LaplacianExtractError::EmptyPartition => ShardedExecError::NotImplementedYet {
                phase: "EmptyPartition — Fiedler bisection produced no meaningful split (graph may be disconnected)",
            },
        })?;

    let in_s = two_way_fiedler_partition_from_laplacian(&l, 200).map_err(|_| {
        ShardedExecError::NotImplementedYet {
            phase: "Fiedler bisection failed — graph may be disconnected",
        }
    })?;

    let blocks = split_laplacian_two_way(&l, &in_s);
    let result = distributed_lanczos(&blocks.a_s, &blocks.a_t, &blocks.b, blocks.size_s, config);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharded::sharded_bundle::ShardedBundle;
    use crate::sharded::ShardId;
    use crate::types::{BundleSchema, FieldDef, Value};

    fn make_schema() -> BundleSchema {
        BundleSchema::new("laplacian_test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(10.0))
            .fiber(FieldDef::numeric("y").with_range(10.0))
    }

    /// Two well-separated 2D clusters (the TFP1 fixture).
    fn make_two_cluster_records(n_per_cluster: usize) -> Vec<Record> {
        let mut records = Vec::new();
        let mut state: u32 = 12345;
        let mut lcg = || -> f64 {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            (state as f64) / (u32::MAX as f64) - 0.5
        };
        let mut pk: i64 = 0;
        for &(cx, cy) in &[(-1.0_f64, 0.0_f64), (1.0, 0.0)] {
            for _ in 0..n_per_cluster {
                let x = cx + 0.3 * lcg() * 2.0;
                let y = cy + 0.3 * lcg() * 2.0;
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(pk));
                r.insert("x".into(), Value::Float(x));
                r.insert("y".into(), Value::Float(y));
                records.push(r);
                pk += 1;
            }
        }
        records
    }

    #[test]
    fn build_bundle_laplacian_produces_symmetric_psd_matrix() {
        let records = make_two_cluster_records(20);
        let bundle = ShardedBundle::wrap_hash_sharded(make_schema(), records, 4, ShardId(0));
        let (l, recs) = build_bundle_laplacian(&bundle, 4).unwrap();
        let n = l.len();
        assert_eq!(n, 40);
        assert_eq!(recs.len(), 40);
        // Symmetric
        for i in 0..n {
            for j in (i + 1)..n {
                assert!((l[i][j] - l[j][i]).abs() < 1e-12);
            }
        }
        // Row sums = 0 (Laplacian kernel direction is all-ones)
        for row in &l {
            let s: f64 = row.iter().sum();
            assert!(s.abs() < 1e-9, "row sum should be 0, got {}", s);
        }
        // Diagonal non-negative (PSD condition)
        for i in 0..n {
            assert!(l[i][i] >= 0.0);
        }
    }

    #[test]
    fn build_bundle_laplacian_refuses_too_few_records() {
        let records = make_two_cluster_records(2); // 4 records total
        let bundle = ShardedBundle::wrap_hash_sharded(make_schema(), records, 2, ShardId(0));
        let r = build_bundle_laplacian(&bundle, 10); // k=10 > n=4
        assert!(matches!(
            r,
            Err(LaplacianExtractError::TooFewRecords { .. })
        ));
    }

    #[test]
    fn build_bundle_laplacian_refuses_no_numeric_fiber() {
        let schema = BundleSchema::new("no_numeric")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"));
        let mut records = Vec::new();
        for i in 0..10 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("name".into(), Value::Text(format!("u_{}", i)));
            records.push(r);
        }
        let bundle = ShardedBundle::wrap_hash_sharded(schema, records, 2, ShardId(0));
        let r = build_bundle_laplacian(&bundle, 4);
        assert_eq!(r.err(), Some(LaplacianExtractError::NoNumericFields));
    }

    #[test]
    fn two_way_partition_separates_two_clusters() {
        let records = make_two_cluster_records(20);
        let bundle = ShardedBundle::wrap_hash_sharded(make_schema(), records, 4, ShardId(0));
        let (l, _) = build_bundle_laplacian(&bundle, 4).unwrap();
        let in_s = two_way_fiedler_partition_from_laplacian(&l, 200).unwrap();
        // Both S and T should be non-empty
        let n_s = in_s.iter().filter(|&&b| b).count();
        let n_t = in_s.iter().filter(|&&b| !b).count();
        assert!(n_s > 0 && n_t > 0);
        assert_eq!(n_s + n_t, 40);
        // Balanced within ±5
        assert!((n_s as i32 - n_t as i32).abs() <= 5);
    }

    #[test]
    fn split_blocks_reconstruct_full_matvec() {
        let records = make_two_cluster_records(15);
        let bundle = ShardedBundle::wrap_hash_sharded(make_schema(), records, 2, ShardId(0));
        let (l, _) = build_bundle_laplacian(&bundle, 4).unwrap();
        let in_s = two_way_fiedler_partition_from_laplacian(&l, 200).unwrap();
        let blocks = split_laplacian_two_way(&l, &in_s);

        // Block matvec on a test vector must equal full matvec after
        // index permutation (S first, then T).
        let n = l.len();
        let s_indices: Vec<usize> = (0..n).filter(|&i| in_s[i]).collect();
        let t_indices: Vec<usize> = (0..n).filter(|&i| !in_s[i]).collect();
        let perm: Vec<usize> = s_indices.iter().chain(t_indices.iter()).copied().collect();

        let v: Vec<f64> = (0..n).map(|i| (i as f64) * 0.1).collect();
        // Permute v
        let v_perm: Vec<f64> = perm.iter().map(|&i| v[i]).collect();
        let l_perm_v = crate::sharded::spectral::block_matvec(
            &blocks.a_s, &blocks.a_t, &blocks.b, &v_perm, blocks.size_s,
        );
        // Full matvec L * v in original index order
        let mut full = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                full[i] += l[i][j] * v[j];
            }
        }
        // Compare l_perm_v[k] with full[perm[k]]
        for k in 0..n {
            let expected = full[perm[k]];
            assert!(
                (l_perm_v[k] - expected).abs() < 1e-10,
                "block matvec mismatch at perm[{}]={}: expected {}, got {}",
                k, perm[k], expected, l_perm_v[k]
            );
        }
    }

    #[test]
    fn shard_lambda_1_from_bundle_end_to_end_on_two_clusters() {
        // End-to-end: build bundle, extract Laplacian, partition,
        // run Lanczos. Compare against direct eigendecomp.
        let records = make_two_cluster_records(20);
        let bundle = ShardedBundle::wrap_hash_sharded(make_schema(), records, 4, ShardId(0));
        let (l, _) = build_bundle_laplacian(&bundle, 4).unwrap();

        // Ground truth via direct eigendecomp
        let n = l.len();
        let mut m = nalgebra::DMatrix::<f64>::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                m[(i, j)] = l[i][j];
            }
        }
        let eigen = nalgebra::SymmetricEigen::new(m);
        let mut eigs: Vec<f64> = eigen.eigenvalues.iter().copied().collect();
        eigs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let truth = eigs
            .iter()
            .copied()
            .find(|&e| e > 1e-9)
            .expect("expected non-zero eigenvalue");

        let config = DistributedLanczosConfig {
            k_max: 60,
            ..Default::default()
        };
        let result = shard_lambda_1_from_bundle(&bundle, 4, &config).unwrap();
        let rel_err = (result.lambda_1 - truth).abs() / truth;
        assert!(
            rel_err < 1e-3,
            "end-to-end λ_1 mismatch: truth={}, got={}, rel err={:.2e}, K used={}",
            truth, result.lambda_1, rel_err, result.iterations_used
        );
    }

    #[test]
    fn shard_lambda_1_from_bundle_works_on_fiedler_bundle() {
        // Fiedler-partitioned bundle should also work end-to-end
        let records = make_two_cluster_records(20);
        let bundle = ShardedBundle::wrap_fiedler_sharded(
            make_schema(),
            records,
            4,
            ShardId(0),
        ).unwrap();
        let config = DistributedLanczosConfig::default();
        let result = shard_lambda_1_from_bundle(&bundle, 4, &config);
        assert!(result.is_ok(), "Fiedler bundle should succeed: {:?}", result.err());
        let r = result.unwrap();
        assert!(r.lambda_1 > 0.0, "λ_1 should be positive for connected graph");
    }

    #[test]
    fn shard_lambda_1_from_bundle_works_on_trivial_bundle() {
        // Trivial-atlas bundle (single chart) should also work
        let records = make_two_cluster_records(20);
        let mut inner = crate::bundle::BundleStore::new(make_schema());
        for r in records {
            inner.insert(&r);
        }
        let bundle = ShardedBundle::wrap_trivial(inner, ShardId(0));
        let config = DistributedLanczosConfig::default();
        let result = shard_lambda_1_from_bundle(&bundle, 4, &config);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.lambda_1 > 0.0);
    }
}
