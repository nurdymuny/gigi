# What the local geometry and projected tail checks establish

24 November 2026. Review and bounded replication of `613e6e25` and
`f4807710`, with the concurrent sweep-direction update `b6f77431` included.
The corrected source and protocol were pushed before the numerical audit.

**Result: the finite-reference diagnosis reproduces.** The dome can change
very little as a whole while its small contribution outside the origin basis
changes substantially. The local Hessian decomposition also survives direct
geometric tests and rotation around the vacuum orbit. Neither result closes
the previous physical truncation gate; its verdict remains
`UNRESOLVED_TRUNCATION`.

## The result in plain language

Most of the dome function is already expressible by the origin basis. Its
useful addition is the small part that is not. Measuring a change against
the whole function can make that addition look stable when it is not.

At g squared equal to 1 and origin cutoff 18, the K=30 approximation differs
from the finite K=48 reference by only 0.0000064925% in total norm. But that
same change is **13.8019% of the reference's exterior component**. Increasing
K to 36 reduces the exterior change to 0.202099%; increasing it to 42 gives
0.000969802%. The denominator matters because it selects the part of the
function that is actually new to the approximation space.

This is a measured mechanism for sensitivity, not a complete causal
attribution of the earlier residual reversal. That experiment used several
functions and the Hamiltonian; this audit deliberately isolates one
function's L2 projection.

## Exact scope and evidence

- Source: `93b3b56994546f398319e0d5078a852d27e1dbd7`.
- Protocol: [local geometry and projected-tail audit](dome-local-review-protocol-2026-11-24.md).
- Evidence: [audit directory](../../research/dome-local-review-20261124/audit-93b3b569/).
- Eight batches, 32 rows, existing g squared values 1 and 2 only.
- Constant dome multiplier m=1, origin cutoffs 12 and 18.
- K=24,30,36,42; references 48 and 54; arithmetic at 80 and 120 digits.
- Completion: `COMPLETE`; independent invocation of saved-overlap replay:
  `PASS`, eight batches and 32 rows. No projected norm required clipping.
- Supervised elapsed time: 88.594 seconds; peak sampled worker memory:
  283,795,456 bytes, approximately 270.65 MiB. One CPU thread; no GPU or
  Hamiltonian eigensolve was needed for this arbitrary-precision audit.

The [receipt](../../research/dome-local-review-20261124/audit-93b3b569/receipt.json)
contains the source revision, source hashes, environment and complete job
coverage. Each batch preserves decimal-string raw norms, overlap vectors
and the origin Gram matrix. The
[CSV](../../research/dome-local-review-20261124/audit-93b3b569/summary.csv)
collects all reported comparisons. Progress, partial batches, worker output
and resource samples are retained alongside them.

## Finite reference measurements

For a function f, let S be the origin-basis Gram matrix and b its vector of
overlaps with f. The squared exterior norm is

    ||P_perp f||^2 = <f,f> - b^T S^-1 b.

For the finite-reference dome Phi_ref and its truncation Phi_K, report

    total   = ||Phi_ref - Phi_K|| / ||Phi_ref||,
    outside = ||P_perp(Phi_ref - Phi_K)|| / ||P_perp Phi_ref||.

The following values use reference 48 at 80 digits. Displayed numbers are
rounded; the evidence retains the unrounded output.

| g squared | Origin cutoff | Exterior fraction of reference norm squared | Outside K=24 | Outside K=30 | Outside K=36 | Outside K=42 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 12 | 4.15154930e-9 | 0.0647000721 | 7.90745118e-4 | 4.04524742e-6 | 1.15918050e-8 |
| 1 | 18 | 2.88992319e-15 | 1.24054727 | 0.138018943 | 0.00202099407 | 9.69802092e-6 |
| 2 | 12 | 9.01619197e-10 | 0.00361218894 | 2.50073708e-5 | 6.20809349e-8 | 7.99786108e-11 |
| 2 | 18 | 1.26505796e-14 | 0.0296707620 | 3.04535186e-4 | 3.61251360e-6 | 1.05852791e-8 |

An outside ratio greater than one is possible: the difference can be larger
than the reference's very small exterior component. It is not a probability.

At K=30, the total-norm ratios are 6.492535879839193e-8 for g squared 1 and
1.1782375271196494e-9 for g squared 2. These do not depend on the origin
cutoff, whereas the exterior ratios do.

## Reference and precision controls

Raising the finite reference from 48 to 54 at origin cutoff 18 gives:

| g squared | K | Outside with reference 48 | Outside with reference 54 | Relative change |
| --- | --- | --- | --- | --- |
| 1 | 30 | 0.138018943387 | 0.138018957157 | 9.97693e-8 |
| 1 | 42 | 9.69802092482e-6 | 9.71972042678e-6 | 0.00223752 |
| 2 | 30 | 3.04535185692e-4 | 3.04535185098e-4 | 1.94911e-9 |
| 2 | 42 | 1.05852790986e-8 | 1.05974806703e-8 | 0.00115269 |

The 13.8% finding is stable under this reference change. At the much smaller
K=42 tail, reference sensitivity is still measurable: approximately 0.224%
and 0.115% of the respective ratios. Two finite references do not certify
the remaining infinite series.

Repeating the reference-48, cutoff-18 batches at 120 digits produces identical
stored binary64 ratios and fractions to the 80-digit calculation: maximum
reported absolute and relative differences are both zero for each coupling.
This passes the frozen precision rule. It does not claim equality of every
high-precision digit, interval-certified arithmetic or an infinite-tail bound.

## What the local geometry contributes

At q=I/g the tested linear map is

    Hess W[D] = D + D^T - tr(D) I.

It identifies actual fluctuation subspaces, not just a list of eigenvalues:

| Geometric fluctuation | Dimension | Hess W eigenvalue | Hess V eigenvalue |
| --- | --- | --- | --- |
| Breathing, proportional to I | 1 | -1 | 1 |
| Symmetric traceless shape change | 5 | 2 | 4 |
| Antisymmetric orbit tangent | 3 | 0 | 0 |

The direct subspace tests, orthogonality and transport to rotated R/g agree
with finite-difference Hessians. Hess V=(Hess W)^2 is checked at critical
points, with an explicit counterexample away from them. The scalar path
barrier and endpoint W difference are also checked. The normal harmonic
frequencies are 1 and 2; they specify local Gaussian inverse widths, not a
globally normalizable envelope by themselves.

The new sweep tests pass as well: both factorizations represent the same
Hamiltonian, and V(-p;g)=V(p;-g). Choosing the weaker factorization does not
remove the stronger bound from that Hamiltonian.

## Corrections and failure tests

The diagnostic now refuses materially negative projected squared norms,
reversed truncation orders and unresolved exterior denominators. A tiny
negative caused by arithmetic cancellation is disclosed and clipped to
zero, never converted to a positive norm by taking its absolute value.
Provenance includes the origin width, multiplier, reference and precision.

The evidence verifier recomputes the projection from saved raw overlaps,
checks coverage and provenance, rejects changed outputs and cancellation
disclosures, and replaces stale PASS status on failure. A planted Gaussian
moment case supplies an independent analytic oracle.

There are **41 focused tests passing**: 7 diagnostic, 11 local geometry and
factorization, 12 evidence/runner, and 11 unchanged dome-mathematics tests.
Eight deliberate mechanism removals are rejected by assertions. Behavioral
RED logs are retained in [the gate directory](../../research/dome-local-review-20261124/gates/).
All nine source hashes frozen by the older registered experiment remain
unchanged. No full Rust suite or live GIGI route test is claimed.

The research notes now distinguish a local harmonic count from a global
excitation prediction, moment ratios from localization probabilities, and
finite-reference agreement from a tail certificate. The concurrent
[two-grain scope](helike-two-grain-scope-2026-11-24.md) also separates the
oriented Chern-Simons difference from an auxiliary sum, and corrects the
claim that an SO(4)-invariant interaction splits inequivalent parity-partner
representations while preserving exact SO(4) and parity.

## The remaining gate

The next physical test needs a globally admissible normal envelope with its
full kinetic derivatives, joint-span conditioning and Hamiltonian graph-norm
control, and an infinite-tail certificate or another independently controlled
representation. These are concrete requirements, not reasons to discard the
local result. The new weak-coupling points 0.15, 0.25 and 0.35 and the proposed
eighteen-mode spectrum have not been measured by this run.

What advances here is the diagnosis and the local construction rule: measure
the part that changes the approximation space, and obtain the normal
directions and harmonic scales from the model's actual Hessian. The next
experiment can build on those verified pieces without overstating what they
already settle.
