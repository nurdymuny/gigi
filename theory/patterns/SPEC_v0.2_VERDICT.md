# Patterns v0.2 — Verdict primitives (SUDOKU lift)

*Davis Geometric internal — spec draft, math-validated, TDD-pending. Companion to `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` which shipped 2026-06-09.*

## §0. What this is

Patterns v0.1 (live on prod) gives operators a named, weighted, anti-joined HUNT. It answers **"what matches?"** — return a ranked list. Patterns v0.2 lifts the entire SUDOKU constraint-curvature machinery into the Patterns surface so a HUNT also answers:

- **"How tight is this pattern on this bundle?"** (K_P pattern curvature)
- **"Can this pattern even match here, before we scan?"** (pattern preflight)
- **"Sat / unsat / near-miss?"** (HUNT verdict trichotomy)
- **"What's the minimum-cost feature flip to make a near-miss match?"** (PATTERN_REPAIR menu)
- **"Why was this row scored 7.3?"** (PATTERN_EXPLAIN)

The math is the SUDOKU machinery that already shipped in `src/geometry/sudoku.rs` (constraint curvature, holonomy preflight, Γ trichotomy, RelaxationOption menu, energy descent). The v0.2 work is **wiring**, not new geometry — the same way v0.1 was "wiring" of FilterCondition + bitmap operations + sorted top-K into a named verb. We're applying the same compression move one level up.

The load-bearing claim from the SUDOKU work — **constraints are curvature; the substrate is domain-blind; the same machinery serves vuln-hunt, fraud, education, hiring, compliance, PRISM reconciliation, ICARUS flight-envelope monitoring** — carries forward verbatim. Every domain-swap test that's green for SUDOKU is green for Patterns v0.2 by construction.

---

## §1. The five primitives

| SUDOKU primitive (shipped) | Patterns v0.2 lift | What it actually does |
|---|---|---|
| `K_c` constraint curvature | **`K_P` pattern curvature** | Per-bundle measurement of how concentrated the pattern's matching neighborhood is. "This pattern is too tight / too loose for this corpus." |
| Holonomy preflight | **`pattern_preflight`** | Before HUNT scans, compute the predicate's holonomy on the bundle's actual field distribution. Non-trivial holonomy → contradictory pattern → return empty *without scanning a row*. |
| Γ trichotomy | **`HUNT verdict`** | Every HUNT response carries `verdict: "sat" | "unsat" | "near_miss"` alongside the rows. |
| RelaxationOption menu | **`PATTERN_REPAIR`** | For each near-miss row, return the ordered minimum-cost feature-flip sequence that would make it match. |
| Energy descent (per-term decomposition) | **`PATTERN_EXPLAIN`** | Decompose a row's `_score` into per-WEIGHT-term contributions, with relaxation options attached. |

These are not five independent features. They form a layered stack:

```
                     PATTERN_EXPLAIN
                          ▲
                   uses ──┘
                          │
                          ▼
                    PATTERN_REPAIR
                          ▲
                   uses ──┘
                          │
                          ▼
                    HUNT verdict
                          ▲
                   uses ──┘
                          │
                          ▼
                  pattern_preflight
                          ▲
                   uses ──┘
                          │
                          ▼
                      K_P curvature
```

Each layer is independently useful. Each upper layer makes the lower ones richer. Build bottom-up.

---

## §2. `K_P` pattern curvature

### §2.1 Definition

Given a pattern `P = (predicate, weight, using)` and a bundle `b` with N rows, define:

$$K_P(b) = \mathrm{Var}_{i \in b}\left[\frac{|\{j \in N_k(i) : P\text{ matches } j\}|}{k}\right]$$

where `N_k(i)` is the k-nearest-neighbor set of row `i` in the bundle's `using` field space (Euclidean over scalar fields, Hamming over categorical, mixed via existing `BundleStats` scaling).

In plain terms: **for each row, ask "what fraction of my neighbors also match this pattern?"** The variance of that ratio across all rows is `K_P`.

- **High `K_P`**: matching rows cluster — sharp concentration. The pattern picks out a distinct region of the bundle. Good signal-to-noise; operator can trust the ranking.
- **Low `K_P`**: matching rows are uniformly sprinkled — diffuse. The pattern's predicate is barely informative; the WEIGHT does most of the work.
- **K_P near zero with high match rate**: pattern is essentially a tautology on this bundle.
- **K_P near zero with low match rate**: pattern is essentially a rare-coincidence detector — interesting, but you may want to check the WEIGHT for accidental discrimination.

### §2.2 Interpretation

`K_P` answers operator questions like:

- "Is my pattern over- or under-fit to this corpus?"
- "Should I tighten the predicate or loosen the WEIGHT?"
- "Is the council catalog drifting — same pattern, much lower `K_P` than last month?"

Drift in `K_P` over time is a council-coordinated signal: a pattern's `K_P` history is a versioning artifact of the catalog.

### §2.3 Algorithm

```
fn pattern_curvature(p: &Pattern, b: &Bundle, k: usize) -> f64:
    let match_set = scan(p, b)                    // existing Phase 3 COVER
    let neighbors_idx = bundle_stats(b).knn_index(k, p.using_fields)
    let ratios = (0..b.n_rows).map(|i|
        let nbrs = neighbors_idx.neighbors(i)
        let matches_in_nbrs = nbrs.intersect(match_set).len()
        matches_in_nbrs as f64 / k as f64
    )
    variance(ratios)
```

Complexity: O(N · k) for the neighbor scan + O(N) for the variance. The kNN index is cached on `BundleStats` (already exists for SUDOKU). Per-bundle, per-pattern, cacheable. Streaming-updatable if the bundle grows.

### §2.4 GQL + HTTP surface

```sql
SHOW PATTERN_CURVATURE p IN b;          -- returns K_P scalar
SHOW PATTERN_CURVATURE p IN b WITH K=20; -- override default k
```

```http
GET /v1/patterns/{name}/curvature?bundle={bundle}&k=20
→ { "k_p": 0.34, "k": 20, "n_matches": 47, "n_rows": 1023 }
```

### §2.5 v0.2 contract

- Default `k = sqrt(N)` rounded up, floor 8, cap 256. Operator overrides via `WITH K`.
- Mixed-type fields use `BundleStats` for per-field z-score scaling. Categorical fields contribute Hamming distance, scalars contribute z-scaled Euclidean. Vector fields use existing cosine.
- Pattern with no `using` fields: error 400, "pattern_curvature requires USING fields to be declared."
- Pattern that matches 0 rows: `K_P = 0` by convention (no variance possible). Return with `n_matches: 0` so the caller can distinguish from a real-zero variance.

---

## §3. Pattern preflight

### §3.1 Definition

Before HUNT scans a single row, check if the pattern's predicate is satisfiable against the bundle's actual field statistics. If not, short-circuit to `verdict: "unsat"` with `reason`.

Two layers of preflight, in order:

1. **Field-statistic preflight** (cheap): for each field appearing in the predicate, check whether the predicate's constraint on that field is compatible with the bundle's actual value distribution.
   - `WHERE x >= 100` against a bundle where `max(x) = 50` → unsat
   - `WHERE color = 'red'` against a bundle where `color` is `{'blue', 'green'}` → unsat
   - `WHERE x >= 0 AND x < 0` → unsat (internally contradictory, no bundle needed)

2. **Holonomy preflight** (cheaper than scan, more expensive than statistic check): for predicates with multi-field constraints, compute the holonomy of the constraint graph around small loops in the bundle's field-graph. Non-trivial holonomy indicates locally inconsistent constraints. (This is the SUDOKU `holonomy_preflight` lifted verbatim — same algorithm, same bound.)

### §3.1.5 Three layers, not two (refinement during math validation)

During Python math validation (`validation_tests.py`), we discovered that **bundle-statistic preflight should only be a verdict gate when `near_miss_budget = 0`**, because near-miss is precisely the case where field values get repaired by flipping. A predicate `color = 'purple'` against a bundle of `{red, blue}` looks "impossible" to the statistic check, but with budget ≥ 1 it's a single-flip near-miss away from sat.

So we actually need **three layers**:

1. **Internal contradiction check** — always a verdict gate. `x >= 5 AND x < 3` cannot be repaired by flipping any bundle row's values. This is the SUDOKU `holonomy_preflight` reduced to a 1-loop on a single field.
2. **Bundle-statistic preflight** — verdict gate ONLY when `near_miss_budget = 0`. With budget ≥ 1, near-miss may repair it, so the scan handles the verdict.
3. **Holonomy preflight** — informational only (fires in the unsat branch to explain *why* unsat). For predicates with 2+ fields and zero matches, it tells the operator whether the joint distribution structurally forbids the conjunction.

### §3.2 Why both layers

The statistic preflight catches the gross-impossibility cases (constraint asks for values that don't exist). The holonomy preflight catches the subtle cases where each field's range is compatible but the *joint* constraint is incompatible (Bayes-net-style inconsistency).

Example where statistic preflight passes but holonomy doesn't:

```sql
DEFINE PATTERN p AS x = 1 AND y = 1 AND z = 1;
-- Bundle has x, y, z each as 0/1 with marginals 0.5/0.5,
-- but joint is constrained so (x=1, y=1, z=1) never co-occurs.
-- Statistic preflight: each predicate clause is individually satisfiable.
-- Holonomy preflight: the (x,y,z) triangle in the joint distribution
-- has non-trivial holonomy on the (1,1,1) corner → unsat.
```

### §3.3 Algorithm

```
fn pattern_preflight(p: &Pattern, b: &Bundle) -> PreflightVerdict:
    // Layer 1: field-statistic preflight (always run)
    for clause in p.predicate.clauses():
        let field = clause.field
        let stats = bundle_stats(b).field_stats(field)
        if !clause.is_satisfiable_against(stats):
            return PreflightVerdict::Unsat {
                reason: format!("field {} cannot satisfy {} (range {})", ...)
            }

    // Layer 2: holonomy preflight (only for multi-field predicates)
    if p.predicate.field_count() >= 2:
        let constraint_graph = build_constraint_graph(p.predicate, b)
        let h = holonomy_loop(constraint_graph, max_loop_len=4)
        if !h.is_trivial(tolerance=1e-6):
            return PreflightVerdict::Unsat {
                reason: format!("holonomy det={:.4} ≠ ±1 on loop {:?}", h.det, h.loop)
            }

    PreflightVerdict::Ok
```

Complexity: layer 1 is O(C) where C = number of predicate clauses. Layer 2 is bounded by the SUDOKU holonomy preflight, which is O(F²) where F = number of fields in the predicate (typically small, 5–20).

### §3.4 GQL + HTTP surface

Pattern preflight runs *automatically* before every HUNT — caller doesn't ask for it. The verdict shows up in the response envelope (see §4).

For ad-hoc preflight queries (without running HUNT):

```sql
SHOW PATTERN_PREFLIGHT p IN b;
-- returns: ok | unsat with reason
```

```http
GET /v1/patterns/{name}/preflight?bundle={bundle}
→ { "verdict": "ok" }
   or
→ { "verdict": "unsat", "reason": "field x cannot satisfy x >= 100 (max=50)" }
```

---

## §4. HUNT verdict trichotomy

### §4.1 The contract

Every HUNT response carries a `verdict` field:

```http
POST /v1/bundles/{name}/hunt { "pattern": "p" }
→
{
  "verdict": "sat",
  "rows": [ { ... }, { ... }, ... ],   // top-N by _score
  "n_matches": 47
}

OR

{
  "verdict": "unsat",
  "rows": [],
  "reason": "field x cannot satisfy x >= 100 (max=50)",
  "preflight_caught": true             // saved the scan
}

OR

{
  "verdict": "near_miss",
  "rows": [],
  "near_miss_count": 12,
  "n_examined": 1023
}
```

### §4.2 Trichotomy logic

```
sat        := at least one row matches the predicate strictly
unsat      := provably zero rows match
              (either preflight caught it OR the scan found zero)
near_miss  := zero rows match strictly,
              but ≥1 row is within the relaxation budget of matching
```

Default relaxation budget: 1 feature flip (Hamming-1 from the predicate's strict satisfaction set). Operator overrides via `WITH NEAR_MISS_BUDGET k`.

### §4.3 Why a trichotomy not just a boolean

`unsat` and `near_miss` are operationally different:

- `unsat`: there's nothing to look at. Operator should reconsider the predicate.
- `near_miss`: there's something to look at, but it's *almost* what you asked for. This is the **patch twin** case, the **almost-fraud** case, the **almost-passing student** case. The whole reason Patterns v0.2 exists.

Returning `near_miss` with `near_miss_count > 0` is the trigger for the operator to call `PATTERN_REPAIR` (§5) and see the actual rows.

### §4.4 GQL surface

The verdict shows up on existing `HUNT` automatically. Two new options:

```sql
HUNT p IN b WITH NEAR_MISS_BUDGET 2;       -- allow 2 flips
HUNT p IN b WITH NEAR_MISS = OFF;          -- skip near_miss check (faster)
```

`WITH NEAR_MISS = OFF` is the v0.1-compatible mode — preserves the existing wire shape (no verdict field). Defaults to on in v0.2.

---

## §5. `PATTERN_REPAIR` menu

### §5.1 Definition

For each near-miss row `r`, compute the minimum-cost ordered sequence of feature flips that would make `r` satisfy the pattern's predicate.

"Feature flip" = changing one field's value to a value that satisfies the corresponding predicate clause. Cost = the per-field relaxation cost from the existing SUDOKU `relaxation_cost` machinery (defaults to 1 per flip; configurable per-field via pattern metadata).

The result is the **repair menu** — ordered by total cost ascending. The first entry is the cheapest path to match.

### §5.2 Why it's load-bearing

This is the **near-miss explanation** that operators actually want:

- SCJ vuln hunt: "this function is 1 flip from matching — the missing bit is `reaches_ExAllocatePool2`. Worth a closer look?"
- Fraud monitoring: "this transaction is 2 flips from triggering — flip `amount_over_threshold` and `same_origin_destination` and it's flagged."
- Education: "this student is 1 assignment from passing — assignment ID 7, completion rate from 0.3 to 0.8."
- Hiring: "this candidate is 1 certification from qualifying — cert AWS-SAA."
- PRISM: "this transaction is 1 field from matching — origin bank changed format on date X."

### §5.3 Algorithm

```
fn pattern_repair(p: &Pattern, r: &Row, max_flips: usize) -> RepairMenu:
    // Use existing SUDOKU relaxation menu machinery
    let violations = p.predicate.violations(r)         // which clauses fail
    if violations.is_empty():
        return RepairMenu::AlreadyMatches
    if violations.len() > max_flips:
        return RepairMenu::TooFar
    // Enumerate flip combinations (small — bounded by max_flips * fields)
    let candidates = enumerate_flip_sequences(violations, max_flips)
    let costed = candidates.map(|seq| (seq, total_cost(seq, p.relaxation_costs)))
    costed.sort_by_cost_asc()
    RepairMenu::Options(costed)
```

Complexity: O(F^k) where F = number of fields in predicate, k = max_flips. With default `k=1` this is just O(F). With `k=2` it's O(F²). The exponential is bounded — operators don't ask for `k=5` flips because that's not "near" anymore.

### §5.4 GQL + HTTP surface

```sql
HUNT p IN b WITH NEAR_MISS_BUDGET 2 REPAIR_MENU;   -- include repair menus
```

```http
POST /v1/bundles/{name}/hunt
  { "pattern": "p", "near_miss_budget": 2, "include_repair_menu": true }
→
{
  "verdict": "near_miss",
  "near_miss_count": 12,
  "near_miss_rows": [
    {
      "row": { ... },
      "repair_menu": [
        { "flips": [["reaches_ExAllocatePool2", 0, 1]], "cost": 1.0 },
        { "flips": [["has_probe_read", 0, 1]], "cost": 1.0 }
      ]
    },
    ...
  ]
}
```

### §5.5 v0.2 contract

- Default `max_flips = 1`. Cap at 3 to keep the enumeration bounded.
- Per-field relaxation costs configurable on the pattern definition: `DEFINE PATTERN ... WITH RELAXATION_COSTS (field_a = 2.0, field_b = 0.5)`. Defaults to 1.0 per field.
- Repair menu is sorted by total cost ascending, then by flip count ascending (cheapest, fewest flips first).
- Repair menu is **bounded to top 5 options per row** by default (operator override). Beyond 5 the menu stops being readable.

---

## §6. `PATTERN_EXPLAIN`

### §6.1 Definition

For a single scored row, decompose its `_score` into per-WEIGHT-term contributions.

A pattern's WEIGHT expression is a tree of `Lit`, `Field`, `Add`, `Sub`, `Mul`, `Div`, `Min`, `Max` nodes. Each leaf (Lit or Field) makes a numeric contribution to the final score; each interior node propagates contributions according to its operator.

```
WEIGHT (
    cast_truncate_alloc * 3
  + multiply_before_alloc * 3
  + reaches_ExAllocatePool2 * 1
)

For row r = {cast_truncate_alloc: 1, multiply_before_alloc: 0, reaches_ExAllocatePool2: 1}:

PATTERN_EXPLAIN(r) →
[
  { term: "cast_truncate_alloc * 3",   value: 3.0, fraction: 0.75 },
  { term: "multiply_before_alloc * 3", value: 0.0, fraction: 0.00 },
  { term: "reaches_ExAllocatePool2 * 1", value: 1.0, fraction: 0.25 },
]
Total: 4.0
```

For `min` / `max`: attribute to the chosen branch (the one whose value matters).

```
WEIGHT (min(sum_of_terms, 10.0))

For row with sum_of_terms = 15:
  → min chose the literal 10.0
  → explanation: "clipped at 10.0 (raw sum was 15)"
```

### §6.2 Why it's load-bearing

Patterns are catalog artifacts. Catalogs are shared by councils. Councils argue. They argue better with receipts. PATTERN_EXPLAIN turns "this row scored 7.3" into "this row scored 7.3 because terms A and B fired (5.0 + 3.0), and the clip held it from 8.0 down to 7.3."

The same surface is also the runtime debugger for pattern authors: "I expected this row to score higher; which term didn't fire?"

### §6.3 Algorithm

Recursive walk of the `WeightExpr` AST, evaluated against the row, returning a tree of `(node, value, contribution)` triples.

```
fn explain(expr: &WeightExpr, row: &Row) -> ExplainNode:
    match expr:
        Lit(v) → ExplainNode::Lit { value: v, contribution: v }
        Field(name) → ExplainNode::Field { name, value: row[name], contribution: row[name] }
        Add(l, r) → ExplainNode::Add {
            left: explain(l, row),
            right: explain(r, row),
            contribution: left.contribution + right.contribution
        }
        Sub(l, r) → similar
        Mul(l, r) → ExplainNode::Mul {
            left: explain(l, row),
            right: explain(r, row),
            contribution: left.contribution * right.contribution
        }
        Div(l, r) → similar (with NaN safety)
        Min(l, r) → ExplainNode::Min {
            left, right,
            chosen: if left.contribution <= right.contribution { "left" } else { "right" },
            contribution: min(left.contribution, right.contribution),
            clipped: (left.contribution > contribution || right.contribution > contribution)
        }
        Max(l, r) → similar
```

Complexity: O(|expr|) per row. For top-K HUNT results this adds O(K * |expr|), negligible.

### §6.4 GQL + HTTP surface

```sql
HUNT p IN b TOP 10 EXPLAIN;
-- Returns each row with an `_explain` field next to `_score`.
```

```http
POST /v1/bundles/{name}/hunt
  { "pattern": "p", "top": 10, "explain": true }
→
{
  "verdict": "sat",
  "rows": [
    {
      "id": 42,
      ...
      "_score": 7.3,
      "_explain": {
        "type": "min",
        "chosen": "left",
        "clipped": false,
        "left": { ... breakdown ... },
        "right": { "type": "lit", "value": 10.0, "contribution": 10.0 }
      }
    },
    ...
  ]
}
```

For ad-hoc explanation of a single row:

```http
POST /v1/patterns/{name}/explain
  { "bundle": "b", "row_pk": 42 }
→ { "_score": 7.3, "_explain": { ... } }
```

### §6.5 v0.2 contract

- `_score` and `_explain` both present in the EXPLAIN response; `_score` last, `_explain` immediately before it (preserves the §5(a) wire contract).
- `_explain.contribution` always equals `_score` when summed correctly (the tree's root's contribution = the final score). Invariant tested.
- Operator-facing rendering: TUIs can render the explain tree as nested bullet points; the wire shape is a JSON tree.

---

## §7. Composition with v0.1

| v0.1 surface | v0.2 status |
|---|---|
| `DEFINE PATTERN` | Same. New optional clause: `WITH RELAXATION_COSTS (...)`. |
| `HUNT` | Same. New optional clauses: `WITH NEAR_MISS_BUDGET`, `WITH NEAR_MISS = OFF`, `REPAIR_MENU`, `EXPLAIN`. New auto-emitted field: `verdict`. |
| `SHOW PATTERNS` | Same. |
| `DROP PATTERN` | Same. |
| `EXCLUDING IN` | Same. Composes with verdict trichotomy — excluded rows are removed *before* the trichotomy is computed. |
| `POST /v1/patterns` | Same. |
| `POST /v1/bundles/{name}/hunt` | Same wire compatibility when `verdict` field is set to `"sat"` and no new clauses are used. New behavior gated on `near_miss_budget` / `explain` / `include_repair_menu` request fields. |
| `_score` LAST | Same. `_explain` (when present) lands second-to-last; `_score` stays trailing. |

**Backwards compatibility**: any v0.1 client that ignores unknown fields keeps working. The default response shape gains a `verdict` field but nothing else changes unless the client opts in. v0.2 is additive.

---

## §8. Domain-neutrality discipline

The SUDOKU domain-swap tests are the template:

For every primitive shipped (§2–§6), every TDD test should be written in **four parallel variants** with bundle/field/pattern names that look like:

- vuln-hunt style (SCJ): `vid_funcs`, `cast_truncate_alloc`, `confirmed_bugs`
- fraud style: `transactions`, `amount_over_threshold`, `flagged`
- education style: `students`, `assignments_complete`, `passing`
- discourse-flow style: `utterances`, `dialog_act`, `boundary`

Each variant runs the **identical primitive call** against domain-named bundles. If they all pass, the substrate is verified domain-blind. If any one fails, that's the primitive leaking domain assumptions.

This is the same discipline that proved SUDOKU general-purpose. Apply it from day one.

---

## §9. Open questions

1. **K_P streaming update on insert.** `K_P` is a per-bundle scalar that depends on the kNN structure. On bundle insert, do we recompute fully (O(N k)) or update incrementally (O(k²) amortized)? *Recommendation:* incremental, with a forced full-recompute every 10% bundle growth. The kNN index is already cached for SUDOKU; reuse.

2. **Holonomy preflight loop length.** SUDOKU's preflight currently scans loops of length ≤4. For patterns over 10+ fields, that may miss higher-order inconsistencies. *Recommendation:* keep ≤4 in v0.2, surface a `preflight_loop_len` knob for operators who need deeper checks.

3. **Repair menu ordering ties.** When two flip sequences have the same cost, what's the tie-break? *Recommendation:* by lexicographic field name (deterministic, replicable across runs).

4. **Explain leaf attribution under `min` / `max`.** If the unchosen branch had higher contribution, should its terms still be visible in the explain tree? *Recommendation:* yes — return both branches, mark the chosen one. Operators want to see what was clipped.

5. **Near-miss budget budget.** Should there be a runtime cap on `max_flips` to prevent runaway enumeration? *Recommendation:* hard cap 3 in v0.2. The use case for "5+ flips from matching" doesn't exist in any of the domain-swap tests.

6. **Verdict caching.** A pattern's verdict on a stable bundle never changes. Should we cache it? *Recommendation:* yes, keyed on `(pattern_id, bundle_mutation_counter)`. Existing MorseCache infrastructure has the right shape.

7. **PATTERN_EXPLAIN performance for large WEIGHT expressions.** With variadic min/max (v0.3) and CLASSIFY (v0.3) the expression tree could grow. *Recommendation:* not a v0.2 concern — current grammar's expressions are small.

8. **Should `K_P` be a free signal on every HUNT, or opt-in?** Computing it is O(N k) which is non-trivial for large bundles. *Recommendation:* opt-in via `HUNT ... EXPLAIN` (the same flag that turns on per-row explain), since the operator clearly wants verbose output. Default off.

---

## §10. TDD phase gates

Sized by SUDOKU-W6 conventions. Each phase ships **red-first**, gates **green + domain-swap + no-regression**.

| Phase | Surface | Tests | Depends on |
|---|---|---|---|
| **PE** (PATTERN_EXPLAIN) | per-WEIGHT-term contribution decomposition | PE1–PE6 (lit, field, add, mul, min-clipped, full SCJ scorer) + 4 domain-swap | v0.1 WeightExpr |
| **PP** (pattern_preflight) | field-statistic + holonomy preflight | PP1–PP5 (impossible range, missing categorical, internally contradictory, holonomy non-trivial, ok happy path) + 4 domain-swap | v0.1 Predicate parsing |
| **K_P** (pattern curvature) | per-bundle scalar | K1–K4 (concentrated, dispersed, empty-match, with-K-override) + 4 domain-swap | `BundleStats` kNN index |
| **VT** (verdict trichotomy) | sat / unsat / near_miss | VT1–VT5 (sat path, unsat-by-preflight, unsat-by-scan, near-miss 1-flip, near-miss 2-flip) + 4 domain-swap | Phases PE, PP |
| **PR** (PATTERN_REPAIR menu) | ordered min-cost flip menu | PR1–PR6 (single-flip, double-flip, with relaxation costs, cap at top 5, already-matches edge case, too-far edge case) + 4 domain-swap | Phase VT |

**Sequencing rationale**: PE is the cheapest and unlocks the runtime debugger for everything else. PP makes HUNT faster (avoids scan). K_P unlocks the operator self-check (am I over/under-fit?). VT is the API change (verdict field). PR is the consumer-facing feature (the repair menu, the near-miss explanation).

Suggested order to ship: **PE → PP → VT → PR → K_P**. PE and PP are independently useful and ship in any order. K_P can wait until the others are landed because it's an opt-in.

Each phase follows the gated-TDD discipline established in the v0.1 work:

1. **Gate 1 (red)**: write the test file, confirm fails for the right reason
2. **Gate 2 (green)**: implement minimally, confirm test passes
3. **Gate 3 (regression)**: full `cargo test --features patterns --tests` + `cargo test --tests` no-feature, both green
4. **Gate 4 (domain-swap)**: same primitive runs identically across 4 domain-named test variants
5. **Gate 5 (commit + cherry-pick)**: on `main` and `scj-v0.1-substrate`

---

## §11. What this does NOT do

To stay scoped:

- **No new geometry.** All math is the SUDOKU machinery already in `src/geometry/sudoku.rs`. We're wiring it through Patterns, not re-deriving it.
- **No new HTTP endpoints created** for any primitive that fits inside HUNT. `PATTERN_REPAIR` and `PATTERN_EXPLAIN` ride as HUNT-response fields when their flags are set. The standalone endpoints (`/curvature`, `/preflight`, `/explain`) are convenience wrappers, not parallel surfaces.
- **No grammar additions to WEIGHT.** v0.2's WEIGHT is byte-identical to v0.1.
- **No persistence change.** Pattern registry stays in-memory (Phase 6 graduation is its own future ship).
- **No sharded support yet.** Phase 5 sharded HUNT comes first; sharded verdict is a Phase 5 follow-up.

---

## §12. Versioning

- **v0.1** = currently live. HUNT returns rows.
- **v0.2** = this spec. HUNT returns rows + verdict + (optionally) explain + repair menu.
- **v0.3** = variadic min/max, CLASSIFY-in-WEIGHT with expression THEN-branch, scope-attribute on DEFINE PATTERN. (Already on the OQ ledger from SCJ Round 10.)
- **v0.4+** = LOAD PATTERNS FROM, sharded HUNT, graduation off the feature flag.

v0.2 is the *next* ship. It's the biggest single jump in operator-facing capability without changing any of the underlying math.

---

— Spec authored 2026-06-09 (Gigi engine team · Davis Geometric)
— Pattern math companion: `theory/patterns/validation_tests.py`
— Sprint plan: `theory/patterns/IMPLEMENTATION_PLAN.md`
