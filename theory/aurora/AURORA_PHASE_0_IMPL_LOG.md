# AURORA Phase 0 — Implementation Log

**Companion to:** `theory/aurora/AURORA_ASKS_v0_1_LOG.md`,
`theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` (the engine
commitment letter at commit `ad306ec`).

**Format:** mirrors `theory/halcyon/HALCYON_PART_V_IMPLEMENTATION_LOG.md`
— summary → commitment quote → TDD discipline → receipts → files touched
→ cross-refs → what's next.

---

## Summary

Phase 0 ships a single additive lift: `Lattice::signed_face_orientations()`
becomes a public method on the general-purpose `Lattice` surface in
`src/lattice/mod.rs`. It returns `Vec<Vec<(EdgeId, EdgeOrientation)>>`
— outer indexed by face id, inner the ordered cycle of `(edge, sign)`
pairs for that face — computed fresh from `Lattice::faces` and
`Lattice::edges` via the existing `resolve_edge()` helper.

This is Phase 0 because AURORA's Phase 1 `CUBED_SPHERE` constructor (A1)
needs the same signed-face-orientation surface that the buckyball
constructor already produces privately inside
`truncated_icosahedron::buckyball_with_signed_faces` (lines 291–321).
Lifting it to a public `Lattice` method means `CUBED_SPHERE` calls the
promoted surface instead of duplicating face-cycle-table traversal
logic. The promotion was tagged CC-4 in the cross-cutting question
list, greenlit in Reply 2 §5 with bit-identity risk = 0, and committed
to as the standalone PR that lands FIRST in Phase 0 — ahead of A1, the
registry refactor (CC-2), and the topology-hint table.

The shipped lift is 23 LOC (including doc comment and braces),
well under the ~80–100 LOC budget named in the commitment letter. It
adds zero fields to the `Lattice` struct, modifies zero existing
methods, and is purely additive — so the byte-identical `to_gql()`
round-trip and Halcyon's bit-identity contracts (IV.10 / III.8b / V.*)
flow through unchanged.

---

## Commitment (quoted verbatim from `GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` §5, commit `ad306ec`)

> Signed-face promotion (CC-4): `Lattice::signed_face_orientations()`;
> ~80–100 LOC standalone PR. GREENLIT (engine commitment in Reply 2 §5;
> bit-identity risk = 0). Lands first in Phase 0.

Why CC-4 lands BEFORE A1, restated:

- A1's `CUBED_SPHERE` constructor needs the same signed-face surface
  the buckyball constructor builds today.
- Without the promotion, `CUBED_SPHERE` would re-implement the
  face-cycle traversal — a duplicate of `buckyball_with_signed_faces`'s
  emit closure (truncated_icosahedron.rs lines 291–321). That
  duplication is exactly the kind of "carry duplicate orientation
  logic" fallback AURORA offered and Reply 2 §5 declined.
- The lift is bit-identity-safe (no new storage on `Lattice`; computed
  fresh per call), so it lands as its own focused commit with zero
  coupling to A1, CC-2, or the topology-hint table.

---

## TDD discipline

Three integration tests in
`tests/aurora_signed_face_orientations.rs` (211 LOC, `#[cfg(feature =
"halcyon")]`-gated to pull in `truncated_icosahedron::buckyball` +
`signed_face_to_walker` + `gauge::holonomy::walk_loop`):

1. **`test_signed_face_orientations_buckyball_canonical_face_count`** —
   shape contract: outer length = 32 (12 pentagons + 20 hexagons),
   per-face inner lengths are 5 then 6 in pin-order. Anchors the
   shape AURORA's `CUBED_SPHERE` constructor will consume.
2. **`test_signed_face_orientations_consistency_with_walk_loop`** —
   walker-equivalence: drive `walk_loop` over each face's
   `(EdgeId, EdgeOrientation)` cycle under the identity `EdgeConnection`;
   assert each face holonomy is the SU(2) identity element bit-identically.
   Proves the lift is drop-in for the existing TDD-HAL-I.4
   `face_edges()` consumer with no integer-tolerance drift (sign is
   `i8` ±1; `assert_eq!` on the `GroupElement::SU2` variant).
3. **`test_signed_face_orientations_buckyball_anchor`** — anti-drift
   anchor: assert that `bb.lattice.signed_face_orientations()` matches
   `signed_face_to_walker(&bb.signed_faces[fidx])` for face 0
   (a pentagon), face 12 (the first hexagon), and the full 32-face
   table. Catches any future storage-permutation refactor that
   silently re-orders the cycle.

**RED first.** The failing test landed before the implementation:

```
cargo test --features halcyon --test aurora_signed_face_orientations --no-run

error[E0599]: no method named `signed_face_orientations` found for
              struct `gigi::lattice::Lattice` in the current scope
  --> tests\aurora_signed_face_orientations.rs:83:19
  --> tests\aurora_signed_face_orientations.rs:137:19
  --> tests\aurora_signed_face_orientations.rs:168:26
```

Three E0599 hits at exactly the three promotion call sites; two
downstream E0282 type-annotation errors resolved themselves once the
method existed. RED confirmed.

**GREEN.** Implementation landed at `src/lattice/mod.rs:208`, 23 LOC.
First compile, all three tests pass:

```
cargo test --features halcyon --test aurora_signed_face_orientations

running 3 tests
test test_signed_face_orientations_buckyball_canonical_face_count ... ok
test test_signed_face_orientations_buckyball_anchor ... ok
test test_signed_face_orientations_consistency_with_walk_loop ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The implementation is a true lift, not a rewrite: it mirrors the per-face /
per-position iteration pattern already used in
`build_edge_face_incidence()` (`src/lattice/mod.rs:153–172`) and
`gauge::holonomy::face_edges()` (`src/gauge/holonomy.rs:44–57`), reusing
the existing `pub fn resolve_edge()` to derive `(EdgeId, EdgeOrientation)`
from each consecutive vertex pair `(face[pos], face[(pos + 1) % n])`.
No new storage on `Lattice` — so `to_gql()` round-trip stays
byte-identical and Halcyon's bit-identity contracts flow through
unchanged.

---

## Receipts

All three test surfaces green, with byte-identical no-default-features
optionality contract intact and the load-bearing Halcyon bit-identity
gold gate clean:

```
cargo test --no-default-features --lib
  test result: ok. 870 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (finished in 3.56s; byte-identical optionality surface)

cargo test --features halcyon --lib -- --test-threads=1
  test result: ok. 996 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (finished in 16.71s; single-threaded honors the Halcyon global
  thermalization cache requirement)

cargo test --features kahler --lib
  test result: ok. 1150 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (finished in 89.60s)

cargo test --features halcyon --test aurora_signed_face_orientations
  test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo test --features halcyon --test halcyon_part_iv_gold -- --test-threads=1
  test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
  (passing: tdd_hal_iv_10_b_energy_drift_two_tier,
           tdd_hal_iv_10_c_gauss_residual_two_tier,
           tdd_hal_iv_10_d_h_total_now_returns,
           tdd_hal_iv_10_e_diagnostics_envelope_shape;
   ignored: tdd_hal_iv_10_a_symplectic_flow_canonical
            — pre-existing ignore, not a drift signal)
```

The load-bearing receipt is **`halcyon_part_iv_gold`**: the IV.10 /
III.8b / V.* bit-identity contracts flow through face cycles. If the
promotion had perturbed orientation computation, the two-tier energy
drift, Gauss residual, H_total return, or diagnostics envelope shape
tests would fail. They are byte-identical to the pre-promotion baseline.

---

## Files touched

| File | LOC delta | Nature |
| --- | --- | --- |
| `src/lattice/mod.rs` | +23 (additive only) | New `pub fn signed_face_orientations(&self) -> Vec<Vec<(EdgeId, EdgeOrientation)>>` inserted just above the existing `resolve_edge()`. No existing fields, methods, types, or call sites modified. |
| `tests/aurora_signed_face_orientations.rs` | +211 (new file) | Three RED-first integration tests, `#[cfg(feature = "halcyon")]`-gated. Re-declares a local `IdentityConnection` (~5 LOC) because `FixedEdgeConnection::identity_everywhere` is `#[cfg(test)] pub(crate)` — invisible from integration tests. |

Total: 234 LOC (23 production + 211 test). Production lift sits well
under the ~80–100 LOC budget from Reply 2 §5.

Phase 1 deferrals (NOT touched in Phase 0, per the locked
context constraints):

- `src/lattice/topology/cubed_sphere.rs` (Phase 1 A1).
- `src/lattice/registry.rs` (Phase 1 CC-2 refactor).
- `src/lattice/topology/hints.rs` (Phase 1 topology-hint table).

---

## Cross-references

- **Reply 2 commit `ad306ec`** — engine acceptance + Q3 answer + the
  Phase 0 commitment quoted above.
- **`theory/aurora/AURORA_ASKS_v0_1_LOG.md`** — the status-board row
  for "Signed-face promotion (CC-4)" flipped from **GREENLIT** to
  **DONE** in this session. Receipt commit hash is **TBD** in the row;
  the parent agent commits Phase 0 in one focused commit after this
  workflow returns and will backfill the hash into the row.
- **`theory/halcyon/HALCYON_PART_V_IMPLEMENTATION_LOG.md`** — the
  format precedent this log mirrors (per-gate entries, RED test paths,
  green criterion quotes, receipt pass-lines, no `Co-Authored-By:
  Claude` footer convention per `feedback_no_ai_coauthor.md`).
- **Halcyon bit-identity gate unaffected** — `halcyon_part_iv_gold`
  passes 4/0 (plus the pre-existing
  `tdd_hal_iv_10_a_symplectic_flow_canonical` ignore, which is NOT a
  drift signal). The IV.10 / III.8b / V.* bit-identity contracts that
  flow through face cycles are intact. The lift is purely additive on
  the public surface; it stores nothing, so `to_gql()` round-trip is
  byte-identical and Halcyon's gold buffers do not move.
- **Reference precedent inside this commit**:
  - `src/lattice/mod.rs:235–245` (`resolve_edge`) — the existing helper
    the lift delegates to.
  - `src/lattice/mod.rs:153–172` (`build_edge_face_incidence`) — the
    iteration pattern the lift mirrors.
  - `src/gauge/holonomy.rs:27–57` (`walk_loop` + `face_edges`) — the
    consumer contract the new method satisfies and the walker-equivalence
    test exercises.
  - `src/lattice/topology/truncated_icosahedron.rs:291–321`
    (`buckyball_with_signed_faces`) — the private-buckyball computation
    the new public method generalizes; the anchor test confirms
    bit-for-bit agreement.

---

## What's next

Phase 1 is unblocked. Three asks land together once the Phase 0 commit
hash is in the status-board row:

- **A1 — `CUBED_SPHERE` topology constructor.**
  `src/lattice/topology/cubed_sphere.rs` (NEW), ~150–250 LOC. Parameterized
  `PANEL_SIZE C`. 6 panels × C² cells; 12C cross-seam edges enumerated
  by panel-coordinate arithmetic per the CC-3 constructor-owned
  resolution. Constructor returns `Lattice` (Phase 2 wraps in
  `LatticeWithMetric` after the Q3 DEC module lands). One parser arm in
  `src/parser.rs` (~line 8778, peer to `"TRUNCATED_ICOSAHEDRON" =>`),
  but routed through the CC-2 registry lookup rather than the
  static match. New integration test
  `tests/lattice_cubed_sphere.rs` calls
  `Lattice::signed_face_orientations()` on the cubed-sphere lattice
  and asserts every face is a 4-cycle with sign-consistent boundary
  orientation.

- **CC-2 — `lattice::registry` open-registry refactor.**
  `src/lattice/registry.rs` grows
  `register_constructor(canonical_name, ConstructorFn)` and
  `get_constructor(canonical_name) -> Option<ConstructorFn>`.
  The parser arm at `parser.rs:8778-8787` collapses to a registry
  lookup. Buckyball + cubed-sphere both land through
  `register_constructor` at module init. ~110–175 LOC. Reply 2 §3
  rationale: lower aggregate cost than shipping A1 on a static match
  arm and retrofitting registry dispatch later.

- **`topology_hint` const table.**
  `src/lattice/topology/hints.rs` (NEW), ~30 LOC. Reserves
  `S2/CUBED_SPHERE` and `S2/TRUNCATED_ICOSAHEDRON` in a `const` table
  so `topology_hint` strings are not free-form for the two shipped
  topologies. Free-form retained for not-yet-registered topologies.

All three Phase 1 items are gated on the Phase 0 commit landing — the
focused commit the parent agent creates after this workflow returns.
Phase 2 (A2 four-trait refactor + `ShallowWater` + CC-1
`hamiltonian_registry` + Q3 DEC module) remains gated on AURORA's
`ShallowWater` factory sketch arriving (UNBLOCKED 2026-06-21 on the
gigi side per Q4/Q5/Q6 resolution).

---

## Authorship note

Per `feedback_no_ai_coauthor.md`: when the parent agent commits Phase 0,
the commit body must NOT carry a `Co-Authored-By: Claude` footer.
Author = `nurdymuny <bee_davis@alumni.brown.edu>` (Bee Rosa Davis) only.
Same convention as every commit in the Halcyon Part V sprint and the
prior AURORA cross-team letters.
