# U(1) LINKING — SHIPPED 2026-07-18

Live U(1) group math + INIT FROM BUNDLE U(1) + HOLONOMY U(1). Completes
Hallie's one open ask: the Navier–Stokes vortex linking-number reading
`∮_C A·dl = κ·Lk(C1, C2)` via HOLONOMY on a chosen U(1) vortex field.

## The four pieces

| Piece | File | What |
|-------|------|------|
| A — U(1) group math | `src/gauge/group_element.rs` | `compose`/`inverse`/`re_trace_half` U1 arms + `normalize_phase` (principal branch `(-π, π]`). SU(2)/SU(3) byte-untouched; Z(N) still panics. |
| B — U(1) buffer | `src/gauge/dense_link_buffer.rs` | `new_identity(U1)` (repr_dim=1, all θ=0), `read_element → U1{θ}`, `write_u1_row`. `new_haar(U1)` stays unsupported. |
| C — INIT FROM BUNDLE U(1) | `src/gauge/inject.rs`, `src/gauge/u1_gauge_field.rs`, `src/gauge/registry.rs`, `src/parser.rs` | `u1_buffer_from_bundle` (theta arity gate, θ/−θ orientation store, typed errors) → `U1GaugeField` (new struct, `GaugeFieldHandle`, plain dyn register) → executor `GROUP U(1)` arm. |
| D — HOLONOMY U(1) | `src/holonomy_cycle.rs` | Group gate admits U(1); circulation arm sums raw `Σ±θ` (unwrapped) — NOT `walk_loop` (its `su2_identity` seed panics on `SU2∘U1`). Row adds `phase`; `order_estimate_u1` sentinel. |

## The receipt (verified) — genuine enclosed flux

`tests/u1_linking_basic.rs` — anchor `U1-LINK`. A **genuine** vortex
linking of two distinct curves (not a loop summing flux on its own edges):
a `+κ` flux tube along z through plaquette `(1,1)–(2,2)` and a `−κ` tube
through `(3,1)–(4,2)` (curl-free away from the two cores, total flux 0 on
the torus). HOLONOMY of a **disjoint planar `xy`-loop** (the `EDGES` form)
reads the flux it **encloses** = `κ·Lk` by discrete Stokes:

- encircle +κ core (Lk=1) → `κ`; a *different* loop around the same core →
  `κ` through a disjoint edge (⇒ enclosed flux, not a painted edge);
- encircle neither (Lk=0) → `0`; encircle both cores → `0` (fluxes cancel);
- wind twice → `2κ`; wind 20× → `20κ` (7.4 > 2π, kept unwrapped);
- reverse the circulation (−κ core) → `−κ` (linking-sign antisymmetry).

All exact to 1e-12, through the live GQL path (bundle → `INIT FROM BUNDLE`
→ `HOLONOMY … AROUND CYCLE EDGES …`). The `AXIS z` form walks the straight
core direction (the SU(2) lens-order readout / U(1) circulation *along* the
core), not an encircling loop — a vortex *linking* is the planar `EDGES`
loop. Phase sign correct (not flipped); round-trip exact to 1e-12.

## Commits (TDD, no Co-Authored-By)

```
tests(u1-group): RED — U(1) group math + DenseLinkBuffer U(1) identity arm
impl(u1-group): GREEN — U(1) abelian phase math + DenseLinkBuffer U(1) arm
tests(u1-inject+holonomy): RED — INIT FROM BUNDLE U(1) + HOLONOMY U(1) + κ·Lk
impl(u1-inject+holonomy): GREEN — INIT FROM BUNDLE U(1) + HOLONOMY U(1) live
docs(halcyon): U(1) group math + INIT FROM BUNDLE U(1) + HOLONOMY U(1) — NS vortex linking live
fix(u1-linking): genuine enclosed-flux vortex linking receipt — retire tautological collinear-flux fixture
```

## Repair (skeptic follow-up)

The first `U1-LINK` fixture painted κ on the z-edges of column `(2,2)` and
then measured the z-cycle at that **same** column — the "vortex" edges were
a subset of the measurement loop's own edges, so `phase = Σ(own edges) =
n·κ` was the *definition* of holonomy, tautologically true regardless of
any linking (no second, disjoint curve; net transverse flux 0). The U(1)
arithmetic and the raw signed-sum circulation code were **correct** and
unchanged; only the fixture was rebuilt into the genuine enclosed-flux
vortex–antivortex pair above (loop disjoint from and encircling the core,
Stokes-enclosed `κ·Lk`, sign + magnitude + Lk=0/1/2/20 + both-cores cases).

## Return shape (HOLONOMY U(1))

`phase` (raw `∮A·dl`, unwrapped — the load-bearing NS field), `re_trace =
cos θ`, `q = (cos θ, 0, 0, sin θ)` (U(1) ⊂ SU(2) σ₃ embed), `order_estimate`
(rational winding → order, else `0` sentinel for continuous κ),
`group_used = "U(1)"`. Client reads `Lk = phase / κ`.

Phase convention: single-element group law normalizes to `(-π, π]`; the
HOLONOMY circulation is kept UNWRAPPED so `Lk > 1` survives.

## Gates

- `cargo check … --bin gigi-stream` (full feature set) — green.
- `cargo test --no-default-features --lib` — 927 passed.
- SU(2) fences `gauge_inject_basic` (10/10), `holonomy_cycle_basic` (7/7) — green.
- New `u1_linking_basic` — 10/10; new lib units (u1_gauge_field, inject U1,
  order_estimate_u1) — 13/13.
- SU(3) `gauge_su3_basic`/`gauge_su3_persistence`, walker, spectral gauge,
  u1_flux, helicity, ingest, chern/betti/obstruction/topology, aurora,
  workflow e2e — all green. SU(2)/SU(3) math untouched.

## Scope (honest)

A gigi query reading the circulation / linking number of a **chosen** U(1)
field. Evidence in Bee's geometric framework, **not** a proof of NS
regularity. Completes the NS observable pair: HELICITY `∫A∧dA` (already
live) + linking `∮A·dl = κ·Lk` (this ship).
