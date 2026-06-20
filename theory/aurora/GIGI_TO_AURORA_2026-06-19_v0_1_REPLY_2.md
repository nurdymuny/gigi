# GIGI → AURORA reply 2, 2026-06-19

**Re:** Engine-side decisions on CC-1 through CC-6, A1/A2/A3 design pins, and Q3 (discrete operator surface)
**In reply to:** `theory/aurora/AURORA_TO_GIGI_REPLY_2026-06-19.md`
**Prior letter:** `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md`
**Author:** Bee Rosa Davis, with Claude (Anthropic)

---

## Letter

AURORA —

Williamson Test 2 step 0 receipt — `refusal_reason = None`, mass and energy diagnostics matching the analytical values within rounding, forward Euler refusing at step 2 on the energy-drift threshold — is the validation event for the post-`ea50585` split. The general-purpose `Lattice` + `gauge` surface compiled against by a downstream Gi-System with no Halcyon-specific code in the import graph is exactly what the split was for. We are recording this as the first AURORA receipt against `gigi-stream.fly.dev` and as the first cross-team validation that the substrate carries non-Yang-Mills physics. The forward-Euler refusal at step 2 is the integrator gap A2 closes; the receipt being honest about the refusal (`refusal_reason` populated rather than silently swallowed) is the contract working as designed.

This letter answers everything you raised. We accept the open-registry direction for CC-1 and CC-2, accept CC-3, accept CC-4 and commit to landing the signed-face promotion ahead of A1, accept CC-5 with the `__` desugar convention, and accept CC-6 query-side discipline. A1/A2/A3 design pins are accepted as written, with one performance constraint on A2 we want named explicitly (trait-object dispatch stays off the integrator hot path). On Q3 we are pushing back on framing: the right surface is a new `src/lattice/dec/` module consuming a `LatticeWithMetric` wrapper, not methods on `Lattice` itself — same operator vocabulary you asked for (`d₀`, `δ₀`, Hodge star), different carrier, and the reason is the same general-purpose discipline that drove `ea50585`. There are three cross-cutting questions back to you we want answered before A2 starts (Q4 aurora_crate repo location, Q5 init-hook lifecycle, Q6 binary-stability contract).

Decision summary up front, detail follows.

—Bee + Claude

---

## 1. Decision summary

| Item | Verdict | One-sentence rationale |
|---|---|---|
| CC-1 — `HamiltonianKind` open registry | ACCEPT | Mirror `lattice::registry` shape; downstream Gi-Systems register without patching gigi. |
| CC-2 — `LatticeTopology` open registry | ACCEPT, land inside A1 sprint | One-time refactor cost (~110–175 LOC) is smaller than the cost of shipping A1 on a static match arm and retrofitting later. |
| CC-3 — constructor-owned seam logic | ACCEPT | Matches buckyball precedent (rotation system + face tracing owned by the constructor, not the `Lattice` struct). |
| CC-4 — signed-face promotion before A1 | ACCEPT, lands as standalone PR | Bit-identity risk is zero per the signed-faces audit; LOC fits the 50–100 estimate; AURORA writes `cubed_sphere.rs` against the promoted surface day one. |
| CC-5 — schema DSL extension | ACCEPT, sequence as Phase 3 general lift | `__` (double underscore) is the documented desugar convention; v0.1 ships the six flat scalar names as a documented exception. |
| CC-6 — `RECEIPT_GATE` as query-side discipline | ACCEPT for v0.1 | `COVER WHERE refusal_reason IS NULL` is the contract; engine does not silently strip refusals; schema-level CHECK revisited later. |
| A1 panel-size, orientation, hint, seam pins | ACCEPT | As written, with CC-4 promoted ahead so the orientation path is via `Lattice::signed_face_orientations()`. |
| A2 four-trait, generic-State, projection, energy-map, naming pins | ACCEPT | With explicit constraint: trait-object dispatch is off the integrator hot path (inner loop is monomorphized via outer-loop enum match). |
| A3 six flattened scalars + query-side gate + `__` desugar | ACCEPT | Six scalar names land verbatim as the v0.1 contract; long-form `kelvin_holonomies__eq` retained as alias when Phase 3 lands. |
| Q3 — DEC operator surface | NEW `src/lattice/dec/` module, NOT methods on `Lattice` | DEC operators are metric-aware; `Lattice` stays metric-agnostic; metric carrier lives on a `LatticeWithMetric` wrapper. |

Three new questions back to you: Q4 (aurora_crate repo location), Q5 (init-hook lifecycle and WAL replay ordering), Q6 (binary-stability contract for the trait-object surface). Detail in §12.

---

## 2. CC-1 detail — open trait-object registry for `HamiltonianKind`

Accepted. Engine commits to mirroring the `lattice::registry` and `gauge::registry` shape verbatim. New module `src/gauge/hamiltonian_registry.rs`.

**Surface (the shape we will ship):**

```rust
pub fn register_hamiltonian(
    name: &'static str,
    factory: fn(&HashMap<String, f64>) -> Box<dyn HamiltonianHandle>,
);

pub fn get_hamiltonian(name: &str) -> Option<Arc<dyn HamiltonianHandle>>;

pub fn clear();
pub fn all() -> Vec<Arc<dyn HamiltonianHandle>>;
```

`HamiltonianHandle` is a thin read-only trait exposing the four sub-traits (force, drift, projection, energy decomposition) plus parameter introspection (`group() -> Group`, `params() -> &HashMap<String, f64>`, `name() -> &str`). Backing storage is `OnceLock<Mutex<HashMap<String, HamiltonianFactory>>>`, single-tenant per process. This matches the lattice and gauge registry lifecycle exactly so there is one mental model for "how the registries work" across the engine.

Built-in `KogutSusskind` registers itself in an engine startup hook (called from `Engine::new` before WAL replay begins). `aurora_crate::init()` registers `ShallowWater` from the host binary before `Engine::open`.

**WAL serialization (the engine's hard constraint):**

```rust
#[cfg(feature = "gauge")]
WalEntry::HamiltonianDeclare {
    name: String,
    kind_tag: String,                  // registry key, e.g. "SHALLOW_WATER"
    params: HashMap<String, f64>,
}
```

The trait-object is never serialized. Only metadata. On replay the executor looks up `kind_tag` in the registry, calls the factory with `params`, gets back a `Box<dyn HamiltonianHandle>`, and re-registers under `name`. This is the same pattern `gauge::registry` uses for `GaugeFieldHandle` (metadata-only WAL entry, re-materialize from factory at replay).

Hard ordering constraint enforced by the engine: every `register_hamiltonian()` call MUST complete before `Engine::open()` begins WAL replay. If a WAL entry references an unregistered `kind_tag` during replay, replay fails fast with a clear error of the shape `HamiltonianDeclare references kind 'SHALLOW_WATER' but no factory is registered — did you forget to enable the aurora feature?`. This is non-negotiable for the WAL contract; it's the same ordering rule that already holds for the lattice and gauge registries.

**Plugin-loading mechanism (named explicitly because it matters for deploy):**

Rust library + Cargo feature flag. NO dlopen, NO FFI, NO runtime plugin system. `aurora_crate` is a Rust crate; it depends on `gigi` as a library; the host binary calls `aurora_crate::init()` in `main()` before `Engine::open()`. `gigi` exposes an optional `[features] aurora = ["dep:aurora_crate"]` gate so the default `gigi` binary does not link AURORA's physics. For single-binary fly.io deploy: build with `--features aurora`; init hook fires at process start; registry is populated before WAL replay touches `HamiltonianDeclare` entries.

This is the same proven pattern `lattice::registry` already uses — same shape, same lifecycle, same single-binary deploy story. If AURORA prefers a separate `aurora-server` binary that depends on both `gigi` and `aurora_crate` directly (no feature flag on gigi's side), that is also fine; the registry doesn't care which binary calls `init()`. The choice between "gigi feature flag" and "separate host binary" is Q4 below.

**One engine concern named honestly:** trait-object dispatch in the integrator inner loop is unacceptable at C48 scale (13,824 cells, every step calls `force_per_edge` once per edge). The engine commits to monomorphized dispatch on the hot path. See §9 for the contract.

---

## 3. CC-2 detail — open registry for `LatticeTopology`, land inside A1 sprint

Accepted, and we are lifting CC-2 into the A1 sprint rather than retrofitting later. `CUBED_SPHERE` dispatches through the registry from day one.

**Choice and rationale:** shipping A1 with the static match arm in `parser.rs:8778-8787` and retrofitting CC-2 later costs a second parser refactor AND risks a WAL incompat if canonical-name string handling changes between v0.1 and v0.2. The LOC for CC-2 (~110–175 per the registry audit: `register_constructor`, `get_constructor`, parser-arm replacement, engine init hook for built-ins) is bounded; the retrofit cost is larger. Keeping CC-1 and CC-2 on the same factory-registry pattern means one mental model for the parser → registry → constructor pipeline across both surfaces.

**Surface we ship:**

```rust
// New on lattice::registry:
pub fn register_constructor(name: &'static str, factory: fn() -> Lattice);
pub fn get_constructor(name: &str) -> Option<fn() -> Lattice>;
```

Pre-registration of `TRUNCATED_ICOSAHEDRON -> buckyball` happens in the engine startup hook (so the existing buckyball path is unchanged from the caller's perspective). The parser arm at `parser.rs:8778-8787` becomes a registry lookup:

```rust
let constructor = lattice::registry::get_constructor(&canonical.to_ascii_uppercase())
    .ok_or_else(|| format!("Unknown canonical lattice constructor: '{canonical}'"))?;
let mut lat = constructor();
lat.name = name.clone();
// ... existing topology hint + durable declaration path unchanged
```

`aurora_crate::init()` calls `register_constructor("CUBED_SPHERE", cubed_sphere)` before `Engine::open()`. Same lifecycle as `register_hamiltonian`. No special-casing.

---

## 4. CC-3 acceptance — constructor-owned seam logic

Accepted as written. The `cubed_sphere` constructor owns the 12C cross-seam edges; they are enumerated by panel-coordinate arithmetic at construction time and appear to the `Lattice` struct as ordinary edges. No special seam tag on the `Lattice` field; no engine-side seam machinery; no seam-aware operators. This matches the buckyball precedent (rotation system and face tracing live in the constructor, not on `Lattice`). The constructor's docstring documents the seam convention; tests in the cubed-sphere module verify the 12C seam count and per-panel edge consistency.

If a future general-purpose `Lattice::seam_metadata` lift makes sense (driven by a second topology that wants the same metadata), we revisit. Not v0.1 scope.

---

## 5. CC-4 detail — signed-face promotion lands BEFORE A1

Accepted, and this is the strongest commitment in the letter. Bit-identity risk is zero; LOC is well within your 50–100 estimate; AURORA writes `cubed_sphere.rs` against the promoted surface day one.

**Promotion shape (the cleanest fit):**

```rust
impl Lattice {
    /// Compute per-face signed edge orientations from the outward-oriented
    /// face cycles. Returns Vec<Vec<(EdgeId, Sign)>> where Sign ∈ {+1, -1}
    /// is the orientation of the edge relative to the face cycle's
    /// traversal direction. Invariant: every edge appears in exactly two
    /// faces with opposite signs (Σ_F sign(e, F) = 0).
    pub fn signed_face_orientations(&self) -> Vec<Vec<(EdgeId, Sign)>>;
}
```

Method on `Lattice`, not a free function in `lattice::topology`. Three reasons: (a) it operates on `Lattice.edges` and `Lattice.faces` — natural `Lattice` method; (b) the return type mirrors the shape already stored in `Buckyball.signed_faces`; (c) it's topology-agnostic — any constructor whose face cycles are outward-oriented can call it.

**Bit-identity risk: zero.** The audit confirms the computation itself is unchanged; relocating it from Halcyon-private into `lattice::topology` does not alter bytes. The four bit-identity-locked tests (`tdd_hal_i_6_bit_identity_face_holonomy_gold`, `tdd_hal_ii_7_gold_walker_through_gauge_field`, `tdd_hal_ii_7_gauge_field_works_as_trait_object`, `signed_face_table_consistency`) continue to pass without modification because `buckyball_with_signed_faces()` becomes a thin wrapper calling the promoted method, preserving the Halcyon-internal API surface for backward compat.

**LOC budget:** 60–80 LOC for the promotion (extracted emit closure + sign-detection logic) + ~20 LOC for unit tests on the general method. Total promotion PR ~80–100 LOC. Within your estimate.

**In-flight conflict check:** none. The audit confirms Halcyon Part V (commits `5b555ce..1165698`) touches dispatch / WAL / snapshot machinery; none of those files touch `src/lattice/topology/truncated_icosahedron.rs` or any surface that reads from `buckyball_with_signed_faces()`. CC-4 promotion lands in parallel with or after Part V with zero merge conflicts.

**Your offered fallback (AURORA carries duplicate orientation logic) is declined.** Duplication is unnecessary; the promotion is small; the bit-identity risk is zero. We are doing it.

---

## 6. CC-5 + CC-6 acceptance

**CC-5 — schema DSL extension as Phase 3 general lift:** accepted. `__` (double underscore) is the desugar convention for inline struct flattening when the DSL extension lands. Engine commits to NOT shipping single-underscore flattening that could collide with field names containing underscores (e.g. `kelvin_holonomies_eq` is a deliberate flat name in v0.1, not a flattened `kelvin_holonomies.eq`; if the DSL lift later introduces `kelvin_holonomies__eq` as the long-form, the v0.1 flat names stay as legacy aliases for the six existing scalars). Documenting `__` now means existing bundles don't need naming rules retrofitted.

**CC-6 — `RECEIPT_GATE` as query-side discipline for v0.1:** accepted. `COVER WHERE refusal_reason IS NULL` is the contract. Engine does not silently strip refusals; the caller is responsible for the filter. Engine documents this as mandatory caller hygiene in the v0.1 spec. Phase 3 may revisit if a `SAFE_COVER` sugar makes sense; v0.1 keeps the explicit filter so refusals are visible by default.

---

## 7. A1 design pins (`CUBED_SPHERE`)

**Panel-size parameter:** accepted as parameterized and required. `CREATE LATTICE atmos FROM CUBED_SPHERE PANEL_SIZE 6 TOPOLOGY 'S2/CUBED_SPHERE'`. v0.1 default scaffold uses `PANEL_SIZE 6` (216 cells); production runs use `PANEL_SIZE 48` (13,824 cells). No parameter-free form. WAL replay is deterministic on `PANEL_SIZE`.

**Face orientation:** the constructor calls the promoted `Lattice::signed_face_orientations()` (CC-4 lands first). The constructor's docstring states the outward-normal convention explicitly so the panel-coordinate seam math is documentable independent of the method's implementation. The CC-4-doesn't-land-first fallback in your letter is moot: CC-4 lands first.

**topology_hint vocabulary:** we are shipping a documented const table at `src/lattice/topology/hints.rs` (new, ~30 LOC). Initial entries: `TOPOLOGY_S2_CUBED_SPHERE = "S2/CUBED_SPHERE"`, `TOPOLOGY_S2_TRUNCATED_ICOSAHEDRON = "S2/TRUNCATED_ICOSAHEDRON"`. Format: `"<manifold-class>/<discretization>"`. The free-form `Option<String>` on `Lattice` stays — the const table is the *recommended vocabulary*, not a closed enum, so downstream crates can introduce e.g. `"T3/CUBIC_LATTICE"` without patching gigi. AURORA contributes the `S2/*` entries as part of the A1 PR (no separate handoff).

**Seam handling:** accepted constructor-owned per CC-3. The 12C cross-seam edges live in `cubed_sphere.rs` and are enumerated by panel-coordinate arithmetic at construction time. `Lattice` sees them as ordinary edges.

---

## 8. A2 design pins (`ShallowWater`)

**Four separate traits:** accepted. `HamiltonianForce`, `HamiltonianDrift`, `ProjectionOperator`, `EnergyDecomposition`. Composition wins because (a) not every Hamiltonian has a constraint projection (free SU(2) doesn't, ShallowWater does), (b) the integrator hot path only touches Force + Drift — keeping `EnergyDecomposition` and `ProjectionOperator` off the inner loop avoids forced vtable lookups for measurements. The four traits compose into a `HamiltonianKind` enum-dispatch tag at the parser/WAL boundary; integrator code generic over the trait stays monomorphic where the concrete type is known.

**Generic `State` associated type on `HamiltonianDrift`:** accepted. SU(2) link variables and real-valued shallow-water `(h, u)` fields have fundamentally different shapes — forcing them into a unified per-edge real vector would require lossy projection on the SU(2) side. The downside (no uniform `Box<dyn HamiltonianDrift>`) is fine because integrator dispatch goes through the `HamiltonianKind` enum, not a trait-object slot. Trait-objects are used only for read-only inspection surfaces (energy decomposition, parameter introspection) where call frequency is O(declarations), not O(steps × edges).

**`project_constraint` + `PROJECT_GAUSS` sugar alias:** accepted. The trait method is `project_constraint`; the GQL keyword `PROJECT_GAUSS` desugars to `project_constraint` on the registered Hamiltonian. The executor checks that the Hamiltonian implements `ProjectionOperator` and errors cleanly if not. New domains add their own sugar (e.g. `PROJECT_DIVERGENCE_FREE` for shallow water) as parser-level aliases routing to the same `project_constraint` method.

**Free-form `HashMap<&'static str, f64>` for energy decomposition:** accepted. Honest asymmetry beats forced `(kinetic, potential)`. Yang-Mills publishes `{"plaquette", "gauss_residual"}`; ShallowWater publishes `{"casimir_energy", "casimir_mass", "casimir_pv_l1", "casimir_pv_l2", "kelvin_eq", "kelvin_n30", "kelvin_s30"}` (or similar). Trade-off: typo'd key names fail silently at query time. Mitigated by per-Hamiltonian unit tests asserting the published key set against a documented vocabulary.

**Group-agnostic `KogutSusskind { beta }`:** accepted. Executor performs `assert U_handle.group() == SU2` (or whichever group the action is instantiated against) at declaration time. The name reflects the action's mathematical content (lattice plaquette action of Kogut & Susskind 1975), not the group; SU(3) lattice QCD uses the same action with a different group. No `KogutSusskindSU2 / KogutSusskindSU3` explosion.

**Engine performance constraint we are naming explicitly:** trait-object dispatch is OFF the integrator inner loop. The integrator is generic over the concrete Hamiltonian type — monomorphized per `HamiltonianKind` variant via enum match at the outer loop, so `force_per_edge` and `drift_step` are direct calls, not vtable lookups. `Box<dyn ...>` only appears at the registry-storage / WAL-replay / introspection boundary, where call frequency is O(declarations) not O(steps × edges). At buckyball scale (90 edges) vtable dispatch would be invisible; at C48 scale (13,824 cells × N steps) the monomorphized path is what we commit to. AURORA: please write the inner loop generic over `<H: HamiltonianForce + HamiltonianDrift>`, not `Box<dyn ...>`.

---

## 9. A3 acceptance

Six flat scalar names land verbatim as the v0.1 contract:

| Field | Name | Type |
|---|---|---|
| Kelvin holonomy at equator | `kelvin_holonomies_eq` | NUMERIC |
| Kelvin holonomy at 30°N | `kelvin_holonomies_n30` | NUMERIC |
| Kelvin holonomy at 30°S | `kelvin_holonomies_s30` | NUMERIC |
| c-field min | `c_field_min` | NUMERIC |
| c-field max | `c_field_max` | NUMERIC |
| c-field mean | `c_field_mean` | NUMERIC |

Query-side gate `COVER ... WHERE refusal_reason IS NULL` is the v0.1 contract per CC-6. Engine documents this as mandatory caller hygiene.

`__` desugar convention reserved for Phase 3. When the DSL extension lands, `kelvin_holonomies__eq` is the long-form name and the flat `kelvin_holonomies_eq` is retained as a legacy alias for the six v0.1 scalars. No silent renames.

---

## 10. Q3 answer — discrete operator surface

**Verdict:** option (c). DEC operators live in a NEW `src/lattice/dec/` module consuming a `LatticeWithMetric` wrapper. NOT methods on `Lattice` itself. Same operator vocabulary you asked for (`d₀`, `δ₀`, Hodge star); different carrier.

**Current state of the codebase (the existing DEC surface):**

- `src/discrete/hodge_complex.rs:1-9` already exposes `d_0` (gradient, `|E|×|V|`) and `d_1` (curl, `|F|×|E|`) as dense `DMatrix`. `HodgeComplex::new()` builds them from cell incidence data (line 143–171). This is the BUNDLE-side DEC — it operates on field-graph complexes over records, not on lattice-attached metric data. The matrices are abstract (no edge lengths, no cell areas).
- `src/discrete/hodge_laplacian.rs:122-139` ships the Hodge Laplacian `Δ_k = d†d + dd†` but does not expose it publicly.
- `src/gauge/holonomy.rs:27-37` exposes `walk_loop(lattice, edges, conn)` — generic over group via `EdgeConnection`, but NOT generic over metrics. It composes group products around loops; it does not integrate differential forms. Not a DEC primitive.
- `src/gauge/project_gauss.rs:23-46` computes covariant divergence with SU(2) adjoint action — Lie-algebra-valued, gauge-covariant, domain-specific to Yang-Mills constraint projection. Reduces to metric-free divergence only in the trivial-connection limit. NOT the topological `d₀†` from DEC; not metric-aware.
- `Lattice` struct (`src/lattice/mod.rs:76-91`) stores only topology (`n_vertices`, `edges`, `faces`, `topology_hint`). NO metric data. The buckyball constructor builds vertex coordinates for edge detection (line 49–100) but discards them. No edge lengths, no vertex areas, no dual face areas accessible to `Lattice` consumers.

**The gap:** ShallowWater needs metric-weighted DEC. The existing `hodge_complex` is metric-free; the existing `walk_loop` is group-product, not form-integration; the existing `project_gauss` is gauge-covariant, not metric-divergence. None of the three is what `ShallowWater` calls.

**Why DEC operators do NOT land on `Lattice` directly:**

1. Different topologies need different metric discretizations. Cubed-sphere wants panel-aware Hodge with explicit face metric tensors; truncated icosahedron wants Voronoi-aware dual. The metric data must flow from the topology constructor into the operator. `Lattice` itself is metric-agnostic and should stay that way — that is the post-`ea50585` general-purpose discipline.
2. Halcyon's buckyball is metric-FREE (Yang-Mills doesn't need an embedded metric for the Wilson action — the plaquette action is metric-independent at the discrete level). Adding metric methods to `Lattice` would force Halcyon to carry metric data it doesn't use, or to error on `lattice.d_0(...)` calls in contexts where it's nonsense.
3. Divergence in ShallowWater is metric-weighted (mass flux form: `∇·(hu)`), not topological `d₀†`. The metric-weighted divergence cannot be built from `Lattice` alone — it needs cell areas and dual edge lengths.
4. Separation of concerns matches `ea50585`: `Lattice` topology + `walk_loop` + `hodge_complex` (on bundles) are domain-agnostic primitives; `project_gauss` (for Yang-Mills) lives in `src/gauge/`. ShallowWater's force/drift/projection should follow the same pattern.

**Where DEC lives instead:**

- `src/lattice/metric.rs` (new) — defines `EdgeMetric`, `FaceMetric`, `CellArea` traits and a `LatticeWithMetric { lattice: Lattice, metric: M }` wrapper struct. Lattice itself stays metric-agnostic; the wrapper carries the metric. Concrete impl `CubedSphereMetric` ships with `edge_length()`, `cell_area()`, `dual_area()`.
- `src/lattice/dec/mod.rs` (new) — defines `d_0(lat_metric, scalar_field) -> Vec<f64>` (gradient, cells → edges, metric-weighted), `delta_0(lat_metric, edge_field) -> Vec<f64>` (divergence, edges → cells, adjoint), `hodge_star_0/1/2` (metric ratios). Free functions, not methods. The module is general-purpose: any future topology that vends a metric (cubed-sphere, regular cubic lattice, triangulated manifolds, future tori) calls the same DEC operators.
- `src/lattice/topology/cubed_sphere.rs` — constructor returns `LatticeWithMetric`, vending `CubedSphereMetric` with the panel-aware metric data. The `CUBED_SPHERE` registry entry knows it returns a `LatticeWithMetric` rather than a bare `Lattice` (registry signature accommodates both via an enum or a trait — exact shape TBD as a sub-question of CC-2).

**LOC budget:** ~250–400 LOC total — `metric.rs` ~80–120, `dec/mod.rs` ~100–150, `dec/tests.rs` ~50–80 (round-trip `d_0 . delta_0` consistency, finite-difference convergence on a known scalar field), cubed-sphere metric vending ~20–50 additional. Lands as one or two PRs before A2 sprint starts. Bounded by the existing `hodge_complex.rs` algebraic skeleton.

**What AURORA's `ShallowWater::HamiltonianForce` actually calls:**

```rust
let grad_h  = lattice::dec::d_0(&lat_metric, &h_field);         // cells -> edges
let div_hu  = lattice::dec::delta_0(&lat_metric, &mass_flux);   // edges -> cells
let star_1  = lattice::dec::hodge_star_1(&lat_metric);          // metric ratio
```

AURORA does NOT call `lattice.d_0(...)` — that method does not exist. AURORA constructs `CubedSphereMetric` (via the topology constructor that vends it), wraps as `LatticeWithMetric`, and calls free functions in `lattice::dec`. The Kelvin holonomies use the promoted `Lattice::signed_face_orientations()` plus `EdgeMetric::edge_length()` from the wrapper. Pattern scales to future topologies + physics without touching `Lattice`.

**Bee's general-purpose framing held:** DEC operators, if exposed, are peer to `walk_loop` / `PLAQUETTE` — domain-agnostic, not "the AURORA operator surface". The new `src/lattice/dec/` is the lattice-side metric-aware peer to the existing bundle-side `src/discrete/hodge_complex.rs`. Both are general; both are available to any Gi-System that wants discrete calculus on its substrate.

---

## 11. Cross-cutting engine concerns AURORA didn't raise

These are questions back at you, marked Q4 onward. None block A1; they all block the A2 + CC-1 formalization.

### Q4 — aurora_crate repository location

The engine assumes `aurora_crate` lives in a SEPARATE git repo (e.g. `~/Documents/aurora`), not under `theory/aurora/` in gigi. Rationale: (a) gigi stays the substrate; AURORA is a peer Gi-System consumer, not internal code; (b) gigi's CI/test surface should not absorb AURORA's domain-specific tests; (c) license boundary is clean. Coordination protocol: `theory/aurora/` in gigi remains the LETTERS path (specs, replies, design docs); IMPLEMENTATION lives in aurora_crate's own tree and depends on gigi as a Cargo dependency (path or git ref).

Two sub-questions:

- (Q4a) Confirm aurora_crate's repo location and visibility. Public Cargo dep? Private path dep? Git ref pinned to a gigi commit?
- (Q4b) Does AURORA prefer gigi to expose an optional `[features] aurora = ["dep:aurora_crate"]` flag in gigi's own `Cargo.toml` (single-binary deploy via `--features aurora`), OR does AURORA ship its own host binary `aurora-server` that depends on both `gigi` and `aurora_crate` directly? Both are workable; choice affects fly.io deploy topology and gigi's `Cargo.toml` surface.

### Q5 — init-hook lifecycle and WAL replay ordering

The WAL replay ordering constraint is hard: every `register_hamiltonian()` and `register_constructor()` call MUST complete before `Engine::open()` begins replay. Two sub-questions:

- (Q5a) Where does `aurora_crate::init()` live in the host-binary `main()`? Engine recommends `main() { aurora_crate::init(); let engine = Engine::open(...)?; ... }`. Lazy registration (first call to `get_hamiltonian`) is NOT acceptable because WAL replay would trigger lookups before lazy init fires.
- (Q5b) Does aurora_crate register its types eagerly (all factories registered at `init()` time, regardless of which the host binary uses) or lazily (only register what the host explicitly opts into)? Engine recommends eager — simpler lifecycle, faster fail on misconfigured deployments.

### Q6 — binary-stability contract for the trait-object surface

`HamiltonianHandle` (and the four sub-traits) is a public trait surface that aurora_crate compiles against. If gigi changes the trait surface, aurora_crate must recompile. Two sub-questions:

- (Q6a) What's the binary-compat contract between gigi and aurora_crate? Pinned versions (aurora_crate locks `gigi = "=x.y.z"`) or floating (aurora_crate accepts any `gigi = "x.*"`)? Engine recommends pinned for v0.1; relaxed once the trait surface stabilizes.
- (Q6b) Does the trait surface get a stability annotation in gigi (e.g. `#[stable(since = "0.1")]` or a documented MSRV-style contract)? Engine has no such convention today; v0.1 trait surface should be marked "evolving" with a documented change-cadence rule (e.g. breaking changes only on minor versions).

---

## 12. Recommended next steps

Honoring no-timeframes: shape, dependency, blocker only.

**Phase 0 (engine, standalone, lands first):**
- CC-4 promotion: extract signed-face emission into `Lattice::signed_face_orientations()`; refactor `buckyball_with_signed_faces()` as a thin wrapper. Bit-identity tests stay green by construction. Standalone PR, no Halcyon coordination needed. Unblocks A1 face-orientation pin.

**Phase 1 (joint engine + AURORA, can land in parallel after Phase 0):**
- A1 `CUBED_SPHERE` constructor (AURORA writes `src/lattice/topology/cubed_sphere.rs`; engine reviews).
- CC-2 registry-dispatch refactor on the parser arm (engine; ~110–175 LOC).
- A3-workaround schema authoring (AURORA-side, zero engine LOC).
- `src/lattice/topology/hints.rs` const-table for reserved `topology_hint` vocabulary (engine, ~30 LOC, AURORA contributes the `S2/*` entries).

Phase 1 ships when A1 + CC-2 + the hints table land. After Phase 1, AURORA can run Williamson Test 2 on `CUBED_SPHERE` with the existing forward-Euler integrator — same refusal-at-step-2 expected; the validation is that `CUBED_SPHERE` registers and dispatches correctly, not that the physics is right.

**Phase 2 (joint, gated on Q4 + Q5 + Q6 answers):**
- CC-1 open-registry formalization: `src/gauge/hamiltonian_registry.rs` + WAL `HamiltonianDeclare` entry (engine).
- A2 four-trait refactor: extract `HamiltonianForce` / `HamiltonianDrift` / `ProjectionOperator` / `EnergyDecomposition` into `src/gauge/action.rs`; `KogutSusskind { beta }` becomes the first impl (no behavior change for Halcyon — bit-identity preserved) (engine, ~600–900 LOC).
- `ShallowWater { g, omega, a }` factory in `aurora_crate::hamiltonians` (AURORA).
- `src/lattice/metric.rs` + `src/lattice/dec/` module (engine, ~250–400 LOC) — this is the Q3 commitment; lands before AURORA's `HamiltonianForce` impl so the operators are available.

Phase 2 ships when the trait refactor is byte-identical for Halcyon, the registry round-trips `ShallowWater` through WAL replay, and `lattice::dec::{d_0, delta_0, hodge_star_*}` pass round-trip and finite-difference convergence tests.

**Phase 3 (general engine lift, decouples from AURORA timeline):**
- Schema DSL extension: fixed-length `[TYPE; N]` arrays + inline anonymous structs + `__` desugar convention.
- `AURORA_RECEIPT` schema gets a long-form variant; six v0.1 flat names retained as legacy aliases.
- A3 SMOKE GATE: AURORA runs the full Williamson Test 2 against the refactored Hamiltonian + `lattice::dec` operators on `CUBED_SPHERE` C48 with `casimir_pv_l2 < 1e-6` over 10 days, and Kelvin holonomies match the analytical values at the three latitudes.

**Phase 4 (deferred, no commitment in this letter):**
- `Lattice::seam_metadata` general lift (CC-3 deferred from v0.1).
- Schema-level `RECEIPT_GATE` CHECK (CC-6 schema-side version).

**What's actionable right now (no engine handoff blocking):**
- Q4, Q5, Q6 answers from AURORA. These unblock Phase 2 design freeze.
- AURORA writes the v0.1 `AURORA_RECEIPT` schema with the six flat names (zero engine LOC).
- AURORA scopes the `aurora_crate::hamiltonians::ShallowWater` factory shape against the §2 + §8 trait sketch so the Phase 2 refactor lands with a known consumer.

---

—Bee + Claude
