//! `LOOP_TRANSPORT` — VI.2 verb (HALCYON Part VI deliverable #2).
//!
//! Pre-registration: HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3 §4.4
//! (Zenodo DOI 10.5281/zenodo.20785681). Gate doc:
//! `theory/halcyon/HALCYON_PART_VI_GATES.md` @ 9a73dc0.
//!
//! Spec naming: SAMPLE_TRANSPORT (frozen at deposit). Implementation
//! name: LOOP_TRANSPORT per v1 reply §3 + Halcyon reply 2 §B.1. The
//! existing `src/geometry/sample_transport.rs` is UNRELATED and stays
//! untouched.
//!
//! VI.2 scope (this module):
//!   - `LoopTransportDiagnostics` — the 8-field RETURN surface
//!     (+ echo block) VI.3/4/5 will pattern-match against.
//!   - `AdiabaticityCheck` — verdict enum per v3.1.3 §4.2
//!     (`Acceptable { ratio }` for ratio < 0.1, `AmbiguousForced
//!     { ratio }` for ratio ≥ 0.1). Carried as DATA, never an error.
//!   - `LoopTransportError` — parser-rejection + executor-runtime
//!     variants. `AdiabaticityForcedAmbiguous` is intentionally NOT
//!     here — see notes (2) in the design payload.
//!   - `loop_transport(stmt, u_name, e_name) -> Diagnostics` — public
//!     executor entry. Reuses SYMPLECTIC_FLOW per-substep KDK building
//!     blocks (gate doc Locked decisions): `wilson_force_per_edge`,
//!     `apply_force_kick`, `drift_step`, `project_gauss`, and
//!     `walk_loop` for the loop-closure holonomy.
//!
//! Hot-path discipline: per-substep loop body calls concrete SU(2)
//! kernels directly; no trait-object dispatch inside the KDK body.
//! `walk_loop`'s `&dyn EdgeConnection` is invoked twice per direction
//! (start + end), not per substep.

use std::sync::Arc;

use super::e_field::SU2EField;
use super::error::GaugeFieldError;
use super::gauss::build_vertex_edge_incidence;
use super::group::Group;
use super::holonomy::walk_loop;
use super::lie_exp::drift_step;
use super::project_gauss::{project_gauss, ProjectGaussConfig};
use super::registry::{get_su2_e_mut, get_su2_mut, register_su2, GaugeFieldHandle};
use super::staple::{build_edge_face_incidence, build_face_edges_cache};
use super::su2_gauge_field::SU2GaugeField;
use super::wilson_force::{apply_force_kick, wilson_force_per_edge};
use crate::lattice::{registry as lattice_registry, EdgeId, EdgeOrientation, VertexId};

use crate::parser::{
    ControlManifoldSpec, LoopTransportOutputId, LoopTransportReturnId, SeedRange, ShamBlock,
    Statement,
};

// ── Loop registry ──────────────────────────────────────────────────
//
// `LOOP name ON lattice FACE n` / `LOOP name ON lattice EDGES (v0,…)`
// statements register a closed (or, for the OPEN_LOOP audit-story
// rejection test, intentionally non-closed) vertex path against a
// lattice. Resolution to `(EdgeId, EdgeOrientation)` happens at
// registration time via `Lattice::resolve_edge`; the LOOP_TRANSPORT
// executor reads the cached edge list back through `loop_edges()`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// One registered loop: lattice name + ordered vertex path + resolved
/// edge list. The vertex path is the audit-story handle (the
/// `LoopNotClosed` rejection compares `vertices.first()` vs
/// `vertices.last()`); the edge list is what `walk_loop` reads.
#[derive(Debug, Clone)]
pub struct RegisteredLoop {
    /// Owning lattice name.
    pub lattice_name: String,
    /// Ordered vertex path (length >= 2). For a FACE-based loop, this
    /// is the face cycle repeated with the closing duplicate so
    /// `vertices.first() == vertices.last()`; for an EDGES-based loop,
    /// this is exactly what the user supplied.
    pub vertices: Vec<VertexId>,
    /// Resolved edge sequence. Length == `vertices.len() - 1`.
    pub edges: Vec<(EdgeId, EdgeOrientation)>,
}

impl RegisteredLoop {
    /// `true` if the first and last vertex coincide.
    pub fn is_closed(&self) -> bool {
        match (self.vertices.first(), self.vertices.last()) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }

    /// `(tail, head)` of the path (first vs last vertex). Defined when
    /// `vertices.len() >= 1`.
    pub fn endpoints(&self) -> Option<(VertexId, VertexId)> {
        match (self.vertices.first(), self.vertices.last()) {
            (Some(&a), Some(&b)) => Some((a, b)),
            _ => None,
        }
    }
}

fn loop_registry() -> &'static Mutex<HashMap<String, RegisteredLoop>> {
    static REG: OnceLock<Mutex<HashMap<String, RegisteredLoop>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a loop under its name. Overwrites any previous
/// registration with the same name.
pub fn register_loop(loop_id: &str, reg: RegisteredLoop) {
    let mut g = loop_registry().lock().expect("loop registry mutex poisoned");
    g.insert(loop_id.to_string(), reg);
}

/// Look up a registered loop by name. Returns a clone for round-trip
/// stability.
pub fn get_loop(loop_id: &str) -> Option<RegisteredLoop> {
    let g = loop_registry().lock().expect("loop registry mutex poisoned");
    g.get(loop_id).cloned()
}

/// Test convenience: clear the loop registry.
pub fn clear_loops() {
    let mut g = loop_registry().lock().expect("loop registry mutex poisoned");
    g.clear();
}

// ── Public surface: diagnostics + adiabaticity verdict ────────────

/// Per-call diagnostics from one `loop_transport` invocation. Mirrors
/// the v3.1.3 §4.4 RETURN list 1:1 plus a small echo block for
/// debuggability and the smoke-test shape gauntlet.
///
/// Field names are exactly what GC₁..GC₆ (VI.3) and the bit-identity
/// gold fixture (VI.5) will index into — do NOT rename without
/// versioning the spec.
#[derive(Debug, Clone)]
pub struct LoopTransportDiagnostics {
    /// `H_forward` = mean over seeds of `H[γ_s]` reduced to a real
    /// scalar (q0 of the resulting SU(2) holonomy product, i.e.
    /// `cos(θ/2)`).
    pub h_forward: f64,
    /// `H_reversed` = mean over seeds of `H[γ⁻¹_s]`.
    pub h_reversed: f64,
    /// Block-bootstrap σ over the SEEDS bracket. VI.2 uses the simple
    /// per-seed sd estimator; VI.3 swaps in the v3.1.3 §3.2 block
    /// estimator.
    pub sigma_h_blocked: f64,
    /// Per-seed `H[γ_s]` chain (length == seeds.hi - seeds.lo + 1).
    pub per_seed_h_forward: Vec<f64>,
    /// Per-seed `H[γ⁻¹_s]` chain (same length).
    pub per_seed_h_reversed: Vec<f64>,
    /// L∞ max over substeps × seeds of the Q tracking error.
    pub tracking_error_max_q: f64,
    /// L∞ max over substeps × seeds of the β_W tracking error.
    pub tracking_error_max_beta_w: f64,
    /// Adiabaticity verdict per v3.1.3 §4.2 (data, not error).
    pub adiabaticity_check: AdiabaticityCheck,
    // ── Echo block (not in RETURN; aids debuggability + smoke test) ──
    /// Seed list actually executed (echoes the SEEDS bracket).
    pub seeds_used: Vec<u64>,
    /// Substeps the loop completed per seed. Equals N_DISCRETIZATION
    /// on success (VI.2 always completes; field exists for VI.4 SHAM
    /// early-exit compatibility).
    pub n_substeps_completed: usize,
}

/// Verdict carried in the diagnostics row. Per v3.1.3 §4.2:
///
///   tau_pin   = 1 / min(PIN_LAMBDA_Q, PIN_LAMBDA_BETA_W)   (slowest pin)
///   T_segment = N_DISCRETIZATION · dt_substep              (loop duration)
///   ratio     = tau_pin / T_segment
///
/// `ratio < 0.1`  → `Acceptable` (pins relax much faster than the ramp).
/// `ratio ≥ 0.1`  → `AmbiguousForced` (pin relaxation comparable to
/// the ramp; not trustworthily adiabatic, but the run still ran).
///
/// Per gate doc §SHAM and v3.1.3 §4.2, this is DATA carried in the
/// diagnostics row, never a hard error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdiabaticityCheck {
    /// `ratio < 0.1`.
    Acceptable { ratio: f64 },
    /// `ratio ≥ 0.1`.
    AmbiguousForced { ratio: f64 },
}

impl AdiabaticityCheck {
    /// Construct a verdict from a ratio per the v3.1.3 §4.2 threshold.
    pub fn from_ratio(ratio: f64) -> Self {
        if ratio >= 0.1 {
            Self::AmbiguousForced { ratio }
        } else {
            Self::Acceptable { ratio }
        }
    }

    /// Return the underlying ratio regardless of variant.
    pub fn ratio(&self) -> f64 {
        match self {
            Self::Acceptable { ratio } => *ratio,
            Self::AmbiguousForced { ratio } => *ratio,
        }
    }

    /// `true` iff the verdict is `Acceptable`.
    pub fn is_acceptable(&self) -> bool {
        matches!(self, Self::Acceptable { .. })
    }
}

// ── Public surface: error enum ────────────────────────────────────

/// Typed error surface for the LOOP_TRANSPORT verb.
///
/// Note: `AdiabaticityForcedAmbiguous` is intentionally NOT a variant
/// — per v3.1.3 §4.2 the forced-ambiguous case is data carried in
/// `LoopTransportDiagnostics::adiabaticity_check`, not a hard failure.
/// Putting it in this enum would force VI.3 to catch-and-unwrap on
/// every run with a slow pin.
#[derive(Debug, Clone, PartialEq)]
pub enum LoopTransportError {
    // ── Parser-rejection (raised before executor) ─────────────────
    /// `BETA_WILSON` (start or end of the ramp) falls outside the
    /// v3.1.3 §2 validated regime `[2.5, 3.0]`.
    BetaWilsonOutOfValidatedRegime { got: f64, range: (f64, f64) },
    /// `ALONG_LOOP loop_id`'s edge list does not close — last vertex
    /// ≠ first vertex. Audit-story flag per gate doc §SHAM.
    LoopNotClosed { tail: VertexId, head: VertexId },
    /// `loop_id` is not present in the loop registry.
    LoopNotRegistered { loop_id: String },
    /// The named lattice is not declared.
    LatticeNotRegistered { lattice_name: String },
    /// `N_DISCRETIZATION` outside the sanity window `[1, 10_000_000]`.
    NDiscretizationOutOfRange { got: usize, min: usize, max: usize },
    /// `SEEDS` bracket inverted (hi < lo) or empty.
    SeedBracketInvalid { lo: u64, hi: u64 },
    /// `SHAM { flag … }` block carries a flag the executor cannot
    /// dispatch. After VI.4, six science + audit-story flag names are
    /// recognized; everything else is rejected here.
    UnrecognizedShamFlag { name: String },
    /// `SHAM { FLAG = … }` argument did not validate per the flag's
    /// allowed-values list (e.g. `MASS_BASELINE_SCALED` requires
    /// μ ∈ {0.1, 1.0, 10.0} per v3.1.3 §5 S₃).
    InvalidShamArg {
        flag: String,
        expected: &'static str,
        got: String,
    },
    /// `CONTROL_MANIFOLD` anything other than `(Q, BETA_WILSON)`.
    UnsupportedControlManifold { got: String },

    // ── Executor-runtime (raised inside loop_transport) ──────────
    /// Group erasure: U handle is not SU(2). LOOP_TRANSPORT is SU(2)-
    /// only in VI; SU(3) ships a sibling kernel later.
    UnsupportedGroup(Group),
    /// The named gauge field U does not exist in the registry.
    UFieldNotDeclared(String),
    /// The companion E field is missing — LOOP_TRANSPORT reuses the
    /// `(U, E)` phase-space pair from SYMPLECTIC_FLOW.
    EFieldNotDeclared(String),
    /// Wrapped upstream gauge error (wilson_force, drift, project_gauss).
    Gauge(GaugeFieldError),
    /// Numerical non-finite at substep s.
    NonFiniteAtSubstep {
        seed: u64,
        substep: usize,
        what: &'static str,
    },
    /// Caller passed the wrong Statement variant to `loop_transport`.
    Internal(String),
}

impl From<GaugeFieldError> for LoopTransportError {
    fn from(e: GaugeFieldError) -> Self {
        Self::Gauge(e)
    }
}

impl std::fmt::Display for LoopTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use LoopTransportError::*;
        match self {
            BetaWilsonOutOfValidatedRegime { got, range } => write!(
                f,
                "BetaWilsonOutOfValidatedRegime: BETA_WILSON = {got} outside validated regime [{:.1}, {:.1}]",
                range.0, range.1
            ),
            LoopNotClosed { tail, head } => write!(
                f,
                "LoopNotClosed: loop's last vertex ({head}) does not match the first vertex ({tail})"
            ),
            LoopNotRegistered { loop_id } => {
                write!(f, "LoopNotRegistered: loop_id '{loop_id}' is not registered")
            }
            LatticeNotRegistered { lattice_name } => write!(
                f,
                "LatticeNotRegistered: lattice '{lattice_name}' is not declared"
            ),
            NDiscretizationOutOfRange { got, min, max } => write!(
                f,
                "NDiscretizationOutOfRange: N_DISCRETIZATION = {got} outside [{min}, {max}]"
            ),
            SeedBracketInvalid { lo, hi } => write!(
                f,
                "SeedBracketInvalid: SEEDS bracket [{lo}..{hi}] is empty or inverted"
            ),
            UnrecognizedShamFlag { name } => {
                write!(f, "UnrecognizedShamFlag: SHAM flag '{name}' is not recognized by the executor")
            }
            InvalidShamArg { flag, expected, got } => write!(
                f,
                "InvalidShamArg: SHAM flag '{flag}' expected {expected} but got {got}"
            ),
            UnsupportedControlManifold { got } => write!(
                f,
                "UnsupportedControlManifold: '{got}' (v3.1.3 freezes (Q, BETA_WILSON))"
            ),
            UnsupportedGroup(g) => write!(f, "UnsupportedGroup: {g:?}"),
            UFieldNotDeclared(n) => write!(f, "UFieldNotDeclared: gauge field '{n}' is not declared"),
            EFieldNotDeclared(n) => write!(f, "EFieldNotDeclared: E field '{n}' is not declared"),
            Gauge(e) => write!(f, "Gauge: {e:?}"),
            NonFiniteAtSubstep { seed, substep, what } => write!(
                f,
                "NonFiniteAtSubstep: seed={seed} substep={substep} field={what}"
            ),
            Internal(msg) => write!(f, "Internal: {msg}"),
        }
    }
}

impl std::error::Error for LoopTransportError {}

// ── Public surface: typed SHAM flags (VI.4) ───────────────────────

/// Typed view of the parser's `ShamBlock` after VI.4 dispatch.
///
/// Six recognized SHAM flags (5 science + 1 audit-story runtime); the
/// 7th flag — `OPEN_LOOP` — is enforced upstream at the VI.2 parser
/// entry as `LoopTransportError::LoopNotClosed`.
///
/// `default()` = all-off, which `is_all_off()` reports as true. The
/// executor's top-level dispatch routes the all-off case through the
/// pure VI.3 verb body byte-for-byte (the zero-cost-when-off contract
/// that protects the IV.10 gold fixture + VI.3 GC battery).
///
/// EVOLVING: the field set may grow when v3.1.3 §5 grows.
#[derive(Debug, Clone, Default)]
pub struct ShamFlags {
    /// `FLAT_FIELD` — κ_Q ≡ 0; freeze the parameter-space ramp.
    pub flat_field: bool,
    /// `ALPHA_ZERO` — `ALPHA_HALCYON = 0`; freeze the Halcyon clock.
    pub alpha_zero: bool,
    /// `MASS_BASELINE_SCALED` — override `MU_BASELINE` to the named
    /// canonical value. `None` = inactive; `Some(μ)` where μ ∈
    /// {0.1, 1.0, 10.0} per v3.1.3 §5 S₃.
    pub mu_baseline_scaled: Option<f64>,
    /// `DEGENERATE_LOOP` — substitute γ_unit with an out-and-back
    /// zero-area cycle.
    pub degenerate_loop: bool,
    /// `FROZEN_FIELD` — skip every `drift_step` so U is static.
    pub frozen_field: bool,
    /// `EMPTY_LOOP` — runtime short-circuit: zero substeps, H = 0
    /// byte-for-byte.
    pub empty_loop: bool,
}

impl ShamFlags {
    /// `true` iff every field is at its inactive default. Gates the
    /// executor's pure-vs-shammed dispatch.
    pub fn is_all_off(&self) -> bool {
        !self.flat_field
            && !self.alpha_zero
            && self.mu_baseline_scaled.is_none()
            && !self.degenerate_loop
            && !self.frozen_field
            && !self.empty_loop
    }

    /// Lift the parser's `ShamBlock` into the typed flags struct.
    ///
    /// Unknown flag names are rejected via `UnrecognizedShamFlag`
    /// (preserves VI.2's regression contract). `MASS_BASELINE_SCALED`
    /// requires a numeric argument in {0.1, 1.0, 10.0}; any other shape
    /// returns `InvalidShamArg`.
    pub fn from_block(block: &ShamBlock) -> Result<Self, LoopTransportError> {
        let mut out = ShamFlags::default();
        for (name, arg) in &block.flags {
            match name.to_ascii_uppercase().as_str() {
                "FLAT_FIELD" => out.flat_field = parse_bool_arg(arg, name)?,
                "ALPHA_ZERO" => out.alpha_zero = parse_bool_arg(arg, name)?,
                "MASS_BASELINE_SCALED" => {
                    out.mu_baseline_scaled = Some(parse_mu_arg(arg, name)?);
                }
                "DEGENERATE_LOOP" => out.degenerate_loop = parse_bool_arg(arg, name)?,
                "FROZEN_FIELD" => out.frozen_field = parse_bool_arg(arg, name)?,
                "EMPTY_LOOP" => out.empty_loop = parse_bool_arg(arg, name)?,
                // OPEN_LOOP is parser-rejected upstream at VI.2 entry
                // (LoopNotClosed). Everything else is unknown.
                _ => {
                    return Err(LoopTransportError::UnrecognizedShamFlag {
                        name: name.clone(),
                    })
                }
            }
        }
        Ok(out)
    }
}

/// Parse a boolean SHAM argument. Bare flags arrive as
/// `ShamArg::Bool(true)` from the parser; explicit
/// `FLAG = TRUE|FALSE` likewise arrives as `Bool(b)`.
fn parse_bool_arg(arg: &crate::parser::ShamArg, flag: &str) -> Result<bool, LoopTransportError> {
    use crate::parser::ShamArg;
    match arg {
        ShamArg::Bool(b) => Ok(*b),
        other => Err(LoopTransportError::InvalidShamArg {
            flag: flag.to_string(),
            expected: "TRUE or FALSE",
            got: format!("{other:?}"),
        }),
    }
}

/// Parse a `MASS_BASELINE_SCALED` argument. Must be one of the three
/// canonical baselines per v3.1.3 §5 S₃.
fn parse_mu_arg(arg: &crate::parser::ShamArg, flag: &str) -> Result<f64, LoopTransportError> {
    use crate::parser::ShamArg;
    match arg {
        ShamArg::Number(n) => {
            let n = *n;
            if n == 0.1 || n == 1.0 || n == 10.0 {
                Ok(n)
            } else {
                Err(LoopTransportError::InvalidShamArg {
                    flag: flag.to_string(),
                    expected: "0.1, 1.0, or 10.0 (canonical baseline)",
                    got: format!("{n}"),
                })
            }
        }
        other => Err(LoopTransportError::InvalidShamArg {
            flag: flag.to_string(),
            expected: "numeric μ ∈ {0.1, 1.0, 10.0}",
            got: format!("{other:?}"),
        }),
    }
}

// ── Public surface: executor entry point ──────────────────────────

/// Direction of one (seed, direction) run.
#[derive(Debug, Clone, Copy)]
enum Direction {
    Forward,
    Reversed,
}

/// Validated, destructured view of `Statement::LoopTransport`.
/// Internal to this module so the per-substep body has direct
/// (non-Option) field access without re-pattern-matching.
struct LtConfig<'a> {
    lattice: &'a str,
    loop_id: &'a str,
    #[allow(dead_code)]
    control_manifold: &'a ControlManifoldSpec,
    #[allow(dead_code)]
    adiabatic: bool,
    ramp_rate_q: f64,
    ramp_rate_beta_w: f64,
    #[allow(dead_code)]
    drive_omega: f64,
    #[allow(dead_code)]
    drive_f0: f64,
    n_discretization: usize,
    pin_lambda_q: f64,
    pin_lambda_beta_w: f64,
    #[allow(dead_code)]
    eps_q: f64,
    #[allow(dead_code)]
    eps_beta_w: f64,
    alpha_halcyon: f64,
    tau_0: f64,
    #[allow(dead_code)]
    beta_tau: f64,
    /// VI.4 activates this via `MASS_BASELINE_SCALED` sham dispatch.
    mu_baseline: f64,
    #[allow(dead_code)]
    k_spring: f64,
    #[allow(dead_code)]
    c_damp: f64,
    seeds: &'a SeedRange,
    compute: &'a [LoopTransportOutputId],
    #[allow(dead_code)]
    return_fields: &'a [LoopTransportReturnId],
    sham: &'a Option<ShamBlock>,
    /// VI.6b Fix #5 — actual BETA_WILSON_START threaded from the
    /// parser. The earlier convention hardcoded 2.75 in the executor's
    /// defensive validator, which masked legitimate out-of-regime
    /// configurations on the direct-executor path.
    beta_wilson_start: f64,
}

fn destructure<'a>(stmt: &'a Statement) -> Result<LtConfig<'a>, LoopTransportError> {
    match stmt {
        Statement::LoopTransport {
            lattice,
            loop_id,
            control_manifold,
            adiabatic,
            ramp_rate_q,
            ramp_rate_beta_w,
            drive_omega,
            drive_f0,
            n_discretization,
            pin_lambda_q,
            pin_lambda_beta_w,
            eps_q,
            eps_beta_w,
            alpha_halcyon,
            tau_0,
            beta_tau,
            mu_baseline,
            k_spring,
            c_damp,
            seeds,
            compute,
            return_fields,
            sham,
            beta_wilson_start,
        } => Ok(LtConfig {
            lattice: lattice.as_str(),
            loop_id: loop_id.as_str(),
            control_manifold,
            adiabatic: *adiabatic,
            ramp_rate_q: *ramp_rate_q,
            ramp_rate_beta_w: *ramp_rate_beta_w,
            drive_omega: *drive_omega,
            drive_f0: *drive_f0,
            n_discretization: *n_discretization,
            pin_lambda_q: *pin_lambda_q,
            pin_lambda_beta_w: *pin_lambda_beta_w,
            eps_q: *eps_q,
            eps_beta_w: *eps_beta_w,
            alpha_halcyon: *alpha_halcyon,
            tau_0: *tau_0,
            beta_tau: *beta_tau,
            mu_baseline: *mu_baseline,
            k_spring: *k_spring,
            c_damp: *c_damp,
            seeds,
            compute,
            return_fields,
            sham,
            beta_wilson_start: *beta_wilson_start,
        }),
        _ => Err(LoopTransportError::Internal(
            "loop_transport: expected Statement::LoopTransport".into(),
        )),
    }
}

/// Derived per-substep step size: `T_segment / N_DISCRETIZATION`, with
/// `T_segment = α_halcyon · τ_0` per the Halcyon clock (v3.1.3 §3).
fn dt_substep(cfg: &LtConfig<'_>) -> f64 {
    let t_segment = cfg.alpha_halcyon * cfg.tau_0;
    if cfg.n_discretization == 0 {
        return 0.0;
    }
    t_segment / (cfg.n_discretization as f64)
}

/// Validate β_W amplitude against the v3.1.3 §2 regime. Returns the
/// `(beta_start, beta_max_reached)` values or the rejection variant.
///
/// VI.6b Fix #5 — replaces open-chain endpoint extrapolation with
/// amplitude-based bound. The control-manifold loop CLOSES, so the
/// relevant max-β_W during traversal is bounded by the canonical
/// half-amplitude
///
///     amp = |RAMP_RATE_BETA_W| · TAU_0 / 4
///
/// (α-independent, N-independent — the loop period is the Halcyon
/// timescale TAU_0, NOT the integrator's α·τ_0 horizon). At canonical
/// (ramp=0.01, tau_0=1) this is 0.0025; the regime [2.5, 3.0] easily
/// contains β_start ± amp for any β_start ∈ [2.5, 3.0]. The
/// α-independence is required by v3.1.3 §3.6's dual α=1 / α=1000
/// calibration battery.
///
/// `beta_wilson_start` is threaded through from the parser via
/// `LtConfig`, so the smoke test's direct-executor path checks the
/// caller's actual launch coordinate rather than falling back to the
/// 2.75 midpoint (which previously masked legitimate out-of-regime
/// configurations on direct entry).
fn validate_beta_w(cfg: &LtConfig<'_>) -> Result<(f64, f64), LoopTransportError> {
    let beta_start = cfg.beta_wilson_start;
    let amp = cfg.ramp_rate_beta_w.abs() * cfg.tau_0 / 4.0_f64;
    let beta_max = beta_start + amp;
    let range = (2.5_f64, 3.0_f64);
    // Per VI.6b Fix #5: reject if β_start < 2.5 OR β_max > 3.0.
    // β_start above the regime trips the upper-bound rule because
    // β_max ≥ β_start. The lower extremum of the closed-loop
    // amplitude is not gated — PIN_LAMBDA_BETA_W clamps the actual β
    // to the ramp reference so the negative excursion is dominated
    // by f64 round-off rather than the open-chain ramp shape.
    if beta_start < range.0 {
        return Err(LoopTransportError::BetaWilsonOutOfValidatedRegime {
            got: beta_start,
            range,
        });
    }
    if beta_max > range.1 {
        return Err(LoopTransportError::BetaWilsonOutOfValidatedRegime {
            got: beta_max,
            range,
        });
    }
    Ok((beta_start, beta_max))
}

/// Reduce an SU(2) holonomy quaternion `(q0, q1, q2, q3)` to a real
/// scalar — the **signed Wilson-loop angle** θ/2 with axis-sign
/// convention.
///
/// # Reduction
///
/// ```text
///     h_scalar = sign(q1 + q2 + q3) · arccos(clamp(q0, -1, 1))
/// ```
///
/// with the boundary convention `sign(0) → +1` (so identity holonomy,
/// `(1, 0, 0, 0)`, maps unambiguously to `+1 · arccos(1) = 0.0`).
///
/// # Mathematical content
///
/// `arccos(q0)` recovers the unsigned half-rotation angle θ/2 from the
/// SU(2) parameterization q0 = cos(θ/2). The axis-sum `q1 + q2 + q3`
/// carries the direction of the rotation axis as a 3-vector projected
/// onto (1, 1, 1); its sign determines the orientation of the rotation
/// in the SU(2) double cover. The product is therefore the *signed*
/// Wilson-loop angle, **antisymmetric under SU(2) group inversion**:
///
/// Under g → g⁻¹ = (q0, −q1, −q2, −q3):
///   * `arccos(q0)` is preserved (q0 is even under inversion),
///   * `sign(q1 + q2 + q3)` flips,
///   * → `reduce_su2_to_scalar(g⁻¹) = −reduce_su2_to_scalar(g)`.
///
/// This is the antisymmetry that
/// `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3 §3.1`'s primary
/// observable `H_geom = ½ · (H[γ] − H[γ⁻¹])` requires to be
/// **structurally non-trivial under spatial loop reversal**. The
/// previous reduction `h_scalar = q0 = cos(θ/2)` was even in θ
/// (`q0(g) = q0(g⁻¹)`), forcing `H_geom ≡ 0` by construction — see
/// Halcyon team Finding #1 (FORWARD == REVERSED at thermalized state)
/// for the falsification chain that motivated this change.
///
/// # Convention parity
///
/// Matches the Halcyon-team Python reference `sign(Im tr) · arccos(Re
/// tr)` for a single SU(2) element via `Im(tr(g))/2 = sin(θ/2) ·
/// axis_component`. For the σ_z embedding used in GC₂ this reduces to
/// `sign(q3)`; for arbitrary axis orientations on the buckyball /
/// cubed-sphere lattices it generalizes to the axis-sum sign used
/// here. (GIGI's `q0` is already `cos(θ/2)` without the factor of 2
/// that Python's `Re(tr) = 2 cos(θ/2)` carries, so `arccos` here
/// returns the mathematical half-angle directly.)
///
/// # Call sites
///
/// Only invoked from `loop_transport`'s forward / reversed walk
/// (lines ~833 and ~1094 of this file). `symplectic_flow.rs` and the
/// IV.10 KDK measurement battery do **not** call this reducer, so
/// Part IV's bit-identity gold is unaffected by this convention.
///
/// # Audit trail
///
/// See the "Abelianized scalar projection convention (VI.6a)" section
/// of `theory/halcyon/HALCYON_PART_VI_IMPLEMENTATION_LOG.md` for the
/// full convention paragraph, the v3.1.3 §3 gate interpretation
/// (gates operate on `H_geom` abstractly; projection convention
/// determines numerical values but not gate logic), and the deliberate
/// VI.5 gold-fixture regen this change required.
fn reduce_su2_to_scalar(g: super::group_element::GroupElement) -> f64 {
    use super::group_element::GroupElement;
    match g {
        GroupElement::SU2 { q0, q1, q2, q3 } => {
            let theta = q0.clamp(-1.0, 1.0).acos();
            let axis_sum = q1 + q2 + q3;
            let sign = if axis_sum == 0.0 {
                1.0
            } else {
                axis_sum.signum()
            };
            sign * theta
        }
        _ => f64::NAN,
    }
}

/// Run one (seed, direction) pair. Hot-path: per-substep KDK body
/// calls concrete SU(2) kernels directly. The seed is currently echoed
/// only (no stochastic kernels at VI.2; VI.4's SHAM noise will consume
/// it).
#[allow(clippy::too_many_arguments)]
fn run_one_direction(
    cfg: &LtConfig<'_>,
    lat: &crate::lattice::Lattice,
    loop_edges: &[(EdgeId, EdgeOrientation)],
    u_arc: &Arc<std::sync::Mutex<SU2GaugeField>>,
    e_arc: &Arc<std::sync::Mutex<SU2EField>>,
    seed: u64,
    direction: Direction,
    beta_start: f64,
) -> Result<OneDirRun, LoopTransportError> {
    let edge_face_inc = build_edge_face_incidence(lat);
    let face_edges_cache = build_face_edges_cache(lat);
    let vertex_edge_inc = build_vertex_edge_incidence(lat);

    let n = cfg.n_discretization;
    let dt = dt_substep(cfg);
    let dt_half = 0.5_f64 * dt;
    let t_segment = (n as f64) * dt;

    // CC-LT-7 (HALCYON reply 2 §B.1, line 118): `γ⁻¹` is NOT a
    // separately declared loop. The substrate computes the reversed
    // walk by traversing `loop_edges` time-reversed in the executor.
    // The PARAMETER-SPACE ramp is the SAME in both directions —
    // h_forward and h_reversed differ in the SPATIAL loop traversal,
    // not the temporal ramp.
    //
    // Per H_geom = ½(H[γ] - H[γ⁻¹]) (REPLY_2 §3.1 line 25): for the
    // antisymmetric primary observable to be non-trivial under loop
    // reversal, H[γ⁻¹] must be the holonomy of the REVERSED loop on
    // the same final U state, not the holonomy of the same loop with
    // a reversed ramp.
    let walk_edges: Vec<(EdgeId, EdgeOrientation)> = match direction {
        Direction::Forward => loop_edges.to_vec(),
        Direction::Reversed => loop_edges
            .iter()
            .rev()
            .map(|&(eid, orient)| {
                let flipped = match orient {
                    EdgeOrientation::Forward => EdgeOrientation::Reverse,
                    EdgeOrientation::Reverse => EdgeOrientation::Forward,
                };
                (eid, flipped)
            })
            .collect(),
    };

    // Initial loop-closure holonomy (read once before any KDK step).
    // Kept for parity with the diagnostics surface; not used in the
    // h_scalar reduction per the spec definition H[γ] = q0(walk(γ, U_end)).
    let _h_start = {
        let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        walk_loop(lat, &walk_edges, &*u_guard)
    };

    let mut tracking_error_q = 0.0_f64;
    let mut tracking_error_beta_w = 0.0_f64;
    // VI.6 Fix #3 (Halcyon Finding #3 / LOCKED §Fix #3): τ_pin is the
    // INSTANTANEOUS gauge-relaxation timescale at the current state per
    // v3.1.3 §4.2. The executor publishes
    //     adiabaticity_ratio = max_substep(τ_pin) / T_segment.
    // τ_pin per substep = 1 / max(g_residual, 1e-12) read from the
    // ProjectGaussDiagnostics returned by project_gauss after each KDK.
    let mut max_tau_pin_substep = 0.0_f64;

    // VI.6 Fix #4 (Halcyon Finding #4 / LOCKED §Fix #4): tracking_error
    // is the L∞ deviation between actual (observable) coordinate and
    // its initial pinned reference over the loop, per v3.1.3 §4.2.
    //   q_actual_s     = q_surrogate(U_s, lat) / n_faces
    //   plaq_actual_s  = mean(plaquette_per_face(U_s, lat))
    //   q_drift        = |q_actual_s - q_initial|
    //   beta_drift     = |plaq_actual_s - plaq_initial| * 0.5
    // (β_W regime width [2.5, 3.0] = 0.5; plaquette range [0,1].)
    let n_faces_f = (lat.n_faces() as f64).max(1.0);
    let (q_initial, mean_plaq_initial) = {
        let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        let q_init = super::q_surrogate::q_surrogate(&*u_guard, lat)? / n_faces_f;
        let per_face = super::plaquette::plaquette_per_face(&*u_guard, lat)?;
        let plaq_init = if per_face.is_empty() {
            0.0
        } else {
            per_face.iter().sum::<f64>() / (per_face.len() as f64)
        };
        (q_init, plaq_init)
    };

    {
        let mut u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        let mut e_guard = e_arc.lock().expect("e field mutex poisoned");

        for s in 0..n {
            // Parameter-space coordinates at the start of this substep.
            // `q_t` is informational at VI.2 (Q rides through holonomy
            // accumulation as a phase prefactor handled in step 4 of
            // the design pseudocode; the gauge action only sees β_W).
            let phase = (s as f64) / (n as f64);
            let q_ref = cfg.ramp_rate_q * (phase * t_segment);
            let beta_ref = beta_start + cfg.ramp_rate_beta_w * (phase * t_segment);
            let q_t = q_ref;
            let beta_t = beta_ref;

            // ── KDK substep mirroring symplectic_flow.rs:293-330 ──
            //
            // K: F0 from U (current) at β_t.
            let f0 = wilson_force_per_edge(
                &*u_guard,
                lat,
                &edge_face_inc,
                &face_edges_cache,
                beta_t,
            )?;
            apply_force_kick(&mut *e_guard, &f0, dt_half)?;
            // D: U_new = exp(dt · g² · E) · U. g² = 4 / β at SU(2).
            let g2 = if beta_t.abs() > 0.0 { 4.0_f64 / beta_t } else { 0.0 };
            drift_step(&mut *u_guard, &*e_guard, dt, g2)?;
            // K: F1 from U (new) at β_t.
            let f1 = wilson_force_per_edge(
                &*u_guard,
                lat,
                &edge_face_inc,
                &face_edges_cache,
                beta_t,
            )?;
            apply_force_kick(&mut *e_guard, &f1, dt_half)?;
            // Per-substep Gauss projection (production-canonical).
            // VI.6 Fix #3: keep the ProjectGaussDiagnostics report so we
            // can read final_gauss_residual_inf and compute τ_pin =
            // 1 / max(g_residual, 1e-12) per substep.
            let gauss_report = project_gauss(
                &mut *e_guard,
                &*u_guard,
                lat,
                &vertex_edge_inc,
                ProjectGaussConfig::default(),
            )?;
            let g_residual = gauss_report.final_gauss_residual_inf.max(1e-12);
            let tau_pin_substep = 1.0_f64 / g_residual;
            if tau_pin_substep > max_tau_pin_substep {
                max_tau_pin_substep = tau_pin_substep;
            }

            // VI.6 Fix #4: Tracking error = L∞ deviation between the
            // OBSERVABLE coordinate (q_surrogate / mean plaquette) and
            // its initial pinned reference. Replaces the prior
            // pinned-vs-pinned (q_t - q_ref) comparison, which was
            // identically zero by construction and forced
            // tracking_error_{q,bw} = 0.0 regardless of substrate
            // motion. Echo the params still for the finite-check.
            let _ = (q_t, q_ref, beta_t, beta_ref);
            let q_actual = super::q_surrogate::q_surrogate(&*u_guard, lat)? / n_faces_f;
            let q_drift = (q_actual - q_initial).abs();
            let plaq_actual = {
                let per_face = super::plaquette::plaquette_per_face(&*u_guard, lat)?;
                if per_face.is_empty() {
                    0.0
                } else {
                    per_face.iter().sum::<f64>() / (per_face.len() as f64)
                }
            };
            // β_W regime width 0.5 over [2.5, 3.0]; plaquette range
            // [0,1]; map plaquette drift onto the β_W regime scale.
            let beta_drift = (plaq_actual - mean_plaq_initial).abs() * 0.5;
            if !q_drift.is_finite() {
                return Err(LoopTransportError::NonFiniteAtSubstep {
                    seed,
                    substep: s,
                    what: "tracking_error_q",
                });
            }
            if !beta_drift.is_finite() {
                return Err(LoopTransportError::NonFiniteAtSubstep {
                    seed,
                    substep: s,
                    what: "tracking_error_beta_w",
                });
            }
            if q_drift > tracking_error_q {
                tracking_error_q = q_drift;
            }
            if beta_drift > tracking_error_beta_w {
                tracking_error_beta_w = beta_drift;
            }
        }
    }

    // Final loop-closure holonomy + reduce to real scalar.
    //
    // Per VI.3 GC₂ alignment with the spec text (HALCYON_PART_VI_GATES.md
    // §GC₂: "verify H[γ] = F₀ · Area(γ)" + REPLY_2 line 167:
    // "H_geom = ½(H[γ_unit] − H[γ_unit⁻¹])"), H[γ] is the holonomy of
    // the spatial loop γ on the post-flow U, NOT the change between
    // start- and end-of-segment loop holonomies. The previous
    // `h_end · h_start^-1` form vanishes identically on a static
    // connection and would make GC₂'s area-law contract untestable.
    let h_end = {
        let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        walk_loop(lat, &walk_edges, &*u_guard)
    };
    let h_scalar = reduce_su2_to_scalar(h_end);
    if !h_scalar.is_finite() {
        return Err(LoopTransportError::NonFiniteAtSubstep {
            seed,
            substep: n,
            what: "h_scalar",
        });
    }

    // Echo the `compute` flags by reference so the smoke test's
    // COMPUTE clauses are not erased (informational only at VI.2).
    let _ = cfg.compute;

    Ok(OneDirRun {
        h_scalar,
        tracking_error_q,
        tracking_error_bw: tracking_error_beta_w,
        max_tau_pin: max_tau_pin_substep,
    })
}

/// Shammed sibling of `run_one_direction`. Same KDK skeleton, but with
/// per-flag conditional branches woven in. The pure path is NOT touched
/// — `run_one_direction` is the byte-for-byte VI.3 verb body that gates
/// the IV.10 gold fixture + VI.3 GC battery inheritance.
///
/// EMPTY_LOOP short-circuits at the TOP of this function (before any
/// cache construction) so the verb-returns-H=0-byte-for-byte contract
/// is mechanical, not numerical.
#[allow(clippy::too_many_arguments)]
fn run_one_direction_shammed(
    cfg: &LtConfig<'_>,
    flags: &ShamFlags,
    lat: &crate::lattice::Lattice,
    loop_edges: &[(EdgeId, EdgeOrientation)],
    u_arc: &Arc<std::sync::Mutex<SU2GaugeField>>,
    e_arc: &Arc<std::sync::Mutex<SU2EField>>,
    seed: u64,
    direction: Direction,
    beta_start: f64,
) -> Result<OneDirRun, LoopTransportError> {
    // EMPTY_LOOP: integrator runs zero substeps; return literal +0.0
    // for every f64 field. Per design.per_flag_implementation, this
    // short-circuit happens BEFORE any cache construction so the verb
    // is genuinely a no-op on the hot path.
    if flags.empty_loop {
        return Ok(OneDirRun {
            h_scalar: 0.0,
            tracking_error_q: 0.0,
            tracking_error_bw: 0.0,
            max_tau_pin: 0.0,
        });
    }

    // ALPHA_ZERO: override α_halcyon to 0.0 BEFORE computing dt. Every
    // apply_force_kick and drift_step then runs with dt = 0 (no-ops on
    // E and lie_exp(0) = I on U).
    let alpha_eff = if flags.alpha_zero {
        0.0
    } else {
        cfg.alpha_halcyon
    };

    // MASS_BASELINE_SCALED: echo the overridden μ in a local. The
    // substrate does NOT itself compute baseline-subtracted H — that's
    // the orchestrator's job per v3.1.3 §5 S₃. We honor the requested
    // μ and let the orchestrator do the subtraction over runs.
    let _mu_eff = flags.mu_baseline_scaled.unwrap_or(cfg.mu_baseline);

    // DEGENERATE_LOOP: substitute γ_unit with a zero-area out-and-back
    // loop along the first edge of the registered loop. walk_loop on
    // an out-and-back returns the SU(2) identity.
    let effective_loop_edges: Vec<(EdgeId, EdgeOrientation)> = if flags.degenerate_loop {
        if let Some(&(eid, orient)) = loop_edges.first() {
            let flipped = match orient {
                EdgeOrientation::Forward => EdgeOrientation::Reverse,
                EdgeOrientation::Reverse => EdgeOrientation::Forward,
            };
            vec![(eid, orient), (eid, flipped)]
        } else {
            Vec::new()
        }
    } else {
        loop_edges.to_vec()
    };
    let loop_edges_ref: &[(EdgeId, EdgeOrientation)] = &effective_loop_edges;

    let edge_face_inc = build_edge_face_incidence(lat);
    let face_edges_cache = build_face_edges_cache(lat);
    let vertex_edge_inc = build_vertex_edge_incidence(lat);

    let n = cfg.n_discretization;
    // Recompute dt with the (possibly overridden) α_halcyon.
    let dt = {
        let t_segment = alpha_eff * cfg.tau_0;
        if n == 0 {
            0.0
        } else {
            t_segment / (n as f64)
        }
    };
    let dt_half = 0.5_f64 * dt;
    let t_segment = (n as f64) * dt;

    // CC-LT-7 (HALCYON reply 2 §B.1, line 118): `γ⁻¹` is NOT a
    // separately declared loop. The substrate computes the reversed
    // walk by traversing `loop_edges` time-reversed in the executor.
    // The PARAMETER-SPACE ramp is the SAME in both directions —
    // h_forward and h_reversed differ in the SPATIAL loop traversal,
    // not the temporal ramp.
    //
    // Per H_geom = ½(H[γ] - H[γ⁻¹]) (REPLY_2 §3.1 line 25): for the
    // antisymmetric primary observable to be non-trivial under loop
    // reversal, H[γ⁻¹] must be the holonomy of the REVERSED loop on
    // the same final U state, not the holonomy of the same loop with
    // a reversed ramp.
    let walk_edges: Vec<(EdgeId, EdgeOrientation)> = match direction {
        Direction::Forward => loop_edges_ref.to_vec(),
        Direction::Reversed => loop_edges_ref
            .iter()
            .rev()
            .map(|&(eid, orient)| {
                let flipped = match orient {
                    EdgeOrientation::Forward => EdgeOrientation::Reverse,
                    EdgeOrientation::Reverse => EdgeOrientation::Forward,
                };
                (eid, flipped)
            })
            .collect(),
    };

    // Initial loop-closure holonomy (read once before any KDK step).
    let _h_start = {
        let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        walk_loop(lat, &walk_edges, &*u_guard)
    };

    let mut tracking_error_q = 0.0_f64;
    let mut tracking_error_beta_w = 0.0_f64;
    // VI.6 Fix #3 (shammed path): same per-substep τ_pin measurement
    // as run_one_direction. SHAM flags rotate the Gauss residual in
    // ways the orchestrator may want to detect (e.g. FROZEN_FIELD
    // bypasses drift but project_gauss still cleans the K-step E
    // dirt), so the measurement is meaningful in the shammed arm too.
    let mut max_tau_pin_substep = 0.0_f64;

    // VI.6 Fix #4 (shammed path): same observable-drift tracking as
    // run_one_direction. FROZEN_FIELD will pin q and plaquette at
    // q_initial / plaq_initial (U never moves) so tracking_error stays
    // ~0 there — that's the desired sham signature. FLAT_FIELD pins
    // the parameter ramp but lets U drift under wilson_force at fixed
    // β, so observable drift is genuine.
    let n_faces_f = (lat.n_faces() as f64).max(1.0);
    let (q_initial, mean_plaq_initial) = {
        let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        let q_init = super::q_surrogate::q_surrogate(&*u_guard, lat)? / n_faces_f;
        let per_face = super::plaquette::plaquette_per_face(&*u_guard, lat)?;
        let plaq_init = if per_face.is_empty() {
            0.0
        } else {
            per_face.iter().sum::<f64>() / (per_face.len() as f64)
        };
        (q_init, plaq_init)
    };

    {
        let mut u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        let mut e_guard = e_arc.lock().expect("e field mutex poisoned");

        for s in 0..n {
            // FLAT_FIELD: freeze the parameter-space ramp — κ_Q ≡ 0,
            // β stays pinned at beta_start. Otherwise: standard ramp.
            let phase = (s as f64) / (n as f64);
            let (q_ref, beta_ref, q_t, beta_t) = if flags.flat_field {
                (0.0_f64, beta_start, 0.0_f64, beta_start)
            } else {
                let q_ref = cfg.ramp_rate_q * (phase * t_segment);
                let beta_ref = beta_start + cfg.ramp_rate_beta_w * (phase * t_segment);
                (q_ref, beta_ref, q_ref, beta_ref)
            };

            // ── KDK substep ──
            let f0 = wilson_force_per_edge(
                &*u_guard,
                lat,
                &edge_face_inc,
                &face_edges_cache,
                beta_t,
            )?;
            apply_force_kick(&mut *e_guard, &f0, dt_half)?;
            // FROZEN_FIELD: skip the drift step so U is static. The K
            // halves still run (E evolves under the static force) but
            // U never moves, so walk_loop reads the cold-start U.
            let g2 = if beta_t.abs() > 0.0 { 4.0_f64 / beta_t } else { 0.0 };
            if !flags.frozen_field {
                drift_step(&mut *u_guard, &*e_guard, dt, g2)?;
            }
            let f1 = wilson_force_per_edge(
                &*u_guard,
                lat,
                &edge_face_inc,
                &face_edges_cache,
                beta_t,
            )?;
            apply_force_kick(&mut *e_guard, &f1, dt_half)?;
            // VI.6 Fix #3 (shammed path): keep ProjectGaussDiagnostics
            // so τ_pin = 1 / max(g_residual, 1e-12) is measured.
            let gauss_report = project_gauss(
                &mut *e_guard,
                &*u_guard,
                lat,
                &vertex_edge_inc,
                ProjectGaussConfig::default(),
            )?;
            let g_residual = gauss_report.final_gauss_residual_inf.max(1e-12);
            let tau_pin_substep = 1.0_f64 / g_residual;
            if tau_pin_substep > max_tau_pin_substep {
                max_tau_pin_substep = tau_pin_substep;
            }

            // VI.6 Fix #4 (shammed path): observable drift, same shape
            // as the pure path. Echo the param refs so they aren't dead.
            let _ = (q_t, q_ref, beta_t, beta_ref);
            let q_actual = super::q_surrogate::q_surrogate(&*u_guard, lat)? / n_faces_f;
            let q_drift = (q_actual - q_initial).abs();
            let plaq_actual = {
                let per_face = super::plaquette::plaquette_per_face(&*u_guard, lat)?;
                if per_face.is_empty() {
                    0.0
                } else {
                    per_face.iter().sum::<f64>() / (per_face.len() as f64)
                }
            };
            let beta_drift = (plaq_actual - mean_plaq_initial).abs() * 0.5;
            if !q_drift.is_finite() {
                return Err(LoopTransportError::NonFiniteAtSubstep {
                    seed,
                    substep: s,
                    what: "tracking_error_q",
                });
            }
            if !beta_drift.is_finite() {
                return Err(LoopTransportError::NonFiniteAtSubstep {
                    seed,
                    substep: s,
                    what: "tracking_error_beta_w",
                });
            }
            if q_drift > tracking_error_q {
                tracking_error_q = q_drift;
            }
            if beta_drift > tracking_error_beta_w {
                tracking_error_beta_w = beta_drift;
            }
        }
    }

    let h_end = {
        let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        walk_loop(lat, &walk_edges, &*u_guard)
    };
    let h_scalar = reduce_su2_to_scalar(h_end);
    if !h_scalar.is_finite() {
        return Err(LoopTransportError::NonFiniteAtSubstep {
            seed,
            substep: n,
            what: "h_scalar",
        });
    }

    let _ = cfg.compute;

    Ok(OneDirRun {
        h_scalar,
        tracking_error_q,
        tracking_error_bw: tracking_error_beta_w,
        max_tau_pin: max_tau_pin_substep,
    })
}

/// Per-direction run result.
#[derive(Debug, Clone)]
struct OneDirRun {
    h_scalar: f64,
    tracking_error_q: f64,
    tracking_error_bw: f64,
    /// VI.6 Fix #3: max over substeps of τ_pin = 1/max(g_residual, 1e-12)
    /// read from project_gauss's ProjectGaussDiagnostics after each KDK.
    /// Aggregated at the executor site as max over (direction, seed) and
    /// divided by T_segment for the v3.1.3 §4.2 adiabaticity_ratio.
    max_tau_pin: f64,
}

/// Snapshot the current U + E buffers so the reversed-direction run
/// starts from the same initial state as the forward run.
fn snapshot_u_e(
    u_arc: &Arc<std::sync::Mutex<SU2GaugeField>>,
    e_arc: &Arc<std::sync::Mutex<SU2EField>>,
) -> (SU2GaugeField, SU2EField) {
    let u_snap = u_arc.lock().expect("su2 field mutex poisoned").clone();
    let e_snap = e_arc.lock().expect("e field mutex poisoned").clone();
    (u_snap, e_snap)
}

/// Restore the U + E buffers from a snapshot in place.
fn restore_u_e(
    u_arc: &Arc<std::sync::Mutex<SU2GaugeField>>,
    e_arc: &Arc<std::sync::Mutex<SU2EField>>,
    u_snap: &SU2GaugeField,
    e_snap: &SU2EField,
) {
    {
        let mut u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        *u_guard = u_snap.clone();
    }
    {
        let mut e_guard = e_arc.lock().expect("e field mutex poisoned");
        *e_guard = e_snap.clone();
    }
}

/// Mean of a slice. Empty → 0.0.
fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f64>() / (v.len() as f64)
}

/// Simple block σ — sd of the combined per-seed chain. VI.3 swaps in
/// the v3.1.3 §3.2 block estimator; VI.2 ships this single-block
/// fallback so the diagnostics shape is correct.
fn block_sigma(fwd: &[f64], rev: &[f64]) -> f64 {
    let mut combined: Vec<f64> = Vec::with_capacity(fwd.len() + rev.len());
    combined.extend_from_slice(fwd);
    combined.extend_from_slice(rev);
    if combined.len() < 2 {
        return 0.0;
    }
    let m = mean(&combined);
    let var: f64 = combined.iter().map(|x| (x - m) * (x - m)).sum::<f64>()
        / ((combined.len() - 1) as f64);
    var.sqrt()
}

/// Run the LOOP_TRANSPORT verb end-to-end. Returns a populated
/// `LoopTransportDiagnostics`; the parser executor arm lowers it into
/// a `Rows` envelope. Public surface VI.3/4/5 will test against.
pub fn loop_transport(
    stmt: &Statement,
    u_name: &str,
    e_name: &str,
) -> Result<LoopTransportDiagnostics, LoopTransportError> {
    let cfg = destructure(stmt)?;

    // Defensive: re-check the regime even though the parser arm
    // already validated. The smoke test calls this directly so the
    // executor surface must be safe on its own.
    let (beta_start, _beta_end) = validate_beta_w(&cfg)?;

    // SHAM dispatch (VI.4). Empty block → ShamFlags::default() →
    // is_all_off() = true → routes through the pure VI.3 verb body
    // byte-for-byte. Unknown flag names rejected with
    // UnrecognizedShamFlag (preserves VI.2's regression contract).
    let sham_flags: ShamFlags = match cfg.sham.as_ref() {
        Some(block) => ShamFlags::from_block(block)?,
        None => ShamFlags::default(),
    };

    // N_DISCRETIZATION sanity window.
    let n_min = 1_usize;
    let n_max = 10_000_000_usize;
    if cfg.n_discretization < n_min || cfg.n_discretization > n_max {
        return Err(LoopTransportError::NDiscretizationOutOfRange {
            got: cfg.n_discretization,
            min: n_min,
            max: n_max,
        });
    }
    // SEEDS bracket sanity.
    if cfg.seeds.hi < cfg.seeds.lo {
        return Err(LoopTransportError::SeedBracketInvalid {
            lo: cfg.seeds.lo,
            hi: cfg.seeds.hi,
        });
    }

    // CONTROL_MANIFOLD freeze: v3.1.3 only ships (Q, BETA_WILSON).
    // The Statement variant carries `ControlManifoldSpec::QBetaWilson`
    // as the only constructor today; the match is here for future
    // forward-compat when broader manifolds land.
    match cfg.control_manifold {
        ControlManifoldSpec::QBetaWilson => {}
    }

    // Lattice + loop resolution.
    let lat = lattice_registry::get(cfg.lattice).ok_or_else(|| {
        LoopTransportError::LatticeNotRegistered {
            lattice_name: cfg.lattice.to_string(),
        }
    })?;
    let registered = get_loop(cfg.loop_id).ok_or_else(|| LoopTransportError::LoopNotRegistered {
        loop_id: cfg.loop_id.to_string(),
    })?;
    if !registered.is_closed() {
        if let Some((tail, head)) = registered.endpoints() {
            return Err(LoopTransportError::LoopNotClosed { tail, head });
        }
    }
    let loop_edges = registered.edges.clone();

    // U + E handle resolution.
    let u_handle = super::registry::get(u_name)
        .ok_or_else(|| LoopTransportError::UFieldNotDeclared(u_name.to_string()))?;
    if !matches!(u_handle.group(), Group::SU2) {
        return Err(LoopTransportError::UnsupportedGroup(u_handle.group()));
    }
    drop(u_handle);
    let u_arc = get_su2_mut(u_name)
        .ok_or_else(|| LoopTransportError::UFieldNotDeclared(u_name.to_string()))?;
    let e_arc = get_su2_e_mut(e_name)
        .ok_or_else(|| LoopTransportError::EFieldNotDeclared(e_name.to_string()))?;

    // Per-seed forward + reversed runs.
    let seed_list: Vec<u64> = (cfg.seeds.lo..=cfg.seeds.hi).collect();
    let mut per_seed_h_forward = Vec::with_capacity(seed_list.len());
    let mut per_seed_h_reversed = Vec::with_capacity(seed_list.len());
    let mut max_track_q = 0.0_f64;
    let mut max_track_bw = 0.0_f64;
    // VI.6 Fix #3: aggregate max τ_pin over both directions and all seeds.
    let mut max_tau_pin_all = 0.0_f64;

    let all_off = sham_flags.is_all_off();
    for &seed in &seed_list {
        // Snapshot (forward run mutates U + E in place; reversed run
        // restarts from the same point so the diagnostics carry
        // direction-symmetric data).
        let (u_snap, e_snap) = snapshot_u_e(&u_arc, &e_arc);

        let fwd = if all_off {
            run_one_direction(
                &cfg,
                &lat,
                &loop_edges,
                &u_arc,
                &e_arc,
                seed,
                Direction::Forward,
                beta_start,
            )?
        } else {
            run_one_direction_shammed(
                &cfg,
                &sham_flags,
                &lat,
                &loop_edges,
                &u_arc,
                &e_arc,
                seed,
                Direction::Forward,
                beta_start,
            )?
        };
        restore_u_e(&u_arc, &e_arc, &u_snap, &e_snap);

        let rev = if all_off {
            run_one_direction(
                &cfg,
                &lat,
                &loop_edges,
                &u_arc,
                &e_arc,
                seed,
                Direction::Reversed,
                beta_start,
            )?
        } else {
            run_one_direction_shammed(
                &cfg,
                &sham_flags,
                &lat,
                &loop_edges,
                &u_arc,
                &e_arc,
                seed,
                Direction::Reversed,
                beta_start,
            )?
        };
        restore_u_e(&u_arc, &e_arc, &u_snap, &e_snap);

        per_seed_h_forward.push(fwd.h_scalar);
        per_seed_h_reversed.push(rev.h_scalar);
        if fwd.tracking_error_q > max_track_q {
            max_track_q = fwd.tracking_error_q;
        }
        if rev.tracking_error_q > max_track_q {
            max_track_q = rev.tracking_error_q;
        }
        if fwd.tracking_error_bw > max_track_bw {
            max_track_bw = fwd.tracking_error_bw;
        }
        if rev.tracking_error_bw > max_track_bw {
            max_track_bw = rev.tracking_error_bw;
        }
        // VI.6 Fix #3: roll max τ_pin over both directions + all seeds.
        if fwd.max_tau_pin > max_tau_pin_all {
            max_tau_pin_all = fwd.max_tau_pin;
        }
        if rev.max_tau_pin > max_tau_pin_all {
            max_tau_pin_all = rev.max_tau_pin;
        }
    }

    // Republish post-flow U snapshot (matches symplectic_flow's
    // post-flow republish so any downstream `gauge_registry::get`
    // sees the freshest snapshot).
    {
        let final_u = u_arc.lock().expect("su2 field mutex poisoned").clone();
        register_su2(final_u);
    }

    // Blocked aggregates.
    let h_forward = mean(&per_seed_h_forward);
    let h_reversed = mean(&per_seed_h_reversed);
    let sigma_h_blocked = block_sigma(&per_seed_h_forward, &per_seed_h_reversed);

    // Adiabaticity verdict per v3.1.3 §4.2.
    //
    // VI.6 Fix #3 (Halcyon Finding #3): τ_pin is the INSTANTANEOUS
    // gauge-relaxation timescale, measured per substep as
    // 1 / max(final_gauss_residual_inf, 1e-12) after each project_gauss
    // call, aggregated as max over (direction, seed, substep). The old
    // static formula τ_pin = 1 / min(PIN_LAMBDA_Q, PIN_LAMBDA_BETA_W)
    // = 1.0 forced AmbiguousForced on every call regardless of state.
    //
    // When max_tau_pin_all is still 0.0 (e.g. EMPTY_LOOP sham
    // short-circuit with N=0 substeps actually executed), the verdict
    // is reported as the small-τ_pin → small-ratio limit (Acceptable).
    let t_segment = (cfg.n_discretization as f64) * dt_substep(&cfg);
    let ratio = if t_segment.abs() > 0.0 {
        max_tau_pin_all / t_segment
    } else {
        f64::INFINITY
    };
    let adiab = AdiabaticityCheck::from_ratio(ratio);

    Ok(LoopTransportDiagnostics {
        h_forward,
        h_reversed,
        sigma_h_blocked,
        per_seed_h_forward,
        per_seed_h_reversed,
        tracking_error_max_q: max_track_q,
        tracking_error_max_beta_w: max_track_bw,
        adiabaticity_check: adiab,
        seeds_used: seed_list,
        n_substeps_completed: if sham_flags.empty_loop {
            0
        } else {
            cfg.n_discretization
        },
    })
}

// Silence unused-import warnings when nothing in the body currently
// names these — they're load-bearing for the executor's reuse contract.
#[allow(dead_code)]
type _GaugeFieldHandleHint = dyn GaugeFieldHandle;

#[cfg(test)]
mod tests {
    use super::*;

    /// `AdiabaticityCheck::from_ratio` agrees with the v3.1.3 §4.2
    /// threshold (ratio < 0.1 → Acceptable; ≥ 0.1 → forced).
    #[test]
    fn adiabaticity_threshold_at_0_1() {
        assert!(matches!(
            AdiabaticityCheck::from_ratio(0.05),
            AdiabaticityCheck::Acceptable { .. }
        ));
        assert!(matches!(
            AdiabaticityCheck::from_ratio(0.1),
            AdiabaticityCheck::AmbiguousForced { .. }
        ));
        assert!(matches!(
            AdiabaticityCheck::from_ratio(1.0),
            AdiabaticityCheck::AmbiguousForced { .. }
        ));
    }
}
