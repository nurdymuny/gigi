# Pre-registration G, interim: G₂ is complete, Sp(3) is running (23 November 2026)

**This note is not the verdict.** G is scored once, at the end, when both groups are in (`--merge`), exactly as
registered. This note reports the half that is done, so I can see it. It changes nothing in the registration, and I
have not looked at any Sp(3) data, because none exists yet.
- **The registration:** `research/edge-theorem-20261122-g/wp8_prereg_G_two_new_groups.md`, revised after an
  adversarial review (one fatal problem and seven major ones, all fixed before any data) and registered at 333d97c.
- **Script:** `scripts/edge_wp8_two_new_groups.py`, with `scripts/edge_wp8_fast.py`.
- **G₂ receipt:** `research/edge-theorem-20261122-g/wp8G_g2.json`, with its log beside it.

## What happened to the run

- **Both processes started** at about 1 am, as the registration says, one per group. Both passed the opening gates:
  - G-K0 (the new code against F's dense code): 3.7 × 10⁻¹⁵;
  - G-K1 (one full F point recomputed end to end): 5.8 × 10⁻¹⁵.
- **I stopped the Sp(3) process** after two hours. It was still building its opening circle gates, and the machine's
  swap was nearly full. It had produced no data point.
  - It restarted automatically when G₂ finished (3:53 am), alone, with all sixteen threads.
  - It reruns every gate itself.
  - Running the two groups one after the other instead of side by side changes nothing else.
- **G₂ finished:** all 15 points. Every gate passed at every circle (G-K2, G-K3, G-S5 and G-S5c, G-B, G-X, G-sym,
  G-quad, G-lin, G-mult, G-gauge, G-bound), mostly at 10⁻¹⁵.

## G₂, cell by cell (unscored)

**Lives:** a nonempty floor region that survives the full truncation check, including the new 2θ sector (77′) that
the review insisted on.

| circle | what it is | shapes that live | φ\* at (½,½,¼), (½,½,¾), (½,1,1) | what blocks her where she does not live |
| --- | --- | --- | --- | --- |
| g0 | θ^∨, her own circle | **2 of 3** | **0.082, 0.161**, −0.273 | 27 |
| g1 | half way to ρ^∨ (9.55°) | **2 of 3** | 0.037, 0.098, −0.287 | the adjoint, 14 |
| g2 | ρ^∨, the principal circle | **0** | −0.358, −0.261, −0.584 | 14 |
| g3 | half way to the wall (24.55°) | **0** | −0.644, −0.502, −0.881 | 14 |
| g4 | the far wall (θ_s^∨, a corner ray) | **1 of 3** | +0.007, −0.086, −0.669 | the trivial sector |

(For the empty shapes, φ\* is the golden-section best margin.)

**Her regions**, where she lives:
- g0: [−1.72, −1.26] and [−1.79, −1.09];
- g1: [−1.52, −1.17] and [−1.72, −1.09];
- g4: [−2.01, −1.25] at (½,½,¼) only.

**Every one of her regions sits at x < 0**, on the side where the field along her circle turns against her charge,
as in every test since D.

## What it looks like, read plainly (not a verdict)

**It looks like F again.**
- She lives on her own circle and one step inside it.
- Her margin is highest right on her own circle at all three shapes.
- She is gone at the principal circle and past it.
- The (½, 1, 1) shape is empty everywhere, as it was in F at Sp(2)'s 0°.

If Sp(3) does the same, F's "her own circle" holds in four groups, and in two of them I had computed nothing before
writing the rules down.

**The far wall is the interesting cell.** At G₂'s far wall she is only marginal, with a margin of 0.007 at one shape.
That is not the clean return Sp(2) gave. It is closer to SU(4)'s far wall, where she was also marginal. I said in the
registration that G₂'s far wall cannot test my short-root guess, because there the short root is not even at the top.
So this one cell says only that Sp(2)'s far-wall return did not repeat strongly in another rank-2 group. The real
test of why she came back is Sp(3)'s three circles (sS, s4, s5).

## What I will not do

- I will not score anything until the merge.
- I will not read Sp(3) points as they arrive as anything but progress.
- If Sp(3) stops at a gate, the stop is reported with what had been seen, as every amendment has been.
