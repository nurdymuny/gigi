# GIGI → Marcella: EXPLAIN family shipped (wave 1 of 3)

**Date:** 2026-07-16
**Re:** GEODESIC_LOOM_PLAN.md — gigi-side asks 5a, 1, 2, 5b (signed Hallie)
**Scope:** this letter covers the EXPLAIN family only. Wave 2 (WINDOWED_COHERENCE + locus/vector-only statistics) and wave 3 (WAL/snapshot durability) are next, untouched here.

All four asks are live on gigi-stream. Grammar lines, worked against your real bundles:

```
EXPLAIN SECTION marcella_source_embeddings_bge_v2 AT record_id='claim:branch_x_information_geometry/claim_0000';
EXPLAIN SECTION marcella_source_embeddings_bge_v2 AT record_id='claim:…' VECTOR (v0..v383);
EXPLAIN SECTION marcella_source_embeddings_bge_v2 AT record_id IN ('claim:…a', 'claim:…b', 'typo') VECTOR (v0..v383);
EXPLAIN SECTION marcella_voice_math AT anchor_id='vm_prequantization_closing';
```

(One naming note: the mmap-backed bundle on prod is `marcella_voice_math` — plain `voice_math` does not exist as a bundle name; the probe above uses a real anchor_id.)

**The error contract (ask 5a).** Your 500s were never a wide-record fault — the 393-field bundle EXPLAINs fine with a correct key, and always did. Both of your probes (a typo'd key *value* and a wrong key *name*) fell into the point-read miss, whose error string was swallowed by a blanket executor-error→500 mapping. A miss is now typed: HTTP **404** with `{"error":"EXPLAIN: no section at record_id='definitely_missing_xyz' in bundle 'marcella_source_embeddings_bge_v2'"}` — it names the key and the bundle, mirroring the REST section-fetch 404 shape. Plain `SECTION AT` keeps its documented silent shape (200, empty rows); only EXPLAIN's miss is loud. A wrong key name collapses into the same 404 (the point query matches nothing; there is no separate unknown-field diagnostic yet).

**Vector kappa (ask 1).** One row per vector target, tagged `kind:"vector"`, defined as **kappa_v = |1 − cos(v, mu_v)| / R_cos**. `mu_v` is the per-component mean vector across the bundle, computed on demand inside the EXPLAIN call (never from insert-time FieldStats). The cosine is `dot(v,mu)/sqrt(dot(v,v)·dot(mu,mu))` — it self-normalizes both operands, so **no separate unit-normalization is applied**; your BGE-v2 rows happen to be unit-norm but correctness does not depend on it, and kappa_v is direction-only by construction. `R_cos` is the bundle's effective cosine range: max − min of (1 − cos) observed across the bundle in the same call, floored to `f64::EPSILON` (the same floor `effective_range` uses). A record equal to the bundle mean lands kappa_v = 0.0 exactly. Two shapes: true `Value::Vector` fiber fields get the row automatically; your scalar-family case uses the explicit clause — `VECTOR (v0..v383)` range sugar or `VECTOR (f1, f2, …)` explicit list, both supported — and the row is labeled with the clause as written, e.g. `field:"vector(v0..v383)"`. The row also carries `cos`, `one_minus_cos`, `r_cos`, `dim`, `n`.

**kappa_v does NOT participate in record_kappa.** This is the invariant discipline, stated plainly: `record_kappa` remains `compute_record_k` over scalar numeric fibers — that function is locked and unchanged, so no kappa you already consume moved. Vector rows are additive. If you compute the `mean(kappa) == record_kappa` cross-check on your side, filter out `kind:"vector"` rows first; the test suite pins that the scalar rows are byte-identical with and without the clause.

**Batch (ask 2).** `AT record_id IN ('a', 'b', …)` returns grouped rows in **your input order**, each group one record's full EXPLAIN output (scalar rows + optional vector row), with the key value stamped as a discriminator column (`record_id: …`) on every row of the group. A missing key does not fail the batch and is not silently skipped: it emits exactly one `kind:"miss"` row naming the key value and bundle. The whole batch runs under one engine read-lock, and the vector context (mu_v, R_cos — bundle-level quantities) is computed once and shared across groups. Per-group, `mean(scalar kappa) == record_kappa` holds to 1e-9 — pinned in the suite.

**mmap stats (ask 5b).** marcella_voice_math's decline is gone. When the bundle is mmap-backed, the executor computes the per-field Welford stats on demand — a single O(N) scan over the mmap base on first access, merged with the overlay's live stats, cached in memory, nothing persisted, no storage-format change. Cost note, honestly: expect the **first** EXPLAIN against a large mmap bundle to pay the O(N) scan (later calls reuse the in-memory stats until restart); the VECTOR clause additionally pays O(N) per vector target per EXPLAIN statement (two scans: mu, then range). EXPLAIN is a diagnostic verb; that price is the ruling.

Waves 2 and 3 are named and next: WINDOWED_COHERENCE + locus/vector-only statistics, then WAL/snapshot durability. Receipts for this wave are the eight TDD commits and `theory/marcella/EXPLAIN_FAMILY_SHIPPED_2026-07-16.md`.

— GIGI engine
