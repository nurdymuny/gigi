# GIGI Sheets — Formulas Spec (v3.1)

**Status:** spec for review · triple-verified against the published
math. The formula bar component + minimal evaluator already ship (see
[`src/lib/formula.ts`](src/lib/formula.ts) +
[`src/components/FormulaBar.tsx`](src/components/FormulaBar.tsx)). This
spec is the contract for the next-level surface.

**v3.1 changelog (paper verification):** identity verified verbatim
against three papers — *The Double Cover Principle* (Theorem 3.1,
Pythagorean Bridge), *Zero Does Not Exist* (§2.4, double-cover
constraint becomes genuine at n ≥ 2), and *The Davis Duality of
Approximation and Obstruction* (Davis invariant C = τ/K). Added
paper-derived vocabulary (S_Q "quadratic sameness", d² "deviation",
Davis Law) and a note that the engine's per-row κ is the chord
K = 2 sin(θ/2), not d² — they differ by K² = 4·d².

**v3 changelog (review pass 2):**
- Identifier disambiguation rule — reserved function names always win
  against field names; collision recoverable via A1 notation.
- Out-of-bounds row indexing (`temperature[1000]` on 50 rows) → `#REF!`.
- Added `SUMIFS` / `COUNTIFS` / `AVERAGEIFS` siblings to match
  `MINIFS` / `MAXIFS`.
- Predicate comparisons specified for numeric vs text/categorical
  ranges. `>M` on a numeric column → `#VALUE!`; `>M` on text →
  lexicographic.
- `DATEDIF` units divergence from Excel called out explicitly —
  uppercase units return `#VALUE!`.
- `TO_DATE` epoch heuristic: 10 digits = seconds, 13 = ms.
- Stats functions: mixed-type behavior matches `MIN`/`MAX` (skip
  non-numeric); `STDEV`/`VAR` return `#DIV0!` when n < 2.
- `PERCENTILE` interpolation matches Excel's `PERCENTILE.INC`.
- Rank tie-breaking wording fixed; explicit "bit-identical Float32"
  rule; fuzz-tolerance deferred to v2 `RANK(ref, ε)` form.
- `CONCAT` moved into Phase 1 alongside `&` (same operation).
- `COHORT(ref)` returns `""` when no cover field is set (not a
  sentinel string that could collide with a user's cohort label).
- Davis-identity integration test gets explicit self-comparison cases.

**v2 changelog (review pass 1):**
- 🔴 **Math correction.** The Davis identity is `S + d² = 1` with
  `d = √(1 − S) = sin(θ/2)`, **not** `S² + d² = 1`. v1 had the wrong
  identity baked in. Implementation, tests, and landing page have been
  updated.
- A1 references are now bundle-order absolute (not visible-order),
  with named-field refs (`temperature`) as the recommended idiom.
- Cell refs are stable against sort / filter — they always identify
  the same bundle row regardless of how the grid is sorted or filtered.
- `%` is postfix only (`MOD(a, b)` is a function).
- `SAME(a, b)` semantics clarified — row-of-a vs row-of-b.
- `COHORT(ref)` takes a cell ref, not a column name.
- View-dependence of ranges acknowledged + a separate `_ABS` family
  punted to v2.
- Stats functions (`MEDIAN`, `STDEV`, `VAR`) promoted into Phase 1.
- `SUMIF` / `COUNTIF` / `AVERAGEIF` family **included** with a tiny
  predicate mini-language.
- Date math added to the near roadmap as Phase 1.5.
- Iterative calculation, named ranges, and array semantics added as
  open questions.

## Goals

1. **Honest parity** — every formula the landing-page comparison row
   advertises (`=SAME`, `=K`, `=DIST`, `=COHORT`, plus an 80/20 set:
   `SUM`, `AVG`/`AVERAGE`, `MIN`, `MAX`, `COUNT`, `IF`, plus the stats
   functions a data tool needs) actually works against real grid data.
2. **GIGI-native** — formulas can ask geometric questions (sameness,
   curvature, cohort) that Excel can't express.
3. **Database-respectful** — formula cells are an overlay on the
   bundle, never silently mutating engine rows. Persistence is
   bundle-side and explicit.
4. **No surprise rewrites** — Davis identity (`S + d² = 1`) holds for
   every `=SAME` / `=DIST` pair, by construction.
5. **No semantic drift under UI changes** — sorting, filtering, or
   hiding a column never rewrites the meaning of an existing formula.

## Non-goals

- Full Excel function library (500+ functions). Targeted 80/20 set +
  stats + GIGI primitives is the cap.
- Multi-sheet references (`Sheet2!A1`). Bundles are single-namespace.
- Volatile functions (`=NOW()`, `=RAND()`) — every formula evaluates
  deterministically from its inputs at the bundle level.
- User-defined functions / macros. Out of scope for v1.
- Array formulas (`{=SUM(A1:A10 * B1:B10)}`). Defer; `SUMIF` family +
  `SUMPRODUCT` cover most needs.

---

## Math: the Davis identity

The single non-negotiable invariant the whole evaluator must honor:

```
S + d² = 1
```

Where:

- `S = (1 + cos θ)/2 = cos²(θ/2)` — sameness ∈ [0, 1]
- `d = sin(θ/2) = √(1 − S)` — Davis distance ∈ [0, 1]
- `d² = sin²(θ/2) = 1 − S` — **deviation** (the Zero-paper §2.4 name
  for the loss-of-sameness quantity)

This is the half-angle Pythagorean identity. The implementation
guarantees this by deriving `d` from `S` inline — there's no separate
distance computation that could drift from sameness. The load-bearing
test in [`tests/unit/davis.test.ts`](tests/unit/davis.test.ts) asserts
`|S + d² − 1| < 1e-6` over 1000 random unit-vector pairs.

**Sameness class.** We use the **quadratic sameness** S_Q = cos²(θ/2),
which is *Theorem 4.4 (Quadratic sameness)* in the Double Cover paper
— the class associated with Fubini-Study geometry on CP¹ (the Bloch
sphere). The alternative **arcsin sameness** S_A = 1 − (2/π)·arcsin(K/2)
exists (Theorem 4.7 of the same paper); they cross at θ = π/2 and we
deliberately pick S_Q for two reasons:
1. **Quadratic protection at the identity** — `|dS/dθ| → 0` as θ → 0,
   so small embedding noise barely moves sameness near the dedup
   threshold.
2. **It falls out of the dot product.** Any inner-product embedding
   gives `cosθ = a · b` for unit vectors, so `S_Q = (1 + cosθ)/2` is
   the natural form.

**Vacuous at n = 1.** Per the Zero-paper §2.4, at n = 1 (one row, or
self-comparison) `S = 1` and `d² = 0` hold trivially — the identity
is satisfied vacuously, no geometric content. This validates the
launch-gate requirement that `=SAME(A1, A1) === 1` and
`=DIST(A1, A1) === 0` exactly. The identity becomes a genuine
constraint only at n ≥ 2.

**Implication for the formula evaluator.** `=SAME(a, b)` returns S
directly. `=DIST(a, b)` returns `√(1 − S)` — never computed as
`√(1 − S²)`. The identity then holds **by construction** in every cell
of every bundle. This matches the published Davis Double Cover
Principle (Theorem 3.1, Pythagorean Bridge).

**Vocabulary note: the engine's per-row κ is NOT d².** The engine
returns a κ value per row (see [`src/lib/kappa.ts`](src/lib/kappa.ts),
thresholded at warn = 0.8 / bad = 2.0). That κ is the **chord
K = |1 − e^iθ| = 2 sin(θ/2)** from the Double Cover paper, bounded
[0, 2]. The relationship to d² is `K² = 4·d²`. The formula bar's
`=K(ref)` returns the engine's κ (chord K), not the deviation d².
Don't conflate them. (The internal `davis.deviation(a, b)` function
returns `1 − S = d²` — bounded [0, 1] — which IS the deviation.)

---

## Surface

### Where formulas live

Two surfaces, both backed by the same evaluator:

1. **The formula bar** — top-of-grid `fx` strip. Typing here previews
   live evaluation in the right-hand panel. Pressing Enter commits the
   formula to the **active cell** (the focused cell from the grid).
   Already shipped; this spec finishes the cell-ref surface.
2. **Inline cell formula** — type `=` into a cell editor and the cell
   becomes a formula cell. The cell renders the evaluated result;
   clicking it shows the formula in the formula bar.

### Reference notation

GIGI Sheets supports two reference styles. **Named-field refs are the
recommended idiom**; A1 is there for Excel-mental-model parity.

#### Named-field references (recommended)

The natural form for a column-oriented database:

```
temperature              → the temperature column (all rows)
temperature[5]           → row 5's temperature value
temperature[*]           → the full column (alias for `temperature`)
amount_usd, fee_usd      → cross-column refs in aggregates
```

This is GIGI-native and **immune to column reordering or visibility
changes**. Lean into bundles' named fields.

##### Identifier disambiguation rule

A bare identifier (no `[…]`, no `(`) in a formula resolves as follows:

1. If the identifier is a **reserved function name** (everything in the
   Required / Stats / Conditional / Strings / Dates / GIGI tables
   below), it is **always** a function. `=MEDIAN` alone is `#NAME!`
   (function called without parentheses), not a field ref. A bundle
   that happens to have a field literally named `median` must access
   it via A1 notation (e.g. `=B5`).
2. Otherwise, the identifier is resolved against the schema. Unknown
   identifier → `#NAME!`.

Reserved names are case-insensitive on the parser side (`MEDIAN` ===
`median` === `Median` as function name) but field names match
**exactly**. So a field called `Temperature` is *not* matched by
`temperature` in a formula — pick a casing convention and stick with it.

##### Out-of-bounds row indexing

`temperature[1000]` on a 50-row bundle returns `#REF!`. Same for
negative or zero indices (rows are 1-based; `temperature[0]` and
`temperature[-3]` are both `#REF!`).

#### A1 references (Excel-style)

```
A1     → column index 0, row 1 (1-based row, 0-based column letter)
B12    → column index 1, row 12
AA1    → column index 26, row 1
```

A1 maps to **schema order**, not visible order. Hidden columns are
still referenceable. Hiding a column does not rewrite any formula.

**Sort and filter do not affect A1 row indices.** Row 1 is always the
first row in *bundle order* (the engine's insertion order or whatever
canonical sort the engine returns). When the grid is sorted by κ, the
formula bar may show `=A3` for what the user sees as the 1st visible
row — that's a feature, not a bug. The grid renders a small
"row → A-number" indicator next to the active cell when display
position ≠ bundle position.

#### Range notation

```
temperature[1:10]   → rows 1-10 of the temperature column
temperature         → full column (alias for `temperature[1:N]`)
A1:A10              → vertical range (same column letter)
A1:C1               → horizontal range (same row)
A1:C10              → rectangular range
```

Full-column shorthands `A:A` and full-row `1:1` are **deferred**.
Their view-dependence on hidden columns and filters is the kind of
footgun the rest of this spec is designed to avoid. Until v2, use
`temperature` (named) or `A1:A1000` (bounded).

Ranges only appear inside function call arguments — you can't bind
`B1 = A1:A10` (that'd produce a range, not a scalar).

---

## Type system

Three formula value types. Null inputs are treated as `0` for
arithmetic and `""` for text (Excel convention).

| Type | Examples | Operators |
|---|---|---|
| number | `1`, `1.5`, `-3`, `1e6` | `+ − × ÷ ^ %` |
| string | `"hello"`, `"a quote: ""b"""` | `&` (concat) |
| boolean | `=A1 > 0` returns true/false | `= < > <= >= <>` |

**String literal escaping.** Double the quote: `"she said ""hi"""` →
`she said "hi"`. Matches Excel.

Mixed-type arithmetic coerces toward numeric. `"3" + 4` is `7`. If
coercion fails, the cell shows `#VALUE!`.

---

## Operators

| Op | Precedence | Notes |
|---|---|---|
| `%` | 8 (postfix only) | `5%` → `0.05`. **Not modulo.** Modulo is the function `MOD(a, b)`. |
| Unary `-` | 8 | |
| `^` | 7 | exponent, right-assoc |
| `*` `/` | 6 | |
| `+` `-` | 5 | binary |
| `&` | 4 | string concat |
| `=` `<>` `<` `<=` `>` `>=` | 3 | comparisons → bool |

Parentheses override precedence.

---

## Function library

### Required (the 80/20 set)

| Fn | Args | Returns | Notes |
|---|---|---|---|
| `SUM` | range / args | number | nulls → 0 |
| `AVG` / `AVERAGE` | range / args | number | `#DIV0!` on empty |
| `MIN` | range / args | number | ignores non-numeric |
| `MAX` | range / args | number | ignores non-numeric |
| `COUNT` | range / args | number | counts numeric cells only |
| `COUNTA` | range / args | number | counts non-empty cells |
| `IF` | `(cond, then, else?)` | any | `else` defaults to `false` |
| `MOD` | `(a, b)` | number | `a mod b`; `#DIV0!` when `b = 0` |
| `ROUND` | `(n, digits?)` | number | **round-half-away-from-zero** (Excel parity, not `Math.round`) |
| `ABS` | `n` | number | |
| `CONCAT` | args | string | function form of `&`; ships in Phase 1 alongside the operator since they're the same op |

### Stats (Phase 1 — promoted from "defer")

The functions a data tool actually needs day-one:

| Fn | Args | Returns | Notes |
|---|---|---|---|
| `MEDIAN` | range / args | number | midpoint; mean of the two middles for even N |
| `STDEV` | range / args | number | **sample** standard deviation (`n − 1` denominator); `#DIV0!` when n < 2 |
| `STDEVP` | range / args | number | population stdev (`n` denominator); `#DIV0!` when n = 0 |
| `VAR` | range / args | number | sample variance; `#DIV0!` when n < 2 |
| `VARP` | range / args | number | population variance; `#DIV0!` when n = 0 |
| `PERCENTILE` | `(range, k)` | number | `k ∈ [0, 1]`, inclusive endpoints (matches Excel `PERCENTILE.INC`); linear interp between bracketing samples |
| `QUARTILE` | `(range, q)` | number | `q ∈ {0,1,2,3,4}`; equivalent to `PERCENTILE(range, q/4)` |

**Mixed-type behavior.** All stats functions match `MIN`/`MAX`: silently
**skip non-numeric values** (nulls, strings, booleans). Empty ranges
or ranges with zero numerics return `#DIV0!`. This way a stray text
cell in an otherwise-numeric column doesn't poison every aggregate
above it.

### Conditional aggregates (Phase 1.5 — micro-predicate language)

Single-criterion forms:

| Fn | Args | Returns | Notes |
|---|---|---|---|
| `SUMIF` | `(range, predicate)` | number | sum where predicate matches |
| `COUNTIF` | `(range, predicate)` | number | count where predicate matches |
| `AVERAGEIF` | `(range, predicate)` | number | avg where predicate matches |

Multi-criterion `*IFS` family — every criterion AND'd together:

| Fn | Args | Returns | Notes |
|---|---|---|---|
| `SUMIFS` | `(sum_range, criteria_range1, pred1, …)` | number | sum where every `(range_i, pred_i)` matches |
| `COUNTIFS` | `(criteria_range1, pred1, …)` | number | count where every pair matches (no separate sum range) |
| `AVERAGEIFS` | `(avg_range, criteria_range1, pred1, …)` | number | avg of `avg_range` where every pair matches |
| `MINIFS` | `(min_range, criteria_range1, pred1, …)` | number | min of `min_range` where every pair matches |
| `MAXIFS` | `(max_range, criteria_range1, pred1, …)` | number | max of `max_range` where every pair matches |

**Predicate grammar** (intentionally tiny):

| Form | Example | Means |
|---|---|---|
| `">N"` | `">5"` | greater than N (numeric) or lexicographically greater than (text) |
| `"<N"` | `"<5"` | less than N |
| `">=N"` `"<=N"` | `">=5"` | as expected |
| `"=N"` or `N` | `"=5"` or `5` | equality |
| `"<>N"` | `"<>5"` | not equal |
| `"value"` | `"warn"` | string equality (case-insensitive) |
| `"prefix*"` | `"INV*"` | starts-with (single trailing `*`) |
| `"*suffix"` | `"*payment"` | ends-with (single leading `*`) |

That's it. No regexes, no compound predicates with AND/OR. Compound
matching uses the `*IFS` family with multiple `(range, predicate)`
pairs.

**Predicate vs column type — comparison semantics.**

- If the criteria range is **numeric**, the predicate operand must
  coerce to a number. `>5`, `>=5.5`, `<0` all work. `>M` on a numeric
  range → `#VALUE!` (the operand failed to coerce).
- If the criteria range is **text** or **categorical**, comparison is
  **lexicographic**. `>M` matches values that sort after "M".
- If the range is **mixed-type** (rare but possible — null values in
  a numeric column, for example), null values never match any
  predicate other than explicit `"="` against empty/null.
- Equality on strings is case-insensitive; ordering comparisons use
  the default JS locale-aware sort (`String.localeCompare`).

This grammar is **~50 lines of code** and unlocks the "aggregate over
filtered subset, as a live formula" idiom.

### Strings (Phase 1.5 — moved out of Phase 1)

`LEN`, `LOWER`, `UPPER`, `TRIM` ship in Phase 1.5. They're useful but
rarely the first thing a data analyst reaches for. Stats beat strings
on priority. (`CONCAT` shipped in Phase 1 alongside the `&` operator.)

### Date math (Phase 1.5)

| Fn | Args | Returns | Notes |
|---|---|---|---|
| `YEAR` / `MONTH` / `DAY` | timestamp | number | UTC components |
| `DATEDIF` | `(start, end, unit)` | number | unit ∈ {`"d"`,`"w"`,`"m"`,`"y"`} |
| `TODAY` | — | timestamp | **deterministic per bundle load**; not volatile |
| `TO_DATE` | string / number | timestamp | ISO 8601, RFC 3339, or epoch ms/seconds (see below) |

Date arithmetic: `timestamp + N` adds N days. `timestamp - timestamp`
returns days. Same as Excel's serial-day model.

**`DATEDIF` unit divergence from Excel.** GIGI uses lowercase
single-letter units (`"d"`, `"w"`, `"m"`, `"y"`); Excel uses
uppercase plus combined units (`"Y"`, `"M"`, `"D"`, `"MD"`, `"YM"`,
`"YD"`). We deliberately don't accept Excel's set — uppercase units
return `#VALUE!` instead of silently doing the wrong thing. If you
paste in an Excel formula with `"Y"`, fix it explicitly.

**`TO_DATE` epoch heuristic.** When the input is numeric:
- 10 digits → interpreted as **epoch seconds** (Unix convention; most
  non-JS environments default to seconds)
- 13 digits → **epoch milliseconds** (JS `Date.now()` convention)
- Anything else → `#VALUE!`

To force one or the other, multiply: `TO_DATE(epoch_s * 1000)` or
`TO_DATE(epoch_ms / 1000)`.

### GIGI primitives (the differentiator)

| Fn | Args | Returns | Math |
|---|---|---|---|
| `SAME` | `(a, b)` | number in [0, 1] | Davis sameness `S = (1 + cosθ)/2` of the **rows** containing `a` and `b` |
| `DIST` | `(a, b)` | number in [0, 1] | `√(1 − S)` derived from `SAME` inline — `SAME + DIST² = 1` exactly |
| `K` | `(ref)` | number ≥ 0 | curvature κ of the row containing `ref` |
| `COHORT` | `(ref)` | string | cohort label for the row containing `ref` |
| `KAPPA_RANK` | `(ref)` | number | 1-based **dense rank** among bundle rows by κ desc |
| `SAMENESS_RANK` | `(pivot_ref, ref)` | number | 1-based dense rank among bundle rows by S to `pivot_ref` desc |

**`SAME(a, b)` clarified.** Both arguments are cell references; only
their **rows** matter. `=SAME(A1, B1)` is `S(row_1, row_1) = 1`
because the rows are the same. `=SAME(A1, A2)` is `S(row_1, row_2)`,
which is what users actually want. The columns are decorative — they
let you pick any cell on the row as a "handle." A future `SAME_ROW(i, j)`
sigil-free form may land if the redundancy becomes a real complaint.

**`COHORT(ref)` clarified.** Returns the cohort label of the row
containing `ref`, derived from the bundle's current cover field. Same
shape as `K(ref)`. Returns the **empty string** `""` when no cover
field is set — *not* a sentinel string like `"all"`, because the user
might legitimately have a cohort literally named `"all"`. To
distinguish "no cover field set" from "cohort with empty label," wrap
in `LEN(COHORT(A1))=0` (the former) vs `COHORT(A1)=""` *and* a non-null
cover-field schema check.

**Davis-identity guarantee.** `DIST` is derived from `SAME` in the
evaluator (`√(1 − SAME)`), not computed independently. So for any two
cells `=SAME(a,b) + =DIST(a,b)^2 = 1` exactly (within float ε). This
is enforced by [`tests/unit/davis.test.ts`](tests/unit/davis.test.ts)
and the formula-bar evaluator test.

### Rank tie-breaking

Both `KAPPA_RANK` and `SAMENESS_RANK` use **dense rank**: equal values
share a rank; the next distinct value gets rank N+1 (not N+k where k
is the cluster size).

"Equal" here means **bit-identical Float32**. The rank function doesn't
fuzz-equate near-but-not-equal floats. If two rows' κ values differ by
1e-15 they get different ranks, even though that difference is
numerically meaningless. A future `KAPPA_RANK(ref, ε)` form could
cluster within an explicit tolerance; deferred to v2.

---

## Determinism + view dependence

A user-visible rule:

> **Formula results depend on the bundle's data — not the grid's
> current view.** Sorting, filtering, hiding columns, or scrolling
> never changes a formula's output.

This is a deliberate cut. Excel formulas depend on the cell grid
because Excel has no "underlying data" — the grid *is* the data. GIGI
has a bundle underneath; the grid is a view. So:

- `=SUM(temperature)` sums the full bundle column, not just visible
  rows. If you want the visible-rows-only aggregate, use the selection
  stats panel (already in the formula bar when a range is selected).
- `=KAPPA_RANK(A1)` ranks across the full bundle, not the visible set.
- `=SUMIF(amount_usd, ">100")` filters by the predicate, not by what
  the grid filter is currently doing.

A future `*_VIEW` family (`SUM_VIEW`, `COUNT_VIEW`) could expose
visible-only aggregates as an explicit opt-in. Tracked as a v2 open
question.

---

## Error sentinels

Errors are first-class values that **poison** any expression they
appear in — operators, aggregates, anything.

| Sentinel | When |
|---|---|
| `#ERROR!` | syntax error |
| `#NAME!` | unknown function name |
| `#REF!` | cell ref points outside the bundle |
| `#DIV0!` | divide-by-zero, empty AVG/MIN/MAX, MOD by 0 |
| `#VALUE!` | type coercion failure |
| `#CIRC!` | circular reference detected at evaluation time |

**Error propagation in aggregates.** If `A5` is `#REF!`, then
`=SUM(A1:A10)` returns `#REF!`. Aggregates poison. Matches Excel.

Error cells render with a small red badge in the grid. Hovering shows
the underlying message.

---

## Recompute model

### Dependency tracking

Each formula has a dependency set: the cells + ranges it reads. When
any source cell changes (engine update, inline edit), the dependent
formulas re-evaluate.

For v1, the dependency graph is computed at parse time and stored
alongside the formula cell. No incremental graph updates — when a
formula changes, its old dependencies are removed and the new set
inserted.

### Circular reference detection

At evaluation time, the evaluator maintains a stack of cells currently
being computed. If a formula references a cell already in the stack,
the chain is broken with `#CIRC!` propagating to every cell in the
cycle.

**Iterative calculation is a v2 open question.** Excel has an opt-in
mode that converges circular refs to a fixpoint over N iterations.
For geometric formulas (e.g. `κ` defined in terms of neighbors' `κ`),
this is genuinely useful. Defer to a v2 explicit-opt-in toggle.

### Evaluation order

Topological order of the dependency graph. If multiple cells are
independent, evaluation order is undefined — but determinism is
guaranteed because no GIGI function is volatile (`TODAY` is
bundle-load-time deterministic, not call-time).

---

## Persistence

Formula cells are an **overlay on top of the bundle**, not engine rows.

- The bundle row stores the *displayed value* (the evaluated result),
  so SDK consumers, GQL queries, and the engine all see the resolved
  number/string. They never need to know a formula exists.
- The *formula text* itself lives in a sidecar map keyed by
  `(bundleName, rowKey, fieldName)`, persisted to `localStorage` for
  v1 and to the engine's view storage in v2.
- On every recompute, the engine row is updated via
  `client.update(...)` with the new value. The formula text stays in
  the sidecar.

**Implication for general-purpose DB use.** Pure-data clients see only
the resolved numbers. Sheets-specific tooling reads both. A round-trip
through the GQL console returns plain values — re-importing into a
new Sheets session reattaches the formulas via the sidecar.

---

## Selection-aware formula bar

When a cell is selected, the formula bar shows that cell's formula
(if any) or its raw value. Editing the formula bar and pressing Enter
commits to the **selected** cell — same shortcut Excel uses.

If a range is selected, the formula bar shows the aggregate stats from
[`selectionStats`](src/lib/selection.ts) — count, sum, mean — rather
than a single formula. Typing here is disabled until a single cell is
selected (matches Excel).

---

## Implementation plan

Four phases. Each delivers an internal-coherent step.

### Phase 1 — single-cell formulas + stats (≈ 1.5 days)

- Extend [`src/lib/formula.ts`](src/lib/formula.ts) evaluator to:
  - Multi-letter A1 column refs (currently parses `A1` only)
  - String literals with `""` escape + `&` operator
  - Power (`^`), comparison operators, postfix `%`
  - Custom Excel-style `ROUND` (round-half-away-from-zero, not `Math.round`)
  - Error propagation through operators **and aggregates**
- Add **named-field refs** (`temperature`, `temperature[5]`) —
  resolved against the schema at parse time
- Add `MEDIAN`, `STDEV`, `STDEVP`, `VAR`, `VARP`, `PERCENTILE`,
  `QUARTILE`, `MOD`, `ABS`
- Inline cell formula: `Cell` detects a leading `=` and renders the
  evaluated result; clicking puts it in edit mode with the raw text
- Sidecar storage helper in `lib/formula-storage.ts`
- Tests: every operator (including `%` vs `MOD` disambiguation), every
  function, all error sentinels (including aggregate poisoning), the
  Davis identity over 100 random row pairs in a real demo bundle

### Phase 1.5 — `*IF` family + strings + dates (≈ 1 day)

- `SUMIF` / `COUNTIF` / `AVERAGEIF` / `MINIFS` / `MAXIFS` with the
  micro-predicate language above
- `LEN`, `LOWER`, `UPPER`, `TRIM`, `CONCAT`
- `YEAR`, `MONTH`, `DAY`, `DATEDIF`, `TO_DATE`, `TODAY`
- Tests: predicate parser unit tests, every string fn, every date fn

### Phase 2 — ranges + dependency graph (≈ 1 day)

- Bounded ranges (`A1:A10`, `temperature[1:10]`) resolved at evaluate-time
- Dependency parse → per-cell `Set<CellRef>`
- Topological recompute when a source cell changes
- `#CIRC!` detection
- Tests: range sums, recompute cascades, circular ref detection

### Phase 3 — GIGI primitives wired to embedder (≈ 1 day)

- Today's `SAME` returns a stub. Wire it to the real
  [`prism-workflows.embedRow`](src/lib/prism-workflows.ts) +
  [`davis.sameness`](src/lib/davis.ts) so the geometry is real.
- `K(ref)` already reads from `kappaMap` — verified by an integration
  test on the demo bundles.
- `COHORT(ref)` returns the row's cohort name (uses the kappa.ts
  cohort logic).
- Wire `KAPPA_RANK` and `SAMENESS_RANK` with dense-rank tie-breaking.
- Integration test: pick 100 random row pairs from a loaded demo
  bundle; assert `=SAME(A_i, A_j) + =DIST(A_i, A_j)^2 = 1` within
  1e-6 for all of them. Include explicit self-comparison cases
  (`=SAME(A1, A1) === 1` and `=DIST(A1, A1) === 0` exactly — the
  degenerate case is the easiest place for a subtle bug to hide).

### Phase 4 — UX polish (≈ 0.5 day)

- Formula bar shows the selected cell's formula
- Range selection swaps the bar for aggregate stats
- Error badge + tooltip in the grid
- `Tab` from formula bar moves the active cell right; `Enter` moves down
- Display-row vs bundle-row indicator when grid is sorted/filtered
- Documentation page in `docs/`

**Total: ~5 days for full Phase 1-4.** Up from v1's ~3.5 days because
the stats functions + `*IF` family expanded scope. Worth it.

---

## TDD plan

Every phase is test-first. The non-negotiable invariant is the
**Davis identity test** in
[`tests/unit/davis.test.ts`](tests/unit/davis.test.ts), now asserting
`|S + d² − 1| < 1e-6` over 1000 random pairs.

Per-phase tests:

**Phase 1**
- `tests/unit/formula.operators.test.ts` — every operator including
  `%` postfix vs `MOD` function disambiguation, string literal escape
- `tests/unit/formula.functions.test.ts` — every function from the
  base + stats tables (incl. `CONCAT` parity with `&`)
- `tests/unit/formula.round.test.ts` — explicit Excel-parity rounding
  cases (`ROUND(-0.5)` = -1, etc.)
- `tests/unit/formula.errors.test.ts` — every sentinel + aggregate
  poisoning
- `tests/unit/formula.named-refs.test.ts` — `temperature`,
  `temperature[5]`, schema resolution, **identifier collision rule**
  (reserved name beats field), **out-of-bounds row indexing** returns
  `#REF!`, zero and negative indices return `#REF!`
- `tests/unit/formula.stats.test.ts` — `STDEV(x)` on n=1 returns
  `#DIV0!`; `MEDIAN` over mixed types skips non-numeric;
  `PERCENTILE(range, 0.5) === MEDIAN(range)` (the inclusive convention)
- `tests/component/Grid.formula-cell.test.tsx` — inline formula renders
  evaluated result; click puts editor in raw-text mode

**Phase 1.5**
- `tests/unit/formula.predicate.test.ts` — the micro-predicate grammar
- `tests/unit/formula.sumif.test.ts` — `SUMIF`, `COUNTIF`, `AVERAGEIF`,
  `SUMIFS`, multi-predicate
- `tests/unit/formula.strings.test.ts` — `LEN`, `LOWER`, `UPPER`,
  `TRIM`, `CONCAT`
- `tests/unit/formula.dates.test.ts` — `YEAR`, `MONTH`, `DAY`,
  `DATEDIF`, `TO_DATE`, `TODAY` (deterministic per load)

**Phase 2**
- `tests/unit/formula.ranges.test.ts` — `A1:A10`, named ranges
- `tests/unit/formula.dependencies.test.ts` — graph build + recompute
- `tests/unit/formula.circular.test.ts` — `#CIRC!` propagation

**Phase 3**
- `tests/unit/formula.gigi.test.ts` — `SAME`, `DIST`, `K`, `COHORT`
- `tests/unit/formula.ranks.test.ts` — `KAPPA_RANK`, `SAMENESS_RANK`,
  dense-rank tie-breaking
- `tests/integration/davis-identity-via-formulas.test.ts` — 100-pair
  integration test on the Iris + payments demo

**Phase 4**
- `tests/component/FormulaBar.selection.test.tsx` — bar reflects
  selected cell's formula
- `tests/component/FormulaBar.range-stats.test.tsx` — bar shows
  aggregates on range selection
- `tests/component/Grid.row-position-indicator.test.tsx` — bundle-row
  marker visible when grid is sorted

---

## What's still NOT in scope, even after Phase 4

- **`SUMIFS` with complex predicate compounds** beyond AND-of-pairs
- **Sheet-to-sheet references** — bundles are single-namespace
- **Volatile functions** (`NOW`, `RAND`, `RANDBETWEEN`)
- **`INDIRECT` / `OFFSET`** — break static dependency tracking
- **Array formulas** (`{=SUM(A1:A10 * B1:B10)}`)
- **Custom functions / macros**
- **Iterative calculation** (deferred to v2 explicit opt-in)
- **Named ranges in a Name Manager UI** — named-field refs replace
  most of the use case; named ranges over arbitrary expressions defer
  to v2

---

## Open questions for sign-off

1. **Persistence boundary.** Sidecar in `localStorage` for v1 vs.
   engine-side view storage. Recommendation: localStorage. Why: zero
   engine change, ships immediately. Cost: formulas don't sync across
   devices yet. The view-storage migration is one helper-swap.

2. **Error UI weight.** Compact red badges in the cell, or full red
   backgrounds? Recommendation: badge only.

3. **Formula vs Sheets-view-spec.** A view today persists
   sort/filter/cover. Should it also capture formula overlays?
   Recommendation: yes — "Stalled tasks view" can include
   `=K(A1)` as a derived column.

4. **GIGI primitives over a range.** `=SAME(A1, A2:A100)` could return
   a column of sameness values, but that's an array formula. Ship
   aggregates instead (`MAX_SAME`, `MIN_DIST`, `MEAN_K`) in v2.

5. **Iterative calculation.** Useful for geometric fixpoints (κ
   defined in terms of neighbors' κ). Defer to v2 explicit-opt-in.
   But: name it now in the docs so users know it's coming.

6. **Named ranges.** Excel's Name Manager allows `=SUM(MyRange)` where
   `MyRange = $A$1:$A$100`. Our named-field refs replace 80% of this
   use case. The remaining 20% (named expressions, not just columns)
   defers to v2.

7. **View-only aggregates.** A `SUM_VIEW(temperature)` family that
   honors the current grid filter. Defer to v2; the conceptual cost
   (engine state depending on UI state) is real.

8. **A1 sigil ambiguity.** `temperature` is a column ref;
   `temperature5` is — also a column ref? Or temperature for row 5?
   Recommendation: brackets are required for row indexing
   (`temperature[5]`), so `temperature5` is just a different (probably
   non-existent) column name. Strict; predictable.

---

If approved, I start with Phase 1 — single-cell formulas + named-field
refs + stats. The evaluator already handles a meaningful subset; Phase
1 finishes the operator set, ships the stats, and adds the inline-cell
affordance.

---

## Phase 1 launch gate (sign-off requirements)

Before Phase 1 lands in `main`, every one of these must be true:

1. **Math invariant green.** `tests/unit/davis.test.ts` passes with
   `|S + d² − 1| < 1e-6` over 1000 pairs **and** the explicit
   self-comparison case `=SAME(A1, A1) === 1` exactly,
   `=DIST(A1, A1) === 0` exactly.
2. **Identifier collision rule covered.** Test confirms a bundle with
   a field literally named `median` does **not** match `=MEDIAN()`;
   the function wins.
3. **Out-of-bounds named-ref test.** `temperature[1000]` on a small
   bundle returns `#REF!`.
4. **`%` vs `MOD` disambiguation.** `=5%` is `0.05`; `=MOD(5, 2)` is
   `1`; `=5 % 2` (binary) is `#ERROR!`.
5. **Aggregate poisoning test.** `=SUM(A1:A10)` where one cell is
   `#REF!` returns `#REF!`, not partial sum.
6. **`STDEV` single-value test.** `=STDEV(A1)` (one cell) returns
   `#DIV0!`; `=STDEVP(A1)` returns 0.
7. **Excel-parity `ROUND` test.** `=ROUND(-0.5)` is `-1`,
   `=ROUND(0.5)` is `1`, `=ROUND(2.5)` is `3` — round-half-away,
   not `Math.round`.
8. **Davis-identity hold via formulas.** Integration test on a real
   demo bundle picks 100 random row pairs and asserts the identity.

Anything left undone bounces Phase 1 back into review. The math
invariant is non-negotiable — it's the load-bearing rule the rest of
the engine depends on.
