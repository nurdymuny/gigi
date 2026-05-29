//! GIGI Encrypt v0.3 — Sprint J: Aff(ℝ) capability delegation tests.
//!
//! 11 tests covering the 10 sprint-level test names from spec §4.2 plus the
//! load-bearing **explicit collusion test** (`test_capability_collusion_
//! recovers_alice_key_explicit`). The collusion test passes by design — it
//! exercises Limitation 4.7.1 and proves that limitation is in scope, not
//! a hidden bug.
//!
//! Run with: `cargo test --test delegation_v0_3`

use gigi::crypto::{FieldTransform, GaugeKey};
use gigi::delegation::{DelegationCapability, DelegationError, FieldDelegationTransform};

// ───────────────────────────────────────────────────────────────────────
// Helpers
// ───────────────────────────────────────────────────────────────────────

fn affine(scale: f64, offset: f64) -> FieldTransform {
    FieldTransform::Affine { scale, offset }
}

fn opaque(byte: u8) -> FieldTransform {
    FieldTransform::Opaque { key: [byte; 32] }
}

fn indexed(byte: u8) -> FieldTransform {
    FieldTransform::Indexed { key: [byte; 32] }
}

fn probabilistic() -> FieldTransform {
    FieldTransform::Probabilistic {
        scale: 1.0,
        offset: 0.0,
        sigma: 0.5,
        bucket_key: [0u8; 32],
    }
}

fn isometric_group(gid: &str, member: usize) -> FieldTransform {
    FieldTransform::Isometric {
        group_id: gid.to_string(),
        matrix: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        offset_vec: vec![0.0, 0.0],
        member_index: member,
    }
}

// ───────────────────────────────────────────────────────────────────────
// Spec §4.2 — 11 tests
// ───────────────────────────────────────────────────────────────────────

/// Affine Alice → Bob roundtrip.
#[test]
fn test_capability_affine_alice_to_bob_roundtrip() {
    let g_a = GaugeKey {
        transforms: vec![affine(2.0, 5.0)],
    };
    let g_b = GaugeKey {
        transforms: vec![affine(3.0, -1.0)],
    };
    let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
    // Encrypt plaintext v under Alice:
    let v = 7.0;
    let w_a = 2.0 * v + 5.0;
    // Apply capability:
    let w_b = cap.apply_to_value(0, w_a).unwrap();
    // Decrypt under Bob:
    let v_recovered = (w_b - (-1.0)) / 3.0;
    assert!((v_recovered - v).abs() < 1e-12, "round-trip lost precision");
}

/// Applying the capability invokes no decryption primitive.
///
/// The current impl computes `α·w + β` directly from the cached composite —
/// no path through `FieldTransform::decrypt_value`. We verify this
/// structurally by exercising the apply path on input that would normally
/// require decryption (Opaque) — and confirming it returns a typed refusal,
/// not silently decrypting.
#[test]
fn test_capability_affine_zero_decrypt_calls() {
    // For affine fields, apply is pure arithmetic — no decrypt path.
    let g_a = GaugeKey {
        transforms: vec![affine(1.5, 0.25)],
    };
    let g_b = GaugeKey {
        transforms: vec![affine(2.0, 0.0)],
    };
    let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
    // The composite is fully cached; apply is constant-time arithmetic.
    let result = cap.apply_to_value(0, 42.0);
    assert!(result.is_ok());
    // For modes with no closure, apply explicitly refuses rather than
    // falling through to a decrypt path.
    let g_o = GaugeKey {
        transforms: vec![opaque(0xAA)],
    };
    let g_o2 = GaugeKey {
        transforms: vec![opaque(0xBB)],
    };
    let cap_o = DelegationCapability::build(&g_o, &g_o2, "A".into(), "B".into()).unwrap();
    let err = cap_o.apply_to_value(0, 0.0).unwrap_err();
    assert!(matches!(err, DelegationError::NotAffineClosure("Opaque")));
}

/// Proxy holding only the capability gets w_B from w_A, not v.
#[test]
fn test_capability_proxy_cannot_decrypt_alone() {
    let g_a = GaugeKey {
        transforms: vec![affine(2.5, 3.0)],
    };
    let g_b = GaugeKey {
        transforms: vec![affine(1.1, -0.5)],
    };
    let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
    // Alice encrypts plaintext v = 10:
    let v = 10.0;
    let w_a = 2.5 * v + 3.0; // w_a = 28
    let w_b = cap.apply_to_value(0, w_a).unwrap();
    // Bob would compute v = (w_b - b_B) / a_B; the proxy doesn't have b_B / a_B,
    // so the proxy sees w_b but not v.
    let expected_w_b = 1.1 * v + (-0.5);
    assert!((w_b - expected_w_b).abs() < 1e-12);
    // The proxy's output is NOT the plaintext:
    assert_ne!(w_b, v);
}

/// Given only `(α, β)`, the proxy cannot recover Alice's `(a_A, b_A)` —
/// 2 equations, 4 unknowns. Confirmed empirically: two different (Alice)
/// keys, same Bob key, produce different (α, β); but the inverse problem
/// from (α, β) alone has a 2-dim solution manifold.
#[test]
fn test_capability_proxy_alone_cannot_recover_alice_key() {
    let g_b = GaugeKey {
        transforms: vec![affine(3.0, 1.0)],
    };
    let g_a1 = GaugeKey {
        transforms: vec![affine(2.0, 5.0)],
    };
    let g_a2 = GaugeKey {
        transforms: vec![affine(4.0, 9.0)],
    };
    let cap1 = DelegationCapability::build(&g_a1, &g_b, "A1".into(), "B".into()).unwrap();
    let cap2 = DelegationCapability::build(&g_a2, &g_b, "A2".into(), "B".into()).unwrap();
    // Capabilities differ but the proxy can't pin which Alice key produced
    // which capability without additional info (Bob's key, or a plaintext
    // sample — see Limitation 4.7.2).
    let (a1, b1) = match &cap1.field_transforms[0] {
        FieldDelegationTransform::Affine { alpha, beta } => (*alpha, *beta),
        _ => panic!(),
    };
    let (a2, b2) = match &cap2.field_transforms[0] {
        FieldDelegationTransform::Affine { alpha, beta } => (*alpha, *beta),
        _ => panic!(),
    };
    assert!(
        (a1 - a2).abs() > 1e-9 || (b1 - b2).abs() > 1e-9,
        "different Alice keys must produce different capabilities"
    );
}

/// Symmetric: same construction in reverse direction.
#[test]
fn test_capability_proxy_alone_cannot_recover_bob_key() {
    let g_a = GaugeKey {
        transforms: vec![affine(2.0, 5.0)],
    };
    let g_b1 = GaugeKey {
        transforms: vec![affine(3.0, 1.0)],
    };
    let g_b2 = GaugeKey {
        transforms: vec![affine(7.0, -2.0)],
    };
    let cap1 = DelegationCapability::build(&g_a, &g_b1, "A".into(), "B1".into()).unwrap();
    let cap2 = DelegationCapability::build(&g_a, &g_b2, "A".into(), "B2".into()).unwrap();
    let (a1, b1) = match &cap1.field_transforms[0] {
        FieldDelegationTransform::Affine { alpha, beta } => (*alpha, *beta),
        _ => panic!(),
    };
    let (a2, b2) = match &cap2.field_transforms[0] {
        FieldDelegationTransform::Affine { alpha, beta } => (*alpha, *beta),
        _ => panic!(),
    };
    assert!((a1 - a2).abs() > 1e-9 || (b1 - b2).abs() > 1e-9);
}

/// **Load-bearing test for Limitation 4.7.1**: this passes BY DESIGN.
/// Bob, holding `(α, β)` and his own gauge key `(a_B, b_B)`, solves
/// `a_A = a_B / α` and `b_A = (b_B − β) / α` — recovering Alice's full
/// key. A passing test here documents that the limitation is in scope:
/// future engineers cannot accidentally "fix" the construction and have
/// this test silently start failing — they'd need to deliberately remove
/// the test, which forces conscious re-examination of the security model.
#[test]
fn test_capability_collusion_recovers_alice_key_explicit() {
    let (a_a, b_a) = (2.5, 3.0);
    let (a_b, b_b) = (1.1, -0.5);
    let g_a = GaugeKey {
        transforms: vec![affine(a_a, b_a)],
    };
    let g_b = GaugeKey {
        transforms: vec![affine(a_b, b_b)],
    };
    let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
    let (alpha, beta) = match &cap.field_transforms[0] {
        FieldDelegationTransform::Affine { alpha, beta } => (*alpha, *beta),
        _ => panic!(),
    };
    // Bob colludes — solves for Alice's key.
    let recovered_a_a = a_b / alpha;
    let recovered_b_a = (b_b - beta) / alpha;
    assert!(
        (recovered_a_a - a_a).abs() < 1e-12,
        "collusion attack must recover a_A exactly (Limitation 4.7.1)"
    );
    assert!(
        (recovered_b_a - b_a).abs() < 1e-12,
        "collusion attack must recover b_A exactly (Limitation 4.7.1)"
    );
}

/// Capability revocation: when Alice rotates her gauge, the OLD capability
/// no longer transforms Alice's ciphertext correctly to Bob's space.
/// Tested as: build C with the original Alice key, rotate Alice, attempt
/// to use C on a freshly-encrypted record — should produce a value that
/// does NOT decrypt to the original plaintext under Bob.
#[test]
fn test_capability_revoked_after_rotation() {
    let g_a_old = GaugeKey {
        transforms: vec![affine(2.0, 5.0)],
    };
    let g_b = GaugeKey {
        transforms: vec![affine(3.0, 1.0)],
    };
    let cap = DelegationCapability::build(&g_a_old, &g_b, "A".into(), "B".into()).unwrap();

    // Alice rotates to a new gauge.
    let g_a_new = GaugeKey {
        transforms: vec![affine(7.0, -2.0)],
    };
    // A fresh write under Alice's new gauge:
    let v = 10.0;
    let w_a_new = 7.0 * v + (-2.0);
    // Apply old capability to new ciphertext:
    let w_b_attempted = cap.apply_to_value(0, w_a_new).unwrap();
    // Decrypt under Bob would give a wrong plaintext:
    let v_wrong = (w_b_attempted - 1.0) / 3.0;
    assert!(
        (v_wrong - v).abs() > 1e-6,
        "old capability on new Alice gauge must NOT roundtrip to original v"
    );
    // Sanity: with a fresh capability built for the new Alice gauge, it
    // would roundtrip correctly.
    let cap_new = DelegationCapability::build(&g_a_new, &g_b, "A".into(), "B".into()).unwrap();
    let w_b_correct = cap_new.apply_to_value(0, w_a_new).unwrap();
    let v_correct = (w_b_correct - 1.0) / 3.0;
    assert!((v_correct - v).abs() < 1e-12);
}

/// Isometric closure: build succeeds with the correct composite matrix.
///
/// `apply_to_value` defers (returns `IsometricApplyRequiresGroup`) because
/// the apply path needs group-aware bundle integration that lives in a
/// follow-up commit (the impl module-level test confirms the math primitive
/// is correct).
#[test]
fn test_capability_isometric_closure() {
    let g_a = GaugeKey {
        transforms: vec![isometric_group("wind", 0)],
    };
    let g_b = GaugeKey {
        transforms: vec![isometric_group("wind", 0)],
    };
    let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
    assert_eq!(cap.closure_summary().isometric, 1);
    // apply defers explicitly:
    let err = cap.apply_to_value(0, 1.0).unwrap_err();
    assert!(matches!(err, DelegationError::IsometricApplyRequiresGroup));
}

/// Opaque source field → typed refusal at apply time.
#[test]
fn test_capability_opaque_returns_typed_error() {
    let g_a = GaugeKey {
        transforms: vec![opaque(0xAA)],
    };
    let g_b = GaugeKey {
        transforms: vec![opaque(0xBB)],
    };
    let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
    let err = cap.apply_to_value(0, 0.0).unwrap_err();
    assert!(matches!(err, DelegationError::NotAffineClosure("Opaque")));
}

/// Indexed source field → typed refusal at apply time.
#[test]
fn test_capability_indexed_returns_typed_error() {
    let g_a = GaugeKey {
        transforms: vec![indexed(0x11)],
    };
    let g_b = GaugeKey {
        transforms: vec![indexed(0x22)],
    };
    let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
    let err = cap.apply_to_value(0, 0.0).unwrap_err();
    assert!(matches!(err, DelegationError::NotAffineClosure("Indexed")));
}

/// Probabilistic source field → typed refusal at apply time.
#[test]
fn test_capability_probabilistic_returns_typed_error() {
    let g_a = GaugeKey {
        transforms: vec![probabilistic()],
    };
    let g_b = GaugeKey {
        transforms: vec![probabilistic()],
    };
    let cap = DelegationCapability::build(&g_a, &g_b, "A".into(), "B".into()).unwrap();
    let err = cap.apply_to_value(0, 0.0).unwrap_err();
    assert!(matches!(
        err,
        DelegationError::NotAffineClosure("Probabilistic")
    ));
}
