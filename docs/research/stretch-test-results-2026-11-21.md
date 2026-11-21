# The stretch test (Test 5): results (21 November 2026)

The protocol was fixed before any code or data:
[`stretch-test-preregistration-2026-11-21.md`](stretch-test-preregistration-2026-11-21.md). Receipt:
`research/su2-floor-family-20261119/stretch_test.json` (`scripts/ricci_stretch_test.py`).

**The hypothesis under test** (from the exploratory reading of Test 4): the field has to point at her **and only at
her**. Her frame and its conjugate alone span the lowest eigenspace of R₀ = Ric − 2[F, ·].

**Controls**, all passed before any result was read:
- D1: the meron spectrum −2, 1, 6, 7, 10 is reproduced.
- D2: the Weitzenböck form with R₀ = Ric ⊗ I − 2[F, ·] holds to 4.4 × 10⁻¹⁵ at four grid points.
- D3: at t = 1 the results agree with Test 4b in every sector.

## Gated result: the hypothesis fails

SU(2), A = sθ on (t, 1, 1). The grid is s ∈ {0.25, …, 3} × t ∈ {0.5, …, 8}, 72 points, with sectors j ≤ 12. Every
point was classified from R₀ before the floor test.

| class (from R₀ alone) | points | twist the floor of every sector |
| --- | --- | --- |
| L1, singled out | 35 | **0** |
| L2, shared | 5 | 0 |
| L0, not lowest | 32 | 0 |

- **P-suf fails.** There are 35 singled-out points (aligned, s ≥ 1.25 with t ≥ 1.5), and the twist is the floor of
  every sector at none of them.
- **P-nec holds only vacuously.** The twist is the floor of every sector nowhere on the grid.
- No check stopped the run.

**Verdict.** "Singled out by the curvature term" is not sufficient. Taken with Test 4 ("aligned" is not sufficient)
and Test 2 (three families where the twist is never the floor), the curvature term alone, its sign and its
uniqueness, does not decide whether the twist is the floor.

## What she does win (reported, no gate)

- At every singled-out point the twist is the floor of the **uniform sector j = 0** and of no sector j ≥ 1.
- Her gap above the floor grows with j, roughly in step with it. For example, at s = 1.5, t = 3 it is 6.6, 12.6,
  25.7 and 41.1 for j = 1, 2, 4 and 8. What she loses on is the cost of **lifting** her to higher sectors.
- The gaps **shrink with the stretch**: in j = 1 at s = 1.5 they are 13.8, 6.6 and 2.1 at t = 1.5, 3 and 8. Whether
  they close beyond t = 8 is not tested.

## Exploratory reading (not tested)

On the Berger field, the lift from j = 0 to sector j costs the scalar Laplacian on her top weight **minus 4(t − 2)j/t**
(the Floor paper's Casimir remark). That subtraction comes from the connection's component along the long circle
coupling to her momentum along it: the fibre-momentum term κ of her Landau anatomy. The A = sθ backgrounds lack
this particular coupling, and she wins only where no lift is needed (j = 0).

**Candidate for the next pre-registration:** the twist is the floor of sector j when the background's fibre
component makes her lift cheaper than every competitor's. This is a kinetic statement, not a curvature one. It
would be tested on a background where that coupling can be dialled independently of the curvature term.

## The record so far, honestly

| reading | test | outcome |
| --- | --- | --- |
| symmetry makes her the floor (scoped skeleton) | Test 2: meron, SU(3) 2a, 2b | counted against (three families N) |
| the field strength aligned with her spin makes her the floor | Test 4 (4b) | counted against (aligned, N) |
| aligned **and** singled out makes her the floor | Test 5 | counted against (35 singled-out points, none floor) |
| she is the floor on the Berger field, every sector, t ∈ [2.75, 10] | Floor paper v2 | **proved** |

What is proved stands. What makes the Berger field special is still open, and it now looks kinetic (her lift), not
only curvature.
