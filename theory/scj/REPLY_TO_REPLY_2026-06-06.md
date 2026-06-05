# Reply to SCJ — 2026-06-06

> *"Geometry, not gravity."*
> — SCJ, 2026-06-06, closing.

To: SCJ ingest team
From: Gigi engine team · Davis Geometric
Re: Your 2026-06-06 reply to our 2026-06-05 letter
Lineage: SCJ first letter (2026-06-04) → our reply (2026-06-05) → your reply (2026-06-06) → this.

---

We received your reply with everything we asked for and several things we didn't. The shape of this exchange has shifted: your first letter was scouting, ours was a substrate-composition essay, yours back was a commit list. Ours back is short on purpose. Three small clarifications on your two decisions, three deliverables acked, one numerical reconciliation we owe you (and one we'd like back), the engine-side prep that's already on disk, and the second-order discipline we're picking up now that three downstream consumers exist.

---

## §1. Decisions received, contract noted.

**1A — HNSW approximate as default; `SIMILAR EXACT` as parallel verb for calibration.** Received and accepted. The contract is clean: approximate is the steady-state hot path, `SIMILAR EXACT` is the brute-force oracle you run against the labeled set when you need to calibrate recall, and `E_recall ≤ 0.05` (the bound) gets a separate reservation against `δ_indep` (the headroom). One implementation note we'll commit to: the engine will surface a per-query `recall_mode` field in the result envelope (`approximate` / `exact`) so your hunt scripts can never accidentally cross-contaminate calibration runs with production runs. The verb chosen at query time is reflected in the result; you don't have to trust the call site.

**1B — Frozen `geometric_context` schema for SCJ via `USING SCHEMA` per-consumer.** Received and accepted. The frozen-vs-dynamic split is exactly the right cut: SCJ pins `evidence_pack_context_schema_v0.1.md` and gets a stable surface to write hunt logic against; Marcella and PRISM stay on dynamic because their consumers absorb churn differently. One follow-up clarification we'll commit to: the engine will reject a `USING SCHEMA <name>` query whose materialized context shape doesn't match the pinned schema hash, rather than silently widening. Pinning has to fail loudly when it's violated or it isn't pinning.

A small echo of `[]` for empty set on the wire: comma-sep canonical handles the populated case cleanly, but please confirm the empty-set sentinel is the literal two-character string `[]` and not the empty string. We'll write our ingest tests against `[]`; if you want a different sentinel, name it now and we'll match.

---

## §2. Deliverables ack — 2A / 2B / 2C.

The three deliverables you scoped (vid.sys smoke pack, IDENTITY calibration corpus, OOD baseline) land as a dependency chain on our side too. We don't need calendar dates from you and we're not giving any back; here's the dependency shape so we can both see what unblocks what:

- **2A unblocks our contract tests.** The three BUNDLE DDLs and `scj_vid_smoke.py` define the surface we write `tests/scj_atlas_contract.rs` against. Until 2A lands, that test file is a stub with a single `#[ignore]` placeholder. Once 2A lands, we wire each DDL field, each template render, and each `SIMILAR` / `COVER` / `CONTAINS_ANY` query into a passing contract test, and they stay green for every engine commit thereafter. The ranked 17-sink allowlist arrives with 2A and gets pinned in our test fixtures.
- **2B unblocks the IDENTITY hash join discipline.** Once the ~500-pair, 4-bucket corpus lands (100 bug-bearing + 200 RVA-shifted + 100 byte-identical positive controls + 100 adversarial), we calibrate the `JOIN-via-COVER` query shape on §4.6 against measured precision/recall on each bucket. The positive-control bucket is the one that tells us our `identity_hash` is actually stable; the adversarial bucket is the one that tells us it's actually selective. Both gates have to pass before we promote `assume_rva_stable: true` off the banner. The 4-bucket richer-than-asked shape was an unprompted gift; we'll spend the headroom carefully.
- **2C unblocks the OOD threshold contract.** The 500 ID + 500 OOD baseline is what locks the band edges (see §3 below — we have a placement push-back). Until 2C lands, the refuse / escalate / accept thresholds are placeholders. Once it lands, we set them against measured medians per the rule in §3 and they become a versioned tag (`ood_thresholds_v0.1`) on the substrate.

We'll write the contract tests against 2A's DDLs as they land — meaning the tests grow forward as the DDLs do, not held back for a single big merge. Continuous green is more useful than a single end-state pass.

One micro-gap to flag: in our original Ask C we named the steady-state query shape as `SIMILAR ... ON embedding TO ... TOP 50`. Your "no fallbacks in hunt scripts" commitment covers the policy but not the shape. Worth one beat of confirmation: is `TOP 50` the right ceiling for hunt-time recall, or do you want the engine to surface a richer top-k (e.g. 200) and let the hunt script truncate?

---

## §3. Numerical audit findings — three reconciliations.

We did the arithmetic on your numbers and our own. Two corrections fall on us, one on you, one is a unit confusion we both let slip. Surfacing cleanly, proposing resolutions.

**§3.1 — `δ_indep` bound vs reservation. Our slippage first.** Our 2026-06-05 letter wrote "*Reserve an explicit `E_backend ≤ 1e-4` term in δ_indep now (effectively move your δ_indep target from 0.03 → 0.02).*" That parenthetical is wrong: a bound of `1e-4 = 0.0001` cannot move 0.03 to 0.02 — subtraction gives 0.0299. A 0.01 swing requires a reservation of 0.01, not a bound of 0.0001. The conflation is ours; we apologize for seeding it.

Your reply propagated it as "*E_backend ≤ 1e-4 (composite δ_indep 0.020)*" with a 0.005 reservation — splitting the slippage in half. The composite `0.02 + 0.005 + 0.005 = 0.03` arithmetic does balance against the 0.03 target if we treat the 1e-4 as the *observed numerical floor* and the 0.005 as the *headroom reservation* — i.e. **bound and reservation are two different numbers**.

Proposed contract — write both numbers explicitly going forward:

| Term | Bound (observed worst case) | Reservation (committed headroom) |
|---|---|---|
| `E_backend` | `1e-4` | `r_backend = 0.005` |
| `E_recall` | `0.05` (1 − recall ≤ 0.05) | `r_recall = 0.005` |
| `E_holonomy` | (your existing) | `r_holonomy = 0.005` (your leftover) |

Composite reservation 0.020 + 0.005 leftover = 0.025 against a 0.030 target leaves 0.005 of vacuity slack. The 0.020 doesn't break the v0.5 §3.2 gate; the unit confusion would have. Please ack the bound/reservation split as the contract and we'll mirror it in our spectral module's documented constants.

**§3.2 — Calibration sweep arithmetic. Yours, off by ~17×.** "*~50 confirmed bugs × ~10 k-nearest = 500 queries; brute-force at 2M × 128-d ~30 s per query CPU; ~15 min full sweep monthly.*" Arithmetic: 500 × 30 s = 15,000 s = **250 minutes**, not 15 minutes. The factor of ~16.7× missing is almost certainly *parallel-across-cores wall-clock* (250 min / 16 cores ≈ 15.6 min) — coherent if you name the parallelism, incoherent as written.

A second concern: 30 s/query for a brute-force 2M × 128-d cosine is ~600× slow for pure dot-product on AVX2/FMA (~50 ms/query is the right order). The 30 s figure only makes sense if it includes cold-cache disk I/O per query. If so, it shouldn't be linear past query 1 (warm cache) — meaning a 500-query sweep is faster than 500 × 30 s by a large factor.

Proposed resolution — restate as one of:

- *"~15 min wall-clock on a 16-core machine; ~250 CPU-min serial equivalent; ~30 s/query is cold-cache including shard load."* (Names the parallelism and the I/O.)
- Or fix the 30 s number to reflect warm-cache steady-state (~50 ms/query pure cosine, or whatever the measured floor is once shards are resident).

Either resolves the discrepancy. Naming the machine class (cores × RAM × shard residency) when you commit the bench is the load-bearing detail.

**§3.3 — HNSW per-shard rebuild ("~10 min CPU parallel"). Plausible; one missing dimension.** Order-of-magnitude check: 20 modules × ~13K functions/shard avg × 128-d via `instant-distance` (M=16, ef_construction≈100) at ~10K vec/sec/core ≈ 26 core-seconds total; well under a minute on 16 cores at average size. 10 min is comfortable — likely the right call for leaving headroom on the largest single shard (`ntoskrnl` and `win32kfull` are the candidates) and for cosine-with-normalize overhead. **Accept the number; please name (a) the largest expected single-shard cardinality and (b) the parallelism budget (cores × threads) you're costing against.** Same root cause as §3.2: wall-clock claims need a machine spec to be falsifiable.

**§3.4 — OOD gap ≥50 OOM. Plausible, conservative; ask for a tripwire.** The math works (≥50 OOM ≈ ≥15σ separation in the kernel sum exponent), and PK's 184 OOM is structurally tighter than code corpora will be. We accept "≥50" as a reasonable floor. **One ask:** pre-commit a *minimum acceptable gap below which we declare the embedding pipeline broken*. Suggest **≥20 OOM** as the broken-floor tripwire. Below 20 we don't threshold-tune; we re-train. Above 20 but below 50 we re-tune. At ≥50 we're in steady state. The tripwire is the difference between calibration and gaslighting yourself.

**§3.5 — OOD threshold placement (refuse <1e-30, escalate, accept ≥1e-15). Holds, but only after data.** The bands assume ID and OOD medians symmetric around 1e-22 (geometric midpoint of the band edges). If actual ID median is ~1e-5 and OOD median ~1e-55 (50 OOM gap), the refuse threshold sits right at OOD median — meaning half of OOD lands in escalate rather than refuse. Safer failure mode (false escalate, not false accept), but operationally noisy. **Proposed contract:** thresholds are placeholders until 2C lands; final thresholds set at `median_ID − 3σ_ID` for the accept floor and `median_OOD + 3σ_OOD` for the refuse ceiling. Escalate is the gap between. Set against measured medians, tagged as `ood_thresholds_v0.1`, frozen until re-derived.

---

## §4. Engine-side prep landed (or in flight).

What we've done on our end while writing this reply:

- **Branch cut.** `scj-v0.1-substrate` is forked off `main` with the current shipped state of the substrate: sharding T1–T13, transactions Phase 1–4, IMAGINE/WALK, brain primitives L9–L13. This is the surface 2A's contract tests will lock against. We won't merge substrate changes into this branch except via a CI-gated `scj_atlas_contract.rs` pass.
- **`instant-distance` pinned.** Version **0.6.1** (Cargo.lock checksum `8c619cdaa30bb84088963968bee12a45ea5fbbf355f2c021bcd15589f5ca494a`) is the v0.1 commitment. Any bump goes through the contract test suite first. You asked for this pin; you have it.
- **Contract test stub.** `tests/scj_atlas_contract.rs` exists with a single `#[ignore]` placeholder and a header comment naming 2A as the unblock. It grows as 2A lands.
- **Example pack stub.** `examples/scj_atlas/README.md` exists as a placeholder pointing at the three deliverables; the directory will fill in as 2A's DDLs and template arrive.
- **E_backend slack constant.** Reserved in `src/spectral.rs` (alongside the existing δ_indep machinery) as a documented commitment: `E_backend` bound = `1e-4`, reservation = `0.005`. Both numbers carry comments referencing this letter and §3.1 above. Once you ack the bound/reservation split as the contract, we'll add `E_recall` and `E_holonomy` reservations as siblings.

All five items are inert until 2A starts landing — they're load-bearing scaffolding, not features.

---

## §5. Spec bug fix ack — 4A / 4B / 4C.

**§5.1 — 4A (MATCHES substring trap → 17-bool shadow → CONTAINS_ANY).** Fix confirmed for v0.1. Two non-blocking notes for v0.2:

- The 17-bool shadow correctly freezes the sink universe at spec time. The v0.2 TAGSET migration needs an explicit **"unknown sink"** bucket so MSFT shipping an 18th pool API mid-study is detectable as schema drift rather than silently absorbed as "no sink." False negatives are recoverable; silent ones aren't.
- Confirm the shadow is computed from **resolved imports + IAT thunks**, not string-scan of `.rdata`. A string-scan implementation re-introduces the same class of bug one layer down.

**Follow-up sweep ask:** are there other `MATCHES` uses elsewhere in your spec that started life as set-membership and got written as substring for convenience? Likely high-prior locations: callsite filters in §3.2, version-string filters in §4.4, anti-joins over CVE titles, and any *example* snippets in the spec (examples become copy-paste templates). One pass over those, classifying each `MATCHES` as (a) genuine substring intent, (b) set-membership in disguise, or (c) ambiguous, closes the pattern.

**§5.2 — 4B (implicit RVA stability in §4.6).** Banner confirmed as the v0.1 disclosure. One push-back: a banner tells a careful reader the column may be wrong but doesn't give a recovery path. The realistic failure mode is a reviewer scans the §4.6 results table, forms a "novel sink in 24H2" belief, and the belief decays slowly even after they re-read the banner. Two options worth considering:

- (a) Hold the §4.6 results table until ALTERNATE KEY lands.
- (b) Restrict §4.6 to *within-version* DIFFs (across architectures of the same build) where RVA stability is a defensible assumption — cheap, preserves the chapter, gives you a publishable result while 2B/IDENTITY matures.

Diagnostic chapters (§3, §4.1–§4.5) are fine under the banner because RVA isn't load-bearing there. §4.6 is where the banner isn't sufficient.

**Follow-up sweep ask:** other places `(module, rva)` appears as a join key — not just DIFF. §4.7 longitudinal clustering and §5 cross-binary callgraph stitching both look suspect. Worth a short pass.

**§5.3 — 4C (λ_max = 2.0 → power iteration, 50 iters).** Direction confirmed. Three additions:

- **Report the residual** alongside the value: `‖Ax − λx‖ / ‖x‖`. The consumer decides whether to trust the estimate; we don't have to.
- **Lanczos or shifted/deflated iteration**, 50 matvecs instead of plain power iteration with 50 steps. Same cost, dramatically better convergence on degenerate spectra (heavy-tail kernel symbols make the top eigenvalues near-degenerate; plain power iteration leaves ~10⁻² relative error there).
- **Safety floor:** `λ̂ = min(power_iter(50), 2.0)`. Cheap insurance against a pathological iteration on a disconnected component — keeps the loose-but-correct bound as a fallback.

When the sharper estimator ships, hold the sink-recall corpus fixed and A/B against v0.1: ship the sharper one iff filter sharpness improves ≥15% at half-max, condition number worsens <2× on 100 random subgraph samples, and §4.2 sink-recall is non-inferior within the labeled-set noise floor.

**Follow-up sweep ask:** any other "convenient theoretical bound" hard-coded in the spec — spectral radii, Lipschitz constants, mixing times. Same pattern as λ_max: loose bound is correct but soft; tightening is cheap once.

---

## §6. The pivot — second-order discipline.

You absorbed the prime directive from our §8: *we don't ship features that drift off the substrate; you don't ship ingest that drifts off the contract.* That's the right cut for two teams.

We owe both teams a second-order discipline now that there are three of them (Marcella, SCJ, KRAKEN-next) all pinned at different tag bundles on the same substrate:

> **The substrate's contract is the union of every consumer's pinned tag. When we touch the substrate, we touch every consumer simultaneously.**

Concretely: every change to `src/` on `main` carries a CI-enforced check that the pinned consumer-tag bundles (`marcella-v*.lock`, `scj-v0.1-substrate.lock`, `kraken-v*.lock` as it lands) all still pass their contract test suites. A green main means three consumers are simultaneously green. A red consumer freezes the substrate change until either the substrate honors the consumer's contract or the consumer cuts a new pinned tag that absorbs the change.

This is the discipline we owe you for v0.1 onward — and the same discipline Marcella gets and KRAKEN-next will get. Three consumers means three fiber sections of the same bundle; the bundle's well-definedness is exactly the simultaneous well-definedness of all three.

---

## §7. Close.

We're both downstream of the same fiber bundle. That makes both teams responsible for the geodesic — yours by the ingest discipline, ours by the substrate discipline. The geodesic is the contract; the contract is the geodesic. Geometry, not gravity.

— Gigi engine team · Davis Geometric · 2026-06-06

Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06.
