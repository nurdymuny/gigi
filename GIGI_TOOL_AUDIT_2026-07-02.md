# GIGI Tool Audit — 2026-07-02

**Scope:** the whole repo, read as one question — *is GIGI a strong tool for
someone building on it alone?* The Connor test: an undergrad with no
supervisor clones it, follows the docs, makes normal mistakes, and either
learns or gets silently lied to. Method: full test-suite run, live burn-test
of learner mistakes against a running `gigi-stream`, code sweep of the query
executors, docs-vs-code diff. Everything below is verified against this
tree, not inferred.

**Baseline verified:** default-feature suite green (894 lib tests + all
integration binaries, 0 failed) *after* the fixes below; one test file
previously broke `cargo test` on a fresh clone.

---

## Fixed in this audit (shipped on this branch, tests added)

| # | What | Where | Why it mattered |
|---|---|---|---|
| 1 | `cargo test` failed to compile on a fresh clone — `tests/aurora_phase_1b_lattice_verb_cubed_sphere.rs` uses `gigi::lattice` without the feature gate | test file, now `#![cfg(feature = "lattice")]` | The first verification command a new user runs died with 4 compile errors. |
| 2 | Getting-started doc told users to run `--bin gigi_stream`; the binary is `gigi-stream` | `docs/GETTING_STARTED.md:370,377` | Literal first server command failed. |
| 3 | **Unknown field names in queries silently returned wrong results** — typo'd `WHERE temmp > 100` matched nothing; typo'd `PROJECT (tempp)` column vanished; typo'd `INTEGRATE OVER cityy` returned empty | `gigi_stream.rs` Cover + Integrate arms; new `FilterCondition::field_name()` in `parser.rs` | Now: `Unknown field 'temmp' — this bundle's fields are: id, city, temp`. Bundle names were already validated; fields were the missing sibling. |
| 4 | **Trailing tokens silently discarded** — `INTEGRATE ... HAVING avg(t) > 100` ran *without* the HAVING and returned unfiltered groups with status ok; any suffix garbage was swallowed | `parser.rs::parse` | Now an explicit error naming the ignored input. This closes the whole class, not one instance. (The book's E1.3 bonus literally asks readers to discover this wart — it's now an error they can see.) |
| 5 | `AggResult::variance` used the naive `sum_sq/n − mean²` form — catastrophic cancellation can drive it negative → `stddev()` = NaN → silent `null` in JSON | `aggregation.rs:31` | Clamped at 0; comment points to Welford `m2` as the preferred accumulator. |
| 6 | *(earlier this session)* INTEGRATE multi-measure aliasing; `count(*)`/text-field count silent-empty; global no-OVER INTEGRATE hardcoded empty | `aggregation.rs`, `parser.rs`, `gigi_stream.rs` | The count(*) family. |

Burn-test after fixes — every learner mistake now teaches:

```
COVER stations WHERE temmp > 100;
  → Unknown field 'temmp' — this bundle's fields are: id, city, temp
INTEGRATE stations OVER city MEASURE avg(temp) HAVING avg(temp) > 100;
  → ...trailing input is not a supported clause and was NOT executed: 'HAVING avg ( temp ) >'
COVER stations WHERE temp > 100;            → correct row, unchanged
INTEGRATE stations OVER city MEASURE count(*), avg(temp);  → correct, unchanged
```

---

## Remaining findings, ranked by what they cost a builder

### P0 — the reference table lies (docs claim ✅ for things that don't exist)

- **`FIBER RANK` / `FIBER SUM` don't parse at all** (`Unknown statement:
  FIBER`), yet `GQL_REFERENCE.md` marks "FIBER / TRANSPORT (window) ✅".
  `HAVING` is listed under INTEGRATE ✅ and has no parser support (it now
  errors honestly instead of silently no-oping, but it should either exist
  or leave the table).
- **Recommendation (the high-leverage version):** don't hand-fix the table —
  *automate the confession*. A CI harness that extracts every statement
  example from `GQL_REFERENCE.md`, runs it against a fresh engine, and
  fails the build on any ✅ row that errors. GIGI's whole brand is receipts;
  the reference should have to produce one too. This also permanently ends
  doc drift, which is otherwise a recurring tax.

### P1 — error UX is the tool's teaching voice, and it mumbles

- Parser errors leak Rust internals and carry no position: `Parse error:
  Expected 'BUNDLE', got Some(Word("TRIGGER"))` (`parser.rs:1914–1965`,
  `{other:?}` formatting). A learner needs: the offending token *as they
  typed it*, a caret/offset into their statement, and when applicable a
  "did you mean" (the statement keyword set is small — Levenshtein ≤ 2
  against known verbs is cheap).
- Admin statements report success while doing nothing: `VACUUM stations;` →
  `{"status":"ok"}` via the `_ => Ok(ExecResult::Ok)` catch-alls
  (`gigi_stream.rs:12899, 14065`). For a tool, a success-lie is worse than
  a 501. Recommendation: an `ExecResult::Notice("parsed; no-op in this
  build")` variant surfaced in the response, so the honest ⚠️ from the
  reference table reaches the wire.

### P2 — hardening the request path (crash class)

- **239 `.unwrap()` in `gigi_stream.rs`.** Many are startup-time (fine); an
  unknown subset sits in request handlers where malformed input = worker
  panic. Worth one focused pass: grep unwraps inside `async fn` handlers
  and the execute paths, convert to 4xx errors. (Not itemized here — the
  pass is mechanical and the wins are cumulative.)
- **Non-finite f64 → silent JSON `null`.** serde_json maps NaN/∞ to `null`;
  any geometry output that can go non-finite (division by tiny ranges,
  empty-graph spectral values) will show users a `null` with no
  explanation. Recommendation: a single `fn finite_or_reason(f64) ->
  serde_json::Value` used at response-assembly chokepoints.

### P3 — what to ADD to make GIGI a stronger *instrument* (the tool thesis)

1. **Honest-error-bar measurement verb.** GIGI made confidence first-class;
   Monte Carlo users need the same for correlated samples: `INTEGRATE ...
   MEASURE avg(x) WITH JACKKNIFE` (or `WITH ERRORBAR`) returning value ±
   error + integrated autocorrelation time. This is the single feature that
   upgrades Halcyon-class runs from "numbers" to "evidence," and it's a
   natural extension of the Welford machinery already on every insert.
2. **`EXPLAIN` for mistakes, not just plans.** `EXPLAIN` exists for query
   plans; the same entry point could answer "why did my COVER return 0
   rows?" (which condition eliminated everything — evaluate conditions
   independently and report per-condition match counts; cheap on scan
   paths).
3. **Bulk data on/off ramps.** Learners arrive with CSVs. A `COPY`-style
   import (`POST /v1/bundles/{b}/import` with CSV/JSONL) and an export
   endpoint would remove the biggest first-hour friction after the docs.
   (The insert API works fine; the friction is writing the loop yourself.)
4. **`SHOW FIELDS <bundle>` / better DESCRIBE ergonomics** — now that
   unknown fields error with the field list, the natural next question the
   error itself teaches ("what ARE my fields?") should have a first-class
   one-word answer. DESCRIBE exists; make sure the error message names it.
5. **Public playground bundle.** Track 1 of the docs depends on whatever
   bundles happen to exist on the public instance. Ship a small, permanent,
   documented `playground` bundle (the book's `stations` corpus) on
   gigi-stream.fly.dev so every doc example and the site's live console
   work verbatim, forever.

### P4 — maintenance debt worth scheduling

- **SDK surface**: `sdk/python` and `sdk/notebook` should get a smoke test
  in CI against a locally spawned server (create/insert/gql round-trip) so
  they can't drift from the route table silently.
- **Untested load-bearing paths**: the `gigi_stream.rs` statement-execution
  arms are primarily exercised live rather than by tests (the count(*) and
  HAVING classes survived because nothing asserted on those paths). A
  table-driven executor test — one bundle, one statement per supported verb,
  assert on rows — would have caught every silent-answer bug in this audit.
  That's the highest-value test file this repo doesn't have.
- Env-var doc drift: code reads `TIGRIS_BUCKET_NAME`, `GIGI_RATE_LIMIT`,
  `GIGI_RATE_WINDOW`, `GIGI_TRUST_PROXY`, `GIGI_INSTANCE`,
  `GIGI_SKIP_BOOT_SNAPSHOT`, cache-size vars — several undocumented, and the
  docs' S3 block names differ from what's read.

---

## The one-paragraph verdict

The engine's core loop — insert, address, aggregate, geometric ride-alongs —
is solid, calibrated, and unusually reproducible; the receipts culture is
real and it's the moat. The weaknesses are all in one family: **the tool's
mouth.** Where GIGI knows something is wrong (unknown field, unsupported
clause, no-op statement, non-finite number), it historically said nothing
and returned something plausible. Every fix in this audit and most of the
roadmap is the same move repeated: make the engine say what it knows. A
tool people learn on alone doesn't need to be bigger — it needs to never
let a beginner's mistake look like a result. Close that class (P0–P2), add
the error-bar verb (P3.1), and GIGI isn't just a good substrate — it's a
better lab partner than most humans.
