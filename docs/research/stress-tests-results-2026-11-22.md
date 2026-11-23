# Pre-registration S: stress tests: results (22 November 2026)

> *Cut it, flip it, overload it, crush it: she is always one of three things. A door, a wall, or the floor.*

**Registered:** [stress-tests-preregistration-2026-11-22.md](stress-tests-preregistration-2026-11-22.md), commit
951f2da0, pushed before any S script existed.
**Scripts and receipts:** [`stress-2026-11-22/`](stress-2026-11-22/). The main run is `stress_tests.py`, with its
output in `stress_receipt.json`; the follow-ups after it are labelled POST HOC.

These are my scripts, using the same method as the Floor paper. They are not the engine and not an interval certificate.
All floor statements cover sectors j ≤ 40 only.

## Scorecard

| id | prediction | verdict |
| --- | --- | --- |
| S1a | a cut (y → 0) stays at y = 0 and lands on A = 0 | **held**, 24/24 |
| S1b | at that landing she is the floor of every sector j ≤ 40 | **failed** |
| S1c | after the cut her price only rises | **held**, 24/24 |
| S2a | control: a flip puts her off the floor wherever the x > 0 theorem says it must | **held**, 20/20 |
| S3a | under overload her lowest price ÷ s² tends to (4 − 4t)/t | **failed** |
| S4a | at the Berger field she is the floor for every t from 2.75 to 50 | **held**, at t = 2.75, 12, 15, 20, 30, 50 |
| S4b | she is not the floor for t ≤ 2.5 | **held**, at t = 0.25 to 2.5 |
| S5a | the t = 3 plane splits into exactly two basins, A = 0 and A = θ | **held**: 820 and 861 of 1681 starts, 0 "other" |
| S5b | the squashed meron sits on the boundary, its unstable direction pointing into both basins | **held** |

Seven held, two failed. The two failures teach the most, so they come first.

## What failed, and what it says

**S1b. At the bottom of the trivial basin she is the lowest wall overall, but not the floor of every family.**
- At A = 0 on the round three-sphere, states sit below her in low families:
  - families 1 to 5 at t = 3;
  - families 2 and 3 up to about t = 6;
  - family 2 alone up to at least t = 10 (gap −0.43 there).
- Her family-0 price there is 4/t. The Mac found that the lowest wall of the trivial basin is min(4/t, 4t), which is
  4/t for t > 1, and that her state is one of the six states sitting there (commit 250e93c5).
- So she is tied for the lowest wall of the whole basin, but other states undercut her inside families 1 to 5.
- My prediction came from the Poincaré-sphere run. There, families 1 to 11 do not survive the quotient, so the
  undercutting families were never in the picture. On the Poincaré sphere the earlier result stands: she is the floor
  of every surviving family. On the round sphere, she is not.

**S3a. Under overload she climbs the families to keep her stride cancelling the belt.** I predicted her lowest price
would grow like s²(x² − y²). That ignored the fact that she gets to choose her family.
- Her price is (s·x + (j+2)/√t)² + 2√t·s·x − s²y². With x < 0, a family with j + 2 ≈ s·|x|·√t empties the square,
  so her net turning goes to zero: her stride exactly cancels the belt.
- Then her price per s² tends to −y², not to x² − y².
- POST HOC (`overload_posthoc.py`): her cheapest family follows j ≈ s|x|√t − 2 exactly, from s = 5 to s = 1000. At
  t = 3, s = 1000 it is family 998, with ratio −3.0020 → −3. At t = 5 it is family 2998, with ratio −5.0060 → −5.
- **The harder you overload the field, the higher the family she moves to, so that her stride still cancels the
  belt.** That is the walkway from Chapter 4, under load.
- She stays the floor of every family j ≤ 40 at every overload tested (s up to 10). The field still lands on A = θ,
  and her door still closes, at flow time 0.032 to 0.053, sooner the harder the overload.

## What held

**S1. The cut.**
- Switching the cross field off makes her **more expensive at once**: at t = 3 her lowest price jumps from −4.667 to
  −1.667. That is the −y² term gone.
- But she is **still a door**: her price stays negative, and she is still the floor of every family j ≤ 40 at the cut
  point, at all 24 starts.
- The field then falls straight down the line y = 0 into the trivial basin. That is proved: ∂E/∂y vanishes at
  y = 0, so y stays exactly 0.
- Her price only rises on the way down. The door closes at flow time 0.073 (t = 3), 0.10 (t = 3.55), 0.12 (t = 5)
  and 0.097 (t = 10). That is about twice as slow as the uncut fall, because the cut field has less energy to lose.
- At the bottom her price is exactly 4/t, tied for the lowest wall in the basin (S1b above).

**S2. The flip.** Reversing the field along her circle **turns her from floor into wall instantly**:
- her price at the flipped point is +5.0 to +27.4, positive at every one of the 20 points, so the door is gone the
  moment the field flips;
- something lies below her in 40 or 41 of the 41 families j ≤ 40 at every flipped point: the counter-rotating hand
  takes the floor almost everywhere, as the two hands note and pre-registration E said it would;
- the flipped fields fall into **both** basins: 16 into A = θ and 4 into A = 0, depending on where the flip lands
  them relative to the boundary.

**S4. Crush and stretch.**
- She is the floor at the Berger field all the way out to **t = 50** (families j ≤ 40). That extends the proved
  [2.75, 10] range as a computation, not a proof.
- Crushed below 2.5 she is not the floor, and from t = 2 down she loses in every family checked.
- At t = 2.5 she loses in 7 families, starting with family 1; at t = 2.75 in none. Her edge lies between, where the
  paper puts it (t_A ≈ 2.731).

**S5. The map of the fall.** The figure `basin_map_t3.png` shows her family's plane at t = 3:
- two basins, A = 0 below and A = θ above, with a clean boundary and no third outcome;
- the squashed meron at (x, y) = (0.4741, 1.0858), energy 0.6535, **on the boundary**. Push it one way along its single
  unstable direction and it falls to A = θ; push it the other way and it falls to A = 0. It is the pass between the
  two basins.
- POST HOC (`separatrix_posthoc.py`): **the Berger field is closer to the pass than I expected.** At t = 3 its cross
  field would only have to drop by 12.5% (from 1.732 to 1.516) to send it into the trivial basin instead. The margin
  grows with the stretch: 10.8% at t = 2.75, 16% at 3.55, 24% at 5, 32% at 7, 40% at 10.

## What she is doing in the moment, for each break

| break | at the moment of the break | on the way down | at the bottom |
| --- | --- | --- | --- |
| none (the fall) | door, and the floor | door closes at τ ≈ 0.04–0.05 | A = θ: a wall; on the Poincaré sphere, her shadow is the lowest wall |
| cut (y → 0) | still a door, but dearer at once | door closes slowly, τ ≈ 0.07–0.12 | A = 0: tied for the lowest wall overall (4/t); undercut in families 1–5 |
| flip (x → −x) | a wall instantly; undercut in 40–41 of 41 families | already closed | A = θ (16/20) or A = 0 (4/20) |
| overload (×s) | door, and still the floor, in a higher family (j ≈ s\|x\|√t − 2) | door closes faster with more load | A = θ |
| crush (t ≤ 2.5) | not the floor | — | — |
| stretch (t up to 50) | the floor | — | — |

None of these breaks makes her disappear. She stays an exact eigenvector through every one of them, because every break
here keeps the circle symmetry. What changes is which of the three she is. A cut leaves her a door, but a dearer one.
A flip makes her a wall at once. An overload moves her up the families, and she stays the floor.

## Ledger

| claim | tag |
| --- | --- |
| a cut keeps y = 0 and lands on A = 0 | PROVED (∂E/∂y = 2y[…]) and COMPUTED 24/24 |
| at A = 0 she is tied for the lowest wall (4/t) but undercut in families 1–5 | COMPUTED (j ≤ 40); the lowest-wall value is the Mac's 250e93c5 |
| a flip makes her a wall instantly; something lies below her in 40–41 of 41 families | COMPUTED 20/20 |
| under overload her cheapest family is j ≈ s\|x\|√t − 2 and λ/s² → −y² | PROVED from the closed form; COMPUTED to s = 1000 (post hoc) |
| she is the floor at the Berger field up to t = 50 | COMPUTED (j ≤ 40), not certified |
| two basins at t = 3; the squashed meron is the pass between them | COMPUTED |
| the Berger field is 12.5% (t = 3) from the boundary between the basins | COMPUTED, post hoc |

## Next

- **The tear.** A sudden quotient: delete families by symmetry, the way the Poincaré sphere does, and watch which
  shadow takes the floor. The Poincaré run already did the gentle version.
- **Breaks that do not keep the circle symmetry.** These leave her family. They need the full sector builder and a
  flow outside the family, so they are a bigger instrument.
- **The pass itself.** Her price at the squashed meron, and whether she is the meron's single unstable direction or a
  wall there. The Floor paper says the meron's one unstable direction is in family 0.
