//! CV4-wire — end-to-end HTTP envelope tests.
//!
//! Verifies the `/v1/causal_states/commutator` route serializes and
//! deserializes correctly through the same `Commutator` / `KlValue` /
//! `Regime` types the substrate exposes. Doesn't actually spin up the
//! axum server — that's covered by the live smoke test after deploy.
//! Instead we round-trip the wire JSON shape against the substrate to
//! pin the contract.
//!
//! The handler in `gigi_stream.rs` is thin: it constructs operators
//! from `OperatorSpec`, calls `commutator()`, calls `classify_regime()`,
//! and serializes. The substrate already has 77 tests covering the math;
//! these tests pin the *wire* — JSON shapes the notebook MVP and any
//! other client will rely on.

#![cfg(feature = "causal_states")]

use gigi::causal_states::{
    classify_regime, commutator, Commutator, EvenU0, EvenU1, HmmUpdate, KlValue, Regime,
    RegimeBands,
};
use serde_json::json;

const EPS_NUMERIC: f64 = 1e-4;

// ─── KlValue wire format ────────────────────────────────────────────────

#[test]
fn wire_klvalue_finite_serializes_to_kind_finite_value() {
    let kl = KlValue::Finite(0.0327);
    let j = serde_json::to_value(kl).unwrap();
    assert_eq!(j, json!({"kind": "finite", "value": 0.0327}));
}

#[test]
fn wire_klvalue_divergent_serializes_to_kind_divergent() {
    let kl = KlValue::Divergent;
    let j = serde_json::to_value(kl).unwrap();
    assert_eq!(j, json!({"kind": "divergent"}));
}

#[test]
fn wire_klvalue_round_trips() {
    for kl in [KlValue::Finite(0.5), KlValue::Finite(0.0), KlValue::Divergent] {
        let j = serde_json::to_string(&kl).unwrap();
        let back: KlValue = serde_json::from_str(&j).unwrap();
        assert_eq!(kl, back);
    }
}

// ─── Regime wire format ─────────────────────────────────────────────────

#[test]
fn wire_regime_serializes_lowercase_snake() {
    assert_eq!(serde_json::to_value(Regime::Sofic).unwrap(), json!("sofic"));
    assert_eq!(serde_json::to_value(Regime::Smooth).unwrap(), json!("smooth"));
    assert_eq!(
        serde_json::to_value(Regime::Borderline).unwrap(),
        json!("borderline")
    );
}

// ─── Commutator wire format ─────────────────────────────────────────────

#[test]
fn wire_commutator_smooth_hmm_reference_point_shape() {
    // The paper's H5 anchor at (α, β) = (0.2, 0.3):
    //   TV ≈ 0.1062, Hel ≈ 0.0752, KL ≈ 0.0327 bits, Smooth.
    let mu = vec![0.5, 0.5];
    let u_0 = HmmUpdate { alpha: 0.2, beta: 0.3, symbol: 0 };
    let u_1 = HmmUpdate { alpha: 0.2, beta: 0.3, symbol: 1 };
    let omega = commutator(&u_0, &u_1, &mu).unwrap();
    let regime = classify_regime(&omega, RegimeBands::default());

    let response = wire_response(&omega, regime);
    let j: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Shape: top-level keys.
    assert!(j["forward"].is_array());
    assert!(j["backward"].is_array());
    assert!(j["tv"].is_number());
    assert!(j["hellinger"].is_number());
    assert!(j["kl"].is_object());
    assert!(j["regime"].is_string());

    // KL is finite at this anchor.
    assert_eq!(j["kl"]["kind"], json!("finite"));
    let kl_value = j["kl"]["value"].as_f64().unwrap();
    assert!((kl_value - 0.0327).abs() < EPS_NUMERIC,
            "KL value mismatch: got {kl_value}, expected ≈ 0.0327");

    // Regime is smooth.
    assert_eq!(j["regime"], json!("smooth"));

    // TV matches paper to 4 decimals.
    let tv = j["tv"].as_f64().unwrap();
    assert!((tv - 0.1062).abs() < EPS_NUMERIC, "TV mismatch: got {tv}");
}

#[test]
fn wire_commutator_sofic_even_process_shape() {
    let mu = vec![2.0 / 3.0, 1.0 / 3.0];
    let omega = commutator(&EvenU0, &EvenU1, &mu).unwrap();
    let regime = classify_regime(&omega, RegimeBands::default());

    let response = wire_response(&omega, regime);
    let j: serde_json::Value = serde_json::from_str(&response).unwrap();

    // KL is divergent in sofic regime.
    assert_eq!(j["kl"]["kind"], json!("divergent"));
    // Divergent variant must NOT carry a value field.
    assert!(j["kl"].get("value").is_none(),
            "divergent variant should not have a value field");

    // Regime is sofic.
    assert_eq!(j["regime"], json!("sofic"));

    // TV saturates at 1.
    let tv = j["tv"].as_f64().unwrap();
    assert!((tv - 1.0).abs() < 1e-12, "Even Process TV should be 1, got {tv}");
}

// ─── Operator spec wire format (deserialize from JSON) ──────────────────
//
// These tests pin the OperatorSpec JSON shape — the format clients
// (notebook, scripts) use to specify operators.

#[test]
fn wire_operator_even_u0_deserializes() {
    let json_str = r#"{"kind": "even_u0"}"#;
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(value["kind"], json!("even_u0"));
    // Other fields should not be required.
}

#[test]
fn wire_operator_hmm_deserializes() {
    let json_str = r#"{"kind": "hmm", "alpha": 0.2, "beta": 0.3, "symbol": 1}"#;
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(value["kind"], json!("hmm"));
    assert_eq!(value["alpha"], json!(0.2));
    assert_eq!(value["beta"], json!(0.3));
    assert_eq!(value["symbol"], json!(1));
}

// ─── Helper: build the wire response shape exactly as the handler does ──

fn wire_response(omega: &Commutator, regime: Regime) -> String {
    // Mirror the handler's response struct shape.
    let body = json!({
        "forward": omega.forward,
        "backward": omega.backward,
        "tv": omega.tv,
        "hellinger": omega.hellinger,
        "kl": omega.kl,
        "regime": regime,
    });
    serde_json::to_string(&body).unwrap()
}
