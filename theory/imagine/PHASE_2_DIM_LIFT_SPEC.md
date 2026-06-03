# Phase 2 dim lift — IMAGINE on N-dimensional substrates

**Status:** spec; not yet implemented
**Blocks:** Marcella's end-to-end IMAGINE_COHERENCE verification against production 384-dim bundles
**Unblocked by Phase 1 + 2D test bundle:** Marcella's `imagine_coherence()` consumer in `brain_primitives.py` activates the moment Phase 2 lands, no code change required.

---

## §1 — Why this is needed

Phase 1 of IMAGINE supports only 2D substrates. The `imagine_geodesic` integrator uses a **conformally-flat 2D metric** with a closed-form expression for Christoffel symbols:

```
ds² = e^{2φ(x, y)} (dx² + dy²)
Γ^x_{xx} = φ_x,  Γ^x_{xy} = φ_y,  Γ^x_{yy} = -φ_x
Γ^y_{xx} = -φ_y, Γ^y_{xy} = φ_x,  Γ^y_{yy} = φ_y
```

This is mathematically tight (T11 matches embedded-picture closed forms to machine precision), but it does not generalize directly to N > 2. Production bundles are 384-dim (BGE-v2 embeddings) and 768-dim (other corpora), so Phase 1 cannot integrate against real metric structure.

Marcella's 2026-06-03 IMAGINE_COHERENCE probe surfaced the operational symptom: even with a 2D seed `[0.0, 0.0]`, the endpoint reads the substrate's mean K from `bundle.curvature_stats().mean()`, which for a 384-dim bundle is large enough to make the conformally-flat 2D integrator diverge at step 1.

---

## §2 — What Phase 2 ships

### §2.1 N-dim Riemannian metric trait

```rust
pub trait RiemannianMetric: Send + Sync {
    /// Dimension of the substrate.
    fn dim(&self) -> usize;

    /// Metric tensor g_ij at a point. Returns an (n × n) symmetric
    /// positive-definite matrix. Phase 2 supports diagonal metrics
    /// directly; off-diagonal entries supported for Kähler bundles via
    /// the existing KahlerStructure surface.
    fn metric_at(&self, point: &[f64]) -> Vec<Vec<f64>>;

    /// Inverse metric g^ij at a point.
    fn inverse_metric_at(&self, point: &[f64]) -> Vec<Vec<f64>>;

    /// Christoffel symbols Γ^k_{ij} at a point. Returns a 3-tensor
    /// `[i][j][k]`. Default impl computes from metric and inverse
    /// metric via the standard formula; override for performance.
    fn christoffel_at(&self, point: &[f64]) -> Vec<Vec<Vec<f64>>> {
        christoffel_from_metric(&self.metric_at(point), &self.inverse_metric_at(point), point)
    }

    /// Sectional curvature at a point in a 2-plane spanned by two
    /// vectors. Used for the `local_k` field on imagined records.
    fn sectional_curvature_at(&self, point: &[f64], u: &[f64], v: &[f64]) -> f64;
}
```

### §2.2 Generalized `imagine_geodesic`

Same RK4 integrator, but the acceleration step uses the trait's `christoffel_at(point)` instead of the hard-coded conformal formula:

```rust
fn acceleration_nd(
    metric: &dyn RiemannianMetric,
    point: &[f64],
    velocity: &[f64],
) -> Vec<f64> {
    let n = point.len();
    let gamma = metric.christoffel_at(point);
    let mut accel = vec![0.0; n];
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                accel[k] -= gamma[i][j][k] * velocity[i] * velocity[j];
            }
        }
    }
    accel
}
```

Cost: O(n³) per step. For n = 384, that's ~57M flops per RK4 sub-step, so per imagined trajectory step (4 sub-steps) is ~230M flops. With 10 steps default = ~2.3 GF per call. Comfortable for a request-time call against a single seed; matters for batch.

### §2.3 Lift the request-shape constraint

In `gigi_stream.rs`:

```rust
// CURRENT (Phase 1):
if req.starting_from.len() != 2 {
    return Err((StatusCode::BAD_REQUEST, ...));
}

// PHASE 2:
if req.starting_from.len() != bundle.fiber_dim() {
    return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
        error: format!(
            "seed dim {} does not match bundle fiber dim {}",
            req.starting_from.len(), bundle.fiber_dim()
        ),
    })));
}
```

This is the only request-shape change. The response shape stays identical — the `trajectory[i].coords` field becomes N-dim instead of 2-dim, but it was already a `Vec<f64>` in the JSON serialization.

### §2.4 Curvature-aware metric construction

For Phase 1 bundles we used `metric_for_constant_k(K)` — a 2D conformally-flat metric. For Phase 2, the metric comes from the bundle directly:

```rust
pub trait BundleMetricSource {
    /// Construct a RiemannianMetric closure from the bundle's
    /// substrate structure. Default impl uses the cached Kähler
    /// structure if present; falls back to a learned local metric
    /// from the k-NN graph otherwise.
    fn imagine_metric(&self) -> Box<dyn RiemannianMetric>;
}

impl BundleMetricSource for BundleStore { ... }
```

The Phase 1 `metric_for_constant_k(K)` becomes a legacy entry point retained only for testing.

---

## §3 — TDD gates for Phase 2

| Gate | Claim | Ground truth |
|---|---|---|
| **T14** | N-dim Christoffel computation matches Phase 1 in n=2 | `christoffel_at(...)` on a conformally-flat 2D metric returns the same Γ as the hard-coded formula |
| **T15** | N-dim geodesic on S³ matches closed form | Closed-form S³ geodesic (great-circle in stereographic) |
| **T16** | N-dim geodesic on a 4-dim torus T⁴ is a straight line | Flat metric, all Γ=0 |
| **T17** | Geodesic on a 384-dim sphere with controlled curvature does not diverge | Synthetic 384-dim sphere via reduction-to-S^(n-1) projection |
| **T18** | Production parity: Marcella's bge_v2 bundle integrates without divergence at typical seeds | Marcella's actual 384-dim bundle, sample seeds with `query_grounding_normalized > 0.3` |

T17 is the "no-divergence at 384-dim" gate. T18 is the production parity gate using Marcella's actual bundle.

---

## §4 — Migration path

1. Land Phase 2 behind the same `imagine` feature flag. Phase 1 functions remain valid for 2D bundles.
2. The HTTP endpoint detects the bundle's fiber dim and routes to the appropriate integrator:
   - `dim == 2` → Phase 1 path (back-compat, faster)
   - `dim >= 3` → Phase 2 path (general, slower)
3. The response shape stays identical. Marcella's consumer code activates without modification.

**Effort estimate:** ~3 days (1 day for trait + integrator, 1 day for TDD gates T14–T16, 1 day for T17/T18 + integration).

---

## §5 — Phase 2 also lifts the K-tolerance constraint

In Phase 1, large bundle K values cause divergence even when the seed is valid. Phase 2's metric is constructed directly from the bundle's geometric structure (Kähler or learned), so the substrate's actual curvature shape (not just the scalar mean K) drives the integration. This makes the integrator stable on bundles with large mean K but well-conditioned local structure — which is the production case.

In other words: Phase 2 fixes BOTH the dim constraint AND the K-tolerance issue Marcella observed. They are the same constraint at the math level (using a substrate-aware metric rather than a constant-K conformal one).

---

## §6 — Where this fits in the bigger plan

| Layer | Status |
|---|---|
| IMAGINE Phase 1 (2D substrates) | shipped 2026-06-03 |
| 2D synthetic test bundle (`imagine_test_2d`) | shipped 2026-06-03 (this sprint) |
| T13 production SwDA seam gate | shipped 2026-06-03 (this sprint) |
| **IMAGINE Phase 2 (N-dim substrates)** | **THIS SPEC — next sprint** |
| WALK Phase 2 (double-cover lift + SUDOKU pre-flight) | blocked on Phase 2 dim lift |
| IMAGINE_HALO real K refactor | depends on Phase 2 |
| Bridge IMAGINE across atlases (Phase F) | depends on Phase 2 + cross-atlas join math |

---

## §7 — Open questions

1. **Should Phase 2 ship Kähler-aware metric construction first or learned metric construction first?** Kähler-aware is cleaner mathematically and works for the Kähler-flagged bundles we already ship (Marcella's `marcella_source_embeddings_bge_v2` has the Kähler structure attached). Learned-metric is more general. **Recommendation:** Kähler-aware first because it lights up Marcella's production bundles immediately.

2. **What happens when the bundle has both a Kähler structure and a learned metric?** Defer to Kähler when present; surface a `metric_source` field in the response for transparency.

3. **Should the Phase 2 integrator step size be adaptive?** Yes for N >= 10, because the O(n³) cost per step makes fixed-step inefficient. Reuse the same `ImagineConfig::adaptive: bool` flag that Phase 1 has but doesn't honor yet.

4. **Cache `christoffel_at` between trajectory steps?** The metric does NOT change between steps (it's a property of the bundle, not the trajectory). For static bundles, we could pre-compute Γ at a grid of points and interpolate. **Recommendation:** spec the cache but defer implementation until profiling shows it's the bottleneck.

---

**Owner:** GGOG engine team
**Consumer ready:** Marcella's `imagine_coherence()` in `brain_primitives.py` is wired fail-open today; activates at the moment Phase 2 lands.
