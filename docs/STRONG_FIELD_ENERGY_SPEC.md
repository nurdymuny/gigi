# S4: lattice energies of a strong SU(2) field — chord, arc, tangent, the 2:1 blend, adjoint (spec)

2026-11-19. Written before any measurement in this file (a scratch probe fixed only the
normalisation and the angle range; see "Calibration"). Research script
`scripts/su2_strong_field_energy.py`, **not an engine run**. It is the deterministic first step
Bee chose for the open item of *Three Ways Across a Turn* (doi 10.5281/zenodo.24375338,
Section 12): whether the chord–tangent energy blend matters when face turns are not small.

## In plain English

On the meshes of our curvature work every face turns by less than 0.1 radian, and there the
chord (Wilson) and the arc (Manton) energies differ by less than 0.1%. To see whether the 2:1
blend of *Three Ways* helps, the turns must be large. Making the Berger metric strongly
non-round, (A, 1, 1) with A up to 100, makes its Levi-Civita field strong on the same meshes:
face angles reach 1–3 radians. The exact links and the exact continuum Yang–Mills energy are
known, so every lattice energy can be checked against the truth. Two effects are measured
separately: how the readings of each face's turn differ from one another (compactness, the
effect the blend addresses), and how the best reading differs from the continuum (ordinary
discretization error from a coarse mesh). The question is which one dominates.

## Fields and meshes

- Berger metrics (A, 1, 1), A ∈ {3, 10, 30, 100}; exact links from `Lattice.exact_links(g)`;
  Levi-Civita connection coordinates x = Φ(g), curvature f_i = x_j x_k − 2x_i.
- Continuum energy E_cont = ½(f₁²/(BC) + f₂²/(CA) + f₃²/(AB)) · Vol(g).
- Shape-regular meshes n = 1, 2, 3 (q6 quadrature), metric face masses M from
  `Lattice.metric_pieces(g)`.

## The five lattice energies

Each cell contributes w_c · 2 Σ_ij M_ij ⟨R_i, R_j⟩ over its four faces, with the face reading R_i
built from the registered face holonomy U_i = cos a + n sin a (the native definition):

| name | reading R(U) | per-face square |
| --- | --- | --- |
| chord (Wilson, native) | U − 1 | 4 sin²(a/2) |
| arc (Manton) | a·n | a² |
| tangent (Lüscher at ε = 2, up to the factor ½) | 2 tan(a/2)·n | 4 tan²(a/2) |
| blend (*Three Ways*, Prop. 2) | (2·chord + tangent)/3 of the energies | a² + 7a⁶/720·… per face |
| adjoint (de Forcrand et al. 1998) | (4/3)·chord(U) − (1/12)·chord(U²) of the energies | a² − a⁶/90·… per face |

(Per-face expansions are in the angle a of S³; the energies are the mass-weighted forms.)

## Measured, for every (n, A)

- E_X / E_cont for the five readings.
- Compactness spread δ_X = E_X/E_arc − 1, and the energy-weighted predictions from the per-face
  series (chord −a²/12, tangent +a²/6, blend +7a⁴/720, adjoint −a⁴/90).
- Discretization error e_disc = E_arc/E_cont − 1.
- Face-angle median, maximum, and counts above 1 and 2 radians; the largest single-face
  tangent share (the U = −1 barrier).

## Predictions (fixed now)

- **P-cancel (Proposition 2 on a real mesh).** Wherever the largest face angle is at most 1.5:
  |δ_blend| ≤ |δ_chord|/5 and |δ_blend| ≤ |δ_tangent|/5.
- **P-series.** Wherever the largest face angle is at most 0.6, each δ_X agrees with its
  energy-weighted series prediction to within 25% of the prediction (or 10⁻⁶ absolute).
- **P-regime (the paper's "strong fields and coarse lattices").** At n = 1 with A ≥ 30, the chord's
  compactness spread |δ_chord| exceeds the discretization error |e_disc|; at A = 3 it does not.
- **Reported, not gated:** blend vs adjoint (the series predict |δ_blend|/|δ_adjoint| ≈ 0.875);
  every E_X/E_cont; the barrier counts.

Each prediction is reported PASS or FAIL; a failure is a finding, not a reason to change the
test.

## Calibration (done before this spec, recorded for honesty)

A scratch probe at n = 1, 2 fixed only two things: the lattice and continuum energies share one
normalisation (E_chord/E_cont = 0.9917 at n = 2, A = 3, no constant to fit), and the chosen A
range gives median face angles from 0.07 to 1.33 and a largest angle of 2.87 at n = 1. No reading
other than the native chord was computed.

## Controls

- **C-native.** At A = 3 the chord energy reproduces the native action of S2a
  (`ricci_su2_pg_force.run` energy) to 10⁻¹².
- **C-arc-small.** At A = 3, n = 2 (face angles ≤ 0.025) all five readings agree with each other to
  10⁻³ relative.

## Not measured

Forces (only energies), other metrics, Monte Carlo (the next step only if this one shows the blend
matters: Bee's "deterministic first").
