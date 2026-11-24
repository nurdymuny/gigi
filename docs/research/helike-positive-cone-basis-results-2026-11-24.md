# Positive-cone basis: valid geometry and the same control test

24 November 2026. **COMPUTED**, not a physical spectrum certificate.

## Result in plain English

The old basis allowed the calculation to wander into matrices that could not
describe a real configuration. In that region even the kinetic energy could
have the wrong sign. The new basis changes the coordinates, not the physical
energy formula: every chart point describes a positive-definite Gram matrix.
It includes the coordinate-dependent kinetic energy and volume measure.

On the same two weak-coupling controls, the refined basis passes the existing
coefficient and identification-weight requirements. This is the useful result:
the control agreement survives removing the unphysical continuation.

It is **not** permission to run the registered grid. The boundary condition at
singular Gram matrices is still unspecified, exact charge-sector identification
is not established, and one gap changes by 0.003456 under the tested refinement,
above the unchanged 0.003 convergence requirement.

## What was fixed, and what stayed fixed

The construction is specified in the
[pre-implementation spec and recorded refinement amendments](helike-positive-cone-basis-spec-2026-11-24.md).
The model, source terms, positive determinant sheet, couplings, harmonic frame,
original harmonic-reference projectors and acceptance thresholds are unchanged.
No engine endpoint or GQL behavior is changed.

Write the Gram matrix as

\[
G=LL^T,\qquad
L=\begin{pmatrix}e^{u_0}&0&0\\a&e^{u_1}&0\\b&c&e^{u_2}\end{pmatrix}.
\]

This is an interior coordinate chart, not eigenvalue clipping. In exact
arithmetic every finite chart point is positive definite. Numerical chart
queries outside the finite, well-conditioned range refuse instead of projecting
back into the cone.

With the original normal frame \(M\), set \(u=u_0+Sz\), where
\(S=J_0^{-1}M\) and \(J=dG/du\). The exact volume and kinetic transformations are

\[
\nu(z)=8|\det S|r_0^3r_1^2r_2,\qquad
B_z=S^{-1}J^{-1}A(G)J^{-T}S^{-T},\qquad
\ell=\nabla_z\log\nu=S^T(3,2,1,0,0,0)^T.
\]

The trial function is \(\psi=\chi/\sqrt\nu\), giving the exact energy form

\[
T_{nm}=\frac12\int
(\nabla\chi_n-\ell\chi_n/2)^T B_z
(\nabla\chi_m-\ell\chi_m/2)\,dz,
\qquad
V_{nm}=\int\chi_n V_{\rm src}(G,+\sqrt{\det G})\chi_m\,dz.
\]

The potential and pulled-back metric are finite Laurent polynomials in the
Cholesky entries. Matrix integration uses completed-square Gaussian moments,
not a Taylor continuation into invalid geometry. Quadrature order integrates
the remaining polynomial exactly in exact arithmetic; floating-point results
are not interval bounds. The kinetic derivative retains the extra occupation
shell.

## Direct geometric witness

Use \(G_0=\operatorname{diag}(2,3,4)\),
\(M=\operatorname{chol}(A(G_0))\), and the same coordinate
\(z=-2M^{-1}(2,3,4,0,0,0)^T\).

| Measurement at that coordinate | Old linear continuation | New chart |
| --- | --- | --- |
| Gram eigenvalues | -4, -3, -2 | 0.2706705665, 0.4060058497, 0.5413411329 |
| Pulled-back kinetic eigenvalues | All -1 | 1, 1, 1, 7.3890560989, 7.3890560989, 7.3890560989 |

The new Gram matrix is \(e^{-2}G_0\). The coordinate labels agree in this
comparison; the physical points deliberately do not. The new chart cannot
represent the old negative Gram matrix. This witness and the algebraic
factorization establish the specific domain repair; sampling alone would not.
The full recorded numbers are in
[domain-witness.json](../../research/positive-cone-20261124/domain-witness.json).

## Same controls, unchanged thresholds

The source/spec checkpoint **ad4536e4cc8c387fdc18bafabe440215f9f529c4** was
committed and pushed before the refined control run. Its receipt records
`source_dirty: false` for the declared sources and retains their snapshots.

Settings remain \(s=1.15,t=3,g^2\in\{0.004,0.002\}\). The refined total
occupation cutoff is \(M_q=6\): 924 trial functions and a 1,716-dimensional
derivative shell. This is the same control experiment at a finer basis, not
the larger registered grid.

| g squared | Helike gap | Competitor gap | Helike reference weight | Competitor reference weight |
| --- | --- | --- | --- | --- |
| 0.004 | 1.0881506933 | 1.0962441490 | 0.9885333264 | 0.9808081919 |
| 0.002 | 1.0890870744 | 1.1013023580 | 0.9943433990 | 0.9905908651 |

These weights use the **original linear-Gram harmonic reference**, not the
coefficients in the new logarithmic basis. The observable is pulled back with
its volume factor; restricted reference functions are not renormalized. The
10/12-node-per-axis overlap checks change weights by at most 2.7981e-7 and
3.1636e-8 respectively, below the unchanged 1e-6 numerical requirement.
That comparison is a numerical resolution check, not a rigorous error enclosure.

For each channel, let \(c(g^2)=(\mathrm{gap}(g^2)-\omega)/g^2\), with the same
harmonic reference frequency \(\omega\). The existing extrapolation is
\(2c(0.002)-c(0.004)\).

| Channel | Fixed target | Computed extrapolation | Absolute error | Required error |
| --- | --- | --- | --- | --- |
| Helike | -0.5300 | -0.5308701642 | 0.0008701642 | at most 0.03 |
| Competitor | -2.7379 | -2.7511293048 | 0.0132293048 | at most 0.03 |

Both coefficient tests pass, and all four original-reference weights exceed
the unchanged 0.95 control requirement. The receipt says `PASS_CONTROLS`,
not physical certification; `certified` remains false.

The mass-matrix error is 7.11e-15. The smallest kinetic-matrix eigenvalues are
1.09048690 and 1.09262765. Pre-symmetrization asymmetry is at most 1.15e-13.
The disclosed background energy shifts are -47.52734375 and -95.05468750;
they cancel in every reported gap. Runtime was 7.59 and 7.33 seconds on the
recorded machine; this is not a performance comparison.

## Refinement is not yet convergence

The earlier \(M_q=4\) run is retained, including its failed competitor
coefficient error of 0.04136704. Increasing the basis, rather than loosening
the threshold, produces the control pass above.

| g squared | Absolute Helike gap change, Mq 4 to 6 | Absolute competitor gap change, Mq 4 to 6 |
| --- | --- | --- |
| 0.004 | 0.0009356071 | **0.0034559546** |
| 0.002 | 0.0002434641 | 0.0009185850 |

The bold entry exceeds the registered 0.003 gap requirement. A passing
extrapolated coefficient test cannot replace this separate convergence test.
Nested Ritz eigenvalues decrease under refinement, as tested, but differences
of two Ritz eigenvalues are not certified upper bounds on exact gaps.

## Strict gate evidence

All evidence is in [positive-cone-20261124](../../research/positive-cone-20261124/).

| Check | Recorded outcome |
| --- | --- |
| Original domain witnesses | Two behavioral RED failures on the old continuation |
| Full contract RED | Retained; includes missing API errors as well as behavioral failures |
| New focused tests | 18 pass: geometry, moments, kinetic integral, fixed controls, saved-state replay |
| Existing refusal/evidence regression tests | 24 pass |
| Existing perturbative controls | 4 pass |
| Deliberate mechanism removals | 8 of 8 caught by assertions, zero incidental test errors |
| Saved eigenpairs | Finite, orthonormal, matrix residual checked; recorded gaps reproduced |
| Saved identification weights | Recomputed from stored eigenvectors in the original reference |
| Corrupted matrix sensitivity | A planted matrix alteration fails the eigenpair residual condition |

The eight removals cover positive diagonals, the volume measure, the nonflat
metric, the source-term sign, Gaussian completion, the derivative shell, the
reference-volume factor, and the kinetic measure derivative. The last has an
independent direct kinetic-integral test: gaps alone could miss a common
energy shift. Logs retain the initial integration refusal and all intermediate
control failures. No full Rust test suite or live server test was run; no Rust
or HTTP implementation changed.

The final saved matrices have binary SHA-256 hashes. Source snapshots use the
declared universal-newline text hash policy for cross-platform replay. Earlier
development receipts have hashes and matrices but not complete snapshots;
they are not presented as clean committed-source runs.

## What remains before a larger run

1. Specify and justify the intended boundary realization at det(G)=0. The
   present interior trial functions vanish toward the boundary. Closing that
   form core selects a particular realization; it cannot silently stand in
   for an unspecified positive-sheet or two-sheet physical operator.
2. Establish charge-sector identification beyond these well-separated
   harmonic-reference controls. High overlap is useful evidence, not an exact
   symmetry certificate.
3. Resolve basis convergence at the unchanged 0.003 requirement. The current
   4-to-6 comparison is not enough, and its one failure remains a failure.

The original registered runner stays on `HOLD_DOMAIN`. No bigger grid,
crossing claim, continuum claim, or Yang-Mills mass-gap conclusion is released.
The next question is specific: **which boundary condition is inherited from
the underlying q-space model, and is this trial family dense in that energy
domain?** That must be answered before interpreting refinement as convergence
to the intended physical spectrum.
