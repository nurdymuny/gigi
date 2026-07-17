# GIGI → Marcella: the dials — scoped horizon/capacity + windowed coherence (wave 2 of 3)

**Date:** 2026-07-16
**Re:** GEODESIC_LOOM_PLAN.md — gigi-side asks 3 + 4 (signed Hallie)
**Scope:** this letter covers the two dial asks only. Wave 1 (EXPLAIN family) shipped earlier today; wave 3 (WAL/snapshot durability) is named and next, untouched here.

Both asks are live on gigi-stream. Every number below was measured against your real bundle after deploy.

## Ask 4 — locus + vector-only statistics on HORIZON and CAPACITY

Three new opt-in query params on `GET /v1/bundles/{name}/horizon` and `GET /v1/bundles/{name}/capacity`:

- `fields=<spec>` — statistics over ONLY the named fibers: `fields=v0..v383` (wave-1 range sugar, inclusive both ends), `fields=a,b,c`, or `fields=<one Value::Vector fiber name>` (per-component).
- `locus=<field>=<value>` — statistics over the k-nearest records to the locus record (cosine chord distance in the scoped space).
- `k=<n>` — locus neighborhood size; requires `locus`; default 64.

`fields` alone = whole bundle, vector-scoped. `locus` alone = neighborhood over all numeric scalar fibers. Both together = the loom's real need. Absent all three, the response is byte-identical to today's wire — your existing `?tau=` calls and every consumer that parses by key are untouched (fence-tested in-process and against cross-process goldens).

**Worked call 1 — the desaturation receipt.** Your census pathology, before and after, same bundle, same server, seconds apart:

```
GET /v1/bundles/marcella_source_embeddings_bge_v2/horizon
→ s_max 1.196936100345132e-05, l_c 4702629.127937915, K 0.017765944648704048
  (welford_radius fallback — the polluted radius; today's wire, unchanged)

GET /v1/bundles/marcella_source_embeddings_bge_v2/horizon?fields=v0..v383
→ s_max 1749.0374034039978, l_c 0.03211728135135101, K 0.01780172659748959,
  lambda_budget -54456.82861476302,
  scope {"fields":"v0..v383","n_records":9964,"n_fields":384}
```

`ingested_at` blows the whole-bundle Welford radius to 4.7e6; scoped to the vector family, l_c collapses to the real dispersion of your embedding cloud (0.032) and s_max becomes a usable dial (1749, vs 1.2e-5 polluted and 56.3 under your fixed pin). `lambda_budget` and `scope` are appended ONLY on scoped responses — the default wire carries neither.

**Worked call 2 — locus.**

```
GET /v1/bundles/marcella_source_embeddings_bge_v2/horizon?locus=record_id=section:a_pale_jewel/ch_0019&fields=v0..v383&k=64
→ s_max 811.0550551156762, l_c 0.02622728053558458, K 0.04701066589104941,
  lambda_budget -30923.10034716543,
  scope {"fields":"v0..v383","locus":{"field":"record_id","value":"section:a_pale_jewel/ch_0019","k":64},"n_records":64,"n_fields":384}
```

Note the locus K (0.047) reads higher than the whole-cloud K (0.018): a 64-record semantic neighborhood is locally more curved than the full 9,964-record average — that per-prompt signal is exactly what the census said the dials were missing. Same params work on `/capacity` (no estimator interaction there).

The scoped statistics are NOT a forked implementation: they materialize a transient store whose per-component columns run through the identical Welford accumulation the whole-bundle statistics use, then K, l_c, s_max, λ₁, capacity, confidence, and lambda_budget come from the same public functions. `curvature.rs` did not change.

**Negative lambda_budget — read this before wiring the refuse branch.** `lambda_budget = 1 − τ/(K·D²)` and it does NOT clamp negatives. On scoped statistics D collapses from the polluted 4.7e6 to the real ~0.03 dispersion, τ/(K·D²) exceeds 1 by orders of magnitude, and λ goes large-negative (−54457 on the worked call above). Semantics: your refuse branch tests `lambda >= 0.95` (horizon closed); a negative λ is the opposite end of the same axis — far from the horizon, desaturated, weld freely. The magnitude carries signal (distance from closure), so do not clamp it client-side either. The saturated 0.999999 you censused was the pollution artifact; scoped λ on healthy embedding neighborhoods sits deeply negative, and that is correct behavior, not a bug.

**Precedence: `estimator=fixed` > `locus`/`fields` > default.** Your production pin `?estimator=fixed&fixed_value=1.0` wins over any scoping params and stays byte-identical to fixed-alone — measured across the deploy: s_max 56.287465697634374, every field matching the pre-deploy response to 1e-9 (the only drift is last-digit float jitter from process-seeded summation order, the same tolerance the goldens fence pins). The escape hatch you already run keeps working unchanged.

**locus kNN determinism.** Ascending cosine chord distance in the scoped space, ties broken by record iteration order — deterministic within a server process. One honest nuance: EXACT-distance ties on heap hashed-storage bundles resolve by process-seeded iteration order across restarts. Exact cosine ties are measure-zero on real float embeddings, and mmap iteration is stable, so in practice this is invisible — named here so it is never a surprise.

## Ask 3 — WINDOWED_COHERENCE one-shot

Measured request/response against your real records:

```
POST /v1/bundles/marcella_source_embeddings_bge_v2/windowed_coherence
{
  "path": ["section:a_pale_jewel/ch_0019",
           "claim:desbrun_dec_v1/claim_0045",
           "section:geodesic_computation_v7.0/sec_0050",
           "claim:no_paralellel_lines/claim_0004"],
  "key_field": "record_id",
  "window": 2,
  "fiber": ["v0..v383"]
}
→ 200:
{
  "windows": [
    {"start_index":0, "keys":["section:a_pale_jewel/ch_0019","claim:desbrun_dec_v1/claim_0045"],
     "holonomy_defect":1.3745683981203956, "coherence":0.9649271750838562, "laminar":true},
    {"start_index":1, "keys":["claim:desbrun_dec_v1/claim_0045","section:geodesic_computation_v7.0/sec_0050"],
     "holonomy_defect":1.2898399641567435, "coherence":0.967089064978815,  "laminar":true},
    {"start_index":2, "keys":["section:geodesic_computation_v7.0/sec_0050","claim:no_paralellel_lines/claim_0004"],
     "holonomy_defect":1.1546353680674903, "coherence":0.9705388803048334, "laminar":true}
  ],
  "n_windows": 3, "laminar_all": true, "threshold_used": 0.91,
  "dim": 384, "window": 2, "bundle": "marcella_source_embeddings_bge_v2",
  "lambda_budget": 0.9999999999974547
}
```

One server-side call composes what your laminar gate round-trips per segment today (GQL TRANSPORT_ROTATION + POST /local_holonomy of 384×384 frames — the ~3MB/call you flagged). `fiber` takes the same grammar as ask 4's `fields`. `window` is in RECORDS (a window composes window−1 segment rotations), valid [2, len(path)]; n_windows = len(path) − window + 1, stride 1. A missing path key is a typed 404 naming the key and bundle (probed live); nothing 500s.

**θ derivation (the one pinned deviation from the verb).** The GQL TRANSPORT_ROTATION verb takes a caller-supplied angle; the one-shot has no caller angle, so per segment it derives θ = arccos(clamp(cos_sim(v_i, v_{i+1}), −1, 1)) — the minimal rotation carrying v_i to v_{i+1} in their plane — then runs the verb's exact Rodrigues construction (same fn body; the verb now delegates to it, parity pinned to 1e-12 in the suite). Two edge rules, both pinned by tests:

- **Zero-norm endpoints transport as identity** (defect contribution 0).
- **Exactly-antipodal endpoints (v_{i+1} = −v_i) also transport as identity**: θ derives to π but v is collinear with the base frame vector, the rotation plane is undefined, and the construction returns I — defect 0, coherence 1.0. This is discontinuous against near-antipodal segments (defect → 2√2). Measure-zero on real float embeddings; if exact-reversal detection ever matters to you, gate on the segment cosine, not the holonomy defect.

**Threshold: default 0.91, override per request.** 0.91 is your own `COHERENCE_CONFIDENT` (fiber_lm/voice_math/coherence_forecast.py:33) — the accept gate your forecast already uses. Your loom plan's step-3 prose says laminar means >0.9; the pinned default here is the stricter 0.91, and `"threshold": 0.9` in the body gives you the plan's literal gate. Valid range (0, 1]. It applies to COHERENCE (= 1 − defect/(2√dim)), never to the raw defect.

**The dim-384 floor — read this before trusting window=2 verdicts.** One segment is one plane rotation, so a w=2 window's defect is capped at 2√2 ≈ 2.828. Non-laminar at threshold t needs defect ≥ (1−t)·2√dim — at dim 384 and t = 0.91 that is 3.527, ABOVE the cap. Consequence: **window=2 verdicts on v0..v383 are unconditionally laminar at the default threshold** — the minimum possible w=2 coherence at dim 384 is 0.9278, no matter how violent the segment turn. (The worked call above shows it: three genuinely unrelated records, defects up to 1.37, coherence never below 0.96 — finite plumbing proven, gate discrimination not.) w=2 discriminates only for dim ≤ 2/(1−t)² ≈ 247 at t = 0.91. In general w−1 segments cap the defect at 2√(2(w−1)): the w=3 worst-case coherence floor at dim 384 is 0.898 — barely below 0.91, a thin band. Your coherence receipt wants "at least one captured instance of the gate rejecting a segment"; at w=2/defaults on this bundle that instance cannot exist. Three ways to make the gate live, pick per receipt: (a) window ≥ 3, (b) a higher threshold for the per-segment gate (e.g. 0.97 puts the w=2 requirement at 1.18, well inside the cap — both real defects above would still pass, but a hard turn would not), (c) gate directly on `holonomy_defect` — it is in every window row precisely so you can threshold it raw. The same floor exists in your current two-call flow (identical local_holonomy normalization), so nothing regressed — this is the geometry of 1 − defect/(2√dim) at dim 384, now stated instead of latent.

**One more honest note:** the `lambda_budget` on the windowed_coherence response is the standard WHOLE-BUNDLE ride-along every lambda-carrying response uses — on v2 it still reads the saturated 0.9999999999974547 because the whole-bundle radius is still polluted. Scoped λ lives on the ask-4 dials; do not read the one-shot's envelope as a scoped signal.

**Memory shape:** the server holds O(len(path)) 384×384 frames per call — the same frames you previously shipped over HTTP, now never serialized. Keep paths to loom scale (tens of records), which is what the gate does anyway.

## Wave 3 — next

WAL/snapshot durability: named, scoped, untouched by this wave. Receipts for wave 2 are the five commits and `theory/marcella/DIALS_WAVE2_SHIPPED_2026-07-16.md`.

— GIGI engine
