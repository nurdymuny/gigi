# Positive-cone trial basis: frozen model, exact energy form

24 November 2026. Source baseline `9c17cc33`. Written before implementation.
This replaces the invalid unrestricted linear Gram continuation with trial
functions whose coordinates cover only positive-definite Gram matrices.
It does not authorize the registered grid or change its physical parameters.

## Contract and unresolved boundary

Keep the sourced one-grain Berger Hamiltonian, the det(q)>0 sheet, the
registered couplings/backgrounds, gap tolerance 0.003, identification weight
0.9, switch tolerance 0.015 and existing control tolerances unchanged.

The registration does not specify a self-adjoint condition at det(G)=0.
Do not invent one. The trial functions below decay at the cone boundary and
are admissible in the interior energy-form core. They repair invalid
geometry and provide a finite variational calculation. Completeness in the
form domain of the intended physical operator is NOT established. Taking
the closure of a vanishing-boundary core would select a Friedrichs/Dirichlet
realization; equating that to the intended physical sheet or two-sheet
problem requires a separate justification. No automatic physical PASS.

## Coordinates and exact measure

Let u=(log r0,log r1,log r2,a,b,c), with

    L = [[r0,0,0],[a,r1,0],[b,c,r2]],   ri = exp(ui),   G = L L^T.

Every finite representable chart point has G>0 in exact arithmetic and
det(L)>0. Reject overflow, underflow and numerically singular charts; never
clip an eigenvalue, replace a nonpositive determinant or project a bad G.
The inverse is the positive-diagonal Cholesky decomposition.

In the existing G order (00,11,22,01,02,12), let J=dG/du. Directly,

    |det J| = 8 r0^4 r1^3 r2^2,
    rho(G) = (det G)^(-1/2),
    rho(G)|det J| = 8 r0^3 r1^2 r2.

The exact pulled-back cometric is Bu=J^-1 A(G) J^-T. It is positive
definite. A Cholesky parametrization alone is NOT permission to substitute
a Euclidean kinetic term.

Center at the same G0 as before. Let M be the old harmonic normal frame,
M M^T=A(G0), and let S=J0^-1 M. Use u=u0+S z, so Bz(0)=I and the harmonic
frequencies are unchanged. The existing perturbative module supplies only
this frame, not the new Hamiltonian's Taylor coefficients.

The measure is nu(z)=8 |det S| r0^3 r1^2 r2 and
ell=grad_z log nu=S^T(3,2,1,0,0,0). Write psi(G(z))=chi(z)/sqrt(nu(z)).
Then the exact weak energy form is

    T_nm = 1/2 integral (grad chi_n - ell chi_n/2)^T
                           Bz(z) (grad chi_m - ell chi_m/2) dz,
    V_nm = integral chi_n V_src(G(z),+sqrt(det G(z))) chi_m dz.

No Taylor continuation, extra curvature potential, or guessed boundary
term is added. The measure contribution is already inside the covariant
derivatives in this form. The exact potential is evaluated equivalently on
the representative q=L, which preserves the positive sheet and colour
invariance. A constant background energy may be subtracted for conditioning;
it must be disclosed and cannot change gaps.

## Trial functions and finite integration

Use the same normalized harmonic Hermite products chi_n(z), frequencies w,
and total occupation cutoff Mq. Their inverse-measure pullbacks have support
only on G>0. In logarithmic diagonal coordinates they decay faster than any
fixed exponential toward the boundary, so every finite trial vector has
finite norm and energy. This is an admissibility statement, not a proof
that these functions exhaust a specified boundary realization.

Bu and V are finite Laurent polynomials in r0,r1,r2,a,b,c. After substitution
they are finite sums exp(alpha dot z) p(z), not truncated series. Matrix
elements reduce to products of one-dimensional Gaussian moments. Complete
the square in each integral and use Gauss-Hermite quadrature of sufficient
order to integrate the remaining polynomial exactly in exact arithmetic.
Floating-point results remain numerical, not interval certificates. The
derivative expands occupation by at most one; retain that extra shell in
kinetic assembly. Refuse rank loss, nonfinite entries or material asymmetry.

The initial implementation is capped at Mq<=4 (210 trial functions) and
bounded matrix-element caches. Raising that cap is a separately reviewed
resource change, not permission to run the full experiment.

## Strict gates

1. Chart: round trips, positive G and positive Bu for varied interior
   points, including points corresponding to the old negative continuation.
   Bad input refuses rather than clips.
2. Independent coordinate checks: finite-difference J, determinant formula,
   measure and kinetic pullback against the existing A(G). Tolerances at
   most 1e-8 for finite differences, 1e-10 for algebraic comparisons.
3. Exact potential: compare the finite expression to the existing q-space
   V_sourced on L, relative/absolute tolerance 1e-10.
4. Gaussian moments: independent direct quadrature checks for exponential
   and polynomial matrix elements, tolerance 1e-11.
5. Harmonic control: recover the same single- and two-quantum energies to
   the existing 1e-6 tolerance. Test the derivative's extra shell.
6. Same weak-coupling controls: s=1.15,t=3,g^2=0.004 and 0.002,Mq=4.
   Extrapolated coefficients remain tested against -0.5300 and -2.7379
   with the existing absolute tolerance 0.03; weights must exceed 0.95.
   Failure is retained, not repaired by moving the tolerance.
7. Variational assembly: mass identity, nonnegative kinetic matrix within
   roundoff, nested finite Ritz values and finite/asymmetry refusal.
8. Boundary and scientific scope: no claimed physical convergence, no
   automatic release of HOLD_DOMAIN, no replacement of exact charge
   identification by frequency matching near an accidental degeneracy.
9. Mechanism removals: wrong chart, omitted measure derivative, Euclidean
   substituted metric, altered source term, insufficient integration or
   missing derivative shell must each be caught by an assertion.

Write behavioral RED fixtures before each implementation change. Preserve
commands, intermediate failures, GREEN and mutation receipts. Only the
same controls are authorized in this stage. Commit and push source/spec
and control evidence before any larger run; the registered runner remains
held until boundary, charge-identification and convergence requirements
are separately satisfied. No full Rust suite is required for these research
scripts; no engine or HTTP behavior is changed.

## References and attribution of method

The coordinate construction and Gaussian/Ritz machinery are standard, not
claimed as new. The exact metric and potential here are derived from the
repository's frozen model.

- Khare and Vishwakarma, *Cholesky decomposition for symmetric matrices,
  Riemannian geometry, and random matrices*, Proposition 7.2, gives the
  triangular-factor Jacobian underlying the calculation above:
  https://math.iisc.ac.in/~khare/papers/Cholesky.pdf
- Teschl, *Mathematical Methods in Quantum Mechanics*, second edition,
  section 2.3, distinguishes an energy form and its self-adjoint realization:
  https://www.mat.univie.ac.at/~gerald/ftp/book-schroe/schroe2.pdf

Neither reference establishes the unproved boundary identification for this
sourced positive-sheet model; that obligation remains local to this work.

## Amendment after the first same-control check, before refinement

The first Mq=4 interior calculation fixes the negative geometry, but fails
the unchanged competitor coefficient tolerance (error 0.04137 > 0.03).
Its *log-coordinate* single-quantum weight is 0.94753, which is not the
original linear-Gram reference observable. Do not change the reference
observable just because the trial coordinates changed.

Keep y=M^-1(G-G0), the old harmonic reference coordinate, and extend the
physical flat-y state by zero outside G>0. The original single-quantum
amplitude is exactly

    a_j = integral phi_j(y(z)) chi(z) sqrt(|det(dy/dz)|) dz.

This uses the original globally orthonormal Hermite reference functions;
do not renormalize their restrictions to the cone. Sum |a_j|^2 for the
same channel indices as before, with the SAME 0.95 control and 0.9
registered requirements. The logarithmic-basis weights remain separately
reported and are not relabeled as these reference weights.

Integrate these overlaps with independent 8- and 10-node-per-axis
Gauss-Hermite rules, requiring agreement of weights to 1e-6. Failure to
resolve the overlap is a refusal, not permission to increase the weight.
For this check only, allow Mq=2,4,6 at the SAME two weak-coupling controls.
Mq=6 has 924 trial functions; the derivative shell has 1,716. Stream the
operator terms and overlap coordinates; no unbounded matrix cache. This
is basis refinement, not a new coupling/grid or changed Hamiltonian.
The two-point extrapolation threshold stays 0.03. No larger registered run
is released by this amendment. Record all Mq=4 failures rather than replacing
them with the refined results.

The 8/10 overlap check subsequently refused with a maximum weight change
1.35953e-6 (required <=1e-6). Retain that refusal. Refine the overlap rule
to 10/12, keeping the SAME 1e-6 requirement, maximum 12 eigenvectors and
at most 12^6 nodes. Tensor contraction plus chunked coordinate evaluation
is bounded below 1 GiB by the declared array dimensions. This numerical
integration refinement does not change the trial operator or observable.
