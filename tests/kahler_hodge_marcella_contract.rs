//! Cross-team interface contract test for L6 — Marcella's Morse
//! compression surface (`BundleStore::morse_compress`).
//!
//! Source of truth: catalog §2.9 ("Witten/Morse spectral
//! compression — Hodge foundation") + IMPLEMENTATION_PLAN.md L6.5.
//! Marcella reads the compressed-cell counts to route transport on
//! a substrate 100×–1000× smaller than the raw record set when
//! prose density is uniform across regions.
//!
//! ### Contract surfaces under test
//!
//! - `BundleStore::morse_compress() -> Option<MorseComplex>`
//! - `MorseComplex { n_critical_0, n_critical_1, n_critical_2,
//!     betti, original_v, original_e, original_f }` field set
//! - `MorseComplex::n_critical()`, `n_original()`,
//!   `compression_ratio()`, `cohomology_preserved()` methods
//! - `BettiNumbers { b0, b1, b2 }::euler_characteristic()`
//! - `HodgeComplex::d_squared_max_abs() == 0` invariant

#![cfg(feature = "kahler")]

use gigi::discrete::{betti, BettiNumbers, HodgeComplex, MorseComplex};
use gigi::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use gigi::BundleStore;

fn kahler_2d() -> KahlerStructure {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
    );
    KahlerStructure::new(j, b)
}

fn small_bundle() -> BundleStore {
    let schema = BundleSchema::new("hodge_test")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .fiber(FieldDef::categorical("tier"))
        .index("tier")
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    for i in 0..20 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(i as f64 * 0.1));
        r.insert("y".into(), Value::Float(i as f64 * 0.05));
        r.insert(
            "tier".into(),
            Value::Text(if i % 3 == 0 { "A" } else { "B" }.into()),
        );
        store.insert(&r);
    }
    store
}

#[test]
fn morse_complex_field_set() {
    let store = small_bundle();
    let m: MorseComplex = store.morse_compress().expect("snapshot");

    // Field-access — compilation fails first if a rename slips in.
    let _: usize = m.n_critical_0;
    let _: usize = m.n_critical_1;
    let _: usize = m.n_critical_2;
    let _: BettiNumbers = m.betti;
    let _: usize = m.original_v;
    let _: usize = m.original_e;
    let _: usize = m.original_f;
}

#[test]
fn morse_complex_methods_return_consistent_values() {
    let store = small_bundle();
    let m = store.morse_compress().expect("snapshot");

    // n_critical = sum of per-degree counts.
    let expected_crit = m.n_critical_0 + m.n_critical_1 + m.n_critical_2;
    assert_eq!(m.n_critical(), expected_crit);

    // n_original = V + E + F.
    let expected_orig = m.original_v + m.original_e + m.original_f;
    assert_eq!(m.n_original(), expected_orig);

    // Compression ratio ≥ 1 (you never expand).
    if m.n_critical() > 0 {
        assert!(
            m.compression_ratio() >= 1.0 - 1e-12,
            "compression_ratio must be ≥ 1; got {}",
            m.compression_ratio()
        );
    }

    // Cohomology preservation always holds by construction
    // (Morse compress copies Betti into critical-cell counts).
    assert!(m.cohomology_preserved());
}

#[test]
fn betti_numbers_field_set_and_euler() {
    let store = small_bundle();
    let m = store.morse_compress().expect("snapshot");
    let b: BettiNumbers = m.betti;

    let _: usize = b.b0;
    let _: usize = b.b1;
    let _: usize = b.b2;

    // Euler characteristic identity.
    let chi = b.b0 as i64 - b.b1 as i64 + b.b2 as i64;
    assert_eq!(b.euler_characteristic(), chi);
}

#[test]
fn hodge_complex_d_squared_zero_invariant() {
    // The d² = 0 chain identity must hold on any well-formed
    // HodgeComplex; this is the gate that prevents "I added an
    // edge but forgot the orientation sign" bugs from drifting
    // into Marcella's Betti reads.
    let edges = vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let faces = vec![(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];
    let hc = HodgeComplex::new(4, edges, faces).expect("tet build");
    assert!(
        hc.d_squared_max_abs() < 1e-12,
        "tetrahedron d² invariant must hold; got {}",
        hc.d_squared_max_abs()
    );
}

#[test]
fn betti_function_signature_stable() {
    // Calling the free function `betti(hc, tol)` MUST stay in the
    // public surface — Marcella consumes it directly when she
    // builds custom complexes from prose neighborhoods.
    let edges = vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let faces = vec![(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];
    let hc = HodgeComplex::new(4, edges, faces).expect("tet build");
    let b = betti(&hc, 1e-8);
    // Tetrahedron = S²; Betti = (1, 0, 1).
    assert_eq!((b.b0, b.b1, b.b2), (1, 0, 1));
}

#[test]
fn morse_compress_returns_none_for_tiny_bundle() {
    // Negative: bundle with < 2 records ⇒ degenerate cell complex
    // ⇒ None. Marcella maps None to "skip Morse routing; walk the
    // raw substrate."
    let schema = BundleSchema::new("tiny")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(0));
    r.insert("x".into(), Value::Float(0.0));
    r.insert("y".into(), Value::Float(0.0));
    store.insert(&r);
    assert!(
        store.morse_compress().is_none(),
        "1-record bundle: morse_compress must return None"
    );
}
