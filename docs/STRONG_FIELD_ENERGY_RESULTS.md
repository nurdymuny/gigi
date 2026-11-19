# S4: lattice energies of a strong SU(2) field (results)

2026-11-19. Spec: `docs/STRONG_FIELD_ENERGY_SPEC.md` (committed before measurement, 11ec4ea).
Script `scripts/su2_strong_field_energy.py`; receipt `research/su2-strong-field-20261119/energies.json`.
Exploratory addendum (post hoc, not gated): `scripts/su2_strong_field_axis_addendum.py`,
`research/su2-strong-field-20261119/axis_addendum.json`. Research scripts, **not engine runs**.

## In plain English

We made the field strong enough that faces turn by up to 2.9 radians and compared five ways of
turning a face's turn into energy. Three things came out.

1. **On these meshes none of the readings is what limits accuracy.** Even at strong field, the
   ordinary error of a coarse mesh is 3 to 15 times larger than any difference between readings.
   The paper's expectation that the readings matter "for strong fields and coarse lattices" did not
   hold here: the coarse lattice is the larger problem.
2. **With GIGI's native chord reading, the 2:1 blend cancels less than predicted.** It still beats
   the chord and the tangent, but not by the predicted factor on the finer meshes, and the
   adjoint-improved action of de Forcrand et al. does much better.
3. **The reason is the chord's scalar part, and fixing it restores the cancellation.** GIGI's action
   couples neighbouring faces through the Whitney mass. The native chord reading U − 1 has a scalar
   part, cos a − 1, which enters those couplings at exactly the order the blend is meant to cancel;
   the tangent reading has none, so the two do not cancel. Reading the chord along the holonomy's
   axis, 2 sin(a/2)·n, removes the mismatch, and the blend then cancels as Proposition 2 predicts:
   100 to 3,000 times better than the chord alone. This was found after the gated run and is
   reported as exploratory.

## Controls

| control | threshold | measured |
| --- | --- | --- |
| C-native: chord energy vs the engine's native action, (3,1,1), n = 1 / n = 2 | 10⁻¹² | 1.5×10⁻¹⁶ / 2.9×10⁻¹⁶ |
| exact links vs stored engine links, (3,1,1) | (reported) | 1.7×10⁻¹⁶ / 2.2×10⁻¹⁶ |
| C-arc-small: all readings agree, A = 3, n = 2 | 10⁻³ | 5.3×10⁻⁵ |

The continuum and lattice energies share one normalisation: at A = 3, E_arc/E_cont − 1 is −4.6%,
−0.83%, −0.32% at n = 1, 2, 3, converging.

## Results

δ_X = E_X/E_arc − 1 (compactness spread against the arc reading); e = E_arc/E_cont − 1
(discretization error).

| n | A | max angle | faces > 2 rad | δ chord | δ tangent | δ blend | δ adjoint | e |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | 0.094 | 0 | −4.9e−4 | +7.9e−4 | −6.1e−5 | −9.4e−7 | −4.6% |
| 1 | 10 | 0.51 | 0 | −1.2% | +2.0% | −0.14% | −0.055% | −9.2% |
| 1 | 30 | 1.48 | 0 | −8.0% | +17.4% | +0.45% | −1.3% | −27.9% |
| 1 | 100 | 2.87 | 40 | −28.9% | +302% | +81.3% | −15.2% | −74.6% |
| 2 | 3 | 0.025 | 0 | −2.4e−5 | +5.3e−5 | +1.8e−6 | −2.9e−10 | −0.83% |
| 2 | 10 | 0.15 | 0 | −5.5e−4 | +1.6e−3 | +1.6e−4 | +1.2e−6 | +0.49% |
| 2 | 30 | 0.63 | 0 | −0.47% | +2.0% | +0.36% | +0.070% | +16.2% |
| 2 | 100 | 1.53 | 0 | −7.8% | +14.4% | −0.39% | −1.3% | −18.1% |
| 3 | 3 | 0.011 | 0 | −4.4e−6 | +1.1e−5 | +5.9e−7 | +1.7e−11 | −0.32% |
| 3 | 10 | 0.067 | 0 | −8.4e−5 | +3.3e−4 | +5.3e−5 | +1.1e−7 | +0.67% |
| 3 | 30 | 0.32 | 0 | +0.085% | +0.52% | +0.23% | +0.011% | +19.3% |
| 3 | 100 | 0.92 | 0 | −2.2% | +4.8% | +0.10% | +2.7e−6 | +35.5% |

At n = 1, A = 100 the tangent energy is four times the arc's: 40 faces turn by more than 2 radians,
near the U = −1 barrier, and the largest single face carries 7% of the tangent energy.

## Predictions

| prediction | verdict |
| --- | --- |
| P-cancel: \|δ_blend\| ≤ \|δ_chord\|/5 and ≤ \|δ_tangent\|/5 where max angle ≤ 1.5 | **PASS** at n = 1 (A = 3, 10, 30), n = 2 (A = 3), n = 3 (A = 3, 100); **FAIL** at n = 2 (A = 10, 30) and n = 3 (A = 10, 30) |
| P-series: each δ within 25% of its energy-weighted per-face series where max angle ≤ 0.6 | **FAIL** at every eligible case |
| P-regime: at n = 1, \|δ_chord\| > \|e\| for A ≥ 30, and < \|e\| at A = 3 | **FAIL** for A ≥ 30 (8.0% vs 27.9%; 28.9% vs 74.6%); holds at A = 3 |

## Exploratory addendum: the chord read along the holonomy axis (post hoc)

Written after the gated results, to test the explanation above. The chord is read as 2 sin(a/2)·n
(a pure vector with the chord's length) instead of U − 1, and blended 2:1 with the tangent as before.

| n | A | δ axis-chord | δ axis-blend |
| --- | --- | --- | --- |
| 1 | 3 | −3.9e−4 | +3.0e−7 |
| 1 | 10 | −0.99% | +2.0e−4 |
| 1 | 30 | −6.9% | +1.2% |
| 1 | 100 | −22.1% | +85.9% |
| 2 | 3 | −2.6e−5 | +1.3e−9 |
| 2 | 10 | −7.9e−4 | +1.2e−6 |
| 2 | 30 | −0.98% | +2.0e−4 |
| 2 | 100 | −5.8% | +0.91% |
| 3 | 3 | −5.3e−6 | +5.2e−11 |
| 3 | 10 | −1.6e−4 | +5.2e−8 |
| 3 | 30 | −0.26% | +1.4e−5 |
| 3 | 100 | −2.2% | +0.11% |

Wherever the largest face angle is below about 0.6, the axis-blend is 100 to 3,000 times smaller than
the axis-chord, and of the size of Proposition 2's 7a⁴/720 at the median angle (for example
1.2×10⁻⁶ against about 9×10⁻⁷ at n = 2, A = 10). Near the barrier (n = 1, A = 100) the tangent term
dominates and no blend helps.

## What it means

- **For *Three Ways*.** Proposition 2 is a per-face statement and holds exactly for an action that is
  a sum over faces (Wilson's). On an action that couples faces, such as GIGI's Whitney-mass action,
  it carries over only when all readings point along the holonomy's axis; the native Wilson reading
  U − 1 does not. The adjoint-improved action, made of two chord-type readings, cancels well on
  either kind.
- **For the strong-field question.** On these meshes the compactness corrections are not what
  limits accuracy at any field strength tested; discretization error is. Part of that error comes
  from the strongly non-round metric itself (A = 100 stretches the round-shaped cells 10:1), so
  "strong field" and "badly resolved mesh" are not separated here.
- **For a Monte Carlo study in Halcyon (Bee's "then decide").** These results do not make the case
  for one on the 2:1 blend with the native chord reading. If one is run, the candidates worth
  comparing are the axis-blend and the adjoint-improved action, on meshes fine enough that
  discretization error is below the compactness spread.

## Not measured

Forces, other metrics, a field that is strong without the metric being non-round, Monte Carlo.
