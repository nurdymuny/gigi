# GAUGE_FIELD INIT FROM BUNDLE — chosen-field → registry seam · SHIPPED 2026-07-18

The missing half of the Poincaré / Navier–Stokes holonomy story. Yesterday's
HOLONOMY ship left one gap: the `GAUGE_FIELD … INIT …` surface offered
IDENTITY / HAAR / FROM(field) / FLUX(U1) only, so a *chosen* per-edge SU(2)
field could reach `gauge::registry` only through the test-only
`SU2GaugeField::from_buffer` factory — the lens p-sweep receipt was
gate-locked to unit tests. This seam adds the GQL path, so the receipt runs
live: **bundle → registry → HOLONOMY**, order tracks `p`, nothing planted by
hand. No new group math — it reuses the untouched `walk_loop` / `edge_element`
read path and mirrors the `INIT HAAR_RANDOM` registration path.

Base `91115c6` · RED `754a425` · GREEN `afc6b91` · worktree
`worktree-wf_9b117350-44c-2`.

## Verb

```
GAUGE_FIELD <name> GROUP SU(2) INIT FROM BUNDLE <bundle> ON LATTICE <l>;
```

Disambiguated from the existing `INIT FROM <field>` (clone a registered
field's buffer) by the `BUNDLE` keyword right after `FROM`. Reads an
edge-endpoint bundle: base `edge_id, vertex_a, vertex_b` (+ `config_id`),
fiber `q0, q1, q2, q3` scalar-first — the identical schema `INIT FLUX`
materializes and `INGEST … AS GAUGE_FIELD` writes. Each record plants its
chosen quaternion on the directed edge `vertex_a → vertex_b`; edges with no
record stay identity. Builds a `DenseLinkBuffer` and `register_su2`'s it, so
HOLONOMY / CHERN_CLASS read it back. It is the OPPOSITE direction of
`gauge::u1_flux` (registry → theta bundle); this is bundle → registry buffer.

## Orientation convention (load-bearing — pinned by I1 + I1b)

The lesson HOLONOMY's walk taught, run backwards. HOLONOMY reads edge `eid`
on the lattice's stored direction `edges[eid] = (u, v)`: `edge_element(Forward)`
is the buffer element as-is, `edge_element(Reverse)` is its inverse (quaternion
conjugate). So a record declaring Ω on `va → vb` is stored:

> **Ω when `resolve_edge(va, vb)` is Forward (edges already run va → vb);
> Ω.inverse() when Reverse.**

Then `edge_element(eid, resolve_orient)` recovers Ω exactly — not Ω†. A
mis-registered seam would flip every π₁ class silently. Emitters that write in
the lattice's own edge order always hit Forward (Ω stored verbatim); the
Reverse arm is there and tested so a −z-stamped constructor still round-trips.

## Validation (typed, no panics)

| condition | error |
|---|---|
| bundle name not in engine | `BundleNotFound(name)` |
| zero edge records | `BundleEmpty(name)` |
| `vertex_a`/`vertex_b` absent | `BundleFieldMissing { column }` |
| fiber columns ≠ group repr (theta bundle → SU(2)) | `FiberArityMismatch { expected:4, got }` |
| `(va, vb)` not a lattice edge | `NonLatticeEdge { vertex_a, vertex_b }` |
| `\|q\|²` off 1 by > 1e-6 | `NonNormalizedQuaternion { edge, norm }` |

**Normalization is a refusal, not a repair.** A non-unit quaternion is
rejected, not silently renormalized — `inverse == conjugate` (hence the
round-trip and the order estimate) holds only for `|q| = 1`, so a silent
renorm would hide an emitter bug and flip the reverse-edge read. Tol `1e-6`
on `|q|²`: the lens golden Ω is unit to f64 (passes), a passing quaternion is
stored verbatim (1e-12 round-trip holds), a drifted field is told at the edge.
**PERSIST is refused** on this init (the source bundle is the durable
artifact, like INIT FLUX; re-run after reopen).

## Files

- `src/gauge/inject.rs` (**new**, `#[cfg(feature="gauge")]`) —
  `su2_buffer_from_bundle`: schema arity + base-column check, identity buffer,
  per-record resolve → normalize → orientation-aware `write_su2_row`, empty
  guard. `resolve_directed` maps a non-edge / negative vertex to
  `NonLatticeEdge` (never a panic). 6 lib unit tests.
- `src/gauge/su2_gauge_field.rs` — `GaugeFieldInit::FromBundle(String)`
  variant (+ exhaustiveness arm in the SU(2)/SU(3) `new` constructors).
- `src/parser.rs` — `parse_init_clause` FROM arm peeks `BUNDLE`; SU(2)
  executor arm (scoped immutable bundle read → `inject::su2_buffer_from_bundle`
  → `register_su2`; PERSIST refused); SU(3) arm refuses (SU(2) this phase);
  SHOW GAUGE_FIELD `FROM_BUNDLE` label.
- `src/gauge/dense_link_buffer.rs` — `write_su2_row` (group-element writer,
  no `q0` zeroing, unlike `write_lie_row`).
- `src/gauge/error.rs` — 6 typed variants above (+ Display).
- `src/gauge/persistence.rs`, `src/wal.rs` — `FROM_BUNDLE` exhaustiveness arms
  that **refuse** WAL declaration (mirroring FROM_FIELD/FLUX); **no on-disk
  format byte allocated** — the WAL/.dhoom format is unchanged.
- `src/gauge/http.rs` — `FROM_BUNDLE` init label. `src/gauge/mod.rs` — module.

## Math anchors (I1..I5 — all green, `tests/gauge_inject_basic.rs`)

| # | Anchor | Result |
|---|--------|--------|
| I1 | round-trip | chosen unit SU(2) per edge → INIT FROM BUNDLE → `edge_element(Forward)` == stored Ω to 1e-12 |
| I1b | orientation pin | reverse-oriented record → Ω on declared `v→u` read, Ω† in canonical slot |
| I2 | lens p-sweep (live) | `re_trace = cos(2πq/p)` to 1e-12, `order = p` for `(p,q) ∈ {(2,1),(3,1),(5,1),(5,2),(7,1),(7,3)}` through bundle → registry → HOLONOMY |
| I3 | p=1 control | identity bundle → `re_trace 1.0`, `order 1` |
| I4 | U(1) deferral | `GROUP U(1) INIT FROM BUNDLE` → typed group error (refuses, no panic) |
| I5 | typed errors | bundle-not-found / arity / non-lattice-edge / non-normalized / empty — no panics |

TDD: RED (`754a425`, test only) → all anchors fail at the grammar (INIT FROM
BUNDLE parses as `INIT FROM <field> "BUNDLE"`, GAUGE_FIELD loses ON LATTICE).
GREEN (`afc6b91`) → 10/10.

## The receipt (I2, live-in-process at p=5)

`L=5, DIM=3` periodic cubic `lens5`; a bundle whose z-wrap at `(0,0)` carries
`Ω = (cos 2π/5, 0, 0, sin 2π/5)`, identity elsewhere:

```
GAUGE_FIELD lens5_field GROUP SU(2) INIT FROM BUNDLE lens5_p5_bundle ON LATTICE lens5;
HOLONOMY lens5_field AROUND CYCLE AXIS z AT (0, 0);
-- re_trace = cos(2π/5) = 0.309017, q3 = +0.951057, order_estimate = 5, group_used = "SU(2)"
```

The same row H2 produces as a unit test, now through the full public path with
nothing planted by hand — this is the p-sweep receipt the HOLONOMY letter
noted was gate-locked. The Poincaré `p ∈ {1,2,3,5,7}` sweep is now a GQL
script.

## Gates (all green, on the isolated worktree)

- `cargo check --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --bin gigi-stream` ✓
- `cargo test --no-default-features --lib` → 927 passed ✓ (optionality contract — the seam is `gauge`-gated, zero effect on the no-feature build)
- `cargo test --features halcyon --test gauge_inject_basic` → 10/10 ✓ (I1..I5)
- `cargo test --features halcyon --test holonomy_cycle_basic` → 7/7 ✓ (the fence this seam feeds — stays green)
- gauge lib (`gauge::`) 112 ✓ (incl. the 6 inject.rs unit tests)
- chern_class_basic (6), chern_class_bundle_target_basic (11), betti_pi1_basic (8), obstruction_basic (5), topology_verbs_gql_integration (9) ✓
- spectral_gauge_basic (21), u1_flux_basic (12), helicity_basic (12) ✓
- gauge_su3_basic (11), gauge_su3_persistence (4) ✓
- ingest_as_gauge_field_basic (18), ingest_gauge_vertex_basic (8) ✓
- halcyon_part_iv_gold (4, 1 ignored) ✓

## Unblocks both Millennium reads

- **Poincaré** — the twisted-BC SU(2) lens field (Ω on z-wrap) → HOLONOMY
  order = p = |π₁(L(p,q))|. Live now (I2/I3).
- **Navier–Stokes linking number** — the topological helicity
  `∮_{C₂} A₁·dl = κ₁·Lk(C₁, C₂)` (complementing HELICITY's `∫ A ∧ dA`) is the
  same INIT FROM BUNDLE → HOLONOMY shape on a `theta` bundle. Blocked only by
  the U(1) fast-follow below.

## Not built (named — U(1) fast-follow)

`GROUP U(1) INIT FROM BUNDLE` returns the typed group error this phase. The
seam itself is group-agnostic; the blocker is U(1) has no live group math yet:
`GroupElement::U1` panics in `compose`/`inverse`/`re_trace_half`, there is no
U(1) `DenseLinkBuffer` arm, no `register_u1`. A HOLONOMY walk over a U(1)
field would panic at the first `compose`. Ordered follow-up: (a) live U(1)
compose/inverse/re_trace, (b) U(1) buffer read/write arm (`repr_dim=1`,
`theta`), (c) `U1GaugeField` + `register_u1`, then (d) INIT FROM BUNDLE U(1)
is one match arm. Not blocked on the seam; not blocking the Poincaré receipt.

## Deploy (orchestrator-sequenced)

Code + gates are green on the isolated worktree. The live L4 prod probe
(`re_trace = 0.309017, order = 5` against `gigi-stream.fly.dev`) is gate-locked
by I2 — the in-process test runs the identical `bundle → register_su2 →
execute_holonomy_cycle` path — and is left to the merge + deploy phase, which
must cherry-pick onto the concurrent Poincaré + Thread-2 (sparse-interior)
sessions' `main` and coordinate the fly lease + the known snapshot-durability
wedge. This ship touches only GAUGE_FIELD INIT (parser + gauge registration),
disjoint from Thread-2's spectral interior solver.
