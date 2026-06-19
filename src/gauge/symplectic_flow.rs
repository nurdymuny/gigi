//! `SYMPLECTIC_FLOW` — KDK leapfrog over `n_steps` with per-step Gauss
//! projection.
//!
//! Closes TDD-HAL-IV.6. Composes the four Part-IV primitives
//! (IV.2 covariant Gauss, IV.3 PROJECT_GAUSS, IV.4 wilson_force +
//! apply_force_kick, IV.5 drift_step) into the production-canonical
//! symplectic flow Halcyon's mass-gap chapter consumes. Mirrors
//! `davis-wilson-lattice/inertia_damping/buckyball_integrator.py::
//! leapfrog_step` line-for-line.
//!
//! ── Algorithm (KDK leapfrog with per-step Gauss projection) ──
//!
//! For each step `s ∈ 0..n_steps`:
//!
//!   F0 = wilson_force_per_edge(U, lat, edge_face_inc, beta);
//!   apply_force_kick(&mut E, &F0, dt/2);     // K — q0=0 enforced
//!   drift_step(&mut U, &E, dt, g2);          // D — U_new = exp(dt·g²·E)·U
//!   F1 = wilson_force_per_edge(U_new, lat, edge_face_inc, beta);
//!   apply_force_kick(&mut E, &F1, dt/2);     // K — q0=0 enforced
//!   if config.project_gauss.is_some() {
//!       project_gauss(&mut E, U, lat, vertex_edge_inc, cfg)?;  // per step
//!   }
//!   if measure_every > 0 && (s+1) % measure_every == 0 {
//!       for obs in &config.measure { history[obs].push(observe(...)) }
//!   }
//!
//! The KDK order — F0 from U BEFORE the drift, F1 from U AFTER the
//! drift — is what makes the leapfrog second-order symplectic. Bee's
//! locked decision IV-D pins the per-step projection cadence; the
//! configurable cadence knob is P1 future-tense.
//!
//! ── Observable battery (reverses III.5 stub) ──
//!
//! `MEASURE` admits the full battery the parser declares. III.5 erred
//! on HTotal/GaussResidualMax/EdgeKinetic/VertexGauss/Energy with
//! `PartIvObservableNotReady`. IV.6 wires the dispatch table to real
//! E-aware computations:
//!
//!   HTotal           → (1/2)|E|² + S_Wilson(U, β)
//!   GaussResidualMax → max_inf_norm(compute_gauss_residual_covariant(U, E))
//!   EdgeKinetic      → (1/2) · Σ_e ||E[e]||² / n_edges
//!   VertexGauss      → mean over vertices of ||G_cov[v]||
//!   Energy           → S_Wilson(U, β)
//!   MeanPlaquette    → reuse III.1 plaquette_mean
//!   QSurrogate       → reuse III.2 q_surrogate
//!
//! S_Wilson is `(β/N) · Σ_f (1 - Re Tr U_f / N) = β · F · (1 - ⟨P⟩)`
//! for SU(2) (N=2, Re Tr / 2 = q0). The constant offset cancels in
//! every `ΔH` measurement that matters for the energy-drift gate.
//!
//! ── Group-erasure escape ──
//!
//! Mirrors III.5's pattern but doubles up: holds dual handles
//! `Arc<Mutex<SU2GaugeField>>` + `Arc<Mutex<SU2EField>>` via
//! `get_su2_mut` + `get_su2_e_mut`. The epilogue calls
//! `republish_su2(u_name, u_arc)` so the dyn read map stays
//! post-mutation coherent on the U side. The SU(2) E sibling registry
//! already stores `Arc<Mutex<…>>` (the same handle the flow holds), so
//! no separate publish step is needed on the E side. No WAL op (mirrors
//! III.5 GIBBS_SAMPLE non-persistence). Future SU(3) heatbath + flow
//! will ship parallel sibling kernels with their own `symplectic_flow_su3`.
//!
//! Reference: `davis-wilson-lattice/inertia_damping/
//! buckyball_integrator.py::leapfrog_step` (the KDK body) +
//! `buckyball_action.py::compute_hamiltonian` (the H_total reduction).

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

use super::e_field::SU2EField;
use super::error::GaugeFieldError;
use super::gauss::{
    build_vertex_edge_incidence, compute_gauss_residual_covariant, max_inf_norm,
    VertexEdgeIncidence,
};
use super::gibbs_sample::ObservableId;
use super::lie_exp::drift_step;
use super::plaquette::plaquette_mean;
use super::project_gauss::{project_gauss, ProjectGaussConfig};
use super::q_surrogate::q_surrogate;
use super::registry::{get_su2_e_mut, get_su2_mut, republish_su2, GaugeFieldHandle};
use super::staple::{build_edge_face_incidence, build_face_edges_cache, EdgeFaceIncidence};
use super::su2_gauge_field::SU2GaugeField;
use super::wilson_force::{apply_force_kick, wilson_force_per_edge};
use crate::lattice::{registry as lattice_registry, Lattice};

/// Configuration for one `SYMPLECTIC_FLOW` call.
///
/// `project_gauss` is `Some(cfg)` when per-step Gauss projection is
/// enabled (the production-canonical default; pass
/// `ProjectGaussConfig::default()` for Halcyon defaults per IV-A) and
/// `None` to skip projection entirely (used by the IV.6 "residual grows
/// without projection" red test and by future debugging probes).
///
/// `measure_every = 0` disables measurement entirely; otherwise the
/// epilogue measures at step `s` when `(s+1) % measure_every == 0`.
#[derive(Debug, Clone)]
pub struct SymplecticFlowConfig {
    pub beta: f64,
    pub dt: f64,
    pub n_steps: usize,
    pub project_gauss: Option<ProjectGaussConfig>,
    pub measure_every: usize,
    pub measure: Vec<ObservableId>,
}

/// Per-call diagnostics from one `symplectic_flow` invocation.
#[derive(Debug, Clone)]
pub struct SymplecticFlowDiagnostics {
    /// Seed actually used (echoes the caller's argument).
    pub seed: Option<u64>,
    /// β echoed from the config.
    pub beta: f64,
    /// dt echoed from the config.
    pub dt: f64,
    /// Number of KDK steps the loop completed.
    pub n_steps_completed: usize,
    /// 99th-percentile CG iterations per step across the projection
    /// chain. DIAGNOSTIC ONLY — explicitly excluded from every A2 row
    /// (see PART_IV_GATES.md). When no projection ran (`project_gauss
    /// = None`), this is `0.0`.
    pub cg_iterations_per_step_p99: f64,
    /// `max_i |H[i] - H[0]| / |H[0]|` over the measurement chain when
    /// `HTotal` is in `measure`; `0.0` when HTotal was not measured.
    /// Used by the IV.10 gold gate's acceptance bound (< 1e-3).
    pub max_energy_drift_rel: f64,
    /// `max_inf_norm(G_cov(U, E))` at the end of the flow. Used by the
    /// "per-step projection holds the residual" red test.
    pub gauss_residual_max: f64,
}

/// JSON-shaped response from `symplectic_flow`. Mirrors the
/// `GibbsSampleResponse` shape (`field`, `measurement_history`,
/// `diagnostics`) plus the E-field name on a sibling slot.
#[derive(Debug, Clone)]
pub struct SymplecticFlowResponse {
    /// Server-generated stable id for this run. Populated unconditionally
    /// by `symplectic_flow` and used as the key for the process-local
    /// diagnostics LRU (`GET /v1/symplectic_flow/diagnostics/{run_id}`).
    /// UUID v4 string. Stable across re-reads of the same response.
    pub run_id: String,
    /// Echoes the `u_name` argument.
    pub field: String,
    /// Echoes the `e_name` argument.
    pub e_field: String,
    /// Per-observable measurement chain. Length per observable is
    /// `n_steps / measure_every` (only steps `s` with `(s+1) %
    /// measure_every == 0` push). `measure_every = 0` → empty map.
    pub measurement_history: HashMap<ObservableId, Vec<f64>>,
    /// Flow diagnostics block.
    pub diagnostics: SymplecticFlowDiagnostics,
}

// ── Process-local diagnostics LRU ──────────────────────────────────
//
// Capacity 32. The HTTP `GET /v1/symplectic_flow/diagnostics/{run_id}`
// handler reads from this cache; restarts intentionally clear it.
// Matches Bee's locked decision IV-H — "HTTP is consumer-facing
// canonical for read": clients fetch the last N runs they kicked off
// in this process lifetime. No `lru` crate dep — a VecDeque-backed
// FIFO is enough because the access pattern is "last N runs", not
// "most-recently-accessed". When capacity is hit we drop the oldest
// front entry.

const DIAGNOSTICS_CACHE_CAPACITY: usize = 32;

#[derive(Debug)]
struct DiagnosticsCache {
    order: VecDeque<String>,
    by_id: HashMap<String, SymplecticFlowResponse>,
}

impl DiagnosticsCache {
    fn new() -> Self {
        Self {
            order: VecDeque::with_capacity(DIAGNOSTICS_CACHE_CAPACITY),
            by_id: HashMap::with_capacity(DIAGNOSTICS_CACHE_CAPACITY),
        }
    }

    fn insert(&mut self, run_id: String, resp: SymplecticFlowResponse) {
        // If the cache already holds this id (vanishingly unlikely with
        // UUID v4 but we honor the invariant for determinism in tests
        // that mock the id), refresh in place — drop the old position
        // and push to the back.
        if self.by_id.contains_key(&run_id) {
            self.order.retain(|k| k != &run_id);
        } else if self.by_id.len() >= DIAGNOSTICS_CACHE_CAPACITY {
            // Evict the oldest entry.
            if let Some(oldest) = self.order.pop_front() {
                self.by_id.remove(&oldest);
            }
        }
        self.order.push_back(run_id.clone());
        self.by_id.insert(run_id, resp);
    }

    fn get(&self, run_id: &str) -> Option<SymplecticFlowResponse> {
        self.by_id.get(run_id).cloned()
    }
}

fn diagnostics_cache() -> &'static Mutex<DiagnosticsCache> {
    static C: OnceLock<Mutex<DiagnosticsCache>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(DiagnosticsCache::new()))
}

/// Look up a previously-completed `SymplecticFlowResponse` by its
/// server-generated `run_id`. Returns `None` when the id is unknown
/// (never registered, or evicted past capacity). Used by the HTTP
/// `GET /v1/symplectic_flow/diagnostics/{run_id}` handler.
pub fn get_diagnostics(run_id: &str) -> Option<SymplecticFlowResponse> {
    let c = diagnostics_cache()
        .lock()
        .expect("symplectic flow diagnostics cache mutex poisoned");
    c.get(run_id)
}

/// Clear the process-local diagnostics LRU. Test convenience — the
/// HTTP harness clears between tests so per-test fixtures land into a
/// known-empty cache.
pub fn clear_diagnostics_cache() {
    let mut c = diagnostics_cache()
        .lock()
        .expect("symplectic flow diagnostics cache mutex poisoned");
    *c = DiagnosticsCache::new();
}

/// Run `n_steps` KDK leapfrog steps on `(U, E)` at inverse temperature
/// `config.beta` and step size `config.dt`, with optional per-step
/// Gauss projection.
///
/// `u_name` must point at an SU(2) gauge field registered via
/// `register_su2`; `e_name` must point at an SU(2) E field registered
/// via `register_su2_e`. The two fields must share the same bound
/// lattice (the E-field constructor already enforces this; we re-check
/// here so a manually-mis-paired call surfaces a typed error).
///
/// SEED contract: when an observable measurement reads E (which
/// requires no further RNG draws at the flow level — the MB E init
/// already consumed its seed at declaration time) and PROJECT_GAUSS
/// runs (which is fully deterministic CG, no RNG), the flow itself
/// needs no fresh seed. We still accept the `seed` argument and echo
/// it in the diagnostics row so the response is self-describing and
/// the WAL / downstream JSON envelope has a stable slot. Stochastic
/// kernels Part V+ might add (Langevin noise, refreshed MB momentum)
/// will reach into this seed; for IV.6 it is an echo-only slot.
pub fn symplectic_flow(
    u_name: &str,
    e_name: &str,
    config: SymplecticFlowConfig,
    seed: Option<u64>,
) -> Result<SymplecticFlowResponse, GaugeFieldError> {
    // Resolve the dual handles through the registries.
    let u_arc =
        get_su2_mut(u_name).ok_or_else(|| GaugeFieldError::FieldNotDeclared(u_name.to_string()))?;
    let e_arc = get_su2_e_mut(e_name)
        .ok_or_else(|| GaugeFieldError::EFieldNotDeclared(e_name.to_string()))?;

    // Acquire the lattice once + cache the two incidence tables out of
    // the step loop (locked decision: caller hoists incidence — we ARE
    // the caller here, so the hoist lives at the flow level).
    let lattice_name = {
        let guard = u_arc.lock().expect("su2 field mutex poisoned");
        guard.lattice_name.clone()
    };
    {
        let e_guard = e_arc.lock().expect("e field mutex poisoned");
        if e_guard.source_lattice != lattice_name {
            return Err(GaugeFieldError::EFieldSourceMismatch {
                e_lattice: e_guard.source_lattice.clone(),
                u_lattice: lattice_name,
            });
        }
    }
    let lat = lattice_registry::get(&lattice_name)
        .ok_or_else(|| GaugeFieldError::LatticeNotDeclared(lattice_name.clone()))?;
    let edge_face_inc = build_edge_face_incidence(&lat);
    let face_edges_cache = build_face_edges_cache(&lat);
    let vertex_edge_inc = build_vertex_edge_incidence(&lat);

    // SU(2) Wilson-action coupling: g² = (2·N) / β = 4 / β.
    let g2 = 4.0_f64 / config.beta;
    let dt_half = config.dt / 2.0_f64;

    let mut history: HashMap<ObservableId, Vec<f64>> = HashMap::new();
    let mut cg_iters: Vec<usize> = Vec::with_capacity(config.n_steps);

    {
        let mut u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        let mut e_guard = e_arc.lock().expect("e field mutex poisoned");

        for s in 0..config.n_steps {
            // ── KDK leapfrog body ──
            //
            // K: F0 from U (current).
            let f0 = wilson_force_per_edge(&*u_guard, &lat, &edge_face_inc, &face_edges_cache, config.beta)?;
            apply_force_kick(&mut *e_guard, &f0, dt_half)?;
            // D: U_new = exp(dt · g² · E) · U.
            drift_step(&mut *u_guard, &*e_guard, config.dt, g2)?;
            // K: F1 from U (new).
            let f1 = wilson_force_per_edge(&*u_guard, &lat, &edge_face_inc, &face_edges_cache, config.beta)?;
            apply_force_kick(&mut *e_guard, &f1, dt_half)?;
            // PROJECT_GAUSS (per-step, IV-D locked).
            if let Some(pg_cfg) = config.project_gauss {
                let diag = project_gauss(
                    &mut *e_guard,
                    &*u_guard,
                    &lat,
                    &vertex_edge_inc,
                    pg_cfg,
                )?;
                cg_iters.push(diag.cg_iterations);
            }

            // ── Measurement epilogue ──
            if config.measure_every > 0 && (s + 1) % config.measure_every == 0 {
                for obs in &config.measure {
                    let v = observe(
                        &*u_guard,
                        &*e_guard,
                        &lat,
                        &vertex_edge_inc,
                        config.beta,
                        *obs,
                    )?;
                    history.entry(*obs).or_default().push(v);
                }
            }
        }
    }

    // Republish the post-flow U into the dyn read map so any downstream
    // `gauge_registry::get(u_name)` sees the freshest snapshot. The
    // E-field registry already stores the Arc<Mutex<…>> we mutated in
    // place, so no separate publish step is required on the E side.
    republish_su2(u_name, u_arc.clone());

    // Reductions for the diagnostics block.
    let cg_iterations_per_step_p99 = p99_usize(&cg_iters);
    let max_energy_drift_rel = match history.get(&ObservableId::HTotal) {
        Some(chain) if !chain.is_empty() => {
            let h0 = chain[0];
            if h0.abs() > 0.0 {
                let mut m = 0.0_f64;
                for &h in chain.iter() {
                    let d = ((h - h0) / h0).abs();
                    if d > m {
                        m = d;
                    }
                }
                m
            } else {
                0.0
            }
        }
        _ => 0.0,
    };
    let gauss_residual_max = {
        let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
        let e_guard = e_arc.lock().expect("e field mutex poisoned");
        let r = compute_gauss_residual_covariant(
            &*u_guard,
            &*e_guard,
            &lat,
            &vertex_edge_inc,
        )?;
        max_inf_norm(&r)
    };

    // Generate a stable run_id and park the response in the diagnostics
    // LRU before returning so the HTTP read path can resolve it. UUID v4
    // is the canonical choice — collision-resistant without the caller
    // having to compute a hash of (seed, β, dt, n_steps, timestamp).
    let run_id = uuid::Uuid::new_v4().to_string();
    let response = SymplecticFlowResponse {
        run_id: run_id.clone(),
        field: u_name.to_string(),
        e_field: e_name.to_string(),
        measurement_history: history,
        diagnostics: SymplecticFlowDiagnostics {
            seed,
            beta: config.beta,
            dt: config.dt,
            n_steps_completed: config.n_steps,
            cg_iterations_per_step_p99,
            max_energy_drift_rel,
            gauss_residual_max,
        },
    };
    {
        let mut c = diagnostics_cache()
            .lock()
            .expect("symplectic flow diagnostics cache mutex poisoned");
        c.insert(run_id, response.clone());
    }
    Ok(response)
}

/// Dispatch an `ObservableId` to its E-aware implementation. Reverses
/// the III.5 `PartIvObservableNotReady` stub for the Part-IV observables.
fn observe(
    u: &SU2GaugeField,
    e: &SU2EField,
    lat: &Lattice,
    vinc: &VertexEdgeIncidence,
    beta: f64,
    obs: ObservableId,
) -> Result<f64, GaugeFieldError> {
    match obs {
        ObservableId::MeanPlaquette => plaquette_mean(u, lat),
        ObservableId::QSurrogate => q_surrogate(u, lat),
        ObservableId::HTotal => Ok(compute_hamiltonian(u, e, lat, beta)?),
        ObservableId::Energy => Ok(wilson_action(u, lat, beta)?),
        ObservableId::EdgeKinetic => Ok(edge_kinetic(e, beta)),
        ObservableId::GaussResidualMax => {
            let r = compute_gauss_residual_covariant(u, e, lat, vinc)?;
            Ok(max_inf_norm(&r))
        }
        ObservableId::VertexGauss => {
            let r = compute_gauss_residual_covariant(u, e, lat, vinc)?;
            let mut acc = 0.0_f64;
            for row in &r {
                acc += (row[0] * row[0] + row[1] * row[1] + row[2] * row[2]).sqrt();
            }
            Ok(acc / (r.len().max(1) as f64))
        }
    }
}

/// `H_total = K + V_pot` for the buckyball SU(2) Kogut-Susskind
/// Hamiltonian. Mirrors Halcyon `buckyball_integrator.py
/// ::compute_hamiltonian` exactly:
///
/// ```text
///     K = (g²/2) · Σ_e Tr(E_e²) = g² · Σ_e |E_vec|²
///     V = (1/(g²·N)) · Σ_f [N - Re Tr U_f]
///       = (1/g²) · Σ_f [1 - q0_f]
///       = (F/g²) · (1 - ⟨P⟩)
/// ```
///
/// where `g² = (2·N)/β = 4/β` for SU(2). The Trace identity uses
/// `Tr(E²) = 2·|E_vec|²` because Halcyon stores E with the quaternion
/// packing `E[..., 1:] = 2·alpha` (canonical sampler convention).
fn compute_hamiltonian(
    u: &SU2GaugeField,
    e: &SU2EField,
    lat: &Lattice,
    beta: f64,
) -> Result<f64, GaugeFieldError> {
    let kin = kinetic_energy(e, beta);
    let s_w = wilson_action(u, lat, beta)?;
    Ok(kin + s_w)
}

/// Kinetic energy `K = g² · Σ_e |E_vec|²`. Halcyon convention:
/// `Tr(E²) = 2·|E_vec|²` so `K = (g²/2)·Σ_e Tr(E²) = g²·Σ_e |E_vec|²`.
/// `g² = 4/β`.
fn kinetic_energy(e: &SU2EField, beta: f64) -> f64 {
    let g2 = 4.0_f64 / beta;
    let mut s = 0.0_f64;
    for edge in 0..e.buffer.n_edges {
        let row = e.read_element_q(edge);
        s += row[1] * row[1] + row[2] * row[2] + row[3] * row[3];
    }
    g2 * s
}

/// Mean kinetic energy per edge.
fn edge_kinetic(e: &SU2EField, beta: f64) -> f64 {
    let n = e.buffer.n_edges.max(1) as f64;
    kinetic_energy(e, beta) / n
}

/// SU(2) Wilson potential `V = (1/g²) · Σ_f [1 - q0_f] = (F/g²) · (1 - ⟨P⟩)`.
/// With `g² = 4/β` this evaluates to `(β·F/4) · (1 - ⟨P⟩)`. Halcyon
/// Kogut-Susskind convention.
fn wilson_action(
    u: &SU2GaugeField,
    lat: &Lattice,
    beta: f64,
) -> Result<f64, GaugeFieldError> {
    let g2 = 4.0_f64 / beta;
    let p_mean = plaquette_mean(u, lat)?;
    Ok((lat.n_faces() as f64) * (1.0_f64 - p_mean) / g2)
}

/// 99th-percentile of a usize slice, returned as f64. Empty → 0.0.
fn p99_usize(v: &[usize]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<usize> = v.to_vec();
    sorted.sort_unstable();
    // p99: ceil(0.99 · n) - 1, clamped to [0, n-1].
    let n = sorted.len();
    let idx = ((0.99_f64 * n as f64).ceil() as usize)
        .saturating_sub(1)
        .min(n - 1);
    sorted[idx] as f64
}

// Silence unused-import warning when nothing in the file currently
// names `GaugeFieldHandle` / `EdgeFaceIncidence` after the
// implementation settled — they're load-bearing for the doc comments
// + downstream consumers.
#[allow(dead_code)]
type _GaugeFieldHandleHint = dyn GaugeFieldHandle;
#[allow(dead_code)]
type _EdgeFaceIncidenceHint = EdgeFaceIncidence;

#[cfg(test)]
mod tests {
    use super::*;

    /// `p99_usize` on the empty slice returns 0.0.
    #[test]
    fn p99_empty_is_zero() {
        let v: Vec<usize> = vec![];
        assert_eq!(p99_usize(&v), 0.0);
    }

    /// `p99_usize` on a flat slice returns the value.
    #[test]
    fn p99_constant_is_constant() {
        let v: Vec<usize> = vec![7; 100];
        assert_eq!(p99_usize(&v), 7.0);
    }
}
