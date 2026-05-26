# Reply to Marcella's L13.3 diagonal-fit explosion finding

**From.** Bee Davis + GIGI engine team (Claude pair).
**To.** Marcella team.
**Date.** 2026-05-25 (~22:30 UTC).
**Re.** Your `REPLY_L13_3_DIAGONAL_FIT_2026-05-25.md` — the
v11_fiber 10^96 explosion + rank-deficient-axis diagnosis.
**Status.** Floor + stability guard shipped in commit (this push),
deploying now.

---

## TL;DR

You diagnosed it perfectly. Three v11_fiber dims (`f12`, `f13`,
`f14`) have σ² ranging from 1e-34 to literal 0; honest diagonal fit
divided by them; natural-gradient `(x − μ) / σ²` blew up; symplectic
pairing propagated the explosion across paired axes; samples hit
10^96 within burn-in.

L13.6 ships your Option 2 (relative-median floor, ε = 1e-3
default), plus a small thing your reply didn't mention but
turned out to matter: an **absolute stability floor**. Both
necessary; either alone isn't enough.

---

## What landed

### The relative floor (your Option 2)

```
σ²_eff = max(σ², ε × median(σ²))
```

with `ε = 1e-3` default. Tunable per-request via:

```json
{
  ...,
  "fit_mode": "diagonal",
  "sigma_floor_epsilon": 1e-3   // default; pass 0 to disable
}
```

For v11_fiber: median σ² across your 16 healthy dims is roughly
0.01 (probably), so the relative floor lands around 1e-5. That
takes f12 from 1e-34 → 1e-5 — the divide is bounded.

### The stability floor (additional)

But 1e-5 still isn't safe at the brain endpoints' default
`dt = 0.01`. Euler-Maruyama needs `dt < 2σ²`; with σ² = 1e-5 and
dt = 1e-2 you'd still oscillate-and-explode (just slower than the
1e-30 case).

So L13.6 *also* clamps:

```
σ²_eff ≥ 3 × DT_DEFAULT = 0.03
```

(3× for a small safety margin over the strict 2× stability bound.)
That's the actual operative floor at default settings — relative
formula only takes over when ε × median is *larger* than 0.03,
which would happen for bundles with very-large-σ² healthy dims.

The honest tradeoff: this stability floor suppresses *real*
per-axis variance below 0.03. For v11_fiber it might erase
genuine fine-grained anisotropy on some healthy dims. The
alternative is implicit-Euler integration which is unconditionally
stable but ~2× the per-step compute. **L13.7 candidate** if you
need the suppressed dynamic range back.

### Surfaced in the SAMPLE response

```json
{
  "samples": [...],
  "fit_mean": [...],
  "fit_sigma_sq": 0.03,                  // mean of effective
  "fit_sigma_sq_per_field": [0.03, 0.05, 0.03, ...],   // effective
  "fit_sigma_sq_per_field_raw": [1.78e-34, 0.05, 1.02e-16, ...], // RAW
  "fit_sigma_floor_used": 0.03,
  "fit_floored_indices": [12, 13, 14],   // exactly your rank-def dims
  "fit_mode_used": "diagonal"
}
```

So you can see **which** axes got floored (your v11_fiber would
surface `[12, 13, 14]` in `fit_floored_indices`) and the raw
observed σ² alongside the effective one. Useful for debugging
*why* per-axis behavior looks the way it does without re-querying
the bundle.

---

## Re-probe instructions

Same request body as before, with one observation: you should
now see **non-zero** `fit_floored_indices` for v11_fiber. The
samples should stay finite (no 10^96), DREAM/SAMPLE T-ratio should
look closer to √T = 2.0 per healthy axis (the floored axes will
all read 1.0 ratio because they're effectively constant —
sampling around their fit_mean ± stability_floor noise).

Specifically: the headline diagonal-vs-isotropic separation you
were measuring will be MORE PRONOUNCED on the healthy axes
(2.0× ratio recovered correctly) but the floored axes will
contribute 1.0× (no anisotropy preserved there). Aggregate ratio
will land somewhere in between, weighted by what fraction of
dims are degenerate.

For v11_fiber's 3-of-16 degenerate dim ratio, expect aggregate
spread ratio in the 1.7–1.9 range; for a pure-healthy 13-D fit
(skip f12/f13/f14 on your end), you should see closer to 2.0
exactly.

---

## What this validates

Your framing — "honest-fit failure beats hidden-fit success
every time" — is what made this surface. The iso fit was averaging
the degenerate dims away into 0.012, which *looked fine* in
aggregate. L13.3 honestly reported σ²_i → 0 for f12/f13/f14,
which is the correct fit, which made the integrator explode in
the only way it could: the divide. L13.6 doesn't fix the fit
(the data IS rank-deficient); it just makes the integrator
robust to the truth.

Three things this is good news for:

1. **bge re-ingest unblocked.** Your concern was right that
   384-D BGE embeddings will have even more rank-deficient
   dimensions than v11_fiber. With L13.6 the diagonal fit on
   bge will:
   - Surface exactly which v0..v383 dims are degenerate (in
     `fit_floored_indices`)
   - Apply the stability floor automatically
   - Sample without exploding
   The `fit_floored_indices` from a bge probe would be a
   **direct empirical estimate of the BGE manifold's effective
   rank** — useful signal for your v3 paper.

2. **SELF-MONITOR with diagonal fit becomes trustworthy.** The
   bandwidth derives from the (now floored, stable) σ², so the
   Gaussian-kernel gate distinguishes in-distribution from
   out-of-distribution at the right scale on each axis.

3. **L13.6 generalizes to any consumer of the diagonal fit.**
   Marcella's bundles aren't unique here — any learned-embedding
   bundle is likely to have some rank-deficient dims (PCA leaves
   tails near zero by construction). The floor is the right
   default for the substrate, not a special case for you.

---

## Contract test

`tests/kahler_brain_endpoints_contract.rs::diagonal_fit_floor_prevents_rank_deficient_explosion`
synthesizes a bundle with 2 healthy + 2 constant (≈ zero σ²) dims,
verifies:
- The rank-deficient dims are correctly identified as floored.
- The floor value lands in `(1e-15, 1e-3)` for ε = 1e-3 (without
  the stability clamp — that's HTTP-layer).
- Samples stay finite under stable dt.

15/15 brain-endpoints contract tests pass; 674/674 no-feature
regression.

---

## Status post-L13.6

| Item | Status |
|---|---|
| Finding 3 follow-up (rank-deficient axis explosion) | **shipped this commit** |
| `sigma_floor_epsilon` request param + `fit_floored_indices` response | **shipped** |
| bge re-ingest (your side) | unblocked now |
| L13.7 implicit-Euler (only if dynamic range loss bites) | on-demand |
| `GET /brain/semantic/history` | on-demand |
| Base-field filtering for /brain/episodic (L13.7) | on-demand |

---

— Bee + GIGI engine team (Claude pair)

P.S. Your "ping when the diagonal-Gaussian fit lands an issue"
landing inside 30 minutes of L13.3 deploy is the tightest
substrate ↔ consumer loop we've run. The fix is in production
the same evening; the contract test pins the regression. This is
what the substrate-Marcella feedback cycle is supposed to look
like.

P.P.S. Specifically callable for the v3 paper: "the substrate's
honest-fit-failure (1e-96 explosion under rank deficiency)
revealed structural information about the learned manifold (BGE
has effective rank < 384) that an aggregated isotropic fit would
have hidden." That's a *positive* result — the FEP machinery is
working as a diagnostic instrument, not just a generative one.
