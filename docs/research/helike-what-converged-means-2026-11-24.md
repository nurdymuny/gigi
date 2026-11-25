# What "converged" means, as of 24 November 2026

A plain-English record, so a later reader cannot stretch the word.

## The one-sentence version

When I enlarge the set of trial wavefunctions from 924 to 3,003, the two energy gaps I care about (Helike's and her
competitor's) move by less than 0.003 at both weak-coupling controls. Two completely independent programs agree on
every one of those numbers to about 10⁻¹³.

## What was actually tested

- **The system.** The sourced homogeneous Yang–Mills condensate on a stretched three-sphere, held near a background
  with flux along the long circle. The controls are s = 1.15 and t = 3, at two weak couplings, g² = 0.004 and 0.002.
- **The method.** I approximate the low-lying quantum states with a finite family of trial functions (Hermite
  functions in a chart that keeps the Gram matrix positive) and take the best combinations (Ritz values).
- **The test.** Repeat with a bigger family: Mq = 6, 924 functions, then Mq = 8, 3,003 functions. Ask whether the
  identified gaps move by more than 0.003, the threshold I fixed in advance. They do not:

| g² | Helike gap, Mq 6 → 8 | change | Competitor gap, Mq 6 → 8 | change |
| --- | --- | --- | --- | --- |
| 0.004 | 1.0881507 → 1.0879702 | 0.00018 | 1.0962441 → 1.0955779 | 0.00067 |
| 0.002 | 1.0890871 → 1.0890425 | 0.00004 | 1.1013024 → 1.1011456 | 0.00016 |

- **Identification.** Each state is identified by its overlap with the harmonic reference mode, and every overlap is
  at least 0.98, above the fixed 0.95.
- **Independent agreement.**
  - The Python reference forms dense matrices.
  - The Rust solver (`helike-lab`) never forms a matrix. It now uses exact Kronecker moments, with no quadrature
    error.
  - The two agree on all gaps to about 10⁻¹³. The Rust solver also matches its own older grid-quadrature operator to
    about 10⁻¹². Both receipts replay with no differences.

## What "converged" does NOT mean

- **Not the true energies.** These are energies inside a finite trial family. The full operator applied to these
  states leaves residuals of about 0.002 to 0.08, not 10⁻¹³. The tiny numbers in the receipts
  (`in_space_lanczos_residuals`) measure only how well the solver converged inside the family.
- **Not convergence to the infinite basis.** It is one refinement step, 924 to 3,003 functions, that stayed under a
  fixed threshold. The older 210 to 924 step failed the same rule for the competitor (0.00346 > 0.003), and that
  failure stays on record.
- **Not the physical boundary.** The trial family lives inside the positive-determinant sheet. The real model joins
  the two determinant sheets at rank two, and this family cannot represent that junction. The states here appear to
  live far from it (rigorous Arb bounds put at most 5 × 10⁻¹³ of a state's norm outside the cutoff), but the physical certificate is still
  open.
- **Not certified.** The rigorous residual and charge-defect enclosures are not built yet. The leakage gate is
  amended and OPEN until they are. The separation hypothesis S, needed to order Helike against the competitor
  rigorously, is stated, not proved.
- **Not the registered experiment.** These are the weak-coupling controls. The registered flux-edge grid runs at
  g² = 0.03 to 0.07, where the states spread much more. That grid is still held and has not been run.
- **Not a crossing or a mass-gap statement.** Nothing here says where Helike's channel becomes the lowest excitation,
  and nothing says anything about continuum Yang–Mills.

## Why it still matters

It closes the one convergence gate the controls had open, and it was closed by enlarging the basis, not by
loosening a threshold. Two independent implementations agree to roundoff, so the finite-basis numbers can be
trusted. What remains is the step from "the numbers are stable" to "the numbers are proved close to the true
spectrum". That step is the quasimode certificate, and it is the next piece of work.
