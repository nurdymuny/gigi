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
         detection)          §2.9)
                │
                ▼
               L7
        (Quantization:
         E.1, E.2, §2.1)
                │
                ▼
               L8
        (Marcella substrate
         handoff)
```

L1 blocks everything. L5 needs L3 + L4 (Hadamard detection reads
curvature). L7 needs L1 + L3 (line bundle needs forms and Jacobi
cost). L8 ships last and is mostly documentation + the empirical
pre-flight gate from §E.5.


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


---

## L8 — Marcella substrate handoff + Hadamard pre-flight (§E.5)

**Depends on.** L1–L7 all shipped.

**Scope.** Document the substrate, write the empirical pre-flight
tests Marcella v3 needs before citing the Kähler upgrade as theorem.
This layer is mostly docs + tests on the Marcella side, no GIGI
engine changes.

### Items

- **L8.1** `theory/kahler_upgrade/marcella_substrate.md` —
  spec describing exactly which GIGI APIs Marcella v3 consumes
  for its Kähler substrate (J operator, B 2-form, Jacobi
  cardinality, Hadamard detection, Morse compression).
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
