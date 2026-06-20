# AURORA → GIGI reply, 2026-06-19

**Re:** Engine-owner response to GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md
**Status:** Decisions on CC-1 through CC-6 + per-ask design pins + one new question
**Author:** Bee Rosa Davis, with Claude (Anthropic)

---

## Letter

Gigi —

The "peer primitive" framing holds on our side too. We won't describe `CUBED_SPHERE` as "the AURORA topology" or `ShallowWater` as "the AURORA Hamiltonian" — both are general-purpose engine surfaces that AURORA happens to consume first. If we catch ourselves drifting toward domain-specific framing in a future letter, the pushback is correct.

P-1 confirmed: `williamson_test2_scaffold.py` now hits `/v1/gql` cleanly. Step 0 receipt emitted, `refusal_reason = None`. Mass and energy diagnostics match the analytical Williamson Test 2 values within rounding. This is the first AURORA receipt. Forward Euler refuses at step 2 (energy drift 4.7× machine_eps at dt=60s), which is expected and demonstrates the integrator gap A2 closes.

Two decisions up front because they gate everything downstream: CC-1 is open registry. CC-4 is promote signed faces now, before A1 lands.

Below: cross-cutting decisions (§1, ordered by gating precedence), A1 design pins (§2), A2 design pins (§3), A3 acceptance (§4), one new question on the discrete operator surface (§5).

—Bee + Claude

---

## 1. Cross-cutting decisions (CC-1 through CC-6)

### CC-1 — `HamiltonianKind`: AURORA votes open trait-object registry

The `lattice::registry` path (constructor keyed by canonical string identifier, dispatched at runtime) is the right pattern for Hamiltonians too. Three reasons:

1. **Matches the direction `ea50585` set for topologies.** The lattice surface already went open-registry. `HamiltonianKind` going closed-enum creates an asymmetry: downstream crates can register new topologies without patching gigi, but they cannot register new Hamiltonians. That asymmetry is wrong for a system where new Gi-Systems bring their own physics.

2. **AURORA volunteers to be the first downstream crate.** If the open registry lands correctly, `ShallowWater { g, omega, a }` ships in `aurora_crate::hamiltonians` and registers through `register_hamiltonian("SHALLOW_WATER", constructor)` without a gigi patch. That is the validation event for the registry — same as `CUBED_SPHERE` being the validation event for the topology registry. If something in the open-registry shape is wrong, AURORA's `ShallowWater` impl will surface it before a third Gi-System hits it.

3. **Future physics (Phaethon plasma, ICARUS)** should not require gigi PRs. The closed-enum approach accumulates technical debt at the rate of one enum patch per Gi-System, which is manageable at two systems and unmanageable at ten.

Accept risk: the A2 refactor scope grows slightly (the trait-object dispatch machinery is slightly more than a match arm). This is a one-time cost.

### CC-2 — `LatticeTopology` dispatch: AURORA votes open registry (but A1 can proceed today)

Consistent with CC-1. The parser `CREATE LATTICE FROM <ident>` arm should ultimately dispatch through `lattice::registry.get(ident)` so downstream crates can register topologies without touching `parser.rs`. `CUBED_SPHERE` should be the first topology that demonstrates this.

However: A1 does not need to wait for CC-2 to be fully resolved. If the engine team wants to land A1 as a static match arm now and retrofit it when CC-2 is formalized, AURORA accepts that sequencing. The constructor itself is independent of the dispatch mechanism. The retrofit is ~3 LOC at the parser arm site.

If CC-2 and A1 are co-scoped, AURORA's preferred outcome is that the `CUBED_SPHERE` arm dispatches through the registry from day one — which means the registry needs `register(ident, constructor_fn)` and `get(ident) -> Option<Lattice>` before A1's parser arm lands. That's the cleaner shape; whether the sprint budget supports it is your call.

### CC-3 — `Lattice::seam_metadata`: AURORA accepts constructor-owned seam logic for now

Per Q2: `CUBED_SPHERE` ships its own panel-adjacency resolver and emits the cross-seam edges directly into `Lattice.edges`. AURORA accepts owning that responsibility and will document the orientation convention as part of the constructor's public contract.

If CC-3 lands in Phase 4, AURORA would benefit retroactively — the seam metadata would allow `ShallowWater`'s flux computations to distinguish intra-panel and cross-seam edges without parsing the topology_hint string. We are not gating A1 on CC-3.

### CC-4 — signed-face surface promotion: AURORA asks this land before or with A1

This is the one Phase 4 item AURORA is asking be treated as Phase 1. The reason is concrete: `ShallowWater` needs signed faces for Kelvin holonomies. The discrete form is

```
kelvin(loop L) = Σ_{e ∈ L} sign(e, L) × u_e × len(e)
```

where `sign(e, L) ∈ {+1, -1}` is the edge-orientation relative to the loop's traversal direction. Without signed faces in the `Lattice` surface, `CUBED_SPHERE` would carry its own orientation logic in `cubed_sphere.rs`, mirroring what `buckyball_with_signed_faces()` does in the Halcyon-internal path. That's the duplication the promotion is meant to avoid.

AURORA's ask: promote signed faces to a method on `Lattice` (or on the topology module, whichever shape the engine team prefers) so that `CUBED_SPHERE` calls the same surface that `TRUNCATED_ICOSAHEDRON` uses. This promotion is ~50–100 LOC (move the computation out of the Halcyon-private path into `lattice::topology::signed_faces(lattice)` or `Lattice::signed_face_orientations()`). If it's too disruptive to Halcyon's bit-identity tests to do now, AURORA will carry its own orientation logic and accept the duplication; just say so.

### CC-5 — schema DSL extension: agree, sequence as general-purpose lift

`AURORA_RECEIPT` ships on flattened scalars. The array and inline struct DSL extension benefits every bundle author and should be planned on its own merits. AURORA will not refer to it as an AURORA feature in future letters.

One downstream effect: if the desugar-to-synthetic-columns approach is chosen for inline struct fields (§4 below), AURORA prefers consistent column naming across bundles — document the desugar convention (`field__subfield`) so bundle authors don't invent competing conventions.

### CC-6 — `RECEIPT_GATE` as schema CHECK: accept query-side discipline for v0.1

`COVER WHERE refusal_reason IS NULL` is the gate for v0.1. AURORA will document this as a mandatory read discipline on the bundle (not a silent convention). If schema-level CHECK lands in a future sprint, `AURORA_RECEIPT` would be the first bundle to use it — but we are not gating v0.1 on it.

---

## 2. A1 design pins (`CUBED_SPHERE` topology)

**Panel-resolution parameter:** Parameterized. `CREATE LATTICE atmos FROM CUBED_SPHERE PANEL_SIZE 6 TOPOLOGY 'S2'`. AURORA's v0.1 scaffold uses `PANEL_SIZE 6` (= 216 cells, C6). Production will need at least `PANEL_SIZE 48` (= 13,824 cells, C48). The parameter-free form should not exist as a special case; every `CUBED_SPHERE` call takes an explicit `PANEL_SIZE`. Agree with the recommendation to let future `TRUNCATED_ICOSAHEDRON LEVEL <n>` follow the same parameterized pattern if geodesic refinement lands.

**Face orientation convention:** Two paths depending on CC-4.
- If CC-4 (signed-face promotion) lands first: `CUBED_SPHERE` calls the promoted `Lattice::signed_face_orientations()` and exports no private orientation logic. Edge orientation = canonical directed graph (lower vertex-index → higher vertex-index within each panel; cross-seam edges oriented according to the receiving panel's local index ordering). The signed-face promotion provides the `sign(e, face)` map consistently.
- If CC-4 doesn't land with A1: AURORA declares the convention explicitly in `cubed_sphere.rs` docstring: face normal = outward on S² (right-hand rule from interior), edge orientation = counterclockwise around face viewed from outside. This is the same convention `buckyball_with_signed_faces()` uses internally, so downstream code (holonomy walkers, divergence operators) sees consistent sign conventions across topologies.

**topology_hint string:** AURORA proposes reserving `"S2/CUBED_SPHERE"` and `"S2/TRUNCATED_ICOSAHEDRON"`. If the engine team wants to enumerate the reserved vocabulary somewhere (an enum or a const table), AURORA will contribute the `S2/*` entries. If topology_hint stays free-form for now, the reservation is a documented convention in the constructors' docstrings.

**Seam handling:** AURORA owns it in `cubed_sphere.rs`. The six 90-degree panel seams produce 4C cross-seam edges (C edges per seam × 4 seams between non-antipodal panels; the remaining 2 antipodal-panel pairs each contribute C edges too, total = 12C cross-seam edges for an edge-regular cubed sphere). These will be enumerated by panel-coordinate arithmetic and added to `Lattice.edges` by the constructor. The constructor's docstring will state the enumeration order and the cross-seam index convention.

---

## 3. A2 design pins (`ShallowWater` Hamiltonian)

**Trait shape: four separate traits.** `HamiltonianForce`, `HamiltonianDrift`, `ProjectionOperator`, `EnergyDecomposition` as distinct traits. The composability argument is real: `ShallowWater`'s projection is a weighted Poisson solver that has nothing in common with `KogutSusskind`'s covariant-Laplacian CG, so coupling them in one impl block would group structurally-unrelated code by the wrong axis. Four traits let a future non-Wilson SU(2) action reuse `HamiltonianDrift` (same Rodrigues exponential) while supplying its own force, without implementing a no-op `project_constraint`.

One constraint: the four traits must compose through a blanket impl or through the `HamiltonianKind` enum-dispatch so the integrator loop's call site reads as one unit. The four separate methods should not force the caller to hold four trait objects.

**Drift signature: generic `State` associated type.** There is no shared per-edge-real-vector type that covers both `Vec<GroupElement>` (KogutSusskind) and `(Vec<f64>, Vec<f64>)` (ShallowWater velocity + depth fields). The honest encoding is `type State: Clone + Send` on `HamiltonianKernel` (or on `HamiltonianDrift`), with each impl declaring its own state type. The integrator loop then dispatches through the enum's associated type rather than a concrete state struct.

One downstream effect: `EMIT RECEIPT` must serialize whatever the `State` associated type contains. Each Hamiltonian impl publishes its serialization through `EnergyDecomposition`.

**Projection contract:** `project_constraint` as the trait method name. Keep `PROJECT_GAUSS` as a GQL sugar alias that routes through `project_constraint` — it's in existing letters and should not break at the GQL layer. If `SHALLOW_WATER` projection has a well-understood name (weighted Helmholtz decomposition, or Chorin's method), surface it as a descriptive comment in the impl rather than a second parser keyword. AURORA does not need a `PROJECT_MASS` keyword; `PROJECT_GAUSS { ... }` routing through the trait is fine.

**Energy decomposition: free-form labeled map.** `HashMap<&'static str, f64>` (or a thin newtype over it). AURORA's observables are not K+V — they are `casimir_energy`, `casimir_mass`, `casimir_pv_l2`, and the Kelvin holonomies (three labeled scalars). A fixed `(kinetic, potential)` tuple forces AURORA to lie: geopotential is partly "kinetic" (it drives the wave phase speed) and partly "potential" (it's a pressure term). The free-form map lets each Hamiltonian publish exactly what it measures; downstream receipt code reads by key. The only constraint: both Hamiltonians should document their key vocabulary so `AURORA_PART_I_IMPLEMENTATION_LOG.md` can reference them.

**Naming: group-agnostic `KogutSusskind { beta }`** with executor compatibility check (`assert U_handle.group() == SU2`). Future SU(3) lattice QCD is a new `Group` field under the same `KogutSusskind` action, not a new top-level `HamiltonianKind`. The Wilson action is defined for any compact group; the group is a parameter, not the identity of the Hamiltonian. Agree with the recommendation.

---

## 4. A3 acceptance (`AURORA_RECEIPT` flattened scalars)

Accept the flattened-scalars workaround. The six scalar names for the array + struct fields:

| AURORA spec field | Flattened scalar name | Type |
|---|---|---|
| `kelvin_holonomies[0]` (equator) | `kelvin_holonomies_eq` | NUMERIC |
| `kelvin_holonomies[1]` (lat30N) | `kelvin_holonomies_n30` | NUMERIC |
| `kelvin_holonomies[2]` (lat30S) | `kelvin_holonomies_s30` | NUMERIC |
| `c_field_summary.min` | `c_field_min` | NUMERIC |
| `c_field_summary.max` | `c_field_max` | NUMERIC |
| `c_field_summary.mean` | `c_field_mean` | NUMERIC |

Full schema as AURORA will author it:

```
CREATE BUNDLE AURORA_RECEIPT
  BASE (
    run_id           TEXT REQUIRED,
    step             INTEGER,
    wall_time        NUMERIC,
    casimir_energy   NUMERIC,
    casimir_mass     NUMERIC,
    casimir_pv_l1    NUMERIC,
    casimir_pv_l2    NUMERIC,
    s_d2_residual    NUMERIC,
    refusal_reason   TEXT
  )
  FIBER (
    kelvin_holonomies_eq    NUMERIC,
    kelvin_holonomies_n30   NUMERIC,
    kelvin_holonomies_s30   NUMERIC,
    c_field_min             NUMERIC,
    c_field_max             NUMERIC,
    c_field_mean            NUMERIC
  );
```

Receipt gate: `COVER ... WHERE refusal_reason IS NULL` per CC-6 decision. This will be documented as a mandatory read discipline on the bundle.

On `Vector { dims: 3 }` vs three scalars for Kelvin holonomies: AURORA prefers three labeled scalars (`_eq`, `_n30`, `_s30`). The labels carry physical meaning that a generic 3-vector index loses; `COVER WHERE kelvin_holonomies_eq > 1e10` is more readable than `knn(holonomies, [1e10, 0, 0], k=1)`.

On desugar convention for the future DSL extension (CC-5): if inline struct `{ min, max, mean }` desugars to synthetic columns, AURORA proposes `__` (double underscore) as the separator — `c_field_summary__min` — to avoid collision with single-underscore names that appear in the scalar vocabulary. Documenting this convention now means existing bundles don't need naming rules retrofitted.

---

## 5. New question — Q3: discrete operator surface for `ShallowWater` force

The `ShallowWater` force law requires three discrete operators on the `Lattice`:

1. `grad h`: gradient of geopotential, a 0-form → 1-form map (value on cells → value on edges). In DEC notation: `d₀`.
2. `div(hu)`: divergence of mass flux, a 1-form → −1-form map (value on edges → value on cells). In DEC notation: `−δ₀` (adjoint of `d₀`, scaled by primal/dual area ratios).
3. Coriolis: `f k × u`, a pointwise rotation on the edge-tangent velocity field. No DEC operator needed; this is cell-local.

The question: does the general-purpose `Lattice` surface expose `d₀`, `δ₀`, and the Hodge star (primal cell areas, edge lengths, dual face areas)?

- **If yes:** `ShallowWater`'s `HamiltonianForce` impl calls `lattice.d0(h)` and `lattice.delta0(hu)` directly. The Kelvin holonomies and divergence-free projector work at machine precision without knowing the panel layout.
- **If no:** `ShallowWater` ships finite-difference stencils over the raw `Lattice.edges` list, and both the force kernel and the weighted-Poisson projector are custom code that duplicates DEC logic GIGI may already have elsewhere.

The `walk_loop` / `PLAQUETTE` surface suggests some loop-integration machinery already exists. AURORA is asking whether `d₀` / `δ₀` / Hodge star are the right abstraction to expose, or whether the engine team has a different operator model in mind. This does not block A1 or the A2 trait refactor; it does affect what `ShallowWater`'s `HamiltonianForce` and `ProjectionOperator` impls look like, so AURORA wants to know before writing those kernels.
