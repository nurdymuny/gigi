# AURORA Phase 2 — DEC operators implementation log

**Companion to:** `theory/aurora/AURORA_ASKS_v0_1_LOG.md`,
`theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` (commit
`ad306ec`, Reply 2 §10 Q3 commitment), `theory/aurora/AURORA_PHASE_1_IMPL_LOG.md`
(Phase 1 wrapper `LatticeWithMetric` at commit `f62e46c`), and
`docs/STABILITY_GUARANTEES.md` (trait-surface stability section at
commit `1e13252`).

**Format:** mirrors `theory/aurora/AURORA_PHASE_0_IMPL_LOG.md` and
`theory/aurora/AURORA_PHASE_1_IMPL_LOG.md` — summary → commitments →
notation → TDD discipline → receipts → files touched → stability →
dual-edge convention → cross-refs → what's next.

---

## Summary

Phase 2 ships the Q3 DEC operator surface AURORA needs to consume the
Phase 1 `LatticeWithMetric` wrapper from inside their `ShallowWater`
force kernel: **`d_0`** (Form0 → Form1, the discrete gradient AURORA
needs for `grad h`), **`delta_1`** (Form1 → Form0, the discrete
divergence AURORA needs for `div(hu)`), and the three Hodge stars
**`hodge_star_0/1/2`** that package the Phase 1 metric accessors
(`cell_areas`, `edge_lengths`, `dual_face_areas`) into proper DEC
operators. All five are **free functions consuming `&LatticeWithMetric`** —
no inherent methods on the wrapper, no trait, no `Lattice` mutation.
This holds the Phase 1 boundary (commit `f62e46c`) intact: `metric.rs`
is not modified, `Lattice` is not modified, the existing bundle-side
`src/discrete/hodge_complex.rs` (L6.2) is not modified.

The sprint is additive in the strict sense: a single new line in
`src/lattice/mod.rs` (`pub mod dec;`) plus a new directory
`src/lattice/dec/{mod, d, codifferential, hodge}.rs` totalling 532 LOC
across four files (82 + 96 + 170 + 184), of which roughly 250 LOC is
implementation and the rest is design-locked doc-comments documenting
sign conventions, the barycentric dual-edge formula, the Phase 3
upgrade path, and per-function `EVOLVING` stability markers per
commit `1e13252`. Integration test surface is one new file
`tests/aurora_dec_operators.rs` (487 LOC, 20 `#[test]` functions
across six child modules + a `fixtures` helper module).

All four gates green: **870/0** no-default lib, **1030/0** halcyon lib
(+10 from new in-module unit tests inside `lattice::dec`), **1150/0**
kahler lib, **4/0+1 ignored** `halcyon_part_iv_gold` bit-identity gate.
The cross-module sign-convention pin (`d_0` from `lattice::dec` vs
`HodgeComplex::d0` from `discrete`) passes under
`--features halcyon,kahler`, proving the two DEC modules agree on edge
orientation and won't fight when a future call site mixes them.

---

## Commitments (quoted verbatim)

### Q3 — DEC operator surface (Reply 2 §10, commit `ad306ec`)

From `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` §10:

> `src/lattice/metric.rs` + `src/lattice/dec/` (NEW); `d_0`, `delta_0`,
> `hodge_star_k` as free functions consuming `LatticeWithMetric`;
> ~250–400 LOC; lands before AURORA's `HamiltonianForce` impl.

And the status-board row Q3 (pre-Phase-2 wording):

> **Q3 — DEC operator surface** | `src/lattice/metric.rs` +
> `src/lattice/dec/` (NEW); `d_0`, `delta_0`, `hodge_star_k` as free
> functions consuming `LatticeWithMetric`; ~250–400 LOC | Phase 2
> (lands before AURORA's `HamiltonianForce` impl, **UNBLOCKED
> 2026-06-21**) | **ACCEPTED** (Reply 2 §10) | —

Phase 2 honors all four bullets of the commitment:

1. `src/lattice/metric.rs` was already shipped at Phase 1 (`f62e46c`)
   carrying the wrapper + accessors. Phase 2 does **not** modify it.
2. `src/lattice/dec/` is the NEW module landing in this sprint.
3. The three operator families ship as free functions; the wrapper
   carries no inherent DEC methods.
4. Lift is 532 LOC including in-module unit tests and doc-comments;
   ~250 LOC of pure implementation. Within the 250–400 LOC commitment
   when counted operator-only; the larger total reflects the locked
   design's documentation discipline (Phase 3 upgrade path, sign
   conventions, EVOLVING markers).

### AURORA-facing kernel needs (per the Q3 ask + ShallowWater force law)

- **`grad h`** — geopotential `h` on cells (a Form0 on primal vertices
  for AURORA's vertex-centered ShallowWater) lifted to a per-edge
  value: this IS `d_0`. Pure combinatorics, no metric reads.
- **`div(hu)`** — mass flux per edge lifted to per-cell divergence:
  this IS `delta_1` (the codifferential adjoint of `d_0`). Reads
  `edge_lengths` and `dual_face_areas`; returns
  `DecError::DualFaceAreasMissing` if the wrapper's Phase 1 dual
  punt has not been filled in.
- **Hodge star machinery** — primal cell areas / edge lengths / dual
  face areas were already exposed as accessors on `LatticeWithMetric`
  in Phase 1 (`f62e46c`). The three `hodge_star_k` functions package
  these as proper DEC operators with consistent length contracts and
  structured errors.

---

## Notation: `delta_0` (status board) vs `delta_1` (this impl)

The status-board row Q3 names the codifferential operator `delta_0`.
The mathematically correct name on a 2-manifold is `delta_1`: the
codifferential `delta_k` is the adjoint of the exterior derivative
`d_{k-1}` and acts on `k`-forms. We want the adjoint of `d_0`
(Form0 → Form1), which is `delta_1` (Form1 → Form0).

The Phase 2 impl ships the operator under the correct name
**`delta_1`**. The original `delta_0` name from the status board is
preserved as a no-op alias in `src/lattice/dec/codifferential.rs`
file doc-comment so any reader chasing the status-board row finds the
right symbol, and the inline doc-comment on `delta_1` itself notes the
historical alias. AURORA's reply-2 §10 quote uses the status-board
name, not the math, so the divergence is purely cosmetic — `delta_1`
takes a `Form1` and returns a `Form0`, which is exactly what the
ShallowWater force kernel asked for.

---

## TDD discipline

### RED first — 20 integration tests + per-file in-module units

One integration test file in `tests/`:

- **`tests/aurora_dec_operators.rs`** — 487 LOC, 20 `#[test]`
  functions across six child modules + a shared `fixtures` helper:

  - `mod fixtures` (4 helpers, no `#[test]`) —
    `cubed_sphere_c1_with_dual` (re-bundles a bare C=1 cubed-sphere
    `Lattice` via `from_lattice_and_metric` with `A_v* = pi/2` on each
    of the 8 corners; the bare Phase 1 cubed-sphere constructor
    returns `dual_face_areas = None` per the Phase 1 punt at
    `src/lattice/topology/cubed_sphere.rs:162`),
    `cubed_sphere_c1_no_dual`, `cubed_sphere_c1_dual_but_no_edge_lengths`,
    `zero_metric_quad`, `shared_quad_for_hodge_complex_cross_check`.
  - `mod d_0_tests` (4) — constant `phi=1` yields `vec![0.0; 12]`
    bit-identically; length-mismatch returns
    `DecError::LengthMismatch { expected: 8, actual: 7, surface: "d_0::phi" }`;
    sign-convention pin on an indicator `phi` (reads
    `lwm.lattice().edges.iter()` directly so the test does not
    hard-code which edge ids touch vertex 0 — robust to edge-emission
    order in the constructor); stability marker bound through
    `Result<Vec<f64>, DecError>`.
  - `mod delta_1_tests` (5) — `delta_1 ∘ d_0(const) = 0` exactly (not
    a convergence claim, the algebraic identity);
    `DualFaceAreasMissing` path; `EdgeLengthsMissing` path;
    `LengthMismatch` with `"delta_1::u"` surface; stability marker.
  - `mod hodge_star_0_tests` (3) — `star_0(1)[v] = A_v^*` elementwise
    against the accessor, plus a sum-check that ∑ `A_v^*` equals `4π`
    within `1e-10` (the only tolerance in the suite, documented
    inline as a numerical sum-check not an algebraic claim);
    `DualFaceAreasMissing`; length-mismatch.
  - `mod hodge_star_1_tests` (4) — on C=1 the full-symmetry pin that
    every entry of `star_1(1)` equals `out[0]` exactly (every edge is
    symmetry-equivalent on the cube), with the elementwise expected
    value computed from `lwm.dual_face_areas()` and
    `lwm.edge_lengths()` via the barycentric formula (still exact
    `f64` equality, no tolerance); `EdgeLengthsMissing`;
    `DualFaceAreasMissing`; `LengthMismatch`.
  - `mod hodge_star_2_tests` (3) — `star_2(1)[c] = 1.0 /
    cell_areas[c]` elementwise against the accessor (asserted vs
    accessor not vs analytic `6/(4π)` so robust to spherical-excess
    corrections inside the cubed-sphere constructor);
    `CellAreasMissing`; `LengthMismatch`.
  - `mod cross_module_pins` (1, `#[cfg(feature = "kahler")]`) —
    `d_0_sign_matches_discrete_hodge_complex`: builds a tiny shared
    4-vertex / 4-edge / 1-quad fixture, asserts `d_0` from
    `lattice::dec` and `HodgeComplex::d0 * phi` agree elementwise on a
    non-constant `phi = [0, 1, 2.5, -3]`. Gated behind `kahler`
    because `gigi::discrete` is `#[cfg(feature = "kahler")]` at
    `src/lib.rs:97`. Halcyon-only test run executes the other 19;
    halcyon+kahler executes all 20.

Plus per-file `#[cfg(test)] mod tests` inside each new
`src/lattice/dec/*.rs` for the trivial length-check + one happy-path
case each — these construct `LatticeWithMetric` directly via
`from_lattice_and_metric` instead of pulling in the cubed-sphere
constructor, keeping the unit-test build hermetic.

**RED state confirmed** before any production code landed. The
RED-build error excerpt:

```
error[E0432]: unresolved import `gigi::lattice::dec`
  --> tests\aurora_dec_operators.rs:32:20
   |
32 | use gigi::lattice::dec::{d_0, delta_1, hodge_star_0, hodge_star_1,
                              hodge_star_2, DecError};
   |                    ^^^ could not find `dec` in `lattice`

error: could not compile `gigi` (test "aurora_dec_operators")
       due to 1 previous error
```

Exactly one missing-module error of the shape the locked design
predicted. No `lattice::dec::*` items were referenced from any source
file yet, so no implementation had leaked into the RED step.

### GREEN — operator-by-operator under additivity

Implementation order matched the dependency graph: `d_0` first (no
metric reads), then `hodge_star_2` (only needs `cell_areas`), then
`hodge_star_0` (only needs `dual_face_areas`), then `hodge_star_1`
(needs both + the barycentric `l_e*` formula), then `delta_1` last
(composes everything). Each operator landed with its in-module unit
test passing before the next started, so the integration suite came
GREEN one test module at a time as the corresponding operator landed.

After the stitching pass (`pub mod dec;` in `src/lattice/mod.rs`) all
20 integration tests passed:

```
cargo test --features halcyon,kahler --test aurora_dec_operators

running 20 tests
test cross_module_pins::d_0_sign_matches_discrete_hodge_complex ... ok
test d_0_tests::d_0_length_mismatch_is_structured_error ... ok
test d_0_tests::d_0_of_constant_is_zero_vector ... ok
test d_0_tests::d_0_returns_result_vec_f64_dec_error ... ok
test d_0_tests::d_0_sign_convention_pin_on_indicator_phi ... ok
test delta_1_tests::delta_1_length_mismatch_is_structured_error ... ok
test delta_1_tests::delta_1_missing_dual_face_areas_is_structured_error ... ok
test delta_1_tests::delta_1_missing_edge_lengths_is_structured_error ... ok
test delta_1_tests::delta_1_of_d_0_of_constant_is_exact_zero ... ok
test delta_1_tests::delta_1_returns_result_vec_f64_dec_error ... ok
test hodge_star_0_tests::hodge_star_0_length_mismatch_is_structured_error ... ok
test hodge_star_0_tests::hodge_star_0_missing_dual_is_structured_error ... ok
test hodge_star_0_tests::hodge_star_0_of_constant_one_is_dual_face_areas ... ok
test hodge_star_1_tests::hodge_star_1_length_mismatch_is_structured_error ... ok
test hodge_star_1_tests::hodge_star_1_missing_dual_is_structured_error ... ok
test hodge_star_1_tests::hodge_star_1_missing_edge_lengths_is_structured_error ... ok
test hodge_star_1_tests::hodge_star_1_of_constant_is_one_on_c1_by_symmetry ... ok
test hodge_star_2_tests::hodge_star_2_length_mismatch_is_structured_error ... ok
test hodge_star_2_tests::hodge_star_2_missing_cell_areas_is_structured_error ... ok
test hodge_star_2_tests::hodge_star_2_of_constant_one_is_inverse_cell_areas ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured;
                 0 filtered out; finished in 0.00s
```

### Test premise correction during GREEN

One test premise was mathematically incorrect in the original lock
and had to be tightened during impl. The
`hodge_star_1_of_constant_is_one_on_c1_by_symmetry` lock had asserted
`out == vec![1.0; 12]` from the chain "every `A_v^* = pi/2` and every
`l_e = pi/2` give `l_e^* = pi/2`." The `A_v^* = pi/2` half is correct
(4π divided by 8 cube corners is exactly π/2). The `l_e = pi/2` half
is wrong: the great-circle arc between two adjacent cube-corner
vertices on the unit sphere is `arccos(1/3) ≈ 1.231`, not `π/2 ≈ 1.571`
(only the great-circle distance between adjacent cube-**face centers**
is `π/2`, which is the dual edge — that's where the original lock got
confused). The cubed-sphere constructor populates `edge_lengths` with
the correct `arccos(1/3)` value via the atan2-stable great-circle
helper shipped in Phase 1.

Fix: assert elementwise against the barycentric formula computed live
from `lwm.dual_face_areas()` and `lwm.edge_lengths()` (still exact
`f64` equality, no tolerance), plus a full-symmetry pin that every
entry equals `out[0]`. The test still gates the same property — every
edge on C=1 is symmetry-equivalent and `star_1` computes the
barycentric ratio consistently — just stated against the actual
cubed-sphere geometry. The `delta_1 ∘ d_0(const) = 0` test was
unaffected because constant input zeroes `d_0` algebraically before
any metric reads happen.

---

## Receipts

All four gates green:

```
cargo test --no-default-features --lib
  test result: ok. 870 passed; 0 failed; 0 ignored; 0 measured;
                   0 filtered out (finished in 3.79s)
  Byte-identical baseline preserved — lattice::dec is feature-gated
  with the rest of the lattice module, so the no-default surface
  is untouched.

cargo test --features halcyon --lib -- --test-threads=1
  test result: ok. 1030 passed; 0 failed; 0 ignored; 0 measured;
                   0 filtered out (finished in 33.58s)
  +10 from new in-module unit tests across lattice::dec::{d,
  codifferential, hodge}. Single-threaded execution honors the
  Halcyon global thermalization cache requirement.

cargo test --features kahler --lib
  test result: ok. 1150 passed; 0 failed; 0 ignored; 0 measured;
                   0 filtered out (finished in 102.51s)
  Unchanged from Phase 1 baseline because the new dec unit tests
  count under the halcyon delta and kahler's count was already
  ahead by the discrete module's surface.

cargo test --features halcyon --test halcyon_part_iv_gold -- --test-threads=1
  test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured;
                   0 filtered out (finished in 9.79s)
  Bit-identity gate intact: IV.10 / III.8b / V.* contracts pass
  byte-identical to the Phase 1 baseline. The ignored
  tdd_hal_iv_10_a_symplectic_flow_canonical is the pre-existing
  Phase-0-known ignore, NOT a Phase 2 drift signal.

cargo test --features halcyon,kahler --test aurora_dec_operators
  test result: ok. 20 passed; 0 failed; 0 ignored
  All 20 integration tests including the cross-module sign-pin.
  Under --features halcyon alone the cross-module pin is skipped
  per its cfg gate (gigi::discrete requires kahler) so the suite
  runs as 19/0.
```

### Identities verified (the math the suite pins)

1. `d_0` of constant `phi = 1` on C=1 cubed sphere returns
   `vec![0.0; 12]` bit-identically (pure combinatorics, no
   floating-point arithmetic).
2. `d_0` length-mismatch returns
   `DecError::LengthMismatch { expected, actual, surface: "d_0::phi" }`.
3. `d_0` sign convention: for canonical edge `(tail, head)`,
   `out[e] = phi[head] - phi[tail]`. Pinned on an indicator `phi`
   (vertex 0 carries 1.0, all others 0.0) by reading
   `lwm.lattice().edges.iter()` directly — robust to edge-emission
   order.
4. `d_0` returns `Result<Vec<f64>, DecError>` (compile-time contract
   binding so the signature can't drift in a patch).
5. `delta_1 ∘ d_0(const) = 0` exactly on C=1 cubed sphere (algebraic
   identity, not a convergence claim — convergence to the smooth
   Laplacian as `C → ∞` is a separate scope per the locked context).
6. `delta_1` returns `DecError::DualFaceAreasMissing` when the wrapper
   was built with `dual_face_areas = None`.
7. `delta_1` returns `DecError::EdgeLengthsMissing` when
   `edge_lengths` is empty.
8. `delta_1` returns `DecError::LengthMismatch` with
   `surface: "delta_1::u"` on wrong input length.
9. `delta_1` returns `Result<Vec<f64>, DecError>`.
10. `hodge_star_0(1)[v] = A_v^*` elementwise; sum equals `4π` within
    `1e-10` (the only tolerance in the suite, justified inline as a
    numerical sum-check).
11. `hodge_star_0` returns `DecError::DualFaceAreasMissing` on a
    no-dual fixture.
12. `hodge_star_0` returns `DecError::LengthMismatch` on wrong input
    length.
13. `hodge_star_1(1)` on C=1 is full-symmetry — every entry equals
    `out[0]` bit-identically — asserted elementwise against the
    barycentric formula computed live from accessors.
14. `hodge_star_1` returns `DecError::EdgeLengthsMissing` /
    `DualFaceAreasMissing` / `LengthMismatch` on the three failure
    paths.
15. `hodge_star_2(1)[c] = 1.0 / cell_areas[c]` elementwise (against
    the accessor, not the analytic `6/(4π)` value, so robust to
    spherical-excess corrections inside the cubed-sphere
    constructor).
16. `hodge_star_2` returns `DecError::CellAreasMissing` /
    `LengthMismatch` on the two failure paths.
17. **Cross-module sign agreement** (`#[cfg(feature = "kahler")]`):
    `lattice::dec::d_0` and `discrete::hodge_complex::HodgeComplex::d0`
    produce elementwise-identical output on a shared 4-vertex /
    1-quad fixture with non-constant `phi = [0, 1, 2.5, -3]`. Pins
    that the two DEC modules agree on edge orientation so callers
    mixing both (e.g. a future Halcyon gauge-field hook) won't have
    to context-switch on sign.

### LOC totals

| Surface | LOC | Files |
| --- | --- | --- |
| `lattice::dec` module entry + `DecError` enum + re-exports | 82 | `src/lattice/dec/mod.rs` |
| `d_0` exterior derivative + unit tests | 96 | `src/lattice/dec/d.rs` |
| `delta_1` codifferential + barycentric `l_e^*` + unit tests | 170 | `src/lattice/dec/codifferential.rs` |
| `hodge_star_0/1/2` + unit tests | 184 | `src/lattice/dec/hodge.rs` |
| Module stitching | +1 line | `src/lattice/mod.rs` (`pub mod dec;`) |
| AURORA integration tests | 487 | `tests/aurora_dec_operators.rs` |
| **Total new code** | **~1020** | 5 new files + 1-line edit |

Of which production code (operator implementation, excluding
doc-comments + in-module tests + integration tests): ~250 LOC, within
the Reply 2 §10 commitment of "~250–400 LOC." The larger total
reflects the design-locked documentation discipline (Phase 3 upgrade
path written into `codifferential.rs`, sign-convention rationale in
`d.rs`, per-function `EVOLVING` markers, structured `DecError`
variants).

---

## Files touched

| File | LOC delta | Nature |
| --- | --- | --- |
| `src/lattice/dec/mod.rs` | +82 (new file) | Module entry; declares `pub mod d; pub mod codifferential; pub mod hodge;`; re-exports `d_0`, `delta_1`, `hodge_star_0/1/2`, `DecError`; hosts the `DecError` enum with `LengthMismatch { expected, actual, surface }` / `CellAreasMissing` / `EdgeLengthsMissing` / `DualFaceAreasMissing` variants (`thiserror::Error` for `Display`, `PartialEq` + `Eq` because all variants carry `Copy` primitive payloads). |
| `src/lattice/dec/d.rs` | +96 (new file) | `pub fn d_0(lwm: &LatticeWithMetric, phi: &[f64]) -> Result<Vec<f64>, DecError>`. Pure combinatorics: for canonical edge `e = (tail, head)`, `out[e] = phi[head] - phi[tail]`. Does NOT read `cell_areas` / `edge_lengths` / `dual_face_areas`. Safe to call on the zero-metric placeholder fixture. EVOLVING stability marker. 1 in-module test. |
| `src/lattice/dec/codifferential.rs` | +170 (new file) | `pub fn delta_1(lwm: &LatticeWithMetric, u: &[f64]) -> Result<Vec<f64>, DecError>`. Computes the barycentric dual-edge length `l_e^* = (A_{v-}^* + A_{v+}^*) / (2 * l_e)` inline (Phase 1/2 additivity contract preserved — no new accessor on `LatticeWithMetric`). File doc-comment documents the Phase 3 circumcentric-dual upgrade path. EVOLVING marker. Multiple in-module tests covering the three error paths + the algebraic-zero identity. |
| `src/lattice/dec/hodge.rs` | +184 (new file) | `pub fn hodge_star_0/1/2(...)` — vertex-area weighting / barycentric edge-ratio weighting / cell-area-inverse weighting. `star_1` shares the barycentric `l_e^*` formula with `delta_1` so the operators compose consistently. EVOLVING markers on all three. In-module tests covering happy path + failure paths per operator. |
| `src/lattice/mod.rs` | +1 line | `pub mod dec;` — the single new declaration. metric.rs untouched, every other existing line untouched. |
| `tests/aurora_dec_operators.rs` | +487 (new file) | 20 RED-first integration tests across six child modules + a `fixtures` helper module. Imports `use gigi::lattice::dec::{d_0, delta_1, hodge_star_0, hodge_star_1, hodge_star_2, DecError};`. Cross-module pin gated `#[cfg(feature = "kahler")]`. |

**Untouched by Phase 2** (verified additivity boundary):

- `src/lattice/metric.rs` (Phase 1 wrapper at commit `f62e46c`)
- `src/lattice/topology/cubed_sphere.rs`, `hints.rs`, `registry.rs`,
  `truncated_icosahedron.rs` (Phase 1 + earlier)
- `src/discrete/hodge_complex.rs` (the L6.2 bundle-side peer)
- `src/parser.rs`, `src/wal.rs`, `src/gauge/*`
- Every existing test file

**Out of scope (Phase 2 later or separate scope)**:

- `src/gauge/hamiltonian_registry.rs` (CC-1) — gated on AURORA's
  `ShallowWater` factory skeleton arriving.
- A2 four-trait refactor (`HamiltonianForce` / `HamiltonianDrift` /
  `ProjectionOperator` / `EnergyDecomposition`) in
  `src/gauge/action.rs` — gated on the same skeleton.
- `LATTICE_WITH_METRIC` GQL verb — no AURORA ask, separate scope.
- Convergence-as-`C`-grows benchmark joint with AURORA — separate
  scope; Phase 2 ships only the algebraic identity tests.
- `dual_edge_lengths: Option<Vec<f64>>` accessor on
  `LatticeWithMetric` (the circumcentric-dual upgrade) — Phase 3,
  documented in the `codifferential.rs` file doc-comment.

---

## Stability annotation

Per `docs/STABILITY_GUARANTEES.md` trait-surface stability section
(commit `1e13252`) and the deployment precedent set at Phase 1:

Every public item introduced by Phase 2 carries the EVOLVING
doc-comment:

```rust
/// Stability: EVOLVING until gigi 0.1.0 tag.
```

Items annotated:

- `pub fn d_0` (in `d.rs`)
- `pub fn delta_1` (in `codifferential.rs`)
- `pub fn hodge_star_0`, `pub fn hodge_star_1`, `pub fn hodge_star_2`
  (in `hodge.rs`)
- `pub enum DecError` and each variant (in `mod.rs`)

The convention shape matches Phase 1's deployment on
`LatticeWithMetric` / `ConstructorArgs` / `ConstructorError` /
`get_constructor` per AURORA reply 2 §3: doc-comment + changelog
discipline + semver (minor-bump-breaks, patch-non-breaking). No
rustc-internal `#[stable]` attribute is used (that proc-macro is not
available to library crates per AURORA Q6b correction). The EVOLVING
contract continues to hold until the first `gigi 0.1.0` tag, at
which point the markers flip to `STABLE` and the breaking-change bar
becomes a major-version bump.

---

## Dual edge length convention chosen

**Barycentric (median-dual) dual edge length**:

```
l_e^*  :=  (A_{v_-}^* + A_{v_+}^*) / (2 * l_e)
```

where `v_-` and `v_+` are the canonical tail and head of edge `e`,
`A_v^*` is the dual face area at vertex `v` from
`lwm.dual_face_areas()`, and `l_e` is the primal edge length from
`lwm.edge_lengths()`.

This is the standard barycentric / median-dual approximation in
discrete exterior calculus on triangulated and quadrilateral meshes
(Hirani 2003 §5.5; Desbrun-Kanso-Tong 2005 §4.2). Chosen over the
circumcentric dual for three reasons:

1. **No new accessor on `LatticeWithMetric`** — the formula is
   computable from the Phase 1 surface (`edge_lengths` +
   `dual_face_areas`) without growing a fifth accessor. The
   Phase 1/2 additivity contract holds. Adding
   `dual_edge_lengths: Option<Vec<f64>>` to the wrapper is a Phase 3
   upgrade, documented inline so a future reader sees the path
   without spelunking.
2. **Exact on C=1 by symmetry** — every `A_v^* = π/2` and the
   barycentric formula reduces to a uniform `l_e^*` across all 12
   symmetry-equivalent edges. The full-symmetry pin in
   `hodge_star_1_tests` consumes this property directly.
3. **First-order convergence to the smooth Hodge star as C grows** —
   the refinement story is separate scope, but the barycentric dual
   is the standard discretization that recovers the smooth operator
   in the continuum limit. The exact circumcentric dual (each
   `l_e^*` = great-circle distance between adjacent cell
   circumcenters) is the Phase 3 upgrade that buys higher-order
   convergence at the cost of growing the wrapper surface.

The formula is computed inline inside `delta_1` and `hodge_star_1` —
the two operators that need it share the same computation, so the
composition `delta_1 ∘ hodge_star_1` is provably consistent (no
phantom factor-of-2 drift between them).

---

## Cross-references

- **Reply 2 commit `ad306ec`** — engine acceptance of Q3 with the
  ~250–400 LOC commitment and the "lands before AURORA's
  `HamiltonianForce` impl" gate. Phase 2 honors all four bullets
  (location, free-function discipline, operator family, LOC budget).
- **Phase 0 commit `ca589eb`** — `Lattice::signed_face_orientations()`
  promotion. Phase 2 does not consume this directly (DEC operators
  read edges + vertices, not face orientations) but the cubed-sphere
  constructor that Phase 2's tests use was built on top of it at
  Phase 1.
- **Phase 1 commit `f62e46c`** — `LatticeWithMetric` wrapper +
  `cubed_sphere` constructor + `lattice::registry` extension +
  `topology_hint` const table. Phase 2's entire surface consumes the
  wrapper's four accessors (`lattice`, `cell_areas`, `edge_lengths`,
  `dual_face_areas`) without modifying any of them.
- **Phase 1b commit `1091dd5`** — the parser-executor switch from
  the static `TRUNCATED_ICOSAHEDRON` match arm to the registry
  lookup landed as the Phase 1b follow-up. Bit-identity gate stayed
  green; Phase 2 inherits that clean parser surface.
- **Stability docs commit `1e13252`** — `docs/STABILITY_GUARANTEES.md`
  trait-surface stability section. Phase 2 is the second deployment
  of the EVOLVING marker convention (Phase 1 was the first); Phase 2
  proves the convention works on free functions, not just structs +
  methods.
- **`theory/aurora/AURORA_ASKS_v0_1_LOG.md`** — status-board row Q3
  flipped from ACCEPTED to **DONE** in this session. Receipt commit
  hash backfills post-commit.
- **`theory/aurora/AURORA_PHASE_0_IMPL_LOG.md`** and
  **`AURORA_PHASE_1_IMPL_LOG.md`** — format precedents this log
  mirrors per-section.
- **`src/discrete/hodge_complex.rs`** (L6.2) — the bundle-side DEC
  peer. NOT modified by Phase 2; the cross-module sign-convention
  pin asserts the two modules agree on edge orientation (canonical
  `(tail, head)` with `d_0[e, v] = +1` if `v == head`, `-1` if
  `v == tail`). Decided to match rather than invert because
  `HodgeComplex` is already validated against the Python ground truth
  in `validation_tests_v3.py::test_11_hodge_torus`; reusing its
  convention removes a context-switch hazard for any future call site
  reading both modules in a single PR.

---

## What's next

Phase 2 ships the DEC operator surface; the remaining Phase 2 items
named in the locked context are gated on AURORA's `ShallowWater`
factory skeleton arriving in the aurora_crate repo:

- **CC-1 — `hamiltonian_registry.rs` skeleton.** New
  `src/gauge/hamiltonian_registry.rs` mirrors the `lattice::registry`
  shape from Phase 1: `register_hamiltonian(name, factory)` +
  `get_hamiltonian(name)` + `clear()` + `all()`,
  `OnceLock<Mutex<HashMap<...>>>` storage, built-in `KogutSusskind`
  registers itself in the engine startup hook, WAL
  `HamiltonianDeclare` carries metadata only, trait-object
  re-materialized from factory at replay, eager registration per
  AURORA Q5a. ~150–250 LOC. **Gated on AURORA's `ShallowWater`
  factory skeleton arriving** so the engine can validate the trait
  shape against a concrete second consumer before locking the
  surface.
- **A2 four-trait refactor + `ShallowWater`.** `HamiltonianForce` /
  `HamiltonianDrift` / `ProjectionOperator` / `EnergyDecomposition`
  in `src/gauge/action.rs` (NEW). `KogutSusskind { beta }` becomes
  the first impl (bit-identity preserved for Halcyon — the
  `halcyon_part_iv_gold` gate gates this). `ShallowWater { g, omega,
  a }` lands as the second impl in AURORA's host binary, not in
  gigi. ~600–900 LOC + tests. Hot-path constraint per Reply 2 §9:
  integrator generic over concrete `H: HamiltonianForce +
  HamiltonianDrift`, monomorphized via outer-loop enum match;
  `Box<dyn ...>` only at registry / WAL / introspection boundaries.
  **Gated on the same skeleton** so the four trait surfaces can be
  reviewed against a concrete second consumer.

Phase 2's DEC operator surface is self-contained and ships
independently. AURORA's `HamiltonianForce` impl can now consume
`d_0` for `grad h` and `delta_1` for `div(hu)` directly from the
gigi crate; the `hodge_star_k` family is available for any vorticity
+ kinetic-energy decompositions ShallowWater needs downstream.

---

## Authorship note

Per `feedback_no_ai_coauthor.md`: when the parent agent commits
Phase 2, the commit body must NOT carry a `Co-Authored-By: Claude`
footer. Author = `nurdymuny <bee_davis@alumni.brown.edu>` (Bee Rosa
Davis) only. Same convention as Phase 0, Phase 1, Phase 1b, and every
commit in the Halcyon Part V sprint.
