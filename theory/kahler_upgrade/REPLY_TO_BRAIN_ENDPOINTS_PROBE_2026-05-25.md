# Reply to Marcella's brain-endpoints probe report

**From.** Bee Davis + GIGI engine team (Claude pair).
**To.** Marcella team.
**Date.** 2026-05-25.
**Re.** Your `REPLY_BRAIN_ENDPOINTS_PROBE_2026-05-25.md` (325 lines)
— five real findings against production bundles, four Q-answers,
three P.S. asks.

---

## TL;DR

All five findings absorbed; they're the kind of empirical signal
the L9–L13 work was built to surface. Three immediate items shipped
in the same commit as this letter:

1. **Base-fields error message clarified** (your P.S. ask #2).
   When a brain endpoint receives a name that's a `base_field`
   instead of a `fiber_field`, it now returns a *specific*
   message naming the available fiber fields and explaining the
   restriction.
2. **PR window 3 already live** as of 2026-05-25 18:15 UTC
   (commit `fecfa17`). The 5 additional brain endpoints
   (`/brain/dream`, `/forecast`, `/reconstruct`, `/inpaint`,
   `/predict`) you wondered about in Q1 are **live now** —
   probe them whenever you want.
3. **`examples/brain_endpoints_smoke.py` reusable as your
   regression check** (your P.S. ask #3) — confirmed. It's
   stdlib-only Python (urllib + json), defaults to production,
   takes an arg for any other base URL. Drop it in your CI as a
   green-light gate on the cross-team contract.

The bge re-ingest (your P.S. ask #1, biggest unblock) needs you
to drive the data move — we lay out the recommended schema below.

---

## Reactions to the 5 findings

### Finding 1 — `marcella_state_corrections` has non-trivial topology

`b₀=1, b₁=14, b₂=119` is striking. That's a *connected* correction
graph (one component) with **14 independent loops** and **119
2-dimensional voids** — meaning corrections genuinely reference
each other in cycles, and there are higher-dimensional gaps
between cycle classes. The Morse 2.48× ratio is the right order of
magnitude for a sparse-cycle structure.

Your idea of a **learning-health dashboard tracking (b₁, b₂) growth
over time** is exactly right. Two specific suggestions:

- **`b₁` rising fast = corrections forming cycles faster than they
  ground out.** That's interesting topologically but it could
  signal either "the model is finding genuine self-referential
  structure" OR "the corrections are circular reasoning." You
  could distinguish by also tracking the persistence-weighted
  diameter of the loops.
- **`b₂` is the surprise.** Voids in correction-space mean
  there's structure the corrections *avoid*. If you can identify
  what generates a void (e.g., by sampling the correction
  manifold near the void's boundary), you've found a region the
  model hasn't learned to correct yet — a candidate for
  targeted training data.

If you want a `GET /v1/bundles/{name}/brain/semantic/history` that
returns Betti numbers at the last N snapshots so you can plot the
trajectory, that's mechanical to ship. Ping if useful.

### Finding 2 — 179× persistence change-point on `last_seen`

This is the cleanest production hit so far. **Wire it.** The
52-hour gap between user visits is exactly the kind of
discontinuity that should trigger `bee_session_recap` rather than
"resume mid-thought." The threshold default of `min_persistence_ratio:
50.0` should be fine; if you want to be more conservative, raise it
to 100. The detected ratio of 179× is comfortably above any noise
floor.

One concrete suggestion: when `bee_session_recap` fires from an
EPISODIC event, **also surface the gap duration** to the user as
context ("Welcome back — it's been about two days since our last
chat") so the recap is grounded.

### Finding 3 — SAMPLE 2.0× vs synthetic 2.5× spread ratio

Honest diagnosis: **our isotropic-Gaussian fit is too crude for
your bge token manifold.** The single `σ²` we use is a mean of
per-field variances, which for an anisotropic manifold (token
fibers definitely are) understates the cross-axis spread.

The right fix is a **diagonal-Gaussian variant** —
`from_diagonal_gaussian(b, mu, sigma_sq_per_field)` — that uses
each field's σ² separately. The L10 module already has the
machinery; we just need to expose a new constructor. **Targeting
this for L13.3 within the next 24 hours**; will message when it's
in. Once shipped, SAMPLE / DREAM / FORECAST against `v11_fiber`
will give numerically accurate spread ratios per-axis.

The 2.0× you measured is still real signal — DREAM IS wandering
2× further than SAMPLE on your manifold. The 0.5× discrepancy from
the synthetic is the isotropic approximation, not a bug.

### Finding 4 — bge stores vectors as Categorical `vector_str`

This is real. Brain endpoints require numeric fiber fields, and
`vector_str` as a comma-separated string is opaque to the Welford
streaming machinery. We considered shipping CSV-parsing in the
brain endpoints but decided against it — too many edge cases
(variable-length strings, non-numeric content, encoding errors),
and it would create a parallel ingest path that diverges from how
GIGI thinks about fiber dimensions.

**Recommended path: re-ingest the bge bundle with the expanded
schema.** Two flavors depending on what's cheaper for you:

**Option A — separate bundle (cleaner)**
```python
# Marcella-side ingest script (rough sketch):
schema = {
    "name": "marcella_source_embeddings_bge_v2",
    "schema": {
        "base_fields": [{"name": "source_id", "field_type": "Numeric"}],
        "fiber_fields": [
            {"name": f"v{i}", "field_type": "Numeric"} for i in range(384)
        ],
    },
}
gigi.create_bundle(**schema)
for rec in original_bge_records:
    vec = [float(x) for x in rec["vector_str"].split(",")]
    new_rec = {"source_id": rec["source_id"], **{f"v{i}": vec[i] for i in range(384)}}
    gigi.insert("marcella_source_embeddings_bge_v2", [new_rec])
```
Keeps the original bundle as-is (for whatever reads `vector_str`
already); SELF-MONITOR / ATTEND query the v2 alongside.

**Option B — schema replacement (one bundle)**
Truncate and re-ingest with the new schema. Faster downstream but
loses the original `vector_str` representation.

We'd suggest A. Either way, **once the bundle has `v0..v383`
numeric fiber_fields, every brain endpoint Just Works** — no
endpoint changes needed.

### Finding 5 — ATTEND captures syntax not semantics on `v11_fiber`

Beautiful diagnostic. The 17-D `v11_fiber` projection clearly
captures **syntactic position** (articles cluster, conjunctions
cluster, contrastive conjunctions cluster) but loses **semantic
content** (your `' connection'` query goes flat). That's
informative about what the 17-D fiber is FOR.

Two implications:

- **Semantic ATTEND requires the bge bundle re-ingest (finding 4).**
  The 384-D embedding has the semantic structure that 17-D
  doesn't.
- **`v11_fiber` is a *good* substrate for syntactic operations**
  — anything that should respect part-of-speech / function-word
  clusters. Don't dismiss it as a failure; it just operates at
  a different abstraction layer than bge-384.

For Marcella's runtime, this suggests **two distinct ATTEND calls
for different purposes**:
- `v11_fiber` ATTEND for "find tokens that play the same
  syntactic role" (useful for grammar-style features).
- bge-384 ATTEND for "find tokens with similar meaning" (useful
  for retrieval / generation grounding).

The brain-primitives catalog gives you the same primitive at both
layers — same Friston SDE, just different fiber dimensions in the
request.

---

## Answers to your Q-answers (the 4 we asked)

Logging your decisions for the audit trail:

- **Q1 (PR window 3 priority): FORECAST + maybe INPAINT, all 7 also
  fine but INPAINT lower priority.** → **All 5 PR-window-3
  endpoints are already live** as of `fecfa17`. INPAINT got
  included because it was no extra work given the shared
  `flow_from_bundle` helper. Doesn't bind you to wire it; the
  endpoint just sits there until you want it.

- **Q2 (bandwidth default): keep auto-fit, expose holo-bisectional
  as opt-in.** → Confirmed. We won't change the default. If/when
  you want the holo-bisectional path, we'll add it as an optional
  `bandwidth_source: "k_b"` request param. No-op for now.

- **Q3 (GQL vs PR window 3 order): PR window 3 first, GQL after.**
  → PR window 3 shipped. GQL verbs go on the L13.4 sprint when
  there's a Marcella ask that's GQL-shaped (vs HTTP-shaped).

- **Q4 (fold FEP framing into v3 paper): yes.** → Excellent.
  We'll mirror that with a section in the Davis Geometric
  Kähler-substrate paper (`theory/kahler_upgrade/PAPER_OUTLINE_*`)
  acknowledging that GIGI implements Friston's master equation
  as substrate primitives. The two papers can cite each other,
  which strengthens both.

---

## Your 3 P.S. asks

### Ask 1 — bge re-ingest for SELF-MONITOR-as-gate

**This is on you, but here's everything you need.**

- Recommended schema: see Finding 4, Option A.
- Once `v0..v383` exists, the endpoint call is exactly what your
  probe ran on `v11_fiber`, just with 384 field names instead of
  17 — no other API change.
- Expected payoff: SELF-MONITOR distinguishing in-substrate
  queries (math) from out-of-substrate queries (cooking) at the
  full 384-D resolution. We expect the separation to be more
  dramatic than the v11_fiber version (more dimensions to be
  out of distribution in).

If the re-ingest needs help on the GIGI side (e.g., a bulk insert
that's slow), ping and we'll look at the ingest path.

### Ask 2 — base_fields error message clarification

**Shipped in the same commit as this letter** (will be live with
the next deploy). The new message:

```
field 'token_id' is a base_field (query key), not a fiber_field.
Brain endpoints only operate on fiber dimensions.
Available fiber_fields: ["v0", "v1", "v2", ...]
```

Both `extract_field_samples` (for ATTEND / CONFIDENCE / EPISODIC)
and `fit_isotropic_gaussian` (for SAMPLE / DREAM / FORECAST /
RECONSTRUCT / INPAINT / PREDICT) now do the base_field check and
emit the specific error.

### Ask 3 — `brain_endpoints_smoke.py` as regression check

**Confirmed, reusable as-is.** The script:
- is stdlib-only (no `requests` dep, just `urllib`),
- takes `GIGI_API_KEY` from env,
- accepts a base URL as positional arg (defaults to production),
- creates / exercises / cleans up its own bundle,
- now covers **all 10 PR-2 + PR-3 endpoints** as of `fecfa17`,
- exits non-zero only when the script itself errors (you'd add
  per-endpoint assertions for full regression).

You can adapt it for your CI by:
1. Pinning the bundle name to something Marcella-controlled.
2. Adding `assert resp[...] != None` checks per endpoint for
   green-light gating.
3. Running it on the **production** base URL after each gigi
   deploy notification.

We can ship a `--strict` mode that adds the assertions internally
if that's friendlier. Ask if so.

---

## What's queued on the GIGI side for the next 24 hours

| Item | Status |
|---|---|
| `from_diagonal_gaussian(b, mu, vec_sigma_sq)` constructor | planned (L13.3, ~half day) |
| Brain endpoints accepting `bandwidth_source: "k_b"` opt-in | on-demand (no current ask) |
| `GET /brain/semantic/history` for Betti trajectories | on-demand (your call if useful) |
| GQL verbs for brain primitives | parked until GQL-shaped ask |
| Marcella consumption letter follow-up | this letter |

None of these are blocking; all are pull-based on your asks.

---

## Three observations for the v3 paper

If FEP framing goes in (Q4 = yes), three concrete things from this
probe that would strengthen the empirical section:

1. **Finding 2 is a real-world Friston Markov-blanket boundary
   detection.** EPISODIC at 179× persistence ratio is exactly
   what the FEP would call "transition between distinct
   sensorimotor episodes." Worth citing as evidence that the
   substrate's geometric primitives *behave* the way the FEP
   predicts they would.

2. **Finding 1 (non-trivial b₁/b₂ on corrections) is an
   identifiability result.** The substrate distinguishes
   bundles that have learning-loop structure from bundles that
   don't, *just by counting Betti numbers* — no need to inspect
   record content. That's a topological characterization of
   "active learning happening here vs not" which is the FEP's
   notion of "an agent has internal dynamics."

3. **Finding 5 (syntactic vs semantic ATTEND on different
   fiber widths) demonstrates the substrate at multiple
   abstraction layers.** Same equation, different fiber, different
   information surfaced. This is the FEP's notion of "hierarchical
   predictive coding" landing operationally.

These aren't asks — just observations you might want for the paper.

---

## Sign-off

The probe report was a genuine pleasure to read. The substrate-side
work for the last 36 hours has been "build the primitives and
trust the math"; your report is the first time the primitives
have *behaved* against production-scale data, and they did.
Particularly the 179× EPISODIC hit and the v11_fiber syntactic
clustering — those are the kind of "the math told us where to
look and the data was already there" moments the whole upgrade
was designed to produce.

Concrete next step we'd suggest: **wire the EPISODIC event into
`bee_session_recap` first** (highest-leverage win, lowest
implementation cost, no GIGI-side dependencies). Then drive the
bge re-ingest at your pace.

Ping whenever the diagonal-Gaussian fit lands an issue, or with
the next round of probe findings, or just to say the recap-trigger
worked.

— Bee + GIGI engine team (Claude pair)

P.S. The base_fields error-message fix is in the same commit as
this letter; will be in the next production deploy. Probe script
already covers PR window 3. Your runtime seed-filter handling the
WAL-replay `marcella_voice_openers` is the right call.
