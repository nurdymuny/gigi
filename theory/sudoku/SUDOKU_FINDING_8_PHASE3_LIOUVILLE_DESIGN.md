# SUDOKU FINDING 8 — IMAGINE Phase 3: Liouville-form n-D RK4 geodesic integrator (DESIGN-ONLY)

**Status:** DESIGN-ONLY — implementation deferred. Doc lands; no code change.
**Date:** 2026-06-26
**Author:** Bee Davis
**Bundle context:** Phase 4 SUDOKU shipment, 8-item local cleanup wave.

---

## 1. Where we are

Phase 1 (shipped pre-2026-06): closed-form constant-K geodesics on a 2-D
hyperbolic patch via `integrate_geodesic_phase_1` (great-circle on
S² when K > 0, hyperbolic-circle when K < 0, straight line when K = 0).

Phase 2 (shipped `a190a72` 2026-06-25, gate `imagine_coherence_phase2`
10/0 PASS): n-D constant-K geodesics + tame-metric fallback for high-K
or dim-mismatch. Marcella's 384-dim embedding queries are unblocked
under Phase 2; the IMAGINE engine's `/v1/bundles/{name}/imagine/coherence`
endpoint returns valid trajectories on the production manifold.

Phase 2 is **constant-K everywhere along the trajectory**, evaluating K
once at the start point and using closed-form spherical / hyperbolic /
flat geodesics from that single K. This is correct iff the Davis manifold
is locally homogeneous; in production it is not, and the gap shows up
when trajectories cross curvature-discontinuity regions.

Phase 3 (this design): **variable-K integration** via Liouville's
conformally-flat metric form, RK4 step on the 2n-D position+velocity
phase space, K sampled per-step from `bundle.curvature_stats()` at the
current point.

## 2. Math: Liouville K-phi relation

A conformally-flat metric in n dimensions has the form

```
g_{ij}(x) = e^{2 phi(x)} delta_{ij}                         (Eq. 2.1)
```

with `phi: M -> R` a scalar potential. Liouville's equation in 2-D
relates the Gaussian curvature K(x) to phi by

```
Laplacian(phi)(x) = -K(x) e^{2 phi(x)}                       (Eq. 2.2; n=2)
```

In higher dimensions (n >= 3), the analog for the conformally-flat
sectional curvature K(x) is

```
-(n-1) Laplacian(phi)(x) - (n-1)(n-2)/2 |grad phi(x)|^2     (Eq. 2.3; n>=3)
  = K(x) e^{2 phi(x)}
```

Eqs. 2.2 / 2.3 are **PDEs in phi**; solving them along a trajectory is
intractable in closed form. The Phase 3 integrator instead samples
K(x_t) directly from `bundle.curvature_stats()` at each RK4 step and
uses the Christoffel symbols of the conformally-flat metric:

```
Gamma^k_{ij}(x) = delta_i^k partial_j phi(x)                 (Eq. 2.4)
              + delta_j^k partial_i phi(x)
              - delta_{ij} delta^{kl} partial_l phi(x)
```

The geodesic equation in coordinates becomes

```
d^2 x^k/dt^2 + Gamma^k_{ij}(x) dx^i/dt dx^j/dt = 0           (Eq. 2.5)
```

which the Phase 3 integrator splits into the 2n-D first-order system

```
dx^k/dt = v^k                                                 (Eq. 2.6a)
dv^k/dt = - Gamma^k_{ij}(x) v^i v^j                          (Eq. 2.6b)
```

and steps with RK4 (4 stages per step, 5 evaluations of K + Gamma).

## 3. Public API surface

### 3.1 New Rust functions (`src/imagine/integrator.rs`)

```rust
/// Phase 3: variable-K Liouville-form n-D RK4 integrator.
///
/// On per-step K sampling from `bundle.curvature_stats()` and conformally-flat
/// Christoffel symbols derived from a local phi(x) via Eq. 2.3.
///
/// Opt-in only via the `integrator: "liouville"` request field. When the
/// caller passes `integrator: "constant_k"` or omits the field, the dispatch
/// at `src/bin/gigi_stream.rs:3282` falls through to
/// `integrate_geodesic_phase_2` (Phase 2 closed-form). This preserves
/// Phase 2 bit-identity for all existing callers (including Marcella).
pub fn integrate_geodesic_phase_3_liouville(
    bundle: &impl HasCurvatureStats,
    start: &[f64],
    velocity: &[f64],
    steps: usize,
    dt: f64,
) -> Result<Trajectory, ImagineError> {
    Err(ImagineError::NotImplemented {
        phase: 3,
        reason: "Liouville-form n-D RK4 integrator — design shipped; \
                 implementation deferred to dedicated Phase 3 sprint. \
                 See theory/sudoku/SUDOKU_FINDING_8_PHASE3_LIOUVILLE_DESIGN.md.",
    })
}

/// Phase 3 high-level: coherence trajectory using the Liouville integrator.
///
/// Behaviorally identical to `imagine_coherence_trajectory_phase_2` on input
/// shapes that resolve to constant local K, by the bit-identity gate test
/// in §5.
pub fn imagine_coherence_trajectory_phase_3(
    bundle: &impl HasCurvatureStats,
    request: &CoherenceRequest,
) -> Result<CoherenceResponse, ImagineError> {
    Err(ImagineError::NotImplemented { phase: 3, reason: "..." })
}

/// Private: sample K at a point.
fn sample_local_k(bundle: &impl HasCurvatureStats, point: &[f64]) -> f64 { ... }

/// Private: build conformally-flat Christoffel symbols from local K.
fn christoffel_conformally_flat(local_k: f64, dim: usize) -> Vec<Vec<Vec<f64>>> { ... }

/// Private: one RK4 step on Eq. 2.6.
fn rk4_step(state: &State, dt: f64, bundle: &impl HasCurvatureStats) -> State { ... }
```

### 3.2 HTTP dispatch (`src/bin/gigi_stream.rs:3282`)

```rust
// Phase 3 opt-in dispatch.
let integrator_choice = request
    .integrator
    .as_deref()
    .unwrap_or("constant_k");

let trajectory = match (integrator_choice, dim, fallback_decision) {
    ("liouville", _, _) => {
        // Phase 3 opt-in path.
        imagine_coherence_trajectory_phase_3(&bundle, &request)?
    }
    ("constant_k", 2, FallbackDecision::None) => {
        // Phase 1 path (bit-identical to pre-Phase-2).
        imagine_coherence_trajectory_phase_1(&bundle, &request)?
    }
    ("constant_k", _, _) => {
        // Phase 2 path — current production behavior.
        imagine_coherence_trajectory_phase_2(&bundle, &request)?
    }
    (other, _, _) => {
        return Err(ImagineError::UnknownIntegrator(other.to_string()));
    }
};
```

This three-arm dispatch preserves Phase 2 (and Phase 1) bit-identity for all
non-Liouville requests **by construction** — only Phase 3 callers (which
must explicitly opt in with `"integrator": "liouville"`) hit the new path,
and the new path returns `NotImplemented` for now.

## 4. Blockers from prior DISCOVER analysis

The DISCOVER phase surfaced four blockers + this design surfaces a fifth.
Each needs resolution before Phase 3 ships:

### 4.1 K-sampling latency

`bundle.curvature_stats()` is currently not point-wise; it returns
manifold-level summary statistics. Phase 3 needs `K(x)` at arbitrary
points along the trajectory.

**Options:**
- (a) **Local KNN K-average**: at point x, find the k-nearest fixture
  records, average their precomputed K. Cost: O(log N) per RK4 step
  with a kd-tree; build cost amortized. Bias: low when fixtures are
  dense, high when sparse.
- (b) **Analytic K from Davis-duality**: derive K(x) from the curvature
  potential `phi(x)` stored at fixture sites and interpolated linearly.
  Cleaner but requires precomputed phi.
- (c) **Constant-K-per-segment**: split the trajectory into segments at
  fixture-density boundaries; use Phase 2 within each segment. Easier
  but loses smoothness across boundaries.

Recommended: (a) for the first cut, (c) as a fallback when fixtures
are too sparse, (b) as a future cleanup.

### 4.2 RK4 step-size adaptivity

Fixed dt risks blowing up across high-K regions (the manifold pinches).
Need an embedded RK4/RK5 (Fehlberg) with per-step error estimate and
step-size halving / doubling.

### 4.3 Trajectory length termination

Phase 2 terminates at fixed proper time. Phase 3 needs both proper-time
and arc-length termination criteria, because the variable-K manifold can
have arbitrary metric scaling.

### 4.4 Numerical instability near K = +inf

Singular curvature regions (rare but present in Marcella's overlap
between embedded passages) blow up Gamma. Need a "tame-metric fallback"
parallel to Phase 2's existing high-K fallback, but invoked per-step:
when local |K| exceeds a threshold, downgrade that step to the tame
metric.

### 4.5 (NEW) Position-to-K adapter

The current `HasCurvatureStats` trait does not expose `K(x)` at a point.
Phase 3 needs a new trait method `curvature_at_point(&self, x: &[f64])
-> f64`. Three resolution paths:

- (a) Add the method to `HasCurvatureStats`; provide a default impl
  that returns the manifold-average K (so all current implementors
  compile); override in the bundles that need point-wise K.
- (b) Introduce a new sub-trait `HasPointCurvature: HasCurvatureStats`;
  Phase 3 takes the sub-trait bound; default Phase 2 implementations
  do not need to migrate.
- (c) Push K-sampling into the IMAGINE engine itself (it stays
  bundle-agnostic but loses the option of bundle-specific K models).

Recommended: (b). It cleanly separates the surface change.

## 5. Bit-identity gate test

The Phase 3 implementation lands GREEN iff:

1. **Gate `G-IMAGINE-P3-BIT-IDENTITY`**: a synthetic bundle with truly
   constant local K (returns the same K at every point) must produce a
   Phase 3 trajectory that matches `integrate_geodesic_phase_2` within
   `1e-10 * magnitude` (component-wise). This is the math sanity test:
   variable-K with constant K = closed-form constant-K.

2. **Gate `G-IMAGINE-P3-CONFORMAL-EQ-EUCLIDEAN`**: when K = 0 everywhere,
   Phase 3 trajectory must be a straight line within `1e-12`.

3. **Gate `G-IMAGINE-P3-MARCELLA-NO-REGRESSION`**: existing
   `imagine_coherence_phase2` tests (currently 10/0) stay GREEN
   bit-identically (Phase 2 dispatch path untouched).

4. **Gate `G-IMAGINE-P3-OPT-IN-ONLY`**: a request without
   `"integrator": "liouville"` MUST NOT touch the Phase 3 code path.
   Test by setting `imagine_coherence_trajectory_phase_3` to panic, then
   running the full `imagine_coherence_phase2` gate suite, and asserting
   no panic.

## 6. Locked gate references (must stay GREEN when Phase 3 lands)

The Phase 4B (this workflow) ships with all 8 locked gates GREEN. When
Phase 3 implementation lands in a future sprint:

- Gate 1: `cargo test --no-default-features --lib` >= 882/0
- Gate 2: `cargo test --features halcyon --test halcyon_part_iv_gold` 4/0 + 1 ign
- Gate 3: `cargo test --features halcyon --release --test halcyon_part_vi_bit_identity_gold -- --include-ignored` 3/0
- Gate 4: `cargo test --features kahler --test davis_conjecture_lambda_brain_ridealong` 25/0
- Gate 5: `cargo test --features halcyon --test aurora_lie_poisson_trait` 12/0
- Gate 6: `cargo test --features wish,imagine --test wish_extensions_halcyon_asks --test wish_integrate_along_path` 17/0
- Gate 7: `cargo test --features kahler,imagine --test imagine_coherence_phase2` 10/0
- Gate 8: `cargo test --features halcyon,gauge --release --lib tdd_hal_v_3_replay` 4/0 (or 5/0)

PLUS the four new Phase 3 gates from §5.

## 7. Why DESIGN-ONLY for this workflow

Per the SUDOKU workflow's own decision rule (ITEM 8 "Decision rule"):
implement only if > 30 min wall-clock left; otherwise design-only. The merge
agent is past that budget — the implementation surface (new trait method,
new private functions, new HTTP dispatch arm, four new gate tests, K-sampling
infrastructure) exceeds the remaining budget and would risk tripping a
locked gate.

The design ships sprint-ready: every blocker has a named resolution,
every public API has a signature, the bit-identity gate is specified.
The next sprint walks in and implements.

## 8. Production posture

Local-only. No `flyctl`, no `git push`, no production-mutating `curl`. The
merge agent applies this doc via `git apply` on local `main`. Phase 2 stays
the path for production callers (including Marcella) until Phase 3 lands
behind an explicit opt-in.

Gate 1 verified GREEN at 882/0 before this doc landed; no source-code change
in the working tree at the time this doc lands.

## 9. Filed under

- ITEM 8 of the 2026-06-26 SUDOKU 8-item local shipment.
- Companion to:
  - `SUDOKU_FINDING_1_ENCODER_HANG.md` (ITEM 1).
  - `SUDOKU_FINDING_3_MMAP_TRIAGE.md` (ITEM 3).
  - `SUDOKU_FINDING_5_LZ4_BENCHMARK.md` (ITEM 5).
  - `SUDOKU_FINDING_7_LAZY_LOADING_DESIGN.md` (ITEM 7).
- Upstream:
  - `theory/imagine/WISH_SPEC_v0.1.md` (the WISH verb that motivates
    variable-K integration in the first place).

— Bee Davis, 2026-06-26.
