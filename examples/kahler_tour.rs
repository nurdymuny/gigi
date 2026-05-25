//! kahler_tour.rs — walk every shipped layer of the Kähler upgrade.
//!
//! One self-contained example exercising L1-L9 of the Kähler
//! catalog plus the DHOOM-encoder array-of-primitives round-trip
//! and a summary of the PR-window HTTP endpoints.
//!
//! Build + run:
//!     cargo run --release --features kahler --example kahler_tour
//!
//! Each section prints what it built, what it computed, and the
//! catalog reference. Output is meant to be read top-to-bottom as
//! a guided tour.

#![cfg(feature = "kahler")]

use gigi::bundle::BundleStore;
use gigi::curvature::{compute_kahler_decomposition, holonomy_debt, HolonomyDebt};
use gigi::dhoom;
use gigi::discrete::hodge_complex::HodgeComplex;
use gigi::discrete::hodge_laplacian::betti;
use gigi::discrete::morse::morse_compress;
use gigi::geometry::{
    flat_transport, BSource, ClosedTwoForm, CohClass, ComplexStructure,
    HadamardSubstructure, InfinitesimalAction, KahlerStructure, LineBundle, MomentMap,
    QuantumCohomology, TransportSegment, TwoForm,
};
use gigi::graph::adjacency::{AuxiliaryAdjacency, PrincipalAdjacency, SparseAdjacency};
use gigi::graph::commutativity::commute;
use gigi::cost::jacobi_estimator::jacobi_field;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use serde_json::json;

// ── tiny output helpers ───────────────────────────────────────────

fn header(n: &str, title: &str) {
    println!();
    println!("══ {} {} ══", n, title);
}

fn line(label: &str, value: impl std::fmt::Display) {
    println!("  {:<36} {}", label, value);
}

fn note(s: &str) {
    println!("  {}", s);
}

// ── L1 — Complex structure J and closed 2-form B ──────────────────

fn l1_complex_structure_and_two_form() {
    header("L1", "complex structure J + closed 2-form B (catalog §1.2)");

    // J² = -I in 2D — the standard complex structure on ℝ².
    let j = ComplexStructure::standard(1); // complex dim = 1, real dim = 2
    line("J real dim", j.dim());
    line("J² = -I (machine-checked at constructor)", "OK");

    // A closed antisymmetric 2-form B (the magnetic bias).
    let b_raw = TwoForm::new(vec![0.0, 1.5, -1.5, 0.0], 2).unwrap();
    let b = ClosedTwoForm::new_constant(b_raw);
    line("B(e_0, e_1)", b.apply(&[1.0, 0.0], &[0.0, 1.0]));
    line("‖B‖_F", b.form().frobenius_norm());

    // Bundle them as a KahlerStructure for downstream layers.
    let kahler = KahlerStructure::new(j, b);
    note(&format!(
        "Bundled (J, B) into a KahlerStructure — dim = {}",
        kahler.j.dim()
    ));
}

// ── L1.5 — Magnetic geodesic transport (catalog §1.5) ─────────────

fn l1_5_flat_transport_classical_and_magnetic() {
    header("L1.5", "flat_transport: classical vs magnetic (catalog §1.5)");

    // Classical (B = 0) transport: straight-line geodesic.
    let seg = TransportSegment::new(
        vec![0.0, 0.0],   // start position
        vec![1.0, 0.0],   // end position
        vec![1.0, 0.0],   // initial vector to parallel-transport
    )
    .unwrap();
    let r_classical = flat_transport(&seg, None, 1e-3, 10, BSource::None).unwrap();
    line("classical: used_magnetic", r_classical.used_magnetic);
    line("classical: b_source", format!("{:?}", r_classical.b_source));
    line(
        "classical: holonomy_norm",
        format!("{:.2e}", r_classical.holonomy_norm),
    );

    // Magnetic transport: same segment, B = 0.5 dx ∧ dy.
    let bias_raw = TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).unwrap();
    let bias = ClosedTwoForm::new_constant(bias_raw);
    let r_mag =
        flat_transport(&seg, Some(&bias), 1e-3, 10, BSource::Override).unwrap();
    line("magnetic: used_magnetic", r_mag.used_magnetic);
    line("magnetic: b_source", format!("{:?}", r_mag.b_source));
    note("(Same segment, different bias B → different transport outcome.)");
}

// ── L2 — Dual adjacency commutativity (catalog §1.1) ──────────────

fn l2_dual_adjacency_commutativity() {
    header("L2", "principal × auxiliary adjacency commutator (catalog §1.1)");

    // Abelian case: Cayley graph of ℤ/4 × ℤ/4 with axial generators.
    // Both adjacencies live in ℂ[G] which is commutative → commute exactly.
    let mut p_edges = vec![];
    let mut a_edges = vec![];
    let m = 4u64;
    for i in 0..m {
        for j in 0..m {
            let here = i * m + j;
            let right = i * m + (j + 1) % m;
            let down = ((i + 1) % m) * m + j;
            p_edges.push((here, right));
            a_edges.push((here, down));
        }
    }
    let p = PrincipalAdjacency::from_pairs(p_edges);
    let a = AuxiliaryAdjacency::from_pairs(a_edges);
    let verdict = commute(&p, &a);
    line("ℤ/4 × ℤ/4 axial generators", format!("{:?}", verdict));

    // Non-commuting case: two non-central transpositions in S_3.
    // Cayley(S_3, {(1 2)}) for principal, Cayley(S_3, {(1 3)}) for aux.
    // Neither generating set is central → operators don't commute.
    let s3_p = SparseAdjacency::from_pairs([
        (0, 1), (2, 4), (3, 5), // (1 2) action on permutations
    ]);
    let s3_a = SparseAdjacency::from_pairs([
        (0, 2), (1, 3), (4, 5), // (1 3) action
    ]);
    let verdict_nc = commute(
        &PrincipalAdjacency::new(s3_p),
        &AuxiliaryAdjacency::new(s3_a),
    );
    line("S_3 non-central transpositions", format!("{:?}", verdict_nc));
    note("(Adachi's discrete Kähler identity: commute ⇔ planner can reorder safely.)");
}

// ── L3 — Jacobi-field cardinality bounds (catalog §1.3) ───────────

fn l3_jacobi_cardinality_bounds() {
    header("L3", "Jacobi cardinality bounds: ℝ², ℍ², S² (catalog §1.3)");
    for (name, k) in [("R² (flat)", 0.0_f64), ("H² (K=-1)", -1.0), ("S² (K=+1)", 1.0)] {
        let r = jacobi_field(k, 2.0, 200);
        line(
            &format!("{:<10} J(t=2)", name),
            format!(
                "{:.6}  (first conjugate point: {:?})",
                r.values.last().unwrap(),
                r.first_conjugate_point.map(|t| format!("{:.4}", t)),
            ),
        );
    }
    note("J(t)=t (flat) ↔ J(t)=sinh(t) (H², monotone) ↔ J(t)=sin(t) (S², dies at π).");
}

// ── L4 — Kähler curvature decomposition (catalog §E.3) ────────────

fn l4_kahler_curvature_decomposition() {
    header("L4", "Kähler curvature on a bundle (catalog §E.3)");

    let kahler = KahlerStructure::new(
        ComplexStructure::standard(1),
        ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).unwrap(),
        ),
    );
    let schema = BundleSchema::new("kahler_demo")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(kahler.clone());
    let mut store = BundleStore::new(schema);
    for i in 0..40 {
        let theta = (i as f64) * 2.0 * std::f64::consts::PI / 40.0;
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Integer(i));
        rec.insert("x".into(), Value::Float(theta.cos()));
        rec.insert("y".into(), Value::Float(theta.sin()));
        store.insert(&rec);
    }
    let decomp = compute_kahler_decomposition(&store, &kahler).unwrap();
    line("Ricci (scalar trace)", format!("{:.4}", decomp.ricci));
    line("Weyl (conformal)", format!("{:.4}", decomp.weyl));
    line("holo-bisectional min/max",
         format!("[{:.4}, {:.4}]", decomp.holo_bisectional_min, decomp.holo_bisectional_max));
    line("holo-sectional K_H", format!("{:.4}", decomp.holo_sectional));
}

// ── L5 — Hadamard detection (catalog §1.4, §1.5) ──────────────────

fn l5_hadamard_detection() {
    header("L5", "Hadamard substructure detection (catalog §1.4)");

    // Build a flat-ish bundle: x, y near zero → K_H ≈ 0 → Hadamard.
    let kahler = KahlerStructure::new(
        ComplexStructure::standard(1),
        ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, 0.1, -0.1, 0.0], 2).unwrap(),
        ),
    );
    let schema = BundleSchema::new("hadamard_demo")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(10.0))
        .fiber(FieldDef::numeric("y").with_range(10.0))
        .with_kahler(kahler);
    let mut store = BundleStore::new(schema);
    for i in 0..50 {
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Integer(i));
        rec.insert("x".into(), Value::Float(0.01 * (i as f64)));
        rec.insert("y".into(), Value::Float(0.01 * (i as f64)));
        store.insert(&rec);
    }
    let regions: Vec<HadamardSubstructure> = gigi::geometry::detect_hadamard(&store);
    if regions.is_empty() {
        line("Hadamard regions detected", "(none — not in K_B ≤ threshold)");
    } else {
        for r in &regions {
            line("Hadamard region", format!("{:?}", r.region));
            line("  conjugate_free", r.conjugate_free);
            line("  K_B max", format!("{:.4}", r.kb_max));
        }
    }
}

// ── L6 — Hodge complex + Morse compression (catalog §2.9) ─────────

fn l6_hodge_complex_and_morse_compression() {
    header("L6", "Hodge complex + Morse compression (catalog §2.9)");

    // S² as boundary of a tetrahedron: 4 vertices, 6 edges, 4 faces.
    // Expected Betti: (1, 0, 1).
    let edges = vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let faces = vec![(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];
    let hc = HodgeComplex::new(4, edges, faces).expect("tetrahedron Hodge");
    let b = betti(&hc, 1e-8);
    line("Betti (b_0, b_1, b_2) of ∂Δ³ ≅ S²", format!("({}, {}, {})", b.b0, b.b1, b.b2));

    let morse = morse_compress(&hc);
    line(
        "Morse compression: critical / original",
        format!("{} / {}", morse.n_critical(), morse.n_original()),
    );
    line("compression ratio", format!("{:.2}×", morse.compression_ratio()));
    line("cohomology preserved", morse.cohomology_preserved());
}

// ── L7 — line bundle, holonomy debt, quantum cohomology ──────────

fn l7_line_bundle_holonomy_quantum_cohomology() {
    header("L7", "line bundle + holonomy_debt + QH* + Toeplitz (catalog §2.1, §2.10, §E.1)");

    // (a) Integer Chern class via Wu-Yang transition data
    // (∮ B = 2π · 3 → ChernClass(3)).
    let lb = LineBundle::from_transition_data(2.0 * std::f64::consts::PI * 3.0, 1e-6)
        .expect("integral");
    line("LineBundle Chern class", format!("c_1 = {}", lb.chern_class().0));

    // (b) Davis non-decoupling: quantized vs continuous holonomy debt.
    let schema = BundleSchema::new("kahler_loop")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(KahlerStructure::new(
            ComplexStructure::standard(1),
            ClosedTwoForm::new_constant(
                TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).unwrap(),
            ),
        ));
    let mut store = BundleStore::new(schema);
    for i in 0..10 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(0.0));
        r.insert("y".into(), Value::Float(0.0));
        store.insert(&r);
    }
    let q = holonomy_debt(&store, 2.0 * std::f64::consts::PI * 3.0, 1e-6).unwrap();
    let c = holonomy_debt(&store, 2.0 * std::f64::consts::PI * 0.7, 1e-6).unwrap();
    line("loop winding 3.0  → debt", match q {
        HolonomyDebt::Quantized(n) => format!("Quantized({})", n),
        HolonomyDebt::Continuous(x) => format!("Continuous({:.4})", x),
    });
    line("loop winding 0.7  → debt", match c {
        HolonomyDebt::Quantized(n) => format!("Quantized({})", n),
        HolonomyDebt::Continuous(x) => format!("Continuous({:.4})", x),
    });

    // (c) Quantum cohomology: H · H = H² on CP².
    let qh = QuantumCohomology::cpn(2);
    let h_sq = qh
        .compose(&CohClass::h_power(1), &CohClass::h_power(1))
        .unwrap();
    line("QH*(CP²): H · H terms (coeff, h, q)",
         format!("{:?}", h_sq.terms));

    // (d) Riemann-Roch capacity: dim H⁰(L^k) on Marcella's CP^191.
    let qh191 = QuantumCohomology::cpn(191);
    let cap = qh191.representational_capacity(1).unwrap();
    line("representational_capacity(CP^191, k=1)", cap);
    note("(= binomial(192, 191) = 192, the Hopf-substrate dim.)");

    // (e) Berezin-Toeplitz operator: T_{const f} on CP^2 at ℏ = 0.5
    // (allow below-safe-bound for demo).
    let top = gigi::geometry::toeplitz_operator(&qh, 1.7, 0.5, 32, true).unwrap();
    line(
        "Toeplitz T_{1.7} on CP²  dim / ℏ",
        format!("{} × {} / ℏ = {}", top.dim, top.dim, top.hbar),
    );
}

// ── L9 — Moment maps + Noether (catalog §2.3) ─────────────────────

fn l9_moment_map_noether() {
    header("L9", "moment map + Noether conservation (catalog §2.3)");

    // Canonical symplectic B on T*ℝ² (ordering x, y, p_x, p_y).
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(
            vec![
                0.0, 0.0, -1.0, 0.0,
                0.0, 0.0, 0.0, -1.0,
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
            ],
            4,
        )
        .unwrap(),
    );
    // SO(2) rotation in (x, y) plane (with matching (p_x, p_y) rotation
    // so the generator is B-symplectic).
    let rot = InfinitesimalAction::new(
        vec![
            0.0, -1.0, 0.0, 0.0,
            1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, -1.0,
            0.0, 0.0, 1.0, 0.0,
        ],
        4,
    )
    .unwrap();
    let mm = MomentMap::new(b, vec![rot], vec!["L_z".into()]).unwrap();

    // Symmetric Hamiltonian → angular momentum conserved.
    let h_sym = |s: &[f64]| 0.5 * (s[0] * s[0] + s[1] * s[1] + s[2] * s[2] + s[3] * s[3]);
    let v_sym = mm
        .measure_conservation(&h_sym, &[1.0, 0.0, 0.0, 1.0], 0.01, 1000, 0, None, None)
        .unwrap();
    line("μ(L_z) at start", mm.moment_value(&[1.0, 0.0, 0.0, 1.0], 0));
    line(
        "symmetric H: drift over t=10",
        format!("{:.3e}  (conserved: {})", v_sym.drift, v_sym.conserved),
    );

    // Asymmetric Hamiltonian → angular momentum drifts.
    let h_asym = |s: &[f64]| 0.5 * s[2] * s[2] + 0.5 * s[3] * s[3] + s[0] * s[0];
    let v_asym = mm
        .measure_conservation(&h_asym, &[1.0, 0.0, 0.0, 1.0], 0.01, 1000, 0, None, None)
        .unwrap();
    line(
        "asymmetric H: drift over t=10",
        format!("{:.3e}  (conserved: {})", v_asym.drift, v_asym.conserved),
    );
    note("(Noether iff: dH(X_ξ) = 0 ⇔ μ_ξ conserved along H-flow.)");
}

// ── DHOOM encoder — arrays of primitives round-trip ───────────────

fn dhoom_arrays_of_primitives() {
    header("DHOOM", "encoder round-trip for arrays of primitives (recent fix)");

    let input = json!({
        "wikitext": [
            {"id": 1, "tokens": ["the", "cat", "sat"]},
            {"id": 2, "tokens": ["on", "the", "mat"]},
        ]
    });
    let encoded = dhoom::encode(&input).expect("encode");
    let decoded = dhoom::decode(&encoded).expect("decode");

    let recs = decoded["wikitext"].as_array().unwrap();
    line("input records",   input["wikitext"].as_array().unwrap().len());
    line("decoded records", recs.len());
    line("recs[0].tokens",  format!("{}", recs[0]["tokens"]));
    line("recs[1].tokens",  format!("{}", recs[1]["tokens"]));
    note("(\\x1F sentinel + JSON inline; categorizer skips primitive-array → nested.)");
}

// ── PR window — HTTP endpoints summary ────────────────────────────

fn pr_window_endpoints_summary() {
    header("PR window", "HTTP endpoints for Marcella (deployed)");

    let endpoints = [
        ("POST /v1/quantum_cohomology/compose",
         "Frobenius/WDVV composition on CP^n / S² / T^n (catalog §2.10)"),
        ("POST /v1/quantum_cohomology/capacity",
         "Riemann-Roch capacity dim H⁰(L^k) (catalog §2.2)"),
        ("POST /v1/bundles/{name}/holonomy_debt",
         "Quantized vs continuous winding (catalog §E.1)"),
        ("POST /v1/bundles/{name}/flat_transport",
         "Magnetic/classical parallel transport (catalog §1.5)"),
    ];
    for (route, desc) in &endpoints {
        println!("  {:<48} {}", route, desc);
    }
    note("(See tests/kahler_pr_window_marcella_contract.rs for wire shapes.)");
}

// ── main ──────────────────────────────────────────────────────────

fn main() {
    println!("GIGI Kähler-upgrade tour — every shipped layer in one run");
    println!("=========================================================");

    l1_complex_structure_and_two_form();
    l1_5_flat_transport_classical_and_magnetic();
    l2_dual_adjacency_commutativity();
    l3_jacobi_cardinality_bounds();
    l4_kahler_curvature_decomposition();
    l5_hadamard_detection();
    l6_hodge_complex_and_morse_compression();
    l7_line_bundle_holonomy_quantum_cohomology();
    l9_moment_map_noether();
    dhoom_arrays_of_primitives();
    pr_window_endpoints_summary();

    println!();
    println!("Done — 11 layers exercised. Catalog: theory/kahler_upgrade/catalog.md");
}
