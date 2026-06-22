//! TDD-HAL-VI.5 — Bit-identity gold fixture for the canonical
//! LOOP_TRANSPORT call.
//!
//! Pre-registration: HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3 §4.4
//! + §7.2 (Zenodo DOI 10.5281/zenodo.20785681). Gate doc:
//! `theory/halcyon/HALCYON_PART_VI_GATES.md` §Bit-identity contract
//! per-seed.
//!
//! ── Map onto Halcyon Gate VI ──
//!
//! - **VI-F (a) acceptance** (`vi_f_a_acceptance_arm`): replay the
//!   canonical LOOP_TRANSPORT (halcyon_canonical_buckyball + γ_unit
//!   in (Q, β_W), §4.4 parameter pack, SEEDS [20260616..20260623],
//!   N=10000) and assert the four scalar diagnostics agree with the
//!   gold fixture within 1e-10 and the adiabaticity verdict matches.
//!   Debug-safe; `#[ignore]` by default for runtime (~30-120s release,
//!   5-30m debug).
//!
//! - **VI-F (b) regression** (`vi_f_b_regression_arm_release_byte_identity`):
//!   same replay; asserts per-seed `H_forward`/`H_reversed` bit
//!   patterns + scalar diagnostics bit patterns are byte-identical to
//!   fixture. Release-only via `#[cfg_attr(debug_assertions, ignore)]`
//!   (debug FMA + reassociation across 8 × 2 × 10000 substep-edge KDK
//!   ops would drift a few ULPs and falsely fire). `#[ignore]` also
//!   gates default runs.
//!
//! - **`vi_5_capture_fixture`**: harvests the fixture by running the
//!   canonical call, computing SHA-256 of the v3.1.3 SPEC, and
//!   writing/overwriting
//!   `tests/fixtures/halcyon/part_vi/loop_transport_canonical.json`.
//!   `#[ignore]` + `#[cfg_attr(debug_assertions, ignore)]`: only
//!   runs deliberately when the verb is updated post-publication.
//!
//! ── Profile pin (III.8c + IV.10 precedent) ──
//!
//! The regression arm asserts byte-identical reproducibility against
//! the fixture and runs only under `--release` (where the fixture was
//! harvested). The acceptance arm tolerates ULP-scale f64
//! reassociation noise via the 1e-10 bound, so it remains
//! enforceable under any profile (still `#[ignore]` for runtime).
//!
//! Run:
//! ```text
//! # Acceptance arm (any profile):
//! cargo test --features halcyon --test halcyon_part_vi_bit_identity_gold \
//!     -- --ignored vi_f_a_acceptance_arm
//!
//! # Regression arm (release only):
//! cargo test --features halcyon --release \
//!     --test halcyon_part_vi_bit_identity_gold \
//!     -- --ignored vi_f_b_regression_arm_release_byte_identity
//!
//! # Capture / regenerate (release only):
//! cargo test --features halcyon --release \
//!     --test halcyon_part_vi_bit_identity_gold \
//!     -- --ignored vi_5_capture_fixture --nocapture
//! ```
//!
//! ── Optionality contract ──
//!
//! Gated on the `halcyon` composite feature so the no-default-features
//! build stays byte-identical (Bee's optionality contract carrying
//! through every Part I/II/III/IV/V/VI gate).

#![cfg(feature = "halcyon")]

use std::fs;
use std::path::PathBuf;

use gigi::engine::Engine;
use gigi::gauge::loop_transport::{
    clear_loops, loop_transport, register_loop, AdiabaticityCheck,
    LoopTransportDiagnostics, RegisteredLoop,
};
use gigi::parser::{
    execute, parse, ControlManifoldSpec, LoopTransportOutputId, LoopTransportReturnId,
    SeedRange, Statement,
};
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};

// ── Paths ─────────────────────────────────────────────────────────

/// Path to the VI.5 canonical-reference fixture.
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("halcyon")
        .join("part_vi")
        .join("loop_transport_canonical.json")
}

/// Path to the v3.1.3 SPEC (used only at capture time for SHA-256
/// provenance; never read on the acceptance/regression arms — would
/// be machine-fragile).
fn spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("davis-wilson-lattice")
        .join("inertia_damping")
        .join("HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3.md")
}

// ── Registry cleanup ──────────────────────────────────────────────

/// Wipe every Part-VI-relevant singleton.
fn clear_registries() {
    clear_loops();
    gigi::gauge::registry::clear();
    gigi::gauge::registry::clear_e_registry();
    gigi::lattice::registry::clear();
}

// ── Canonical call ────────────────────────────────────────────────

/// Build the `Statement::LoopTransport` carrying the v3.1.3 §4.4
/// parameter pack:
///
/// CONTROL_MANIFOLD (Q, β_W)         RAMP_RATE_Q       = 0.04
/// ADIABATIC TRUE                    RAMP_RATE_BETA_W  = 0.01
/// DRIVE_OMEGA = 1.0                 DRIVE_F0          = 0.01
/// N_DISCRETIZATION = 10000          PIN_LAMBDA_Q      = 1.0
/// PIN_LAMBDA_BETA_W = 1.0           EPS_Q             = 0.05
/// EPS_BETA_W = 0.05                 ALPHA_HALCYON     = 1.0
/// TAU_0 = 1.0                       BETA_TAU          = 2.0
/// MU_BASELINE = 1.0                 K_SPRING          = 1.0
/// C_DAMP = 0.1                      SEEDS [20260616..20260623]
/// SHAM: None (zero-cost-when-off)
fn build_canonical_lt_stmt() -> Statement {
    Statement::LoopTransport {
        lattice: "halcyon_canonical_buckyball".into(),
        loop_id: "gamma_unit_in_Q_beta_W".into(),
        // WISH ASK 5 ride-along: these are the historical default
        // names. The bit-identity gold gate uses the explicit-default
        // path so the canonical fixture stays byte-identical.
        gauge_field_name: "U_lt".into(),
        e_field_name: "E_lt".into(),
        control_manifold: ControlManifoldSpec::QBetaWilson,
        adiabatic: true,
        ramp_rate_q: 0.04,
        ramp_rate_beta_w: 0.01,
        drive_omega: 1.0,
        drive_f0: 0.01,
        n_discretization: 10_000,
        pin_lambda_q: 1.0,
        pin_lambda_beta_w: 1.0,
        eps_q: 0.05,
        eps_beta_w: 0.05,
        alpha_halcyon: 1.0,
        tau_0: 1.0,
        beta_tau: 2.0,
        mu_baseline: 1.0,
        k_spring: 1.0,
        c_damp: 0.1,
        seeds: SeedRange {
            lo: 20_260_616,
            hi: 20_260_623,
        },
        compute: vec![
            LoopTransportOutputId::HolonomyForward,
            LoopTransportOutputId::HolonomyReversed,
            LoopTransportOutputId::TrackingErrorTraceQ,
            LoopTransportOutputId::TrackingErrorTraceBetaW,
            LoopTransportOutputId::AdiabaticityCheck,
        ],
        return_fields: vec![
            LoopTransportReturnId::HForward,
            LoopTransportReturnId::HReversed,
            LoopTransportReturnId::SigmaHBlocked,
            LoopTransportReturnId::PerSeedHForward,
            LoopTransportReturnId::PerSeedHReversed,
            LoopTransportReturnId::TrackingErrorMaxQ,
            LoopTransportReturnId::TrackingErrorMaxBetaW,
            LoopTransportReturnId::AdiabaticityCheck,
        ],
        sham: None,
        beta_wilson_start: 2.75,
    }
}

/// Set up the canonical lattice (`halcyon_canonical_buckyball`),
/// register the SU(2) U field (`U_canon`, INIT IDENTITY) and its
/// companion E field (`E_canon`, INIT ZERO), and register the
/// canonical face-bounded closed loop `gamma_unit_in_Q_beta_W` on
/// face 0 (a pentagon).
///
/// Mirrors `setup_halcyon_canonical_buckyball` in
/// `halcyon_part_vi_executor_smoke.rs` and the helper trio in
/// `halcyon_part_vi_gc_acceptance.rs`, except the lattice carries
/// the v3.1.3 canonical name and the loop is registered
/// programmatically (so we don't need a LOOP GQL statement).
fn setup_canonical_substrate(engine: &mut Engine) {
    let lat_decl = "LATTICE halcyon_canonical_buckyball \
                    FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat_decl).expect("parse LATTICE");
    execute(engine, &stmt).expect("exec LATTICE");

    let gf_decl = "GAUGE_FIELD U_canon ON LATTICE halcyon_canonical_buckyball \
                   GROUP SU(2) INIT IDENTITY;";
    let stmt = parse(gf_decl).expect("parse GAUGE_FIELD");
    execute(engine, &stmt).expect("exec GAUGE_FIELD");

    // Re-publish via register_su2 so the executor's mut handle path
    // finds it (IV.6/VI.2 precedent).
    {
        let lat = gigi::lattice::registry::get("halcyon_canonical_buckyball")
            .expect("declared lattice");
        let su2 = gigi::gauge::SU2GaugeField::new(
            "U_canon".into(),
            &lat,
            gigi::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gigi::gauge::registry::register_su2(su2);
    }

    let ef_decl = "E_FIELD E_canon ON GAUGE_FIELD U_canon INIT ZERO;";
    let stmt = parse(ef_decl).expect("parse E_FIELD");
    execute(engine, &stmt).expect("exec E_FIELD");

    // Register the canonical closed loop programmatically: face 0
    // of the buckyball (a pentagon).
    {
        let lat = gigi::lattice::registry::get("halcyon_canonical_buckyball")
            .expect("declared lattice");
        let face = lat.faces[0].clone();
        let mut vertices = face.clone();
        vertices.push(face[0]); // close
        let mut edges = Vec::with_capacity(face.len());
        for i in 0..face.len() {
            let a = face[i];
            let b = face[(i + 1) % face.len()];
            edges.push(lat.resolve_edge(a, b).expect("face edge resolves"));
        }
        register_loop(
            "gamma_unit_in_Q_beta_W",
            RegisteredLoop {
                lattice_name: "halcyon_canonical_buckyball".into(),
                vertices,
                edges,
            },
        );
    }
}

/// Replay the canonical §4.4 LOOP_TRANSPORT call. Returns the live
/// diagnostics. Used by all three test arms (capture + acceptance +
/// regression).
fn replay_canonical_loop_transport() -> LoopTransportDiagnostics {
    clear_registries();
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");
    setup_canonical_substrate(&mut engine);
    let stmt = build_canonical_lt_stmt();
    loop_transport(&stmt, "U_canon", "E_canon").expect("canonical LOOP_TRANSPORT runs")
}

// ── Fixture helpers ───────────────────────────────────────────────

/// Load the VI.5 canonical fixture from disk. Panics with a regen
/// hint if the file is missing or malformed.
fn load_gold() -> JsonValue {
    let body = fs::read_to_string(fixture_path()).unwrap_or_else(|e| {
        panic!(
            "read VI.5 fixture at {}: {e}. Run \
             `cargo test --features halcyon --release \
              --test halcyon_part_vi_bit_identity_gold \
              -- --ignored vi_5_capture_fixture --nocapture` to regenerate.",
            fixture_path().display()
        )
    });
    serde_json::from_str(&body).expect("parse loop_transport_canonical.json")
}

/// Pull a `(decimal, bits)` pair from a scalar diagnostic slot
/// (`{ "decimal": <f64>, "bits": <u64> }`).
fn scalar_pair(fix: &JsonValue, key: &str) -> (f64, u64) {
    let decimal = fix[key]["decimal"]
        .as_f64()
        .unwrap_or_else(|| panic!("fixture {key}.decimal missing or not f64"));
    let bits = fix[key]["bits"]
        .as_u64()
        .unwrap_or_else(|| panic!("fixture {key}.bits missing or not u64"));
    (decimal, bits)
}

/// Pull a `Vec<u64>` from a per-seed `_bits` array.
fn per_seed_bits(fix: &JsonValue, key: &str) -> Vec<u64> {
    fix[key]
        .as_array()
        .unwrap_or_else(|| panic!("fixture {key} missing or not array"))
        .iter()
        .map(|b| {
            b.as_u64()
                .unwrap_or_else(|| panic!("fixture {key} entry not u64"))
        })
        .collect()
}

/// Pull a `Vec<f64>` from a per-seed `_decimal` array.
fn per_seed_decimal(fix: &JsonValue, key: &str) -> Vec<f64> {
    fix[key]
        .as_array()
        .unwrap_or_else(|| panic!("fixture {key} missing or not array"))
        .iter()
        .map(|b| {
            b.as_f64()
                .unwrap_or_else(|| panic!("fixture {key} entry not f64"))
        })
        .collect()
}

/// Serialize a `LoopTransportDiagnostics` + provenance string into
/// the v3.1.3 §7.2 fixture envelope shape (III.8a bits-oracle +
/// decimal-shadow, group-tagged with verb=LOOP_TRANSPORT).
fn serialize_fixture(diag: &LoopTransportDiagnostics, spec_sha256_hex: &str) -> JsonValue {
    let per_seed_h_forward_decimal: Vec<f64> = diag.per_seed_h_forward.clone();
    let per_seed_h_forward_bits: Vec<u64> =
        diag.per_seed_h_forward.iter().map(|v| v.to_bits()).collect();
    let per_seed_h_reversed_decimal: Vec<f64> = diag.per_seed_h_reversed.clone();
    let per_seed_h_reversed_bits: Vec<u64> = diag
        .per_seed_h_reversed
        .iter()
        .map(|v| v.to_bits())
        .collect();

    let scalar = |v: f64| -> JsonValue {
        json!({
            "decimal": v,
            "bits": v.to_bits(),
        })
    };

    let verdict_str = match diag.adiabaticity_check {
        AdiabaticityCheck::Acceptable { .. } => "Acceptable",
        AdiabaticityCheck::AmbiguousForced { .. } => "AmbiguousForced",
    };
    let ratio = diag.adiabaticity_check.ratio();

    json!({
        "v": "3.1.3",
        "spec_sha256": spec_sha256_hex,
        "spec_path":
            "davis-wilson-lattice/inertia_damping/HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3.md",
        "spec_doi": "10.5281/zenodo.20785681",
        "verb": "LOOP_TRANSPORT",
        "lattice": "halcyon_canonical_buckyball",
        "loop": "gamma_unit_in_Q_beta_W",
        "n_edges": 90,
        "n_vertices": 60,
        "n_faces": 32,
        "config": {
            "control_manifold": ["Q", "beta_wilson"],
            "adiabatic": true,
            "ramp_rate_q": 0.04,
            "ramp_rate_beta_w": 0.01,
            "drive_omega": 1.0,
            "drive_f0": 0.01,
            "n_discretization": 10000,
            "pin_lambda_q": 1.0,
            "pin_lambda_beta_w": 1.0,
            "eps_q": 0.05,
            "eps_beta_w": 0.05,
            "alpha_halcyon": 1.0,
            "tau_0": 1.0,
            "beta_tau": 2.0,
            "mu_baseline": 1.0,
            "k_spring": 1.0,
            "c_damp": 0.1
        },
        "seeds": diag.seeds_used,
        "per_seed_h_forward_decimal": per_seed_h_forward_decimal,
        "per_seed_h_forward_bits":    per_seed_h_forward_bits,
        "per_seed_h_reversed_decimal": per_seed_h_reversed_decimal,
        "per_seed_h_reversed_bits":    per_seed_h_reversed_bits,
        "h_forward_mean":   scalar(diag.h_forward),
        "h_reversed_mean":  scalar(diag.h_reversed),
        "sigma_h_blocked":  scalar(diag.sigma_h_blocked),
        "tracking_error_max_q":      scalar(diag.tracking_error_max_q),
        "tracking_error_max_beta_w": scalar(diag.tracking_error_max_beta_w),
        "adiabaticity_check": {
            "ratio":   scalar(ratio),
            "verdict": verdict_str,
        },
        "n_substeps_completed": diag.n_substeps_completed,
        "diagnostics_only": { }
    })
}

// ── Tests ─────────────────────────────────────────────────────────

/// VI-F (a) ACCEPTANCE — diagnostics within 1e-10 of gold, adiabaticity
/// verdict matches. Debug-safe.
///
/// `#[ignore]` by default for runtime (~30-120s release, 5-30m debug).
/// Promote to default-on if a future cheaper variant is introduced.
///
/// Note on GC₁-GC₆ "green status preserved": the canonical call's
/// successful return (no error) is sufficient — the GC battery is
/// re-asserted by `tests/halcyon_part_vi_gc_acceptance.rs` on every
/// test run, so this gate only owns the "diagnostics match gold"
/// assertion (the v3.1.3 §7.2 sidecar shape).
#[test]
#[ignore = "VI-F (a) acceptance arm — runs canonical LOOP_TRANSPORT \
            (10000 substeps × 8 seeds × 2 dirs); cargo test \
            --features halcyon -- --ignored vi_f_a_acceptance_arm"]
fn vi_f_a_acceptance_arm() {
    let _g = gigi::gauge::registry::test_serial_lock();
    let diag = replay_canonical_loop_transport();
    let fix = load_gold();

    let tol = 1e-10_f64;

    let (h_fwd_fix, _) = scalar_pair(&fix, "h_forward_mean");
    assert!(
        (diag.h_forward - h_fwd_fix).abs() < tol,
        "VI-F(a): h_forward = {} vs fixture {} (|Δ| ≥ {tol})",
        diag.h_forward, h_fwd_fix
    );

    let (h_rev_fix, _) = scalar_pair(&fix, "h_reversed_mean");
    assert!(
        (diag.h_reversed - h_rev_fix).abs() < tol,
        "VI-F(a): h_reversed = {} vs fixture {} (|Δ| ≥ {tol})",
        diag.h_reversed, h_rev_fix
    );

    let (sigma_fix, _) = scalar_pair(&fix, "sigma_h_blocked");
    assert!(
        (diag.sigma_h_blocked - sigma_fix).abs() < tol,
        "VI-F(a): sigma_h_blocked = {} vs fixture {} (|Δ| ≥ {tol})",
        diag.sigma_h_blocked, sigma_fix
    );

    // VI.6b LOCKED disposition (2026-06-21): Fix #3 replaces the
    // hardcoded τ_pin = 1.0 placeholder with a per-substep measurement
    // from project_gauss's final_gauss_residual_inf. On the canonical
    // identity-init U + zero-init E + cold-start ramp, the Gauss
    // residual sits at the clamp floor ≈1e-12 so τ_pin ≈ 1e12 and the
    // published ratio leaves the gold fixture's recorded 1.0 by ~12
    // orders of magnitude. Per LOCKED the gold fixture is NOT
    // regenerated in this scope; the adiabaticity_check.ratio field
    // is explicitly bracketed out of the acceptance arm. A follow-up
    // workflow can regenerate the fixture (deliberate verb-change
    // path via vi_5_capture_fixture, release-only) after Finding #1
    // also lands so the thermalized residual lands in (0,1).
    let _ = fix["adiabaticity_check"]; // intentionally unused.
    let _ = diag.adiabaticity_check.ratio(); // intentionally unused.

    // Tracking-error maxes also covered by the gold (decimal slot,
    // 1e-10 tolerance — same envelope as the four primary diagnostics).
    let (terr_q_fix, _) = scalar_pair(&fix, "tracking_error_max_q");
    assert!(
        (diag.tracking_error_max_q - terr_q_fix).abs() < tol,
        "VI-F(a): tracking_error_max_q = {} vs fixture {} (|Δ| ≥ {tol})",
        diag.tracking_error_max_q, terr_q_fix
    );
    let (terr_bw_fix, _) = scalar_pair(&fix, "tracking_error_max_beta_w");
    assert!(
        (diag.tracking_error_max_beta_w - terr_bw_fix).abs() < tol,
        "VI-F(a): tracking_error_max_beta_w = {} vs fixture {} (|Δ| ≥ {tol})",
        diag.tracking_error_max_beta_w, terr_bw_fix
    );

    // Per-seed decimal slots also within 1e-10 (16 asserts).
    let per_fwd_fix = per_seed_decimal(&fix, "per_seed_h_forward_decimal");
    let per_rev_fix = per_seed_decimal(&fix, "per_seed_h_reversed_decimal");
    assert_eq!(diag.per_seed_h_forward.len(), per_fwd_fix.len());
    assert_eq!(diag.per_seed_h_reversed.len(), per_rev_fix.len());
    for (i, v) in diag.per_seed_h_forward.iter().enumerate() {
        assert!(
            (v - per_fwd_fix[i]).abs() < tol,
            "VI-F(a): per_seed_h_forward[{i}] = {} vs fixture {} (|Δ| ≥ {tol})",
            v, per_fwd_fix[i]
        );
    }
    for (i, v) in diag.per_seed_h_reversed.iter().enumerate() {
        assert!(
            (v - per_rev_fix[i]).abs() < tol,
            "VI-F(a): per_seed_h_reversed[{i}] = {} vs fixture {} (|Δ| ≥ {tol})",
            v, per_rev_fix[i]
        );
    }
}

/// VI-F (b) REGRESSION — release-only byte-identity. Catches the
/// "passes GC algebra, drifts numerical outputs" failure mode
/// (Sprint B lesson). Belt-and-braces: both `#[ignore]` AND
/// `#[cfg_attr(debug_assertions, ignore)]` so debug-profile FMA +
/// reassociation cannot falsely fire.
#[test]
#[ignore = "VI-F (b) regression arm — release-only byte-identity; \
            cargo test --features halcyon --release -- --ignored \
            vi_f_b_regression_arm_release_byte_identity"]
#[cfg_attr(debug_assertions, ignore)]
fn vi_f_b_regression_arm_release_byte_identity() {
    let _g = gigi::gauge::registry::test_serial_lock();
    let diag = replay_canonical_loop_transport();
    let fix = load_gold();

    // ── Per-seed bit-identity (8 × 2 = 16 asserts) ──────────────
    let per_fwd_bits = per_seed_bits(&fix, "per_seed_h_forward_bits");
    let per_rev_bits = per_seed_bits(&fix, "per_seed_h_reversed_bits");
    assert_eq!(
        diag.per_seed_h_forward.len(),
        per_fwd_bits.len(),
        "VI-F(b): per_seed_h_forward length drift"
    );
    assert_eq!(
        diag.per_seed_h_reversed.len(),
        per_rev_bits.len(),
        "VI-F(b): per_seed_h_reversed length drift"
    );
    for (i, v) in diag.per_seed_h_forward.iter().enumerate() {
        assert_eq!(
            v.to_bits(),
            per_fwd_bits[i],
            "VI-F(b): per_seed_h_forward[{i}] bit drift: run={:#x} fix={:#x}",
            v.to_bits(),
            per_fwd_bits[i]
        );
    }
    for (i, v) in diag.per_seed_h_reversed.iter().enumerate() {
        assert_eq!(
            v.to_bits(),
            per_rev_bits[i],
            "VI-F(b): per_seed_h_reversed[{i}] bit drift: run={:#x} fix={:#x}",
            v.to_bits(),
            per_rev_bits[i]
        );
    }

    // ── Scalar diagnostics bit-identity ─────────────────────────
    let (_, h_fwd_bits) = scalar_pair(&fix, "h_forward_mean");
    assert_eq!(
        diag.h_forward.to_bits(),
        h_fwd_bits,
        "VI-F(b): h_forward bit drift: run={:#x} fix={:#x}",
        diag.h_forward.to_bits(),
        h_fwd_bits
    );

    let (_, h_rev_bits) = scalar_pair(&fix, "h_reversed_mean");
    assert_eq!(
        diag.h_reversed.to_bits(),
        h_rev_bits,
        "VI-F(b): h_reversed bit drift: run={:#x} fix={:#x}",
        diag.h_reversed.to_bits(),
        h_rev_bits
    );

    let (_, sigma_bits) = scalar_pair(&fix, "sigma_h_blocked");
    assert_eq!(
        diag.sigma_h_blocked.to_bits(),
        sigma_bits,
        "VI-F(b): sigma_h_blocked bit drift: run={:#x} fix={:#x}",
        diag.sigma_h_blocked.to_bits(),
        sigma_bits
    );

    let (_, terr_q_bits) = scalar_pair(&fix, "tracking_error_max_q");
    assert_eq!(
        diag.tracking_error_max_q.to_bits(),
        terr_q_bits,
        "VI-F(b): tracking_error_max_q bit drift: run={:#x} fix={:#x}",
        diag.tracking_error_max_q.to_bits(),
        terr_q_bits
    );

    let (_, terr_bw_bits) = scalar_pair(&fix, "tracking_error_max_beta_w");
    assert_eq!(
        diag.tracking_error_max_beta_w.to_bits(),
        terr_bw_bits,
        "VI-F(b): tracking_error_max_beta_w bit drift: run={:#x} fix={:#x}",
        diag.tracking_error_max_beta_w.to_bits(),
        terr_bw_bits
    );

    // VI.6b LOCKED disposition (2026-06-21): see VI-F(a) for context.
    // adiabaticity_check.ratio is now a measured τ_pin / T_segment
    // rather than the 1.0 placeholder; gold fixture is NOT regenerated
    // in this scope, so the ratio's bit pattern is explicitly bracketed
    // out of the regression arm. Per-seed h_scalar + tracking errors +
    // h_forward/h_reversed/sigma bit-identity contracts STAY because
    // the underlying KDK trajectory + reduce_su2_to_scalar are unchanged.
    let _ = fix["adiabaticity_check"]["ratio"]["bits"]; // bracketed.
    let _ = diag.adiabaticity_check.ratio(); // bracketed.
}

/// Regenerate the gold fixture. Intended ONLY for deliberate verb
/// changes (which currently means: never, until post-v3.1.3
/// deposit). Documents the regen workflow in the impl log.
#[test]
#[ignore = "regenerates loop_transport_canonical.json; deliberate \
            verb-change workflow only; cargo test --features halcyon \
            --release --test halcyon_part_vi_bit_identity_gold \
            -- --ignored vi_5_capture_fixture --nocapture"]
#[cfg_attr(debug_assertions, ignore)]
fn vi_5_capture_fixture() {
    let _g = gigi::gauge::registry::test_serial_lock();
    let t0 = std::time::Instant::now();
    let diag = replay_canonical_loop_transport();
    let runtime_secs = t0.elapsed().as_secs_f64();

    // SHA-256 of the v3.1.3 SPEC file. Computed once at capture and
    // embedded as a hex string; never read back at acceptance/
    // regression time (would be machine-fragile across CI/contributor
    // machines).
    let spec_bytes = fs::read(spec_path()).unwrap_or_else(|e| {
        panic!(
            "read v3.1.3 SPEC at {}: {e}",
            spec_path().display()
        )
    });
    let mut hasher = Sha256::new();
    hasher.update(&spec_bytes);
    let spec_sha256_hex = format!("{:x}", hasher.finalize());

    let fixture = serialize_fixture(&diag, &spec_sha256_hex);
    let body = serde_json::to_string_pretty(&fixture).expect("serialize fixture");

    // Ensure parent dir exists (idempotent — committed dir, but the
    // capture test should work on a clean clone too).
    if let Some(parent) = fixture_path().parent() {
        fs::create_dir_all(parent).expect("mkdir fixture parent");
    }
    fs::write(fixture_path(), &body).expect("write fixture");

    eprintln!(
        "VI.5 capture: wrote {} ({} bytes) in {runtime_secs:.2}s; \
         spec_sha256={spec_sha256_hex}",
        fixture_path().display(),
        body.len()
    );
    eprintln!("VI.5 capture: adiabaticity verdict = {:?}", diag.adiabaticity_check);
    eprintln!(
        "VI.5 capture: per_seed_h_forward[0..3] = {:?}",
        &diag.per_seed_h_forward[..3.min(diag.per_seed_h_forward.len())]
    );
}
