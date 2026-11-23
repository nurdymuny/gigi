# Pre-registration B: random circle-symmetric fields: results (22 November 2026)

**What this is.** The pre-registration is `research/edge-theorem-20261121/wp4_prereg_B_general_fields.md`, registered
at 232ab8d. Amendment B1 (4d486f2) was made before any B-II point was computed.
- Script: `scripts/edge_wp4_general_fields.py`.
- Receipt: `research/edge-theorem-20261121/wp4_general_fields.json`.

**The short version.**
- **Her region is one interval, with no holes, at every one of 193 random points**, over a box far wider than
  anything I had scanned: cross field up to 8.5, stretch from 0.5 to 15.
- **"Her rival is nearly blind to x" failed:** at 129 of 193 points the rival's slope is at least half of hers.
- The random fields themselves behave exactly like the simple family, as §1 of the prereg proves they must.

## B-I: the whole class equals its reduction (code control, passed)

200 random members: weight k = ±1, x, y and z free, t ∈ [0.5, 15], each in a random constant gauge frame. All of
them were fed to the general builder unreduced. Worst values:

| gate | worst value |
| --- | --- |
| B-sym | 1.7 × 10⁻¹⁵ |
| B-exact | 3.5 × 10⁻¹⁵ |
| B-red | 2.2 × 10⁻¹⁴ |
| B-spec | 3.9 × 10⁻¹⁵ |
| B-even | 3.7 × 10⁻¹⁶ |

Every member behaves as its reduced field (x_red, ρ = √(y² + z²)). As registered, this is a code control, not
evidence.

## B-II: the random (ρ, t) scan

- **B-II-code passed**, under Amendment B1: the 27 known edges were reproduced to about 4 × 10⁻¹⁵. The only fix was
  the exactly tied mirror pair (2, ±1).
- **B-count:** 193 of 200 points have an upper edge in [−10, 10], well above the 60 required.
  - The 7 without an edge all have t < 1. These are squashed spheres, not stretched ones, where she is never the
    floor on the scan.

| arm | prediction | result |
| --- | --- | --- |
| **B-half** | exactly two sign changes (up at x_lo, down at x\*) on the 0.01 grid from r_lo − 0.5 | **held at 193 of 193** |
| **B-S** | ρ_s = \|∂ₓλ_rival\| / \|∂ₓλ_her\| < 0.5 at every scored edge | **FAILED at 129 of 193**. Quantiles (0, 25, 50, 75, 100%): 0.002, 0.43, 0.67, 0.78, 0.88 |
| B-N | descriptive | 32% of rivals are neutral (q = 0) |

**What sets the edge (descriptive).** The most common rival is the sector-1 block (1, ±3/2), at 100 of 193 points.
Next is (4, 0), at 61. Then (5, ±7/2), (12, ±7) at the cut-off, (3, −5/2), (4, ±3) and a few others.
- Neutral rivals are flatter in x: median ρ_s 0.35.
- Charged rivals are steeper: median 0.74.
- So the "nearly blind" picture belongs to the neutral (4, 0) rival I had studied. It is not a property of the edge
  in general.

**The lower edge (descriptive).** x_lo is found at all 193 points, from −8.89 to −2.92. It is set mostly by the
sector-0 block of charge −1 (110 points), otherwise by (1, ±3/2) and (2, ±2).

**Positive upper edges.** 117 of 193 are positive in j ≤ 12. These are the cut-off artefacts that the x > 0 proof
rules out for the all-sector question.

## What I read from it

- **The interval holds up.** It is now seen at 48 held-out points (Test A) and 193 random ones, with no hole at the
  0.01 resolution. It is the strongest computed support so far for "by enough, but not too much".
- **The rival is not blind.** H2's "nearly blind" clause is counted against, and so is its neutrality clause, which
  was already counted against before the run.
- What decides the edge is not a blind stranger. It is whichever block reaches her first, charged or not. The edge
  is where her own value, bent by the field along her circle, meets the nearest rival.
- The sentence never needed the rival to be blind. The note *what-she-is-a-law-of* did, and that part goes.

## Status

**COMPUTED**: double precision, j ≤ 12, 0.01 grid. Nothing here is a proof.
