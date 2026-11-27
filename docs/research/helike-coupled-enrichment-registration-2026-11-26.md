# One coupled residual enrichment: frozen experiment

Registered before the implementation and enriched solve. This changes the trial space explicitly; it changes
neither the Hamiltonian nor an existing acceptance threshold. The old grid remains unresolved and held.

## Input and mathematical construction

Use the saved support export at s=1.18, t=3, g^2=0.03, Mq=12, probe cutoff 14. Its LF SHA256 is
`15ed4b72a41ab4f1125fbe3a3bb3b6f35775c0ef077f1fc3f8b8f1a8b3969833`. The parent aggregate digest is
`b790b53f6186b6d3b1767c91cc1f087486d2db98c2bd291fce14db40524210a6`.

The columns R are the four saved residuals (ground, both Helike partners, competitor), with all V_12 entries
set exactly to zero. Reconstruct both unchanged exact-moment operators and verify the saved eigenstates and
residuals against fresh actions before using them. Require norm-one states within 1e-8, old finite-space
residuals within 1e-10*max(1,abs(E)), and saved ambient residual agreement within that same scaled tolerance.

Form G=R^T R using the existing faer symmetric eigensolver. Require exactly four independent columns:
lambda_min > 1e-12*lambda_max > 0. Refuse rank deficiency; do not silently drop or replace a direction.
Set U=R V diag(lambda^-1/2). Entries in V_12 remain exactly zero. Require max|U^T U-I| <= 1e-10.
These are diagnostic construction tolerances, not changes to the old gap or identification gates.

The isometric lift J maps (x,y) to the occupation-key embedding of x plus U y. The new space is
W=V_12 plus span(U): dimension 18,568, inside V_14 of dimension 38,760. Every old basis function is retained.
No 16-dimensional selected-eigenvector replacement is permitted.

Compute C=P_12 H_14 U and D=U^T H_14 U. The new operator is

    H_W (x,y) = (H_12 x + C y, C^T x + D y).

Require D's antisymmetric discrepancy <= 1e-10*max(1,max|D_ij|), then symmetrize only that rounding-sized
discrepancy. Verify the full block action against J^T H_14 J on three fixed deterministic test vectors,
to norm tolerance 1e-9*max(1,norm(direct action)), before the solve. This check includes both C and C^T.

## One solve, no automatic escalation

Use the existing block Lanczos implementation: 12 levels, block size 4, scaled residual tolerance 1e-10,
maximum Krylov dimension 1600, and the same initial harmonic coordinate vectors. Keep eight Rayon threads
and a 900-second process limit. No Mq=14 eigenproblem, full grid, second enrichment, or expanded-shell probe.

Require every returned W residual to meet the original solver tolerance. Check that the first 12 ordered
energies do not exceed the saved V_12 energies by more than 1e-8 (nested variational sanity check; gaps need
not be monotone). Expand the vectors into V_14 for the unchanged harmonic-reference identification.
Reference integration must still resolve at 1e-6. The identification's recorded Lanczos residuals refer to W,
not to the ambient V_14 operator; label that scope explicitly and report ambient residuals separately.

For the new ground, reference-selected Helike pair and competitor, independently apply H_14 and record:

- norm of J^T(H_14-E)psi (inside W);
- norm of (I-JJ^T)(H_14-E)psi (unrepresented within V_14);
- total ambient residual norm, and the orthogonal split identity;
- state support and overlap with the old ground, competitor and full two-dimensional Helike subspace.

Compare both gap changes with the unchanged 0.003 step threshold. Report the unchanged grid weight rule 0.9
and necessary export condition 0.95 separately. Report residual ratios rather than inventing a new physical
pass threshold. A small energy step is not by itself success. This is one new basis experiment, not a
retroactive amendment or pass of the old grid. The domain gate is NOT RUN, and certification/release remain false.

The probe ends at shell 14; its residual is not the full-operator residual or a certified gap error bar. The
degree-10 rigorous cutoff cap is unchanged. The experiment can establish whether this first coupled enrichment
improves the measured obstruction, but cannot establish continuum convergence or close the grid gate.

## Focused gated TDD

Before the expensive run:

1. Planted lifts preserve norms, retain all old coordinates and reproduce an analytic coupled matrix exactly.
2. Duplicate positions, nonfinite data, non-orthogonal directions, old-space contamination and deficient rank
   refuse. Orthogonal rotation of residual columns preserves the projector, not necessarily individual columns.
3. Both weak controls (s=1.15,t=3,g^2=0.002 and 0.004) match the projected Mq=4 operator using an Mq=2 base plus
   four test directions. These are small operator tests, not extra grid runs.
4. Removing C, C^T, normalization or ambient projection must break the focused tests. Capture red/green logs.
5. The independent evidence checker must reject bad norms, altered settings, mismatched dimensions, incorrect
   residual splits, a changed parent, or any attempted grid/certificate pass. Output-directory reuse refuses.

Save commands, build and execution logs, source/binary/input hashes, basis directions, new coefficients,
residuals and all measured outcomes. Stop after this one solve, even if the result suggests another iteration.
