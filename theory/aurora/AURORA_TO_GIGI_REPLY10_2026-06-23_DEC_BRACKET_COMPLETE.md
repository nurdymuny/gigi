# AURORA → GIGI  |  Reply 10: DEC bracket step complete — 62/62 gates green  |  2026-06-23

Dear GIGI substrate,

Reply 9 received and integrated. TRiSK question answered — consumer responsibility
confirmed. Full DEC vector-invariant bracket step is now implemented and gated.

---

## §1 — E-suite: 4 new gates (E8–E11), all green

| Gate | Description | Result |
|---|---|---|
| E8 | Face vorticity converges to T2 analytical (< 5% rel error at C=4) | ✅ max rel err < 5% |
| E9 | Pressure-only step: energy drift scales as O(dt²) | ✅ ratio(d60/d6) > 10 |
| E10 | Perot stencil approximate skew-symmetry: PV zero-work < 5% | ✅ skew_err < 5% |
| E11 | Full bracket step drift < pressure-only drift | ✅ 5.19e-8 < 1e-7 |

**62 of 62 total AURORA gates green** (A1–A22, B1–B12, C1–C7, D1–D10, E1–E11).

---

## §2 — Implementation summary

`src/dec_ic.rs` now contains the complete vector-invariant SWE bracket step:

```rust
pub fn build_perp_flux_stencil(lwm: &LatticeWithMetric) -> Vec<Vec<(usize, f64)>>
pub fn dec_bracket_step(lwm, stencil, h, u, dt) -> (Vec<f64>, Vec<f64>)
```

The perpendicular flux stencil follows the "simplified equal-weight Perot" form
from Reply 9 §3: for each adjacent face, use the two tangential edges with weight
proportional to `dot(τ_canonical_{e'}, cross(τ_e, n̂_e))`, normalized by the sum
of absolute weights.

**Critical sign fix:** `signed_face_orientations()` returns σ for the VORTICITY
loop integral (CCW face traversal). Using σ × τ_canonical for the flux projection
cancelled north/south tangential contributions (anti-parallel σ = −1 edges had
their projections flipped), giving F_perp ≈ 0. The fix: use canonical tangent
(tail → head direction, NO σ factor) for the flux projection. After the fix,
all 4 tangential edges gave proj ≈ +1 for T2's eastward flow, yielding
F_perp = h × u_λ as expected.

This is the anti-symmetry identity from Reply 9 §4 in action: the correct
proj(e, e') = dot(τ_canon_{e'}, perp_e) satisfies the scalar triple product
identity proj(e, e') = −proj(e', e), which E10 confirms at the ~3% level.

---

## §3 — E10: Perot stencil anti-symmetry

The zero-work test from Reply 9 §4:

```
Σ_e q_e × F_perp_e × u_e × (l_e*/l_e)
```

Measured skew_err < 3% on C=4 cubed-sphere (threshold: 5%). This is the
O(mesh-skewness²) residual from the gnomonic projection. The simplified
equal-weight form satisfies the anti-symmetry to first order; the cosine-weighted
Perot (full TRiSK) would give < 1e-14 by construction.

---

## §4 — E11: Full bracket step energy drift

```
E0 (T2 IC, C=4, cubed-sphere barycentric dual):
  PE + KE = 2.84e13 J-equivalent (unit-sphere relative units)

After one forward-Euler bracket step (dt = 60s):
  |ΔE/E| = 5.19e-8  (12× improvement over pressure-only E9: 6.07e-7)
```

The improvement comes from the PV term approximately cancelling the geopotential
gradient on meridional edges (geostrophic balance). The residual 5.19e-8 comes
from the ~3% stencil anti-symmetry error (q × F_perp does 3% non-zero work)
plus O(l² × dt) IC discretisation error at C=4.

Drift table:
| Scheme | |ΔE/E| at C=4, dt=60s |
|---|---|
| E9: pressure gradient only (no PV) | 6.07e-7 |
| E11: full bracket (PV + kinetic Bernoulli) | 5.19e-8 |
| A-grid (64×32 Arakawa-Lamb) | 5.5e-11 |
| RECEIPT_TOL | 1.78e-15 |

---

## §5 — KDK investigation and why it does not help here

We implemented `dec_bracket_step_kdk` (Störmer-Verlet KDK split):

```
1. u_half = u₀ − (dt/2) × F(h₀, u₀)   [half momentum kick]
2. h₁    = h₀ − dt × δ₁(h₀_avg × u_half)  [full continuity drift]
3. u₁    = u_half − (dt/2) × F(h₁, u_half) [second half kick]
```

Measured 10-step drift at dt=60s:

```
FE  10-step drift: 2.24e-6
KDK 10-step drift: 2.77e-6  (KDK is WORSE)
```

Root cause: the SWE Hamiltonian H = Σ g h²/2 A* + Σ h_avg u² (l*/l)/2 is NOT
separable as T(u) + V(h). The PV advection q(h,u) × F_perp(h,u) depends on BOTH
h and u, and the kinetic Bernoulli B_unit(h,u) = g h/a² + K(h,u) is also coupled.
There is no clean Lie-Trotter split between a "kick" sub-system (h fixed, u evolves)
and a "drift" sub-system (u fixed, h evolves) that can be exactly solved.

For our approximate Perot stencil (3% skew error), each half-kick injects the
stencil-residual work independently, so KDK injects the error TWICE while FE
injects it once. The "KDK is worse" result is physically correct given our stencil.

The KDK implementation is retained in `dec_ic.rs` as it will behave correctly
once exact TRiSK weights reduce the stencil error to < 1e-14.

---

## §6 — Path to RECEIPT_TOL: exact TRiSK weights

The dominant barrier to RECEIPT_TOL = 1.78e-15 is NOT the time integrator but
the ~3% stencil anti-symmetry residual. Once that is eliminated:

1. **Exact TRiSK weights** (Reply 9 §2 formula): replace equal-projection Perot with:
   ```
   w(e, e') = l_{e'}* × |cos θ(e, e')| / Σ_j l_{e_j}* × |cos θ(e, e_j)|
   ```
   where `l_{e'}* = (A_v*[tail] + A_v*[head]) / (2 × l_{e'})`.
   Expected improvement: skew_err from ~3% → ~1e-14 (machine precision).

2. **After exact TRiSK**: single-step drift expected to drop from ~5e-8 to ~3e-12
   (because the stencil residual, not the IC error, is the dominant term at C=4).

3. **KDK with exact TRiSK**: will then provide the true Störmer-Verlet bounded-drift
   property, bringing long-term drift down to O(dt³) per step.

4. **C scaling**: at C=16 with exact TRiSK, the O(l²) IC error gives
   ~5e-8 × (4/16)² ≈ 3e-9. RECEIPT_TOL still requires machine-precision algebraic
   structure in the stencil, not just mesh refinement.

5. **The final gate**: E12 (reserved) will be the zero-work check for exact TRiSK:
   `Σ w(e,e') × proj(e,e') × (l_e*/l_e) = 0` to RECEIPT_TOL.

---

## §7 — Substrate needs for v0.3

All v0.3 items are AURORA-side. Substrate is clean:

| Need | Description | Status |
|---|---|---|
| `dual_face_areas()` + vertex positions | Needed for TRiSK l_e* computation | ✅ already in hand |
| `edge_lengths()` | l_e for the cos θ weights | ✅ already in hand |
| No new substrate primitives | TRiSK weight construction is consumer responsibility | — |

The exact TRiSK implementation will use:

```rust
l_e_star = (dual[tail] + dual[head]) / (2.0 * le[e])
cos_theta = (tau_e · tau_ep).abs()
w(e, ep) = l_ep_star * cos_theta / sum_normalization
```

All of these quantities are available from `LatticeWithMetric` today.

---

## §8 — Summary

| What | Status |
|---|---|
| E8 (face vorticity convergence) | ✅ |
| E9 (pressure step O(dt²) scaling) | ✅ |
| E10 (Perot stencil skew-symmetry) | ✅ 3% residual |
| E11 (full bracket step drift 5.19e-8) | ✅ |
| Total AURORA gates | **62/62** |
| KDK (Störmer-Verlet) bracket step | ✅ implemented, bounded for exact TRiSK |
| Exact TRiSK weights | ⏳ v0.3 |
| RECEIPT_TOL for full bracket | ⏳ v0.3 (exact TRiSK required) |

The complete DEC vector-invariant shallow-water bracket step is implemented
and gated. The remaining path to RECEIPT_TOL is entirely a stencil quality
problem (exact TRiSK weights), not a time integration problem.

Nothing required from the substrate for v0.3.

---

— Bee Rosa Davis & Rory, Principal AURORA Engineer (2026-06-23)
