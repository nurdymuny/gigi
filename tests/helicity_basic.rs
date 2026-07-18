//! HELICITY — discrete Chern-Simons / fluid-helicity observable
//! (Navier-Stokes Tier 1, Ask 1). RED phase (2026-07-17).
//!
//! Fluid helicity H = ∫ u·ω dV (ω = ∇×u) is the central topological
//! invariant of a flow — conserved by Euler, measuring vortex-line
//! linking (Moffatt). With the velocity read as a connection 1-form A
//! (one signed real per edge), H = ∫ A∧dA = the discrete Chern-Simons
//! functional. These tests pin the contract Hallie handed us:
//!
//!   HELICITY <bundle> ON FIBER (<a_field>) [ON LATTICE <l>] [DENSITY];
//!
//! The bundle is a periodic cubic L³ edge-endpoint store (vertex_a,
//! vertex_b, a_e), FORWARD edges only (3·L³ of them), with the site
//! convention vid(i,j,k) = (i·L + j)·L + k. The verb returns ONE row:
//!   { helicity (f64), n_edges_used (int), n_cells (int),
//!     mode_used = "chern_simons" }
//! and, with DENSITY, the per-cell helicity density Vector (length
//! n_cells, summing to helicity).
//!
//! Pinned discrete formula (co-located plaquette circulations, periodic
//! wrap on s+ê):
//!   H = Σ_s [ A_x(s)·Ω_x(s) + A_y(s)·Ω_y(s) + A_z(s)·Ω_z(s) ]
//!   Ω_x(s) = A_y(s) + A_z(s+ŷ) − A_y(s+ẑ) − A_z(s)   # yz-plaquette
//!   Ω_y(s) = A_z(s) + A_x(s+ẑ) − A_z(s+x̂) − A_x(s)   # zx-plaquette
//!   Ω_z(s) = A_x(s) + A_y(s+x̂) − A_x(s+ŷ) − A_y(s)   # xy-plaquette
//!
//! These fail on the pre-ship tree: `HELICITY …` does not parse.
//!
//! Run: `cargo test --features halcyon --test helicity_basic`

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::f64::consts::PI;

// ── Site / edge fixtures (Hallie's vid convention) ───────────────────

/// vid(i,j,k) = (i·L + j)·L + k — i is x (slowest, stride L²), j is y
/// (stride L), k is z (fastest, stride 1).
fn vid(i: i64, j: i64, k: i64, l: i64) -> i64 {
    (i * l + j) * l + k
}

fn make_edge_bundle(engine: &mut Engine, name: &str) {
    let schema = BundleSchema::new(name)
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("a_e"));
    engine.create_bundle(schema).expect("create_bundle");
}

/// One forward edge-endpoint record (vertex_a = tail vid, vertex_b =
/// head vid, a_e = the signed velocity 1-form on that edge).
fn edge(va: i64, vb: i64, ae: f64) -> Record {
    let mut r = Record::new();
    r.insert("vertex_a".to_string(), Value::Integer(va));
    r.insert("vertex_b".to_string(), Value::Integer(vb));
    r.insert("a_e".to_string(), Value::Float(ae));
    r
}

/// Emit a full periodic cubic L³ forward-edge bundle: for each site
/// (i,j,k) three edges (+x,+y,+z) whose fibers come from `a_of(i,j,k)`
/// = (a_x, a_y, a_z) — the per-site edge values (already including any
/// h-factor the field carries).
fn build_cubic_bundle(
    engine: &mut Engine,
    name: &str,
    l: i64,
    a_of: impl Fn(i64, i64, i64) -> (f64, f64, f64),
) {
    make_edge_bundle(engine, name);
    let mut recs: Vec<Record> = Vec::with_capacity((3 * l * l * l) as usize);
    for i in 0..l {
        for j in 0..l {
            for k in 0..l {
                let s = vid(i, j, k, l);
                let (ax, ay, az) = a_of(i, j, k);
                recs.push(edge(s, vid((i + 1) % l, j, k, l), ax)); // +x
                recs.push(edge(s, vid(i, (j + 1) % l, k, l), ay)); // +y
                recs.push(edge(s, vid(i, j, (k + 1) % l, l), az)); // +z
            }
        }
    }
    engine
        .batch_insert(name, &recs)
        .expect("batch_insert cubic bundle");
}

// ── Analytic edge fields ─────────────────────────────────────────────

/// N1 ABC Beltrami (A=B=C=1): u = (sin z + cos y, sin x + cos z,
/// sin y + cos x); edge fiber a_e = u_d(site)·h, h = 2π/L. ∇×u = u, so
/// helicity density u·ω = |u|², and the analytic continuum helicity is
/// ∫|u|² dV = 24π³ ≈ 744.1506.
fn abc(l: i64) -> impl Fn(i64, i64, i64) -> (f64, f64, f64) {
    let h = 2.0 * PI / (l as f64);
    move |i, j, k| {
        let (x, y, z) = (i as f64 * h, j as f64 * h, k as f64 * h);
        let ux = z.sin() + y.cos(); // +x edge
        let uy = x.sin() + z.cos(); // +y edge
        let uz = y.sin() + x.cos(); // +z edge
        (ux * h, uy * h, uz * h)
    }
}

/// Right-handed helical mode: A = (0, sin x, cos x)·h ⇒ ∇×A = +A
/// (positive Beltrami / right-handed / positively linked) ⇒ H > 0,
/// exactly +4π²·L·sin(2π/L).
fn helical_right(l: i64) -> impl Fn(i64, i64, i64) -> (f64, f64, f64) {
    let h = 2.0 * PI / (l as f64);
    move |i, _j, _k| {
        let x = i as f64 * h;
        (0.0, x.sin() * h, x.cos() * h)
    }
}

/// Left-handed helical mode: A = (0, cos x, sin x)·h ⇒ ∇×A = −A
/// (negative Beltrami / left-handed / negatively linked) ⇒ H < 0, the
/// exact mirror image of the right-handed value.
fn helical_left(l: i64) -> impl Fn(i64, i64, i64) -> (f64, f64, f64) {
    let h = 2.0 * PI / (l as f64);
    move |i, _j, _k| {
        let x = i as f64 * h;
        (0.0, x.cos() * h, x.sin() * h)
    }
}

/// Unlinked / non-helical: A = (0, cos x, 0)·h ⇒ A·(∇×A) = 0 ⇒ H = 0.
fn unlinked(l: i64) -> impl Fn(i64, i64, i64) -> (f64, f64, f64) {
    let h = 2.0 * PI / (l as f64);
    move |i, _j, _k| {
        let x = i as f64 * h;
        (0.0, x.cos() * h, 0.0)
    }
}

/// Exact discrete ABC helicity: H = 12π²·L·sin(2π/L) → 24π³ as L→∞.
fn abc_expected(l: i64) -> f64 {
    12.0 * PI * PI * (l as f64) * (2.0 * PI / (l as f64)).sin()
}

/// Exact discrete single-helical-mode magnitude: 4π²·L·sin(2π/L).
fn helical_expected(l: i64) -> f64 {
    4.0 * PI * PI * (l as f64) * (2.0 * PI / (l as f64)).sin()
}

// ── Run + accessors ──────────────────────────────────────────────────

fn run_helicity(engine: &mut Engine, q: &str) -> Vec<Record> {
    let stmt = parse(q).unwrap_or_else(|e| panic!("HELICITY must parse: {q}\n  err: {e}"));
    match execute(engine, &stmt)
        .unwrap_or_else(|e| panic!("HELICITY must execute: {q}\n  err: {e}"))
    {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows envelope, got {other:?}"),
    }
}

fn hel(row: &Record) -> f64 {
    match row.get("helicity") {
        Some(Value::Float(f)) => *f,
        other => panic!("expected helicity Float, got {other:?}"),
    }
}
fn n_edges(row: &Record) -> i64 {
    match row.get("n_edges_used") {
        Some(Value::Integer(n)) => *n,
        other => panic!("expected n_edges_used Integer, got {other:?}"),
    }
}
fn n_cells(row: &Record) -> i64 {
    match row.get("n_cells") {
        Some(Value::Integer(n)) => *n,
        other => panic!("expected n_cells Integer, got {other:?}"),
    }
}
fn mode(row: &Record) -> String {
    match row.get("mode_used") {
        Some(Value::Text(t)) => t.clone(),
        other => panic!("expected mode_used Text, got {other:?}"),
    }
}
fn density(row: &Record) -> Vec<f64> {
    match row.get("density") {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("expected density Vector, got {other:?}"),
    }
}

// ══════════════════════════════════════════════════════════════════════
// N1 — ABC Beltrami golden numbers (the headline)
// ══════════════════════════════════════════════════════════════════════

/// L=16 → 725.171 (ratio 0.9745 of the 24π³ continuum), full envelope.
#[test]
fn n1_abc_beltrami_l16() {
    let mut engine = Engine::open_memory().expect("memory engine");
    build_cubic_bundle(&mut engine, "abc16", 16, abc(16));
    let rows = run_helicity(&mut engine, "HELICITY abc16 ON FIBER (a_e);");
    assert_eq!(rows.len(), 1, "single summary row");
    let row = &rows[0];
    let h = hel(row);
    println!(
        "[HELICITY] ABC L=16 H={h:.6} ratio={:.6} (golden 725.171)",
        h / (24.0 * PI.powi(3))
    );
    assert!(
        (h - abc_expected(16)).abs() < 1e-3,
        "L=16 helicity {h} vs exact discrete {}",
        abc_expected(16)
    );
    assert!(
        (h - 725.171).abs() < 1e-2,
        "L=16 helicity {h} must reproduce Hallie's golden 725.171"
    );
    // ratio to the 24π³ continuum
    let ratio = h / (24.0 * PI.powi(3));
    assert!((ratio - 0.9745).abs() < 1e-3, "L=16 ratio {ratio} vs 0.9745");
    // envelope
    assert_eq!(mode(row), "chern_simons");
    assert_eq!(n_cells(row), 4096, "n_cells = L³ = 16³");
    assert_eq!(n_edges(row), 12288, "n_edges = 3·L³ = 3·4096");
}

/// L=24 → 735.679 (ratio 0.9886).
#[test]
fn n1_abc_beltrami_l24() {
    let mut engine = Engine::open_memory().expect("memory engine");
    build_cubic_bundle(&mut engine, "abc24", 24, abc(24));
    let rows = run_helicity(&mut engine, "HELICITY abc24 ON FIBER (a_e);");
    let h = hel(&rows[0]);
    println!("[HELICITY] ABC L=24 H={h:.6} ratio={:.6} (golden 735.679)", h / (24.0 * PI.powi(3)));
    assert!((h - abc_expected(24)).abs() < 1e-3, "L=24 {h}");
    assert!((h - 735.679).abs() < 1e-2, "L=24 golden 735.679, got {h}");
    assert_eq!(n_cells(&rows[0]), 24 * 24 * 24);
    assert_eq!(n_edges(&rows[0]), 3 * 24 * 24 * 24);
}

/// L=32 → 739.378 (ratio 0.9936).
#[test]
fn n1_abc_beltrami_l32() {
    let mut engine = Engine::open_memory().expect("memory engine");
    build_cubic_bundle(&mut engine, "abc32", 32, abc(32));
    let rows = run_helicity(&mut engine, "HELICITY abc32 ON FIBER (a_e);");
    let h = hel(&rows[0]);
    println!("[HELICITY] ABC L=32 H={h:.6} ratio={:.6} (golden 739.378)", h / (24.0 * PI.powi(3)));
    assert!((h - abc_expected(32)).abs() < 1e-3, "L=32 {h}");
    assert!((h - 739.378).abs() < 1e-2, "L=32 golden 739.378, got {h}");
    assert_eq!(n_cells(&rows[0]), 32 * 32 * 32);
}

/// L=48 → 742.027 (ratio 0.9971). Optional/release gate — heavier
/// (331776 edges); run with `--ignored`.
#[test]
#[ignore = "L=48 release gate — 331776 edges, slow in debug"]
fn n1_abc_beltrami_l48() {
    let mut engine = Engine::open_memory().expect("memory engine");
    build_cubic_bundle(&mut engine, "abc48", 48, abc(48));
    let rows = run_helicity(&mut engine, "HELICITY abc48 ON FIBER (a_e);");
    let h = hel(&rows[0]);
    assert!((h - abc_expected(48)).abs() < 1e-2, "L=48 {h}");
    assert!((h - 742.027).abs() < 1e-2, "L=48 golden 742.027, got {h}");
}

// ══════════════════════════════════════════════════════════════════════
// N2 — chirality / linked-tube sign pin (load-bearing)
//
// Two vortex tubes with linking number n ⇒ H = 2·n·κ₁·κ₂ (Moffatt): a
// right-handed / positively-linked configuration has H > 0, its mirror
// (left-handed, linking number negated) has H < 0, and an unlinked
// configuration has H = 0. ABC (N1) passes even with a flipped Ω sign
// by its (x,y,z) symmetry — the chiral modes below do NOT: a flipped
// Ω orientation flips the sign here. Exact discrete values pin it.
// ══════════════════════════════════════════════════════════════════════

/// Right-handed helical mode (∇×A = +A, positive linking) ⇒ H = +4π²L
/// sin(2π/L) > 0. Exact at L=4: +16π².
#[test]
fn n2_right_handed_positive_helicity() {
    let mut engine = Engine::open_memory().expect("memory engine");
    build_cubic_bundle(&mut engine, "rh4", 4, helical_right(4));
    let rows = run_helicity(&mut engine, "HELICITY rh4 ON FIBER (a_e);");
    let h = hel(&rows[0]);
    assert!(h > 0.0, "right-handed / positively-linked ⇒ H > 0, got {h}");
    assert!(
        (h - helical_expected(4)).abs() < 1e-9,
        "H = +4π²·L·sin(2π/L) = {}, got {h}",
        helical_expected(4)
    );
    // L=4: sin(π/2)=1 ⇒ exactly 16π²
    assert!((h - 16.0 * PI * PI).abs() < 1e-9, "L=4 ⇒ +16π², got {h}");
}

/// Left-handed helical mode (∇×A = −A, linking number negated) ⇒ the
/// EXACT mirror of the right-handed value: H = −4π²L sin(2π/L) < 0.
/// This is the sign pin: a flipped Ω convention would swap these.
#[test]
fn n2_left_handed_mirror_negative() {
    let mut engine = Engine::open_memory().expect("memory engine");
    build_cubic_bundle(&mut engine, "rh4", 4, helical_right(4));
    build_cubic_bundle(&mut engine, "lh4", 4, helical_left(4));
    let hr = hel(&run_helicity(&mut engine, "HELICITY rh4 ON FIBER (a_e);")[0]);
    let hl = hel(&run_helicity(&mut engine, "HELICITY lh4 ON FIBER (a_e);")[0]);
    assert!(hl < 0.0, "left-handed / negatively-linked ⇒ H < 0, got {hl}");
    assert!(
        (hl + helical_expected(4)).abs() < 1e-9,
        "H_left = −4π²·L·sin(2π/L), got {hl}"
    );
    // exact mirror antisymmetry: H_right + H_left = 0
    assert!(
        (hr + hl).abs() < 1e-9,
        "mirror pair must cancel exactly: {hr} + {hl}"
    );
}

/// Unlinked (non-helical) field ⇒ H = 0 exactly, despite nonzero A.
#[test]
fn n2_unlinked_zero() {
    let mut engine = Engine::open_memory().expect("memory engine");
    build_cubic_bundle(&mut engine, "ul4", 4, unlinked(4));
    let h = hel(&run_helicity(&mut engine, "HELICITY ul4 ON FIBER (a_e);")[0]);
    assert!(h.abs() < 1e-12, "unlinked (n=0) ⇒ H = 0, got {h}");
}

// ══════════════════════════════════════════════════════════════════════
// N3 — zero field ⇒ H = 0 exactly
// ══════════════════════════════════════════════════════════════════════

#[test]
fn n3_zero_field_exact_zero() {
    let mut engine = Engine::open_memory().expect("memory engine");
    build_cubic_bundle(&mut engine, "z4", 4, |_i, _j, _k| (0.0, 0.0, 0.0));
    let rows = run_helicity(&mut engine, "HELICITY z4 ON FIBER (a_e);");
    assert!(hel(&rows[0]).abs() < 1e-15, "zero field ⇒ exactly 0");
    assert_eq!(n_cells(&rows[0]), 64);
    assert_eq!(n_edges(&rows[0]), 192);
}

// ══════════════════════════════════════════════════════════════════════
// N4 — non-cubic / non-3D ⇒ typed error
// ══════════════════════════════════════════════════════════════════════

/// max_vid+1 not a perfect cube ⇒ typed error (⌊∛V⌉³ ≠ V).
#[test]
fn n4_non_cubic_v_typed_error() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_edge_bundle(&mut engine, "nc");
    // vertices 0..=5 ⇒ V = 6, not a cube.
    engine
        .batch_insert("nc", &[edge(0, 1, 1.0), edge(2, 3, 1.0), edge(4, 5, 1.0)])
        .expect("batch_insert");
    let stmt = parse("HELICITY nc ON FIBER (a_e);").expect("must parse");
    let err = execute(&mut engine, &stmt).expect_err("non-cubic V must error");
    assert!(
        err.to_lowercase().contains("cubic") || err.contains("∛") || err.contains("V ="),
        "non-cubic error must name the cubic-lattice requirement: {err}"
    );
}

/// A cubic V but a partial (2D-like) bundle missing the +z edges ⇒
/// typed error (incomplete forward-edge bundle).
#[test]
fn n4_partial_2d_bundle_typed_error() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_edge_bundle(&mut engine, "flat");
    // L=2 cubic (V=8) but only +x and +y edges (no +z) — a 2D-ish slab.
    let l = 2i64;
    let mut recs = Vec::new();
    for i in 0..l {
        for j in 0..l {
            for k in 0..l {
                let s = vid(i, j, k, l);
                recs.push(edge(s, vid((i + 1) % l, j, k, l), 1.0));
                recs.push(edge(s, vid(i, (j + 1) % l, k, l), 1.0));
            }
        }
    }
    engine.batch_insert("flat", &recs).expect("batch_insert");
    let stmt = parse("HELICITY flat ON FIBER (a_e);").expect("must parse");
    let err = execute(&mut engine, &stmt).expect_err("2D/partial bundle must error");
    assert!(!err.is_empty(), "partial bundle must error loudly: {err}");
}

/// An edge that is not a unit forward step along one axis ⇒ typed error.
#[test]
fn n4_non_unit_edge_typed_error() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_edge_bundle(&mut engine, "bad");
    // L=2 cubic V=8, but a stray diagonal edge (0 → 7) that is not a
    // unit step, plus enough structure to reach the fill loop.
    let l = 2i64;
    let mut recs = Vec::new();
    for i in 0..l {
        for j in 0..l {
            for k in 0..l {
                let s = vid(i, j, k, l);
                recs.push(edge(s, vid((i + 1) % l, j, k, l), 1.0));
                recs.push(edge(s, vid(i, (j + 1) % l, k, l), 1.0));
                recs.push(edge(s, vid(i, j, (k + 1) % l, l), 1.0));
            }
        }
    }
    // corrupt one z-edge at site 0 into a diagonal 0→7
    recs[2] = edge(0, 7, 1.0);
    engine.batch_insert("bad", &recs).expect("batch_insert");
    let stmt = parse("HELICITY bad ON FIBER (a_e);").expect("must parse");
    let err = execute(&mut engine, &stmt).expect_err("non-unit edge must error");
    assert!(!err.is_empty(), "non-unit edge must error loudly: {err}");
}

// ══════════════════════════════════════════════════════════════════════
// N5 — DENSITY: per-cell field sums to the scalar; length == n_cells
// ══════════════════════════════════════════════════════════════════════

#[test]
fn n5_density_sums_to_scalar() {
    let mut engine = Engine::open_memory().expect("memory engine");
    build_cubic_bundle(&mut engine, "abc8", 8, abc(8));
    let scalar = hel(&run_helicity(&mut engine, "HELICITY abc8 ON FIBER (a_e);")[0]);
    let rows = run_helicity(&mut engine, "HELICITY abc8 ON FIBER (a_e) DENSITY;");
    let dens = density(&rows[0]);
    assert_eq!(dens.len(), 8 * 8 * 8, "density length == n_cells = L³");
    assert_eq!(dens.len() as i64, n_cells(&rows[0]));
    let sum: f64 = dens.iter().sum();
    assert!(
        (sum - scalar).abs() < 1e-9,
        "per-cell density must sum to helicity: Σ={sum} vs H={scalar}"
    );
    // scalar path (no DENSITY) must equal the DENSITY-path helicity
    assert!((hel(&rows[0]) - scalar).abs() < 1e-9, "DENSITY must not change H");
}

// ══════════════════════════════════════════════════════════════════════
// N6 — sign under field negation: A → −A ⇒ H → +H (quadratic in A)
// ══════════════════════════════════════════════════════════════════════

#[test]
fn n6_negation_invariance() {
    let mut engine = Engine::open_memory().expect("memory engine");
    build_cubic_bundle(&mut engine, "abc8", 8, abc(8));
    let abc_neg = {
        let base = abc(8);
        move |i: i64, j: i64, k: i64| {
            let (x, y, z) = base(i, j, k);
            (-x, -y, -z)
        }
    };
    build_cubic_bundle(&mut engine, "abc8n", 8, abc_neg);
    let h = hel(&run_helicity(&mut engine, "HELICITY abc8 ON FIBER (a_e);")[0]);
    let hn = hel(&run_helicity(&mut engine, "HELICITY abc8n ON FIBER (a_e);")[0]);
    assert!(
        (h - hn).abs() < 1e-9,
        "helicity is quadratic in A: H(−A) = H(A); {h} vs {hn}"
    );
    assert!(h > 0.0, "ABC helicity is positive; got {h}");
}
