# Local geometry and projected-tail audit

24 November 2026. Review target: `f4807710`. This is a bounded replication and
extension of an already inspected diagnostic, not a held-out physical test.
The prior dome verdict and all its inputs remain unchanged.

## Review findings

1. The new overlap code uses `abs` on a projected squared norm. A materially
   negative Schur complement must refuse; it is not a norm to repair by sign.
2. K greater than the reference order silently omits part of the difference.
   Orders must satisfy 0 <= K <= K_ref, with finite positive physical scales.
3. A reference function contained in the origin span has no resolved exterior
   component. The relative exterior error is then undefined, not zero.
4. The comparison with K_ref=48 is finite-reference evidence. No implemented
   infinite-tail certificate supports the prose's claimed all-orders bound.
5. Individual-function L2 errors do not by themselves bound a nearly dependent
   five-function span or the Hamiltonian graph norm. No future K is selected
   for a physical experiment by this audit.
6. The local Hessian gives eigenspaces, not a unique frame inside a degenerate
   eigenspace. Gaussian inverse widths are absolute Hessian eigenvalues only
   in the local harmonic approximation with the stated kinetic metric.
7. Local well energies motivate a global weak-coupling conjecture, not its
   proof. Correct the prediction's monotonicity wording, keep moment ratios
   distinct from localization probabilities, and distinguish large-gauge
   boundary conditions from the finite model's constant-colour constraint.

## Mathematical contract

For the origin basis Gram matrix S and overlap vector b(f), compute
`outside2(f) = <f,f> - b(f)^T S^-1 b(f)` using high-precision Cholesky.
Retain the two terms and the raw difference. The roundoff allowance is
`10^(-(dps-20)) * max(abs(norm2), abs(projection2), 1)`.
Values below minus that allowance refuse. A smaller negative is explicitly
reported and clipped to zero; never take its absolute value. The reference
exterior norm must exceed its allowance or the ratio is unresolved.

For each finite reference, report total and exterior relative L2 differences,
the reference exterior fraction, origin width, monomial, precision, reference
order, and cancellation diagnostics. All scalars must be finite. Preserve
the origin Gram matrix, overlap vectors and raw norms as decimal strings so
the projection and ratios can be replayed independently of polynomial assembly.

At the second vacuum, check the operator on a fluctuation D directly:
`Hess W(I/g)[D] = D + D^T - tr(D) I`. Check the three orthogonal eigenspaces,
their rotated versions at R/g, `Hess V = (Hess W)^2` at critical points,
and a counterexample away from them. Test the path barrier separately.
These local checks do not measure an excitation gap at a new coupling.

## Frozen replication scope

- Existing controls only: (g^2, alpha, beta)=(1,1.7,2),(2,2.3,3).
- Original scalar dome monomial m=1, cutoffs N=12 and 18.
- K=24,30,36,42 compared with finite reference 48, at 80 digits.
- At N=18 repeat all four K values against reference 54, at 80 digits.
- At N=18 repeat the reference-48 batch at 120 digits.
- Eight batches, 32 finite-reference rows in total. No eigenproblem is solved.
- Precision agreement: relative 1e-8 and absolute 1e-12 on reported ratios and
  fractions. Reference-order changes are measured, not forced below a chosen
  threshold. A finite reference never becomes an infinite-tail certificate.
- One CPU thread; maximum 45 minutes and 8 GiB for the worker. Resource stops
  retain partial evidence and are NOT_EVALUATED, not scientific failures.

The proposed couplings 0.15,0.25,0.35 remain unmeasured. A global anisotropic
envelope, its kinetic derivatives, admissibility, and acceptance thresholds
are still prerequisites for that separate physical experiment.

## Focused gates and execution

Demonstrate RED on negative projected tails, reversed orders, unresolved
denominators and missing provenance. GREEN must include an independent
Gaussian-moment oracle, equality at K=K_ref, raw-evidence replay, rotated
Hessian checks and rejection after deliberate mechanism removals. A verifier
must reject incomplete or changed evidence and replace any stale PASS.

Commit and push corrected sources and this protocol before the replication.
Write every completed batch and progress event immediately. Preserve source
snapshots, hashes, commands, test logs, resources and the final receipt.
No full Rust suite is needed: engine code and routes are untouched.
