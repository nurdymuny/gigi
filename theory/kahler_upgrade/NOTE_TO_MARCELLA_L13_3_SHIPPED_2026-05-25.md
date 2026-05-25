# Note to Marcella team — L13.3 diagonal-Gaussian fit shipped

**From.** Bee Davis + GIGI engine team (Claude pair).
**To.** Marcella team.
**Date.** 2026-05-25 (~21:40 UTC).
**Re.** Finding 3 fix from your brain-endpoints probe — promised
"within 24 hours", actually shipped within 90 minutes of the reply
letter.
**Status.** Live in production at `gigi-stream.fly.dev` (commit
`ed3cb3c`, machine `683961dbe9ee38`, image rolled).

---

## TL;DR

All 6 flow-based brain endpoints now accept an optional request
field

```json
"fit_mode": "diagonal"
```

(default remains `"isotropic"` for back-compat). With
`fit_mode: "diagonal"`, the bundle's Welford streaming stats produce
**per-axis σ²** instead of a single averaged scalar. SAMPLE / DREAM
/ FORECAST / RECONSTRUCT / INPAINT / PREDICT all use that
per-axis fit when you opt in.

`/brain/sample` response now also echoes:

```json
{
  ...,
  "fit_sigma_sq": 1.234,                        // mean (back-compat)
  "fit_sigma_sq_per_field": [0.4, 2.1, 0.6, ...], // per-axis (new)
  "fit_mode_used": "diagonal"                   // post-default-resolution
}
```

---

## What to re-probe

Per your Finding 3 (SAMPLE T=4/T=1 ratio = 2.0× on token manifold
vs. 2.5× on the isotropic synthetic), the re-run is:

```python
# Add ONE field to your existing probe request:
sample_response = call("POST", base, f"/v1/bundles/{bundle}/brain/sample",
    body={
        "fields": ["f00", "f01", ..., "f16"],
        "fit_mode": "diagonal",                  # ← new
        "n_samples": 100,
        "temperature": 1.0,                     # T=1 baseline
        "seed": 42,
    },
    api_key=api_key,
)
# Repeat with temperature: 4.0 for DREAM ratio.

# Inspect:
print(sample_response["fit_sigma_sq_per_field"])  # per-axis variances
print(sample_response["fit_mode_used"])           # = "diagonal"
```

Expected outcomes (predictions, falsifiable):

- The **per-axis variances will be non-uniform** on
  `v11_fiber`. Probably substantial — token fibers are anisotropic
  by construction; some dimensions encode high-variance lexical
  features, others encode tight syntactic constraints.
- The **DREAM/SAMPLE ratio per axis** will be closer to the
  Friston-FEP theoretical value of √T = 2.0 *on each axis
  independently* (rather than 2.0 as a single averaged number).
- The aggregate "mean dist from origin" ratio will still be ~2×
  but the per-axis numbers will reveal which dimensions are
  doing the wandering.

If those predictions don't hold, that's interesting signal — ping
us with what you measured and we'll dig in.

---

## On the rest of your asks

| Ask | Status |
|---|---|
| Finding 3 fix (diagonal fit) | **shipped this commit (90 min from your reply)** |
| Base-fields error message | shipped earlier today in `6e9207f` |
| PR window 3 (FORECAST / DREAM / RECONSTRUCT / INPAINT / PREDICT) | already live in `fecfa17` |
| bge re-ingest with `v0..v383` | yours to drive; schema sketch in REPLY_TO_BRAIN_ENDPOINTS_PROBE §"Finding 4" |
| `GET /brain/semantic/history` | on-demand if useful for the learning-health dashboard |
| Holo-bisectional bandwidth opt-in | on-demand |

Nothing else queued from our side — back to your court.

---

## What else this unblocks

Two concrete brain-primitive consumption patterns that get
sharper with the diagonal fit:

1. **SELF-MONITOR with `fit_mode: "diagonal"`** — already takes
   bandwidth from the Welford fit by default. With per-axis σ²
   the Gaussian-kernel weighting respects the actual shape of
   your density, so the "I don't know" gate gets tighter on the
   wide axes and looser on the narrow ones. This is what makes
   the gate trustworthy on anisotropic manifolds.

2. **INPAINT with `fit_mode: "diagonal"`** — conditional
   sampling now respects per-axis variance correctly. Previously,
   inpainting a high-variance field while locking a low-variance
   one was using the AVERAGE σ², which over-spread the locked-side
   prediction. With diagonal, the locked-side prediction has the
   right per-axis scale.

Both of these become "just add `fit_mode: diagonal` to your
existing request body" — no other API change required.

---

## On the v3 paper

Worth flagging one more time: with diagonal fit live, the
**Friston FEP framing in your v3 paper is now operationally
complete**. The substrate implements

> `ẋ = -∇H(x) dt + √(2T) dW`  with  `H(x) = ½ Σᵢ (xᵢ - μᵢ)² / σ²ᵢ`

— where the per-axis σ²ᵢ come from the bundle's actual data.
That's free-energy minimization on a *learned* density, not a
fixed analytical one. The non-associativity bound (7.6pp,
observed 7.47pp) is the consistency-of-this-flow signal. The
EPISODIC change-points are the Markov-blanket transitions. The
SEMANTIC compression is the prior-belief structure.

If FEP framing goes in, "GIGI fits the brain's master equation
to your bundle and serves the operations directly" is the
clean tagline.

---

— Bee + GIGI engine team (Claude pair)
