# Pre-registration A: the edge at 48 held-out points: results (22 November 2026)

**What this is.** The pre-registration is `research/edge-theorem-20261121/wp4_prereg_A_heldout.md`, registered at
232ab8d. Amendment 1 was made at 9098bd9, before any held-out point was computed.
- Script: `scripts/edge_wp4_heldout.py`.
- Receipt: `research/edge-theorem-20261121/wp4_heldout.json`.
- None of the 48 points had been computed before the run.

**The short version.**
- My edge equation predicted the edge to 10⁻¹² wherever its competitor was the one I named. That was at 34 of the 38
  points where I named it.
- At the other 4 points a different block set the edge. So the claim as I registered it, "for y ≳ 1 the edge is the
  root of P₂, set by (4, 0)", **failed**.
- The three structural predictions held everywhere:
  - no sector from 13 to 79 undercuts her inside her region;
  - the region is one interval;
  - the small-y competitors are in sectors 2 and 3.

## One thing went wrong before the run, and I fixed it in the open

The first run stopped at its own known-answer gate, A-code-2. P₂ was recovered correctly (113 terms; the Berger
check vanished to 10⁻¹⁸), and the known edge is a root of P₂ at all 18 known rows. But it is **not the largest** real
root: P₂ has more real roots above the edge, where a *higher* eigenvalue of the (4, 0) block meets hers.
- My registered rule, "take the largest real root", was wrong.
- I wrote Amendment 1 before any held-out point was computed: take the largest real root where the **lowest**
  eigenvalue of B₄,₀ meets her.
- With that rule the 18 known rows agree to 2.4 × 10⁻¹³.
- The statement "the edge is the largest real root of P₂" is **counted against**, and the paper says so.

## Gates (all passed)

| gate | result |
| --- | --- |
| A-code-1 (closed-form blocks = full spectra) | 4.1 × 10⁻¹⁵ |
| A-code-2 (P₂ recovered, known rows) | 113 terms; 2.4 × 10⁻¹³ (with Amendment 1) |
| A-code-3 (Berger path, t_A) | 1.0 × 10⁻¹⁸ relative |
| A-mult (her value has exactly two copies) | held at all 48 points, all j ≤ 12 |
| A-bound and A-bound-hi | held; x_lo − r_lo ≥ 1.61 everywhere |
| A-branch (P₁ never mimics P₂) | 100% of P2-class points |

## The arms

| arm | prediction | result |
| --- | --- | --- |
| **A1** | at 38 points (30 random + 8 grid), edge = the P₂ root and competitor (4, 0) | **FAILED at 4 of 38**; held at 34, where it agrees to 1.35 × 10⁻¹² |
| **A2** | at the 10 small-y points, the competitor is in sector 2 or 3 | **held**, 10 of 10: (2, ±1) eight times, (3, −½) twice |
| **A3** | at three x < 0 points per held-out point, no non-twist block of any sector 13 ≤ j ≤ 79 lies below her | **held** at all 48 points; smallest margin +0.52 |
| **A4** | her floor set (j ≤ 12) is a single interval on the 0.01 grid | **held** at all 48 points |

**The four A1 failures, each with what set the edge:**

| t | y | P₂ predicted | measured | competitor |
| --- | --- | --- | --- | --- |
| 2.295 | 2.687 | −0.6089 | −0.6941 | (1, −3/2) |
| 2.2 | 2.5 | −0.6378 | −0.7148 | (1, −3/2) |
| 3.5 | 2.5 | −0.2979 | −0.3202 | (1, −3/2) |
| 11.0 | 1.2 | +0.0847 | +0.0736 | (12, −7) |

- **Three are the sector-1 block (1, −3/2), at small stretch and strong cross field.**
  - I should have seen this coming. My own competitor map (WP1) already showed (1, ±3/2) taking over at y ≥ 2 for
    t = 2, 3, 4.
  - My prereg even quoted that, in the B note, and I still drew A's P2-class box over that corner. So "y ≳ 1" was
    the wrong description of where P₂ governs.
  - In each case the (1, −3/2) block reaches her first. The P₂ root lies inside her region there and is not the edge.
- **One is the cut-off sector itself, (12, −7), at t = 11, y = 1.2.**
  - There the j ≤ 12 edge is positive (+0.074).
  - For x > 0 she is proved to lose to high sectors anyway, so this edge is an artefact of stopping at j = 12.
  - It is still a failure of the clause as I registered it.

## What I read from it

1. **The P₂ equation is right where (4, 0) is the competitor, and nothing else.** At 34 points never computed before,
   it gives the edge to 10⁻¹². Since P₂ comes from the same closed-form rows, that agreement follows once (4, 0) is
   the competitor; the prereg said this in advance. So what actually held is the **competitor identity**, over most
   of the box.
2. **The corrected description:** P₂ governs the j ≤ 12 edge for moderate cross field. At small stretch with a strong
   cross field, the sector-1 block (1, ±3/2) governs it instead. At large stretch with a positive j ≤ 12 edge, the
   cut-off sector shows up. Each region has its own polynomial (WP1 has them).
3. **Her region really is an interval with no holes**, at 48 fresh points (to the grid's resolution, 0.01).
4. **No high sector undercuts her inside her region at x < 0**, through j = 79. This is the arm that could have
   broken H1 outright, and it did not.

## Descriptive (no gate)

- **The lower edge** x_lo runs from −7.81 to −3.64. It is always set by the sector-0 block of charge −1 (with its
  mirror). The bound r_lo from the trial state θ⁰ ⊗ τ₀ is at least 1.6 below it everywhere, so the trial state is a
  bound and not the actual competitor.
- **The gauge-orthogonal edge** sits above the H + GG\* edge at every point, by 1.5 × 10⁻⁴ to 0.091.
- **The old Test 6 model** (c₁/t + c₂/t², nearest fitted height) misses the measured edge by 0.076 on average and
  0.20 at worst. The exact edge is the right object.
- **20 of the 48 j ≤ 12 upper edges are positive.** These are the cut-off artefacts the x > 0 proof already rules out
  for the all-sector question. The all-sector edge is at most min(x\*, 0).

## Status

- **COMPUTED** (double precision, j ≤ 12 for the edge, j ≤ 79 for A3).
- Nothing here is a proof.
- The A3 and A4 results make H1's corrected form, "an interval [x_lo, x\*], with no undercut through j = 79", a
  stronger candidate for an interval proof.
- A1's failure tells me which region the P₂ clause belongs to.
