# AURORA v0.1 engine-asks — tracking log

**Received** 2026-06-19 from AURORA (first cross-team contact, engine-asks
v0.1 spec + `williamson_test2_scaffold.py` scaffold).

**Reply letter**: `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md`.

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

Two questions back from AURORA:

- **Q1**: does the schema DSL support fixed-length arrays + inline
  anonymous structs in `CREATE BUNDLE SCHEMA`? **Answer: no, not today.**
  See the reply letter §5.
- **Q2**: does the lattice-registry have a seam-handling primitive for
  cubed-sphere panel adjacency, or does `CUBED_SPHERE` ship its own?
  **Answer: `CUBED_SPHERE` ships its own.** See the reply letter §7.

## Status board

| Ask | Shape | Phase | Status | Receipt |
| --- | --- | --- | --- | --- |
| P-1 (dispatch) | bugfix on `/v1/gql` | shipped | **DONE** | commit `5b555ce` |
| `ea50585` split | lift lattice/gauge out of halcyon namespace | shipped | **DONE** | commit `ea50585` |
| A1 — `CUBED_SPHERE` | register a constructor | Phase 1 | greenlight pending | — |
| A3-workaround | flatten array+struct to 6 scalars; gate via `COVER WHERE` | Phase 1 | greenlight pending (AURORA-side schema authoring; zero engine LOC) | — |
| A2 — `ShallowWater` | refactor first, then register | Phase 2 | blocked on CC-1 | — |
| A3-extension — `[TYPE; N]` arrays | DSL lift | Phase 3 (optional general-purpose) | not started | — |
| A3-extension — inline structs | DSL lift | Phase 3 (optional general-purpose) | not started | — |
| `Lattice::seam_metadata` | general lift (CC-3) | Phase 4 (optional general-purpose) | not started | — |
| Signed-face promotion | general lift (CC-4) | Phase 4 (optional general-purpose) | not started | — |

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

## Cross-cutting general-purpose questions (CC-N)

These affect Halcyon + AURORA + every future Gi-System.

- **CC-1 — `HamiltonianKind`: closed enum vs open trait-object registry.**
  Closed enum matches today's `Group` dispatch. Open registry matches
  `lattice::registry` and lets downstream crates ship their own
  Hamiltonians without patching gigi. Decision required before A2.
- **CC-2 — `LatticeTopology` dispatch: same closed-enum vs open-registry
  question, applied to topologies.** Today the parser dispatches on
  canonical-identifier strings against a static match. Could route
  through `lattice::registry` directly.
- **CC-3 — `Lattice::seam_metadata` as a first-class field.** Would
  generalize panel-adjacency / gluing across `CUBED_SPHERE`, future tori,
  future Riemann surfaces. Without it, each topology buries seam logic
  in its edge enumeration.
- **CC-4 — signed-face surface promotion.** `buckyball_with_signed_faces`
  is Halcyon-internal today. ShallowWater needs signed faces too.
  Promote to general `lattice::topology` surface?
- **CC-5 — schema DSL extension as general-purpose lift.** Fixed-length
  arrays + inline anonymous structs benefit every bundle author.
  Sequence as general engine feature, not as AURORA-shaped feature.
- **CC-6 — `RECEIPT_GATE` as schema-attached CHECK clause.** Engine has
  only query-side `FilterCondition` today. Schema-level gates would
  generalize: predicates that fire on `INSERT` / commit, not just on
  read.

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
- Reply letter sent: `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md`.
- Bee greenlit: pending.
- Sprint slot: not assigned.
- AURORA-side blocking pressure: P-1 unblocked (already shipped); A1 +
  A3-workaround unblocked from engine side; A2 awaiting CC-1 decision.

## References

- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md` — the reply
  letter this log tracks against.
- `theory/halcyon/HALCYON_REQUEST_2026-06-19_SNAPSHOT_EVERY.md` —
  pattern this log mirrors.
- `theory/halcyon/GIGI_TO_HALCYON_REPLY_2026-06-19.md` — Halcyon Part V
  reply, the prior cross-team handoff this letter chases in shape.
- `src/lattice/topology/truncated_icosahedron.rs` — reference
  constructor that `CUBED_SPHERE` registers peer to.
- `src/lattice/registry.rs` — registry shape A1 lands through.
- `src/gauge/symplectic_flow.rs` — integrator A2 refactors.
- `src/gauge/wilson_force.rs`, `src/gauge/lie_exp.rs`,
  `src/gauge/project_gauss.rs` — SU(2)-Wilson-specific kernels A2
  abstracts.
- `src/types.rs` — `FieldType` / `Value` / `BundleSchema` shape A3-extension
  modifies.
- `src/parser.rs` — canonical-identifier dispatch (~line 8778) for A1,
  `SYMPLECTIC_FLOW` grammar for A2, `FieldSpec` grammar for A3-extension.
- commit `5b555ce` — `/v1/gql` gauge dispatch fix.
- commit `ea50585` — lattice/gauge primitives split out of halcyon
  namespace.
