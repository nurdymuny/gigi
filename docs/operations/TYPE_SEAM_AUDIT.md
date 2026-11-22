# Type Seam Audit

Date: 2026-08-02. Repo state: main @ 4f9c2a4 (working tree). Read-only audit; this file is the only write.

## 1. Scope

This audit was motivated by the vector-brain gap fixed at commit bb1d8e4: brain endpoints read fibers through `extract_field_samples` (src/stream_shared.rs), which matched only `Value::Float`/`Integer`; `Value::Vector` fell to a catch-all `_` arm whose fail-open hardening silently dropped every record, so a fully-vector bundle answered `/brain/*` with HTTP 200 and `n_samples: 0`. That bug is one instance of a class — a partial match over `Value`/`FieldType` variants with a permissive default arm, downstream of an insert path that performs no value-vs-FieldType validation — and this audit sweeps the rest of the codebase for the same class along four axes: (a) `Value`/`FieldType` match seams in the engine, (b) documentation-vs-parser drift in GQL_REFERENCE.md and README.md, (c) endpoints that answer HTTP 200 with empty or zero results indistinguishable from real answers, and (d) the ingest-to-consumer matrix (what INGEST produces vs what each consumer can read). Every finding below was traced end-to-end in source and carries file:line evidence; the docs-vs-engine findings were additionally verified empirically against a freshly built debug binary (built 2026-08-02, post-dating the last parser.rs edit) with the full feature set — roughly 80 statements executed, every claimed error captured. Findings reported independently by more than one hunt are merged here with all evidence retained. Already-known issues (the fixed bb1d8e4 bug itself, the missing `gnss_geodesic` example, flaky registry/WAL tests under parallel load, and SIMILAR's known doc-vs-parser drift) are excluded per the audit brief; SIMILAR appears only inside the sibling-set finding M-33, which enumerates the other 29 verbs in the same state.

Totals: 72 raw findings across four hunts, merged to **64 distinct findings: 15 high, 39 medium, 10 low**. All 64 are CONFIRMED by a verification pass; none remain at lead status (see section 3).

---

## 2. Findings by severity

### HIGH

#### H-1. Bulk-write WHERE bypasses timestamp coercion — BULK RETRACT can delete every row

- **Surface:** GQL `BULK RETRACT` / `BULK REDEFINE`; REST `POST /v1/bundles/{name}/bulk-delete` and `PATCH /v1/bundles/{name}/points`; also trigger filters, virtual-bundle filters, SPECTRAL WHERE, `pattern_curvature`, and REST `/aggregate`'s filtered path.
- **Behavior:** Read verbs coerce WHERE literals aimed at TIMESTAMP fields (`coerce_conditions_to_schema`, called from `filtered_query` at src/bundle.rs:2351), but every bulk-write path evaluates raw conditions: `Engine::bulk_update` (src/engine.rs:1515) and `Engine::bulk_delete` (src/engine.rs:1568) call `matches_filter` directly, as do `BundleStore::bulk_update` (src/bundle.rs:1803) and `BundleStore::bulk_delete` (src/bundle.rs:3274) used by the REST handlers (src/bin/gigi_stream.rs:10466, 10636). With `ts` stored as `Value::Timestamp` and the literal parsed as `Value::Text('2026-01-01')` or `Value::Integer` (epoch-ms), `Value::cmp` falls to the type-tag catch-all (src/types.rs:83) where Timestamp(tag 5) > Text(tag 4) > Integer(tag 2) always. So `BULK RETRACT b WHERE ts > '2020-01-01'` silently deletes every record regardless of date, and `WHERE ts < '2026-01-01'` silently deletes zero — both returning a success Count.
- **Evidence:** src/types.rs:83 `_ => type_order(self).cmp(&type_order(other))`; src/bundle.rs:57-61 doc comment naming this exact silent-wrong-answer class; src/parser.rs:10741-10747 (BulkRetract builds conditions via `filter_to_query_conditions`, no schema access) → src/engine.rs:1566-1568 (`matches_filter` with raw conditions). Grep confirms `coerce_conditions_to_schema` is called only at src/bundle.rs:2351/2357 and src/mmap_bundle.rs:713 — no bulk-write caller. Trigger filters src/engine.rs:328, virtual bundles src/virtual_bundles.rs:187, spectral src/spectral.rs:1505, pattern_curvature src/parser.rs:9580, aggregation src/aggregation.rs:396 share the uncoerced evaluation.
- **Verdict:** CONFIRMED — traced end-to-end on both the GQL and REST paths; inserts do store `Value::Timestamp` (src/engine.rs:1596), so the constant-comparison is the live behavior. Destructive, silent, success-reporting.
- **Fix sketch:** Call `coerce_conditions_to_schema` at the top of `Engine::bulk_update`/`bulk_delete` and `BundleStore::bulk_update`/`bulk_delete` (all have `&self.schema`), and in `TriggerManager::evaluate_mutation`, `filtered_group_by`, and the spectral/pattern direct-match sites. Better: make `matches_filter` schema-aware, or coerce once at `QueryCondition` construction.

#### H-2. Brain confidence/intent_gate matrix built with `d = fields.len()` — misreads vector-expanded rows

- **Surface:** `POST /v1/bundles/{name}/brain/confidence`, `/brain/intent_gate` (query_grounding), `/brain/confidence_with_explain`.
- **Behavior:** Follow-on to the bb1d8e4 fix: `extract_field_samples` now returns rows of width sum-of-dims (384 for one vector(384) field), but the cache build still sets `d = fields.len()` (= 1) and flattens all rows into `data`. `MaterializedMatrix::new` only `debug_assert`s `data.len() == n*d`, so in release the matrix silently records n rows x 1 col over a buffer of n*384 values: row i is read as `data[i..i+1]` — component i of record 0's embedding. KDE/nearest then run over misread memory. With an explicit bandwidth in the request (skipping the fit that would 400), `/brain/confidence` on a vector bundle returns 200 with a plausible `n_samples` and garbage raw/normalized values. The companion query gate `query.len() != fields.len()` (src/bin/gigi_stream.rs:7025) also rejects a real 384-dim query with 400 — so the vector-visibility fix never actually reached these three endpoints.
- **Evidence:** src/bin/gigi_stream.rs:4608-4621 (`let d = fields.len();` then flatten); src/vector_cache.rs:104-110 (debug_assert only); callers at src/bin/gigi_stream.rs:6903, 7036, 7172; post-fix row width at src/stream_shared.rs:76-83, 108-110; reachability via explicit bandwidth at src/bin/gigi_stream.rs:7037-7038 (isotropic fit otherwise 400s at 5345-5361).
- **Verdict:** CONFIRMED — n x 1 view over an n x 384 buffer in release builds; response carries healthy-looking `n_samples` over garbage.
- **Fix sketch:** Derive d from the actual row width (`samples.first().map(|r| r.len())`), assert it in release (400/500 on mismatch), and validate query length against the expanded width, not `fields.len()`.

#### H-3. Edge sync stores every Vector fiber as Null under a Categorical schema

- **Surface:** Edge sync (cloud src/edge.rs push → gigi_edge ingest); any direct JSON insert into the edge binary.
- **Behavior:** The cloud side serializes Vector values as JSON arrays (src/edge.rs:475-477) and schema types as `"vector(N)"` (src/edge.rs:459). The edge binary's `json_to_value` has `_ => Value::Null` — JSON arrays (and objects) become Null — and `str_to_field_type` has `_ => FieldType::Categorical` — `"vector(4)"` (and `"binary"`) become Categorical. Syncing a vector bundle to an edge node succeeds with 200s while every embedding is silently stored as Null under a Categorical schema; a sync back up would then null the cloud copy too. This is the exact vector-blindness class of the fixed brain-endpoint bug, on the edge-sync surface, unfixed.
- **Evidence:** src/bin/gigi_edge.rs:127-139 (no Array arm; contrast src/bin/gigi_stream.rs:1077-1084 which converts numeric JSON arrays to `Value::Vector`); src/bin/gigi_edge.rs:168-173 (`_ => FieldType::Categorical`); sender at src/edge.rs:239-334, 459, 475-477; receiving handlers use the broken helpers at src/bin/gigi_edge.rs:199 (schema) and :270 (record insert).
- **Verdict:** CONFIRMED — both directions of the sync path verified in source.
- **Fix sketch:** Port gigi_stream's Array→Vector arm to gigi_edge; parse `vector(N)` in `str_to_field_type` and reject unknown type strings loudly instead of defaulting to Categorical.

#### H-4. FiberMetric has no Vector arm — embeddings priced by the 0/1 discrete metric; Null numeric sits at the origin

- **Surface:** GEODESIC verb (Dijkstra edge weights at src/spectral.rs:721, 786), partition function → free energy → thermodynamic profile (src/curvature.rs:1116) — anything calling `FiberMetric::distance` on a bundle with Vector fibers.
- **Behavior:** `component_distance` matches Numeric/Categorical/OrderedCat/Timestamp explicitly; `FieldType::Vector{..}` falls to the `_` arm ("Binary / fallback: discrete — 0 if equal, 1 otherwise"). Two 384-dim embeddings differing by 1e-9 in one component are at distance 1.0; bit-identical ones at 0.0. On an INGEST-produced vector bundle, GEODESIC silently degenerates toward hop counting (every vector field contributes exactly 0 or 1 per edge) and the partition function computes Boltzmann weights over an equality metric — plausible-looking numbers, HTTP 200, no note. Additionally the Numeric and Timestamp arms do `as_f64().unwrap_or(0.0)`, so a Null numeric fiber is silently at the origin: a record with a missing value is "near" records whose value is 0.
- **Evidence:** src/metric.rs:55-62 (fallback arm; verified verbatim in this audit); src/metric.rs:16-21 (Null→0.0); consumers src/spectral.rs:721, 786 and src/curvature.rs:1116, all called with the full `schema.fiber_fields` including Vector. Reachability nuance: adjacency comes from indexed fields (src/spectral.rs:280-304, src/bundle.rs:3135), so the standard embeddings-plus-indexed-metadata shape silently walks the graph with degenerate weights; a bare vector bundle with no indexed fiber returns no-path instead.
- **Verdict:** CONFIRMED — reported independently by two hunts; both traces agree.
- **Fix sketch:** Add a `(FieldType::Vector{..}, Value::Vector(a), Value::Vector(b))` arm computing normalized L2 or chord distance (src/dials.rs:438 `chord_d_sq` already has the convention), with a length-mismatch policy; treat Null in the Numeric/Timestamp arms as max-distance or skip the component rather than 0.0.

#### H-5. mmap/overlay bundles: PULLBACK, SELECT GROUP BY, REST /aggregate and /join return empty success

- **Surface:** GQL PULLBACK join; GQL `SELECT ... GROUP BY` (SQL compat); REST `POST /v1/bundles/{name}/aggregate`; REST /join; gigi_edge join endpoint.
- **Behavior:** These arms gate on `store.as_heap()`: PULLBACK does `match (l.as_heap(), r.as_heap()) { (Some, Some) => pullback_join(...), _ => Vec::new() }`; SELECT GROUP BY does `None => HashMap::new()`; REST /aggregate does `.as_heap().map(...).unwrap_or_default()` on both its filtered and unfiltered paths. After a snapshot restore, production bundles are mmap-backed (`BundleRef::Overlay`), for which `as_heap()` is None — the exact query that returned rows before a restart returns an empty row set / empty groups with HTTP 200 and no note, byte-identical to a genuine no-match. Every brain endpoint got the `heap_or_promote` (#107) fix; these did not.
- **Evidence:** src/parser.rs:11086-11088 (PULLBACK `_ => Vec::new()`), src/parser.rs:11120-11122 (GROUP BY `None => HashMap::new()`); src/bin/gigi_stream.rs:2633-2636 (/join), 2680-2688 (/aggregate); src/bin/gigi_edge.rs:622-624 (same pattern); `as_heap()` None for Overlay at src/mmap_bundle.rs:1360-1365; snapshot restore opens .dhoom as MmapBundle+Overlay at src/engine.rs:666-729. The correct pattern exists in the same file: src/bin/gigi_stream.rs:4076, 4330 return explicit "not heap-resident" errors; `heap_or_promote` at src/stream_shared.rs:201-214.
- **Verdict:** CONFIRMED — reported independently by two hunts; overlay reachability established via the boot path.
- **Fix sketch:** Promote to a temp heap store via the existing `heap_or_promote`, or return the explicit "bundle is mmap-resident; verb unavailable" error the SPECTRAL FULL arm already uses.

#### H-6. Overlay bundles report h1=0 "consistent" and zeros for spectral/betti/entropy/free-energy as computed results

- **Surface:** GET `/v1/bundles/{name}/consistency`, `/spectral`, `/betti`, `/entropy`, `/free-energy`; GQL CONSISTENCY / SPECTRAL (non-FULL) / HORIZON / DEPTH.
- **Behavior:** `consistency_h1_sampled` does `as_heap().map(...).unwrap_or_default()`: on a mmap/overlay bundle the contradiction scan never runs and the endpoint answers 200 `{"h1": 0, "status": "consistent"}` for a bundle that may be full of contradictions — served alongside a genuinely-computed curvature value (Overlay `scalar_curvature` works via `curvature_stats().mean()`), which makes the never-run scan look computed. The `BundleRef::Overlay` arms return holonomy 0.0, betti (0,0), entropy 0.0, spectral_gap 0.0, free_energy 0.0; REST `spectral_report` and the GQL non-FULL SPECTRAL arm serve these via `unwrap_or(0.0)` as results. Contrast: SPECTRAL FULL and SPECTRAL MODE MATRIX explicitly refuse with "bundle is not heap-resident" — the refusal pattern exists exactly one arm away.
- **Evidence:** src/bin/gigi_stream.rs:8869-8884 (consistency_h1_sampled), 8907-8916 (handler), 2845-2847 (spectral_report), 14089-14093 (GQL non-FULL) vs 14030-14032 and 14069-14071 (the two refusing arms); src/mmap_bundle.rs:1692 (holonomy), 1699 (betti), 1706 (entropy), 1713 (spectral_gap), 1720 (free_energy), 1675-1680 (curvature genuinely computed).
- **Verdict:** CONFIRMED.
- **Fix sketch:** Promote overlay to temp heap for these verbs, or return the existing "not heap-resident" error; at minimum add a `storage_mode`/`not_computed` marker to the responses.

#### H-7. INTEGRATE sum()/avg() emit 0.0 over non-numeric fields and empty bundles, with presence-diluted AVG

- **Surface:** GQL `INTEGRATE ... MEASURE` — library executor (src/parser.rs) and the duplicated arm in the stream binary — global and OVER groups; HTTP /v1/gql passthrough. (Reported independently by three hunts.)
- **Behavior:** `accumulate_measures` counts presence for `count` (any non-Null value) but only adds to `sum` when `as_f64` succeeds. Min/Max/Stddev/Variance were sentinel-gated to Null for exactly this case — the in-code comment says a presence-only accumulator "must surface Null, not a fake 0.0" — but Sum and Avg were left ungated: `AggFunc::Sum => Value::Float(agg.sum)` and `Avg => Value::Float(agg.avg())` emit `Float(0.0)` when the field is Vector (INGEST embedding) or Text (CSV column that voted categorical). `avg()` also returns 0.0 when count==0, so an empty bundle returns one row `{avg_price: 0.0}` with 200. Mixed columns silently dilute AVG because presence-count includes non-numeric rows. This lands on the exact bundle shape the ingest docs recommend ("embeddings ingest as first-class vectors", GQL_REFERENCE.md:1683).
- **Evidence:** src/aggregation.rs:19-25 (`avg()` 0.0 on empty — verified verbatim), 175-193 (presence vs numeric accumulation); src/parser.rs:11019-11038 (Sum/Avg ungated, Min/Max/Stddev/Variance gated on `agg.min.is_finite()`); src/bin/gigi_stream.rs:13957-13990 (duplicated arm, same gap, with the comment naming the failure at 13974-13981); field-NAME validation exists (13917-13936) but no type guard.
- **Verdict:** CONFIRMED — the half-applied sentinel proves the maintainers considered this exact class and missed Sum/Avg.
- **Fix sketch:** Track `numeric_count` separately from presence count in `AggResult`; gate Sum/Avg on `numeric_count > 0` with the same Null sentinel as Min/Max; divide avg by `numeric_count`.

#### H-8. Curvature machinery blind to Vector fibers and empty bundles: K=0, confidence 1.0, infinite capacity, anomalies off, empty metric tensor

- **Surface:** Insert/delete/bulk response `curvature`+`confidence` fields; GET `/curvature`, `/capacity`, `/horizon`, `/health`; GQL CURVATURE / CAPACITY / DEPTH; `compute_anomalies`; `metric_tensor`. (Merged from three hunts.)
- **Behavior:** `Value::as_f64` has no Vector arm, so `compute_record_k` skips every Vector fiber and `FieldStats` are never fed for them (single insert src/bundle.rs:1351-1357, batch/turbo 1474, 1784). Two degenerate inputs collapse to the same healthy-looking answer: (a) a fully-vector bundle — the flagship NPZ/JSONL embedding shape — prices every record kappa=0.0, `CurvatureStats` mean/std 0, `/curvature` returns k_mean=0.0, per_field=[], anomaly_rate=0.0 with 200, and `metric_tensor` returns the empty `MetricTensorInfo` without note; (b) an empty bundle — `scalar_curvature` returns 0.0 when `field_stats` is empty, so confidence = 1/(1+0) = 1.0 (maximum confidence over zero records), capacity = tau/0 = Infinity (serialized as JSON null), and the capacity dial reports regime "flat" with "No curvature barriers — every query resolves cleanly". GQL DEPTH classifies the empty bundle as depth I. None of these responses carries a record count on the default unscoped path, so an automated caller gating on confidence/regime (the Marcella refuse-gate pattern) reads an empty or invisible bundle as maximally trustworthy. Curvature-driven anomaly detection never fires on vector bundles. Same total-blindness class as the fixed brain bug, on the flagship K surface, with no equivalent refusal.
- **Evidence:** src/types.rs:181-188 (`as_f64` matches Integer/Float/Timestamp only); src/bundle.rs:1064-1066 (compute_record_k skip), 1078-1082 (returns 0.0 when n==0), 1351-1357/1474/1784 (stats feeds), 1572-1574, 2958-2999 (anomalies over all-zero scores); src/curvature.rs:12-32 (scalar_curvature), 36-38 (confidence(0)=1.0), 297-302 (capacity → INFINITY); src/dials.rs:651-653 (regime text, verified verbatim); src/metric.rs:107-135 (empty MetricTensorInfo); src/bin/gigi_stream.rs:2736-2803 (curvature_report — no record-count field in the struct), 9772-9812 (health), 10648-10654 (insert responses), 14248-14260 (DEPTH); all-Vector bundles reachable through public INGEST (src/ingest.rs:589-660, 956, 977). Only `/scan` pushes a "no numeric fibers" note; the curvature/anomaly/metric endpoints have nothing.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Fold per-component vector variance into FieldStats (src/geometry/bundle_stats.rs already computes VectorFieldStats — reuse it) or add the total-blindness refusal: if a bundle has fiber data but zero kappa-participating fields, say so. Add `record_count` to CurvatureReport/CapacityReport and a distinct "empty" regime (or 422) when `store.len()==0`, mirroring `/scan`'s honest empty message.

#### H-9. One NaN in a GAUGE_FIELD NPZ permanently poisons FieldStats; /scan lens silently zeroes; NaN serializes as JSON null

- **Surface:** Insert-time kappa, `/v1/bundles/{name}/scan` global lens, curvature/anomaly thresholds, JSON wire.
- **Behavior:** `ingest_npz_as_gauge_field` stamps fiber components as `Value::Float` straight from the decoded array with no finiteness screen, and `FieldStats::update` has no guard: one NaN makes `mean` NaN and the Welford delta arithmetic keeps it NaN for every later record. Every subsequent `compute_record_k` is NaN, `CurvatureStats` goes NaN, threshold comparisons are all false (anomaly detection silently off, anomaly_rate 0.0). In `/scan`, the global lens recomputes the same stats, gets all-NaN z-scores, and `z.max(0.0)` maps NaN to 0.0 — the whole lens returns 0.0 for every record with no note. On the wire, NaN k_mean serializes as JSON null (serde_json maps non-finite to Null) — silently. GAUGE_FIELD NPZ is the one public ingest door that admits NaN as a number (CSV/JSONL route non-finite through serde_json, which cannot represent it — that path hits M-14 instead); the feature-gated NPZ path is what the production Halcyon harvest pipeline runs, and the unguarded FieldStats is shared by CSV numeric and HTTP inserts.
- **Evidence:** src/ingest.rs:1606-1608 (no screen; zero `is_finite`/`is_nan` hits in all of ingest.rs); src/bundle.rs:874-889 (`update` unguarded); src/ml/scan.rs:266-269 (`z.max(0.0)` — NaN.max(0.0)==0.0 in Rust); src/wire.rs:16 (`json!(f)` non-finite → null).
- **Verdict:** CONFIRMED — permanent, silent, bundle-wide corruption.
- **Fix sketch:** Screen non-finite values at the NPZ decode boundary (reject or count-and-report) and/or guard `FieldStats::update` with `is_finite`; have `/scan` note when a lens's stats are non-finite instead of emitting zeros.

#### H-10. /anomalies: zero-match filters and nonexistent field names return clean verdicts dressed in whole-bundle stats

- **Surface:** `POST /v1/bundles/{name}/anomalies` (filters); `POST /v1/bundles/{name}/anomalies/field`.
- **Behavior:** `compute_anomalies` returns `Vec::new()` when the pre-filter matches nothing. The handler then answers 200 with `anomaly_count: 0` AND `total_records: store.len()` (the whole bundle) plus k_mean/k_std of the whole bundle (the filter is never passed to the stats) — so a typo'd filter value (`"emea"` vs `"EMEA"`) reads as "scanned 50k records, region is clean". No `n_matched` field exists to distinguish "filter matched 0" from "matched many, none anomalous". `/anomalies/field` is worse: `req.field` is never validated against the schema, so any nonexistent field name yields `anomaly_count: 0` — a guaranteed clean verdict on the alerting surface.
- **Evidence:** src/bundle.rs:2979-2981 (`if base_points.is_empty() { return Vec::new(); }`); src/bin/gigi_stream.rs:9125-9154 (handler, mixed scopes), 9876-9917 (field_anomalies; `req.field` used only inside `contributing_fields.contains()` at 9896, no schema check).
- **Verdict:** CONFIRMED — the whole-bundle stats actively dress the empty result as a completed clean scan.
- **Fix sketch:** Add `n_matched` (post-filter candidate count) to the response; 422 when a filter or field name references a field not in the schema (as `/scan/fit` already validates `label_field`).

#### H-11. /geodesic: distance 0.0 / path_found true for nonexistent records; always path_found false on overlay

- **Surface:** `POST /v1/bundles/{name}/geodesic`; GQL `GEODESIC FROM..TO`.
- **Behavior:** `geodesic_distance` short-circuits `Some(0.0)` when `bp_a == bp_b` BEFORE the existence check; `geodesic_path` has the identical pattern. Base points are computed by pure hash of the key record (no lookup), so from==to with a key that was deleted or never existed — or on a completely empty bundle — returns 200 `{"distance": 0.0, "path_found": true}`. Different nonexistent keys return `path_found: false`, indistinguishable from "records exist but are disconnected". On mmap/overlay bundles, the handler's `.unwrap_or(0)` fallbacks plus the Overlay geodesic arms returning None mean every request answers `{"distance": null, "path_found": false}` — including genuinely adjacent pairs — with no unsupported-storage signal.
- **Evidence:** src/spectral.rs:661-668 (equality short-circuit before `adj.contains_key`), 743-756 (geodesic_path, same); src/bundle.rs:2774-2775 (base_point is pure hash); src/bin/gigi_stream.rs:9016-9017 (`unwrap_or(0)`), 14282-14287 (GQL arm); src/mmap_bundle.rs:1751-1754 (Overlay → None).
- **Verdict:** CONFIRMED.
- **Fix sketch:** Check existence (`store.get_fiber(bp)`) before the equality short-circuit and return a distinct "record not found" error; refuse or promote on overlay instead of `unwrap_or(0)`.

#### H-12. GENERATE BASE (marked working in the reference) executes as a silent no-op returning OK

- **Surface:** GQL verb GENERATE BASE — CLI, embedded engine, tests; gigi-stream returns a generic notice at best.
- **Behavior:** Parses fine, returns bare OK, creates no bundle and no section stubs. Empirically: `GENERATE BASE gen1 FROM id=1 TO id=10 STEP 1;` → OK, then `COVER gen1 ALL;` → "Error: No bundle: gen1". GQL_REFERENCE.md marks GENERATE BASE working (line 1732) and the parity checklist counts it (line 2451). Contradicts the doc's own hardening claim that no-op statements never return a bare ok (GQL_REFERENCE.md:74, README.md:609-610).
- **Evidence:** src/parser.rs:11615-11618 (verified verbatim in this audit: `reject_virtual_write` then `Ok(ExecResult::Ok)`); empirical CLI run on the current build.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Implement stub generation, or return `ExecResult::Notice("GENERATE BASE is not implemented — nothing was created")` like ITERATE does, and correct the reference row.

#### H-13. TRANSPLANT executes as a silent no-op returning OK; documented MAP clause does not parse

- **Surface:** GQL verb TRANSPLANT — all execution paths.
- **Behavior:** `TRANSPLANT sensors INTO sensors_archive;` → OK, but `COVER sensors_archive ALL;` → "No bundle: sensors_archive". Nothing is copied, no target-exists check, success is reported. Additionally the section XIV MAP-rename form (`MAP (temp -> temperature)`) is rejected by the trailing-token guard: `parse_transplant` accepts only INTO/WHERE/RETRACT SOURCE.
- **Evidence:** src/parser.rs:11611-11614 (verified verbatim: bare Ok after virtual-write guard); parse_transplant src/parser.rs:8006-8027 (no MAP arm); trailing guard 8496-8516; GQL_REFERENCE.md:1768-1789, grammar line 2402. Empirical: OK then No bundle.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Loud Notice/error until implemented; add MAP to the parser or delete it from section XIV.

#### H-14. FILL executes as a silent no-op returning OK

- **Surface:** GQL verb FILL — all execution paths.
- **Behavior:** `FILL sensors ON temp USING TRANSPORT;` → OK. No gap-filling happens on any path. GQL_REFERENCE.md section XIII documents three USING methods with no status warning; the parity checklist counts FILL under "Generate series" as working (line 2451).
- **Evidence:** src/parser.rs:11619-11622 (verified verbatim); parse_fill 8052-8070 parses the documented forms. Empirical: OK, no effect.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Same treatment as GENERATE BASE: loud notice + status correction.

#### H-15. The documented flat positional SECTIONS batch silently inserts ONE record with all-NULL fields

- **Surface:** GQL verb SECTIONS (batch insert) — the exact form shown in GQL_REFERENCE.md section II line 510.
- **Behavior:** The reference's batch example puts N records as one flat value list in one paren pair. `parse_sections` pattern 3 treats that as a single row; the executor then names positional values `_0`, `_1`, ... which match no schema field (the "use schema field order" comment is false — no remapping exists), and `coerce_record_to_schema` never rejects unknown field names — so one record with every schema field NULL is inserted and OK is returned. Empirically: `SECTIONS pair (1, 2.0, 2, 3.0);` → OK; `COVER pair ALL;` shows a persisted `NULL | NULL` row. Silent data corruption on the documented primary batch form. The undocumented tuple form `SECTIONS pair (id, v) (3,4.0),(4,5.0);` works correctly.
- **Evidence:** src/parser.rs:3079-3092 and 3160-3174 (pattern 3 collects all values into one row), 10617-10622 (`format!("_{i}")` naming); src/types.rs:116-153 (no unknown-field rejection); src/engine.rs:1594+ (batch_insert stores it). Empirical NULL row persisted.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Chunk positional values by schema arity (or map `_i` → schema field i), reject value counts that aren't a multiple of the field count, and show the tuple form in the reference.

### MEDIUM — engine seams

#### M-1. SPECTRAL_GAUGE collapses Float vertex ids to vertex 0 via `as_i64().unwrap_or(0)`

- **Surface:** GQL SPECTRAL GAUGE / magnetic spectrum (Halcyon verb family).
- **Behavior:** `rec.get("vertex_a").and_then(|v| v.as_i64()).unwrap_or(0)` — `as_i64` accepts only Integer and Timestamp, not Float. Records whose vertex ids were ingested as JSON `1.0`/`2.0` (serde_json gives Float; json_to_value keeps Float; Numeric fields get no integer coercion at insert) all map to vertex 0: the whole graph silently folds into self-loops on one vertex and the spectrum is computed over the collapsed graph. The result envelope's vertex count is the only clue.
- **Evidence:** src/spectral.rs:1510-1511; src/types.rs:190-196; contrast the same function's typed `SpectralGaugeError` discipline for missing endpoint fields (src/spectral.rs:1480-1487).
- **Verdict:** CONFIRMED.
- **Fix sketch:** Accept Float ids with an integrality check (`as_f64` + `fract()==0`); return a typed error naming the record when an id is missing or non-integral.

#### M-2. Old `group_by` silently drops records and whole groups when the agg field is non-numeric

- **Surface:** REST `POST /v1/bundles/{name}/aggregate`; GQL `SELECT col, COUNT(field) GROUP BY` (SQL-compat arm).
- **Behavior:** Unless `agg_field == "*"`, a record whose agg field fails `as_f64` hits `None => continue` BEFORE the group entry is created — so COUNT over a categorical field returns an empty groups map (HTTP 200) instead of counting non-null values, and groups whose members are all non-numeric vanish entirely. The multi-measure INTEGRATE path was fixed with `group_by_measures` (whose doc comment states the correct COUNT(field) semantics); the REST endpoint and SELECT arm still use the old function. The SELECT arm additionally serializes `agg_result.min`/`max` (INFINITY sentinels) with no finite gate, relying on serde_json's non-finite→null.
- **Evidence:** src/aggregation.rs:85-91 (skip-before-entry), 120-148 (the fixed function and its doc); src/bin/gigi_stream.rs:2681, 2688; src/parser.rs:11120-11121, 11128-11137.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Route both callers through `group_by_measures`; apply the finite gate before serializing min/max.

#### M-3. REST filter specs with an unknown op silently degrade to equality

- **Surface:** Every REST endpoint taking ConditionSpec filters: /points query, PATCH bulk update, POST bulk-delete, /aggregate.
- **Behavior:** `condition_spec_to_query_condition` ends with `_ => QueryCondition::Eq(...)`. A typo'd or unsupported op ("ge", "gt ", "equals", "lte.") turns a range filter into an equality test — usually matching zero records — and the request succeeds. On the bulk-delete path this deletes whatever equals the boundary literal, returning success. Malformed `between` values also fall back to Eq.
- **Evidence:** src/bin/gigi_stream.rs:9921-9974 (exact-match op whitelist, Eq catch-all, between fallback at 9971); callers at 9992-9996, 2683-2687, 10444-10448, 10620-10624. No 422 path exists for unknown ops.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Return 422 naming the unknown op and the accepted list; make between's arity failure an error.

#### M-4. Encryption fails open across four seams: plaintext fallthrough, Null→0.0 fabrication, panic on tamper, Identity transforms from HTTP create

- **Surface:** Encrypted bundles (Affine/Probabilistic/Isometric/Opaque field modes) — insert and read paths; HTTP bundle creation. (Merged from two hunts.)
- **Behavior:** (1) `encrypt_value` for Affine (`other => other.clone()`) and Probabilistic (`_ => return v.clone()` — comment: "Non-numeric falls through.") silently stores a non-numeric value UNENCRYPTED; since inserts don't type-validate fibers, a Text value posted into an encrypted numeric field persists in the clear with HTTP 200. `default_for_type` also maps `FieldType::Vector` to Affine, which has no Vector arm — a vector fiber in an encrypted bundle stays plaintext at rest. (2) `encrypt_fiber`'s Isometric gather maps any non-numeric group member — including `Value::Null` for a simply-missing field (numeric default is Null, substituted at insert) — to 0.0; `decrypt_fiber` returns `Value::Float(~0.0)`: a missing measurement round-trips into a fabricated real value. (3) Opaque `decrypt_value` does `.expect("AEAD decrypt failed — ciphertext tampered or wrong key/AAD")` — a corrupt/tampered blob panics the thread instead of returning an error. (4) The HTTP create handler builds every FieldDef with encryption None and then derives the GaugeKey directly; `GaugeKey::derive` maps mode None to `FieldTransform::Identity` — so an `"encrypted": true` bundle created over HTTP with vector fibers stores everything plaintext; the stream's /query CreateBundle executor likewise never fills default modes before deriving.
- **Evidence:** src/crypto.rs:102-107, 118-125, 168-169, 264-271, 372-377, 481-488; src/types.rs:260-266; src/bundle.rs:1320, 1566-1575; src/bin/gigi_stream.rs:1937-1944, 1962-1965, 12739-12823.
- **Verdict:** CONFIRMED — all four seams verified in source.
- **Fix sketch:** Enforce numeric-only values at insert for Affine/Probabilistic/Isometric fields; give Affine a Vector arm or map Vector to Opaque; keep Null as Null through Isometric groups (presence mask); convert the Opaque `expect` into a Result surfaced as 4xx; make the HTTP create path fill default encryption modes before deriving keys.

#### M-5. ML suite zero-fills Null/missing numeric features with `unwrap_or(0.0)` at nine sites

- **Surface:** /ml cluster, reduce, solve, prescribe, predict (feature matrix + mu/sd), changepoints series, scan amount field.
- **Behavior:** Feature extraction is uniformly `r.get(&fd.name).and_then(|v| v.as_f64()).unwrap_or(0.0)`. CSV INGEST stores empty cells as `Value::Null`, so bundles with missing data get silent zero-imputation: cluster centroids dragged toward the origin, OLS/target vectors treating gaps as literal 0, changepoint statistics seeing a step at every run of missing values — all with HTTP 200 and no note. infer's standardization then mean-imputes using a mu already biased by the zero-fill — internally inconsistent.
- **Evidence:** src/ml/cluster.rs:342; src/ml/reduce.rs:70; src/ml/solve.rs:85, 94-95; src/ml/prescribe.rs:100; src/ml/infer.rs:221, 267, 329, 364 (and the inconsistent `unwrap_or(*mu)` at 233); src/ml/changepoints.rs:105, 139; src/ml/scan.rs:571, 599; src/ingest.rs:783-790 (Null ingress).
- **Verdict:** CONFIRMED (pattern spot-verified verbatim at cluster.rs:342 and infer.rs:221/233).
- **Fix sketch:** Skip records with missing features (as `fit_full_covariance` does) or mean-impute consistently, and report an imputed/skipped count in each endpoint's notes array — the notes plumbing already exists.

#### M-6. changepoints/scan auto time-selection is blind to `FieldType::Timestamp` — series analyzed in storage order

- **Surface:** /ml changepoints (auto time detection); /ml scan velocity lens.
- **Behavior:** changepoints builds its time-candidate list from `matches!(f.field_type, FieldType::Numeric)` only, so a real TIMESTAMP fiber named "date" is never auto-selected; `time_field` stays None, the sort is skipped, and changepoints are computed over storage-iteration order — plausible-looking boundaries on a meaningless ordering, HTTP 200, no note. (Explicitly naming `time` works.) scan's velocity lens has the same Numeric-only filter and then tells the user "velocity skipped: needs ... a time-like numeric field" even when a genuine Timestamp fiber exists. The sort itself handles Timestamp fine via `as_f64` — inclusion in the candidate list is all that's missing.
- **Evidence:** src/ml/changepoints.rs:62-63, 72, 87-96; src/ml/scan.rs:209, 449-456.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Include `FieldType::Timestamp` in the candidate lists; when no time field is found, refuse or note "analyzing in insertion order".

#### M-7. Brain extractors reject schema-valid TIMESTAMP fibers with a misleading corruption message; scan lens covers a different field set than insert-time kappa

- **Surface:** `/v1/bundles/{name}/brain/*` (intent_gate, confidence, attend, episodic), topology/persistence, full-covariance fit; /scan global lens. (Merged from two hunts.)
- **Behavior:** The fixed `extract_field_samples` scalar arms accept only `Value::Float`/`Integer`; `Value::Timestamp` — which `coerce_record_to_schema` guarantees is what TIMESTAMP fibers store — falls to the skip arm, so a Timestamp fiber causes every record to be skipped and the total-blindness refusal fires with "values in field X do not match its schema type ... scalar fields need Float/Integer" — blaming corruption for schema-valid data. The full-covariance fit's copy (`_ => all_present = false`) skips all such records and errors "found 0 (sparse records were skipped)" although the records are complete. /brain/episodic 400s with "non-numeric value (only Float / Integer supported)" for the same case. Relatedly, /scan's `num_defs` filter matches `FieldType::Numeric` only, so Timestamp fibers are excluded from the global curvature lens even though insert-time kappa includes them via `as_f64` — the two curvature computations silently cover different field sets.
- **Evidence:** src/stream_shared.rs:96-126, 134-142; src/bin/gigi_stream.rs:5142-5151, 5166-5171, 7522-7530; src/types.rs:116-154; src/ml/scan.rs:208-210 vs src/bundle.rs:1064.
- **Verdict:** CONFIRMED — fail-closed rather than silent, but a genuine functional break with misleading blame.
- **Fix sketch:** Add `Some(Value::Timestamp(t)) if field_cols[slot] == 1 => row.push(*t as f64)` to both extractors (matching `as_f64`'s numeric family), or reject Timestamp fields at field-resolution time with an accurate message; include Timestamp in scan's `num_defs` or document the exclusion.

#### M-8. Dials locus projection zero-fills missing scalars and whole missing vectors

- **Surface:** GET dials `locus=`/`fields=` scoped statistics (k-NN cosine over the scoped space); WINDOWED_COHERENCE fiber scope.
- **Behavior:** `scope_projection` pushes `unwrap_or(0.0)` for a missing/non-numeric scalar and `_ => out.extend(repeat(0.0).take(dims))` for a record whose Vector field is absent, Null, or the wrong variant. Records with missing embeddings become the all-zeros vector in the cosine-chord k-NN — wholly-zero projections hit the degenerate-norm guard and sit at fixed distance 1.0, still in the pool, entering the k=64 neighbor set whenever the population is small; partially-missing records get genuinely skewed non-max distances. Scoped statistics shift silently; no skipped-count reporting.
- **Evidence:** src/dials.rs:413-431, 443-444, 559-570, 1088.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Return `Option<Vec<f64>>` from `scope_projection`, exclude records with missing scoped fields from the neighbor pool, and report a skipped count, mirroring `extract_field_samples`' skip-and-log contract.

#### M-9. `ts_literal` claims loud validation, but unparseable timestamp text silently compares as type tags

- **Surface:** All read verbs using `filtered_query` with WHERE on TIMESTAMP fields (the already-coerced path).
- **Behavior:** When the literal text is not ISO 8601 (`'01/15/2026'`, `'garbage'`), `ts_literal` returns None, `coerce_conditions_to_schema` leaves the raw Text condition in place, and the comparison falls to constant type-tag ordering (matches all rows for Gt, none for Lt). The doc comment claims "unparseable — the executor validates loudly before we get here", but no such validator exists: project-wide grep shows exactly two non-test `parse_iso_ms` callers — `ts_literal` itself (silent None) and the write-path coercion (the only loud error).
- **Evidence:** src/bundle.rs:44-55; src/types.rs:83, 130-138.
- **Verdict:** CONFIRMED — the comment is aspirational, not descriptive.
- **Fix sketch:** Make `coerce_conditions_to_schema` return Result and error when a Text literal aimed at a Timestamp field fails `parse_iso_ms`, propagating a 422 naming the accepted formats (message already exists in types.rs:133-138).

#### M-10. /brain/attend zip-truncates the query against vector-expanded sample rows

- **Surface:** `POST /v1/bundles/{name}/brain/attend` (and other extract-based comparisons of query to samples).
- **Behavior:** `attend()` computes d_sq via `s.iter().zip(query.iter())` — Rust zip silently truncates to the shorter side. The handler validates `query.len() == fields.len()`, but post-bb1d8e4 sample rows for a vector(384) fiber are 384-wide while the accepted query is 1-wide. Result: HTTP 200 with softmax attention weights computed from only the first embedding component of each record — wrong weights presented as a full answer, healthy-looking `n_samples`. Requires explicit bandwidth in the request (the isotropic fit otherwise 400s on vector fields).
- **Evidence:** src/geometry/attention.rs:46-50, 80; src/bin/gigi_stream.rs:7309-7319, 7346, 5345-5361.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Validate query length against the expanded sample width after extraction; make attend/focus error on `s.len() != query.len()` instead of zip-truncating.

#### M-11. /spectral response has no n/reason field: empty, disconnected, and overlay all read as identical zeros

- **Surface:** GET `/v1/bundles/{name}/spectral`; GQL SPECTRAL (non-FULL).
- **Behavior:** `spectral_gap` returns 0.0 for n<2, for multi-component graphs, and (via `unwrap_or`) for overlay bundles. The response `{lambda1: 0.0, diameter: 0, spectral_capacity: 0.0}` is served 200 with no record count or reason, so "bundle is empty", "bundle is mmap-backed", and "graph is genuinely disconnected" are indistinguishable computed-looking answers. The FULL variant returns `n_vertices` — the honest envelope exists one arm away.
- **Evidence:** src/spectral.rs:315-325; src/bin/gigi_stream.rs:2845-2853, 14072-14082 (FULL), 14089-14093.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Add `n_records` and a reason enum (empty | disconnected | computed | overlay_unsupported) to SpectralReport.

#### M-12. predict_volatility never validates field/group_by; a typo returns 200 with `predictions: []`

- **Surface:** `POST /v1/bundles/{name}/predict`.
- **Behavior:** The handler only accumulates when `record.get(&req.field).and_then(as_f64)` succeeds. A misspelled, non-numeric, or Vector field name means no group entry is ever created, and the response is 200 `{"predictions": []}` with the typo'd field echoed back — no error, no record count. A typo'd group_by silently collapses everything into one group keyed "null".
- **Evidence:** src/bin/gigi_stream.rs:9834-9871 (no schema check anywhere in the handler; contrast /solve and /infer, which validate).
- **Verdict:** CONFIRMED.
- **Fix sketch:** 422 when `req.field`/`req.group_by` are not schema fields; include `n_records_scanned` in the response.

#### M-13. GQL scalar wire silently turns Infinity/NaN into null with no shape warning

- **Surface:** `POST /v1/gql` — CAPACITY, HORIZON, and any scalar verb producing a non-finite f64.
- **Behavior:** `ExecResult::Scalar(v)` is serialized via `serde_json::json!({"value": v})`; serde_json renders non-finite f64 as null. CAPACITY on a flat or empty bundle deliberately computes tau/0 = INFINITY, and the caller receives 200 `{"value": null}` — a wire answer that looks like a missing value rather than the documented "infinite capacity" semantics, indistinguishable from any NaN-producing degenerate case. The REST capacity dial wraps the same value in regime text; the GQL verb hands back bare null.
- **Evidence:** src/bin/gigi_stream.rs:15399, 14236-14246; src/curvature.rs:297-302; src/dials.rs:651-653.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Special-case non-finite scalars on the wire: `{"value": null, "non_finite": "inf"}` or a string form plus a note field, so callers can tell "flat/infinite" from "not computed".

#### M-14. One non-numeric CSV cell ('NaN', 'inf', a typo) silently flips the whole column to Categorical

- **Surface:** `INGEST ... FORMAT CSV` → every numeric consumer downstream.
- **Behavior:** `dhoom::coerce` parses 'NaN'/'inf' successfully as f64 but `Number::from_f64` returns None for non-finite, so the cell becomes a JSON String; any String/Bool cell votes the column non-numeric, and then EVERY value in the column — including the clean numbers — is stored as `Value::Text(n.to_string())`. INGEST returns success (IngestStats reports only records_emitted/bundle_created/bytes_read — no inferred column types, no warning), and the column is now invisible to kappa, FieldStats, /scan lenses, and numeric metric distance, while INTEGRATE avg/sum on it return the fake 0.0 of H-7.
- **Evidence:** src/dhoom.rs:215-244, 3240-3263; src/ingest.rs:745-756, 791-799, and the three IngestStats return sites (820, 1008, 1620).
- **Verdict:** CONFIRMED.
- **Fix sketch:** Report inferred column types (or a notes list naming which columns went Categorical and the first offending cell/row) in IngestStats; treat 'NaN'/'inf' cells explicitly (Null or loud error) rather than as type-flipping strings.

#### M-15. EMIT CSV → INGEST CSV round trip silently demotes Vector and Timestamp columns to text

- **Surface:** `EMIT CSV TO` + `INGEST FORMAT CSV` — the engine's own documented export/import pair.
- **Behavior:** `csv_cell` serializes via Display: `Value::Timestamp(ms)` renders as `T1234567890`, `Value::Vector` as `[1,2,3]` (quoted), `Value::Float(NaN)` as `NaN`. On re-ingest, none parse as numbers (the primitive-array sentinel requires a \x1F prefix EMIT never writes), so those columns silently become Categorical Text — timestamps lose time semantics, embeddings lose vector-ness — and an emitted 'NaN' cell additionally flips an otherwise-clean numeric column (M-14). No error at either end; the export is not round-trippable through the import.
- **Evidence:** src/parser.rs:10269-10306, 10366; src/types.rs:156-177; src/dhoom.rs coerce path; src/bin/gigi_stream.rs:13578.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Emit Timestamp as epoch-ms integer (or ISO) and NaN as an empty cell; either refuse to EMIT Vector columns to CSV or emit a form ingest recognizes.

#### M-16. INGEST can never feed a TIMESTAMP field; the conflict contradicts the engine's own insert coercion

- **Surface:** INGEST CSV/JSONL into a bundle with a TIMESTAMP field; auto-created bundles.
- **Behavior:** CSV/JSONL inference only ever yields Numeric/Categorical/Vector, and `types_compatible` accepts Timestamp only against Timestamp, so ingesting epoch-ms integers into a pre-created `ts TIMESTAMP` bundle always fails with SchemaConflict "existing=Timestamp, incoming=Numeric" — before a single record is inserted — even though INGEST's own `flush_batch` calls `engine.batch_insert`, which runs `coerce_record_to_schema`, which happily coerces exactly those integers (and ISO text) to `Value::Timestamp`. The coercion machinery sits directly on INGEST's own insert path but is unreachable because the schema gate fires first. The workaround (auto-create) silently stores ISO date strings as Categorical Text, losing all time semantics with no note.
- **Evidence:** src/ingest.rs:1072-1081, 1144-1153, 758-770, 950-961, 1012-1024; src/engine.rs:1594-1617; src/types.rs:116-154.
- **Verdict:** CONFIRMED — loud error on the main path, silent demotion on the workaround path.
- **Fix sketch:** In `ensure_bundle_compatible`, treat inferred Numeric (and ISO-parsable Categorical) as compatible with an existing Timestamp field and route the batch through `coerce_record_to_schema`, mirroring the insert path.

#### M-17. GQL CREATE BUNDLE silently maps unknown type words — including VECTOR — to Categorical

- **Surface:** GQL CREATE BUNDLE, then INGEST NPZ/HTTP inserts against the resulting schema.
- **Behavior:** `parse_field_spec` accepts ANY word as the field type, and `spec_to_field_def`'s catch-all turns any unrecognized type word into Categorical without error: `FIBER (emb VECTOR)` — or a typo like FLAOT — creates a Categorical field. The HTTP schema path DOES parse `vector(768)` into `FieldType::Vector`, so the two surfaces disagree. Downstream, INGEST NPZ into the GQL-created bundle fails with SchemaConflict "existing=Categorical, incoming=Vector(dims=15552)" — an error that contradicts the DDL the user wrote — and HTTP inserts of embeddings are rejected by `schema_coerce`. The same silent catch-all exists in the app-bundles manifest loader (that one at least eprintln-warns).
- **Evidence:** src/parser.rs:2805-2807, 8533-8546; src/bin/gigi_stream.rs:1211-1227, 1157-1164, 15786-15797; src/ingest.rs:1144-1153.
- **Verdict:** CONFIRMED — acceptance is silent; the eventual error blames the wrong party.
- **Fix sketch:** Add a VECTOR(dims) arm to the GQL field-type grammar (parity with HTTP) and make the catch-all a parse error naming the accepted type words.

### MEDIUM — documentation vs engine

All findings in this group were verified both by reading the parser dispatch (src/parser.rs:2303-2577 and the relevant parse/execute arms) and empirically against a freshly built debug binary carrying the full feature set.

#### M-18. README Quick start Path A is unrunnable: CREATE BUNDLE and SECTION AT forms both fail to parse

- **Surface:** README "Your first 10 minutes with GIGI" — /v1/gql and CLI.
- **Behavior:** Step 2 `CREATE BUNDLE sensors FIBER (…) KEYS (sensor_id);` → "Expected '(' here, found 'FIBER'" (the token KEYS appears nowhere in src/). Step 4 `SECTION sensors AT (sensor_id='S-001');` → "Expected a name here, found '('" — parenthesized AT is not accepted. The same forms recur in the day-one table (README.md:82) and section 6.1 (490, 599). A new user's first two copy-pastes both fail.
- **Evidence:** README.md:369, 381; src/parser.rs:6263-6275, 6343 (expect LParen), 6661+ (parse_kv_pairs requires bare word). Empirical: both statements error exactly as above.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Rewrite the quickstart to the real grammar (`CREATE BUNDLE sensors (sensor_id TEXT BASE, temp NUMERIC FIBER, …)`; `SECTION sensors AT sensor_id='S-001';`) or teach the parser KEYS/paren-AT.

#### M-19. Seven checkmarked verbs do not exist: LENS, RICCI, CONFIDENCE, PROFILE, PREDICT, TRANSLATE, SNAPSHOT

- **Surface:** GQL statement dispatch — every execution path.
- **Behavior:** All are marked working in GQL_REFERENCE.md and all fail: LENS/RICCI/CONFIDENCE/PROFILE/PREDICT/TRANSLATE each → "Unknown statement"; `SNAPSHOT sensors AS 'pre';` → "Expected the keyword GAUGE_FIELD here" (SNAPSHOT exists only as the Halcyon `SNAPSHOT GAUGE_FIELD <n> PERSIST` form, and only under the gauge feature). `Statement::Ricci` and `Statement::Predict` enum variants exist but are never constructed by any parse function. TRANSLATE's own example also uses double quotes the tokenizer cannot lex.
- **Evidence:** src/parser.rs:2303-2577 (no arms), 857, 874, 12123-12141 (dead "must be executed via HTTP" arm), 3654-3656 (SNAPSHOT requires GAUGE_FIELD), 2190 (tokenizer); GQL_REFERENCE.md:39, 54, 71, 999, 1087, 1592, 1629, 1643. Empirical errors captured for each.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Flip these rows to not-implemented (or implement); remove or wire the dead Ricci/Predict enum variants.

#### M-20. "This table is enforced" is false — the truth test covers ~30 statements and none of the failing rows

- **Surface:** GQL_REFERENCE.md status table + tests/gql_reference_truth.rs.
- **Behavior:** The reference claims the truth test "runs one statement per ✅ row … and fails CI when a row and the engine disagree." The test's `works` array is exactly 30 statements (point reads, covers, writes, INTEGRATE variants, one PULLBACK, SHOW/DESCRIBE/EXPLAIN, CURVATURE/SPECTRAL/HEALTH, JACKKNIFEs, SHOW FIELDS). LENS, RICCI, TRANSLATE, DIVERGENCE, ATLAS beyond BEGIN/COMMIT/ROLLBACK, RETURNING, FILTER, EMIT DHOOM, GENERATE BASE, ITERATE, SECTIONS, GAUGE TRANSFORM, encryption rows, CAPACITY/HORIZON/DEPTH, and the fiber HOLONOMY/TRANSPORT/SPECTRAL forms are all absent — which is exactly how the dozen-plus marked-working drifts survived. The enforcement claim is the meta-bug.
- **Evidence:** GQL_REFERENCE.md:31-35; tests/gql_reference_truth.rs:48-89 (the complete list).
- **Verdict:** CONFIRMED.
- **Fix sketch:** Extend the works/honest-gap lists to genuinely cover every marked-working row, or soften the claim to name the covered subset.

#### M-21. GAUGE TRANSFORM — the documented schema-migration verb — does not parse; GAUGE VERIFY likewise

- **Surface:** GQL verb GAUGE (schema migration), marked working in status row 1 and section I.
- **Behavior:** `GAUGE sensors TRANSFORM (ADD altitude NUMERIC RANGE 5000 DEFAULT 0);` → "Expected CONSTRAIN, UNCONSTRAIN, VS, or ROTATE_KEY, got TRANSFORM". The SQL→GQL map row "ALTER TABLE t → GAUGE t TRANSFORM (…)" is therefore wrong; the only real schema evolution is the undocumented `ALTER BUNDLE <b> ADD BASE <f> <type>` ("ALTER BUNDLE" appears nowhere in the reference). `GAUGE VERIFY sensors;` fails identically. GAUGE CONSTRAIN parses but the executor answers "gauge-constraint statements parse but are not enforced yet" while section XVII is headed as working.
- **Evidence:** src/parser.rs:7483-7597, 2666-2683, 11512-11518; GQL_REFERENCE.md:92, 2040. Empirical errors captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Document ALTER BUNDLE ADD BASE as the real migration path and mark GAUGE TRANSFORM not-implemented, or implement TRANSFORM over the machinery in src/gauge.

#### M-22. RETURNING (marked working, own section) is not in the parser at all

- **Surface:** SECTION / SECTIONS / REDEFINE / RETRACT … RETURNING — GQL surface.
- **Behavior:** Every RETURNING example in section II errors as unsupported trailing input. The parity checklist repeats "RETURNING … On SECTION, REDEFINE, RETRACT" (line 2449). The functionality exists only as HTTP endpoints (POST /v1/bundles/{name}/update and /delete with returning), not in GQL.
- **Evidence:** grep RETURNING src/parser.rs → zero matches; src/bin/gigi_stream.rs:11198, 11340; src/bundle.rs:4104, 4113; GQL_REFERENCE.md:556-579, 2330-2333, 2449. Empirical trailing-input errors.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Mark not-implemented and point readers at the HTTP endpoints, or add a RETURNING suffix to the four write verbs.

#### M-23. INTEGRATE loses four documented capabilities: FILTER, multi-field OVER, RESTRICT TO, RANK BY/FIRST — plus unscrubbed HAVING examples

- **Surface:** GQL verb INTEGRATE.
- **Behavior:** All four documented forms fail with trailing-input errors: `count(*) FILTER (WHERE temp < 0)` (section V and parity line 2450), `OVER region, city` (parse_integrate takes exactly one OVER field), `RESTRICT TO (COVER …)`, `RANK BY avg_t ASC FIRST 5`. The section V body also still shows HAVING in several examples plus the TRANSLATE sample output, although the status table correctly marks HAVING as not implemented.
- **Evidence:** src/parser.rs:4766-4848; GQL_REFERENCE.md:662-673, 695, 698-705, 708-715, 726, 1007, 2434, 2450. Empirical errors for all four forms.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Fix section V to the real surface (single OVER field, no FILTER/RESTRICT/RANK/HAVING) or implement; delete the stale HAVING examples.

#### M-24. COVER's computed projections, the function library, the CONFIDENCE filter, and IN-subqueries all fail

- **Surface:** GQL verb COVER + the whole section XV Built-in Functions chapter.
- **Behavior:** PROJECT is a bare identifier list, so computed projections (`temp_max - temp_min AS x`, `CLASSIFY…`, `RESOLVE(…)`, `CONFIDENCE()`) error — which voids the ~60 functions in section XV that the parity checklist marks working (2444-2448). `COVER … CONFIDENCE >= 0.95` → trailing-input error. `WHERE ANOMALY() = TRUE` / `Z_SCORE() > 3` / `HAVING CURVATURE(f) > v` → "Expected comparison operator after 'ANOMALY', got LParen". `ON city IN (INTEGRATE …)` and `IN (cold PROJECT city)` → IN accepts literals only.
- **Evidence:** src/parser.rs:3250-3337, 6761-6877, 6905-6923; GQL_REFERENCE.md:635, 662-678, 2444-2448. Empirical errors captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Scrub sections IV/XV down to what parses (names, literals, MATCHES/VOID/DEFINED/IN/NOT IN/BETWEEN/CONTAINS) or build an expression grammar for PROJECT/WHERE.

#### M-25. WITH / WITH RECURSIVE CTEs claimed working are Unknown statements; the one real subquery form is undocumented

- **Surface:** GQL WITH statement.
- **Behavior:** `WITH cold AS (COVER sensors ON city='Moscow') COVER cold;` → "Unknown statement: 'WITH'". Parity checklist claims "Nested covers + WITH + WITH RECURSIVE"; sections IV and VII show CTE examples with no warning. The honest not-implemented rows (PRODUCT/UNION/INTERSECT/SUBTRACT) also still have unmarked example blocks. The only implemented subquery form, `EXISTS (COVER b WHERE …)` inside filters, appears nowhere in the reference.
- **Evidence:** src/parser.rs:2303-2577 (no arm), 6736-6748; src/bin/gigi_stream.rs:13590+; GQL_REFERENCE.md:657-659, 670-673, 870-875, 2350, 2437. Empirical Unknown-statement error.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Correct the parity row and annotate the CTE examples; document EXISTS(COVER …).

#### M-26. PULLBACK chain joins and AS self-join aliases (both shown as working) fail

- **Surface:** GQL verb PULLBACK.
- **Behavior:** `PULLBACK orders ALONG customer_id ONTO customers ALONG region ONTO regions;` — the second ALONG is swallowed as right_field and the second ONTO becomes a trailing-input error. `PULLBACK sensors AS s1 ALONG region ONTO sensors AS s2;` → "Expected the keyword ALONG here, found 'AS'". Only single-hop `PULLBACK a ALONG f ONTO b [PRESERVE LEFT]` works.
- **Evidence:** src/parser.rs:4952-4981; GQL_REFERENCE.md:861-862, grammar 2343. Empirical errors for both forms.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Loop `(ALONG f ONTO b)+` with optional AS aliases, or reduce section VII to the single-hop form.

#### M-27. ATLAS SAVEPOINT / ROLLBACK TO / ISOLATION / ON ERROR / SHOW LAST ERROR all fail; embedded path is transaction theater

- **Surface:** GQL transactions (ATLAS).
- **Behavior:** `parse_atlas` accepts exactly BEGIN|COMMIT|ROLLBACK. `ATLAS SAVEPOINT cp1;` → "Unknown ATLAS action: SAVEPOINT"; `ATLAS BEGIN ISOLATION FLAT;` → trailing-input error; `SHOW LAST ERROR;` → "Unknown SHOW target: LAST". Parity claims "ATLAS with FLAT/CURVED isolation". AtlasBegin/Commit/Rollback also execute as bare Ok on the embedded path ("handled at the transport layer"), and gigi-stream just answers {status: ok} — CLI users get transaction theater.
- **Evidence:** src/parser.rs:6251-6259, 7037, 11493-11496; src/bin/gigi_stream.rs:12925-12929; GQL_REFERENCE.md:919-951, 2439. Empirical errors captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Trim section VIII to BEGIN/COMMIT/ROLLBACK and drop the SAVEPOINT/ISOLATION/ON ERROR prose, or implement them.

#### M-28. EMIT DHOOM / JSON / WITH / BARE (marked working) all rejected — only EMIT CSV TO exists

- **Surface:** GQL output control (EMIT wire-format suffix).
- **Behavior:** `COVER sensors ON city='Moscow' EMIT DHOOM;` → "EMIT supports FORMAT CSV only (got 'DHOOM')". Same for JSON and the WITH CURVATURE/CONFIDENCE metadata forms. Section X is headed as working and design principle 4 says "EMIT controls DHOOM serialization"; the grammar documents the full form. Reality: the only EMIT is the file-export suffix `EMIT CSV TO '<path>'` gated on GIGI_EMIT_DIR.
- **Evidence:** src/parser.rs:8466-8488; GQL_REFERENCE.md:16, 977-981, 1708-1728, 2413. Empirical errors captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Rewrite section X around EMIT CSV TO; move DHOOM wire-format material to the HTTP docs where it lives.

#### M-29. All six rich SUBSCRIBE variants don't exist; UNSUBSCRIBE takes a bundle name, not a sub_id

- **Surface:** WebSocket subscription protocol (SUBSCRIBE is not a GQL statement at all).
- **Behavior:** The WS handler supports only `SUBSCRIBE <bundle> [WHERE …]` and `SUBSCRIBE <bundle> ON K [> t]`. The documented `SUBSCRIBE ANOMALIES sensors ON temp` parses the whole tail as a bundle name → "ERROR: Bundle 'ANOMALIES sensors ON temp' not found"; same failure shape for CURVATURE…DRIFT, CONSISTENCY, SPECTRAL, PHASE, DIVERGENCE…THRESHOLD. The doc's `UNSUBSCRIBE sub_id` is wrong — the code removes by bundle name. Sending SUBSCRIBE through /v1/gql fails as Unknown statement.
- **Evidence:** src/bin/gigi_stream.rs:12046-12132; src/parser.rs:2303-2577; GQL_REFERENCE.md:957-967.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Document the two real WS forms (+ ON K) and the bundle-name UNSUBSCRIBE; drop or implement the six event-typed variants.

#### M-30. BETTI returns beta0+beta1 summed into one unlabeled scalar; doc promises separate beta0, beta1, beta2

- **Surface:** GQL verb BETTI (marked working) — embedded and stream.
- **Behavior:** `BETTI sensors;` → `2.000000`. Both executors return `Scalar(b0 + b1)`; beta2 is never computed and beta0=2,beta1=0 is indistinguishable from beta0=1,beta1=1 — a reader following section XI ("beta1 > 0 means non-contractible loops") will misread the number. The halcyon dispatcher only intercepts BETTI with `ORDER k`, so production `BETTI b;` also gets the sum. `BETTI sensors ON city, region;` (also documented) is a trailing-input error; the working `BETTI b ORDER k` form is in README but not the reference.
- **Evidence:** src/parser.rs:5598-5607, 11755-11758; src/bin/gigi_stream.rs:14158-14161; src/bin/halcyon_gql_dispatch.rs:531; GQL_REFERENCE.md:197; README.md:513. Empirical scalar 2.000000.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Return a row {beta_0, beta_1} (and document ORDER k); never sum distinct invariants into one number.

#### M-31. ITERATE marked working twice, but the executor is an explicit not-implemented stub

- **Surface:** GQL verb ITERATE (recursive joins).
- **Behavior:** Parses and answers "NOTICE: ITERATE is not implemented yet — nothing was executed". Section VII heads it "Recursive Joins — ITERATE" as working with five example variants; parity row "Recursive queries … ITERATE" (2438). (The notice itself is the honest pattern H-12/H-13/H-14 should follow; the finding here is the doc status.)
- **Evidence:** src/parser.rs:8243+, 11703-11706; GQL_REFERENCE.md:866, 2438. Empirical notice captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Flip both doc rows to in-progress until the executor lands.

#### M-32. DIVERGENCE's primary documented form (FROM/TO) fails; the VS parser accepts any junk separator

- **Surface:** GQL verb DIVERGENCE (marked working, status line 55).
- **Behavior:** `DIVERGENCE FROM sensors TO sensors;` → bundle_a becomes "FROM", the separator word is unchecked (`self.expect_word()?; // VS`), bundle_b becomes "TO", and the real second name becomes a trailing-input error. The status row and section XI lead with the FROM/TO form. Only `DIVERGENCE a VS b` works (HTTP-only execution) — and because the separator is never verified, `DIVERGENCE a NONSENSE b` also parses.
- **Evidence:** src/parser.rs:2509-2515, 12128; src/bin/gigi_stream.rs:12959+; GQL_REFERENCE.md:55, 1428, 1435, 1438. Empirical error captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Accept both spellings (peek FROM → parse FROM a TO b) and `expect_keyword("VS")` on the other arm.

#### M-33. ~30 unmarked section XI verbs are all Unknown statements — SIMILAR's full sibling set enumerated

- **Surface:** GQL analytics verbs listed in the SQL→GQL table ("50+ GQL-only operations") and section XI.
- **Behavior:** Verified Unknown-statement (or equivalent parse failure) for every one of: SECTIONAL, SCALAR, DEVIATION, TREND, BOTTLENECK, CLUSTER, MIXING, CONDUCTANCE, LAPLACIAN, WILSON, EULER, COCYCLE, COBOUNDARY, TRIVIALIZE, CHARACTERISTIC, MUTUAL, TEMPERATURE, PHASE, CRITICAL, PARTITION, CALIBRATE, FLOW, SIMILAR (already known), CORRELATE, SEGMENT, OUTLIER, DOUBLECOVER, RECALL, COMPLETENESS, DIFF. These sections carry no status marker, but they sit between marked-working sections and are counted in "Score: 45 SQL operations matched. 50+ GQL-only operations" (line 224), so a reader has no way to tell them from the live verbs.
- **Evidence:** src/parser.rs:2303-2577 (none has an arm — all 30 checked); GQL_REFERENCE.md:145-224, 1047, 1636. 30/30 empirical probes errored.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Add an explicit not-implemented/design-spec marker to every one of these sections (the FIBER-window section already models the honest pattern).

#### M-34. FREEENERGY's documented TOLERANCE syntax rejected (real form is AT); ENTROPY ON/BY forms fail

- **Surface:** GQL verbs FREEENERGY and ENTROPY (marked working, status line 62).
- **Behavior:** `FREEENERGY sensors TOLERANCE 0.1;` → "Expected the keyword AT here, found 'TOLERANCE'". The verb requires `FREEENERGY sensors AT 0.1;` (works, verified) which appears nowhere in the docs; the `BY city` variant fails too. Same class for ENTROPY: `ENTROPY sensors ON status;` / `ON temp BY city` → trailing-input error; only global `ENTROPY sensors` works. In-family inconsistency: CAPACITY/HORIZON do accept TOLERANCE.
- **Evidence:** src/parser.rs:5630-5633, 5755-5769, 5775-5779; GQL_REFERENCE.md:186, 1419-1420, 1477, 1482. Empirical: TOLERANCE error, AT success, ENTROPY ON error.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Accept TOLERANCE as an AT alias (family consistency) or fix the doc to AT; document what ENTROPY actually accepts.

#### M-35. SPECTRAL t ON <field> and legacy HOLONOMY t AROUND (f1,f2) die on misleading errors despite working status

- **Surface:** GQL verbs SPECTRAL and HOLONOMY.
- **Behavior:** `SPECTRAL sensors ON region;` → "Expected '(' here, found ';'" because parse_spectral consumes any word after ON as if it were FIBER without checking. The grammar documents `("ON" field)?`. `HOLONOMY sensors AROUND (city, region);` (shown as working, and in the mapping table) → "Expected the keyword CYCLE here, found '('" — the AROUND arm now belongs to the Halcyon gauge-cycle form.
- **Evidence:** src/parser.rs:5102-5106, 7768-7809 (AROUND expects CYCLE at 7774-7775); GQL_REFERENCE.md:1219, 2356, 2358. Empirical errors captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Validate the FIBER keyword and emit a named error ("SPECTRAL ON <field> is not supported; use SPECTRAL <b> or ON FIBER (…) MODES k"); delete or re-implement the legacy holonomy form.

#### M-36. BUNDLE OPTIONS block, CONSTRAINT block, and inline CHECK/IN/REFERENCES modifiers all fail to parse

- **Surface:** GQL schema definition (BUNDLE / CREATE BUNDLE), sections I and XVII marked working.
- **Behavior:** `OPTIONS (STORAGE AUTO, TOLERANCE 0.1)` → trailing-input error; `total NUMERIC REQUIRED CHECK (total > 0)` → "Expected ',' here, found 'CHECK'"; same for `IN ('a','b')`, `REFERENCES customers(id)`, `ARITHMETIC`, `NULLABLE`; `CONSTRAINT (CHECK (a >= b))` → trailing-input error. Section XVII is headed "Constraints" as working and the parity row claims "CHECK, UNIQUE, REFERENCES, MORPHISM, IN, EXCLUDE". What actually parses per field: RANGE, DEFAULT, AUTO, UNIQUE, REQUIRED, INDEX, ENCRYPTED[mode] — plus bundle-level INVARIANT/ADJACENCY/WITH ENCRYPTION SEED (the latter two undocumented).
- **Evidence:** src/parser.rs:2582-2658, 2819-2861; GQL_REFERENCE.md:2040, 2044-2076, 2440. Six empirical parse errors captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Cut sections I/XVII to the seven real modifiers + INVARIANT, or implement the missing ones; verify UNIQUE/REQUIRED enforcement before leaving them marked working.

#### M-37. Section XI TRANSPORT (marked working) re-lists the OF-forms the status table already declares not-implemented

- **Surface:** GQL verb TRANSPORT — doc-internal contradiction.
- **Behavior:** Status row 49 honestly says "FIBER / TRANSPORT (window) … rejected loudly", but the section XI TRANSPORT entry shows `TRANSPORT sensors OF temp FROM id=42 TO id=100` and both SHIFT forms, repeated at line 1312 and the LAG/LEAD mapping rows. Empirically `TRANSPORT sensors OF temp ALONG wind SHIFT -1;` → "Expected '(' here, found 'temp'" — parse_transport blindly consumes 'OF' as if it were FROM. Only `TRANSPORT b FROM (k=v) TO (k=v) ON FIBER (…)` parses (HTTP-execution only).
- **Evidence:** src/parser.rs:7602-7618, 12125; GQL_REFERENCE.md:49, 126-127, 1312-1314, 1526-1532. Empirical error captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Delete the OF-forms from section XI and the LAG/LEAD mapping rows; validate separator keywords so wrong forms get named errors.

#### M-38. Embedded/CLI path silently OKs access-control and prepared-statement verbs the doc says are 501

- **Surface:** GQL WEAVE/GRANT/REVOKE/POLICY/AUDIT and PREPARE/EXECUTE/DEALLOCATE on the CLI and embedded engine.
- **Behavior:** Over gigi-stream these correctly return HTTP 501, matching the doc. But the embedded executor returns bare `Ok(ExecResult::Ok)` with no notice — empirically `GRANT COVER ON sensors TO analyst;` → "OK" on the CLI. A user believes a permission was granted; nothing was stored. Violates the repo's own no-bare-ok contract (README.md:609-610).
- **Evidence:** src/parser.rs:11499-11509, 11625-11628; src/main.rs:451; src/bin/gigi_stream.rs:12931-12957. Empirical CLI GRANT → OK.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Return the same explicit Notice the Set/Backup stubs use.

#### M-39. README day-one INTEGRATE shape is wrong; the GIGI Lang example emits non-parsing GQL

- **Surface:** README claimed-real operations ("a real operation the engine implements, not an aspirational future").
- **Behavior:** `INTEGRATE temp OVER sensors COVER ALL;` (README.md:85, 493) has bundle/field reversed and a COVER ALL clause that doesn't exist → trailing-input error, empirically verified — under the header claiming every row is real. Section 6.7's "emitted GQL" (772-783) uses `INSERT INTO conversations FIELDS (…)`, an `EMBED('…', MODEL=…)` function, parenthesized SECTION AT, and `EMIT DHOOM` — none of which parse (FIELDS exists only for SUGGEST_ADJACENCY/SHOW FIELDS; EMBED appears nowhere).
- **Evidence:** README.md:76-77, 85, 493, 772-783; src/parser.rs:6096, 6956, 8471-8476. Empirical errors captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Correct to `INTEGRATE sensors OVER city MEASURE avg(temp);`; mark the GIGI Lang block as aspirational output of a future translator.

### LOW

#### L-1. predict treats Vector/Timestamp targets as categorical class labels via Display strings

- **Surface:** /ml predict (classification branch).
- **Behavior:** `is_reg = matches!(tf.field_type, FieldType::Numeric)`; any other target type — including Vector and Timestamp — enters the classification branch where classes are `format!("{}", value)` strings: a Vector target yields one "class" per distinct embedding (labels like `[0.113,0.98,...]`), a Timestamp target classes like `T1690000000000`, predicted with confidence and HTTP 200. Output is visibly absurd rather than silently plausible, hence low.
- **Evidence:** src/ml/infer.rs:204, 419-422; src/types.rs:163-173.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Reject Vector targets with 422; treat Timestamp targets as regression (as_f64 family) or reject with a naming error.

#### L-2. DHOOM wire encoder silently drops nested objects and non-primitive arrays to empty string

- **Surface:** DHOOM encode (JSON→wire) for records with nested objects or arrays-of-objects in a field position.
- **Behavior:** `value_to_dhoom`'s final arm `Value::Array(_) | Value::Object(_) => String::new()` serializes any non-primitive array or object field as the empty string; decode reads it back as empty Text. A round-trip silently discards that field's structure with no warning in EncodeResult (which already carries diagnostics). Primitive arrays are handled via the \x1F sentinel; only nested-object shapes hit this.
- **Evidence:** src/dhoom.rs:247-271 (catch-all at 270).
- **Verdict:** CONFIRMED.
- **Fix sketch:** Emit a warning entry in EncodeResult, or JSON-stringify the value under the sentinel escape.

#### L-3. circulation weight field silently defaults non-numeric weights to 1.0

- **Surface:** /ml circulation (weighted flow decomposition).
- **Behavior:** `.and_then(|v| v.as_f64()).unwrap_or(1.0)` — a weight field whose values are Text or Null (or a misspelled weight field name entirely) gives every edge weight 1.0: the Hodge decomposition runs unweighted while the response names the weight field. The skipped-record counter covers missing endpoints and self-loops, not degraded weights.
- **Evidence:** src/ml/circulation.rs:96-101.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Validate the weight field is Numeric/Timestamp up front (the endpoint already 422s on missing fields); count/report weight-defaulted edges.

#### L-4. SPECTRAL fiber reads zero-fill Null/missing components

- **Surface:** SPECTRAL_GAUGE ON FIBER (...), MODE MATRIX weights, helicity/density weights.
- **Behavior:** Fiber component extraction uses `.and_then(|v| v.as_f64()).unwrap_or(0.0)`: a record whose named component is Null, Text, or Vector contributes a silent 0.0 link value to the Laplacian/observable instead of being skipped or reported — the spectrum shifts with no indication any input was non-numeric. Inputs here are usually machine-generated harvest data, so low; but no skipped/defaulted tally exists in any result envelope.
- **Evidence:** src/spectral.rs:843, 1518-1521, 2096-2103; related Null-as-zero convention in src/metric.rs:16-21.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Count and report zero-filled components in the SPECTRAL result envelope, or skip such records the way the fixed brain extractor does.

#### L-5. CURVATURE … WITHIN (cover) shown as working does not parse

- **Surface:** GQL verb CURVATURE.
- **Behavior:** `CURVATURE sensors ON temp WITHIN (COVER sensors ON region='EU');` → trailing-input error. Section XI and the grammar both document WITHIN; ON multi-field and BY work. Loud failure, doc-only drift.
- **Evidence:** src/parser.rs:5067-5093; GQL_REFERENCE.md:1023, 2354. Empirical error captured.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Remove WITHIN from the section and grammar, or implement the scoped variant.

#### L-6. FISHER's documented un-parenthesized syntax fails; verb is feature-gated out of default builds with no doc note

- **Surface:** GQL verb FISHER (post_kahler_phase1 feature).
- **Behavior:** The reference shows `FISHER sensors ON temp, humidity;` → "Expected '(' here, found 'temp'" (README shows the correct parenthesized form). The BY variant is unsupported. On a default-features build (Cargo `default = []`) FISHER is Unknown statement entirely; the Dockerfile builds with the feature, but the reference never mentions the gate.
- **Evidence:** src/parser.rs:2477-2478, 5008-5016; Cargo.toml:27; Dockerfile:11; GQL_REFERENCE.md:1457, 1461; README.md:593. Empirical error on the doc's exact syntax.
- **Verdict:** CONFIRMED (empirical).
- **Fix sketch:** Unify doc syntax with the parser (accept both) and annotate feature-gated verbs in the reference.

#### L-7. Brain guide's "all endpoints require heap-resident bundles (404 on mmap)" is stale post-#107

- **Surface:** HTTP /v1/bundles/{name}/brain/* consumer documentation.
- **Behavior:** BRAIN_PRIMITIVES_CONSUMER_GUIDE.md states "All require the bundle to be heap-resident (the engine returns 404 if it's only on-disk mmap)" (line 55) and repeats it in the failure-modes table (line 535). The #107 `heap_or_promote` fix makes brain endpoints polymorphic over heap and mmap+overlay (used at ~18 call sites; README line 93 advertises this). A consumer following the guide builds unnecessary touch-a-record workarounds. Some non-brain kahler endpoints (e.g. holonomy_debt) do still 404 on mmap, so the claim is wrong for brain/* and accidentally right elsewhere. The brain_explain docstring at src/bin/gigi_stream.rs:7635 is also stale.
- **Evidence:** src/stream_shared.rs:179-214; src/bin/gigi_stream.rs:7647-7650, 4330-4337; BRAIN_PRIMITIVES_CONSUMER_GUIDE.md:55-58, 535.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Update the guide and the docstring to the #107 behavior; note which non-brain endpoints still require heap.

#### L-8. Reverse drift: COMPLETE/PROPAGATE/SUGGEST_ADJACENCY and triggers marked not-wired ARE wired

- **Surface:** GQL verbs COMPLETE / PROPAGATE / SUGGEST_ADJACENCY / ON-BEFORE-AFTER triggers.
- **Behavior:** Status rows say "Parsed; sheaf module built but not wired" and "TriggerManager built but not wired", but the executors call `crate::sheaf::complete/propagate/suggest_adjacency` and `engine.create_trigger/drop_trigger`, and inserts evaluate triggers on mutation. Readers skip capabilities that exist. Caveat: on mmap-only bundles the sheaf verbs silently return zero rows via `None => Vec::new()` — an empty answer indistinguishable from "no completions" (the H-5 pattern).
- **Evidence:** src/parser.rs:11363-11419 (including the mmap fallbacks at 11383, 11401), 11709-11743; src/engine.rs:1348; GQL_REFERENCE.md:72-73, 2095.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Update the two status rows; make the mmap fallback a notice instead of empty rows.

#### L-9. Implemented-but-undocumented surface: working verbs and clauses absent from GQL_REFERENCE.md

- **Surface:** GQL_REFERENCE completeness.
- **Behavior:** Parser accepts, executor handles, reference never mentions: filter forms `CONTAINS`, `BETWEEN lo AND hi`, `NOT IN (…)`, `EXISTS (COVER b WHERE …)`; `ALTER BUNDLE <b> ADD BASE <f> <t>`; `BETTI b ORDER k`; `HORIZON … LENGTH_SCALE SPECTRAL_GAP|WELFORD_RADIUS|FIXED n`; DEPTH threshold overrides; `GEODESIC … MAX_HOPS n RESTRICT TO b`; `SPECTRAL b ON FIBER (h) MODE MATRIX [DIAGONAL d] [FULL [LIMIT k]]`; `HOLONOMY f AROUND CYCLE AXIS|EDGES`; PERCEIVE; HELICITY; METRIC <bundle>; WASSERSTEIN/PERSISTENCE/REEB; DEFINE PATTERN/HUNT/SHOW PATTERNS; EXPLAIN SECTION … VECTOR/IN; CREATE SESSION; the full Halcyon family (LATTICE/GAUGE_FIELD/GIBBS_SAMPLE/E_FIELD/SYMPLECTIC_FLOW/LOOP/LOOP_TRANSPORT/SNAPSHOT GAUGE_FIELD/SPECTRAL_GAUGE/CHERN_CLASS/PONTRYAGIN/PI_1/OBSTRUCTION) — README documents most of the Halcyon set; the language reference does not.
- **Evidence:** src/parser.rs:2303-2577 dispatch vs the reference's verb inventory; src/parser.rs:6736-6748, 6790-6839 (filter forms), 2666, 5600, 5804, 5991, 7768-7809.
- **Verdict:** CONFIRMED (spot-checked per item).
- **Fix sketch:** One pass adding a "shipped but new" section to the reference, or generate the verb inventory from SUGGESTABLE_VERBS + dispatch.

#### L-10. Encryption spec describes the v0.1 affine/permutation design, not the shipped v0.2–v0.4 suite

- **Surface:** GIGI_GEOMETRIC_ENCRYPTION_SPEC.md vs shipped GaugeKey modes. (No file named GIGI_ENCRYPTION_SPEC.md exists; this is the closest match.)
- **Behavior:** The spec's key model (section 3.1 FieldTransform: scale/offset for numeric, `permutation_key` for text, bit-flip for bool) and section 2.3 claims ("WHERE field > literal … compare encrypted literal", "GROUP BY … use deterministic encryption") describe the v0.1 design. Shipped reality is the six-mode suite (Affine / Opaque AES-GCM-SIV / Indexed CMAC / Probabilistic / Isometric / Identity) — text is AEAD-opaque or PRF-indexed, never permuted; order-preserving comparison on encrypted numerics is not a shipped query path. No status column is wrong per se, but a reader implementing against section 3 builds the wrong client. The GQL encryption surfaces the reference marks working all verified working (inline ENCRYPTED modes, WITH ENCRYPTION SEED hex, GAUGE … ROTATE_KEY FORWARD_SECRET, PROJECT INVARIANT).
- **Evidence:** GIGI_GEOMETRIC_ENCRYPTION_SPEC.md:111-124, 145-181; src/types.rs:228-246; src/crypto.rs:8-9, 19-23, 672+; src/parser.rs:2688-2739, 2851-2858, 5641-5679, 7580-7592. Empirical CLI: the four GQL surfaces all OK.
- **Verdict:** CONFIRMED.
- **Fix sketch:** Stamp the file as superseded by theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md, or refresh sections 2.3/3 to the shipped modes.

---

## 3. Unverified leads

None. Every finding in section 2 carries a CONFIRMED verdict from an independent verification pass (source trace for all 64; additionally ~80 empirical statement executions against a freshly built binary for the docs-vs-engine set). No finding remains at PLAUSIBLE/lead status. Two systemic observations from the sweeps that were deliberately NOT filed as findings, recorded here so they are not lost: (a) the heap insert path performs no value-vs-FieldType validation (only Timestamp coercion) — this is the root that makes the poisoned-row and fail-open seams (M-4, several zero-fill sites) reachable at all; (b) sheaf adjacency's `as_f64` silent-skip was judged consistent with its numeric-adjacency semantics and intentionally not reported.

---

## 4. Coverage notes (verbatim, per hunt)

The four hunts' own coverage statements, unedited, so the tail of this report is honest about what was and was not swept.

### Hunt: value-match-seams

> Swept: types.rs Value/FieldType (Ord/Hash/as_f64 family), GQL executor verb arms in parser.rs (INTEGRATE, SELECT compat, PULLBACK, BULK RETRACT/REDEFINE, pattern_curvature), the duplicated GQL arms + REST handlers in bin/gigi_stream.rs (bulk update/delete, /aggregate, condition specs, json_to_value, brain episodic, full-covariance fit), aggregation.rs, metric.rs (FiberMetric + metric tensor), curvature.rs (scalar_curvature, partition_function), spectral.rs (SPECTRAL_GAUGE assembly + FiberMetric edge weights), ml/* (cluster, reduce, infer, solve, prescribe, factorize, circulation, changepoints, scan), dials.rs, crypto.rs (all five transforms + value_to_bytes/bytes_to_value), hash.rs (total — no seam), wire.rs (total), dhoom.rs, edge.rs + bin/gigi_edge.rs (sync both directions), discrete/pk_http.rs + geometry/pk_http.rs (both use the fixed extractor), stream_shared.rs (verified the bb1d8e4 fix + its remaining Timestamp gap), sheaf/mod.rs (as_f64 silent-skip adjacency — judged consistent with numeric-adjacency semantics, not reported), ingest.rs (CSV/JSONL/NPZ typing), bundle.rs condition coercion + bulk paths + FieldStats, engine.rs bulk/trigger paths, geometry/bundle_stats.rs (vector-aware, clean). Cross-checked which condition-evaluation sites bypass coerce_conditions_to_schema (filtered_query and mmap query_filtered coerce; everything else does not). NOT covered: gauge/* module internals beyond inject/dispatch spot-checks, sharded/* (execution/fiedler/laplacian), imagine/*, patterns/http.rs, transactions/*, wal.rs replay decoding, mmap_bundle.rs record decoding internals, join.rs, bin/gigi_convert.rs and bin/gigi_server.rs, halcyon dispatch beyond the topology arms, and doc-vs-parser drift in GQL_REFERENCE.md (beyond the known SIMILAR case). One systemic root I did not file separately: heap insert performs no value-vs-FieldType validation (only Timestamp coercion), which is what makes the poisoned-row/fail-open seams (crypto plaintext fallthrough, skip-arms) reachable in the first place.

### Hunt: docs-vs-engine

> Swept: GQL_REFERENCE.md end-to-end (status table, SQL→GQL map, all 26 sections, EBNF, parity checklist) diffed against src/parser.rs's full top-level dispatch (lines 2303-2577) and the relevant parse_*/execute arms, plus the gigi-stream statement dispatch and WS SUBSCRIBE handler; README.md primer/quickstart/§6.1/§6.7; BRAIN_PRIMITIVES_CONSUMER_GUIDE.md (all 12 endpoints route-verified, auth + heap claims checked); GIGI_GEOMETRIC_ENCRYPTION_SPEC.md §2-§3 (the closest match to the task's "GIGI_ENCRYPTION_SPEC.md" — no file of that exact name exists). Findings were EMPIRICALLY verified against a freshly-built (2026-08-02, post-dating the last parser.rs edit) debug gigi.exe carrying the full feature set — ~80 statements executed; every claim marked "empirical" has a captured error/output. Feature-gate nuance: default build (Cargo `default = []`) additionally lacks FISHER/SNAPSHOT/LATTICE-family; prod Docker builds with the full flag set. NOT covered: GQL_SPECIFICATION.md and GQL_ADDENDUM_v2.1.md (not named in the task); numeric verification of encryption invariant-preservation claims and of holonomy/JACKKNIFE response-column names (parse+execute verified, per-field response shapes not diffed); UNIQUE/REQUIRED enforcement semantics; SQL-compat INSERT/SELECT paths; openapi.json vs route table; SDKs, e2e, dashboards; WS protocol verbs other than SUBSCRIBE/UNSUBSCRIBE; HTTP surface beyond brain routes. Known-issues list (vector extract_field_samples bug, gnss_geodesic, flaky WAL tests, SIMILAR-as-flagged) excluded as instructed — SIMILAR appears only inside the sibling-set finding.

### Hunt: silent-empty-200s

> Swept: all ML REST handlers (/scan /scan/fit /cluster /infer /reduce /solve /factorize /changepoints /prescribe /circulation) plus their src/ml cores — these are well-guarded (422 on empty/degenerate, /scan returns an honest 'bundle is empty' message with n:0); the analysis REST family (/curvature /spectral /consistency /betti /entropy /free-energy /geodesic /metric /anomalies /anomalies/field /health /predict /aggregate /divergence); dials (capacity/horizon — locus-miss is a clean 404 and the scoped path echoes n_records; only the default unscoped path lacks counts); GQL verb arms INTEGRATE/CURVATURE/SPECTRAL/CONSISTENCY/BETTI/ENTROPY/FREE_ENERGY/CAPACITY/HORIZON/DEPTH/GEODESIC/FISHER/WASSERSTEIN/PERSISTENCE/REEB (PK arms are guarded — empty cohorts and <2 points error); pk_http REST modules (guarded); patterns/hunt (guarded, v0.2 envelope carries verdict+reason); the brain vector-cache path and extract-based brain endpoints (attend, confidence, intent_gate, distance_to_fit_mean — fit-based endpoints refuse empty bundles via 'no observations', and n_samples is present where it matters); stream_shared extractor post-fix semantics. NOT audited: the WebSocket text-verb surface (handle_ws_command ~line 12046), sharded/* endpoints (spectral_gap/curvature/holonomy_loop), WISH, imagine_coherence internals, causal_states/commutator, quantum_cohomology, holonomy_debt, flat_transport, verify_invariant, brain dream/forecast/reconstruct/inpaint/sudoku/episodic/semantic beyond fit-guard reasoning, public GQL bundle-scoping differences, EMIT CSV, and the gigi_server.rs binary (separate surface). The Betti/Pi1/Obstruction GQL arms are flagged UNREACHABLE from HTTP (dispatcher in halcyon_gql_dispatch.rs handles them) — I did not audit the dispatcher's own empty-input behavior.

### Hunt: ingest-consumer-matrix

> Swept: all three ingest producers end-to-end (NPZ generic incl. multi-array Null fill, NPZ GAUGE_FIELD, CSV via dhoom::coerce, JSONL) plus the type-inference/compat gates (types_compatible, ensure_bundle_compatible); consumers verified by reading code, not just grepping: FiberMetric + its GEODESIC/partition-function callers, compute_record_k/FieldStats/CurvatureStats and the /curvature+anomaly HTTP surface, metric_tensor, /scan lens construction (global lens, num_defs filter, NaN behavior), INTEGRATE MEASURE + SELECT agg paths, dials scoping/projection (handles Vector correctly; Null-as-0.0 noted), spectral fiber reads, encrypt modes (Affine/Opaque/Indexed/Probabilistic/Isometric fiber paths), wire.rs JSON serialization of non-finite floats, EMIT CSV round trip, and the fixed brain extractor (only the NEW Timestamp gap reported). NaN ingress traced honestly: the CSV/JSONL unwrap_or(NAN) fallbacks are effectively dead code (serde_json cannot carry NaN without arbitrary_precision), so the live NaN path is NPZ float payloads. NOT covered: mmap bundle read paths (INGEST refuses mmap loudly), sharded execution, DHOOM binary encoder round-trip of Binary/Timestamp modifiers, kahler-gated geometry/imagine consumers, join.rs/COVER WHERE predicate coercion against Text-ified numbers, ml/infer-solve-factorize internals beyond scan, WAL replay of Vector schemas, and virtual bundles.

---

## 5. Recurring patterns

The 64 findings cluster into six shapes. Fixing the shape fixes the family.

1. **Catch-all `_` arms over `Value`/`FieldType` that fail open.** The bb1d8e4 class, still live at: `Value::cmp` type-tag fallback (H-1, M-9), FiberMetric discrete fallback (H-4), edge `json_to_value`/`str_to_field_type` (H-3), crypto plaintext fallthrough (M-4), `spec_to_field_def` Categorical default (M-17), DHOOM empty-string arm (L-2), brain-extractor Timestamp skip (M-7). Every partial match over these enums needs either full variant coverage or a typed refusal — never a permissive default.

2. **`as_f64()/as_i64().unwrap_or(x)` zero/one-fill.** Null, Text, and Vector silently become 0.0 (nine ML sites M-5, dials M-8, spectral fiber reads L-4, metric Null-at-origin H-4, Isometric gather M-4), 0 (SPECTRAL_GAUGE vertex ids M-1, geodesic base points H-11), or 1.0 (circulation L-3). The convention converts missing data into confident wrong numbers. Skip-and-count is the already-established house pattern (`extract_field_samples`); apply it uniformly.

3. **`as_heap()` None → empty success on overlay/mmap.** PULLBACK/GROUP BY//aggregate//join (H-5), consistency/spectral/betti/entropy/free-energy zeros (H-6), sheaf verbs (L-8), geodesic unwrap_or(0) (H-11). Post-snapshot production bundles are overlay-backed, so this is the normal prod storage mode, not an edge case. Both correct patterns already exist in-tree (`heap_or_promote`, the "not heap-resident" refusal); they are just unevenly applied.

4. **Degenerate inputs answered 200 with computed-looking values.** Empty bundle → K=0/confidence 1.0/infinite capacity (H-8); zero-match filter → clean anomaly verdict with whole-bundle stats (H-10); nonexistent records → distance 0.0 (H-11); INTEGRATE avg 0.0 (H-7); no n/reason fields on /spectral (M-11); Infinity→null on the scalar wire (M-13). Responses need the count of what was actually scanned, and refusals need to be distinguishable from zeros.

5. **Half-applied fixes.** Min/Max/Stddev/Variance sentinel-gated but not Sum/Avg (H-7); `group_by_measures` fixed but two callers still on the old `group_by` (M-2); heap_or_promote applied to brain but not /aggregate (H-5); vector expansion applied to the extractor but not the matrix build, query-length gates, or attend (H-2, M-10); Vector arm added but not Timestamp (M-7); two SPECTRAL arms refuse on overlay, the rest return zeros (H-6). When a class-fix lands, sweep the siblings in the same pass — this audit is effectively that sweep for bb1d8e4.

6. **Doc drift with no enforcement, in both directions.** Twelve-plus marked-working rows fail (M-19 through M-37), three verbs are silent no-ops behind working marks (H-12/13/14), the truth test covers ~30 statements while claiming full enforcement (M-20), and two capabilities that DO work are marked not-wired (L-8). The single highest-leverage fix in this family is M-20: make tests/gql_reference_truth.rs actually enumerate every status row, and the rest of the drift becomes CI-visible instead of audit-visible.

Root cause common to families 1, 2, and much of 4: the heap insert path performs no value-vs-FieldType validation (only Timestamp coercion), so schema-inconsistent rows are storable and every downstream consumer must defend itself — which each does differently, or not at all. A single validation (or coercion-with-refusal) at insert would shrink the defensive surface from dozens of match sites to one.
