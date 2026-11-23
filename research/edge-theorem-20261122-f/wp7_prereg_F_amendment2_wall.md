# Pre-registration F, Amendment 2: her state on a wall where the top is degenerate and untied (22 November 2026)

Written and committed **after gate F-mult stopped the run, and before any further data is read.**

**What the session running F has seen** (in addition to the three 0° printouts disclosed for Amendment 1):
- only this stop message:

  > `stop: F-mult sp2 45.0 (0.5, 0.5, 0.25) 1: resid 3.4e-15, spread 1.08`

- No point at 9.22° to 36.87°, and no other number from the receipt, has been read.

## What happened

At 45° (the C-type wall), her root is no longer the unique top root under e_K: three roots tie for the top in Sp(2).
So in each sector the space of top Q_K-charge has more than one state.
- The registration said her copies would then be "the whole tied top space, by the C1 rule".
- That assumes the states in it share one price, as in Test C, where a non-abelian symmetry forced the tie.
- Here nothing forces it. The top-charge space is invariant under H (residual 3.4 × 10⁻¹⁵), so its states are exact
  eigenvectors. But their prices differ: in the trivial sector at the first shape the spread is 1.08.
- So "her copies" is not defined there, and gate F-mult stopped the run, correctly.

## The amended rule (fixed now)

**Where her top-charge space is degenerate and not tied** (this can only happen at the walls; F-K3 records where her
top root is unique):
- **Her state is the cheapest exact state of top charge.** That is the lowest eigenvector of H restricted to the
  top-charge space, in each sector, at each x.
  - This gives her the best chance, so it is conservative against H_C (which predicts she does not live at 45°) and
    generous to every hypothesis that predicts she lives there.
- **Her copies** are that state, plus the cheapest state of the bottom (mirror) charge space when it is tied with
  hers (to 10⁻⁹ relative), as before.
- **The other states of the top space are competitors.** They stay in the spectrum.
- **F-mult at such a wall:** the top-charge space is exactly invariant (off-block residual ≤ 10⁻⁹‖H‖), and the mirror
  tie is reported. Spread within the top space is reported, not gated.
- **F-quad at such a wall:** the quadratic part of H restricted to the top space is q_K²·I, to 10⁻⁹. (Every state of
  top charge has charge q_K under e_K.) Her lowest eigenvalue need not be exactly quadratic in x, because the linear
  part can mix the top states; that is reported, not gated.
- **Everywhere her top state is unique** (every angle except 45°, by F-K3), the amended rule coincides with the
  registered one: the cheapest state of a one-dimensional top space is that state. **Points already computed are
  therefore unaffected and are kept.** The run resumes at Sp(2) 45°.

Nothing else changes: no hypothesis, prediction, threshold, angle, shape or sector. Amendment 1 (H_P-peak) is
untouched.

## Disclosure for the verdict

The results note will say:
- that 45° was scored under this amendment;
- that the amendment was written after the stop and before any further data;
- exactly what had been seen (above).

## Correction to the implementation (22 November, after F's first complete receipt; disclosed)

**What went wrong.** When I implemented this amendment, I applied "her state is the cheapest exact state of top
charge, and her copies are that state plus its mirror" to **every** degenerate top-charge space. This amendment covers
only the **untied** ones. Where the top space is degenerate but its states are exactly tied (forced by a wall's
reflection symmetry, as in SU(4) at 0° in sector 6), the registered C1 rule still applies: her copies are the whole
tied space.

**How it showed.** In the first complete receipt, several wall points have her margin at exactly the floor tolerance
(10⁻⁹). There she "ties" a state that is really her own tied partner, left in the spectrum by the bug.

**What this session had seen when writing this:** F's complete first receipt, all 36 points and the scores. This
correction is therefore **not blind**. It is a mechanical fix of the code to the rule already written, with no new
choice. The combined rule, consistent with both the registration (C1) and this amendment:
- her copies are the top-space states tied with the cheapest one, to 10⁻⁹ relative: the whole space when all are
  tied (C1), just the cheapest when none are (this amendment);
- plus the mirror states tied with them.

**What is recomputed.** Every point computed under the faulty rule where her top space is degenerate: SU(4) at 0°,
SU(4) at 45°, and Sp(2) at 45°, which is 9 points. Sp(2) at 0° was computed before the resume, under the registered C1
code, and is unaffected. Interior angles have a one-dimensional top space and are unaffected. The results note will
report both versions of these 9 points.
