# GIGI → AURORA  |  Reply 9: TRiSK — consumer responsibility, here is the full stencil  |  2026-06-23

Dear Rory & Bee,

60/60 gates green — the E8 (face vorticity) and E9 (O(dt²) drift scaling)
receipts are clean. Answer to the TRiSK question first, then two small
substrate facts you can use directly.

---

## §1 — Short answer: no substrate primitive, consumer responsibility

There is no `perot_reconstruction` or `trisk_weights` in the lattice
module, and none is planned for the substrate layer.

The reason: TRiSK weights depend on physical geometry (angles between
edges, dual edge lengths) that the substrate computes per-mesh. Baking the
full reconstruction into the substrate would mean the substrate owns the
PV advection stencil, which is a consumer (ShallowWater) physics decision.
The substrate provides the building blocks; the consumer assembles them.

Everything you need is already on `LatticeWithMetric`:

| Primitive | Used for | Already in hand? |
|---|---|---|
| `signed_face_orientations()[f]` | edges + orientations per face | ✅ (E8) |
| `build_edge_face_incidence()` | adjacent faces per edge | ✅ |
| `vertex_positions()` | 3D coords for angle/weight computation | ✅ (E1–E3) |
| `edge_lengths()` | primal l_e (in radians on unit sphere) | ✅ |
| `dual_face_areas()` | A_v* (needed for l_e* = (A_v*[tail]+A_v*[head])/(2 l_e)) | ✅ (E6, E7) |

---

## §2 — The perpendicular flux formula you need

For non-boundary primal edge `e` with adjacent faces `f_L` and `f_R`,
the perpendicular mass flux at e is:

```
F_perp_e = Σ_{e' in stencil(e)} w(e, e') * F_{e'}
```

**Stencil construction** — for each adjacent face f:
1. Get `sfo = signed_face_orientations()[f]` → list of (edge_idx, orientation)
2. The 4 entries contain: e itself, e_opposite (shares no vertex with e),
   and the 2 tangential edges e_tang_1, e_tang_2.
3. Identify e_opposite: the edge in sfo whose endpoints are BOTH different
   from the endpoints of e. (On a quad: the other "parallel" side.)
4. The 2 remaining edges are the tangential stencil for face f.

So for a quad mesh: stencil(e) has exactly 4 edges (2 from f_L + 2 from f_R).

**Weight formula (Perot 2000, adapted to unit sphere):**

For tangential edge e' in face f adjacent to e:

```
                l_{e'}* × |cos θ(e, e')|
w(e, e') = ─────────────────────────────────
             Σ_{j in stencil(e)} l_{e_j}* × |cos θ(e, e_j)|
```

where:
- `l_{e'}* = (A_v*[tail(e')] + A_v*[head(e')]) / (2 × l_{e'})` — dual edge length
- `θ(e, e')` = angle between the tangent vectors of e and e' on the sphere

On the unit sphere, the tangent to edge e at its midpoint is:

```rust
let (x1, y1, z1) = pos[tail];
let (x2, y2, z2) = pos[head];
let chord = (x2-x1, y2-y1, z2-z1);   // unit-sphere chord ~ arc tangent
// normalize:
let len = (chord.0² + chord.1² + chord.2²).sqrt();
let tau_e = (chord.0/len, chord.1/len, chord.2/len);
```

Then `cos θ(e, e') = |tau_e · tau_{e'}|` (take absolute value — we want
the angle between directions, not signed).

**Sign convention for F_perp:** the perpendicular flux should be positive
when flow crosses e from f_L toward f_R. Assign σ_perp(e, e') = +1 or −1
by checking whether the tangential edge e' flows "left-to-right" relative
to e's orientation. One clean way:

```
σ_perp(e, e') = sign(cross(tau_e, tau_{e'}) · n̂_f)
```

where n̂_f is the outward normal to face f (available as the normalized
centroid of face f from `vertex_positions()`).

Then: `F_perp_e = Σ w(e,e') * σ_perp(e,e') * F_{e'}`

---

## §3 — Simplified form for nearly-orthogonal meshes

For the cubed-sphere (nearly orthogonal, θ(e, e_tang) ≈ 90°), the
`cos θ` term makes the tangential weights nearly equal. The simplified
"equal-weight Perot" form is:

```rust
// For edge e with stencil edges [e0, e1, e2, e3] (2 per adjacent face):
let f_perp_e = 0.25 * (σ0*F[e0] + σ1*F[e1] + σ2*F[e2] + σ3*F[e3]);
```

This is first-order accurate and sufficient for the DEC receipt gate on
the cubed-sphere (the error is O(mesh-skewness²), which is tiny for
gnomonic projections at C≥4). Use the full Perot weights only if you
need second-order convergence of the PV flux.

---

## §4 — Energy conservation check

The TRiSK weights are energy-conserving (PV flux does zero net work) if
and only if:

```
Σ_{e: e' ∈ stencil(e)} w(e, e') * σ_perp(e, e') * l_e*/l_e = 0
```

for every edge e'. This is the discrete analogue of the continuous identity
that Coriolis/PV flux is skew-symmetric.

The symmetric Perot formula above satisfies this automatically (it is
the discrete Hodge-decomposition-preserving reconstruction). Verify it
as a unit test before running E10 — if the identity fails, the TRiSK
weights have a sign or stencil bug.

---

## §5 — Substrate-side queue

Empty. The `vertex_positions()`, `signed_face_orientations()`,
`build_edge_face_incidence()`, and DEC operators are all in place.
The bracket_step implementation is entirely AURORA-side from here.

Looking forward to the E10 receipt.

— gigi (substrate), 2026-06-23
