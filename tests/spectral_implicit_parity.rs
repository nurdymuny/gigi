//! TDD-SBF gates — the implicit clique operator must return exactly the
//! numbers the explicit adjacency walk returned, and must do it without
//! building the graph.
//!
//! The whole premise of the change is "same graph, same answers, no edges."
//! So the gates are parity gates against a brute-force O(N²) oracle that
//! shares no code with either implementation, plus a scale gate that would
//! fail loudly if the quadratic path crept back in.
//!
//! SBF-1  edge count + degrees exact vs brute force
//! SBF-2  λ₁ parity implicit vs explicit
//! SBF-3  disconnected and perfect-clique shortcuts unchanged
//! SBF-4  (β₀, β₁) exact
//! SBF-5  scale + memory at 50k records
//!
//! The fixtures deliberately force *bucket overlap* — records agreeing on
//! several fields at once — because the natural first bug in an
//! inclusion–exclusion implementation is counting an edge once per shared
//! field instead of once total.  A fixture where fields never co-agree would
//! pass a broken implementation.

use gigi::bundle::BundleStore;
use gigi::types::{BasePoint, BundleSchema, FieldDef, Record, Value};
use std::collections::HashSet;
use std::time::Instant;

// ─── fixture construction ─────────────────────────────────────────────────

/// Deterministic LCG — no rand dependency, reproducible per seed.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn upto(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }
}

/// A bundle with `n` records, `nf` indexed categorical fields of the given
/// cardinalities, and a `missing_rate` fraction of records that omit a field
/// entirely (exercising the NO_GROUP path).
fn make_store(n: usize, cards: &[u64], missing_pct: u64, seed: u64) -> BundleStore {
    let mut schema = BundleSchema::new("t");
    schema.base_fields.push(FieldDef::numeric("id"));
    for k in 0..cards.len() {
        let name = format!("f{k}");
        schema.fiber_fields.push(FieldDef::categorical(&name));
        schema.indexed_fields.push(name);
    }
    let mut store = BundleStore::new(schema);
    let mut rng = Lcg(seed);
    for i in 0..n {
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Integer(i as i64));
        for (k, &c) in cards.iter().enumerate() {
            if missing_pct > 0 && rng.upto(100) < missing_pct {
                continue;
            }
            // Correlated draw: with probability 1/3 the field copies f0's
            // bucket, so subsets genuinely co-agree and every
            // inclusion–exclusion term is exercised.
            let v = if k > 0 && rng.upto(3) == 0 {
                rng.upto(cards[0].min(c))
            } else {
                rng.upto(c)
            };
            rec.insert(format!("f{k}"), Value::Text(format!("v{v}")));
        }
        store.insert(&rec);
    }
    store
}

/// Brute-force O(N²) reference: the Def 3.9 graph, built by comparing every
/// pair of records directly on their field values.  Shares no code with
/// either the implicit operator or `field_index_graph`.
fn brute_force(store: &BundleStore) -> (Vec<(BasePoint, usize)>, usize) {
    // Column positions of the indexed fields inside a section row.
    let cols: Vec<usize> = store
        .schema
        .indexed_fields
        .iter()
        .filter_map(|f| {
            store
                .schema
                .fiber_fields
                .iter()
                .position(|fd| &fd.name == f)
        })
        .collect();

    // A record "has" an indexed field iff its cell is non-Null — the same
    // condition under which the engine puts it in a bucket.
    let mut rows: Vec<(BasePoint, Vec<Value>)> = store
        .sections()
        .map(|(bp, vals)| {
            (
                bp,
                cols.iter()
                    .map(|&c| vals.get(c).cloned().unwrap_or(Value::Null))
                    .collect(),
            )
        })
        .filter(|(_, v): &(BasePoint, Vec<Value>)| v.iter().any(|x| !matches!(x, Value::Null)))
        .collect();
    rows.sort_by_key(|(bp, _)| *bp);

    let mut deg: Vec<(BasePoint, usize)> = Vec::with_capacity(rows.len());
    let mut edges = 0usize;
    for (i, (p, rp)) in rows.iter().enumerate() {
        let mut d = 0usize;
        for (j, (_, rq)) in rows.iter().enumerate() {
            if i == j {
                continue;
            }
            let shares = rp.iter().zip(rq.iter()).any(|(a, b)| {
                !matches!(a, Value::Null) && !matches!(b, Value::Null) && a == b
            });
            if shares {
                d += 1;
                if i < j {
                    edges += 1;
                }
            }
        }
        deg.push((*p, d));
    }
    let _ = HashSet::<BasePoint>::new(); // keep the import honest if unused
    (deg, edges)
}

// ─── SBF-1 · edge count + degrees exact ───────────────────────────────────

#[test]
fn sbf1_edges_and_degrees_match_brute_force() {
    let cases: &[(usize, &[u64], u64)] = &[
        (60, &[4], 0),
        (80, &[4, 3], 0),
        (80, &[4, 3], 20),
        (70, &[5, 4, 3], 0),
        (70, &[5, 4, 3], 25),
        (90, &[2, 2], 0),        // huge overlapping buckets
        (50, &[50], 0),          // all singletons -> empty graph
        (64, &[1, 1], 0),        // one giant bucket twice over: max overlap
    ];
    for (ci, (n, cards, miss)) in cases.iter().enumerate() {
        for seed in 0..25u64 {
            let store = make_store(*n, cards, *miss, seed * 7919 + ci as u64);
            let (bdeg, bedges) = brute_force(&store);

            let g = gigi::spectral::test_hooks::implicit_graph(&store)
                .expect("indexed fields present");
            let ideg = gigi::spectral::test_hooks::degrees(&g);
            let iedges = gigi::spectral::test_hooks::edge_count(&g);
            let ibps = gigi::spectral::test_hooks::vertices(&g);

            assert_eq!(
                ibps.len(),
                bdeg.len(),
                "case {ci} seed {seed}: vertex count implicit {} vs brute {}",
                ibps.len(),
                bdeg.len()
            );
            assert_eq!(
                iedges, bedges,
                "case {ci} seed {seed}: |E| implicit {iedges} vs brute {bedges}"
            );
            for (k, (bp, d)) in bdeg.iter().enumerate() {
                assert_eq!(ibps[k], *bp, "case {ci} seed {seed}: vertex order");
                assert_eq!(
                    ideg[k] as usize, *d,
                    "case {ci} seed {seed}: deg({bp}) implicit {} vs brute {d}",
                    ideg[k]
                );
            }
        }
    }
}

// ─── SBF-2 · λ₁ parity ────────────────────────────────────────────────────

#[test]
fn sbf2_lambda1_matches_explicit_path() {
    let cases: &[(usize, &[u64], u64)] = &[
        (60, &[3], 0),
        (80, &[4, 3], 0),
        (80, &[4, 3], 15),
        (70, &[5, 4, 3], 0),
        (90, &[3, 3], 20),
    ];
    let mut compared = 0;
    for (ci, (n, cards, miss)) in cases.iter().enumerate() {
        for seed in 0..12u64 {
            let store = make_store(*n, cards, *miss, seed * 104_729 + ci as u64);
            let implicit = gigi::spectral::spectral_gap(&store);
            let explicit = gigi::spectral::test_hooks::spectral_gap_explicit(&store);
            assert!(
                (implicit - explicit).abs() <= 1e-9,
                "case {ci} seed {seed}: λ₁ implicit {implicit} vs explicit {explicit}"
            );
            compared += 1;
        }
    }
    assert!(compared >= 50, "expected a real sweep, compared {compared}");
}

// ─── SBF-3 · shortcut parity ──────────────────────────────────────────────

#[test]
fn sbf3_disconnected_returns_zero() {
    // Every record its own bucket value ⇒ no edges ⇒ many components.
    let store = make_store(40, &[40], 0, 11);
    assert_eq!(gigi::spectral::spectral_gap(&store), 0.0);
}

#[test]
fn sbf3_perfect_clique_uses_analytic_formula() {
    // One bucket holding everything ⇒ K_n ⇒ λ₁ = n/(n−1).
    let store = make_store(30, &[1], 0, 12);
    let n = 30.0;
    let got = gigi::spectral::spectral_gap(&store);
    assert!(
        (got - n / (n - 1.0)).abs() < 1e-12,
        "clique λ₁ = {got}, want {}",
        n / (n - 1.0)
    );
}

// ─── SBF-4 · Betti parity ─────────────────────────────────────────────────

#[test]
fn sbf4_betti_matches_brute_force() {
    let cases: &[(usize, &[u64], u64)] = &[
        (60, &[4], 0),
        (80, &[4, 3], 0),
        (80, &[4, 3], 20),
        (70, &[5, 4, 3], 15),
        (64, &[1, 1], 0),
    ];
    for (ci, (n, cards, miss)) in cases.iter().enumerate() {
        for seed in 0..10u64 {
            let store = make_store(*n, cards, *miss, seed * 31 + ci as u64);
            let (_, bedges) = brute_force(&store);
            let (b0, b1) = gigi::spectral::betti_numbers(&store);
            // β₁ = |E| − |V| + β₀, with |V| the full record count (as the
            // production formula uses store.len()).
            let want_b1 = (bedges + b0).saturating_sub(store.len());
            assert_eq!(
                b1, want_b1,
                "case {ci} seed {seed}: β₁ {b1} vs {want_b1} (|E|={bedges}, β₀={b0})"
            );
        }
    }
}

// ─── SBF-5 · scale ────────────────────────────────────────────────────────

#[test]
fn sbf5_fifty_thousand_records_is_fast() {
    // The shape that took >180 s and multi-GB on the explicit path:
    // low-cardinality indexed fields ⇒ giant near-cliques.
    let store = make_store(50_000, &[4, 2], 0, 4242);

    let t0 = Instant::now();
    let (b0, b1) = gigi::spectral::betti_numbers(&store);
    let betti_ms = t0.elapsed().as_millis();

    let t1 = Instant::now();
    let lambda1 = gigi::spectral::spectral_gap(&store);
    let spectral_ms = t1.elapsed().as_millis();

    println!(
        "SBF-5: N=50000 fields=[4,2] · BETTI {betti_ms} ms (β₀={b0}, β₁={b1}) \
         · SPECTRAL {spectral_ms} ms (λ₁={lambda1})"
    );
    assert!(betti_ms < 1_000, "BETTI took {betti_ms} ms, budget 1000");
    assert!(
        spectral_ms < 15_000,
        "SPECTRAL took {spectral_ms} ms, budget 15000"
    );
    assert!(lambda1.is_finite(), "λ₁ not finite");
    // 781M edges is the whole point: the explicit path would have had to
    // materialise every one of them.
    assert!(
        b1 > 500_000_000,
        "expected a near-clique edge count, got β₁={b1}"
    );
}

/// SBF-5b — the eigensolver itself at scale.
///
/// `sbf5` above lands on a bundle the engine reports as disconnected, so
/// `spectral_gap` returns 0.0 through the components shortcut and never
/// exercises the power iteration.  (That disconnection is NOT a property of
/// the data — an independent union-find says the graph is connected; it is
/// the low-32-bit base-point truncation at src/bundle.rs:1386 dropping one
/// record from the index once N passes the 32-bit birthday bound.  Tracked
/// separately; it is pre-existing and unrelated to TDD-SBF.)  This gate uses
/// a size below that threshold so the mat-vec actually runs, and asserts a
/// non-degenerate λ₁ so it can never silently become a shortcut test.
#[test]
fn sbf5b_eigensolver_runs_at_scale() {
    let store = make_store(20_000, &[6, 3], 0, 1337);
    let (b0, b1) = gigi::spectral::betti_numbers(&store);
    assert_eq!(b0, 1, "fixture must be connected for this gate to mean anything");

    let t = Instant::now();
    let lambda1 = gigi::spectral::spectral_gap(&store);
    let ms = t.elapsed().as_millis();
    println!(
        "SBF-5b: N=20000 fields=[6,3] connected · SPECTRAL {ms} ms \
         (λ₁={lambda1}, |E|≈{b1})"
    );
    assert!(
        lambda1 > 0.0 && lambda1.is_finite(),
        "λ₁={lambda1} — shortcut fired, eigensolver not exercised"
    );
    assert!(ms < 15_000, "SPECTRAL took {ms} ms, budget 15000");
}
