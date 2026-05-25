//! Cross-team interface contract test for L7 — Marcella's
//! quantization surfaces (`LineBundle`, `holonomy_debt`,
//! `QuantizedTwoForm`, `QuantumCohomology`, `ToeplitzOperator`,
//! `representational_capacity`, `hilbert_polynomial`).
//!
//! Source of truth: catalog §2.1, §2.2, §2.8, §2.10, §E.1, §E.2
//! + IMPLEMENTATION_PLAN.md L7. Marcella's runtime reads each
//! surface for a distinct routing decision:
//!
//! - `LineBundle` integrality ⇒ "can I use DHOOM Chern
//!   compression on this bundle's B?"
//! - `holonomy_debt` ⇒ "does this loop's residue persist under
//!   gauge?" (Davis non-decoupling)
//! - `QuantumCohomology::compose` ⇒ "is composition associative
//!   on this region?" (Frobenius / WDVV)
//! - `toeplitz_operator` ⇒ semiclassical Berezin-Toeplitz, with
//!   safe-ℏ gate.
//! - `representational_capacity` ⇒ Riemann-Roch bound for the
//!   AGI-claim publishable statement.
//!
//! ### Contract surfaces under test
//!
//! - `LineBundle { chern, integral_value }` field set + variants
//!   of `IntegralityError`.
//! - `HolonomyDebt::{Quantized(i64), Continuous(f64)}`.
//! - `QuantizedTwoForm { chern, loop_area, dim }` field set.
//! - `QuantumCohomology::{Cpn{n,q_truncation}, TorusTn{n},
//!     Sphere2, NonToy}` variants.
//! - `ToeplitzOperator { dim, matrix, hbar,
//!     truncation_dominates_correction }` field set.
//! - `HilbertPolynomial { coefficients, scale_denominator }`.

#![cfg(feature = "kahler")]

use gigi::curvature::{holonomy_debt, HolonomyDebt};
use gigi::dhoom::{decode_chern, encode_chern, QuantizedTwoForm};
use gigi::geometry::{
    toeplitz_operator, ChernClass, ClosedTwoForm, CohClass, ComplexStructure,
    HilbertPolynomial, IntegralityError, KahlerStructure, LineBundle, QuantumCohomology,
    QuantumError, ToeplitzError, ToeplitzOperator, TwoForm,
};
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use gigi::BundleStore;

fn kahler_2d() -> KahlerStructure {
    let j = ComplexStructure::standard(1);
    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
    );
    KahlerStructure::new(j, b)
}

fn flat_bundle() -> BundleStore {
    let schema = BundleSchema::new("l7_test")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("x").with_range(2.0))
        .fiber(FieldDef::numeric("y").with_range(2.0))
        .with_kahler(kahler_2d());
    let mut store = BundleStore::new(schema);
    for i in 0..10 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("x".into(), Value::Float(0.0));
        r.insert("y".into(), Value::Float(0.0));
        store.insert(&r);
    }
    store
}

#[test]
fn line_bundle_field_set() {
    let lb = LineBundle::from_transition_data(2.0 * std::f64::consts::PI, 1e-10)
        .expect("integer chern");
    let _: ChernClass = lb.chern;
    let _: f64 = lb.integral_value;
    let _: ChernClass = lb.chern_class();
    let _: i64 = lb.chern.0;
    assert!(!lb.chern.is_trivial());
}

#[test]
fn integrality_error_variants_exhaustive() {
    fn assert_exhaustive(e: &IntegralityError) -> &'static str {
        match e {
            IntegralityError::DiracString { .. } => "dirac",
            IntegralityError::DimensionUnsupported { .. } => "dim",
        }
    }
    let dirac = LineBundle::from_transition_data(0.7 * std::f64::consts::PI, 1e-10)
        .expect_err("non-integer");
    assert_eq!(assert_exhaustive(&dirac), "dirac");

    let tf = TwoForm::new(vec![0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0,
                               0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0], 4)
        .expect("antisymmetric 4x4");
    let cf = ClosedTwoForm::new_constant(tf);
    let dim_err = LineBundle::from_constant_two_form(&cf, 1.0, 1e-10)
        .expect_err("dim 4 unsupported");
    assert_eq!(assert_exhaustive(&dim_err), "dim");
}

#[test]
fn holonomy_debt_variants_and_methods() {
    let store = flat_bundle();

    // Quantized variant.
    let q = holonomy_debt(&store, 2.0 * std::f64::consts::PI * 3.0, 1e-6).unwrap();
    assert!(matches!(q, HolonomyDebt::Quantized(3)));
    assert!(q.is_quantized());
    assert_eq!(q.winding(), 3.0);

    // Continuous variant.
    let c = holonomy_debt(&store, 2.0 * std::f64::consts::PI * 0.7, 1e-6).unwrap();
    assert!(matches!(c, HolonomyDebt::Continuous(_)));
    assert!(!c.is_quantized());

    // Exhaustive match.
    fn assert_exhaustive(d: HolonomyDebt) -> &'static str {
        match d {
            HolonomyDebt::Quantized(_) => "quant",
            HolonomyDebt::Continuous(_) => "cont",
        }
    }
    assert_eq!(assert_exhaustive(q), "quant");
    assert_eq!(assert_exhaustive(c), "cont");
}

#[test]
fn quantized_two_form_field_set_and_round_trip() {
    let tf = TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric");
    let b = ClosedTwoForm::new_constant(tf);
    let area = 4.0 * std::f64::consts::PI;
    let qf: QuantizedTwoForm = encode_chern(&b, area, 1e-10).expect("integral");

    let _: i64 = qf.chern;
    let _: f64 = qf.loop_area;
    let _: usize = qf.dim;
    assert_eq!(qf.chern, 1);

    let decoded = decode_chern(&qf);
    let orig = b.form().matrix()[1];
    let new = decoded.form().matrix()[1];
    assert!((orig - new).abs() < f64::EPSILON);
}

#[test]
fn quantum_cohomology_variants_exhaustive() {
    fn assert_exhaustive(qh: &QuantumCohomology) -> &'static str {
        match qh {
            QuantumCohomology::Cpn { .. } => "cpn",
            QuantumCohomology::TorusTn { .. } => "tn",
            QuantumCohomology::Sphere2 => "s2",
            QuantumCohomology::NonToy => "nontoy",
        }
    }
    assert_eq!(assert_exhaustive(&QuantumCohomology::cpn(2)), "cpn");
    assert_eq!(assert_exhaustive(&QuantumCohomology::TorusTn { n: 3 }), "tn");
    assert_eq!(assert_exhaustive(&QuantumCohomology::Sphere2), "s2");
    assert_eq!(assert_exhaustive(&QuantumCohomology::NonToy), "nontoy");
}

#[test]
fn frobenius_compose_cp2_is_associative_for_marcella() {
    // The whole reason Marcella adopted L7.5: associativity is a
    // theorem on QH*(CP²). Contract test re-asserts the CP²
    // associator is zero — the L7.5 ground truth carries through
    // to the cross-team API.
    let qh = QuantumCohomology::cpn(2);
    let basis = [
        CohClass::h_power(0),
        CohClass::h_power(1),
        CohClass::h_power(2),
    ];
    for a in &basis {
        for b in &basis {
            for c in &basis {
                let ab = qh.compose(a, b).unwrap();
                let abc1 = qh.compose(&ab, c).unwrap();
                let bc = qh.compose(b, c).unwrap();
                let abc2 = qh.compose(a, &bc).unwrap();
                let neg = CohClass {
                    terms: abc2.terms.iter().map(|&(c, h, q)| (-c, h, q)).collect(),
                };
                let diff = abc1.add(&neg);
                assert!(
                    diff.linf_norm() < 1e-12,
                    "CP² associator must be 0 on contract surface"
                );
            }
        }
    }
}

#[test]
fn quantum_error_unsupported_manifold_carries_reason() {
    let err = QuantumCohomology::NonToy
        .compose(&CohClass::h_power(0), &CohClass::h_power(1))
        .expect_err("NonToy");
    match err {
        QuantumError::UnsupportedManifold { reason } => {
            assert_eq!(reason, "general_GW_invariants_not_computable");
        }
    }
}

#[test]
fn toeplitz_operator_field_set() {
    let op: ToeplitzOperator = toeplitz_operator(
        &QuantumCohomology::cpn(1),
        1.0,
        0.5,
        100,
        false,
    )
    .expect("safe ℏ");
    let _: usize = op.dim;
    let _: &Vec<f64> = &op.matrix;
    let _: f64 = op.hbar;
    let _: bool = op.truncation_dominates_correction;
}

#[test]
fn toeplitz_error_variants_exhaustive() {
    fn assert_exhaustive(e: &ToeplitzError) -> &'static str {
        match e {
            ToeplitzError::UnsupportedManifold => "unsupp",
            ToeplitzError::HbarBelowSafeBound { .. } => "below",
            ToeplitzError::NonPositiveHbar(_) => "neg",
        }
    }
    let u = toeplitz_operator(&QuantumCohomology::NonToy, 1.0, 0.1, 64, true)
        .expect_err("nontoy");
    assert_eq!(assert_exhaustive(&u), "unsupp");
    let b = toeplitz_operator(&QuantumCohomology::cpn(1), 1.0, 0.01, 100, false)
        .expect_err("below bound");
    assert_eq!(assert_exhaustive(&b), "below");
    let n = toeplitz_operator(&QuantumCohomology::cpn(1), 1.0, 0.0, 100, true)
        .expect_err("neg ℏ");
    assert_eq!(assert_exhaustive(&n), "neg");
}

#[test]
fn representational_capacity_riemann_roch() {
    let qh = QuantumCohomology::cpn(2);
    // dim H⁰(CP², L^3) = binomial(5, 2) = 10. AGI-claim
    // publishable bound Marcella reads at k = 1/ℏ.
    assert_eq!(qh.representational_capacity(3).unwrap(), 10);

    // NonToy refuses.
    assert!(matches!(
        QuantumCohomology::NonToy.representational_capacity(3),
        Err(QuantumError::UnsupportedManifold { .. })
    ));
}

#[test]
fn hilbert_polynomial_field_set() {
    let p: HilbertPolynomial = QuantumCohomology::cpn(2).hilbert_polynomial().unwrap();
    let _: &Vec<i64> = &p.coefficients;
    let _: i64 = p.scale_denominator;
    assert_eq!(p.degree(), 2);
    // CP² Hilbert poly = (k+1)(k+2)/2. Eval at k=3: 4·5/2 = 10.
    let (num, den) = p.eval(3);
    assert_eq!(num as f64 / den as f64, 10.0);
}
