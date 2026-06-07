# Patterns v0.2 — Implementation plan

Gated TDD sprint plan for shipping the five verdict primitives from `SPEC_v0.2_VERDICT.md`. Math validation has shipped at 30/30 green in `validation_tests.py`; this plan is what turns the math into Rust.

## Sequencing

Build in this order:

```
  PE (PATTERN_EXPLAIN)
       ↓
  PP (pattern_preflight)
       ↓
  VT (verdict trichotomy)   ◄── PE and PP feed this
       ↓
  PR (PATTERN_REPAIR menu)  ◄── consumer-facing feature
       ↓
  K_P (pattern curvature)   ◄── opt-in operator self-check
```

Each phase ships independently and is independently useful. PE first because it's cheapest, unlocks the runtime debugger for everything else, and changes no public response shape (only adds an opt-in `_explain` field).

## Phase gates

Per phase, follow the gated-TDD discipline used for v0.1 and SUDOKU W6:

1. **Gate 1 (red)**: write the test file at `tests/pattern_v02_<phase>.rs`. Confirm `cargo test --features patterns --test pattern_v02_<phase>` fails for the right reason (function-not-found / variant-not-found).
2. **Gate 2 (green)**: implement minimally in `src/patterns/<module>.rs` (new sub-tree) or extend existing files where appropriate. Confirm the test file goes green.
3. **Gate 3 (regression)**:
   - `cargo test --features patterns --tests`: ALL green (incl. v0.1's 49 pattern tests + everything else)
   - `cargo test --tests`: ALL green (no-feature build byte-identical)
4. **Gate 4 (domain-swap)**: each test exists in 4 parallel variants with domain-named bundles (vuln-hunt, fraud, education, discourse-flow). All 4 variants identical numerical output.
5. **Gate 5 (Python math sync)**: `python theory/patterns/validation_tests.py` still 30/30 after Rust ships. The Rust port must match Python on the toy data, modulo serialization.
6. **Gate 6 (commit + cherry-pick)**: one commit per phase on `main`, cherry-picked to `scj-v0.1-substrate`, both pushed.

A phase doesn't graduate until all 6 gates are green.

---

## Phase PE — PATTERN_EXPLAIN

**Surface**: WeightExpr instrumentation that records per-term contributions.

**File adds**:
- `src/patterns/explain.rs` (new) — `explain(expr: &WeightExpr, row: &Record) -> ExplainNode` with `ExplainNode` enum mirroring `WeightExpr`
- `tests/pattern_v02_explain.rs` (new) — PE1–PE6 + 4 domain-swap variants
- Wire: extend `hunt_http` in `src/bin/gigi_stream.rs` to emit `_explain` when `request.explain == true`
- HTTP: `POST /v1/patterns/{name}/explain { bundle, row_pk }` for ad-hoc

**Tests (PE1–PE6 + DS)**:
- `PE1_explain_lit_returns_literal_value`
- `PE2_explain_field_returns_row_value`
- `PE3_explain_add_root_equals_eval`
- `PE4_explain_mul_root_equals_product`
- `PE5_explain_min_chosen_branch_and_clip_flag`
- `PE6_explain_full_scj_scorer_invariant`
- `DS_explain_vuln_hunt`, `DS_explain_fraud`, `DS_explain_education`, `DS_explain_discourse`

**Invariant**: `explain(expr, row).contribution == eval_weight(expr, row)` for every `(expr, row)` pair. Assert in every test.

**Risk**: low. No new math, just AST walk with tracking.

---

## Phase PP — pattern_preflight

**Surface**: three-layer preflight per spec §3.1.5 — internal contradiction (always gate), bundle-statistic (gate only at `budget=0`), holonomy (informational).

**File adds**:
- `src/patterns/preflight.rs` (new) — three functions `preflight_internal`, `preflight_statistic`, `preflight_holonomy`
- `tests/pattern_v02_preflight.rs` (new) — PP1–PP5 + 4 domain-swap variants
- Wire: `pattern_preflight` is called automatically by `verdict` (Phase VT), but also exposed as `GET /v1/patterns/{name}/preflight?bundle={bundle}`

**Tests**:
- `PP1_preflight_catches_impossible_numeric_range`
- `PP2_preflight_catches_missing_categorical`
- `PP3_preflight_catches_internal_contradiction`
- `PP4_preflight_holonomy_catches_joint_contradiction`
- `PP5_preflight_passes_satisfiable_predicate`
- `DS_preflight_*` (4 variants)

**Risk**: medium. Layer 3 (holonomy) requires lifting `holonomy_preflight` from `src/geometry/sudoku.rs` and adapting to predicate clauses. Source-of-truth math lives in the Python implementation in `validation_tests.py`.

---

## Phase VT — verdict trichotomy

**Surface**: HUNT response gains a `verdict` field. Order: internal preflight → (statistic preflight if budget=0) → sat scan → near-miss scan → unsat.

**File adds**:
- `src/patterns/verdict.rs` (new) — `Verdict` enum + `compute_verdict` orchestrator
- Edit `src/parser.rs` — `Statement::Hunt` gains `near_miss_budget: Option<usize>` field
- Edit `src/bin/gigi_stream.rs` — HUNT response gains `verdict` field, parses `near_miss_budget` from request
- `tests/pattern_v02_verdict.rs` (new) — VT1–VT5 + 4 domain-swap variants

**Tests**:
- `VT1_verdict_sat_when_rows_match`
- `VT2_verdict_unsat_by_preflight` (budget=0)
- `VT3_verdict_unsat_by_scan_when_no_match_and_no_near_miss`
- `VT4_verdict_near_miss_at_distance_1`
- `VT5_verdict_trichotomy_exhaustive`
- `DS_verdict_*` (4 variants)

**Wire shape change** (additive, backwards-compatible):

```json
// v0.1 response (unchanged when verdict is "sat" and no v0.2 flags used)
[ {row}, {row}, ... ]

// v0.2 response when near_miss_budget or any v0.2 flag set
{
  "verdict": "sat" | "unsat" | "near_miss",
  "rows": [...],
  "n_matches": int,             // when sat
  "near_miss_count": int,       // when near_miss
  "reason": "...",              // when unsat
  "preflight_caught": bool      // when unsat
}
```

Backwards-compat option: keep the `[row, ...]` shape when no v0.2 flags set. Wrap in `{verdict, rows}` only when v0.2 features used. **Default v0.2 to wrapped shape; gate the old shape behind `legacy_response: true` in the request if any consumer breaks.**

**Risk**: medium-high. This is the v0.1 wire breaking change (small but real). Pin the contract with snapshot tests.

---

## Phase PR — PATTERN_REPAIR menu

**Surface**: For each near-miss row, return the ordered min-cost flip sequence.

**File adds**:
- `src/patterns/repair.rs` (new) — `repair_menu(p: &Pattern, r: &Record, max_flips: usize, costs: &HashMap<String, f64>) -> RepairMenu`
- Edit `src/parser.rs` — `Statement::DefinePattern` gains `relaxation_costs: HashMap<String, f64>`, `Statement::Hunt` gains `repair_menu: bool`
- Edit `src/bin/gigi_stream.rs` — HUNT response's `near_miss_rows` entries gain `repair_menu` field
- `tests/pattern_v02_repair.rs` (new) — PR1–PR6 + 4 domain-swap variants

**Tests**:
- `PR1_repair_single_flip_uniform_cost`
- `PR2_repair_double_flip`
- `PR3_repair_custom_costs_sort_correctly`
- `PR4_repair_already_matches_returns_sentinel`
- `PR5_repair_too_far_returns_sentinel`
- `PR6_repair_min_cost_is_actually_minimum`
- `DS_repair_*` (4 variants)

**Risk**: low. Math is straightforward enumeration; the Python validation already shipped 30/30.

---

## Phase K_P — pattern curvature

**Surface**: per-bundle scalar measuring match concentration in kNN structure.

**File adds**:
- `src/patterns/curvature.rs` (new) — `pattern_curvature(p: &Pattern, b: &Bundle, k: usize) -> f64`
- Cache: lift the existing `BundleStats` kNN index machinery from `src/geometry/sudoku.rs` if not already shared
- HTTP: `GET /v1/patterns/{name}/curvature?bundle={bundle}&k={k}`
- `tests/pattern_v02_curvature.rs` (new) — K1–K4 + 4 domain-swap variants

**Tests**:
- `K1_kp_concentrated_pattern_is_strictly_positive`
- `K2_kp_responds_to_match_concentration` (same pattern, clustered bundle vs scattered)
- `K3_kp_empty_match_is_zero`
- `K4_kp_concentrated_exceeds_tautology`
- `DS_kp_*` (4 variants)

**Risk**: low for correctness, medium for perf. The naive O(N·k) kNN scan is fine for ≤10k rows but needs the BundleStats cache for larger bundles. Hook into the existing mutation_counter-keyed cache so K_P is recomputed lazily on bundle change.

---

## What this whole plan ships

- **5 new src files**: `explain.rs`, `preflight.rs`, `verdict.rs`, `repair.rs`, `curvature.rs` under a new `src/patterns/` module
- **5 new test files**: ~6 tests × 5 phases × (1 main + 4 domain-swap) ≈ **150 new tests**
- **HUNT response shape change** (additive, default-on, with `legacy_response: true` opt-out)
- **2 new HTTP endpoints**: `/v1/patterns/{name}/preflight`, `/v1/patterns/{name}/curvature`, plus `_explain` field on HUNT
- **2 new GQL clauses on HUNT**: `WITH NEAR_MISS_BUDGET k`, `EXPLAIN`, `REPAIR_MENU`
- **1 new GQL clause on DEFINE PATTERN**: `WITH RELAXATION_COSTS (...)`

Estimated test count after ship: **49 (v0.1) + 150 (v0.2) ≈ 199 pattern tests**, all green with `--features patterns`, all skipped with no features, 849 lib tests byte-identical.

## What this does NOT ship

- **Sharded verdict**: Phase 5 sharded HUNT is a separate ship. v0.2 verdict on a non-sharded bundle only.
- **Persistence of K_P over time**: K_P drift detection (catalog-level) is a v0.3 concern.
- **LOAD PATTERNS FROM `<path>`**: still on the v0.1 follow-up queue from SCJ Q2.
- **Variadic min/max, CLASSIFY-in-WEIGHT**: v0.3.

## Workflow per phase (recipe)

```bash
# Gate 1 — red
$EDITOR tests/pattern_v02_<phase>.rs
cargo test --features patterns --test pattern_v02_<phase>    # expect red

# Gate 2 — green
$EDITOR src/patterns/<module>.rs
cargo test --features patterns --test pattern_v02_<phase>    # expect green

# Gate 3 — regression
cargo test --features patterns --tests
cargo test --tests

# Gate 4 — domain-swap
# (variants are already in the test file; gate 2 ran them)

# Gate 5 — Python math sync
python theory/patterns/validation_tests.py                   # expect 30/30

# Gate 6 — ship
git add -A
git commit -m "patterns/v0.2: phase <PE|PP|VT|PR|K_P> — ..."
git checkout scj-v0.1-substrate
git cherry-pick main
git push origin scj-v0.1-substrate
git checkout main
git push origin main
```

## When to start

PE is independent of everything else and can start now. PP can start in parallel (no overlap with PE). VT requires PE + PP to land first. PR requires VT. K_P is independent of all the others and can ship last (or in parallel with anyone).

---

— Plan authored 2026-06-09. Spec: `SPEC_v0.2_VERDICT.md`. Math: `validation_tests.py`.
