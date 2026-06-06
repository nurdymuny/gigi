# PATTERN_HUNT — named, weighted, anti-joined candidate ranking as a substrate primitive

**Status:** spec v0.1; ready to implement when prioritized
**Effort:** ~6 phases, parser-through-sharded
**Composes with:** Kähler L1–L13, sharding T1–T13, ACID Phase 1–4, IMAGINE T11–T13, TAGSET (Ask A, pending), HNSW (Ask C, pending), ALTERNATE KEY (Ask B, pending)
**Authored:** 2026-06-06
**Owner:** Gigi engine team

---

## §0 — TL;DR

`DEFINE PATTERN` + `HUNT` + `EXCLUDING IN` move SCJ's Python hunt orchestrator into the substrate as three composable GQL forms. A pattern is a **named, versioned, predicate body** with a **per-row weight expression**. `HUNT pattern IN bundle EXCLUDING IN confirmed_bugs TOP 50` is one line. SCJ's `scripts/scj_hunt_hyperv.py` — the loop that scored 8,810 functions in 7.3 seconds and surfaced JUROJIN at 10.0 — collapses to one statement.

The machinery (predicate evaluation, arithmetic-expression weighting, left-anti-join, top-N truncation, sharded merge) becomes substrate. The **content** (which fiber-field combinations matter for binaries, the empirically-tuned weight values, the Ghidra extraction recipe, the disclosure discipline) stays consumer-side, in a `gigi_patterns` bundle category that consumers contribute to and version-pin. Patterns are the answer to "what does a consumer council look like for a primitive that nobody can spell yet."

Six phases: parser → in-memory registry → planner → EXCLUDING IN → sharded HUNT → graduation off feature flag. No new comparison operators. No new arithmetic surface. Seven new reserved words: `DEFINE`, `PATTERN`, `WEIGHT`, `HUNT`, `EXCLUDING`, `TOP`, `USING`.

---

## §1 — Why this matters more than "let people write COVER queries"

### §1.1 The COVER framing

Today an operator with a code bundle can ask `COVER vid_funcs WHERE has_alloc = 1 AND has_userloop = 1 AND has_arith = 1 PROJECT (name, has_alloc + has_userloop + has_arith AS score) RANK BY score DESC FIRST 50`. That works. It returns rows. But it requires the operator to type the predicate body and the score arithmetic at every call site. The pattern is **anonymous, unversioned, untestable, unsharable**. Two operators looking at the same bundle write two different queries against it and disagree about which candidates are real.

### §1.2 The GIGI framing

A pattern is an **object**. It has a name. It has a body. It has a weight. It has a `USING` declaration of which fiber fields it touches. It composes with other patterns by name. It version-pins against a bundle schema. Consumers ship pattern catalogs the same way they ship bundle schemas — a `gigi_patterns` bundle is just a bundle whose rows are PATTERN definitions.

This is not "COVER with macro expansion." It's a stronger primitive that subsumes ad-hoc COVER queries because the geometric substrate has a notion of *named, weight-ranked, anti-joined search* that a row-bag query language doesn't. We claim the stronger primitive and we test for it.

### §1.3 Marketing claim that survives Lysyanskaya review

> *"Drop your source code into a bundle. Run one GQL query. Get the same 51-candidate Hyper-V ranked list SCJ surfaced in 7.3 seconds, including the two known-bug recoveries (JUROJIN, KICHIJOTEN), reproducibly, against a version-pinned pattern catalog. The substrate doesn't know what a bug is. It knows how to execute a weighted predicate-filtered ranked query against a bundle, and that's enough."*

Note what the claim does **not** say: it doesn't say GIGI finds bugs. It says GIGI **executes** the pattern that finds bugs, when a consumer supplies the pattern. SCJ owns the patterns. GIGI owns the executor.

---

## §2 — The math

### §2.1 PATTERN_HUNT, formally

A pattern P over bundle B is a tuple `(name, pred, weight, using)` where:

- `pred ∈ PredExpr(B.fiber)` — a predicate body in the existing `pred_expr` grammar at `GQL_SPECIFICATION.md:1196-1202`, evaluable per row to {true, false} using the FilterCondition machinery at `src/parser.rs:792-812`.
- `weight : Row(B) → ℝ` — an arithmetic expression in the existing `expr` grammar at `GQL_SPECIFICATION.md:1226-1229`, evaluable per row to a real number. Boolean fields coerce to {0, 1}.
- `using ⊆ B.fiber.fields` — the declared field-touch set, used by the planner for index selection and decryption-scope minimization (same defensive shape as `ProjectInvariant` at `src/parser.rs:611-630`).

A `HUNT P IN B EXCLUDING IN E_1, …, E_k TOP n` evaluates to the ordered candidate set:

```
HUNT(P, B, [E_1, …, E_k], n) =
    top_n(
        { (row, weight(row)) | row ∈ B, pred(row), ∀i: row.pk ∉ E_i.pk },
        by_score_desc
    )
```

The `EXCLUDING IN E_i` clause is **left-anti-join by base PK** in v0.1. When ALTERNATE KEY ships (Ask B), v0.2 adds `EXCLUDING IN E BY identity` for cross-version-stable exclusion.

### §2.2 Weight as a real-valued scoring functional

`weight` is a closed expression over `B.fiber.fields ∪ ℝ` under `{+, -, *, /, parens, bool→{0,1}}`. This is exactly the `expr` chain at `GQL_SPECIFICATION.md:1226-1229`. No new arithmetic. No conditionals in v0.1 — `CLASSIFY ... WHEN ... THEN ... ELSE` already exists in PROJECT and can be lifted into WEIGHT in v0.2 if SCJ's risk_score.py shows the need (OQ-2).

The ranking induced by `weight` is total and stable on its score image. Tie-breaking (OQ-5) falls back to base PK ascending. This matches how `RANK BY ... FIRST k` already breaks ties in COVER.

### §2.3 Anti-join as bitmap difference, not nested EXISTS

The naive desugar of `EXCLUDING IN E` is `WHERE NOT EXISTS (COVER E WHERE pk = outer.pk)` — that's the FilterCondition::Exists path at `src/parser.rs:3022-3035`. It works. It's slow. The planner instead computes the anti-join as a Roaring bitmap difference over the `field_index` for the PK column — `bitmap(B) \ bitmap(E_1) \ ... \ bitmap(E_k)` — then materializes only the surviving rows. This is **the right resolver** for EXCLUDING IN — not because we like reusing bitmaps, but because it's the resolver that lets sharded HUNT (Phase 5) merge anti-joins as a tournament without re-evaluating the predicate, and the bitmap machinery is already shipped.

---

## §3 — Phase 1: parser surface (DEFINE / HUNT / DROP / SHOW)

### §3.1 Shape

Three new top-level statements + one new clause type, all behind a `patterns` Cargo feature flag (Cargo.toml:47-57 landing zone).

### §3.2 EBNF extension

Slot into `statement` at `GQL_SPECIFICATION.md:1118`:

```
statement = ... existing ... | define_pattern_stmt | drop_pattern_stmt
          | show_patterns_stmt | hunt_stmt

define_pattern_stmt = "DEFINE" "PATTERN" name "AS" pred_expr
                      ( "WEIGHT" "(" expr ")" )?
                      ( "USING" field_list )?

drop_pattern_stmt   = "DROP" "PATTERN" name
show_patterns_stmt  = "SHOW" "PATTERNS"

hunt_stmt = "HUNT" name "IN" name
            ( "EXCLUDING" "IN" name )*
            ( "ON" pred_atom ("AND" pred_atom)* )?
            ( "WHERE" pred_expr )?
            ( "RANK" "BY" sort_specs )?
            ( "TOP" number )?
            ( "PROJECT" "(" proj_list ")" )?
            ( emit )?
            ( confidence_filter )?

field_list = name ("," name)*
```

Every non-terminal on the right is verbatim from the existing grammar:
- `pred_expr`, `pred_atom`: `GQL_SPECIFICATION.md:1196-1202`
- `expr`: `GQL_SPECIFICATION.md:1226-1229`
- `sort_specs`, `proj_list`, `emit`, `confidence_filter`: existing COVER clause parsers
- `name`, `number`: existing lexer terminals

### §3.3 AST additions

In `src/parser.rs`, immediately after the `Cover` variant block (lines 114-126):

```rust
DefinePattern {
    name: String,
    pred: Vec<FilterCondition>,
    or_groups: Vec<Vec<FilterCondition>>,
    weight: Option<Vec<String>>,    // tokenized arith expr, eval'd at HUNT
    using_fields: Vec<String>,
},
DropPattern { name: String },
ShowPatterns,
Hunt {
    pattern: String,
    bundle: String,
    excluding: Vec<String>,
    extra_on: Vec<FilterCondition>,
    extra_where: Vec<FilterCondition>,
    rank_by: Option<Vec<SortSpec>>,
    top: Option<usize>,
    project: Option<Vec<String>>,
    confidence_filter: Option<(bool, f64)>,
},
```

Dispatcher edits at `src/parser.rs:1090-1218`:

```rust
"DEFINE" => self.parse_define_pattern(),
"HUNT"   => self.parse_hunt(),
```

Plus `"PATTERN"`/`"PATTERNS"` arms in `parse_drop()` (line 1154) and `parse_show()` (line 1110).

### §3.4 Concrete examples (the SCJ heuristic translated)

```sql
-- SCJ's "integer overflow into allocation" heuristic, as a v0.1 pattern.
DEFINE PATTERN int_overflow_to_alloc AS
    has_alloc = 1
    AND has_arith = 1
    AND has_userloop = 1
    AND uses_untrusted_size = 1
WEIGHT (
    has_alloc * 3.0
    + has_arith * 2.0
    + has_userloop * 2.0
    + uses_untrusted_size * 3.0
    + (CURVATURE(taint) > 0.7) * 2.0
)
USING (has_alloc, has_arith, has_userloop, uses_untrusted_size, taint);

-- Composition: a stricter pattern that requires the base one PLUS a sink.
DEFINE PATTERN int_overflow_with_sink AS
    has_alloc = 1
    AND has_arith = 1
    AND has_userloop = 1
    AND uses_untrusted_size = 1
    AND reaches_pool_alloc = 1
WEIGHT (
    has_alloc * 3.0 + has_arith * 2.0 + has_userloop * 2.0
    + uses_untrusted_size * 3.0 + reaches_pool_alloc * 4.0
)
USING (has_alloc, has_arith, has_userloop, uses_untrusted_size, reaches_pool_alloc);

-- The hunt that replaces scripts/scj_hunt_hyperv.py:
HUNT int_overflow_to_alloc IN vid_funcs
    EXCLUDING IN confirmed_bugs
    TOP 50
    PROJECT (name, module, _score, CURVATURE(taint) AS taint_curv);

-- TAGSET form (post-Ask A — replaces 17 boolean shadows):
DEFINE PATTERN int_overflow_v2 AS
    sinks_reached CONTAINS_ANY ('ExAllocatePool2', 'ExAllocatePoolWithTag')
    AND arith_ops CONTAINS_ANY ('Mul', 'Shl', 'Add')
    AND uses_untrusted_size = 1
WEIGHT (
    sinks_reached.cardinality * 1.5
    + arith_ops.cardinality * 1.0
    + uses_untrusted_size * 3.0
);
```

The `_score` projection field is special: it's the WEIGHT expression's per-row value, exposed to PROJECT under a fixed name. Operators can rename it (`PROJECT (..., _score AS risk)`).

### §3.5 Phase 1 TDD gates

| Gate | Claim | Ground truth |
|---|---|---|
| **PH1** | `DEFINE PATTERN p AS f = 1 AND g IN (2,3) WEIGHT (f * 2 + g) USING (f,g)` parses and round-trips through AST | Rust unit test in `tests/pattern_hunt_parser.rs` — parse, serialize, re-parse, assert AST equality |
| **PH2** | `HUNT p IN b EXCLUDING IN e TOP 10 PROJECT (name, _score)` parses with all clauses in any order COVER accepts them | Combinatorial parser test mirroring `parse_cover` clause-order flexibility at `src/parser.rs:1867-1935` |
| **PH3** | Predicate operator surface is exactly COVER's — `=`, `!=`, `<`, `>`, `<=`, `>=`, `IN`, `NOT IN`, `MATCHES`, `CONTAINS`, `VOID`, `DEFINED`, `BETWEEN`, `EXISTS`, `AND`/`OR`/`NOT` — no new operators | Test enumerates all 14 operators inside a DEFINE PATTERN body and asserts each parses; negative test asserts no novel keyword is silently accepted |
| **PH4** | Without the `patterns` feature flag, the dispatcher does not reach the new arms; no-feature build is byte-identical to pre-spec | `cargo build --no-default-features` byte-for-byte match against pre-PH commit; `tests/no_patterns_feature.rs` asserts `DEFINE PATTERN ...` parses to a `Statement::Unknown` or `ParseError::UnknownVerb` |

Companion Python validation under `theory/scj/validation/ph1_parser_roundtrip.py` and `ph3_operator_surface.py`. Each carries a `# Circular-logic guard: the parser does not validate the pattern semantically; only that the surface tokens map to the expected AST shape.` header.

---

## §4 — Phase 2: in-memory pattern registry

### §4.1 Shape

Patterns live on the `Engine` struct alongside `query_cache` and `trigger_manager` (`src/engine.rs:367-398`). One `HashMap<String, PatternDef>`, lifetime tied to the engine. Lost on restart. This is the **Prepare precedent** (`src/parser.rs:506-509`) — prepared statements work the same way today.

### §4.2 What ships

- `Engine.pattern_registry: HashMap<String, PatternDef>` added behind `#[cfg(feature = "patterns")]`.
- `Statement::DefinePattern` executor inserts into the map; `Statement::DropPattern` removes; `Statement::ShowPatterns` returns the keyset as a result envelope.
- Name collision: `DEFINE PATTERN p` over an existing `p` is an error unless `OR REPLACE` is supplied (mirror `CREATE OR REPLACE TRIGGER` at `src/parser.rs:576-581`).
- `USING` fields are **validated lazily** at HUNT-time, not DEFINE-time — a pattern can be defined before its target bundle exists. HUNT fails with a typed error if the bundle's fiber lacks any of the `using` fields.

### §4.3 Phase 2 TDD gates

| Gate | Claim | Ground truth |
|---|---|---|
| **PH5** | `DEFINE PATTERN p ...; SHOW PATTERNS` returns `p` in the result set | Integration test in `tests/pattern_hunt_registry.rs` |
| **PH6** | `DEFINE PATTERN p AS x = 1; DEFINE PATTERN p AS x = 2` errors without `OR REPLACE` and silently overwrites with it | Two-case integration test |
| **PH7** | `HUNT p IN b` against a bundle whose fiber lacks a field in `p.using` returns a typed `PatternFieldMissing` error, not a panic | `tests/pattern_hunt_missing_field.rs` |
| **PH8** | Pattern registry survives transaction begin/commit/abort cycles cleanly (DEFINE PATTERN is non-transactional in v0.1, like PREPARE) | Mirrors `tests/transactions_prepare.rs` shape |

---

## §5 — Phase 3: HUNT planner + executor

### §5.1 Shape

`HUNT p IN B` desugars at execution time into a Cover-shaped query plan:

1. Resolve `p` via the registry. Substitute `p.pred` into the WHERE conditions, `p.weight` into a synthetic PROJECT column named `_score`, and `p.using` into the index-selection hint.
2. Append the caller's `extra_on`, `extra_where`, `project` (with `_score` always available).
3. Default `RANK BY _score DESC` if no explicit RANK BY.
4. Default `TOP n` becomes `FIRST n` in the underlying Cover plan (single-line alias at `parse_cover` — accept both spellings, normalize to FIRST in the executor).
5. Dispatch to `store.filtered_query_projected_ex(...)` at `src/bundle.rs:1173` — the same code path COVER uses.

### §5.2 The `_score` field

`_score` is a reserved projection column emitted by HUNT and not by any other verb. The executor evaluates `p.weight` over each surviving row **after** predicate filtering, **before** RANK BY and TOP. PROJECT expressions can reference `_score` by name. Renaming via `PROJECT (_score AS risk_score)` is supported.

### §5.3 NULL handling in WEIGHT (resolves OQ-4)

A NULL fiber field referenced in WEIGHT coerces to 0.0 for arithmetic and to `false` (i.e., 0) for boolean. This matches the existing `RESOLVE(f, default)` semantics at PROJECT (`GQL_SPECIFICATION.md:1217`). Patterns that want strict NULL handling should opt in via `RESOLVE(f, 0)` inside the WEIGHT expression itself, making the default explicit.

### §5.4 Phase 3 TDD gates

| Gate | Claim | Ground truth |
|---|---|---|
| **PH9** | Single-pattern HUNT against a hand-built corpus reproduces the same ranking the equivalent COVER query produces | `tests/pattern_hunt_equivalence.rs` — assert HUNT and equivalent COVER return identical rows in identical order |
| **PH10** | `_score` is computed correctly over a corpus with mixed-NULL fiber fields | `tests/pattern_hunt_null_coercion.rs` — table-driven test of weighted score against hand-computed expected values |
| **PH11** | HUNT recovers JUROJIN at rank 1 (or in the top-3) against a synthetic 1,000-function vid.sys corpus with a planted JUROJIN-shaped row | `tests/pattern_hunt_jurojin_recovery.rs` + `theory/scj/validation/ph11_jurojin.py` — assert recall=1.0 at TOP 10 |
| **PH12** | TOP n truncation is stable across ties (base PK ascending) | `tests/pattern_hunt_tie_breaking.rs` |

---

## §6 — Phase 4: EXCLUDING IN

### §6.1 Shape

`EXCLUDING IN e_1, e_2, …` evaluates to **left-anti-join by base PK** in v0.1. The planner:

1. Materializes the candidate Roaring bitmap from the predicate filter on B.
2. For each excluded bundle e_i, reads its PK bitmap (no fiber access required — this is just the bundle's PK index).
3. Bitmap difference: `candidates \ e_1.pks \ e_2.pks \ ...`.
4. Materializes only the surviving rows. Evaluates WEIGHT. Ranks. Truncates.

The PK-only access pattern means EXCLUDING IN does not require decrypting the excluded bundles' fiber. This matters when `confirmed_bugs` is a curated, access-controlled bundle and the caller has read access only to its PK column.

### §6.2 Composition with COVER

`EXCLUDING IN` is a **clause**, not just a HUNT-only feature. The Phase 4 parser also extends COVER to accept `EXCLUDING IN`:

```sql
COVER vid_funcs WHERE has_alloc = 1
    EXCLUDING IN confirmed_bugs
    PROJECT (name, has_arith);
```

Same bitmap-difference planner path. Same semantics.

### §6.3 Identity-key matching (v0.2, gated on Ask B)

When ALTERNATE KEY ships, `EXCLUDING IN e BY identity` matches by the bundle's declared identity hash rather than base PK. This is the cross-version-stable form: a function whose PK changed (file moved, line numbers shifted) but whose identity hash (function-body hash) is unchanged still gets excluded.

### §6.4 Phase 4 TDD gates

| Gate | Claim | Ground truth |
|---|---|---|
| **PH13** | `HUNT p IN A EXCLUDING IN B` returns exactly the rows in A matching p whose PK is not in B | `tests/pattern_hunt_excluding.rs` — hand-built corpora with known PK overlap |
| **PH14** | Multiple `EXCLUDING IN` clauses compose as set difference (order-independent) | `tests/pattern_hunt_multi_excluding.rs` |
| **PH15** | `EXCLUDING IN` on COVER yields the same result set as the equivalent `WHERE NOT EXISTS (COVER ...)` form | `tests/cover_excluding_equivalence.rs` |
| **PH16** | EXCLUDING IN does not access the excluded bundle's fiber (decryption-scope minimality) | `tests/pattern_hunt_no_fiber_decrypt.rs` — assert the excluded bundle's decryption counter is unchanged across the HUNT |

---

## §7 — Phase 5: sharded HUNT

### §7.1 Shape

Once the `sharded` feature is on and the target bundle is an atlas, HUNT executes per-chart and merges via the existing top-N tournament pattern from sharded COVER:

1. Plan once on the coordinator. Broadcast the resolved pattern (pred + weight + using) to each chart.
2. Each chart runs the local HUNT independently — predicate filter, WEIGHT evaluation, local TOP n.
3. Coordinator merges the k charts' TOP n streams via a heap-based tournament, yields the global TOP n.

EXCLUDING IN sharded HUNT: the excluded bundle's PK bitmap is **broadcast** to each chart (it's tiny — just a Roaring bitmap of PKs, no fiber). Each chart applies the difference locally. No coordinator-side post-processing.

### §7.2 Refusal regimes

If the bundle is in the **Expander regime** (Davis 2026d §4 — high cross-chart curvature, no clean partition), sharded HUNT refuses with `PatternRefusal::ExpanderRegime` and recommends falling back to single-shard execution. This is the same refusal pattern sharded SAMPLE_TRANSPORT uses.

### §7.3 Phase 5 TDD gates

| Gate | Claim | Ground truth |
|---|---|---|
| **PH17** | Sharded HUNT top-10 against a 4-chart atlas matches the single-shard top-10 against the union bundle | `tests/sharded_pattern_hunt.rs` — Mayer-Vietoris-style equivalence test |
| **PH18** | Sharded HUNT with EXCLUDING IN broadcasts only the PK bitmap, not the excluded bundle's rows | Network-trace test asserting message sizes |
| **PH19** | Sharded HUNT refuses cleanly in Expander regime with a typed error, no partial results | `tests/sharded_pattern_hunt_expander.rs` |

---

## §8 — Phase 6: graduation off `patterns` feature flag

### §8.1 Shape

Once Phase 1–5 are green and at least one production consumer (SCJ) has shipped a `gigi_patterns`-backed catalog and run it against confirmed-bug ground truth for one full release cycle, the `patterns` flag becomes default-on, then is removed. Same graduation path Kähler followed.

### §8.2 Persistence (resolves OQ-1)

At graduation, the registry moves from in-memory to a `gigi_patterns` bundle. Rows in `gigi_patterns` have schema:

```
BASE   pk: pattern_name : str
FIBER  body: str (the DEFINE PATTERN text)
       weight: str (the WEIGHT expression text)
       using: list[str]
       version: int
       schema_pin: bundle_schema_hash
```

WAL replay on startup rebuilds the in-process registry from the bundle. `DEFINE PATTERN` becomes a transactional write to `gigi_patterns`. `DROP PATTERN` becomes a transactional delete. The bundle is just a bundle — sharded if you want, replicated if you want, ACID-bound to the writes that touch it.

This is where pattern versioning, sharing across operators, and version-pinning against a bundle's schema all become tractable. It is also where consumers (SCJ shipping their Hyper-V catalog, a hypothetical web-security consumer shipping an OWASP catalog, PRISM shipping a payment-fraud catalog) interact with each other via bundle import/export.

### §8.3 Phase 6 TDD gates

| Gate | Claim | Ground truth |
|---|---|---|
| **PH20** | A pattern DEFINEd, dropped, and re-DEFINEd in three transactions, with engine restart between each, yields the expected final state | `tests/pattern_hunt_persistence.rs` |
| **PH21** | Pattern's `schema_pin` mismatching the target bundle's current schema hash yields a typed `PatternSchemaDrift` warning at HUNT time | `tests/pattern_hunt_schema_drift.rs` |

---

## §9 — API surface

### §9.1 GQL

The four verbs above. No HTTP surface in v0.1 — patterns are accessed through the existing `/v1/gql` endpoint.

### §9.2 HTTP (graduation)

At Phase 6, optionally expose:

- `GET /v1/patterns` → list (same as `SHOW PATTERNS`)
- `GET /v1/patterns/{name}` → fetch one
- `POST /v1/patterns` → DEFINE (body = the EBNF text)
- `DELETE /v1/patterns/{name}` → DROP
- `POST /v1/bundles/{b}/hunt` → execute, body = `{pattern, excluding[], top, project}`

### §9.3 Rust types

```rust
pub struct PatternDef {
    pub name: String,
    pub pred: Vec<FilterCondition>,
    pub or_groups: Vec<Vec<FilterCondition>>,
    pub weight: Option<WeightExpr>,
    pub using: Vec<String>,
    pub version: u32,
    pub schema_pin: Option<u64>,
}

pub enum WeightExpr {
    Lit(f64),
    Field(String),
    Add(Box<WeightExpr>, Box<WeightExpr>),
    Sub(Box<WeightExpr>, Box<WeightExpr>),
    Mul(Box<WeightExpr>, Box<WeightExpr>),
    Div(Box<WeightExpr>, Box<WeightExpr>),
    BoolCoerce(Box<FilterCondition>),
    Curvature(String),
    Resolve(String, f64),
}
```

`WeightExpr` is a minimal arithmetic AST — strictly the subset of `expr` from the spec grammar that doesn't require runtime side-effects. Aggregate functions (SUM, AVG) and TRANSPORT/SHIFT are deferred to v0.2 (OQ-2).

### §9.4 Backwards compatibility

With the `patterns` feature off (Phases 1-5), the engine compiles and runs exactly as before. No reserved-word collisions in existing user GQL — verified by checking the §XIII table at `GQL_SPECIFICATION.md:1246-1273` against the seven new keywords. With the feature on, no existing verb's behavior changes; the only addition is the new dispatcher arms.

---

## §10 — Failure modes

- **Pattern references a field that doesn't exist on the target bundle.** Typed `PatternFieldMissing` at HUNT time. Caught by PH7.
- **WEIGHT expression divides by zero.** Returns NaN; NaN sorts to bottom under DESC; the row is effectively de-ranked but not dropped. Operators get a per-row `_score_status` field with values `Ok`/`NaN`/`Inf`.
- **EXCLUDING IN bundle has no PK index.** Falls back to nested-EXISTS path with a `PatternPerfWarning`. PH13 covers the bitmap path; a separate test covers the fallback.
- **Sharded HUNT in Expander regime.** Refuses cleanly. PH19.
- **Pattern schema-pin drift.** Warning, not error. The HUNT still runs; the result envelope carries a `schema_drift: true` flag. Operators decide whether to trust it.
- **`OR REPLACE` race in concurrent DEFINE.** Last-write-wins under the registry mutex; transactional once the bundle backing lands at Phase 6.
- **`gigi_patterns` bundle missing on a Phase 6 engine.** Engine starts with an empty registry; first `DEFINE PATTERN` creates the bundle implicitly with the canonical schema. Same shape as the existing `gigi_audit` bundle bootstrap.

---

## §11 — Open questions

1. **Pattern registry persistence — in-memory only (v0.1) or always `gigi_patterns`-backed (v0.2)?** *Recommendation:* ship in-memory in Phase 2; the registry-to-bundle migration is Phase 6's whole job. Don't make Phase 2 wait on a bundle schema decision.

2. **WEIGHT expression DSL scope.** Restricted arithmetic (`+ - * /`, parens, fields, literals, `CURVATURE`, `RESOLVE`) only in v0.1, or full `expr` chain including aggregates and conditionals? *Recommendation:* start restricted. Lift `CLASSIFY ... WHEN ... THEN ... ELSE` into WEIGHT in v0.2 if and only if SCJ's `risk_score.py` has a case that doesn't translate cleanly under the restricted DSL. Aggregates (SUM, AVG) inside WEIGHT need careful semantics (over what window?) and shouldn't ship until the use case is concrete.

3. **Anti-join key.** Base PK only (v0.1) or identity hash from ALTERNATE KEY (v0.2)? *Recommendation:* ship base PK in Phase 4. Add `EXCLUDING IN e BY identity` as a clause extension once Ask B (ALTERNATE KEY) ships. Until then, identity-stable exclusion is the consumer's problem (they can write a custom anti-join with EXISTS).

4. **NULL handling in WEIGHT.** Coerce to 0, skip the row, or error? *Recommendation:* coerce to 0 (matches PROJECT's `RESOLVE` default). Document explicitly. Pattern authors who want strict can wrap in `RESOLVE(f, 0)` to make the coercion visible.

5. **Tie-breaking when multiple rows share the top score.** *Recommendation:* base PK ascending. Stable. Mirrors COVER's existing RANK BY ties. Document as part of the v0.1 contract so consumers don't depend on insertion order.

6. **`HUNT ... WITH CONFIDENCE` — what's the gate threshold and how does it compose with WEIGHT?** *Recommendation:* deferred to a follow-up sub-spec once Brain primitives expose `confidence` as a queryable scalar. The thinking: `WITH CONFIDENCE > 0.7` filters candidates whose `brain/confidence` lookup against the bundle's nearest-neighbor embedding exceeds the threshold, after WEIGHT ranking. Composes as a post-filter, not a pre-filter — confidence is expensive, you want it after the bitmap operations.

7. **SCJ feedback — does the v0.1 grammar handle their full 10-weight risk_score Python today?** *Recommendation:* the appendix below shows the translation. The honest answer is: 8 of the 10 weights translate verbatim, 1 requires `CURVATURE(taint) > 0.7` (already supported), and 1 (their "function calls userspace and also accepts size param" cross-field implication) needs `CLASSIFY ... WHEN ... THEN ... ELSE` inside WEIGHT — which is what makes OQ-2 a real question rather than a hypothetical.

8. **What's the result envelope when HUNT returns zero candidates?** *Recommendation:* an empty result with `_score_status: AllFiltered` metadata, not a 404. Mirrors COVER's empty-result shape.

---

## §12 — Effort summary + sequence

| Phase | Surface | Test gates | Depends on |
|---|---|---|---|
| **1** | Parser AST + dispatcher + EBNF | PH1–PH4 | (none) |
| **2** | In-memory registry + Engine plumbing | PH5–PH8 | Phase 1 |
| **3** | HUNT planner + executor + `_score` | PH9–PH12 | Phase 2 |
| **4** | EXCLUDING IN clause + bitmap anti-join | PH13–PH16 | Phase 3 |
| **5** | Sharded HUNT + tournament merge | PH17–PH19 | Phase 4 + `sharded` flag |
| **6** | `gigi_patterns` bundle + WAL replay + graduation | PH20–PH21 | Phases 1–5 + one prod consumer + one release cycle |

Suggested sequencing: Phases 1–3 are the minimum viable demo (SCJ can reproduce their hunt against an unsharded vid.sys bundle with no exclusion list). Phase 4 makes the demo "actually replaces their orchestrator" (the JUROJIN/KICHIJOTEN anti-join). Phase 5 makes it production-scale (Hyper-V driver-class corpora are large and shard naturally by source file). Phase 6 is graduation and consumer-council infrastructure.

What unblocks what: Phase 1 unblocks SCJ writing patterns against `vid.sys` bundles offline (just to verify the grammar shape). Phase 3 unblocks the first reproducible JUROJIN recovery demo. Phase 4 unblocks the "operator UX" claim — one statement, ranked candidates, exclusions applied. Phase 5 unblocks deployment beyond toy corpora. Phase 6 unblocks consumer pattern catalogs and the council model.

---

## §13 — Composition with existing work

This spec composes with the work already shipped:

- **Kähler L1–L8 (`src/geometry/`, `src/curvature.rs`):** patterns over scalar fiber fields like `taint`, `heat`, scalar curvature are first-class. A pattern can write `WEIGHT (heat * 0.5 + CURVATURE(taint) * 2.0)`. No new geometric primitive added; Marcella's 0.0013 non-associativity bound unaffected.
- **Brain primitives L9–L13 (`src/brain/`):** optional `HUNT ... WITH CONFIDENCE` (OQ-6, deferred) gates novel candidates via `/brain/confidence`. Optional `EXPLAIN HUNT` (future) uses `/brain/explain` to surface the geodesic path from a candidate to its nearest confirmed-bug neighbor — the "why did the substrate surface this?" answer. For v0.1, brain is consumed only as opaque scalar fields when the consumer chooses to project them in.
- **Sharding T1–T13 (`src/sharded/`):** HUNT is per-chart executable + tournament merge at the coordinator. Refuses cleanly in Expander regime. EXCLUDING IN broadcasts only the PK bitmap, not the fiber.
- **Transactions Phase 1–4 (`src/transactions/`):** v0.1 pattern registry is non-transactional (matches PREPARE). At Phase 6, the `gigi_patterns` bundle is a normal transactional bundle and DEFINE/DROP become 2PC participants. HUNT reads pin to the caller's MVCC snap_id.
- **IMAGINE / WALK (`src/imagine/`):** patterns can target imagined records — `HUNT p IN b WHERE provenance.imagined = true` works without grammar change. Provenance propagates through the result envelope.
- **TAGSET (Ask A, pending):** post-TAGSET, the v0.2 idiomatic pattern form replaces 17-boolean shadows with `sinks_reached CONTAINS_ANY [...]`. Patterns are the consumer-side answer to "use the new type idiomatically."
- **HNSW (Ask C, pending):** post-HNSW, optional `HUNT ... NEAREST <embedding>` pre-filters HUNT to a candidate ball around an anchor — accelerates patch-twin hunting (the SUSANOO use case in SCJ's round-5 letter).
- **ALTERNATE KEY (Ask B, pending):** post-ALTKEY, `EXCLUDING IN e BY identity` does cross-version-stable exclusion. Until then, base-PK exclusion is the v0.1 contract.
- **Consumer-council framing (round-5/6 correspondence):** `gigi_patterns` is the bundle category SCJ contributes their Hyper-V catalog to. The substrate doesn't know what a bug is; the catalog does. The patent claim is the executor; the consumer's claim is the catalog. This is exactly the Marcella/Gigi split applied one layer down.

**We are not building this from scratch.** The math is mostly done, the substrate is mostly built. Phases 1–5 are the **wiring** of machinery we already proved (FilterCondition, the `expr` arithmetic chain, Roaring bitmaps, sharded tournament merge) into a named, weighted, anti-joined pattern-detection abstraction. Phase 6 graduates it into a consumer-council surface.

---

## §14 — What stays consumer-side

Four things SCJ keeps owning, in the same shape Marcella owns discourse semantics while Gigi owns the Friston substrate:

1. **The pattern content.** Which fiber-field combinations matter for binaries (`has_alloc + has_arith + has_userloop + uses_untrusted_size`) versus web (`has_user_input + reaches_render + no_escape`) versus payment fraud (PRISM's eventual claim) is the consumer's claim, not the substrate's. The substrate cannot guess which fields to combine.
2. **The weight values.** `3.0` for `has_alloc`, `2.0` for `has_arith`, etc. — empirically tuned against the consumer's confirmed-bug ground truth. The substrate cannot guess weights. Two consumers in the same domain with different ground-truth sets will tune to different weights and both will be right for their tuning corpus.
3. **The Ghidra decompiler recipe + extraction pipeline.** How `vid.sys` becomes a bundle with these 17 boolean shadows (or, post-TAGSET, these `sinks_reached`/`arith_ops` sets) is consumer-side Python. The substrate ingests bundles; it doesn't extract them.
4. **The disclosure discipline.** Two-person review before disclosure. 90-day clock. No auto-PoC generation. No exploitation tooling. SCJ owns the operational protocol around what to do with a HUNT result. The substrate produces ranked candidates; what humans do with them is governed by the consumer.

The substrate doesn't know what JUROJIN is. It knows how to execute `HUNT int_overflow_to_alloc IN vid_funcs EXCLUDING IN confirmed_bugs TOP 50` against a bundle, and that's enough.

---

## §15 — When to start

Phase 1 is unblocked **now**:

1. The EBNF reuses existing non-terminals (`pred_expr`, `expr`, `proj_list`, `sort_specs`, `pred_atom`) verbatim.
2. The dispatcher edit is two lines.
3. The seven new reserved words do not collide with the existing reserved-word table at `GQL_SPECIFICATION.md:1246-1273`.
4. The `patterns` feature flag landing zone is approved (Cargo.toml:47-57 model).
5. SCJ has working Python (`risk_score.py`, `exclusions.py`, `scj_hunt_hyperv.py`) that defines the ground truth for PH11 and the appendix.

Phase 2 is unblocked when Phase 1 lands and the Engine struct has the registry field added.

Phase 5 is unblocked when sharded is on by default in the target build and an atlas-shaped pattern corpus exists. This is a release-cycle gate, not a Phase 4 gate.

Phase 6 is unblocked when (a) Phases 1–5 are green for at least one release cycle, (b) SCJ (or another production consumer) has shipped a `gigi_patterns` catalog and run it against confirmed-bug ground truth, (c) the `gigi_patterns` bundle schema has survived one round of consumer council review.

---

## §16 — Appendix: SCJ's `risk_score.py` translated into v0.1 grammar

SCJ's current Python heuristic (10 weights, anti-join against `confirmed_bugs`, top-50 over vid.sys):

```python
# scj/geodesic/risk_score.py (sketch — actual file lives at the SCJ repo)
def score(fn):
    s = 0.0
    if fn.has_alloc:               s += 3.0
    if fn.has_arith:               s += 2.0
    if fn.has_userloop:            s += 2.0
    if fn.uses_untrusted_size:     s += 3.0
    if fn.reaches_pool_alloc:      s += 4.0
    if fn.taint_curvature > 0.7:   s += 2.0
    if fn.calls_userspace:         s += 1.5
    if fn.has_size_param:          s += 1.0
    if fn.calls_userspace and fn.has_size_param:  s += 2.0   # cross-field
    if fn.no_bounds_check:         s += 2.5
    return s

# scripts/scj_hunt_hyperv.py
candidates = [f for f in vid_funcs if matches_predicate(f)]
candidates = [f for f in candidates if f.pk not in confirmed_bugs_pks]
candidates.sort(key=score, reverse=True)
return candidates[:50]
```

In v0.1 GQL (8 of 10 weights translate directly):

```sql
DEFINE PATTERN scj_hyperv_v1 AS
    has_alloc = 1
    AND (has_arith = 1 OR has_userloop = 1)
    AND uses_untrusted_size = 1
WEIGHT (
    has_alloc * 3.0
    + has_arith * 2.0
    + has_userloop * 2.0
    + uses_untrusted_size * 3.0
    + reaches_pool_alloc * 4.0
    + (CURVATURE(taint) > 0.7) * 2.0
    + calls_userspace * 1.5
    + has_size_param * 1.0
    + no_bounds_check * 2.5
)
USING (has_alloc, has_arith, has_userloop, uses_untrusted_size,
       reaches_pool_alloc, taint, calls_userspace, has_size_param, no_bounds_check);

HUNT scj_hyperv_v1 IN vid_funcs
    EXCLUDING IN confirmed_bugs
    TOP 50
    PROJECT (name, module, _score);
```

The one weight that does **not** translate cleanly in v0.1 is the cross-field implication `calls_userspace AND has_size_param → +2.0`. In v0.1 a consumer must work around it with a synthetic field at ingest time:

```sql
-- Consumer-side ingest: add a derived boolean column
-- (in their extraction pipeline, not in GQL)
fn.userspace_and_size = fn.calls_userspace and fn.has_size_param
```

Then add `+ userspace_and_size * 2.0` to the WEIGHT. This is annoying but acceptable for v0.1. v0.2 lifts `CLASSIFY ... WHEN ... THEN ... ELSE` into WEIGHT (OQ-2) and the cross-field implication writes as:

```sql
WEIGHT (
    ... existing weights ...
    + (CLASSIFY WHEN calls_userspace = 1 AND has_size_param = 1 THEN 2.0 ELSE 0.0)
)
```

at which point the translation is complete.

The honest summary: **v0.1 replaces 8/10 of risk_score.py verbatim, 1 with `CURVATURE`, and 1 with a consumer-side derived field. The orchestrator (`scj_hunt_hyperv.py`) collapses to one `HUNT` statement.** That's the load-bearing demonstration.

— Spec authored 2026-06-06 (Gigi engine team)
