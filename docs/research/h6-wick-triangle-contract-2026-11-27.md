# The documented Wick-triangle conjecture: frozen contract

27 November 2026. Baseline: 36eb3b4f9e240f2fa3d226dc11f2111dcf3a8c6d.

## The question already in the notes

H6-local-fields.md, Step 6, predicts T_O != 0 and explicitly calls this
a finite free-field computation. The Lowest Note, section 9, repeats it.
Resolve that statement, not a newly substituted conjecture. A vanishing
answer must be published as a refutation of this particular prediction.

## Fixed mathematics

Work at pairwise distinct points of oriented Euclidean R^4. At free order
there are n = dim(g) independent colour copies. Use canonical fields with
Lagrangian sum F_mu_nu^a F_mu_nu^a / 4 and F = dA at this order.
In Feynman gauge, <A_mu^a(x) A_nu^b(y)> = delta_ab delta_mu_nu G(x-y),
G(r) = 1/(4 pi^2 |r|^2). Obtain the F covariance by differentiating G,
including the minus sign from the y derivative. Gauge-dependent gradients
must cancel under antisymmetrization.

Index the six independent components by mu < nu. The observable is the
vacuum-subtracted (equivalently here Wick-ordered) scalar
O = sum_{a,mu,nu} :F_mu_nu^a F_mu_nu^a: = 2 sum_{a,A} :(F_A^a)^2:.
T_O is its connected three-point function, not a stress-tensor correlator,
not a three-point function of individual F's, and not an interacting
Yang--Mills correlator. Hodge star squares to +1 in these conventions.

## Acceptance gates

1. Record red tests before the mathematical engine and proof are added.
2. Independent formulas: differentiate G and compare all 36 entries with
   the induced action on two-forms of the reflection I - 2 rr^T/|r|^2.
3. Prove symbolically that covariance exchanges Hodge eigenspaces; give a
   coordinate-free written argument, not only sampled triangles.
4. Count connected Wick pairings independently, recover the factor
   64 n in T_O = 64 n tr(C12 C23 C31), and test noncollinear rational
   configurations with direct component contractions as a second oracle.
5. Nonzero controls: O's two-point function; an ordinary scalar Wick
   triangle; and O's connected four-point function. Compute coefficients
   with the fixed normalization. These prevent an always-zero engine.
6. Refuse coincident points, incompatible covariance blocks and unsupported
   truncation sizes. Never silently add contact terms or use rounded data.
7. Lean verifies the finite-dimensional trace argument from explicit Hodge
   involution and anticommutation hypotheses. No new axioms, admitted
   steps or claim of a full QFT formalization. Remove a needed hypothesis
   and break signs; the altered proofs must fail.
8. Mechanism-removal tests also reject a missing derivative term, a wrong
   Hodge orientation entry, missing O's factor of two, wrong Wick count,
   and an always-zero contraction function. Fresh output directories only;
   receipts retain failures and hash the final source.
9. Correct only affected current claims in H6, the register and the paper.
   Preserve historical reviews. Verify unrelated paper claims against the
   baseline, rebuild the PDF and inspect the changed pages. No new author
   or attribution. No engine suite, numerical eigensolve or GPU job.

## Interpretation

If T_O = 0, do not write a relative expansion T_O * (1 + O(g^2)) and use it
to conclude the interacting correlator is zero. Write an additive remainder.
The four-point control can supply the already documented alternative
diagnostic, conditional on H6's perturbative matching. A non-Gaussian
composite does not by itself establish that the whole theory is non-free.
H6 and the other six top-level hypotheses remain open.
