# AURORA v0.1 engine-asks — tracking log

**Received** 2026-06-19 from AURORA (first cross-team contact, engine-asks
v0.1 spec + `williamson_test2_scaffold.py` scaffold).

**Reply letters**:
- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md` (initial engine response).
- `theory/aurora/AURORA_TO_GIGI_REPLY_2026-06-19.md` (AURORA decisions on CC-1..6 + Q3).
- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` (engine acceptance + Q3 answer + Q4/Q5/Q6).
- `theory/aurora/GIGI_TO_AURORA_2026-06-21_Q4_Q5_Q6_RESOLVED.md` (engine acknowledgment of AURORA's Q4b/Q5/Q6 answers; commit `c06f073`).
- `theory/aurora/AURORA_TO_GIGI_REPLY2_2026-06-21.md` (AURORA reply 2: Q4a path-dep detail, Q5a explicit init() code shape, **Q6b correction — doc-comment + changelog convention, not `#[stable]` proc-macro which is rustc-internal**).

**First AURORA receipt**: Williamson Test 2 step 0 against
`gigi-stream.fly.dev`, `refusal_reason = None`, mass/energy diagnostics
within rounding of analytical values, forward Euler refuses at step 2
(energy drift 4.7× machine_eps at dt=60s). Validates the post-`ea50585`
general-purpose surface as the first non-Halcyon physics carrier.

## The asks

AURORA is running shallow-water atmospheric dynamics on a cubed sphere via
`SYMPLECTIC_FLOW`, mirroring the Halcyon SU(2) Yang-Mills loop on the
buckyball. Three asks, ordered by ascending engine complexity.

- **A1 — `CUBED_SPHERE` topology**: add `CUBED_SPHERE` (6 panels × C² cells)
  alongside the existing `TRUNCATED_ICOSAHEDRON`. Both are S² topologies;
  different tilings.
- **A2 — `SYMPLECTIC_FLOW` `HamiltonianKind`**: add `ShallowWater { g, omega, a }`
  alongside the existing `KogutSusskind { beta }`.
- **A3 — bundle schema `AURORA_RECEIPT`**: 10 fields including
  `kelvin_holonomies: [FLOAT64; 3]` (fixed-length array) and
  `c_field_summary: { min, max, mean: FLOAT64 }` (inline anonymous struct),
  plus a `RECEIPT_GATE refusal_reason IS NULL` clause.

Questions back-and-forth (resolved unless flagged OPEN):

- **Q1** (AURORA→GIGI): does the schema DSL support fixed-length arrays +
  inline anonymous structs in `CREATE BUNDLE SCHEMA`? **Answer: no, not
  today.** Reply 1 §5. Resolved.
- **Q2** (AURORA→GIGI): does the lattice-registry have a seam-handling
  primitive for cubed-sphere panel adjacency, or does `CUBED_SPHERE` ship
  its own? **Answer: `CUBED_SPHERE` ships its own.** Reply 1 §7. Resolved.
- **Q3** (AURORA→GIGI): does the general-purpose `Lattice` surface expose
  `d_0`, `delta_0`, Hodge star? **Answer: not today; not adding them to
  `Lattice`.** New `src/lattice/dec/` module consuming `LatticeWithMetric`
  wrapper. Reply 2 §10. Resolved (engine commits to ~250-400 LOC lift
  before A2 starts).
- **Q4** (GIGI→AURORA): aurora_crate repo location + gigi-feature-flag vs
  separate host binary for deploy. **Answer (2026-06-21): Q4b — separate
  host binary; gigi's `Cargo.toml` stays clean.** `aurora/` is its own
  workspace, links gigi as a pinned dep. Makes AURORA the canonical
  example of the downstream-crate pattern (Halcyon is the special-three
  feature-flag pattern; everyone else follows AURORA). Resolved.
- **Q5** (GIGI→AURORA): `aurora_crate::init()` lifecycle in host-binary
  `main()`; eager-vs-lazy registration; WAL replay ordering. **Answer
  (2026-06-21): eager `init()` at top of `main()`, no lazy anything.**
  `hamiltonian_registry` exposes a public `register()` API; AURORA's
  `main()` calls it once at startup with their `ShallowWater` factory.
  No thread-local magic, no auto-registration. Resolved.
- **Q6** (GIGI→AURORA): binary-stability contract on the
  `HamiltonianHandle` trait surface; pinned-vs-floating gigi dep;
  stability annotation. **Answer (2026-06-21): AURORA confirms gigi
  grows a stability annotation convention because of them — that
  framing is accurate and appropriate.** The convention itself is our
  design work to specify; tracked as a Phase 2 deliverable alongside
  `hamiltonian_registry.rs`. AURORA is the first external pinned
  consumer; the convention extends `docs/STABILITY_GUARANTEES.md` from
  feature-flag stability (already covered) to trait-surface stability
  (new). Resolved.

## Status board

| Ask | Shape | Phase | Status | Receipt |
| --- | --- | --- | --- | --- |
| P-1 (dispatch) | bugfix on `/v1/gql` | shipped | **DONE** | commit `5b555ce` |
| `ea50585` split | lift lattice/gauge out of halcyon namespace | shipped | **DONE** | commit `ea50585` |
| First AURORA receipt | Williamson Test 2 step 0 against `gigi-stream.fly.dev` | shipped | **DONE** | `refusal_reason = None`, forward Euler refuses at step 2 |
| Signed-face promotion (CC-4) | `Lattice::signed_face_orientations()`; 23 LOC additive lift in `src/lattice/mod.rs` (well under the ~80–100 LOC budget); 3 RED-first integration tests in `tests/aurora_signed_face_orientations.rs` (211 LOC) | Phase 0 (lands first) | **DONE** | commit `ca589eb` — RED→GREEN, 3/3 passing; no-default-features 870/0 byte-identical; halcyon 996/0; kahler 1150/0; `halcyon_part_iv_gold` 4/0+1-pre-existing-ignored, bit-identity IV.10/III.8b/V.* contracts intact |
| A1 — `CUBED_SPHERE` | parameterized PANEL_SIZE; calls promoted signed-face surface; constructor returns `LatticeWithMetric`; new `src/lattice/topology/cubed_sphere.rs` (590 LOC incl. 9 in-module tests) + `tests/aurora_cubed_sphere_construction.rs` (8 RED-first tests) + minimal `src/lattice/metric.rs` wrapper (195 LOC, NO DEC methods — Phase 2 owns those) | Phase 1 | **DONE** | commit `f62e46c` — combinatorics V=6C²+2, E=12C², F=6C², χ=2 exact at C=1,2,3; total cell area = 4π within 1e-10 at C=4; 15/15 AURORA integration tests pass; no-default-features 870/0; halcyon 1020/0; kahler 1150/0; `halcyon_part_iv_gold` 4/0+1-pre-existing-ignored, bit-identity IV.10/III.8b/V.* contracts intact |
| CC-2 registry-dispatch refactor | additive `lattice::registry::get_constructor(&str) -> Option<Constructor>` surface with `Constructor = fn(&ConstructorArgs) -> Result<LatticeWithMetric, ConstructorError>`; lazy `init_builtin_constructors()` via `OnceLock` registers both `TRUNCATED_ICOSAHEDRON` (wrapping existing `buckyball()`) and `CUBED_SPHERE`; +175 LOC in `src/lattice/registry.rs` + `tests/aurora_lattice_registry_dispatch.rs` (4 RED-first tests including bit-identity guard). Parser-executor switch at `src/parser.rs:8773-8813` deferrable to tiny Phase 1b follow-up; bit-identity preserved with both paths in parallel | Phase 1 (lifted from "optional" into A1 sprint) | **DONE** | commit `f62e46c` — `test_registry_dispatched_buckyball_bit_identical_to_direct` passes via `PartialEq` + `to_gql()` re-emission; all 7 new in-module tests + 10/10 existing `tdd_hal_i_8_*` registry tests green |
| `topology_hint` const table | `src/lattice/topology/hints.rs` (NEW, 101 LOC incl. 5 in-module tests) reserves `S2/CUBED_SPHERE`, `S2/TRUNCATED_ICOSAHEDRON` symmetrically via `TOPOLOGY_HINTS: &[(&str, &str)]`; case-insensitive `lookup()` + verbose alias `topology_hint_for()`; unknown identifier returns `None` (never silent default) + `tests/aurora_topology_hint_table.rs` (3 RED-first tests) | Phase 1 | **DONE** | commit `f62e46c` — 3/3 AURORA integration tests + 5/5 in-module tests pass; symmetrically registered for both Phase 1 topologies; extension protocol documented in module doc-comment |
| A3-workaround | six flat scalar names: `kelvin_holonomies_eq/n30/s30`, `c_field_min/max/mean`; gate via `COVER WHERE refusal_reason IS NULL` | Phase 1 | **ACCEPTED** (Reply 2 §9; zero engine LOC) | — |
| CC-1 `hamiltonian_registry` | new `src/gauge/hamiltonian_registry.rs` (203 LOC): `OnceLock<Mutex<HashMap<String, Box<dyn HamiltonianFactory>>>>` storage; public `register(name, factory, wal_writer, registered_at)` + `with_factory(name, closure)` + `contains` + `list_registered` + `clear`; `RegistryError { DuplicateName, WalEmitFailed }`; WAL `HamiltonianDeclare` (op `0x0C`) metadata-only with `[name, kind_tag, group_tag, registered_at]` payload; eager `register()` per Q5 (empty registry until host binary explicitly registers; `test_registry_eager_init_no_auto_populate` verifies); downstream-crate pattern per Q4b; first-write-wins on duplicate names. WAL replay handling deferred (engine acknowledges variant as no-op; Q5 eager-init contract makes this safe) | Phase 2 (**UNBLOCKED 2026-06-21**, gated on AURORA's `ShallowWater` factory sketch arriving) | **DONE** | commit `TBD` — 7/7 AURORA integration tests in `tests/aurora_phase_2_hamiltonian_registry.rs` (register/with_factory round-trip, duplicate-name rejection, unknown-name None, kind_tag round-trip, WAL emission round-trip via `WalReader::read_all`, eager-init no-auto-populate, list_registered triples); no-default-features 870/0 byte-identical, halcyon 1031/0, kahler 1150/0, `halcyon_part_iv_gold` 4/0+1-pre-existing-ignored, `halcyon_part_vi_bit_identity_gold` 3/0+0-ignored under `--include-ignored` (VI byte-identity intact by construction — `symplectic_flow.rs` untouched); EVOLVING stability markers per commit `1e13252` on every public item; integrator-generic-over-`H` refactor + KogutSusskind lift + WAL replay materialization all deferred to a later workflow with its own bit-identity discipline |
| A2 — four-trait refactor + `ShallowWater` | new `src/gauge/action.rs` (300 LOC): `HamiltonianForce` / `HamiltonianDrift` / `ProjectionOperator` / `EnergyDecomposition` four sub-traits unified by `HamiltonianHandle: Send + Sync + Debug` super-trait; `HamiltonianFactory` with `kind_tag() -> &'static str` + `group_tag() -> &'static str` + `from_params(&HashMap<String, f64>) -> Result<Box<dyn HamiltonianHandle>, FactoryError>`; group-agnostic by construction (trait methods speak only in `&[f64]` / `&mut [f64]` / `Vec<f64>` / `BTreeMap<String, f64>` so AURORA's `group_tag = "R"` compiles alongside future SU(2)/SU(3) impls); object-safe (no associated types leak, State erased to flat `&[f64]` buffer at trait boundary); `project_constraint<L: LatticeWithMetric>(&self, &L, &mut [f64]) -> Result<(), ProjectionError>` is the `PROJECT_GAUSS` sugar; `evaluate(&[f64]) -> BTreeMap<String, f64>` with `energy_keys() -> &'static [&'static str]` static contract; `FactoryError { MissingParam, InvalidParam, UnsupportedGroup }` + `EnergyError` + `ProjectionError` enums. **Trait surface only — integrator refactor of `symplectic_flow.rs` to be generic over concrete `H: HamiltonianForce + HamiltonianDrift` is DEFERRED to a later workflow** with its own RED→GREEN cycle against IV.10 + VI bit-identity gates (those gates remain byte-identical in this sprint because `symplectic_flow.rs` was not touched). KogutSusskind lift to `HamiltonianFactory` impl also deferred (natural pairing with the integrator refactor) | Phase 2 (**UNBLOCKED 2026-06-21**) | **DONE** (trait surface only; integrator refactor + KogutSusskind lift DEFERRED) | commit `TBD` — 6/6 AURORA integration tests in `tests/aurora_phase_2_trait_surface.rs` (factory trait shape, HamiltonianHandle sub-trait bounds, deterministic 7-key EnergyDecomposition matching AURORA's `casimir_energy/mass/pv_l1/pv_l2 + kelvin_eq/n30/s30` contract, NoOpHamiltonian compiles, MockShallowWaterFactory signature alignment with AURORA's downstream stub, typed `FactoryError::MissingParam` on missing `g`); no-default-features 870/0 byte-identical, halcyon 1031/0, kahler 1150/0, `halcyon_part_iv_gold` 4/0+1-pre-existing-ignored, `halcyon_part_vi_bit_identity_gold` 3/0+0-ignored under `--include-ignored`; EVOLVING stability markers per commit `1e13252` on all six pub traits + three error enums; hot-path constraint satisfied by construction (registry stores `Box<dyn HamiltonianFactory>`, factory `from_params` returns `Box<dyn HamiltonianHandle>` — boxed handle is the extent of trait-object cost; integrator generic-over-`H` enforces the cold inner loop in the deferred refactor); one follow-up letter queued to AURORA covering 7 minor signature deltas (Debug bound, State→`&[f64]` erasure, HashMap→BTreeMap, String keys, drop `&Grid` param, `wal_writer + registered_at` on register, `with_factory` closure pattern) — net ~20–25 LOC AURORA-side rework |
| Q3 — DEC operator surface | `src/lattice/metric.rs` (Phase 1, untouched) + `src/lattice/dec/` (NEW, Phase 2: `mod.rs` 82 LOC, `d.rs` 96 LOC, `codifferential.rs` 170 LOC, `hodge.rs` 184 LOC); `d_0` (Form0→Form1, pure combinatorics), `delta_1` (Form1→Form0; the math correct name for the status-board's `delta_0`, the codifferential adjoint of `d_0` on a 2-manifold), `hodge_star_0/1/2` (vertex-area / barycentric-edge-ratio / cell-area-inverse weighting) as free functions consuming `&LatticeWithMetric`; barycentric dual-edge formula `l_e^* = (A_{v-}^* + A_{v+}^*) / (2*l_e)` computed inline so no new accessor on the wrapper (Phase 3 upgrade path documented in `codifferential.rs`); `DecError { LengthMismatch, CellAreasMissing, EdgeLengthsMissing, DualFaceAreasMissing }` with `&'static str` surface labels for diagnostics; +1 line `pub mod dec;` in `src/lattice/mod.rs`; 487 LOC `tests/aurora_dec_operators.rs` integration suite | Phase 2 (lands before AURORA's `HamiltonianForce` impl, **UNBLOCKED 2026-06-21**) | **DONE** | commit `TBD` — 20/20 AURORA integration tests (19 under `--features halcyon`, 20 under `--features halcyon,kahler` including cross-module sign-pin vs `discrete::hodge_complex`); identities verified: `d_0(const) = 0` bit-identical, `delta_1 ∘ d_0(const) = 0` exact (algebraic identity, not convergence), `hodge_star_2(1)[c] = 1/cell_areas[c]` elementwise, `hodge_star_0(1)[v] = A_v^*` elementwise + sum=4π within 1e-10, `hodge_star_1(1)` full-symmetry on C=1, structured errors on all length/missing-metric failure paths, `d_0` sign convention matches `HodgeComplex::d0`; no-default-features 870/0 byte-identical, halcyon 1030/0 (+10 new in-module unit tests), kahler 1150/0, `halcyon_part_iv_gold` 4/0+1-pre-existing-ignored bit-identity gate intact; EVOLVING stability markers per commit `1e13252` on all five public fns + `DecError`; `metric.rs` and `discrete/hodge_complex.rs` untouched (additivity contract held) |
| Stability annotation convention (Q6) | extend `docs/STABILITY_GUARANTEES.md` from feature-flag stability to trait-surface stability; **convention shape settled per AURORA reply 2 §3 (2026-06-21):** doc-comment + changelog discipline + semver (`/// Stability: EVOLVING until gigi 0.1.0 tag.` + minor-bump-breaks + patch-non-breaking); my earlier `#[stable(since = "0.1.x")]` proc-macro draft was wrong — that attribute is rustc-internal, not available to library crates. Attach the EVOLVING doc-comment to `HamiltonianFactory` + `HamiltonianForce`/`HamiltonianDrift`/`ProjectionOperator`/`EnergyDecomposition` when they land. | Phase 2 (gigi-side design work, parallel with `hamiltonian_registry.rs`) | **OWNED** (gigi-side per AURORA Q6 confirmation 2026-06-21); shape locked per AURORA reply 2 | — |
| A3-extension — `[TYPE; N]` arrays | DSL lift | Phase 3 (general purpose) | accepted, not started | — |
| A3-extension — inline structs + `__` desugar | DSL lift; `kelvin_holonomies__eq` long-form, flat names as legacy aliases | Phase 3 (general purpose) | **ACCEPTED** convention (Reply 2 §6) | — |
| A3 SMOKE GATE | full Williamson Test 2 on C48, `casimir_pv_l2 < 1e-6` over 10 days, three Kelvin holonomies match analytical | Phase 3 (joint) | gated on Phase 2 | — |
| `Lattice::seam_metadata` (CC-3) | general lift; deferred from v0.1 | Phase 4 (optional, no commitment) | accepted constructor-owned for v0.1 (Reply 2 §4) | — |
| Schema-level `RECEIPT_GATE` CHECK (CC-6) | schema-attached predicate fired on INSERT | Phase 4 (optional) | accepted query-side for v0.1 (Reply 2 §6) | — |

## What's mechanical

### A1 — `CUBED_SPHERE` topology constructor

- ~150–250 LOC. New `src/lattice/topology/cubed_sphere.rs` module with
  `pub fn cubed_sphere(...) -> Lattice` (parameter shape TBD per
  design question below). One parser match arm in `src/parser.rs`
  (~line 8778, peer to `"TRUNCATED_ICOSAHEDRON" =>`). One `pub mod`
  line in `src/lattice/topology/mod.rs`. New integration test
  `tests/lattice_cubed_sphere.rs`.
- No buckyball-specific hardcoding found in the general `Lattice` struct
  or registry — pentagon/hexagon logic is confined to
  `truncated_icosahedron.rs`. Constructor lands cleanly as a peer.

### A3 workaround (zero engine LOC)

- AURORA-side schema rewrite: 6 NUMERIC fields
  (`kelvin_holonomies_x/y/z`, `c_field_min/max/mean`) in place of the
  array + struct. `refusal_reason IS NULL` lands as
  `FilterCondition::Void("refusal_reason")` on `COVER WHERE`, which is
  already supported.

## What needs design conversation first

### A2 — refactor `SYMPLECTIC_FLOW` integrator

The four KDK kernels are SU(2)-Wilson-specific today:

- `wilson_force_per_edge` (staple sum, `-β/8` coefficient for N=2)
- `drift_step` (Rodrigues exponential on imaginary quaternion)
- `project_gauss` (covariant divergence + adjoint action)
- kinetic-energy decomposition (assumes `g² = 4/β`)

Refactor: extract `HamiltonianForce` / `HamiltonianDrift` /
`ProjectionOperator` / `EnergyDecomposition` traits into
`src/gauge/action.rs`. `KogutSusskind { beta }` becomes the first impl
(no behavior change for Halcyon — bit-identity preserved). `ShallowWater { g, omega, a }`
becomes the second impl.

Total: ~600–900 LOC + tests.

Blocked on **CC-1** below — the closed-enum vs open-registry decision
must land before A2 starts, because reversing it after A2 ships costs a
second refactor.

## Cross-cutting general-purpose questions (CC-N) — RESOLVED 2026-06-19

All six CC items resolved in Reply 2. Summary:

- **CC-1 — `HamiltonianKind` open trait-object registry.** **RESOLVED:
  open registry.** New `src/gauge/hamiltonian_registry.rs` mirrors
  `lattice::registry` shape; trait-object factory keyed by `kind_tag`;
  WAL `HamiltonianDeclare` stores metadata only (name, kind_tag, params);
  trait-object is re-materialized from factory at replay. Engine enforces
  registration completes before `Engine::open()` WAL replay. Reply 2 §2.
- **CC-2 — `LatticeTopology` open registry.** **RESOLVED: open registry,
  lifted into A1 sprint.** New `register_constructor` / `get_constructor`
  on `lattice::registry`. Parser arm at `parser.rs:8778-8787` replaced
  with registry lookup. ~110–175 LOC. Rationale: lower aggregate cost
  than shipping A1 on static match arm + retrofitting. Reply 2 §3.
- **CC-3 — `Lattice::seam_metadata`.** **RESOLVED for v0.1:
  constructor-owned seam logic.** `cubed_sphere` constructor enumerates
  12C cross-seam edges by panel-coordinate arithmetic; `Lattice` sees
  them as ordinary edges. Matches buckyball precedent. Phase 4 lift
  deferred without commitment. Reply 2 §4.
- **CC-4 — signed-face surface promotion.** **RESOLVED: promote BEFORE
  A1, as standalone PR.** `Lattice::signed_face_orientations() ->
  Vec<Vec<(EdgeId, Sign)>>`. ~80–100 LOC. Bit-identity risk = 0 per
  signed-faces audit. `buckyball_with_signed_faces()` becomes thin
  wrapper. AURORA's offered fallback (carry duplicate orientation logic)
  declined. Reply 2 §5.
- **CC-5 — schema DSL extension.** **RESOLVED: Phase 3 general lift; `__`
  desugar convention reserved.** v0.1 ships six flat scalar names as
  documented exception; `kelvin_holonomies__eq` long-form when DSL lands;
  flat names retained as legacy aliases. Engine commits to NOT shipping
  single-underscore flattening (collision risk). Reply 2 §6.
- **CC-6 — `RECEIPT_GATE` as schema-attached CHECK.** **RESOLVED for
  v0.1: query-side discipline.** `COVER WHERE refusal_reason IS NULL` is
  the contract; engine does not silently strip refusals. Schema-level
  CHECK Phase 4 deferred without commitment. Reply 2 §6.

## Engine commitments named explicitly (Reply 2)

- **Hot-path performance**: trait-object dispatch is OFF the integrator
  inner loop. Integrator generic over `H: HamiltonianForce +
  HamiltonianDrift`, monomorphized per `HamiltonianKind` variant via
  outer-loop enum match. `Box<dyn ...>` only at registry / WAL /
  introspection boundaries. AURORA's inner loop must follow.
- **Plugin mechanism**: Rust library + Cargo feature flag. NO dlopen, NO
  FFI, NO runtime plugin system. `aurora_crate` is a Rust crate
  depending on `gigi`; host binary calls `aurora_crate::init()` in
  `main()` before `Engine::open()`.
- **WAL replay ordering**: all `register_hamiltonian()` /
  `register_constructor()` calls MUST complete before `Engine::open()`
  begins replay. Engine fails fast on unknown `kind_tag` during replay.
- **Coordination protocol**: `aurora_crate` lives in a SEPARATE git repo;
  `theory/aurora/` in gigi remains the LETTERS path only. Implementation,
  tests, and CI for AURORA's physics live in aurora_crate's tree.
- **Metric data carrier**: NEW `LatticeWithMetric { lattice, metric }`
  wrapper struct + `src/lattice/dec/` module. `Lattice` stays
  metric-agnostic. AURORA's DEC operators consume the wrapper, not raw
  `Lattice`. This is the Q3 commitment.
- **Single-tenant registry**: `lattice` + `gauge` + `hamiltonian`
  registries all use `OnceLock<Mutex<HashMap<...>>>`. Multi-tenant gigi
  is out of v0.1 scope.
- **Determinism**: ShallowWater factory MUST be deterministic — same
  params HashMap returns byte-identical initial state. Matches gauge-field
  replay contract (HaarRandom + seed reproduces buffer). If ShallowWater
  has stochastic initial conditions, the seed lives in params.

## Sprint-shape design questions (per-ask)

### A1 design questions

- Panel-resolution parameter: `CUBED_SPHERE PANEL_SIZE C` argument or
  fixed constant? Recommendation: parameterized.
- Face orientation convention: does `CUBED_SPHERE` export signed faces?
  Folds into CC-4.
- Seam-handling ownership: constructor owns it today; folds into CC-3.
- `topology_hint` string vocabulary: reserved or free-form?

### A2 design questions

- Trait shape: four separate traits or one `HamiltonianKernel` trait
  with four methods?
- Drift signature: generic `State` associated type or shared
  per-edge-real-vector convention?
- Projection contract: rename `project_gauss` to `project_constraint`?
- Energy decomposition: free-form labeled map or fixed `(kinetic, potential)`
  tuple?
- Naming: `KogutSusskind { beta }` (group-agnostic) or `KogutSusskind { group, beta }`?
  Decides whether SU(3) lands as new `HamiltonianKind` or new `Group` variant.

### A3 design questions

- `RECEIPT_GATE` keyword vs `COVER WHERE` discipline (folds into CC-6).
- Array vs Vector field type: reuse existing `Vector { dims: 3 }` or
  introduce structurally-distinct `Array<T>`?
- Nested-field query syntax: dotted-path `FilterCondition` or
  parse-time desugar to synthetic columns?
- v1 shipping shape: flatten-to-scalars or wait for DSL extension?

## What AURORA already has (zero blocking on engine side)

- `/v1/gql` gauge dispatch fix shipped at `5b555ce`. `williamson_test2_scaffold.py`
  can land HTTP requests against `gigi-stream.fly.dev` today.
- Post-`ea50585` general-purpose surface: `Lattice`, `lattice::registry`,
  `GroupElement`, `EdgeConnection`, `walk_loop`, `LATTICE` statement
  variants, `SHOW LATTICE` introspection. AURORA compiles against
  `lattice + gauge` features without dragging in Halcyon-specific code.
- `FilterCondition::Void("refusal_reason")` already supports the
  `IS NULL` predicate AURORA needs for receipt gating.
- `Vector` field type (`FieldType::Vector { dims }`) exists today as a
  fixed-dim dense f64 embedding — one viable landing for
  `kelvin_holonomies` if AURORA prefers a single 3-vector field over
  three scalars.
- SU(2) `KogutSusskind` path is production-tested through Halcyon
  Sprint 2 and will become the reference impl of the new
  `HamiltonianKind` traits during the A2 refactor — no behavior change
  for Halcyon.

## Status

- Logged: yes (this doc).
- Reply letters sent:
  - `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md` (initial).
  - `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` (post-AURORA-decisions).
- Bee greenlit: Phase 0 (CC-4) landed; Phase 1 unblocked.
- Sprint slot: Phase 0 closed (see `AURORA_PHASE_0_IMPL_LOG.md`, commit
  `ca589eb`, hash backfilled at `21557ce`); Phase 1 closed (see
  `AURORA_PHASE_1_IMPL_LOG.md`, commit `f62e46c`); Phase 1b closed
  (parser-executor switch from static match arm to registry lookup at
  commit `1091dd5`); Phase 2 DEC operator surface closed (see
  `AURORA_PHASE_2_DEC_IMPL_LOG.md`, commit `TBD`); Phase 2 trait
  surface + CC-1 hamiltonian registry closed (see
  `AURORA_PHASE_2_TRAIT_REGISTRY_IMPL_LOG.md`, commit `TBD`); receipt
  commit hashes for the Q3 + CC-1 + A2 rows to be backfilled once
  the focused Phase 2 commits land.
- AURORA-side blocking pressure: P-1 shipped; first receipt validated;
  Phase 0 (CC-4) **shipped** as a true additive lift (23 LOC,
  no-default-features 870/0 byte-identical, halcyon 996/0, kahler 1150/0,
  `halcyon_part_iv_gold` bit-identity gate clean); Phase 1 (A1 + CC-2 +
  topology_hint + minimal `LatticeWithMetric` wrapper) **shipped** as a
  fully additive sprint (~736 production LOC + 305 integration test LOC;
  no-default-features 870/0, halcyon 1020/0, kahler 1150/0, 15/15 new
  AURORA integration tests, `halcyon_part_iv_gold` bit-identity gate
  still clean; `test_registry_dispatched_buckyball_bit_identical_to_direct`
  proves the CC-2 surface is zero-bit-drift on the existing buckyball
  path); Phase 2 DEC operator surface (Q3) **shipped** as a fully
  additive sprint (532 LOC across `src/lattice/dec/{mod,d,codifferential,hodge}.rs`
  + 1-line `pub mod dec;` in `src/lattice/mod.rs` + 487 LOC `tests/aurora_dec_operators.rs`;
  no-default-features 870/0 byte-identical, halcyon 1030/0 (+10 in-module
  unit tests), kahler 1150/0, 20/20 AURORA integration tests under
  halcyon+kahler (19/0 under halcyon alone, cross-module sign-pin skipped),
  `halcyon_part_iv_gold` bit-identity gate intact; `metric.rs` and
  `discrete/hodge_complex.rs` untouched; barycentric dual-edge formula
  computed inline, Phase 3 circumcentric-dual upgrade path documented;
  EVOLVING stability markers on all five public fns + `DecError` per
  commit `1e13252`); Phase 2 trait surface + registry (A2 four-trait
  `src/gauge/action.rs` + CC-1 `src/gauge/hamiltonian_registry.rs` +
  WAL `HamiltonianDeclare` op `0x0C`) **shipped** as a fully additive
  sprint (503 LOC across two new gauge files + 75 LOC in `src/wal.rs`
  + 4 LOC in `src/gauge/mod.rs` + ~5 LOC engine replay match-arm +
  473 LOC across two new integration test files;
  no-default-features 870/0 byte-identical, halcyon 1031/0, kahler
  1150/0, `halcyon_part_iv_gold` bit-identity gate intact,
  `halcyon_part_vi_bit_identity_gold` 3/0 under `--include-ignored`
  intact, 13/13 AURORA integration tests pass; group-agnostic by
  construction — AURORA's `group_tag = "R"` ShallowWater compiles
  against the same trait surface as future SU(2) `KogutSusskind`;
  object-safe — State erased to `&[f64]` buffer at trait boundary;
  Q5 eager-init contract verified — registry empty until host binary
  explicitly registers; EVOLVING markers on all six pub traits + four
  error enums + five public registry fns per commit `1e13252`; one
  follow-up letter queued to AURORA on 7 minor signature deltas;
  `symplectic_flow.rs` + `wilson_force.rs` + `loop_transport.rs` +
  `project_gauss.rs` + `holonomy.rs` all untouched — integrator
  refactor to generic-over-`H`, KogutSusskind lift to
  `HamiltonianFactory` impl, and WAL replay materialization of
  `HamiltonianDeclare` all **deferred to a later workflow** with
  their own bit-identity discipline against IV.10 + VI gates). All
  named asks closed; remaining AURORA-side work is implementing
  `ShallowWaterFactory: HamiltonianFactory` against the published
  trait surface and running a full Williamson Test 2 against
  `gigi-stream.fly.dev`.

## References

- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md` — initial
  engine response.
- `theory/aurora/AURORA_TO_GIGI_REPLY_2026-06-19.md` — AURORA's decisions
  on CC-1..6 + design pins + Q3.
- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` — engine
  acceptance + Q3 answer + Q4/Q5/Q6 back to AURORA.
- `theory/aurora/AURORA_PHASE_0_IMPL_LOG.md` — Phase 0 CC-4 lift
  (`signed_face_orientations()` promotion at commit `ca589eb`).
- `theory/aurora/AURORA_PHASE_1_IMPL_LOG.md` — Phase 1 sprint
  (A1 + CC-2 + topology_hint + minimal `LatticeWithMetric` wrapper at
  commit `f62e46c`).
- `theory/aurora/AURORA_PHASE_2_DEC_IMPL_LOG.md` — Phase 2 Q3 DEC
  operator surface (`src/lattice/dec/` module: `d_0`, `delta_1`,
  `hodge_star_0/1/2`, `DecError`; commit `TBD`).
- `theory/aurora/AURORA_PHASE_2_TRAIT_REGISTRY_IMPL_LOG.md` — Phase 2
  trait surface + CC-1 hamiltonian registry (`src/gauge/action.rs`
  four sub-traits + `HamiltonianHandle` + `HamiltonianFactory`;
  `src/gauge/hamiltonian_registry.rs` register/with_factory/contains/
  list_registered/clear + `RegistryError`; WAL `OP_HAMILTONIAN_DECLARE
  = 0x0C` variant; group-agnostic by construction; integrator
  refactor + KogutSusskind lift + WAL replay materialization
  explicitly deferred; commit `TBD`).
- `theory/halcyon/HALCYON_REQUEST_2026-06-19_SNAPSHOT_EVERY.md` — pattern
  this log mirrors.
- `theory/halcyon/GIGI_TO_HALCYON_REPLY_2026-06-19.md` — Halcyon Part V
  reply, the prior cross-team handoff this letter chases in shape.
- `src/lattice/topology/truncated_icosahedron.rs` — reference constructor
  that `CUBED_SPHERE` registers peer to; source of CC-4 promotion
  (function `buckyball_with_signed_faces`).
- `src/lattice/registry.rs` — registry shape A1 lands through; CC-2
  refactor extends with `register_constructor` / `get_constructor`.
- `src/lattice/mod.rs:76-91` — `Lattice` struct: topology only, no
  metric (preserved post-Q3).
- `src/lattice/metric.rs` (new, Phase 2) — `EdgeMetric` / `FaceMetric` /
  `CellArea` traits + `LatticeWithMetric` wrapper.
- `src/lattice/dec/mod.rs` (new, Phase 2) — `d_0`, `delta_0`,
  `hodge_star_k` as free functions consuming `LatticeWithMetric`.
- `src/lattice/topology/hints.rs` (new, Phase 1) — reserved
  `topology_hint` vocabulary const table.
- `src/discrete/hodge_complex.rs:1-9, 143-171` — existing bundle-side
  DEC; peer to the new lattice-side `lattice::dec` module.
- `src/gauge/holonomy.rs:27-37` — `walk_loop`; not a DEC primitive
  (group product, not form integration).
- `src/gauge/project_gauss.rs:23-46` — covariant divergence with SU(2)
  adjoint action; gauge-covariant, not metric `d_0^†`.
- `src/gauge/hamiltonian_registry.rs` (new, Phase 2) — CC-1 open-registry
  surface.
- `src/gauge/action.rs` (new, Phase 2) — `HamiltonianForce` /
  `HamiltonianDrift` / `ProjectionOperator` / `EnergyDecomposition`
  traits.
- `src/gauge/symplectic_flow.rs` — integrator A2 refactors; outer-loop
  enum match over `HamiltonianKind`; inner loop generic over concrete
  `H`.
- `src/gauge/registry.rs:1-241` — gauge-field registry (precedent for
  CC-1 hamiltonian registry shape).
- `src/wal.rs:552-620, 776-843` — WAL encoding precedent for
  metadata-only trait-object handles; CC-1 `HamiltonianDeclare` follows
  the same pattern.
- `src/parser.rs:8773-8808` — canonical-identifier dispatch; CC-2
  refactor replaces this with registry lookup.
- `src/types.rs` — `FieldType` / `Value` / `BundleSchema` shape
  A3-extension modifies.
- commit `5b555ce` — `/v1/gql` gauge dispatch fix.
- commit `ea50585` — lattice/gauge primitives split out of halcyon
  namespace (the post-split surface is what enables AURORA's first
  receipt).
