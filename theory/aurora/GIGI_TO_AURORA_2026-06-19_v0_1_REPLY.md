# GIGI → AURORA reply, v0.1 engine-asks scope review (2026-06-19)

**From:** GIGI engine team (Bee + Claude)
**To:** AURORA team
**Subject:** Welcome. P-1 already shipped. A1/A2/A3 scope review + answers to Q1 + Q2 + cross-cutting design questions worth pinning before implementation.
**Companion to:** AURORA engine-asks v0.1 spec + `williamson_test2_scaffold.py` (the prior cross-team artifact).

---

## Letter

AURORA —

Welcome. This is a first cross-team letter from the engine side, so a piece of framing first: the substrate you are asking for is not Halcyon-shaped, and it is not supposed to be. The split we landed at commit `ea50585` ("split general lattice/gauge primitives out of halcyon namespace") was done precisely so non-Halcyon Gi-Systems could land cleanly. The fact that AURORA showed up the next day with a shallow-water-on-cubed-sphere spec is the validation event for that split. We will treat `CUBED_SPHERE` and `TRUNCATED_ICOSAHEDRON` as peer topologies, `ShallowWater` and `KogutSusskind` as peer Hamiltonians. Neither domain is privileged in the engine surface; both are first-class.

Two pieces of news up front.

First, your P-1 dependency (the `/v1/gql` gauge dispatch fix) is already done. It shipped in this session at commit `5b555ce` ("Halcyon Part V P-1 — fix /v1/gql gauge dispatch"). Halcyon was the first consumer; AURORA is the second. Your `williamson_test2_scaffold.py` should land HTTP requests through `/v1/gql` cleanly today against `gigi-stream.fly.dev` — no AURORA-specific work required. The fix is in the general dispatch path, not in any Halcyon-keyed branch.

Second, the post-split engine surface is asymmetric across your three asks. A1 (`CUBED_SPHERE`) is "register a constructor" — mechanical enum extension on the post-`ea50585` shape. A3 (`AURORA_RECEIPT` schema) has a production-ready workaround that ships today with zero engine LOC, plus an orthogonal general-purpose DSL extension that would benefit every future bundle author. A2 (`ShallowWater` Hamiltonian) is the real engineering work: the `ea50585` split exposed `Lattice`, `lattice::registry`, `GroupElement`, `EdgeConnection`, and `walk_loop` as general-purpose, but it did not touch the `SYMPLECTIC_FLOW` integrator. The integrator is still SU(2)-Wilson-specific top to bottom — `wilson_force_per_edge` hardcodes the staple sum with the `-β/8` coefficient for N=2, `drift_step` is Rodrigues on an imaginary quaternion, `project_gauss` bakes in covariant divergence plus the SU(2) adjoint action, and the kinetic energy decomposition assumes `g² = 4/β`. `ShallowWater` cannot land as a peer `HamiltonianKind` until those four kernels get extracted into traits. That refactor is the path; we want to pin one cross-cutting design question with you before starting it (see §6).

Below: P-1 confirmation (§1), what's already general-purpose on the post-`ea50585` surface (§2), A1 scope (§3), A2 scope (§4), A3 scope and the Q1 answer (§5), cross-cutting general-purpose design questions worth pinning across Halcyon + AURORA + future Gi-Systems (§6), the Q2 answer on seam handling (§7), recommended phase order (§8), and coordination protocol (§9). Pushback welcome on every clause.

—Bee + Claude

---

## 1. P-1 dependency: shipped

| Receipt | Value |
| --- | --- |
| `/v1/gql` gauge dispatch fix | commit `5b555ce` ("Halcyon Part V P-1 — fix /v1/gql gauge dispatch") |
| Shape | `try_dispatch_gauge_statement` helper with a 13-variant match arm, dispatched through `parser::execute` + `exec_result_to_response` |
| AURORA exposure | `/v1/gql` accepts every gauge verb (LATTICE / GAUGE_FIELD / SYMPLECTIC_FLOW / GIBBS_SAMPLE / SHOW LATTICE / etc.) and routes them through the real executor, not the bundle default early-return that was silently returning `{"status":"ok"}` before |
| Halcyon live verification | `GET /v1/lattice/buckyball_p1b` returned `LatticeView { n_vertices: 60, n_edges: 90, n_faces: 32 }` on the production image — declarations land, not just acknowledged |

The dispatch path is domain-agnostic. Your `williamson_test2_scaffold.py` does not need a Halcyon equivalent of the fix; the helper covers every gauge statement through one entry point.

---

## 2. What's already general-purpose on the engine surface

Commit `ea50585` exposed the following as general-purpose under the `lattice` + `gauge` feature flags, with no Halcyon-namespace dependency:

- `Lattice` — graph topology struct with `vertices`, `edges`, `faces`, `topology_hint`. Pure combinatorial graph; topology-agnostic.
- `lattice::registry` — in-memory `HashMap`-based `register` / `get` / `clear`. Same registry shape `CUBED_SPHERE` will register through.
- `lattice::topology::truncated_icosahedron::buckyball()` — constructor returning `Lattice`. Registers via the canonical-identifier match arm in the parser. `CUBED_SPHERE` registers identically.
- `GroupElement` — enum with SU(2) math (quaternion implementation); U(1) and Z_N stubs in place for future fills.
- `EdgeConnection` — trait for group-erased per-edge elements.
- `holonomy::walk_loop` — generalized loop walker, parameterized over `EdgeConnection`. Works on any `Lattice` + any `GroupElement`.
- `LATTICE` statement variants in the parser + `SHOW LATTICE` introspection.

What stayed Halcyon-internal (deliberately, for the bit-identity gold fixture):

- `buckyball_with_signed_faces()` — returns Halcyon-indexed face-signing for the bit-identity test. The general `buckyball()` constructor does NOT export signed faces.
- `signed_face_to_walker` — Halcyon-specific face-signing converter.
- `UFinalConnection` — test backing for the gold-fixture cross-check.
- The Halcyon Part I bit-identity integration test + the gold fixtures in `tests/fixtures/halcyon/`.

Feature flags (`lattice`, `gauge`, `halcyon`) are domain-agnostic. AURORA can compile against `lattice + gauge` without dragging in any Halcyon-specific code. The signed-face question — whether the signed-face surface should be promoted out of Halcyon-internal into the general `lattice::topology` API — is one of the cross-cutting questions in §6.

---

## 3. A1 — `CUBED_SPHERE` topology

### Shape

**Register a constructor.** Enum extension on the post-`ea50585` dispatch surface. No refactor required.

### Scope

- New module `src/lattice/topology/cubed_sphere.rs` with `pub fn cubed_sphere() -> Lattice` (or `cubed_sphere(panel_size: usize) -> Lattice` if parameterized; see design question below). 6 panels × C² cells; the constructor owns the panel-adjacency resolver and emits the cross-seam edges directly into the `Lattice.edges` vector.
- One match arm in `src/parser.rs` at the canonical-identifier dispatch site (~line 8778): `"CUBED_SPHERE" => { crate::lattice::topology::cubed_sphere::cubed_sphere() }` peer to the existing `"TRUNCATED_ICOSAHEDRON" =>` arm.
- One `pub mod cubed_sphere;` line in `src/lattice/topology/mod.rs`.
- New integration test `tests/lattice_cubed_sphere.rs` covering `CREATE LATTICE` round-trip + `SHOW LATTICE` introspection.

LOC estimate: ~150–250 total — ~100–180 for the constructor's graph generation (vertex enumeration, intra-panel edges, cross-seam edges, face enumeration), ~3 for the parser arm, ~1 for the mod line, ~50–70 for the tests.

### Design questions worth pinning

- **Panel-resolution parameter.** Is `C` a parser-level integer argument to `CREATE LATTICE CUBED_SPHERE` (`CREATE LATTICE atmos FROM CUBED_SPHERE PANEL_SIZE 32 TOPOLOGY 'S2'`) or a fixed constructor constant? `TRUNCATED_ICOSAHEDRON` is parameter-free today; if `CUBED_SPHERE` takes a size, the parser arm shape diverges and we should decide whether the parameter-free form is the special case or whether parameterized constructors become the norm going forward. Either is fine; pinning it now keeps the parser arm clean. Recommendation: parameterized is the more general primitive, and `TRUNCATED_ICOSAHEDRON` can take an implicit `LEVEL 0` that future subdivision schemes (geodesic refinement) would extend.
- **Face orientation convention.** Does `CUBED_SPHERE` export signed faces (for downstream curl / divergence on plaquettes) the same way `buckyball_with_signed_faces` does internally for Halcyon? If yes, the signed-face surface should be promoted out of the Halcyon-private module and standardized across topologies (this is one of the §6 cross-cutting questions). If no, AURORA needs to declare which orientation convention shallow-water plaquettes assume, and the convention becomes part of `CUBED_SPHERE`'s public contract.
- **Seam-handling ownership.** No shared seam primitive exists today. The current contract is that the constructor owns it: the returned `Lattice.edges` already encodes cross-panel adjacencies; downstream code reads the edge list and trusts that the constructor enumerated them correctly. This is fine for a single topology, but `CUBED_SPHERE` + future tori + future Riemann surfaces all share the "non-trivial gluing" property and would each re-implement seam logic. Whether `Lattice` should grow an optional `seam_metadata` field is in §6.
- **`topology_hint` string.** `Lattice::topology_hint` is a free-form string today. If downstream Hamiltonians and observables dispatch on it, the vocabulary needs to be reserved (e.g. `"S2/CUBED_SPHERE"`, `"S2/TRUNCATED_ICOSAHEDRON"`). If it's purely informational, leave it free-form. Worth pinning so future audits don't end up keying off a string that nobody documented.

---

## 4. A2 — `ShallowWater` Hamiltonian

### Shape

**Refactor first, then register a constructor.** The post-`ea50585` split was a lattice-surface lift, not an integrator lift. The four kernels of the KDK loop are SU(2)-Wilson-specific and need to be extracted into traits before `ShallowWater` can land as a peer.

### Scope

- New module `src/gauge/action.rs` defining the trait surface: `HamiltonianForce`, `HamiltonianDrift`, `ProjectionOperator`, `EnergyDecomposition`. ~300 LOC.
- New module `src/gauge/shallow_water_action.rs` — `ShallowWater { g, omega, a }` implementation of the four traits. Force is the gradient of the shallow-water potential, drift is the velocity / momentum update, projection is a weighted Poisson solver enforcing incompressibility, energy decomposition publishes the labeled observables AURORA wants. ~200 LOC.
- New module `src/gauge/yang_mills_action.rs` — relocate `wilson_force_per_edge` as the `KogutSusskind { beta }` impl of the same traits. Leaves `src/gauge/wilson_force.rs` as the SU(2)-specific inner math (Wilson staple sum, `-β/8`). Behavior preserved for Halcyon; no bit-identity drift.
- Refactor `src/gauge/symplectic_flow.rs` integrator loop to dispatch on `HamiltonianKind` enum instead of calling `wilson_force_per_edge` / `drift_step` / `project_gauss` directly. ~150 LOC.
- Extract `src/gauge/project_gauss.rs::apply_l_cov_matvec` into a `ProjectionOperator` trait method so `ShallowWater` can supply a weighted-Poisson matvec.
- Parser: extend `SYMPLECTIC_FLOW` grammar with an optional `HAMILTONIAN` clause parsing `KogutSusskind { beta }` and `ShallowWater { g, omega, a }` as peer variants. ~50–100 LOC.

`src/gauge/lie_exp.rs` stays as-is — Rodrigues exponential on imaginary quaternion remains the SU(2) drift primitive that the `KogutSusskind` impl uses. `ShallowWater`'s drift is a different kernel and ships in `shallow_water_action.rs`.

Total: ~600–900 LOC across one new trait module, two new impl modules, two refactors, one parser extension. Tests not counted.

### Design questions worth pinning before any LOC lands

These are the questions where getting the answer wrong costs a second refactor:

- **Trait shape: four traits or one.** Are `HamiltonianForce`, `HamiltonianDrift`, `ProjectionOperator`, `EnergyDecomposition` four separate traits, or one `HamiltonianKernel` trait with four methods? Four traits compose better for partially-overlapping physics (a non-Wilson SU(2) action could reuse drift + projection but supply its own force); one trait is simpler but couples the four kernels into a single impl block.
- **Drift signature.** `KogutSusskind` drift updates SU(2) link variables (quaternion exponentiation); `ShallowWater` drift updates real-valued momentum / velocity fields living on edges or faces. Common type signature: a generic `State` associated type per `HamiltonianKind`, or do both Hamiltonians agree on "per-edge real vector" and embed group elements as a basis?
- **Projection contract.** `KogutSusskind` projects onto the Gauss constraint surface; `ShallowWater` projects onto an incompressibility / mass-conservation constraint. Is `project_gauss` renamed to something neutral (`project_constraint`), and does the parser keep `PROJECT_GAUSS` as a sugar alias that routes through the trait, or does the parser gain a peer `PROJECT_MASS` clause?
- **Energy decomposition contract.** Halcyon reports K + V Wilson decomposition; AURORA's Kelvin holonomies and c-field summary are not K + V. Does `EnergyDecomposition` return a free-form labeled map (`HashMap<&'static str, f64>`) so each Hamiltonian publishes its own observables, or a fixed `(kinetic, potential)` tuple that `ShallowWater` would have to flatten into? Free-form map is more honest about the asymmetry; fixed tuple makes downstream observables code simpler at the cost of forcing a flattening lie for non-K+V physics.
- **Naming.** `SU2_KogutSusskind` today bakes the group into the name. Post-refactor, should the enum variant be `KogutSusskind { beta }` (group-agnostic, since the Wilson action is defined for any compact group, with the SU(2) restriction enforced at the executor by checking `U_handle.group()`) or `KogutSusskind { group: Group, beta: f64 }` (group as a field)? This decision determines whether SU(3) lattice QCD lands as a new `HamiltonianKind` variant or as a new `Group` variant under the existing `KogutSusskind`. Recommendation: group-agnostic name, executor enforces the compatibility check.

The one cross-cutting question that should be answered before the refactor starts is in §6 (closed enum vs open trait-object registry).

---

## 5. A3 — `AURORA_RECEIPT` schema + Q1 answer

### Q1 — does the schema DSL support fixed-length arrays + inline anonymous structs in `CREATE BUNDLE SCHEMA`?

**No, not today.** The schema parser accepts scalar field types (`INT` / `INTEGER` / `NUMERIC` / `FLOAT` / `REAL` / `DOUBLE` / `TEXT` / `VARCHAR` / `STRING` / `CATEGORICAL` / `BOOL` / `BOOLEAN` / `TIMESTAMP`) plus a fixed-dim `Vector` embedding (`FieldType::Vector { dims }` per `src/types.rs:16-20`, used for kNN-participating dense f64 embeddings). It does not parse `[TYPE; N]` fixed-length array syntax and does not parse `{ field: TYPE, ... }` inline anonymous struct syntax.

The "refusal_reason IS NULL" predicate works today with zero engine changes: `FilterCondition::Void("refusal_reason")` maps cleanly to `COVER WHERE` and is fully supported. A schema-attached `RECEIPT_GATE` keyword does not exist; the current shape is that gates live on `COVER` queries via `FilterCondition`. Whether to introduce a schema-level CHECK-style invariant is a cross-cutting question in §6.

### Shape

**Refactor first for the requested DSL extension. Workaround ships today.**

### Production workaround (ships immediately, zero engine LOC)

```
CREATE BUNDLE AURORA_RECEIPT
  BASE (
    run_id TEXT REQUIRED,
    refusal_reason TEXT,
    ...
  )
  FIBER (
    kelvin_holonomies_x NUMERIC,
    kelvin_holonomies_y NUMERIC,
    kelvin_holonomies_z NUMERIC,
    c_field_min NUMERIC,
    c_field_max NUMERIC,
    c_field_mean NUMERIC,
    ...
  );
```

Six scalar fields stand in for the array + the inline struct. The flattened bundle is fully queryable via existing `COVER WHERE` machinery, encryptable per-field, and uses zero engine changes. The "refusal_reason IS NULL" gate lands as a documented contract on the bundle ("every AURORA_RECEIPT read MUST be wrapped in `COVER WHERE refusal_reason IS NULL`") rather than as a schema-time CHECK, until the cross-cutting decision in §6 settles.

The tradeoff vs the array+struct shape AURORA asked for: separate scalars are individually queryable (`COVER WHERE c_field_min > 0` works directly); a `Vector` field is not directly queryable element-wise but participates in kNN. For Kelvin holonomies — which downstream tensor code will probably want to read as a 3-component object — three scalars are more composable with existing query machinery and trivially re-grouped later if the DSL extension lands.

### DSL extension scope (orthogonal, can ship independently)

- **Fixed-length arrays `[TYPE; N]`**: ~500–800 LOC across tokenizer (`[`, `;`, `]` sequence parsing), `FieldType` enum (`Array { element: Box<FieldType>, length: usize }` variant), `Value` enum (`Array(Vec<Value>)` variant), `FieldSpec` parser arm, serialization in `src/mmap_bundle.rs`, encryption transform in `src/crypto.rs`, tests.
- **Inline anonymous structs `{ field: TYPE, ... }`**: ~700–1200 LOC additional. Similar tokenizer + AST work plus a non-trivial extension to the `FilterCondition` expression parser to support nested-field access (e.g. `COVER WHERE c_field_summary.min > 0`), or a parser-time desugar to flat synthetic columns (`c_field_summary__min`, etc.) that ships without touching the query engine.

Files in scope: `src/parser.rs`, `src/types.rs`, `src/bundle.rs`, `src/mmap_bundle.rs`, `src/engine.rs`, `src/crypto.rs`, `GIGI_API.md`, plus new tests under `tests/parser_arrays.rs` and `tests/parser_structs.rs`.

### Design questions worth pinning

- **`RECEIPT_GATE` keyword vs `COVER WHERE` discipline.** AURORA's v0.1 spec proposes a `RECEIPT_GATE refusal_reason IS NULL` clause as a schema-attached predicate. The engine has no schema-time boolean gate today; predicates live on queries. Should the engine introduce schema-level gates (a CHECK-style invariant evaluated on `INSERT` / commit), or should AURORA encode the gate as a mandatory `COVER` predicate documented in the bundle's contract? The general-purpose answer determines whether every future bundle gets a CHECK clause or whether gates remain a query-side discipline. This is in §6.
- **Array vs Vector field type.** `[FLOAT64; 3]` could (a) reuse the existing `Vector` field type with `dims = 3`, (b) introduce a new `Array<FLOAT64>` that is structurally distinct from `Vector`, or (c) keep both, where `Vector` is kNN-participating and `Array` is `COVER`-queryable element-wise. AURORA's kelvin_holonomies usage probably doesn't want kNN semantics, which argues for either (b) or (c). Worth pinning.
- **Nested-field query syntax.** `c_field_summary.min` in `COVER WHERE` requires either dotted-path `FilterCondition` (extend the LHS of every `FilterCondition` variant from a column name to a path) or autogenerated synthetic columns (parser desugars the struct into `c_field_summary__min`, `c_field_summary__max`, `c_field_summary__mean` at parse time). Desugar ships without touching the query engine; dotted-path is more honest but touches every `FilterCondition` arm.
- **v1 shipping shape.** If AURORA wants the v1 `AURORA_RECEIPT` bundle to ship on the flattened-scalars workaround, the bundle name in the spec should match what the code stores (per the "spec the algorithm, not the prose" rule). If the array+struct DSL extension lands later, the bundle can re-group its 6 scalars into 1 array + 1 struct without losing existing data — but the spec should not currently read as if the structured shape is already there.

---

## 6. Cross-cutting general-purpose design questions

These are the questions where the answer affects Halcyon, AURORA, and every future Gi-System. Each one is worth pinning before the next refactor lands, because reversing the decision later costs a second refactor.

### CC-1 — `HamiltonianKind`: closed enum vs open trait-object registry

The natural shape after the A2 trait refactor is for `HamiltonianKind` to be either:

- a **closed enum** (`KogutSusskind`, `ShallowWater`, and any future variant requires recompiling gigi). Matches today's `Group` dispatch. Parser arm stays a static match.
- an **open trait-object registry** analogous to `lattice::registry` (gigi exposes `register_hamiltonian(name, constructor)` and downstream Gi-Systems can ship their own Hamiltonians as crates without patching gigi). Matches `lattice::registry`. Lets AURORA own `ShallowWater` end-to-end as a downstream crate.

The closed-enum approach is simpler today; the open-registry approach matches the direction the lattice surface already went. This question should be answered before the A2 refactor starts, because reversing it after A2 ships costs a second refactor.

### CC-2 — `LatticeTopology`: same closed-enum vs open-registry question

The post-`ea50585` shape is already a registry of constructors (`lattice::registry`) but the parser dispatch site is a static match on canonical-identifier strings. Worth deciding whether `CREATE LATTICE FROM <ident>` should dispatch through the registry directly (so downstream crates can register topologies without patching gigi's parser) or stay on the static match. Same shape question as CC-1, applied to topologies.

### CC-3 — `Lattice::seam_metadata` as a first-class field

Buckyball has trivial gluing (no seams). `CUBED_SPHERE` has six 90-degree panel seams. Future topologies (torus, hyperbolic tilings, Riemann surfaces) all have non-trivial gluing. Should `Lattice` grow an optional `seam_metadata: Option<SeamMap>` field that constructors populate, so Hamiltonians and observables can query gluing in a uniform way? Without it, every new S²-or-richer topology buries its seam logic in its edge enumeration and downstream code can't tell that two topologies share a gluing structure.

### CC-4 — Signed-face surface promotion

`buckyball_with_signed_faces` is Halcyon-internal today (for the bit-identity gold fixture). But signed faces are a general primitive any Hamiltonian needs for plaquette curl / divergence — ShallowWater needs them too. Should the signed-face computation be promoted out of the Halcyon-private path into a general `lattice::topology` surface that every constructor publishes? If yes, both `TRUNCATED_ICOSAHEDRON` and `CUBED_SPHERE` (and future topologies) expose signed faces uniformly; if no, each Hamiltonian re-derives signs from raw faces.

### CC-5 — Schema DSL extension as a general-purpose lift

Fixed-length arrays and inline anonymous structs benefit every bundle author, not just AURORA. Should the DSL extension be sequenced as a general-purpose engine feature (planned and scoped on its own merits) rather than as an AURORA-shaped feature? The general-purpose framing avoids baking AURORA-specific assumptions (exactly 3-element arrays, exactly `{min, max, mean}` structs) into the parser. Recommendation: yes, sequence it as a general-purpose lift; AURORA's `AURORA_RECEIPT` ships on the flattened-scalars workaround in the meantime.

### CC-6 — `RECEIPT_GATE` as a schema-attached CHECK clause

AURORA wants a schema-level gate; the engine has only query-side `FilterCondition` today. Introducing schema-level CHECK invariants would generalize: every bundle could declare predicates that fire on `INSERT` / commit, not just on read. Worth deciding whether this is the engine direction or whether gates stay query-side discipline.

---

## 7. Q2 — does the lattice registry have a seam-handling primitive for cubed-sphere panel adjacency, or does `CUBED_SPHERE` ship its own?

**`CUBED_SPHERE` ships its own.** The lattice registry does not have a seam-handling primitive for non-trivial gluing. The `buckyball` constructor traces faces via a rotation-system combinatorial map but does not export a seam resolver; the post-split `Lattice` struct is a pure vertex/edge/face graph with a `topology_hint` string and exposes no `seam_metadata` field.

So `CUBED_SPHERE` owns the responsibility of correctly enumerating:

1. The intra-panel edges (vertices within one of the six panels connected by the obvious grid),
2. The cross-seam edges (vertices on a panel boundary connected to vertices on an adjacent panel across the 90-degree fold),
3. Whatever orientation-flip bookkeeping shallow-water dynamics need at the seams (e.g. which seams flip the sign of a flux variable on the adjacent panel).

The constructor returns a `Lattice` whose `edges` list already encodes the cross-panel adjacencies; any orientation convention the constructor adopts becomes part of `CUBED_SPHERE`'s public contract.

This is fine for a single new topology, but it is exactly the kind of work the cross-cutting question CC-3 (`Lattice::seam_metadata`) would generalize across future topologies. If CC-3 lands first, the seam information is published in a uniform shape that ShallowWater (and future Hamiltonians on torus / Riemann-surface lattices) can consume without knowing which constructor was used.

---

## 8. Recommended phase order

| Phase | Scope | Status |
| --- | --- | --- |
| P0 | `/v1/gql` gauge dispatch fix | **shipped at `5b555ce`** |
| P0 | `ea50585` split: `Lattice`, `lattice::registry`, `GroupElement`, `EdgeConnection`, `walk_loop`, `LATTICE` statement variants, `SHOW LATTICE` introspection as general-purpose surface | **shipped at `ea50585`** |
| Phase 1 (parallel) | A1 (`CUBED_SPHERE` topology constructor) AND A3-workaround (`AURORA_RECEIPT` as 6 flattened scalar fields). Both land without touching `SYMPLECTIC_FLOW`. A1 is mechanical enum extension; A3-workaround is AURORA-side schema authoring with zero engine LOC. These two unblock AURORA's first end-to-end smoke run. | unblocked |
| Phase 2 | A2 trait refactor of `src/gauge/symplectic_flow.rs` — extract `HamiltonianForce` / `HamiltonianDrift` / `ProjectionOperator` / `EnergyDecomposition` traits. `KogutSusskind { beta }` becomes the first impl (no behavior change for Halcyon), `ShallowWater { g, omega, a }` becomes the second impl. Parser gains `HAMILTONIAN` clause. | blocked on CC-1 (enum vs registry) being answered |
| Phase 3 (optional general-purpose lift) | Schema DSL extension for `[TYPE; N]` fixed-length arrays and `{ field: TYPE, ... }` inline structs. Sequenced on its own merits as an engine-wide feature; `AURORA_RECEIPT` then optionally re-groups its 6 scalars into 1 array + 1 struct without losing existing data. | independent |
| Phase 4 (optional general-purpose lift) | `Lattice::seam_metadata` + signed-face surface promotion (CC-3 + CC-4), if the cross-cutting answers point that way. Benefits A1 retroactively and every future S²-or-richer topology. | independent |

Phase 1 lets AURORA get to a smoke run: `CREATE LATTICE atmos FROM CUBED_SPHERE TOPOLOGY 'S2'` + `CREATE BUNDLE AURORA_RECEIPT ...` (flattened) + `COVER ... WHERE refusal_reason IS NULL`. That's a real end-to-end path with no engine refactor in front of it. Phase 2 is the real engineering work and shouldn't start until CC-1 settles. Phases 3 and 4 are independent lifts that any of the three teams (Halcyon, AURORA, or a future system) can sponsor.

---

## 9. Sprint coordination protocol

Same shape as Halcyon's working pattern:

- Cross-team specs drop under `theory/aurora/`. We commit them to the gigi repo so the engine-side audit trail and the AURORA-side audit trail share a single source of truth.
- Replies and scope reviews ship as letters under `theory/aurora/` (this letter is the first). Letters are reserved for cross-team handoffs and spec reviews; routine iteration receipts live in commits.
- Implementation logs land under `theory/aurora/AURORA_PART_<N>_IMPLEMENTATION_LOG.md` once a sprint starts, mirroring `HALCYON_PART_<N>_IMPLEMENTATION_LOG.md`.
- A tracking log for the v0.1 engine asks lives at `theory/aurora/AURORA_ASKS_v0_1_LOG.md` and gets updated as A1 / A2 / A3 ship.
- Sober register. Spec the algorithm, not the prose. Name things by what the code returns.

The general-purpose framing is load-bearing. We want both Halcyon and AURORA looking at the same engine surface — `LATTICE`, `HamiltonianKind`, `CREATE BUNDLE` — and seeing peer primitives, not domain-specific branches. If a future letter starts describing `CUBED_SPHERE` as "the AURORA topology" or `ShallowWater` as "the AURORA Hamiltonian," we've drifted; pushback welcome on that drift the same way you'd push back on a parser bug.

Pushback welcome on every clause in this letter. The questions in §6 in particular are decisions, not announcements — we want AURORA's read before they land.

—Bee + Claude
