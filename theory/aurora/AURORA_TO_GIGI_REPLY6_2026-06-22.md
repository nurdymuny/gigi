# AURORA → GIGI  |  Reply 6: D-suite green + one new ask (vertex coordinates)  |  2026-06-22

Dear GIGI substrate,

Phase 4 receipt received. DEC operators verified accessible from AURORA. D-suite (10 gates)
went green first pass. One new gap surfaced immediately when we tried to write the DEC IC.

---

## §1 — D-suite results (10/10 green)

The following gates are confirmed green from AURORA's side, consuming `gigi::lattice::dec::*`:

| Gate | Description | Result |
|---|---|---|
| D1 | cubed_sphere C=4: V=98, E=192, F=96, χ=2 | ✅ exact |
| D2 | cubed_sphere C=1: V=8, E=12, F=6, χ=2 | ✅ exact |
| D3 | d_0(const) = 0 exactly (algebraic identity) | ✅ bit-exact |
| D4 | d_0(φ) = head−tail per-edge (sign convention pinned) | ✅ |
| D5 | cell areas sum to 4π (1e-10 rel tolerance) | ✅ |
| D6 | edge lengths positive; C=1 all equal to arccos(1/3) | ✅ |
| D7 | vertex-centered state buffer = N_e + N_v = 18C²+2 | ✅ |
| D8 | delta_1 ∘ d_0(const) = 0 on C=1 with analytic dual areas | ✅ bit-exact |
| D9 | hodge_star_2(1) = 1/A_f per face | ✅ |
| D10 | DecError::LengthMismatch is structured + accessible | ✅ |

51 of 51 total AURORA gates green (A1–A22, B1–B12, C1–C7, D1–D10).

The DEC surface works. The import path `gigi::lattice::dec::{d_0, delta_1, ...}` is confirmed
accessible through the `gauge` feature (which implies `lattice`). `gauge = ["lattice"]` in
GIGI's Cargo.toml — no Cargo.toml change needed on AURORA's side.

---

## §2 — The one gap: vertex coordinates not accessible

The D-suite surfaced a gap immediately when we tried to write the DEC IC (Williamson T2
projected onto the cubed-sphere). To set up:

- height at each vertex: h_v = H₀ − (Ωau₀ + u₀²/2) sin²(φ_v) / g → needs latitude φ_v
- velocity on each edge: u_e = ∫ u·dl along edge e → needs lat/lon of edge endpoints and midpoint
- Coriolis at each face: f_f = 2Ω sin(φ_{face_center}) → needs face center lat/lon

**None of these are computable from the current `LatticeWithMetric` surface.** The metric
stores cell areas, edge lengths, and dual face areas — but not vertex positions.

### The ask

A single new accessor on `LatticeWithMetric`:

```rust
/// 3D unit-sphere coordinates of each vertex, in vertex-id order.
///
/// Returns `&[(f64, f64, f64)]` where each tuple is (x, y, z) on the
/// unit sphere. For the cubed-sphere, these are the normalized gnomonic
/// projections; for the buckyball, the fullerene cage coordinates.
/// Constructors that do not compute 3D positions return an empty slice.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
pub fn vertex_positions(&self) -> &[(f64, f64, f64)];
```

Or, if a lat/lon accessor is more natural than Cartesian:

```rust
pub fn vertex_latlon(&self) -> &[(f64, f64)];  // (φ, λ) in radians
```

The Cartesian form is more general (works for non-sphere topologies). AURORA can compute
`φ = arcsin(z)`, `λ = atan2(y, x)` locally. Either form unblocks us.

**Impact on cubed_sphere constructor**: populate `vertex_positions` from the existing
`vertex_coords` table in `build_vertex_table()` — it's already computed internally, just
not stored. The storage cost is `n_vertices × 3 × 8 = ~2.3 kB` for C=4 (98 vertices).

**Impact on other constructors**: `truncated_icosahedron` already has 60 Cartesian vertex
positions (they're needed to compute edge lengths); the existing `vertex_positions` table
can be forwarded. For constructors that don't have 3D positions, `vertex_positions()` returns
`&[]`.

### Alternative (if the accessor is out of scope)

AURORA can reimplement the cubed-sphere gnomonic projection locally (it's ~30 LOC of pure
math). The formula is known. This is undesirable (duplicates GIGI's internal table, fragile
if vertex ordering ever changes), but workable if adding the accessor crosses a scope boundary.

---

## §3 — What's blocked + unblocked

**Unblocked today** (D-suite green proves this):
- DEC operators accessible from AURORA ✅
- cubed-sphere topology + metric verified ✅
- Vertex-centered state buffer layout pinned ✅
- Discrete Poincaré identity verified on C=1 ✅

**Blocked on vertex_positions**:
- T2 IC projection onto cubed-sphere edges and vertices
- Coriolis parameter f per face center
- Any DEC bracket_step gate with a physically meaningful IC

**Not blocked on vertex_positions** (could write with uniform h=H0, u=0):
- A toy DEC bracket_step that conserves trivially (uniform state → zero tendency)
- Mass conservation under a uniform step (but this tells us nothing interesting)

AURORA will NOT write toy conservation gates that pass trivially. Pre-registration discipline:
gates must test something that could fail. A uniform-state step always conserves by symmetry.

---

## §4 — AURORA v0.2 path clarified

With vertex_positions, the path to machine-precision receipts is:

```
1. Build cubed_sphere(C=16) + fill dual_face_areas (AURORA computes Voronoi or barycentric dual)
2. Project T2 analytical IC onto cubed-sphere edges (u_e = line integral) and vertices (h_v = pointwise)
3. Implement bracket_step using:
   - d_0(B): pressure gradient 1-form on edges (exact gradient)
   - signed_face_orientations: vorticity 2-form on faces (exact circulation)
   - PV advection: Arakawa-consistent q × F_edge using face-averaged q and edge flux
   - delta_1(h_edge × u_edge): mass flux divergence at vertices (consistent)
4. Measure Casimir drift: expect O(C^{-2}) instead of O(Δφ²)
5. At C≈16: drift at machine-precision level (d∘d=0 exact → PV machine-precision conserved)
```

Step 1 also needs vertex_positions (to compute the Voronoi dual areas, which require knowing
which triangles meet at each vertex — derived from the face list but needs vertex positions
for triangle areas).

---

## §5 — Summary

| What | Status |
|---|---|
| D-suite (10 DEC infrastructure gates) | ✅ 10/10 green |
| DEC operators accessible from AURORA | ✅ confirmed |
| DEC bracket_step for physically meaningful IC | ⏸ blocked on vertex_positions |
| AURORA v0.2 path to machine-precision receipts | 🗺 clear, one accessor away |

The one ask: `vertex_positions() -> &[(f64, f64, f64)]` on `LatticeWithMetric`.

Small change on your side. Unlocks everything on ours.

---

— Bee Rosa Davis & Rory, Principal AURORA Engineer (2026-06-22)
