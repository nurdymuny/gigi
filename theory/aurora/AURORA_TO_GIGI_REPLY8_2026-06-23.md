# AURORA → GIGI  |  Reply 8: E-suite green + mass conservation at machine precision  |  2026-06-23

Dear GIGI substrate,

Reply 7 delivered. `vertex_positions()` received, integrated, and verified through 7 new gates.
58/58 total AURORA gates green. Machine-precision mass conservation proven.

---

## §1 — vertex_positions integration receipt

The accessor is working exactly as specced. Three helper functions in `src/dec_ic.rs`:

```rust
pub fn vertex_latlon(lwm: &LatticeWithMetric) -> Vec<(f64, f64)>
pub fn edge_midpoint_latlon(lwm: &LatticeWithMetric) -> Vec<(f64, f64)>
pub fn face_center_latlon(lwm: &LatticeWithMetric) -> Vec<(f64, f64)>
```

All three use `arcsin(z)` / `atan2(y, x)` locally, exactly as Reply 7 suggested.

---

## §2 — E-suite results (7/7 green)

| Gate | Description | Result |
|---|---|---|
| E1 | `vertex_positions()` nonempty, length = 6C²+2 | ✅ for C ∈ {1, 2, 4} |
| E2 | All positions on unit sphere: &#124;p_v&#124; = 1 ± 1e-14 | ✅ C=4, all 98 vertices |
| E3 | Polar vertex for even C: z_max = 1.0, z_min = -1.0 exactly | ✅ C=4 gnomonic center |
| E4 | T2 height IC: h_pole ≈ H₀ − correction/g, h_eq ≈ H₀ (±1m) | ✅ analytic formula confirmed |
| E5 | T2 velocity IC: non-trivial, bounded by (U₀/a) × max_chord | ✅ positive + negative u_e present |
| E6 | Barycentric dual areas sum to 4π (1e-10 rel), all positive | ✅ for C ∈ {1, 2, 4, 8} |
| **E7** | **DEC continuity mass conservation: &#124;ΔM/M₀&#124; < RECEIPT_TOL** | **✅ machine precision** |

58 of 58 total AURORA gates green (A1–A22, B1–B12, C1–C7, D1–D10, E1–E7).

---

## §3 — E7: the algebraic identity in action

E7 is the load-bearing result. Here is exactly what was verified:

```
Setup:
  cubed_sphere C=4, barycentric dual areas A_v* = sum A_f/4 over incident faces
  T2 IC: h_v (vertex heights), u_e (velocity 1-forms, unit-sphere convention)
  Mass flux: F_e = 0.5*(h[tail] + h[head]) * u_e
  Continuity tendency: dh_v/dt = -delta_1(F)_v  (GIGI's delta_1 with dual areas)

Result:
  M₀ = sum_v h_v * A_v* ≈ H₀ * 4π ≈ 1.18e5 m*sr
  dM = sum_v dh_v/dt * A_v* = -sum_v delta_1(F)_v * A_v*
  |dM/M₀| = 0.0 exactly (< RECEIPT_TOL = 8 × machine_eps)
```

The algebraic reason: for any F_e, GIGI's delta_1 computes:

```
for edge e = (tail, head):
    weighted = (l_e*/l_e) * F_e     // computed ONCE
    acc[head] += weighted
    acc[tail] -= weighted
then acc[v] /= A_v*
```

So sum_v acc_before_division[v] = sum_e (weighted - weighted) = 0 exactly in IEEE 754.
Therefore sum_v delta_1(F)_v * A_v* = sum_v acc_before_division[v] = 0 exactly.

This is d ∘ d = 0 at work: the discrete Poincaré identity means the codifferential of
any 1-form has zero integrated flux, regardless of the metric. Mass is machine-precisely
conserved by the DEC continuity equation for ALL choices of F_e.

The A-grid achieves this too (centered differences cancel), but the key difference:
in DEC the VORTICITY discretization also satisfies d ∘ d = 0, which is what eliminates
the A-grid's irremovable 4.6e-11 energy drift from discrete geostrophic imbalance.

---

## §4 — Unit convention used

Working on the UNIT sphere (l_e in radians, A_v* in steradians):

- Height 0-form: h_v [m] (physical meters, same as A-grid)
- Velocity 1-form: u_e = (U₀/a) × cos(φ_mid) × ê_λ · Δpos_unit [rad/s × rad ≈ rad²/s]
- Mass flux 1-form: F_e = h_avg × u_e [m × rad²/s]
- Continuity: dh_v/dt = -delta_1(F)_v = m × rad²/s / rad² = m/s ✓

Where Δpos_unit = pos[head] − pos[tail] on the unit sphere (dimensionless chord vector).
The 1/a factor ensures delta_1 gives m/s, not m²/s (which would be wrong by a dimension).

---

## §5 — What's next: DEC bracket_step v0.2

All topology primitives are in place. From GIGI's `Lattice`:

| Primitive | Used for | Available? |
|---|---|---|
| `signed_face_orientations()` | face vorticity ζ_f | ✅ |
| `build_edge_face_incidence()` | PV advection stencil | ✅ |
| `build_vertex_edge_incidence()` | kinetic energy K_v | ✅ |
| `d_0(B_v)` | pressure gradient 1-form | ✅ |
| `delta_1(h_avg × u_e)` | continuity | ✅ (E7 proved) |
| `face_center_latlon(lwm)` | Coriolis f_f = 2Ω sin(φ_f) | ✅ in dec_ic |

The remaining implementation work (AURORA-side, no substrate changes needed):

**Step 1 — Face vorticity:**
```
zeta_f = (1/A_f) * sum_{(e, orient) in signed_face_orientations()[f]} orient.sign * u_e
f_f    = 2 * OMEGA/A_EARTH * sin(phi_face_center)   [rad/s, unit sphere Coriolis]
h_f    = mean over face vertices of h_v
q_f    = (zeta_f + f_f) / h_f                        [rad/s / m = rad/(m*s)]
```

**Step 2 — Kinetic energy per vertex:**
```
K_v = (0.5/A_v*) * sum_{(e, orient) in build_vertex_edge_incidence()[v]}
      u_e^2 * (l_e*/l_e) * (l_e*/2)
```
(contribution of each incident edge to the dual-cell kinetic energy budget)

**Step 3 — Momentum (pressure gradient + PV advection):**
```
B_v = g * h_v + K_v       [m^2/s^2, unit sphere units?]
p_e = d_0(B)_e = B[head] - B[tail]

// PV advection: for edge e with adjacent faces f_L, f_R:
q_e_avg = 0.5 * (q_{f_L} + q_{f_R})
// Perpendicular mass flux at edge e* (crossing face f_L and f_R boundary):
// F_e_perp = h_e* * u_e_perp  — requires Perot reconstruction
du_e/dt = q_e_avg * F_e_perp - p_e
```

**The hard part is F_e_perp.** This is the mass flux PERPENDICULAR to primal edge e,
flowing through the dual edge e* (which connects the centers of f_L and f_R).

For energy conservation, the Perot reconstruction must satisfy:
sum_e du_e/dt * u_e * l_e*/l_e = 0  (zero net work by PV advection)

This is the discrete analogue of the continuous identity that Coriolis + PV flux does no work.

**Question for GIGI:** Is there an existing `perot_reconstruction` or `TriSK_weights` utility
in the lattice module (or planned)? The Thuburn-Ringler-Taylor-Skamarock (TRiSK) weights give
the Hamiltonian-structure-preserving PV advection on arbitrary meshes. If this is a substrate-
side primitive, I should consume it rather than implement it locally.

If not (it's a consumer-side responsibility), I'll implement:
- For each face f with edges {e₁, e₂, e₃, e₄}: the perpendicular flux at edge eᵢ due to
  the tangential flux at edge eⱼ (i≠j) involves a stencil coefficient derived from the face
  geometry (the "rotation" operator within the face).

---

## §6 — E8 (receipt gate) target

Once the full bracket_step is implemented:

E8: For T2 IC on cubed_sphere C=4 with barycentric dual areas,
    after one DEC bracket_step (forward Euler at dt=3600s):
    |ΔE/E₀| < RECEIPT_TOL  (machine-precision energy conservation)

This is the gate that closes the 25,845× gap from the A-grid irremovable floor.

---

## §7 — Summary

| What | Status |
|---|---|
| vertex_positions() integration | ✅ working |
| E-suite (7 DEC IC + mass conservation gates) | ✅ 7/7 green |
| DEC mass conservation to machine precision (E7) | ✅ |
| Total AURORA gates | **58/58** |
| DEC bracket_step momentum + PV advection | ⏳ in progress |
| E8 (receipt gate: ΔE/E < RECEIPT_TOL) | ⏳ pending bracket_step |

The substrate is clean. The remaining work is AURORA-side physics.
One question to GIGI: does the lattice module have (or plan to have) a
TRiSK-style perpendicular mass flux reconstruction utility?

---

— Bee Rosa Davis & Rory, Principal AURORA Engineer (2026-06-23)
