//! L6 real-data smoke test (per bee's "test with real data" rule
//! and per IMPLEMENTATION_PLAN.md L6 def-of-done).
//!
//! L6 computes the Hodge complex + Betti numbers + Morse
//! compression on a bundle. This test loads the 20-record sensor
//! dataset, attaches a Kähler structure, and exercises the full
//! L6 surface:
//!
//! 1. `morse_compress()` returns Some with a sensible compression
//!    ratio on real data.
//! 2. Cohomology preservation always holds by construction.
//! 3. Euler characteristic computed from Betti matches the
//!    combinatorial V - E + F (Hodge ↔ Euler identity).
//! 4. A bundle without Kähler attached still produces a Morse
//!    snapshot because the construction only depends on the
//!    field-index graph, but downstream Marcella consumers gate on
//!    the Kähler-attached path. We test the no-Kähler case
//!    succeeds too — it's the same algorithm, no Kähler-specific
//!    branch.
//! 5. Sensor bundle has structure: at least one connected
//!    component (b_0 ≥ 1) on a non-empty bundle.

#![cfg(feature = "kahler")]

use gigi::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};
use gigi::types::{BundleSchema, FieldDef, Value};
use gigi::BundleStore;
use std::collections::HashMap;
use std::fs;

fn load_sensor_records() -> Vec<HashMap<String, Value>> {
    let path = std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{}/test_data/sensor_data.json", d))
        .expect("CARGO_MANIFEST_DIR not set");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path, e));
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("parse");
    parsed
        .as_array()
        .expect("array")
        .iter()
        .map(|item| {
            let obj = item.as_object().expect("object");
            let mut rec = HashMap::new();
            for (k, v) in obj {
                let val = match v {
                    serde_json::Value::String(s) => Value::Text(s.clone()),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            Value::Integer(i)
                        } else {
                            Value::Float(n.as_f64().expect("f64"))
                        }
                    }
                    serde_json::Value::Bool(b) => Value::Bool(*b),
                    _ => panic!("unexpected"),
                };
                rec.insert(k.clone(), val);
            }
            rec
        })
        .collect()
}

fn sensor_schema_with_kahler() -> BundleSchema {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.1, -0.1, 0.0], 2).expect("antisymmetric"),
    );
    let k = KahlerStructure::new(j, b);
    BundleSchema::new("sensor_hodge")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
        .index("status")
        .index("unit")
        .with_kahler(k)
}

#[test]
fn real_sensor_data_morse_compression_lifecycle() {
    let records = load_sensor_records();
    assert_eq!(records.len(), 20);

    let mut store = BundleStore::new(sensor_schema_with_kahler());
    for rec in &records {
        store.insert(rec);
    }
    assert_eq!(store.len(), 20);

    let m = store
        .morse_compress()
        .expect("≥ 2 records ⇒ morse_compress must return Some");

    // ── Cohomology preservation (always holds by construction) ──
    assert!(
        m.cohomology_preserved(),
        "Morse compression must preserve cohomology (Betti)"
    );

    // ── Sanity: original sizes match what we inserted ──
    assert_eq!(
        m.original_v, 20,
        "original vertex count = number of records (20)"
    );

    // ── At least one connected component on a non-empty bundle ──
    assert!(
        m.n_critical_0 >= 1,
        "non-empty bundle: b_0 ≥ 1; got {}",
        m.n_critical_0
    );

    // ── Hodge ↔ Euler identity ──
    // V - E + F must equal b_0 - b_1 + b_2 by the Euler-Poincaré
    // theorem. This is the cross-check that the algorithm is
    // internally consistent (matches Python test_11's V-E+F = 0
    // check on T²).
    let chi_topological = m.betti.euler_characteristic();
    let chi_combinatorial =
        m.original_v as i64 - m.original_e as i64 + m.original_f as i64;
    assert_eq!(
        chi_topological, chi_combinatorial,
        "Hodge↔Euler identity: b_0-b_1+b_2 ({}) must equal V-E+F ({})",
        chi_topological, chi_combinatorial
    );

    // ── Compression ratio sanity: ≥ 1 (you never expand) ──
    if m.n_critical() > 0 {
        assert!(
            m.compression_ratio() >= 1.0 - 1e-12,
            "compression ratio must be ≥ 1; got {}",
            m.compression_ratio()
        );
    }

    // ── Diagnostic ──
    println!(
        "L6 sensor smoke: V={}, E={}, F={}, Betti=({}, {}, {}), \
         critical=({}, {}, {}), compression={:.2}×",
        m.original_v,
        m.original_e,
        m.original_f,
        m.betti.b0,
        m.betti.b1,
        m.betti.b2,
        m.n_critical_0,
        m.n_critical_1,
        m.n_critical_2,
        m.compression_ratio()
    );
}

#[test]
fn real_sensor_data_no_kahler_also_works() {
    // The Morse compression depends on the cell complex, not the
    // Kähler structure. A bundle without Kähler attached still
    // produces a valid Morse snapshot — exercises the path
    // Marcella uses on legacy bundles.
    let records = load_sensor_records();
    let schema = BundleSchema::new("sensor_no_kahler")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
        .index("status");
    let mut store = BundleStore::new(schema);
    for rec in &records {
        store.insert(rec);
    }
    let m = store
        .morse_compress()
        .expect("Morse compression works on non-Kähler bundles too");
    assert_eq!(m.original_v, 20);
    assert!(m.n_critical_0 >= 1);
    assert!(m.cohomology_preserved());
}

// ── Step 4b instrumentation: sparsity of d_0, d_1 on real bundles ──
//
// Per the 2026-06-02 SEMANTIC perf letter: the rank-based Betti
// speedup depends on the boundary matrices being sparse. nnz of
// d_0 = 2·|E|, nnz of d_1 = 3·|F| by construction; what matters is
// |E|/|V| and |F|/|E|. This test reports the actual ratios on the
// real sensor fixture so we have a data point before quoting
// "sub-second on first call" to Marcella.
//
// The test ASSERTS conservative bounds (catches a complex that's
// gone unexpectedly dense) but its main job is to PRINT the
// measured numbers — read the test output to confirm sparsity
// against your bundle.

#[test]
fn d0_d1_sparsity_on_real_sensor_bundle() {
    use gigi::discrete::hodge_complex::nnz_report;
    use gigi::discrete::HodgeComplex;

    // Build the same store + complex the existing tests use.
    let records = load_sensor_records();
    let schema = BundleSchema::new("sparsity_smoke")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
        .index("status");
    let mut store = BundleStore::new(schema);
    for rec in &records {
        store.insert(rec);
    }

    // Pull the underlying HodgeComplex (re-build via the same path
    // morse_compress() uses, but bypass the Morse step so we can
    // inspect the raw d_0, d_1 shape).
    let bps: Vec<gigi::types::BasePoint> = store.sections().map(|(bp, _)| bp).collect();
    let n_vertices = bps.len();
    let bp_to_idx: std::collections::HashMap<gigi::types::BasePoint, usize> =
        bps.iter().enumerate().map(|(i, &b)| (b, i)).collect();
    let mut edge_set: std::collections::BTreeSet<(usize, usize)> =
        std::collections::BTreeSet::new();
    for &bp in &bps {
        for nb in store.geometric_neighbors(bp) {
            if let (Some(&i), Some(&j)) = (bp_to_idx.get(&bp), bp_to_idx.get(&nb)) {
                if i != j {
                    edge_set.insert((i.min(j), i.max(j)));
                }
            }
        }
    }
    let edges: Vec<(usize, usize)> = edge_set.iter().copied().collect();
    let adj: std::collections::HashMap<usize, std::collections::HashSet<usize>> = edges
        .iter()
        .flat_map(|&(a, b)| [(a, b), (b, a)])
        .fold(std::collections::HashMap::new(), |mut acc, (a, b)| {
            acc.entry(a).or_default().insert(b);
            acc
        });
    let mut face_set: std::collections::BTreeSet<(usize, usize, usize)> =
        std::collections::BTreeSet::new();
    for &(a, b) in &edges {
        if let (Some(na), Some(nb)) = (adj.get(&a), adj.get(&b)) {
            for &c in na.intersection(nb) {
                if c > b {
                    face_set.insert((a, b, c));
                }
            }
        }
    }
    let faces: Vec<(usize, usize, usize)> = face_set.into_iter().collect();
    let hc = HodgeComplex::new(n_vertices, edges, faces).expect("build complex");

    let report = nnz_report(&hc);
    // Print the measured numbers so this becomes a data point we can
    // cite in the Marcella perf letter follow-up.
    println!("\nSparsity report (sensor_data.json, 20 records):");
    println!("  |V| = {:>6}", report.n_vertices);
    println!("  |E| = {:>6}", report.n_edges);
    println!("  |F| = {:>6}", report.n_faces);
    println!("  edges_per_vertex = {:.3}", report.edges_per_vertex);
    println!("  nnz(d_0) = {:>6}  (= 2·|E|)", report.d0_nnz);
    println!("  nnz(d_1) = {:>6}  (= 3·|F|)", report.d1_nnz);
    println!("  density(d_0) = {:.6}", report.d0_density);
    println!("  density(d_1) = {:.6}", report.d1_density);

    // Sanity assertions.
    //   d_0 has exactly 2 nonzeros per row by construction.
    assert_eq!(report.d0_nnz, 2 * report.n_edges);
    //   d_1 has exactly 3 nonzeros per row by construction.
    assert_eq!(report.d1_nnz, 3 * report.n_faces);

    // Conservative bound: catch a complex that's gone unexpectedly
    // dense (sentinel for a regression in geometric_neighbors). For
    // the sensor bundle's 20 records, edges-per-vertex should sit
    // comfortably below 10. If this trips, the rank-based Betti's
    // perf characteristic on this fixture has changed and we need
    // to look.
    assert!(
        report.edges_per_vertex < 10.0,
        "sensor fixture has unexpectedly dense complex: edges_per_vertex = {} (expected < 10); \
         the rank-based Betti's speedup hinges on sparsity",
        report.edges_per_vertex
    );
}

/// Larger-scale sparsity probe — synthesize a 1k-vertex bundle with
/// a representative indexed-categorical (cardinality ≈ √n, so each
/// bucket has ≈ √n records → edges ≈ n · √n / 2 = O(n^{1.5})). This
/// is in the same shape regime as Marcella's
/// marcella_source_embeddings_bge_v2 (which we don't have access to
/// in test data, but its index structure should give similar
/// edges-per-vertex).
///
/// The point of this test: produce a quoted nnz ratio at scale we
/// can put in the Marcella follow-up letter without speculation.
#[test]
fn d0_d1_sparsity_on_synthetic_1k_bundle() {
    use gigi::discrete::hodge_complex::nnz_report;
    use gigi::discrete::HodgeComplex;

    let n_records = 1_000_usize;
    // Use ~32 buckets so each bucket has ~31 records → edges per
    // record ≈ 30. Matches the "moderate-cardinality categorical"
    // regime that real embedding bundles tend to live in.
    let n_buckets = 32_usize;
    let schema = BundleSchema::new("sparsity_1k")
        .base(FieldDef::categorical("id"))
        .fiber(FieldDef::numeric("x"))
        .fiber(FieldDef::categorical("bucket"))
        .index("bucket");
    let mut store = BundleStore::new(schema);
    for i in 0..n_records {
        let mut rec: HashMap<String, Value> = HashMap::new();
        rec.insert("id".into(), Value::Text(format!("r{:04}", i)));
        rec.insert("x".into(), Value::Float(i as f64 * 0.001));
        rec.insert(
            "bucket".into(),
            Value::Text(format!("b{:02}", i % n_buckets)),
        );
        store.insert(&rec);
    }

    // Re-build the HodgeComplex same way morse_compress does.
    let bps: Vec<gigi::types::BasePoint> = store.sections().map(|(bp, _)| bp).collect();
    let n_vertices = bps.len();
    let bp_to_idx: std::collections::HashMap<gigi::types::BasePoint, usize> =
        bps.iter().enumerate().map(|(i, &b)| (b, i)).collect();
    let mut edge_set: std::collections::BTreeSet<(usize, usize)> =
        std::collections::BTreeSet::new();
    for &bp in &bps {
        for nb in store.geometric_neighbors(bp) {
            if let (Some(&i), Some(&j)) = (bp_to_idx.get(&bp), bp_to_idx.get(&nb)) {
                if i != j {
                    edge_set.insert((i.min(j), i.max(j)));
                }
            }
        }
    }
    let edges: Vec<(usize, usize)> = edge_set.iter().copied().collect();
    let adj: std::collections::HashMap<usize, std::collections::HashSet<usize>> = edges
        .iter()
        .flat_map(|&(a, b)| [(a, b), (b, a)])
        .fold(std::collections::HashMap::new(), |mut acc, (a, b)| {
            acc.entry(a).or_default().insert(b);
            acc
        });
    let mut face_set: std::collections::BTreeSet<(usize, usize, usize)> =
        std::collections::BTreeSet::new();
    for &(a, b) in &edges {
        if let (Some(na), Some(nb)) = (adj.get(&a), adj.get(&b)) {
            for &c in na.intersection(nb) {
                if c > b {
                    face_set.insert((a, b, c));
                }
            }
        }
    }
    let faces: Vec<(usize, usize, usize)> = face_set.into_iter().collect();
    let hc = HodgeComplex::new(n_vertices, edges, faces).expect("build 1k complex");

    let report = nnz_report(&hc);
    println!("\nSparsity report (synthetic 1k-record / 32-bucket bundle):");
    println!("  |V| = {:>6}", report.n_vertices);
    println!("  |E| = {:>6}", report.n_edges);
    println!("  |F| = {:>6}", report.n_faces);
    println!("  edges_per_vertex = {:.3}", report.edges_per_vertex);
    println!("  nnz(d_0) = {:>6}", report.d0_nnz);
    println!("  nnz(d_1) = {:>6}", report.d1_nnz);
    println!("  density(d_0) = {:.6}", report.d0_density);
    println!("  density(d_1) = {:.6}", report.d1_density);

    // Per-row sparsity is fixed by construction; assert it loudly.
    assert_eq!(report.d0_nnz, 2 * report.n_edges);
    assert_eq!(report.d1_nnz, 3 * report.n_faces);

    // Honest bound: for a 1k bundle with 32 buckets, each bucket has
    // ~31 records → ~31·30/2 = 465 within-bucket edges, × 32 buckets
    // = ~15k edges. So edges-per-vertex should be ≈ 15 in the
    // happy case. If it exceeds 100 the index has somehow exploded
    // and we need to know.
    assert!(
        report.edges_per_vertex < 100.0,
        "1k-bucket-32 fixture exceeded expected edges/vertex: got {} (expected < 100)",
        report.edges_per_vertex
    );
}
