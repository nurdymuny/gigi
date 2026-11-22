# Pre-registration F, Amendment 1: a peak hypothesis that can tell the principal circle from "any regular circle"

Written and pushed **before any F result is in the repository.**
- At the time of writing, `research/edge-theorem-20261122-f/` holds only the registration. There is no receipt
  `wp7_principal_circle.json` and no results note. F is still running on the other machine.
- The author of this amendment (the Windows session, at Bee's request) has seen **no F output of any kind**.
- **If the session running F has seen progress printouts** before this amendment reached it (the script prints each
  point as it finishes), the results note must say so. This amendment is then scored and reported as **not blind**.

This amendment adds one hypothesis and its scoring rule. It changes no existing hypothesis, gate, threshold, shape,
angle, sector or code. It is scored from fields the receipt already records.

## Why it is needed

As registered, F cannot single out the principal circle.
- **H_reg** predicts she lives at every interior angle (9.22°, 18.43°, 27.65°, 36.87°) and at neither wall.
- **H_P** predicts the angles where she lives form one contiguous run that contains 18.43° and contains neither wall.

Every outcome that supports H_reg also satisfies H_P. So "she lives at all four interior angles" would be counted as
support for the principal circle without showing that the principal circle is special. To separate the two, H_P
needs a prediction H_reg does not make. The natural one is that **her margin is largest at the principal circle.**

## The statistic

For each (group, angle, shape), the peak statistic is **φ\*** = the receipt's `phi_grid_best`. That is the largest
value, over the scan grid in x (step 0.25), of φ_A: the lowest other state in the main sectors minus her price, her
copies removed, as defined in `scripts/edge_wp7_principal_circle.py`.
- φ\* is recorded whether or not her floor set is empty.
- It is in eigenvalue units of the same operator at every angle (same metric, same t = 3), so it compares across
  angles.
- Where the receipt has no `phi_grid_best` (no real root of the bounding quadratic: she is never the floor of the
  trivial sector), φ\* = −∞ for ranking.

**Why not the width of her floor set.** Her coupling to the field along the circle is q_K, which falls from 1 to 0.707
along the arc (§0 of the registration). A width in x is stretched by roughly 1/q_K, which would bias any peak in width
toward larger angles. The margin has no such bias.

## H_P-peak (new, registered now)

**Prediction.** In each group, for at least 2 of the 3 shapes, the angle that maximizes φ\* over all six angles
(0°, 9.22°, 18.43°, 27.65°, 36.87°, 45°) is **18.43°**.

**Scoring, per group:**
- **Supported:** at least 2 of 3 shapes peak at 18.43°.
- **Refuted:** at least 2 of 3 shapes peak at some other angle.
- **Not decided:** otherwise.
- **Ties:** if the top two values of φ\* for a shape are within 10⁻⁶·max(1, |φ\*|), that shape counts as "not
  decided" for this hypothesis.

**Overall:** H_P-peak is supported only if it is supported in **both** groups, and refuted if it is refuted in either.
A split between groups is reported as a finding, not a verdict, as in the registration.

**What it means.**
- If H_reg holds and H_P-peak is supported, she lives on every regular circle on the arc but is most firmly the floor
  on the principal one.
- If H_reg holds and H_P-peak is refuted, she lives on the regular circles but the principal circle is not where she
  is strongest. The principal reading of *the arc* (Part II.1) would then not be supported by F.

## Reported alongside, not scored

- **The local shape:** whether φ\*(18.43°) exceeds both neighbours, φ\*(9.22°) and φ\*(27.65°), per shape. A peak at
  18.43° that is not a local maximum cannot occur by construction; a local maximum at 18.43° that is not the global
  maximum is reported.
- **Sp(2)'s Weyl vector.** In Sp(2) the Weyl vector ρ (as opposed to ρ^∨) sits at 26.57°, between 18.43° and
  27.65°. A peak at 27.65° in Sp(2) together with a peak at 18.43° in SU(4) is reported as a split that points at ρ
  rather than ρ^∨. That is a description only; ρ is not on the grid.
- **The monotone alternative.** Whether φ\* is monotone along the interior angles, increasing or decreasing. A
  monotone φ\* would mean she simply prefers one end of the arc, and nothing singles out the principal circle.

## Scope (unchanged from the registration, restated because the peak inherits it)

- Everything is at t = 3 only.
- In SU(4) the arc is the one diagram-symmetric line through a two-dimensional range of directions. "Peaks at 18.43°"
  means "on this line". In Sp(2) the arc covers the whole range of directions.
- The angle grid is coarse, about 9° per step. A peak "at 18.43°" means the best of these six angles, not a located
  maximum.

## Note on the registration text (no effect on the run)

§2 S2 lists 2q_K² at 9.22° as 1.974. The correct value is 2 × 0.987² = **1.949** (1.974 is q_K² itself). The gate
F-quad uses the computed q_K, not the typed list, so the run is unaffected.
