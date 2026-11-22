# GIGI Sample-Transport Sprint Spec

**Status:** Draft, ready for review
**Author:** Bee Rosa Davis, with Claude (Anthropic)
**Date:** May 2026
**Depends on:** `bundle/transport.rs`, `curvature.rs`, `parser/gql.rs`, `query/exec.rs`
**Targets:** Marcella v11+ generation, KRAKEN multi-hypothesis sensor fusion, ICARUS path planning
**Goal:** add a GQL verb that returns a distribution of geometrically-valid transport candidates instead of the single geodesic answer, with a curvature-bounded creativity budget.

---

## 1. Motivation

The current `TRANSPORT` verb is deterministic: given a source and destination on a fiber, it returns the unique geodesic rotation matrix between them.

```sql
TRANSPORT corpus FROM (token_str='walk') TO (token_str='walked')
  ON FIBER (f11, f12);
-- returns: a single SO(2) matrix.
```

This is correct database behavior. It is also the structural root of the dryness problem in Marcella v10/v11/v12 generation: the geometric channel is a flat plane of zero entropy, so all variation in generation has to come from the final-softmax sampler over logits. Statistical sampling cannot introduce geometric variation that the fiber didn't supply.

We need a verb that returns *a curvature-bounded neighborhood* of valid destinations from a source point on a fiber, not just the geodesic. The neighborhood is the set of admissible completions under the Davis field-equation budget `‖Hol − I‖ < τ`.

## 2. Formal Definition

Let `(E, B, F, π, Φ)` be the fiber bundle of the queried corpus and let `p_src ∈ E` be a section value at a base point `b_src ∈ B`. Fix a creativity budget `τ ∈ (0, 1]` (interpreted as a maximum admissible `d²` per candidate under the Double Cover `S + d² = 1`).

The **sample-transport neighborhood** of `p_src` at budget `τ` is

```
N(p_src, τ) := { p ∈ E : ‖Hol(γ_{p_src → p}) − I‖² ≤ 4τ }
```

equivalently (by `K² = 4 sin²(θ/2) = 4(1−S) = 4 d²`, see `double_cover_v3.pdf` §3):

```
N(p_src, τ) := { p ∈ E : d²(p_src, p) ≤ τ }
```

`SAMPLE_TRANSPORT` returns `k` draws from this set, weighted by a sampling kernel `w : N → ℝ_{>0}`. The default kernel is `w(p) = exp(−β · d²(p_src, p))` for a temperature `β > 0`.

## 3. GQL Grammar

```ebnf
sample_transport_stmt
  : "SAMPLE_TRANSPORT" ident
    "FROM" "(" key_expr ")"
    "ON" "FIBER" "(" field_list ")"
    "BUDGET" number
    "N" integer
    [ "WEIGHTED" "BY" expr ]
    [ "SEED" integer ]
    ";"
  ;
```

Example:

```sql
SAMPLE_TRANSPORT corpus
  FROM (token_str='walk')
  ON FIBER (f11, f12)
  BUDGET 0.3
  N 16
  WEIGHTED BY exp(-3 * d_sq)
  SEED 42;
```

### Semantics

- `BUDGET τ` — maximum `d²` per candidate. Hard constraint: any candidate with `d²(p_src, p) > τ` is excluded.
- `N k` — number of candidates returned.
- `WEIGHTED BY` — optional sampling-kernel expression. Defaults to `exp(-1 * d_sq)`. Must evaluate to a positive scalar in terms of `d_sq` and/or other fiber observables.
- `SEED` — optional deterministic seed for reproducibility. Without it, draws are sampled from the engine's CSPRNG.

### Response shape

```json
{
  "candidates": [
    {
      "destination_key": {"token_str": "walked"},
      "rotation": [[0.98, -0.20], [0.20, 0.98]],
      "d_sq": 0.040,
      "sameness": 0.960,
      "weight": 0.961,
      "curvature_K": 0.404
    },
    {
      "destination_key": {"token_str": "walking"},
      "rotation": [[0.95, -0.32], [0.32, 0.95]],
      "d_sq": 0.105,
      "sameness": 0.895,
      "weight": 0.730,
      ...
    },
    ...
  ],
  "budget": 0.3,
  "n_admissible": 47,
  "n_returned": 16,
  "kappa": 0.12,
  "confidence": 0.893
}
```

Note that `n_admissible` is the size of `N(p_src, τ) ∩ E_{stored}` — the engine reports how many records were geometrically valid, in addition to the `k` sampled.

## 4. Query Plan

```
1. PARSE         → AST node SampleTransport(corpus, src_key, fiber_fields, budget,
                                            k, weight_expr, seed)
2. RESOLVE       → look up bundle, validate fiber_fields belong to it,
                   compile weight_expr against the bundle schema
3. POINT-FETCH   → O(1) hash lookup of p_src via GIGI hash on src_key
4. NEIGHBORHOOD  → query the fiber's spatial index (existing BVH / R-tree on the
                   fiber subspace) for all records with squared chord distance to
                   p_src ≤ 4τ.  This is the candidate set.
                   Complexity: O(log N + |candidates|) with the index, O(N) without.
5. BUDGET-FILTER → for each candidate, compute exact d² via the
                   bundle/transport.rs holonomy routine.  Discard any whose exact
                   d² exceeds τ (the index bound is a chord upper estimate;
                   true holonomy may be slightly larger).
6. WEIGHT        → evaluate weight_expr against each candidate's (d_sq, sameness, K).
7. SAMPLE        → draw k indices without replacement, weighted by the kernel.
                   Use the alias-method sampler in core/sample.rs (existing).
8. ASSEMBLE      → for each sampled candidate, emit destination_key, rotation
                   matrix (from bundle/transport.rs), d_sq, sameness, weight, K.
9. ANALYTICS     → standard κ + confidence on the bundle, attached to response.
```

All steps except step 5 reuse existing infrastructure. Step 5 is the one new computational path: `parser/gql.rs` extension + `query/sample_transport.rs` (new file).

## 5. Rust API

In `src/query/mod.rs`:

```rust
pub struct SampleTransportPlan {
    pub corpus: String,
    pub src_key: KeyExpr,
    pub fiber_fields: Vec<FieldId>,
    pub budget: f32,
    pub k: usize,
    pub weight_expr: Option<Expr>,
    pub seed: Option<u64>,
}

pub struct SampleTransportResult {
    pub candidates: Vec<TransportCandidate>,
    pub budget: f32,
    pub n_admissible: usize,
    pub n_returned: usize,
    pub kappa: f32,
    pub confidence: f32,
}

pub struct TransportCandidate {
    pub destination_key: KeyMap,
    pub rotation: Matrix2<f32>,   // for 2D fibers; generalize to MatrixD for higher
    pub d_sq: f32,
    pub sameness: f32,
    pub weight: f32,
    pub curvature_k: f32,
}
```

In `src/query/sample_transport.rs` (new):

```rust
pub fn execute_sample_transport(
    plan: SampleTransportPlan,
    bundle: &BundleRef,
) -> Result<SampleTransportResult, QueryError> {
    // 1. Look up p_src
    let p_src = bundle.section_at(&plan.src_key)?;

    // 2. Neighborhood candidates via fiber spatial index
    let chord_bound = 4.0 * plan.budget;
    let raw_neighbors = bundle.fiber_index(&plan.fiber_fields)
        .range_query(&p_src, chord_bound)?;

    // 3. Budget filter via exact holonomy
    let mut admissible: Vec<TransportCandidate> = raw_neighbors
        .iter()
        .filter_map(|n| {
            let hol = transport::holonomy(&p_src, n, &plan.fiber_fields, bundle).ok()?;
            let d_sq = double_cover::d_sq_from_holonomy(&hol);
            if d_sq <= plan.budget {
                Some(TransportCandidate {
                    destination_key: n.key.clone(),
                    rotation: hol.matrix(),
                    d_sq,
                    sameness: 1.0 - d_sq,
                    weight: 0.0,           // filled in step 4
                    curvature_k: 2.0 * d_sq.sqrt(),
                })
            } else { None }
        })
        .collect();
    let n_admissible = admissible.len();

    // 4. Evaluate sampling weight kernel
    let weight_expr = plan.weight_expr
        .unwrap_or_else(|| Expr::default_weight()); // exp(-d_sq)
    for cand in admissible.iter_mut() {
        cand.weight = weight_expr.eval_on_candidate(cand)?;
    }

    // 5. Weighted sample without replacement
    let mut rng = match plan.seed {
        Some(s) => Rng::seed_from_u64(s),
        None    => Rng::from_entropy(),
    };
    let sampled = sample::alias_without_replacement(&admissible, plan.k, &mut rng);

    // 6. Bundle-level analytics
    let kappa = bundle.curvature();
    let confidence = 1.0 / (1.0 + kappa);

    Ok(SampleTransportResult {
        candidates: sampled,
        budget: plan.budget,
        n_admissible,
        n_returned: sampled.len(),
        kappa,
        confidence,
    })
}
```

## 6. Marcella Integration

`marcella/generate.py` changes — approximately 50 lines:

1. Add `use_sample_transport: bool` and `creativity_budget: float = 0.3` and `n_geometric_candidates: int = 16` to `generate()` signature.
2. After the model's forward pass produces `next_logits`, if `use_sample_transport` is true:
   - Identify the current fiber location (we have `R_acc` from the parallel scan; this maps to a section in the corpus bundle via the existing `marcella/fiber_bundle/lookup.py`).
   - Issue `SAMPLE_TRANSPORT corpus FROM (...) ON FIBER (f11, f12) BUDGET <τ> N <k>` to GIGI.
   - Receive `k` candidate destination keys + their geometric weights.
3. Restrict the next-token softmax to the union of (a) the `top_k` statistical candidates and (b) the `k` geometric candidates. Combine logits via `combined = α · logit + (1 − α) · log(weight)` where `α ∈ [0, 1]` is a single mixing knob exposed to the caller.
4. Sample from `combined`.

This gives the generator two sources of variation that explicitly trade off: statistical-likelihood diversity from the transformer, and geometric-admissibility diversity from GIGI. The mixing knob `α` controls the trade.

## 7. TDD Plan

Per the spec's convention, each numbered test is a `cargo test` target.

1. **`sample_transport_empty_budget`**: BUDGET 0 → returns 0 candidates (or exactly p_src if it's in the corpus).
2. **`sample_transport_full_budget`**: BUDGET 1.0 → admissible set = entire corpus on the fiber (modulo manifold topology).
3. **`sample_transport_seed_reproducibility`**: same SEED two runs → identical candidate ordering.
4. **`sample_transport_weight_default`**: no WEIGHTED BY → uses `exp(-d_sq)`, verified against manual computation.
5. **`sample_transport_weight_custom`**: `WEIGHTED BY (1 - d_sq)^2` → uses provided kernel; verified.
6. **`sample_transport_n_admissible_vs_returned`**: corpus of 100, BUDGET 0.05 admitting 7 candidates, N 16 → `n_admissible = 7`, `n_returned = 7` (cannot exceed admissible).
7. **`sample_transport_budget_monotonic`**: increasing BUDGET monotonically grows `n_admissible`.
8. **`sample_transport_d_sq_bounded`**: every returned candidate has `d_sq ≤ BUDGET` exactly.
9. **`sample_transport_kappa_passthrough`**: response carries the same `kappa` as a `CURVATURE corpus` query against the same bundle.
10. **`sample_transport_marcella_e2e`**: end-to-end against `marcella.generate.generate()` with `use_sample_transport=True` on a held-out checkpoint — verify generated text has higher distinct-2 than the `use_sample_transport=False` baseline.

## 8. Performance

Cost dominated by step 4 (neighborhood query + exact holonomy per candidate).
For a corpus of `N` records on a 2D fiber with an R-tree index:

| Phase | Complexity |
|-------|------------|
| Point fetch `p_src` | O(1) |
| Neighborhood query | O(log N + r) where r = raw neighbors |
| Holonomy per candidate | O(d²) per candidate where d = fiber dim |
| Weight eval | O(r) |
| Alias sample | O(r) preprocessing + O(k log r) draws |
| **Total** | **O(log N + r·d² + k log r)** |

Empirical target: < 5 ms per `SAMPLE_TRANSPORT` call at corpus size 1e6, fiber dim 2, BUDGET 0.3, N 16. (Equivalent to a few times the cost of a current `TRANSPORT` point-to-point query.)

## 9. Backward Compatibility

- No changes to `TRANSPORT`, `HOLONOMY`, `GEODESIC`, or any existing verb.
- No schema migration required.
- New verb is additive in the parser.
- Existing Marcella checkpoints work unchanged with `use_sample_transport=False` (the default during rollout).

## 10. Versioning

Target: GIGI v0.9 or v1.0 alongside the Marcella v12 generation refactor.
DHOOM wire-protocol change: yes — new response envelope `SampleTransportResult`. Bump DHOOM minor version.

## 11. Related Work and Citations

- Davis, B.R. (2026). *The Double Cover Principle*. Davis Geometric, Zenodo. (`S + d² = 1`, the budget interpretation.)
- Davis, B.R. (2025). *The Field Equations of Semantic Coherence*. (Branch VII / Sudoku Principle / admissible completions under bounded holonomy.)
- Davis, B.R. (2026). *The Davis Duality of Approximation and Obstruction*. (The curvature lower bound that makes BUDGET τ a meaningful constraint.)
- `MARCELLA_TEAM_UPDATE.md` — the corpus / fiber layout that this verb consumes.
- `GIGI_SUDOKU_SPRINT_SPEC.md` — the sheaf-completion sprint that established BUDGET semantics in vanishing-cohomology form.

---

## Acceptance criteria

- [ ] All 10 TDD tests pass in `cargo test --release`.
- [ ] `e2e/sample_transport_test.mjs` round-trips against a live `gigi-server`.
- [ ] `marcella/generate.py` integration produces measurably higher distinct-2 than the deterministic baseline on the v11 corpus.
- [ ] `GQL_REFERENCE.md` and `GIGI_API.md` updated to document the verb.
- [ ] `MARCELLA_TEAM_UPDATE.md` updated with the integration recipe.
