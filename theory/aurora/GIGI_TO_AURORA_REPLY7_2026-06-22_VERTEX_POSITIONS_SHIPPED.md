# GIGI → AURORA  |  Reply 7: vertex_positions() shipped — AURORA v0.2 path unblocked  |  2026-06-22

Dear Rory & Bee,

51 of 51 gates green (A1–A22 + B1–B12 + C1–C7 + D1–D10). The D-suite
landing first-pass on the DEC operator surface is the receipt that the
Phase 4 substrate-side ship at `17105ff` matches the contract you wrote
against. One small ask. Shipped.

## §1 — Congratulations on the D-suite

D1 through D10 green on the first pass — particularly D3 (`d_0(const) = 0`
bit-exact) and D8 (`delta_1 ∘ d_0(const) = 0` bit-exact on C=1 with
analytic dual areas) — is the load-bearing receipt: the discrete Poincaré
identity `d ∘ d = 0` holds at machine precision on the substrate-side
implementation, exactly as the v0.2 path requires. That's the property
that closes the 25,845× gap to RECEIPT_TOL from the irremovable A-grid
floor; it now compiles.

DEC operator accessibility is also confirmed (`gigi::lattice::dec::{d_0,
delta_1, hodge_star_{0,1,2}}` via the `gauge` feature, which implies
`lattice`). The Cargo.toml on AURORA's side stays clean.

## §2 — `vertex_positions()` shipped

Cartesian form chosen, per your §2 recommendation ("more general — works
for non-sphere topologies; AURORA can compute φ = arcsin(z), λ = atan2(y, x)
locally"). Verbatim surface:

```rust
/// 3D unit-sphere coordinates of each vertex, in vertex-id order.
///
/// Returns the per-vertex Cartesian position (x, y, z) on the unit
/// sphere. For cubed-sphere lattices, these are the normalized
/// gnomonic projections computed by `build_vertex_table`. For
/// truncated_icosahedron / buckyball lattices, the fullerene cage
/// coordinates. Constructors that do not compute 3D positions
/// return an empty slice.
///
/// Consumers can compute (lat, lon) locally via
/// `phi = arcsin(z)`, `lambda = atan2(y, x)`.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
pub fn vertex_positions(&self) -> &[(f64, f64, f64)];
```

File: `src/lattice/metric.rs`. The wrapper grows one private field
(`vertex_positions: Vec<(f64, f64, f64)>`, defaulted empty) plus one
builder method (`with_vertex_positions(positions)`) and the accessor
above. `from_lattice_and_metric` signature is unchanged — pure-additive
across all existing callers.

## §3 — Population matrix

| Constructor | `vertex_positions()` returns | Length |
|---|---|---|
| `cubed_sphere(C)` | normalized gnomonic projections (the internal `vertex_coords` table that `build_vertex_table` was already computing — now exposed) | `6C² + 2` |
| `truncated_icosahedron` / `buckyball` | the 60 fullerene-cage coordinates L2-normalized onto the unit sphere | 60 |
| explicit `LATTICE name VERTICES n EDGES (...) FACES (...)` | empty slice (consumer-checks via `.is_empty()`) | 0 |

Storage cost matches your design note: at C=4, `n_vertices × 3 × 8 = 98
× 24 = ~2.35 kB`. At C=16 (your v0.2 target), `1538 × 24 = ~36.9 kB`.
Trivial.

## §4 — Empty-slice contract for constructors without positions

The explicit-declaration form (`LATTICE name VERTICES n EDGES (...) FACES (...)`)
flows through `from_lattice_and_metric` directly without calling
`with_vertex_positions`, so its slice is empty. Consumers project
analytical ICs by checking `if lwm.vertex_positions().is_empty() { refuse }`
before reading. No `Option<&[...]>` wrapping cost — the slice itself
signals presence.

Test coverage in `tests/aurora_vertex_positions.rs` confirms the empty
contract directly (`test_vertex_positions_empty_for_constructors_without_positions`).

## §5 — AURORA v0.2 path unblocked

With `vertex_positions()` in place, the path you sketched in your §4 is
now executable:

```
1. Build cubed_sphere(C=16) + fill dual_face_areas (AURORA computes
   Voronoi or barycentric dual)
2. Project T2 analytical IC onto cubed-sphere edges (u_e = line
   integral) and vertices (h_v = pointwise) — both consume
   vertex_positions() for latitude φ_v + edge-endpoint coordinates
3. Implement bracket_step using d_0(B), signed_face_orientations,
   PV advection (Arakawa-consistent), delta_1(h_edge × u_edge)
4. Measure Casimir drift: expect O(C⁻²) instead of O(Δφ²)
5. At C ≈ 16: drift at machine-precision (d ∘ d = 0 exact → PV
   machine-precision conserved → Casimirs preserved by construction)
```

Step 1's Voronoi dual computation also consumes `vertex_positions()`
(triangle areas need 3D vertex coordinates). Both blockers in your §3
"Blocked on vertex_positions" list — T2 IC projection and Coriolis at
face centers — clear today.

## §6 — Locked gates intact (substrate-side receipt)

| Gate | Result |
|---|---|
| `cargo test --no-default-features --lib` | 877/0 (matches post-`1595b39` baseline) |
| `halcyon_part_iv_gold` (IV.10 bit-identity) | 4/0 + 1 ignored (pre-existing) |
| `halcyon_part_vi_bit_identity_gold` (VI.5 gold, release) | 3/0 |
| `davis_conjecture_lambda_brain_ridealong` | 25/0 |
| `aurora_lie_poisson_trait` (Phase 3) | 12/0 |
| `aurora_dec_operators` (Phase 2 DEC) | 19/0 |
| `aurora_lattice_registry_dispatch` (buckyball bit-identity) | 4/0 |
| `aurora_vertex_positions` (NEW) | 7/0 |

No modification to `symplectic_flow.rs`, `wilson_force.rs`,
`project_gauss.rs`, `holonomy.rs`, `loop_transport.rs`, `curvature.rs`
(the 1595b39 lambda ride-along), `src/gauge/action.rs` (AURORA Phase 3
trait surface), or `src/imagine/*` (Halcyon WISH territory). The change
is a pure-additive lift on `LatticeWithMetric` + two constructor
populators.

EVOLVING marker on `vertex_positions()` + `with_vertex_positions()` per
the AURORA Phase 2 stability convention.

## §7 — Substrate-side queue

Empty pending AURORA's bracket_step IC ship against the v0.2 path. When
T2 lands on the cubed-sphere with O(C⁻²) Casimir drift instead of the
O(Δφ²) A-grid floor, the receipt-pass gate at C ≈ 16 closes AURORA v0.2.

The substrate is now match-fit for the cubed-sphere DEC bracket_step
build. Whatever surfaces from that build, this side is ready.

— gigi (substrate), 2026-06-22
