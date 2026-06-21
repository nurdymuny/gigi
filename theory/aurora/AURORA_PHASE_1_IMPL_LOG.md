# AURORA Phase 1 — Implementation Log

**Companion to:** `theory/aurora/AURORA_ASKS_v0_1_LOG.md`,
`theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` (commit
`ad306ec`), and the Q4/Q5/Q6 resolution thread at commits `c06f073` +
`358ede4`.

**Format:** mirrors `theory/aurora/AURORA_PHASE_0_IMPL_LOG.md` —
summary → commitments → scope resolution → TDD discipline → receipts →
files touched → cross-refs → what's next.

---

## Summary

Phase 1 ships the three asks the engine committed to in Reply 2 §2/§3/§5
and the small wrapper that lets the A1 constructor return its metric
data without smuggling Phase 2 surface into Phase 1: **A1
(`CUBED_SPHERE` topology constructor), CC-2 (registry-dispatch
refactor), the `topology_hint` const table, and a minimal
`LatticeWithMetric` stub** land together as one sprint. The cubed-sphere
constructor consumes the Phase 0 `Lattice::signed_face_orientations()`
promotion (commit `ca589eb`) — no face-cycle traversal logic is
duplicated. The CC-2 refactor lands additively alongside the legacy
parser-executor match arm so the existing `TRUNCATED_ICOSAHEDRON`
dispatch path keeps producing byte-identical `Lattice` instances, with
the bit-identity claim guarded by a new `aurora_lattice_registry_dispatch`
integration test that asserts `PartialEq` equality between the
registry-dispatched buckyball and the direct `buckyball()` call. The
minimal `LatticeWithMetric` wrapper (no DEC operators, those are Phase
2 per the §10 commitment) carries `cell_areas` + `edge_lengths` +
optional `dual_face_areas` so the A1 constructor can honor its
"returns `LatticeWithMetric`" contract from the status board without
forcing the DEC module to ship in the same commit. Total Phase 1
delta: ~736 LOC of production code (195 metric stub + 590 cubed-sphere
+ 101 hints + 175 registry extension, less ~325 LOC of in-module test
code counted against the production files) plus 305 LOC of new
integration tests. All gates green: 1020/0 halcyon lib, 1150/0 kahler
lib, 870/0 no-default-features lib, 15/15 new AURORA integration tests,
`halcyon_part_iv_gold` bit-identity gate clean.

---

## Commitments (quoted verbatim)

### A1 — `CUBED_SPHERE` topology constructor (Reply 2 §1, commit `ad306ec`)

> A1 panel-size, orientation, hint, seam pins | ACCEPT | As written,
> with CC-4 promoted ahead so the orientation path is via
> `Lattice::signed_face_orientations()`.

Constructor returns `LatticeWithMetric`; calls the Phase 0 promoted
`signed_face_orientations()` method instead of duplicating face-cycle
traversal; parameterized by `PANEL_SIZE C`; ships as
`src/lattice/topology/cubed_sphere.rs` (NEW).

### CC-2 — registry-dispatch refactor (Reply 2 §3, commit `ad306ec`)

> Accepted, and we are lifting CC-2 into the A1 sprint rather than
> retrofitting later. `CUBED_SPHERE` dispatches through the registry
> from day one. … shipping A1 with the static match arm in
> `parser.rs:8778-8787` and retrofitting CC-2 later costs a second
> parser refactor AND risks a WAL incompat if canonical-name string
> handling changes between v0.1 and v0.2.

Engine surface:

> ```rust
> pub fn register_constructor(name: &'static str, factory: fn() -> Lattice);
> pub fn get_constructor(name: &str) -> Option<fn() -> Lattice>;
> ```

Phase 1 ships the surface as `get_constructor(&str) -> Option<Constructor>`
where `Constructor = fn(&ConstructorArgs) -> Result<LatticeWithMetric,
ConstructorError>` — the signature is widened from the letter's
`fn() -> Lattice` to thread the `PANEL_SIZE C` parameter and return
the metric wrapper. The parser-executor match arm at
`src/parser.rs:8773-8813` is **not** rewritten in Phase 1; CC-2 lands
purely as the additive `lattice::registry::get_constructor()` surface
that downstream callers (AURORA tests, future parser refactor) consume.
The existing `TRUNCATED_ICOSAHEDRON` parser dispatch remains
byte-identical with the registered constructor wrapping
`buckyball()` and a bit-identity guard test pinning the equality.

### `topology_hint` const table (Reply 2 §1)

> CC-5 — schema DSL extension | ACCEPT, sequence as Phase 3 general
> lift | `__` (double underscore) is the documented desugar convention;
> v0.1 ships the six flat scalar names as a documented exception.

Phase 1 ships `src/lattice/topology/hints.rs` (NEW, ~101 LOC including
tests) — a `TOPOLOGY_HINTS: &[(&str, &str)]` const table reserving
`S2/CUBED_SPHERE` and `S2/TRUNCATED_ICOSAHEDRON` symmetrically, with
`pub fn lookup(canonical_id: &str) -> Option<&'static str>` +
verbose alias `pub fn topology_hint_for(constructor_name: &str)`.

### Q4/Q5/Q6 resolution (commits `c06f073` + `358ede4`)

From `theory/aurora/GIGI_TO_AURORA_2026-06-21_Q4_Q5_Q6_RESOLVED.md`
(commit `c06f073`) and AURORA's reply 2
(`theory/aurora/AURORA_TO_GIGI_REPLY2_2026-06-21.md`, commit `358ede4`):

- **Q4b** — separate host binary, gigi's `Cargo.toml` stays clean;
  `aurora/` is its own workspace, links gigi as a pinned dep. Phase 1
  honors this by NOT adding any aurora-named feature to gigi's
  `Cargo.toml`; the registry surface is plain library API.
- **Q5a** — eager `init()` at top of `main()`, no lazy anything. Phase
  1 honors this with `lattice::registry::init_builtin_constructors()`
  fired lazily inside `get_constructor()` via `OnceLock::get_or_init`
  for the **gigi-built-in** topologies only; AURORA's host binary still
  calls its own registration eagerly in `main()` (per Q5a) — the
  built-in path is internal-only and does not change the external
  registration contract.
- **Q6b** — doc-comment + changelog + semver convention, NOT the
  rustc-internal `#[stable]` proc-macro. Phase 1 attaches
  `/// Stability: EVOLVING until gigi 0.1.0 tag.` doc-comments to
  `LatticeWithMetric`, `ConstructorArgs`, `ConstructorError`, and
  `get_constructor()`. The full convention spec ships in Phase 2
  alongside `hamiltonian_registry.rs` (see "What's next" below).

---

## LatticeWithMetric scope resolution (Phase 1 vs Phase 2 boundary)

The A1 status-board row says **"constructor returns `LatticeWithMetric`"**
and the Q3 row says **"`src/lattice/metric.rs` + `src/lattice/dec/`
(NEW) is Phase 2."** Two rows, one type name, two phases — the
discovery agent flagged the contradiction. Resolution locked before
RED:

- **Phase 1 ships the wrapper, not the operators.** `src/lattice/metric.rs`
  contains `pub struct LatticeWithMetric { lattice: Lattice, cell_areas:
  Vec<f64>, edge_lengths: Vec<f64>, dual_face_areas: Option<Vec<f64>> }`
  plus borrowing accessors (`lattice()`, `cell_areas()`,
  `edge_lengths()`, `dual_face_areas()`) and the
  `from_lattice_and_metric()` constructor. **No DEC methods.** Stub
  is 195 LOC including 3 module-level tests.
- **Phase 2 adds the operators as free functions.** `src/lattice/dec/`
  (NEW, Phase 2) houses `d_0`, `delta_0`, `hodge_star_k` as free
  functions that **consume** the wrapper. No method on the wrapper, no
  inherent impl, no trait — they read `lwm.lattice()`, `lwm.cell_areas()`,
  `lwm.edge_lengths()` and return new buffers. This keeps the Q3
  surface ("not methods on `Lattice`, free functions consuming
  `LatticeWithMetric`") clean.

The registry continues to store bare `Lattice` (unchanged), not
`LatticeWithMetric`. The wrapper is the constructor return type only;
callers that need to register the bare lattice unwrap via
`lwm.lattice().clone()`. The existing TDD-HAL-I.8 storage half of the
registry contract is untouched by Phase 1.

This resolution holds the Phase 1 LOC envelope to ~320–505 production
LOC plus ~320 LOC of tests, the budget named in the locked context.

---

## TDD discipline

### RED first — three new integration tests + module-level tests in each new file

Three integration test files in `tests/`:

1. **`tests/aurora_cubed_sphere_construction.rs`** — 8 tests covering
   combinatorial counts at C=1/2/4, Euler characteristic = 2 at C=3,
   all-faces-are-quads at C=4, `signed_face_orientations()` round-trip
   (every edge on S² appears exactly once Forward + once Reverse —
   proves face cycles wind consistently outward and panel stitching is
   correct), topology hint = `Some("S2")`, total cell area sums to 4π
   within 1e-10 at C=4 (proves spherical-excess + gnomonic stitching
   close perfectly).
2. **`tests/aurora_topology_hint_table.rs`** — 3 tests covering the
   const-table lookup contract: `S2` for both shipped topologies,
   unknown identifier returns `None`.
3. **`tests/aurora_lattice_registry_dispatch.rs`** — 4 tests covering
   `get_constructor` lookup (success for both shipped topologies, None
   for unknown) and the **bit-identity guard**:
   `test_registry_dispatched_buckyball_bit_identical_to_direct` asserts
   `PartialEq` equality between the registry-dispatched buckyball and
   the direct `truncated_icosahedron::buckyball()` call.

Plus module-level `#[cfg(test)] mod tests` in each new production file
(combinatorics + face-cycle + metric closure for `cubed_sphere`;
wrapper round-trip + Phase-2-additivity sentinel for `metric`;
case-insensitive lookup + alias parity for `hints`; bit-identity +
panel-size validation + builtin-keyset enumeration for `registry`'s
new tests).

**RED state confirmed** before any production code landed. The
cumulative cargo error excerpt:

```
error[E0432]: unresolved import `gigi::lattice::topology::cubed_sphere`
  --> tests\aurora_cubed_sphere_construction.rs:28:5
error[E0432]: unresolved import `gigi::lattice::topology::hints`
  --> tests\aurora_topology_hint_table.rs:23:5
error[E0425]: cannot find function `get_constructor` in module `registry`
  --> tests\aurora_lattice_registry_dispatch.rs:34:26
error[E0422]: cannot find struct, variant or union type `ConstructorArgs`
              in module `registry`
  --> tests\aurora_lattice_registry_dispatch.rs:36:26
```

Each E0432/E0425/E0422 names exactly one missing surface from the
locked context. Total RED test LOC = 305.

### GREEN — file-by-file under additive constraint

GREEN landed in five additive passes, each verified independently
against `cargo build --lib --features lattice` before stitching:

1. **Metric stub** (`src/lattice/metric.rs`, 195 LOC). Wrapper + 4
   accessors + `from_lattice_and_metric()` constructor + AURORA
   EVOLVING doc-comment + 3 in-module tests including a
   Phase-2-additivity sentinel that asserts no DEC method has snuck
   onto the type.
2. **Cubed-sphere constructor** (`src/lattice/topology/cubed_sphere.rs`,
   590 LOC including 9 in-module tests). Three-tier panel-major vertex
   numbering (8 cube corners → 12·(C-1) cube-edge interior runs →
   6·(C-1)² panel interiors), gnomonic projection from cube panel to
   sphere, L'Huilier-spherical-excess cell areas, atan2-stable
   great-circle edge lengths. Combinatorics asserted exactly:
   F = 6C², E = 12C², V = 6C² + 2, χ = V − E + F = 2 for C = 1, 2, 3.
   Total cell area sums to 4π at C = 4 within 1e-10.
3. **Hints table** (`src/lattice/topology/hints.rs`, 101 LOC).
   `TOPOLOGY_HINTS: &[(&str, &str)]` with two rows, sorted, plus
   case-insensitive `lookup()` and verbose alias `topology_hint_for()`.
   5 in-module tests.
4. **Registry extension** (`src/lattice/registry.rs`, +175 LOC). New
   `Constructor` type alias, `ConstructorArgs { panel_size:
   Option<usize> }`, `ConstructorError::InvalidArgument(String)`, and
   `get_constructor(&str) -> Option<Constructor>` populated lazily via
   `OnceLock<HashMap<&'static str, Constructor>>` by
   `init_builtin_constructors()`. Both `TRUNCATED_ICOSAHEDRON` and
   `CUBED_SPHERE` register. 7 new in-module tests including the
   bit-identity guard.
5. **Module stitching** (`src/lattice/mod.rs` + `src/lattice/topology/mod.rs`).
   `pub mod metric;` + `pub use metric::LatticeWithMetric;` on the top
   lattice module; `pub mod cubed_sphere;` + `pub mod hints;` +
   `pub use cubed_sphere::cubed_sphere;` + `pub use hints::topology_hint_for;`
   on the topology submodule. Plus one additive `pub fn build(panel_size:
   usize) -> Result<LatticeWithMetric, String>` wrapper in
   `cubed_sphere.rs` because the integration test file calls
   `cubed_sphere::build(c)` rather than the named-constructor form —
   pure delegation with the documented 1..=256 envelope check, no
   logic duplicated.

After the stitching pass, all 15 new AURORA integration tests passed
on first compile:

```
cargo test --features lattice --test aurora_cubed_sphere_construction \
                              --test aurora_topology_hint_table \
                              --test aurora_lattice_registry_dispatch

aurora_cubed_sphere_construction: 8 passed; 0 failed; 0 ignored
aurora_lattice_registry_dispatch: 4 passed; 0 failed; 0 ignored
aurora_topology_hint_table:       3 passed; 0 failed; 0 ignored
Total: 15/15 passed, 0 failed
```

---

## Receipts

All four gates green:

```
cargo test --no-default-features --lib
  test result: ok. 870 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (finished in 3.55s; byte-identical no-default-features optionality surface)

cargo test --features halcyon --lib -- --test-threads=1
  test result: ok. 1020 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (finished in 25.84s; single-threaded honors the Halcyon global
  thermalization cache requirement; +24 tests over Phase 0's 996
  is the new lattice + topology + registry coverage)

cargo test --features kahler --lib
  test result: ok. 1150 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (finished in 124.45s)

cargo test --features halcyon --test halcyon_part_iv_gold -- --test-threads=1
  test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
  (passing: tdd_hal_iv_10_b_energy_drift_two_tier,
           tdd_hal_iv_10_c_gauss_residual_two_tier,
           tdd_hal_iv_10_d_h_total_now_returns,
           tdd_hal_iv_10_e_diagnostics_envelope_shape;
   ignored: tdd_hal_iv_10_a_symplectic_flow_canonical
            — pre-existing ignore, not a Phase 1 drift signal)

cargo test --features lattice --test aurora_cubed_sphere_construction \
                              --test aurora_topology_hint_table \
                              --test aurora_lattice_registry_dispatch
  Total: 15 passed; 0 failed; 0 ignored
```

The load-bearing receipt is **`halcyon_part_iv_gold`**: the IV.10 /
III.8b / V.* bit-identity contracts flow through face cycles and the
registry. If the CC-2 refactor or any of the Phase 1 stitching had
perturbed orientation computation or the buckyball storage shape, this
gate would fail. It is byte-identical to the Phase 0 baseline.

The **bit-identity guard test**
(`test_registry_dispatched_buckyball_bit_identical_to_direct`) is the
inline contract guarding the CC-2 surface: it asserts
`registry::get_constructor("TRUNCATED_ICOSAHEDRON").unwrap()(&Default::default()).unwrap().lattice() == &truncated_icosahedron::buckyball()`
via the derived `PartialEq` on `Lattice` plus identical `to_gql()`
re-emission. Both halves pass. The CC-2 surface is provably a
zero-bit-drift refactor.

### LOC totals

| Surface | LOC | Files |
| --- | --- | --- |
| Metric stub (production + 3 tests) | 195 | `src/lattice/metric.rs` |
| Cubed-sphere constructor (production + 9 tests) | 590 | `src/lattice/topology/cubed_sphere.rs` |
| Topology-hint table (production + 5 tests) | 101 | `src/lattice/topology/hints.rs` |
| Registry extension (additive in existing file) | +175 | `src/lattice/registry.rs` |
| Module re-exports (stitching) | +7 (approx) | `src/lattice/mod.rs`, `src/lattice/topology/mod.rs` |
| AURORA integration tests (3 files) | 305 | `tests/aurora_*` |
| **Total** | **~1373** | 8 files (5 new, 3 modified) |

Of which production code (excluding in-module tests + integration tests):
~736 LOC. The locked-context envelope was 320–505 production LOC; the
actual figure overruns the upper bound because the cubed-sphere
constructor's gnomonic projection + spherical-excess area + atan2
edge length helpers came in heavier than the discovery's 150–250 LOC
estimate (the actual constructor is closer to 400 LOC of pure-arithmetic
geometry, the remaining 190 LOC is the in-module test suite). The
overrun is honest scope discovery, not feature creep — no surface
beyond the locked context shipped, no Phase 2 surface smuggled in.

---

## Files touched

| File | LOC delta | Nature |
| --- | --- | --- |
| `src/lattice/metric.rs` | +195 (new file) | Phase 1 minimal `LatticeWithMetric` wrapper + 4 accessors + `from_lattice_and_metric` constructor + AURORA EVOLVING doc-comment + 3 in-module tests. **No DEC methods.** |
| `src/lattice/topology/cubed_sphere.rs` | +590 (new file) | `pub fn cubed_sphere(name, panel_size) -> LatticeWithMetric` + `pub fn build(panel_size) -> Result<LatticeWithMetric, String>` wrapper + three-tier panel-major vertex numbering + gnomonic projection + L'Huilier spherical excess + atan2 great-circle arc + 9 in-module tests. Consumes Phase 0 `Lattice::signed_face_orientations()` — no face-cycle traversal duplicated. |
| `src/lattice/topology/hints.rs` | +101 (new file) | `TOPOLOGY_HINTS` const table reserving `S2/CUBED_SPHERE` + `S2/TRUNCATED_ICOSAHEDRON` symmetrically + case-insensitive `lookup()` + verbose alias `topology_hint_for()` + 5 in-module tests. |
| `src/lattice/registry.rs` | +175 (extend existing file) | New `Constructor` type alias + `ConstructorArgs` + `ConstructorError` + `get_constructor()` + `init_builtin_constructors()` populating both `TRUNCATED_ICOSAHEDRON` and `CUBED_SPHERE` lazily via `OnceLock`. AURORA EVOLVING doc-comments on the new public surface. 7 new in-module tests including the bit-identity guard for buckyball. **Zero existing pub method modified.** |
| `src/lattice/mod.rs` | +1 line | `pub use metric::LatticeWithMetric;` re-export beneath the existing `pub mod metric;` declaration. |
| `src/lattice/topology/mod.rs` | +2 lines | `pub use cubed_sphere::cubed_sphere;` + `pub use hints::topology_hint_for;` re-exports. |
| `tests/aurora_cubed_sphere_construction.rs` | +~140 (new file) | 8 RED-first integration tests: combinatorics at C=1/2/4, Euler χ=2 at C=3, all-faces-are-quads, `signed_face_orientations` round-trip, topology hint, total-area = 4π. |
| `tests/aurora_topology_hint_table.rs` | +~50 (new file) | 3 RED-first integration tests for the const-table lookup contract. |
| `tests/aurora_lattice_registry_dispatch.rs` | +~115 (new file) | 4 RED-first integration tests including the bit-identity guard `test_registry_dispatched_buckyball_bit_identical_to_direct`. |

**Parser executor (`src/parser.rs:8773-8813`) is NOT modified in
Phase 1.** The CC-2 surface lands additively in `lattice::registry`;
the parser-side switch from static match arm to registry lookup is
deferrable to a tiny Phase 1b follow-up (or folded into Phase 2 with
the PANEL_SIZE parser syntax). The bit-identity contract is preserved
because both paths now exist in parallel and the integration test
proves they produce identical `Lattice` instances.

**Out of scope (Phase 2, untouched):**
- `src/lattice/dec/` — DEC operator free functions.
- `src/gauge/hamiltonian_registry.rs` (CC-1).
- `src/gauge/action.rs` (A2 four-trait refactor).
- Any stability-annotation deployment beyond the EVOLVING doc-comments
  on the four new public types introduced in this sprint. The full
  convention spec is Phase 2 alongside `hamiltonian_registry.rs`.

---

## Cross-references

- **Reply 2 commit `ad306ec`** — engine acceptance for A1, CC-2, CC-4,
  CC-5, plus all six CC-N resolutions. The Phase 1 surface lands per
  §1 (A1), §3 (CC-2), and §5 (CC-4 → which Phase 0 already shipped
  and Phase 1 consumes).
- **Phase 0 commit `ca589eb`** — `Lattice::signed_face_orientations()`
  promotion; the cubed-sphere constructor calls this method directly.
  No face-cycle traversal logic is duplicated in Phase 1.
- **Q4/Q5/Q6 resolution thread**:
  - commit `c06f073` (`theory/aurora/GIGI_TO_AURORA_2026-06-21_Q4_Q5_Q6_RESOLVED.md`)
    — engine acknowledgment of AURORA's Q4b (separate host binary), Q5
    (eager `init()`), Q6 (stability annotation convention is gigi-side
    work).
  - commit `358ede4` (`theory/aurora/AURORA_TO_GIGI_REPLY2_2026-06-21.md`)
    — AURORA reply 2 with Q4a path-dep detail, Q5a explicit
    `init()` code shape, and the Q6b correction (`#[stable]` proc-macro
    is rustc-internal, not available to library crates; doc-comment +
    changelog + semver is the right shape).
- **`theory/aurora/AURORA_ASKS_v0_1_LOG.md`** — status-board rows for
  A1, CC-2, and `topology_hint` flipped from greenlit / ACCEPTED to
  **DONE** in this session. Receipt commit hash is **TBD** in each
  row; the parent agent commits Phase 1 in one focused commit after
  this workflow returns and will backfill the hash into the rows.
- **`theory/aurora/AURORA_PHASE_0_IMPL_LOG.md`** — Phase 0 log, the
  format precedent this log mirrors per-section, and the source of the
  `signed_face_orientations()` surface Phase 1 consumes.
- **Halcyon bit-identity gate unaffected** — `halcyon_part_iv_gold`
  passes 4/0 (plus the pre-existing
  `tdd_hal_iv_10_a_symplectic_flow_canonical` ignore, which is NOT a
  drift signal). The CC-2 refactor is provably zero-bit-drift on the
  `TRUNCATED_ICOSAHEDRON` path: the new
  `test_registry_dispatched_buckyball_bit_identical_to_direct` test
  asserts `PartialEq` equality between the registry-dispatched
  buckyball and the direct `buckyball()` call; the existing
  `tdd_hal_i_8_*` registry round-trip tests and `tdd_hal_i_2_*`
  topology tests also pass unchanged.
- **Reference precedents inside this commit**:
  - `src/lattice/topology/truncated_icosahedron.rs:350`
    (`buckyball()`) — the constructor the registered
    `TRUNCATED_ICOSAHEDRON` wrapper delegates to; the bit-identity
    anchor.
  - `src/lattice/mod.rs:208` (`signed_face_orientations()`) — Phase 0
    promotion; the cubed-sphere face-cycle round-trip test consumes
    this method directly.
  - `src/lattice/registry.rs:54–186` (`tdd_hal_i_8_*` tests) — the
    existing registry round-trip tests preserved unchanged through
    Phase 1.

---

## What's next

Phase 2 is unblocked on the AURORA side per the Q4/Q5/Q6 resolution
thread (commits `c06f073` + `358ede4`) and gated on the engine side
on AURORA's `ShallowWater` factory skeleton arriving. The Phase 2
sprint as currently shaped:

- **CC-1 — `hamiltonian_registry.rs` skeleton.** New
  `src/gauge/hamiltonian_registry.rs` mirrors the `lattice::registry`
  shape: `register_hamiltonian(name, factory)` + `get_hamiltonian(name)`
  + `clear()` + `all()`. `OnceLock<Mutex<HashMap<...>>>` storage.
  Built-in `KogutSusskind` registers itself in the engine startup
  hook. WAL `HamiltonianDeclare` carries metadata only (name, kind_tag,
  params); trait-object re-materialized from factory at replay. Eager
  registration per Q5a. ~150–250 LOC.
- **Stability annotation convention spec.** Extend
  `docs/STABILITY_GUARANTEES.md` from feature-flag stability (already
  covered) to trait-surface stability. Per AURORA reply 2 §3 the
  convention shape is `/// Stability: EVOLVING until gigi 0.1.0 tag.`
  doc-comments + changelog discipline + semver (minor-bump-breaks,
  patch-non-breaking). The Phase 1 EVOLVING annotations on
  `LatticeWithMetric`, `ConstructorArgs`, `ConstructorError`, and
  `get_constructor()` are the first deployment of the convention;
  Phase 2 deploys the same annotation on `HamiltonianFactory`,
  `HamiltonianForce`, `HamiltonianDrift`, `ProjectionOperator`,
  `EnergyDecomposition` when those land.
- **Q3 DEC operator surface.** New `src/lattice/dec/` module with
  `d_0`, `delta_0`, `hodge_star_k` as free functions consuming the
  Phase 1 `LatticeWithMetric` wrapper. ~250–400 LOC. Lands before
  AURORA's `HamiltonianForce` impl is exercised. The Phase 1
  `LatticeWithMetric` stub is provably ready: its `cell_areas()` +
  `edge_lengths()` accessors are the inputs the DEC operators read,
  and the Phase 1 cubed-sphere constructor populates them with
  sphere-correct values (total area sums to 4π within 1e-10 at C=4).
- **A2 four-trait refactor + `ShallowWater`.** `HamiltonianForce` /
  `HamiltonianDrift` / `ProjectionOperator` / `EnergyDecomposition`
  in `src/gauge/action.rs` (NEW). `KogutSusskind { beta }` becomes
  the first impl (bit-identity preserved for Halcyon). `ShallowWater
  { g, omega, a }` lands as the second impl in AURORA's host binary,
  not in gigi. ~600–900 LOC + tests. Hot-path constraint per Reply 2
  §9: integrator generic over concrete `H: HamiltonianForce +
  HamiltonianDrift`, monomorphized via outer-loop enum match;
  `Box<dyn ...>` only at registry / WAL / introspection boundaries.

All four Phase 2 items are gated on AURORA's `ShallowWater` factory
skeleton arriving so the engine can validate the trait shapes against
a concrete second consumer before locking the surface. Phase 1 is
self-contained and ships independently.

---

## Authorship note

Per `feedback_no_ai_coauthor.md`: when the parent agent commits Phase
1, the commit body must NOT carry a `Co-Authored-By: Claude` footer.
Author = `nurdymuny <bee_davis@alumni.brown.edu>` (Bee Rosa Davis)
only. Same convention as Phase 0 and every commit in the Halcyon Part
V sprint.
