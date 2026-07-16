# EXPLAIN family — shipped 2026-07-16 (Marcella wave 1 of 3)

Four asks from `GEODESIC_LOOM_PLAN.md` ('gigi-side asks (UPDATED)', signed Hallie),
implemented TDD (RED → GREEN per ask) on top of main @ 87b6700. Waves 2
(WINDOWED_COHERENCE + locus/vector-only statistics) and 3 (WAL/snapshot
durability) deliberately untouched.

## Design choices

### Ask 5a — error contract
- **Root cause:** the executor error channel is `Result<ExecResult, String>` and
  `gql_query`'s read/write/virtual branches blanket-mapped every `Err` to HTTP
  500. A point-read miss was indistinguishable from an internal fault. There
  was **no wide-record defect**: the 393-field bundle EXPLAINs correctly with a
  correct key (pinned by a 393-field fixture mirroring v2's shape — record_id
  TEXT base + v0..v383 + 8 extra numeric fibers).
- **Contract:** miss ⇒ `Err("NOT_FOUND: EXPLAIN: no section at <key> in bundle
  '<b>'")`. The `NOT_FOUND: ` sentinel (pub const in `gigi::explain`) is
  stripped by `exec_error_to_response` in gigi_stream and mapped to **404**
  with `{"error":"EXPLAIN: no section at record_id='x' in bundle 'b'"}` —
  mirroring the REST section-fetch 404 (`Record '<id>' not found in bundle
  '<name>'`, src/bin/gigi_stream.rs). Key values render quoted for text,
  bare for numbers; composite keys join in field-name order. Wrong key NAME
  collapses to the same typed miss (point query matches nothing). Plain
  `SECTION AT` miss keeps its silent `200 {"rows":[],"count":0}` shape.
- Both executor arms (embedded `parser::execute` and the server read path) now
  delegate to one shared function — `gigi::explain::execute_explain_section` —
  ending the hand-copied duplication that let the two arms drift (the server
  arm's miss didn't even name the bundle).

### Ask 1 — vector kappa
- **Formula:** `kappa_v = |1 − cos(v, mu_v)| / R_cos`, emitted as an ADDITIVE
  row: `field` (label), `kind:"vector"`, `kappa`, `cos`, `one_minus_cos`,
  `r_cos`, `dim`, `n`, `record_kappa` (ride-along stamp).
- **cos** = `dot(v,mu) / sqrt(dot(v,v)·dot(mu,mu))`, clamped to [−1,1].
  **Normalization ruling:** no separate unit-normalization — the cosine
  self-normalizes both operands; kappa_v is direction-only by construction
  (v2 rows are unit-norm, verified live, but correctness does not depend on
  it). The `sqrt(x·x) == x` identity of correctly-rounded f64 makes
  `cos(mu,mu) == 1.0` exactly, so the record==mean anchor lands
  `kappa_v == 0.0` exactly, not approximately.
- **mu_v** = per-component mean over `store.records()` computed on demand in
  the same call (never insert-time FieldStats: `Value::Vector` never enters
  FieldStats, and one population for both mu and R_cos is what makes the zero
  anchor exact). Zero-norm vectors count toward mu (a mean of vectors, not of
  directions) but are skipped from the cosine range; fewer than 2 defined
  cosines ⇒ no row (mirrors the scalar count<2 skip). Undefined target cosine
  ⇒ no row, never a fabricated one.
- **R_cos** = max − min of (1 − cos) across the bundle in the same EXPLAIN
  call, floored to `f64::EPSILON` (the same floor `bundle::effective_range`
  uses).
- **Two shapes:** (a) true `Value::Vector` fiber fields ⇒ automatic row per
  field on the explained record (schema order); (b) scalar-family clause
  `VECTOR (v0..v383)` / `VECTOR (f1, f2, …)` — **both** range sugar and
  explicit list (mixable; the parser's existing DotDot token — SEEDS
  [lo..hi] — made ranges cheap). Row label = clause as written:
  `vector(v0..v383)`. Range endpoints must share a prefix; unpadded
  rendering; 100k-field expansion cap; clause typos (unknown/non-numeric
  field) are loud errors, data-level sparsity (record can't assemble) just
  omits the row.
- **INVARIANT DISCIPLINE:** `compute_record_k` untouched (LOCKED);
  `record_kappa` unchanged in definition and value; scalar rows byte-identical
  with and without the clause (asserted); vector rows excluded from
  `mean(kappa) == record_kappa` — consumers must filter `kind:"vector"`.

### Ask 2 — batch
- **Grammar:** `EXPLAIN SECTION b AT <field> IN (v1, …, vn) [VECTOR (…)]
  [PROJECT (…)]` — reuses the WHERE-IN literal-list shape.
- **Rows:** grouped in INPUT order (deliberately unlike PER-grouping's
  ascending sort — the caller's list is the contract); discriminator column
  `<field>=value` stamped on every row of the group; each group is the
  record's full EXPLAIN output (scalar rows + optional vector row).
- **Miss contract:** per-key typed miss entry — one row
  `{<field>: value, kind:"miss", miss:"no section at <field>=<v> in bundle
  '<b>'"}`; the batch never fails wholesale and never silently skips
  (deliberate divergence from chern PER's fail-fast). A found record with
  nothing to decompose emits one `kind:"empty"` note row so no group is
  invisible.
- **One lock:** the caller's engine read-lock spans the whole statement; one
  store resolution; bundle-level vector contexts (mu_v, R_cos) computed once
  and cached across groups (`Explainer.ctx_cache`).

### Ask 5b — mmap on-demand stats
- The old gate read stats off `store.as_heap()` and declined Overlay bundles.
  `Explainer::new` now resolves stats polymorphically: heap = borrowed
  precomputed `field_stats` (zero-copy, behavior unchanged); mmap-backed =
  `BundleRef::field_stats()` ⇒ `OverlayBundle::field_stats()` — a single O(N)
  scan over the mmap base on first access, Welford-merged with overlay stats,
  cached in memory inside the overlay. **Nothing persisted; no storage-format
  change; src/mmap_bundle.rs untouched** (the accessor already existed).
- **Cost:** O(N) on the first EXPLAIN against a large mmap bundle (later
  calls reuse the cached base stats until restart); VECTOR contexts are O(N)
  per vector target per statement (two scans: mu, then range). Accepted per
  ruling — EXPLAIN is a diagnostic verb.

## Math anchors (all passing)

| Anchor | Test | Result |
|---|---|---|
| record == bundle mean vector ⇒ kappa_v = 0 **exactly** | explain_vector::record_equal_to_mean_vector_has_kappa_v_zero_exactly | `== 0.0`, `cos == 1.0` bit-exact |
| orthogonal pair a=(2,0) ⊥ b=(0,1): kappa_v(a)=√5−2, kappa_v(b)=√5−1 | explain_vector::vector_clause_assembles_scalar_family_one_kappa_v_row | 1e-9 |
| hand-computed 3-vector fixture: (√17−4)/3 and (√17−1)/3, cos=4/√17, R=3/√17 | explain_vector::vector_field_gets_automatic_kappa_v_row | 1e-9 |
| scale invariance: b = 3·a gets identical kappa_v (direction-only, no pre-normalization) | explain_vector::kappa_v_is_direction_only_scale_invariant | 1e-12 |
| batch invariant: mean(scalar kappa) == record_kappa per group | explain_batch (every group) + explain_errors wide fixture + explain_mmap | 1e-9 |
| sugar == list: VECTOR (v0..v1) ≡ VECTOR (v0, v1) | explain_vector::range_sugar_and_explicit_list_agree | bit-equal |
| mmap kappas from scanned stats: 5/9, 0, record_kappa 5/18 | explain_mmap::mmap_backed_bundle_returns_real_rows_via_on_demand_stats | 1e-9 |

## Gate table (merged main == 87b6700 + this branch; all green)

| Gate | Result |
|---|---|
| cargo check --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --bin gigi-stream | ok |
| cargo test --no-default-features --lib | ok — 915 passed, 0 failed |
| cargo test --test explain_kappa (existing fence) | ok — 3 passed |
| tests/explain_errors.rs (NEW) | ok — 6 passed |
| tests/explain_vector.rs (NEW) | ok — 8 passed |
| tests/explain_batch.rs (NEW) | ok — 6 passed |
| tests/explain_mmap.rs (NEW) | ok — 4 passed |
| halcyon: spectral_gauge_basic + spectral_gauge_where_basic + spectral_full_basic + u1_flux_basic | ok — 4 suites, 52 tests (12+21+7+12), 0 failed |
| halcyon: spectral_magnetic_basic (contains ~5min statistics test) | ok — 9 passed, 0 failed, 293.8s (the expected long test) |
| halcyon: ingest_as_gauge_field_basic + ingest_gauge_vertex_basic + ingest_npz_key_basic + ingest_npz_dtype_basic + ingest_gql_bypass_basic + halcyon_l24_workflow_e2e + chern_class_basic + chern_class_bundle_target_basic + betti_pi1_basic + obstruction_basic + topology_verbs_gql_integration + halcyon_part_iv_gold + aurora_lie_poisson_trait | ok — 13 suites, 95 tests, 0 failed, 1 ignored (pre-existing) |
| kahler,imagine: imagine_coherence_phase2 | ok — 10 passed, 0 failed |
| kahler: davis_conjecture_lambda_brain_ridealong | ok — 25 passed, 0 failed |
| lattice: cubic_lattice + lattice_obc_basic | ok — 2 suites, 17 tests (7+10), 0 failed |
| patterns: pattern_hunt_parser | ok — 15 passed, 0 failed |
| emit_csv + noop_notices + timestamp_ergonomics + gql_reference_truth + ingest_dir_gate + pathguard_escapes + ingest_executor + ingest_csv_basic + ingest_jsonl_basic | ok — 9 suites, 49 tests (1+3+6+1+11+3+4+14+6), 0 failed |

## Live probes (production, gigi-stream.fly.dev)

Pre-deploy receipts (2026-07-16, image deployment-01KXPESN0FYTYPJNVEVGCVX5R3):

- M1 `COVER marcella_source_embeddings_bge_v2 PROJECT (record_id) RANK BY
  record_id FIRST 2` ⇒ real keys
  `claim:branch_x_information_geometry/claim_0000` / `…_0001`.
- The mmap bundle in Hallie's report is **marcella_voice_math** (base
  anchor_id Categorical indexed; canonical is its numeric fiber). Real key:
  `anchor_id='vm_prequantization_closing'`. Pre-deploy EXPLAIN with that REAL
  key ⇒ HTTP 200 Notice "EXPLAIN κ needs heap-resident field statistics;
  this bundle is mmap-backed — HEALTH gives the aggregate view" — Hallie's
  decline, reproduced verbatim.
- Pre-deploy missing key ⇒ HTTP 500 {"error":"EXPLAIN: no section at that
  key"} — the bug of ask 5a, reproduced.

Post-deploy (release v248, image deployment-01KXPKGW1EMGBK8CB8ZDZ76ZJS,
/v1/health ok):

- **M2** EXPLAIN v2 AT record_id='claim:branch_x_information_geometry/claim_0000'
  ⇒ HTTP 200, **385 rows** (v2 has 385 numeric fibers on prod), loudest field
  v294, `record_kappa` 0.11960424188019436 — byte-identical to the pre-deploy
  correct-key probe. The wide bundle explains; the fix didn't move any kappa.
- **M3** same + `VECTOR (v0..v383)` ⇒ HTTP 200, 386 rows, ONE vector row:
  `{field:"vector(v0..v383)", kind:"vector", kappa:0.809031525423029,
  cos:0.6932827623533859, r_cos:0.37911654615218704, dim:384, n:9964}` —
  the on-demand mu/R_cos scans over all 9,964 records inside the call.
- **M4** AT record_id='definitely_missing_xyz' ⇒ **HTTP 404**
  `{"error":"EXPLAIN: no section at record_id='definitely_missing_xyz' in
  bundle 'marcella_source_embeddings_bge_v2'"}` — the 500 is gone.
- **M5** AT record_id IN ('…claim_0000', '…claim_0001', 'missing_xyz') ⇒
  HTTP 200, 771 rows: group claim_0000 (385 rows, record_kappa 0.1196…),
  group claim_0001 (385 rows, record_kappa 0.12284263657633207), then ONE
  miss row `{record_id:"missing_xyz", kind:"miss", miss:"no section at
  record_id='missing_xyz' in bundle 'marcella_source_embeddings_bge_v2'"}`
  — input order, discriminator on every row.
- **M6** EXPLAIN marcella_voice_math AT anchor_id='vm_prequantization_closing'
  ⇒ HTTP 200 with REAL rows (the decline notice is gone):
  ingested_at kappa 0.1603917204023554, canonical kappa 0.0 (sigma 0, range
  floored to 2.220446049250313e-16 = f64::EPSILON — the floor visible live),
  record_kappa 0.0801958602011777 == mean(0.16039…, 0)/2 exactly.
- **M7** heap regression: EXPLAIN claude_substrate_v0 AT thought_id='t001' ⇒
  HTTP 200, ts row with kappa == record_kappa (single numeric fiber), shape
  unchanged.
- **M8** POST /v1/bundles/marcella_source_embeddings_bge_v2/imagine_coherence
  (4-dim seed, 3 steps) ⇒ HTTP 200, endpoint_coherence 0.9972441962101206
  (≈1.0), refused=false, phase-2 tame-metric audit present (high_k_auto_tame,
  as expected on v2's raw K).

**Substrate drill:** claude_substrate_v0 exported pre-deploy
(.deploy-backups/2026-07-16-pm/claude_substrate_v0_backup.json, 20 records);
the restart dropped it (the known snapshot-wedge fragility — bundles count
5050→5046); restored post-deploy via `BUNDLE claude_substrate_v0 …` +
POST /v1/bundles/claude_substrate_v0/import ⇒
`{"status":"imported","count":20,"total":20,"curvature":0.23899417256975808}`;
export re-verified count 20.

## Deviations

- `Explainer` struct introduced in `gigi::explain` (new module) — the two
  executor arms had identical hand-copied logic; the asks were implemented
  once. No behavior change beyond the asks.
- Batch adds a `kind:"empty"` note row for found-but-undecomposable records —
  not in the ask text, but the alternative (an invisible group) violates the
  spirit of the no-silent-skip ruling. Documented in the executor doc
  comments; the letter covers misses only, since the empty case cannot occur
  on v2 (all 392 fibers numeric with count ≥ 2).
- `EXPLAIN SECTION b (…)` (insert form) now fails at parse with a clear error
  instead of parsing and Notice-ing at execution. EXPLAIN of an insert was
  always nonsense; the executor Notice for non-PointQuery inners (EXPLAIN
  COVER …) is unchanged.
- Everything else matches the ask text and rulings.

## Commits (this branch, base 87b6700)

1. tests(explain-errors): RED — typed NOT_FOUND + 393-field regression pins
2. impl(explain-errors): GREEN — sentinel + 404 mapping, shared executor
3. tests(explain-vector): RED — kappa_v anchors
4. impl(explain-vector): GREEN — kappa_v + VECTOR clause
5. tests(explain-batch): RED — IN (…) groups, miss entries, per-group invariant
6. impl(explain-batch): GREEN — batch executor, Explainer ctx cache
7. tests(explain-mmap): RED — mmap real rows
8. impl(explain-mmap): GREEN — polymorphic on-demand stats
9. docs(marcella): reply letter + this report
