# AURORA v0.1 engine-asks — tracking log

**Received** 2026-06-19 from AURORA (first cross-team contact, engine-asks
v0.1 spec + `williamson_test2_scaffold.py` scaffold).

**Reply letters**:
- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md` (initial engine response).
- `theory/aurora/AURORA_TO_GIGI_REPLY_2026-06-19.md` (AURORA decisions on CC-1..6 + Q3).
- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` (engine acceptance + Q3 answer + Q4/Q5/Q6).

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
  separate host binary for deploy. Reply 2 §11. **OPEN.**
- **Q5** (GIGI→AURORA): `aurora_crate::init()` lifecycle in host-binary
  `main()`; eager-vs-lazy registration; WAL replay ordering. Reply 2 §11.
  **OPEN.**
- **Q6** (GIGI→AURORA): binary-stability contract on the
  `HamiltonianHandle` trait surface; pinned-vs-floating gigi dep;
  stability annotation. Reply 2 §11. **OPEN.**

## Status board

| Ask | Shape | Phase | Status | Receipt |
| --- | --- | --- | --- | --- |
| P-1 (dispatch) | bugfix on `/v1/gql` | shipped | **DONE** | commit `5b555ce` |
| `ea50585` split | lift lattice/gauge out of halcyon namespace | shipped | **DONE** | commit `ea50585` |
| First AURORA receipt | Williamson Test 2 step 0 against `gigi-stream.fly.dev` | shipped | **DONE** | `refusal_reason = None`, forward Euler refuses at step 2 |
| Signed-face promotion (CC-4) | `Lattice::signed_face_orientations()`; ~80–100 LOC standalone PR | Phase 0 (lands first) | **GREENLIT** (engine commitment in Reply 2 §5; bit-identity risk = 0) | — |
| A1 — `CUBED_SPHERE` | parameterized PANEL_SIZE; calls promoted signed-face surface; constructor returns `LatticeWithMetric` | Phase 1 | greenlit, gated on Phase 0 | — |
| CC-2 registry-dispatch refactor | replace static match arm with `lattice::registry::get_constructor()`; ~110–175 LOC | Phase 1 (lifted from "optional" into A1 sprint) | **ACCEPTED** (Reply 2 §3) | — |
| `topology_hint` const table | `src/lattice/topology/hints.rs` reserves `S2/CUBED_SPHERE`, `S2/TRUNCATED_ICOSAHEDRON`; ~30 LOC | Phase 1 | accepted | — |
| A3-workaround | six flat scalar names: `kelvin_holonomies_eq/n30/s30`, `c_field_min/max/mean`; gate via `COVER WHERE refusal_reason IS NULL` | Phase 1 | **ACCEPTED** (Reply 2 §9; zero engine LOC) | — |
| CC-1 `hamiltonian_registry` | new `src/gauge/hamiltonian_registry.rs`; trait-object factory; WAL `HamiltonianDeclare` metadata-only | Phase 2 (gated on Q4/Q5/Q6) | **ACCEPTED** (Reply 2 §2) | — |
| A2 — four-trait refactor + `ShallowWater` | `HamiltonianForce` / `HamiltonianDrift` / `ProjectionOperator` / `EnergyDecomposition`; generic `State`; `project_constraint` + `PROJECT_GAUSS` sugar; free-form energy map; group-agnostic `KogutSusskind { beta }` | Phase 2 | **ACCEPTED** with hot-path constraint: trait-object dispatch off integrator inner loop, generic over concrete `H: HamiltonianForce + HamiltonianDrift` | — |
| Q3 — DEC operator surface | `src/lattice/metric.rs` + `src/lattice/dec/` (NEW); `d_0`, `delta_0`, `hodge_star_k` as free functions consuming `LatticeWithMetric`; ~250–400 LOC | Phase 2 (lands before AURORA's `HamiltonianForce` impl) | **ACCEPTED** (Reply 2 §10) | — |
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
- Bee greenlit: pending on Phase 0 (CC-4 promotion) kickoff.
- Sprint slot: not assigned.
- AURORA-side blocking pressure: P-1 shipped; first receipt validated;
  Phase 0 (CC-4) engine-side standalone; Phase 1 (A1 + CC-2 + A3-workaround)
  unblocked once Phase 0 lands; Phase 2 (A2 + CC-1 + Q3 DEC module) gated
  on Q4/Q5/Q6 answers from AURORA.

## References

- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md` — initial
  engine response.
- `theory/aurora/AURORA_TO_GIGI_REPLY_2026-06-19.md` — AURORA's decisions
  on CC-1..6 + design pins + Q3.
- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` — engine
  acceptance + Q3 answer + Q4/Q5/Q6 back to AURORA.
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
