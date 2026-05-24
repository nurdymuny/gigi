# GIGI Kähler Upgrade — Implementation Plan

*Companion to `catalog.md`. Layered by dependency. No timelines —
bee manages those. This document defines TDD specs, math-validation
gates, and e2e-validation gates per layer, plus the ordering
constraints between layers.*


## 0. Working agreements

### Test discipline (carried over from the catalog)

1. **Red-first.** Every Rust module ships its `#[cfg(test)]` block
   in the SAME commit that introduces the module. Tests fail before
   implementation; pass after. No "I'll write tests later" merges.
2. **Independent ground truth.** Numerical assertions compare against
   a closed-form or symbolic value derived in a different formalism
   than the code under test. If you can't think of one, you don't
   understand the claim well enough to write the code.
3. **Negative cases required.** Every test that asserts a property
   must include a configuration where the property must fail.
   "Passes by returning zero" is the failure mode we're guarding
   against.
4. **Math validation never regresses.** The 11 existing tests in
   `theory/kahler_upgrade/validation/` must continue to pass on
   every merge. CI runs them. New tests (12, 13, 14, …) are added
   as the corresponding layer lands.
5. **e2e never regresses.** GIGI's existing `cargo test` suite must
   keep passing. New Kähler types are *optional* fields on existing
   structs — a bundle without `kahler` attached must behave
   identically to before this upgrade.

### Layering rules

- A layer's tests + math validation + e2e validation must all be
  green before the next layer starts.
- Within a layer, items can land in any order or in parallel.
- Cross-layer dependencies are declared explicitly under each layer
  ("depends on L1"). No hidden coupling.

### Definition of done (per layer)

- All TDD tests pass in `cargo test`.
- Corresponding Python math-validation test passes:
  `PYTHONIOENCODING=utf-8 python -X utf8 theory/kahler_upgrade/validation/<file>.py`.
- `cargo test --release` (full GIGI suite) passes with no regressions.
- **Real-data smoke test passes** (bee's rule, added 2026-05-24).
  Each layer ships a `tests/kahler_<layer>_real_data_smoke.rs` that
  loads actual records from `test_data/` (sensor_data.json,
  employees.csv, etc.), exercises the new feature, and asserts the
  behavior matches the catalog claim under realistic load — not
  just synthetic 2-record minimal examples. Unit tests prove the
  math; real-data tests prove the integration. A layer is NOT done
  until both pass.
- `cargo bench` (where applicable) — no regression beyond 5% on
  pre-existing benchmarks. New benchmarks added for new ops.
- `cargo clippy -- -D warnings` clean on new modules.
- Docs (`///` rustdoc) reference the relevant `catalog.md` section.
- One example added to `examples/` if user-facing.


## 1. Layer dependency graph

```
              L1 (Foundation: J, B, schema wiring)
                        │
                        ▼
                   L1.5 (B-perturbed
                         transport, flat case)
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
       L2              L3              L4
   (Adjacency)    (Cost & Spectral)  (Curvature
        │               │              decomp E.3)
        │               │               │
        └───────┬───────┴───────┬───────┘
                ▼               ▼
               L5              L6
        (Substructure       (Hodge complex
         detection,          §2.9, Morse
         curved-B            compress API)
         transport)
                │
                ▼
               L7
        (Quantization L7.1-L7.4: E.1, E.2, §2.1
         +  L7.5: Frobenius/WDVV on toy manifolds (§2.10)
         +  L7.6: Berezin-Toeplitz on toy manifolds (§2.8)
         +  L7.7: Riemann-Roch capacity API (§2.2))
                │
                ▼
               L8
        (Marcella substrate
         handoff — opens
         EARLY via L8.0
         consumption draft)
```

L1 blocks everything. L1.5 ships flat-space B-perturbed transport
right after L1 so Marcella's runtime can start integrating against
the API surface without waiting for L7. L5 takes over for
curved-manifold B-transport. L5 needs L3 + L4 (Hadamard detection
reads curvature). L7 needs L1 + L3 (line bundle needs forms and
Jacobi cost). L8 was originally scoped as documentation + empirical
pre-flight; we now ALSO open it early via L8.0 — the Marcella team
contributes `marcella_kahler_consumption_draft.md` as a PR before L2
ships, so the API surface stabilizes before L7 finalizes.

### Why this layout (versus Marcella team's tier-1-first proposal)

The 2026-05-24 letter from Marcella team correctly argues that
their tier-1 items (B-perturbed transport, Frobenius composition,
Berezin-Toeplitz regime) are the generative primitives, not the
GIGI-engine items L2 (adjacency) and L4 (curvature decomp). We
agree on priority — and on what those items unlock for Marcella.

We DON'T fully re-order the layers because the dependency stack
is real: L5 needs L3 + L4; L7 needs L1 + L3. Shuffling tier ordering
doesn't actually unlock Marcella's items faster — it just delays
GIGI's internal stability without buying time elsewhere.

What we DO change, per their letter:
- **Add L1.5** (B-perturbed transport, flat case) so the API
  surface lands right after L1.
- **Add L7.5 / L7.6 / L7.7** so Frobenius, Berezin-Toeplitz, and
  Riemann-Roch capacity all ship in L7 (scoped to toy Kähler
  manifolds — CP^n, T^n, S² — until §E.5 pre-flight confirms
  Marcella's actual embedding manifold structure).
- **Add L8.0** as the consumption-draft PR slot; doesn't gate
  anything but unblocks the interface conversation now.
- **Add Marcella-runtime API requirements** to L3, L5, L6, L7
  inline so each layer ships exposed-to-Marcella, not just
  exposed-to-planner.


---

## L1 — Foundation: complex structure, closed 2-form, schema wiring

**Scope.** Introduce the Kähler structure as optional metadata on
`BundleSchema`. Nothing else changes externally yet. This is the
type-plumbing layer.

### Items

- **L1.1** `src/geometry/mod.rs` — new module, declares submodules
  `complex_structure`, `forms`, `line_bundle` (placeholder),
  `moment_map` (placeholder), `hodge` (placeholder). Behind a
  `kahler` Cargo feature flag for safe rollout.
- **L1.2** `src/geometry/complex_structure.rs` — `ComplexStructure`
  type encoding `J: TₚM → TₚM` with `J² = -I`. Stored as a
  per-fiber `Vec<Vec<f64>>` (small dim) or sparse representation.
  Constructor verifies `J² = -I` to machine precision; rejects
  otherwise.
- **L1.3** `src/geometry/forms.rs` — `TwoForm` and `ClosedTwoForm`
  types. `ClosedTwoForm::new(t: TwoForm)` returns `Result` —
  succeeds only when `dT` is below ε in discrete exterior calculus.
- **L1.4** `src/types.rs` — extend `BundleSchema` with
  `pub kahler: Option<KahlerStructure>` where
  `KahlerStructure = { J: ComplexStructure, B: ClosedTwoForm }`.
  When `None`, the bundle is "pure Riemannian" and behaves exactly
  as today.

### TDD spec

```rust
// src/geometry/complex_structure.rs
#[cfg(test)]
mod tests {
    #[test] fn j_squared_is_neg_identity() { /* positive */ }
    #[test] fn rejects_non_almost_complex() { /* negative: J²≠-I */ }
    #[test] fn standard_j_on_r2_is_rotation_by_90() { /* anchor */ }
}

// src/geometry/forms.rs
#[cfg(test)]
mod tests {
    #[test] fn closed_form_constructor_accepts_dB_zero() { /* positive */ }
    #[test] fn closed_form_constructor_rejects_dB_nonzero() { /* negative */ }
    #[test] fn flat_two_form_on_R2_is_closed() { /* anchor */ }
}

// src/types.rs (extension)
#[cfg(test)]
mod kahler_schema_tests {
    #[test] fn schema_without_kahler_is_unchanged() { /* e2e: serialization,
                                                       size, behavior */ }
    #[test] fn schema_with_kahler_round_trips_through_dhoom() {}
}
```

### Math validation gate

- `validation_tests.py` (existing v1–v3): **all 11 must still pass**.
- No new Python tests in this layer — it's pure type plumbing.

### e2e validation gate

- `cargo test --release` clean.
- `cargo test --release --features kahler` clean.
- One integration test in `tests/kahler_optionality.rs`:
  - Build a bundle without `kahler`. Insert 1000 records. Query.
    Verify every output is bit-identical to a non-feature build.

### Definition of done

- L1 items above pass per §0.
- `examples/kahler_schema.rs` demonstrates attaching J + B to a
  toy schema.


---

## L1.5 — B-perturbed transport API (flat case)

**Depends on.** L1.
**Parallel with.** L2.

**Scope.** Marcella team's #1 ask in the 2026-05-24 letter: surface
a `bundle.transport_along(γ, with_B)` API at L2 timing so the
Marcella runtime can integrate against the surface while the rest
of the upgrade lands. We carve this out as a sub-layer because
it's a tiny additive surface that doesn't touch the planner.

GIGI already has a `TRANSPORT` GQL verb in `gigi_stream.rs` using
quaternion-based parallel transport on fiber bundles. L1.5 adds
the **magnetically-perturbed** variant (catalog §1.2): the
trajectory satisfies `∇_{γ̇} γ̇ = B(γ̇, ·)^♯` instead of
`∇_{γ̇} γ̇ = 0`. Energy is conserved along the flow (the
antisymmetry argument in catalog §1.2).

**Scope caveat.** L1.5 ships **flat-space** B-transport only —
exactly the case validated by `validation_tests.py::test_2`
(cyclotron trajectory on R², radius error 2e-14). On a curved
underlying manifold the magnetic geodesic equation is a nonlinear
ODE system that requires the curvature decomposition (L4) and the
Hadamard-region machinery (L5) before it's safe to run in
production. Curved-space B-transport is L5.

### Items

- **L1.5.1** `src/geometry/transport.rs` — new submodule under
  `geometry`. Contains:
  ```rust
  pub struct TransportSegment {
      pub from_point: Vec<f64>,
      pub to_point: Vec<f64>,
      pub initial_velocity: Vec<f64>,
      pub bias: Option<ClosedTwoForm>,
  }
  pub fn flat_transport(seg: &TransportSegment, steps: usize) -> TransportResult {
      // RK4 integration of:
      //   ẍ = B(ẋ, ·)^♯   when seg.bias = Some(B)
      //   ẍ = 0           when seg.bias = None
      // Returns the trajectory, final velocity, and
      // kinetic-energy drift (must be < tolerance per catalog §1.2).
  }
  ```
- **L1.5.2** Wire the API into `BundleStore` as
  `bundle.transport_along(seg, opts) -> SectionResult`. The opts
  parameter carries `with_B: Option<ClosedTwoForm>`; absent =
  classical transport (unchanged behavior); present = magnetic
  trajectory.
- **L1.5.3** Surface in GQL: extend the existing `TRANSPORT` verb
  with an optional `WITH B = <2form>` clause and an optional
  `ALLOW_NON_CLOSED` modifier. Backwards-compatible — TRANSPORT
  without `WITH B` behaves exactly as it does today.

### API contract (ratified per Marcella consumption draft §12 + 2026-05-24 reply)

Default behavior (B from bundle attribute when present):
```
TRANSPORT <section_id> FROM <start> TO <end> ON <bundle>
  [WITH B = <2form_symbol>]
  [ALLOW_NON_CLOSED]
RETURNS {
  transported_section: <id>,
  path_length: f64,
  energy_drift: f64,
  holonomy_norm: f64,
  used_magnetic: bool,
  b_source: "bundle" | "override" | "none" | "fallback_non_closed",
  closedness_norm: f64    // present only when b_source = fallback_non_closed
}
```

Resolution order for B:
1. `WITH B = <override>` if provided → `b_source: "override"`.
2. Else `bundle.kahler.B` if present → `b_source: "bundle"`.
3. Else classical quaternion transport → `b_source: "none"`.

Non-closed B handling:
- Default: validation error `{"error": "non_closed_b",
  "closedness_norm": ..., "tolerance": ...}` — fails loudly.
- With `ALLOW_NON_CLOSED`: falls back to classical transport,
  returns the result with `b_source: "fallback_non_closed"`
  and the `closedness_norm` for diagnostic.

Energy drift acceptance:
- 1e-9 per turn hard limit (response surfaces actual drift).
- Test-2-derived ground truth is 6e-15 on flat R²; production
  limit gives 6 orders of magnitude headroom for curved
  manifolds in L5.

### TDD spec

```rust
// src/geometry/transport.rs
#[cfg(test)]
mod tests {
    #[test]
    fn flat_classical_transport_is_straight_line() {
        // B = None ⇒ trajectory is exactly the straight line from
        // from_point to to_point at initial_velocity. Anchors the
        // "no behavior change without B" contract.
    }

    #[test]
    fn flat_magnetic_transport_cyclotron_radius() {
        // Port of validation_tests.py::test_2: B = b·dx∧dy,
        // initial v = (1, 0), cyclotron radius = |v|/b, period
        // 2π/b. Asserts radius error < 1e-12 AND energy drift
        // < 1e-12 over one full period.
    }

    #[test]
    fn flat_magnetic_transport_energy_conserved() {
        // ½|v|² constant along flow. Independently of cyclotron
        // form — works for any closed B on flat space.
    }

    #[test]
    fn opposite_signed_B_reverses_curvature_direction() {
        // Negative case: B with opposite sign sends the trajectory
        // the other way. Catches "we accidentally returned the
        // wrong sign" bugs.
    }
}
```

### Math validation gate

- `validation_tests.py::test_2_magnetic_trajectory` (the canonical
  cyclotron test) is the Python ground truth. Rust port must
  produce the SAME cyclotron radius for the same (B, v₀) inputs
  to within 1e-12.

### e2e validation gate

- `tests/kahler_transport_real_data_smoke.rs` — load sensor data,
  build a small Kähler bundle on (temperature, humidity), call
  `transport_along` with `with_B = None` and with `with_B = Some(B)`,
  verify the trajectories are different and that energy is
  conserved in both. Asserts the API surface works under realistic
  load.
- `tests/kahler_transport_marcella_contract.rs` — explicit assertion
  that the API shape matches what the Marcella consumption draft
  (L8.0) says it should be. If Marcella team's draft changes the
  contract, this test fails first.

### Definition of done

- L1.5 items pass per §0 (including bee's real-data rule).
- Marcella runtime can call `bundle.transport_along(seg, opts)` and
  get a result that includes the trajectory + conserved energy.
- Marcella team confirms the API shape matches their consumption
  draft (L8.0 dependency).


---

## L2 — Adjacency (§1.1)

**Depends on.** L1.

**Scope.** Principal + auxiliary adjacency operators as first-class
types in the storage layer, with the centrality-based commutativity
check the catalog identified as load-bearing.

### Items

- **L2.1** `src/graph/mod.rs` — new module.
- **L2.2** `src/graph/adjacency.rs` —
  `PrincipalAdjacency`, `AuxiliaryAdjacency` types wrapping the
  field-index graph already built in `src/spectral.rs::field_index_graph`.
- **L2.3** `src/graph/commutativity.rs` —
  `commute(p: &PrincipalAdjacency, a: &AuxiliaryAdjacency) -> CommutativityClass`
  using the **group-algebra centrality criterion** (NOT the full
  commutator matrix — that's O(n³) and the planner can't afford it).
  - For Cayley-graph-shaped adjacencies: check whether generator
    sets are closed under conjugation. O(|S|² × |G|) — small.
  - For general adjacencies: fall back to a sample-based test
    (commutator on a random vector basis; bounded error).
- **L2.4** `src/query.rs` — tag each `QueryPlan` node with
  `commutativity_class` so the planner can reorder where safe.
- **L2.5** (Marcella-facing, ratified in 2026-05-24 reply Q3)
  Surface `commutativity` in the HTTP query response JSON:
  ```json
  "commutativity": {
    "class": "Abelian" | "GeneratorSetsCentral" | "NumericallyVerified"
           | "NotCommute" | "Unknown" | "NotApplicable",
    "principal_field": "<field_id>",
    "auxiliary_field": "<field_id>",
    "max_commutator_entry": 0.0,        // present only when class=NotCommute
    "reason": "single_field"             // present only when class=NotApplicable
  }
  ```
  Omitted entirely (not null) when no Kähler structure is attached.
  Contract test: `tests/kahler_adjacency_marcella_contract.rs`.

### TDD spec

```rust
// src/graph/adjacency.rs
#[cfg(test)]
mod tests {
    #[test] fn principal_adjacency_from_field_index_matches_existing() {
        // anchor: ensure new types don't drift from src/spectral.rs::field_index_graph
    }
    #[test] fn auxiliary_adjacency_distinct_from_principal() {}
}

// src/graph/commutativity.rs
#[cfg(test)]
mod tests {
    // Direct ports of validation_tests.py::test_1_kahler_commutativity:
    #[test] fn z4_x_z4_adjacencies_commute() { /* positive */ }
    #[test] fn s3_non_central_transpositions_do_not_commute() { /* negative */ }
    #[test] fn s3_full_conjugacy_class_commutes_for_wrong_reason() {
        // The "trap" case from the catalog. Test must DETECT this case
        // (it commutes) AND the centrality check must explain WHY
        // (the generator set is central, not because of Kähler structure).
    }
}
```

### Math validation gate

- `validation_tests.py::test_1_kahler_commutativity` still PASS.
- No new Python tests — the existing one anchors the claim.

### e2e validation gate

- Build a `BundleStore` with a known commuting/non-commuting structure;
  issue a query whose plan can be reordered when ops commute. Verify
  result is bit-identical to the un-reordered plan. (`tests/query_reorder.rs`)
- Performance: reordered plan must be ≥ original on a 100k-row workload.

### Definition of done

- L2 items pass per §0.
- Query planner emits `commutativity_class` in `EXPLAIN` output.


---

## L3 — Cost & Spectral

**Depends on.** L1.
**Parallel with.** L2, L4.

**Scope.** Jacobi-field cardinality estimator + caching of spectral
gap at construction.

### Items

- **L3.1** `src/cost/mod.rs` — new module.
- **L3.2** `src/cost/jacobi_estimator.rs` —
  `cardinality_bound(store, query) -> CardinalityBound { lower, upper, mean }`
  via Jacobi-field integration along the query trajectory. Uses the
  same RK4 ODE solver from
  `validation_tests.py::jacobi_field` ported to Rust.
- **L3.3** `src/spectral.rs::cache_gap_on_insert` — currently
  `spectral_gap()` is on-demand. Add a cache that updates
  incrementally per insert (the gap shifts in a bounded way with
  rank-1 updates; use Davis-Kahan to bound the update).
- **L3.4** Wire `cost::cardinality_bound` into `src/query.rs::QueryPlan`
  so query plans carry geometric bounds.

### TDD spec

```rust
// src/cost/jacobi_estimator.rs
#[cfg(test)]
mod tests {
    #[test] fn jacobi_field_flat_matches_t() { /* K=0: J(t)=t */ }
    #[test] fn jacobi_field_hyperbolic_matches_sinh() { /* K=-1 */ }
    #[test] fn jacobi_field_spherical_matches_sin_with_conjugate_at_pi() {}
    #[test] fn cardinality_bound_hyperbolic_exceeds_flat() {
        // Bishop-Günther direction; matches v1 test 4
    }
}
```

### Math validation gate

- `validation_tests.py::test_3_hadamard_cartan` still PASS (Rust
  port of Jacobi integrator must produce same numbers as Python).
- `validation_tests.py::test_4_trajectory_ball_volume` still PASS.

### e2e validation gate

- `tests/cost_estimation.rs` — query a synthetic bundle with known
  ground-truth cardinality (uniform grid). Verify
  `cardinality_bound(q).mean` is within 5% of actual count over 100
  random queries.
- `tests/spectral_cache.rs` — insert 10k records one at a time;
  verify cached `spectral_gap()` matches the on-demand recomputation
  every 100 inserts to within ε.

### Marcella-runtime surface (per consumption draft §4)

Two surfaces ship at L3 (both ratified in the 2026-05-24 reply Q4):

1. **In-response field on every retrieval** when Kähler is attached:
   ```
   { "bundle_spectral_gap": 0.083,
     "spectral_gap_cached_at": "<iso8601>" }
   ```
   Cached on the bundle; freshness timestamp lets the runtime
   detect drift between reads and insert-driven bumps.
   Omitted entirely (not null) when no Kähler attached — keeps
   response shape unchanged for non-Kähler users.

2. **Dedicated endpoint** for out-of-band reads:
   ```
   GET /v1/bundles/<bundle_id>/spectral_gap
   RETURNS { lambda_2: f64, mix_time: u64,
             cheeger_lower: f64, cheeger_upper: f64 }
   ```
   Marcella's runtime uses `lambda_2` to set the rose-mechanism α
   coefficient via `α = 1 - 1/sqrt(mix_time)` — replaces the
   hardcoded 0.7 with a theorem-bound value per bundle.

Contract test: `tests/kahler_spectral_marcella_contract.rs`.

### Definition of done

- L3 items pass per §0.
- API change: `Section::query()` returns a `SectionResult` with a
  new `cardinality_bound` field.


---

## L4 — Curvature decomposition (E.3)

**Depends on.** L1.
**Parallel with.** L2, L3.

**Scope.** Extend `CurvatureStats` with the four Kähler invariants
when the bundle has a Kähler structure attached.

### Items

- **L4.1** `src/bundle.rs::KahlerCurvature` struct as in catalog
  §E.3.
- **L4.2** `src/curvature.rs::compute_kahler_decomposition(store, J, B) -> KahlerCurvature`.
- **L4.3** Streaming update in `src/bundle.rs::update_curvature()`
  on insert — must be O(1) per insert (the four invariants
  factor through the same Welford statistics as the scalar K).
- **L4.4** Surface in `/v1/bundles/<n>/curvature` API
  (`src/bin/gigi_stream.rs`).

### TDD spec

```rust
// src/curvature.rs
#[cfg(test)]
mod kahler_curvature_tests {
    #[test] fn fubini_study_cp1_ricci_is_einstein_constant_2g() {}
    #[test] fn fubini_study_cp1_weyl_is_zero() {}
    #[test] fn fubini_study_cp1_holo_sectional_is_4() {}
    #[test] fn fubini_study_cp1_holo_bisectional_in_1_to_4() {}
    #[test] fn flat_c1_all_curvature_components_zero() { /* negative */ }
}
```

### Math validation gate

- **New** `validation_tests_v4.py` with
  `test_14_kahler_curvature_decomposition`: independently compute
  the four invariants on CP¹ Fubini-Study via finite-difference
  Christoffel symbols (different formalism than the Rust streaming
  computation). Verify Rust and Python agree to 1e-6.

### e2e validation gate

- `tests/curvature_api.rs` — GET
  `/v1/bundles/<n>/curvature` on a Kähler bundle returns all four
  invariants; on a non-Kähler bundle returns the existing scalar K
  unchanged with `kahler: null`.

### Definition of done

- L4 items pass per §0.
- `KahlerCurvature` appears in serialized snapshots / DHOOM payloads
  when `kahler` feature is on.


---

## L5 — Hadamard substructure detection (§1.4, §1.5)

**Depends on.** L3 + L4.

**Scope.** Detect bundles / fibers / sub-bundles where `K_B ≤ 0`
everywhere. Tag them as Hadamard; surface guarantees in the API.

### Items

- **L5.1** `src/geometry/hadamard.rs` —
  `HadamardSubstructure` trait + `detect(store) -> Vec<HadamardRegion>`.
- **L5.2** Continuous-query convergence guarantees — extend
  `src/sheaf/mod.rs::propagate` with a
  `propagate_with_convergence_bound()` variant returning
  `(records, convergence_rate)` when the propagation runs over a
  Hadamard region.
- **L5.3** `bundle.tag('hadamard')` predicate exposed via SQL/GQL.
- **L5.4** (Marcella ask) `bundle.is_hadamard_region(query) -> bool`
  + `bundle.transport_inverse(γ) -> Section` Rust API exposed for
  the Marcella runtime to call directly. Lets Marcella's
  self-inspect output assert "this turn landed in a Hadamard
  sub-bundle; residue is provably stable" (catalog §1.4-1.5
  product application).
- **L5.5** (Marcella ask) Curved-manifold extension of L1.5's
  flat-space B-perturbed transport. The magnetic geodesic equation
  on a Kähler-Hadamard region is well-posed (no conjugate points,
  invertible). `bundle.transport_along(seg, with_B)` becomes
  globally well-defined on Hadamard sub-bundles.

### TDD spec

```rust
// src/geometry/hadamard.rs
#[cfg(test)]
mod tests {
    #[test] fn detects_hyperbolic_synthetic_bundle() { /* positive */ }
    #[test] fn rejects_spherical_synthetic_bundle() { /* negative */ }
    #[test] fn mixed_bundle_isolates_hadamard_subregion() {
        // Half positive curvature, half negative; only the negative
        // half is tagged.
    }
}
```

### Math validation gate

- `validation_tests.py::test_3_hadamard_cartan` still PASS (foundation
  unchanged).
- The Hadamard detection's negative case must trigger on the S²
  configuration from that test.

### e2e validation gate

- `tests/hadamard_propagate.rs` — set up a Hadamard sub-bundle,
  run a continuous propagation, verify the returned
  `convergence_rate` matches Adachi's bound (computed independently
  in Python and stamped into the test as a literal).

### Definition of done

- L5 items pass per §0.
- `EXPLAIN` shows "Hadamard region: yes/no" per bundle access.


---

## L6 — Hodge complex (§2.9)

**Depends on.** L1.

**Scope.** Discrete exterior calculus operators `d_0, d_1` from
cell incidence, Hodge Laplacians, Betti numbers per bundle.

### Items

- **L6.1** `src/discrete/mod.rs` — new module.
- **L6.2** `src/discrete/hodge_complex.rs` — `d_0, d_1, d_2`
  operators built from `BundleStore`'s cell structure; `d² = 0`
  enforced by construction.
- **L6.3** `src/discrete/hodge_laplacian.rs` — Hodge Laplacians
  `Δ_k = d†d + dd†`, betti numbers via eigendecomposition.
- **L6.4** Compression API: `compress_to_morse(store) -> MorseComplex`
  keeps only critical points + connections.
- **L6.5** (Marcella ask) Expose `bundle.morse_compress() ->
  MorseComplex` directly on `BundleStore` so the Marcella runtime
  can run transport on the compressed structure when prose density
  is uniform across regions. Per the 2026-05-24 letter, this is
  what makes Marcella scale to 10⁶+ substrate items without
  linear-walk costs (catalog §2.9 product application).

### TDD spec

```rust
// src/discrete/hodge_complex.rs
#[cfg(test)]
mod tests {
    #[test] fn d_squared_zero_on_torus_grid() {}
    #[test] fn d_squared_zero_on_tetrahedron() {}
}

// src/discrete/hodge_laplacian.rs
#[cfg(test)]
mod tests {
    #[test] fn betti_of_t2_grid_is_1_2_1() {}
    #[test] fn betti_of_tetrahedron_is_1_0_1() {} // S² Betti
    #[test] fn euler_characteristic_matches_v_minus_e_plus_f() {}
}
```

### Math validation gate

- `validation_tests_v3.py::test_11_hodge_torus` still PASS (Rust
  Betti must match Python Betti on identical T² grid).

### e2e validation gate

- `tests/hodge_compression.rs` — build a 1k-record bundle with
  known Morse structure (synthetic). Run `compress_to_morse()`,
  verify the compressed representation reconstructs the original
  cohomology to within ε; storage reduction reported.

### Definition of done

- L6 items pass per §0.
- `bundle.morse_compress()` exposed as a Rust API.


---

## L7 — Quantization: prequantization, Davis-debt, DHOOM compression
   (§2.1, §E.1, §E.2)

**Depends on.** L1, L3.

**Scope.** Line bundle as transition cocycle, Chern-class integrality
check, quantized holonomy debt (Davis Non-Decoupling extension),
DHOOM compression on integrally-quantized B.

### Items

- **L7.1** `src/geometry/line_bundle.rs` —
  - `LineBundle::from_transition_data(g_alpha_beta) -> Result<LineBundle, IntegralityError>`.
  - `LineBundle::chern_class() -> ChernClass(i64)`.
  - Constructor fails when `[B/2π]` is non-integral (returns the
    Dirac-string locus in the error).
- **L7.2** `src/curvature.rs::holonomy_debt(store, loop_keys) -> HolonomyDebt`
  where `HolonomyDebt = Quantized(i64) | Continuous(f64)`. Picks
  the variant based on whether `bundle.kahler.B` has integral
  Chern class.
- **L7.3** `src/dhoom.rs::QuantizedTwoForm`, `encode_chern`,
  `decode_chern`.
- **L7.4** Wire savings: snapshot writer (`src/wal.rs`?) picks
  the smaller of `Continuous` or `Chern` encoding per bundle.

### TDD spec

```rust
// src/geometry/line_bundle.rs
#[cfg(test)]
mod tests {
    #[test] fn wu_yang_integer_charge_constructs_globally() {}
    #[test] fn wu_yang_non_integer_charge_returns_dirac_string() {}
}

// src/curvature.rs
#[cfg(test)]
mod holonomy_debt_tests {
    #[test] fn integrally_quantized_loop_returns_integer_winding() {}
    #[test] fn non_quantized_loop_returns_continuous() {}
    #[test] fn davis_non_decoupling_floor_persists_under_gauge() {
        // Gauge transforms (src/gauge.rs) must NOT eliminate the debt.
        // This is the cosmological non-decoupling claim, tested.
    }
}

// src/dhoom.rs
#[cfg(test)]
mod chern_compression_tests {
    #[test] fn integral_b_compresses_at_least_10x() {}
    #[test] fn non_integral_b_falls_back_to_dense_encoding() {}
    #[test] fn round_trip_reconstructs_b_to_machine_epsilon() {}
}
```

### Math validation gate

- `validation_tests_v2.py::test_7_prequantization_integrality`
  still PASS.
- **New** `validation_tests_v4.py::test_12_quantized_holonomy_debt`:
  build a loop γ on S² with `[B/2π] = n`, integrate `A_φ dφ`, check
  result is `2π·n` for integer n; check continuous deviation for
  non-integer n.
- **New** `validation_tests_v4.py::test_13_dhoom_chern_roundtrip`:
  generate random `B` on `{S², T², CP², 4×4 grid torus}`, encode
  to Chern cocycle, decode, compare to original. Compression ratio
  ≥ 10× for integral cases. Round-trip error ≤ machine epsilon.

### e2e validation gate

- `tests/quantized_holonomy.rs` — insert records that form a closed
  loop on a Kähler bundle with `[B/2π] = 1`. Verify
  `holonomy_debt(loop) == HolonomyDebt::Quantized(1)`.
- `tests/dhoom_wire_savings.rs` — write a snapshot of a quantized
  bundle, measure wire size; compare to a snapshot of an equivalent
  non-quantized bundle. Document the ratio in a benchmark file
  (`benches/chern_compression.rs`).

### Definition of done

- L7 items pass per §0.
- Patent CIP draft saved to `_local/patents/dhoom_chern_cip.md`
  (bee writes the legal text; this is the technical-disclosure stub).


### L7.5 Frobenius / WDVV composition (catalog §2.10, scoped)

**Promoted from "Marcella v3 substrate / phase-1 deferred" to
active L7 sub-item per the 2026-05-24 Marcella team letter.**

**Scope caveat.** Full Frobenius / WDVV on arbitrary Kähler
manifolds requires computing Gromov-Witten invariants, which is
genuinely research-grade. We ship **toy-manifold composition only**:
the QH*(CP^n) product (closed-form via `(H^k mod n+1, q^k div n+1)`
arithmetic), plus T^n and S². Marcella's runtime can start
integrating against the API surface immediately; the production
deployment on her actual embedding manifold is gated on §E.5
pre-flight (which manifold she lives on, and whether its GW
invariants are known/computable).

### Items
- **L7.5.1** `src/geometry/quantum_cohomology.rs` —
  `QuantumCohomology` trait, with `Cpn { n: usize, q_truncation: usize }`,
  `TorusTn { n: usize }`, `Sphere2` implementations.
- **L7.5.2** `bundle.frobenius_compose(a: &Section, b: &Section) -> Section`
  on bundles whose schema declares `kahler.B` and an attached
  `QuantumCohomology` type. Returns `Result` (errors when the
  attached type doesn't support general composition yet).

### TDD spec
```rust
// src/geometry/quantum_cohomology.rs
#[cfg(test)]
mod tests {
    #[test] fn cp2_associator_zero_on_27_triples() { /* port test 8 */ }
    #[test] fn h_cubed_equals_q_on_cp2() { /* sanity for CP² */ }
    #[test] fn unknown_manifold_returns_unimplemented() { /* negative */ }
}
```

### Math validation gate
- `validation_tests_v2.py::test_8_frobenius_wdvv` still PASS.
- New `tests/kahler_frobenius_real_data_smoke.rs`: build a CP²
  Kähler bundle from synthetic real-shaped data, run
  `frobenius_compose` on all 27 basis triples, verify associator
  = 0 to machine epsilon.

### Per-region status (ratified in 2026-05-24 reply Q5)

When Marcella's embedding manifold has regions that are
toy-classifiable (CP^n, T^n, S²) and regions that aren't, the API
returns a region partition:

```json
{
  "regions": [
    { "region_id": "...", "manifold_class": "CP_3" | "T_n" | "S2" | "non_toy",
      "frobenius_ok": bool, "covers_fraction": f64,
      "associator_norm": f64 | null,
      "reason": "general_GW_invariants_not_computable" | null }
  ],
  "global_associator_ok": bool,
  "callable_on_regions": ["..."]
}
```

`POST .../frobenius_compose` takes a `region_id` parameter for
explicit region selection. Without `region_id`, errors with the
region partition as data — never silently picks a region.

Region detection runs after L5 Hadamard regions are detected;
re-classification on insert is debounced. Contract test
`tests/kahler_l7_marcella_contract.rs` asserts the region-partition
shape matches the consumption draft.


### L7.6 Berezin-Toeplitz operators on toy manifolds (catalog §2.8, scoped)

**Promoted from research-mode to active L7 sub-item per the
2026-05-24 letter.** Same scoping logic as L7.5: ship the API on
toy manifolds (CP^n, T^n, S²) where coherent states have
closed-form expressions; general case stays research-mode.

### Items
- **L7.6.1** `src/geometry/toeplitz.rs` — coherent-state
  construction on CP^n at fixed `k` (where ℏ = 1/k), Toeplitz
  operator `T_f` for `f: M → ℝ` smooth.
- **L7.6.2** `bundle.toeplitz_operator(f: SmoothFunction) -> Operator`
  surface on Kähler bundles with attached `QuantumCohomology` of
  toy type.

### TDD spec
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn toeplitz_bohr_correspondence_holds_at_small_hbar() {
        // T_f T_g - T_{fg} = O(ℏ), checked at ℏ ∈ {1, 1/2, 1/4, 1/8}
        // via the same BCH ground truth used in test 10.
    }
    #[test] fn toeplitz_on_general_manifold_returns_unimplemented() {}
}
```

### Math validation gate
- `validation_tests_v3.py::test_10_berezin_toeplitz` still PASS.

### Production ℏ lower bound (ratified in 2026-05-24 reply Q6)

`ℏ ≥ 4 / embedding_dim` is the safe deployment bound. Reason: the
bosonic-operator truncation `N ≳ 1/ℏ` must resolve ℏ-scale
features in the BT correction; `N ≤ d/4` gives headroom for the
truncation to not dominate the O(ℏ³) correction term.

`POST .../berezin_toeplitz` enforces:
- `hbar < 4/embedding_dim` → 400 `{"error": "hbar_below_safe_bound",
  "minimum": 4.0/d, "supplied": h}`
- `hbar ≥ 4/embedding_dim` → 200 with the operator + corrections
- Opt-out via body `"allow_below_safe_hbar": true` returns the
  result with `truncation_dominates_correction: bool` diagnostic
  so the caller knows reliability is compromised.

For Marcella's typical d ≈ 1024, safe minimum is ≈ 0.0039 — well
below the "deterministic" use case, so the production range is
wide. Contract test asserts both paths (in-bound + opt-out).


### L7.7 Riemann-Roch capacity API (catalog §2.2)

**Adopted per the 2026-05-24 letter, no scoping caveats.** Direct
consequence of L7 having the line bundle. Hilbert polynomial of
the Kähler embedding manifold, evaluated at `k = 1/ℏ`, gives the
capacity bound Marcella needs for her AGI-claim publishable
statement.

### Items
- **L7.7.1** `bundle.representational_capacity(k_max: i64) -> i64`
  — computes `dim H⁰(M, L^k)` via Riemann-Roch on toy manifolds.
  Returns the integer capacity bound.
- **L7.7.2** `bundle.hilbert_polynomial() -> Polynomial<i64>` —
  the full polynomial in k, so Marcella's runtime can read off
  coefficients (the integer ∫_M (B/2π)^n / n! that dominates the
  asymptotic).

### TDD spec
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn cp1_capacity_matches_test_9_theta_function_basis() {
        // Port of validation_tests_v3.py::test_9:
        //   dim H⁰(L^n) on T² with τ = i must equal n
        // for n ∈ {1, 2, 3, 4, 5}. Same answer through our
        // Riemann-Roch path.
    }
}
```


---

## L8 — Marcella substrate handoff + Hadamard pre-flight (§E.5)

**Depends on.** L1–L7 all shipped (for L8.1+).
L8.0 is the early-handoff slot — depends only on the 2026-05-24
Marcella letter being received (DONE: consumption draft mirrored
into this folder, GIGI replied with 7 question answers).

**Scope.** Document the substrate, write the empirical pre-flight
tests Marcella v3 needs before citing the Kähler upgrade as theorem.
This layer is mostly docs + tests on the Marcella side, no GIGI
engine changes — with the exception of L8.0, which opens the
cross-team interface conversation before L2 even ships.

### Flip protocol (ratified in 2026-05-24 reply Q7)

`KAHLER_ENABLED` default flips from `false` to `true` when ALL of:

1. L7 has shipped — all 7 sub-items (L7.1 through L7.7) with
   contract tests green.
2. All §E.5 pre-flight tests pass on Marcella's *actual*
   embedding manifold (not synthetic stand-ins).
3. Consumption-spec contract tests pass on the latest sheets
   bundle deployment for ≥ 1 full week without regression.
4. Marcella v3 paper draft reviewed by ≥ 1 external geometer
   (ideally from the ICDG colloquium circle given the Adachi-
   program provenance).

**GIGI proposes the flip; Marcella has veto with a stated reason.**
The principle: you're the gatekeeper of the thing you consume.
A postponement is data — usually means a pre-flight check is
borderline and we should investigate rather than override.

Co-announce when the flip happens.

### Items

- **L8.0** `theory/kahler_upgrade/marcella_kahler_consumption_draft.md`
  — Marcella team contributes this as a PR to this folder per the
  2026-05-24 letter. Documents the API surface they need from each
  layer (L2 commutativity class, L3 spectral gap, L4 curvature
  decomp, L5 Hadamard predicate, L6 Morse compress, L7
  transport_along + representational_capacity + Frobenius
  compose). GIGI team reviews; disagreements surface NOW, not at
  L8.1. Acceptance criteria: the API shapes in the draft match
  what L1.5 / L3 / L5 / L6 / L7 actually ship (the
  `kahler_transport_marcella_contract.rs` test in L1.5 enforces
  this for transport_along; analogous contract tests added per
  layer).
- **L8.1** `theory/kahler_upgrade/marcella_substrate.md` —
  spec describing exactly which GIGI APIs Marcella v3 consumes
  for its Kähler substrate (J operator, B 2-form, Jacobi
  cardinality, Hadamard detection, Morse compression, B-perturbed
  transport, representational capacity, Frobenius compose).
- **L8.2** `marcella/v3/preflight/hadamard_check.py` — implements
  §E.5 check 1 (sample 1000 Jacobi fields, verify non-vanishing).
  Lives in the Marcella repo, references GIGI APIs.
- **L8.3** `marcella/v3/preflight/closedness_check.py` — §E.5 check 2.
- **L8.4** `marcella/v3/preflight/holo_sectional_check.py` — §E.5
  check 3.
- **L8.5** Marcella v3 paper draft — citations to catalog items
  conditional on pre-flight pass/fail.

### TDD spec

- The pre-flight tests ARE the TDD spec for Marcella v3.
- Each must include a synthetic-data positive control (constructed
  to satisfy the check) and a synthetic-data negative control
  (constructed to violate it).

### Math validation gate

- All catalog tests (v1–v3) and new v4 tests still PASS.
- `theory/kahler_upgrade/validation/results_v*.txt` regenerated
  and committed alongside any catalog update.

### e2e validation gate

- Marcella v6 run through the three pre-flight checks; results
  documented in `marcella/v3/preflight/v6_results.md`. Coverage %
  recorded; v3 paper cites whichever items have ≥ 95% coverage as
  theorem, the rest as "verified on V6 to X% coverage."

### Definition of done

- L8 items pass per §0.
- v3 paper draft sent for review with empirical gates documented.


---

## A. Test inventory after all layers ship

### Python (`theory/kahler_upgrade/validation/`)

| File | Tests | Items covered |
|---|---|---|
| `validation_tests.py` (v1) | 1–6 | 1.1, 1.2, 1.3, 1.4, 1.5, 2.3, 2.5 |
| `validation_tests_v2.py` | 7–8 | 2.1, 2.10 |
| `validation_tests_v3.py` | 9–11 | 2.2, 2.8, 2.9 |
| `validation_tests_v4.py` (**new**) | 12–14 | E.1, E.2, E.3 |

### Rust (`src/**/tests`)

Per layer, listed under each L* §. Cross-layer integration tests
live in `tests/`:

- `tests/kahler_optionality.rs` (L1 gate)
- `tests/query_reorder.rs` (L2 gate)
- `tests/cost_estimation.rs` (L3 gate)
- `tests/spectral_cache.rs` (L3 gate)
- `tests/curvature_api.rs` (L4 gate)
- `tests/hadamard_propagate.rs` (L5 gate)
- `tests/hodge_compression.rs` (L6 gate)
- `tests/quantized_holonomy.rs` (L7 gate)
- `tests/dhoom_wire_savings.rs` (L7 gate)


## B. CI gates

GitHub Actions workflow `kahler-upgrade.yml`:

1. **`cargo test --release`** — full GIGI suite (with and without
   `--features kahler`). Must pass on every PR touching `src/`.
2. **`cargo clippy --features kahler -- -D warnings`** —
   no new warnings.
3. **Python math validation** —
   `PYTHONIOENCODING=utf-8 python -X utf8 validation_tests*.py`,
   exit code 0. Catches Rust/Python drift before merge.
4. **`cargo bench --features kahler -- --baseline main`** —
   regression budget: 5% slowdown allowed on existing benches;
   new benches added per layer.


## C. Out of scope for this upgrade

The catalog flags these as research-mode (§3 implementation order
items 13–15). They get separate proposals, NOT this plan:

- §2.6 Floer invariants — requires symplectic-PDE infrastructure.
- §2.7 Mirror symmetry / A-B duality — partially conjectural.
- §2.8 Berezin-Toeplitz quantum regime — validated numerically (v3
  test 10) but no clear product surface yet.
- §1.6 Hypersurface trajectory fast paths — testable, but
  unprioritized until a real use case surfaces.
- §2.4 K-theoretic operations — established mathematics, lower
  priority than the items above.

If a product team asks for any of these, surface the research-mode
flag from the catalog and have them spec the use case first.
