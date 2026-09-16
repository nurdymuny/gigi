//! Can GIGI host and measure an arbitrary weighted graph -- the SSSP kind?
//!
//! I told Bee twice that it could not, on the grounds that GIGI's graph is the
//! FIELD-INDEX graph: base points joined when they share an indexed field
//! value, a union of cliques induced by stored records. That is true of
//! `spectral::field_index_graph` and it is not the only graph path.
//!
//! `spectral_gauge_spectrum` reads an EDGE bundle -- one record per edge, with
//! `vertex_a` and `vertex_b` as base fields -- and assembles a weighted graph
//! Laplacian from it. That is how Halcyon stores the buckyball lattice, and it
//! accepts any graph at all.
//!
//! So this fixture takes a graph of exactly the shape the SSSP paper
//! benchmarks -- a sparse random weighted digraph, average degree 4 -- stores
//! it as a GIGI bundle, and asks the engine geometric questions about it.

use std::fs;

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("gigi_ssspgraph_{tag}"))
}

/// Deterministic sparse weighted graph, the paper's `random_sparse_graph`
/// shape: n vertices, average out-degree 4, weights in [1, 100).
fn sparse_edges(n: usize, avg_degree: usize, seed: u64) -> Vec<(usize, usize, f64)> {
    let mut s = seed | 1;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let mut edges = Vec::new();
    for u in 0..n {
        let deg = 1 + (next() as usize) % (avg_degree * 2).min(n - 1).max(1);
        for _ in 0..deg {
            let v = (next() as usize) % n;
            if v != u {
                let w = 1.0 + (next() % 99) as f64;
                edges.push((u, v, w));
            }
        }
    }
    edges
}

#[test]
fn an_sssp_weighted_graph_is_a_gigi_bundle() {
    let d = dir("edges");
    let _ = fs::remove_dir_all(&d);

    let n = 2000usize;
    // Dedup by endpoint pair, keeping the lightest parallel edge. GIGI keys an
    // edge bundle on (vertex_a, vertex_b), so a multigraph upserts down to a
    // simple graph -- which is exactly the right semantics for shortest paths,
    // where only the lightest parallel edge can ever be on a geodesic. A true
    // multigraph would need an explicit edge id in the base.
    let edges: Vec<(usize, usize, f64)> = {
        let mut m: std::collections::BTreeMap<(usize, usize), f64> = Default::default();
        for (u, v, w) in sparse_edges(n, 4, 42) {
            m.entry((u, v)).and_modify(|e| { if w < *e { *e = w } }).or_insert(w);
        }
        m.into_iter().map(|((u, v), w)| (u, v, w)).collect()
    };

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        // The edge-bundle shape SPECTRAL_GAUGE reads: endpoints in the base,
        // edge data in the fiber.
        e.create_bundle(
            BundleSchema::new("road")
                .base(FieldDef::numeric("vertex_a"))
                .base(FieldDef::numeric("vertex_b"))
                .fiber(FieldDef::numeric("weight")),
        )
        .unwrap();

        for (u, v, w) in &edges {
            let mut r = Record::new();
            r.insert("vertex_a".into(), Value::Integer(*u as i64));
            r.insert("vertex_b".into(), Value::Integer(*v as i64));
            r.insert("weight".into(), Value::Float(*w));
            e.insert("road", &r).unwrap();
        }

        let store = e.bundle("road").expect("bundle present");
        assert_eq!(
            store.len(),
            edges.len(),
            "every edge must be stored as a record"
        );

        // The engine answers geometric questions about it.
        // BundleRef exposes the storage-agnostic view; these two take the
        // heap store directly.
        let k = store.scalar_curvature();
        let heap = store.as_heap().expect("fresh bundle is heap-backed");
        let (b0, b1) = gigi::spectral::betti_numbers(heap);
        let cap = gigi::curvature::capacity(1.0, k.abs().max(1e-12));

        println!("SSSP graph as a GIGI bundle: {} edges over {n} vertices", edges.len());
        println!("  scalar curvature K = {k:.6}");
        println!("  confidence         = {:.4}", gigi::curvature::confidence(k));
        println!("  Betti              = b0 {b0}, b1 {b1}");
        println!("  capacity tau/K     = {cap:.2}  (tau = 1)");

        assert!(k.is_finite(), "curvature must be a number on a real graph");
        assert!(b0 >= 1, "a non-empty graph has at least one component");

        // b0 == record count says the DEFAULT spectral path is not seeing the
        // graph: betti_numbers reads the FIELD-INDEX graph, which joins records
        // that share an indexed field value, and nothing is indexed yet. So the
        // graph is stored but invisible as a graph.
        assert_eq!(b0, edges.len(), "unindexed: every edge is its own component");

        // Index the source endpoint. Now records sharing a source vertex are
        // joined, and the field-index graph becomes a real object over the
        // edges -- adjacency by common tail.
        drop(store);
        e.add_index("road", "vertex_a").expect("index vertex_a");
        let store = e.bundle("road").expect("bundle present");
        let heap = store.as_heap().expect("heap-backed");
        let (ib0, ib1) = gigi::spectral::betti_numbers(heap);
        let gap = gigi::spectral::spectral_gap(heap);
        println!("  after INDEX vertex_a: Betti b0 {ib0}, b1 {ib1}, gap {gap:?}");
        assert!(
            ib0 < b0,
            "indexing an endpoint must connect records: {ib0} components vs {b0} unindexed"
        );
    }

    // And it survives the durability path, so the graph is a first-class
    // citizen rather than a scratch structure.
    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.snapshot().expect("snapshot must succeed");
    }
    {
        let e = Engine::open_mmap(&d).expect("must reopen");
        let store = e.bundle("road").expect("bundle present after reload");
        assert_eq!(store.len(), edges.len(), "all edges must reload");
        let rec = store.records().next().expect("a record");
        for f in ["vertex_a", "vertex_b", "weight"] {
            assert!(rec.get(f).is_some(), "edge field {f} must survive reload");
        }
    }

    let _ = fs::remove_dir_all(&d);
}
