# Test 2, family 1: the squashed meron is class N (20 November 2026)

Protocol: [`skeleton-tests-preregistration-2026-11-20.md`](skeleton-tests-preregistration-2026-11-20.md) §4,
"Protocol for family 1", committed in 5063d460 before this script existed. Script `scripts/ricci_su2_test2_family.py`.
Receipts: `research/su2-floor-family-20261119/test2_berger_control.json` and `test2_meron.json`. Status: **Numerical**
(double precision).

## Code control: passes

On the Berger family (the Levi-Civita field of (t, 1, 1)) the script gives T_all = {2.70, 2.75, …, 10.00}, exactly
the required set, so the class is P. The per-sector onsets are listed below; j = 4 is the last sector to become the
floor, as required.

| j | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| floor from t = | 2.25 | 2.55 | 2.45 | 2.50 | 2.70 | 2.65 | 2.60 | 2.55 | 2.50 | 2.45 | 2.40 | 2.40 | 2.35 |

## Family 1, the squashed meron: class N

- **The extremal state is the floor at no grid point, in no sector.** T_all is empty, and so is every per-sector
  set.
- **Checks held everywhere.** At all 2,600 (t, j) pairs the extremal state was an exact eigenvector of H and
  gauge-orthogonal:
  - residual ≤ 4.7 × 10⁻¹²;
  - ‖G*b‖ ≤ 8.5 × 10⁻¹⁵.

  This is as the counting argument requires, which confirms the meron's circle invariance numerically.
- **Consistency at t = 1.** There the meron is the Berger field, and the extremal values 10, 17, 26 for j = 0, 1, 2
  equal the floor law λ₁(j) = j² + 6j + 10. They also appear in the published meron spectrum (−2, 1, 6, 7, 10 in
  spins 0 to 2).

**Verdict by the pre-registered rule.** Any independent tested family classified N counts against the scoped
hypothesis. This one is N. So the scoped hypothesis is **counted against**:

> In every independent circle-invariant family, the unique maximal-charge state is the floor of every listed sector
> on a parameter range of positive length.

## What this does and does not touch

- **Untouched.** Everything certified about the Berger field:
  - the Floor Theorem at t = 3 and t = 4;
  - the floor law on [2.75, 10];
  - the Landau structure of the twist, now proved.

  Those are statements about one background, and they stand.
- **What falls.** The reading that the extremal mode is the floor *because of the circle symmetry*, in any
  circle-invariant setting. On the resting field of the same family of shapes, with the same symmetry, the extremal
  state is never the floor.
- **What the floor property depends on, then.** It is something about the Berger field beyond its symmetry. One
  candidate is that the Berger field is not at rest. That is not tested here.

## Exploratory (not pre-registered, no gate)

Where the extremal state sits in the gauge-orthogonal spectrum of the squashed meron:

| t | j = 0 | j = 2 | j = 4 | j = 8 | j = 12 |
|---|---|---|---|---|---|
| 0.5 | top (the doubled highest value) | top | top | top | top |
| 1.0 | top | top | top | top | top |
| 3.0 | 2nd of 6 | 8th of 18 | 11th of 30 | 11th of 54 | 9th of 78 |
| 10.0 | 2nd of 6 | 7th of 18 | 6th of 30 | 5th of 54 | 5th of 78 |

Rank counts the eigenvalues strictly below the extremal value, plus one. "Top" means the extremal value is the
largest eigenvalue of the sector. At t = 1 it is shared with other states, because the round sphere's larger
symmetry adds degeneracy.

- For t ≤ 1 the extremal state is the **ceiling** of every sector.
- As the sphere stretches, it descends through the spectrum but does not reach the floor by t = 10.
- On the Berger field it is the floor from t ≈ 2.7.

So the same symmetric state is the ceiling on one circle-invariant background and the floor on another. That
suggests the skeleton, if there is one, lies in the extremal state's *position* moving with the background, not in
the state being the floor. It is an observation, not a test.
