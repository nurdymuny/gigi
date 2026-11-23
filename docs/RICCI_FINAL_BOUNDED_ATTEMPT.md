# RF2G: research synthesis and one bounded final attempt

2026-11-17. Pre-run status: NOT RUN. No production changes or deployment.

## What the evidence actually establishes

1. RF2A supports a specific homogeneous continuum Ricci/Yang-Mills mapping,
   including symbolic instantaneous agreement and a moving-background test.
   Fixed-background trajectory equivalence was rejected. This is not general
   Ricci flow or a lattice convergence theorem.
2. RF2B/C show that energy convergence and shape-regular refinement did not
   repair full lattice flow. Finest Berger full error remained about 25.51%.
3. RF2D validates differentiation of the old action, Stokes/incidence, and
   quadrature. Its exact residual split puts the dominant discrepancy in
   action/load consistency. An infinitesimal Abelian control also fails:
   finest full error about 26.13%, despite nearly correct energy/Rayleigh value.
4. Tiny high-frequency components dominate the coarse Abelian generator
   residual. This diagnoses amplification; it does not prove a valid cure.
   Signed SU(2) and double-cover tests found no local sign/branch loss in
   these data. Mass whitening removes apparent Euclidean nonnormality; HHE
   bounds do not themselves repair spatial consistency.
5. RF2F's compatible polynomial/BDM1 recovery has verified energy derivatives,
   exact flux preservation and stable local fits, but finest flow error is
   80.91%. Energy 1.99999058 versus 2 is not a successful flow result.
6. RF2E's exploratory regularized energy recorded a 1.4291% Abelian score,
   but was excluded after the variational-first direction. It has no full
   verified acceptance, Berger extension or authorized release claim. It is
   not silently substituted for a solution here.

## External research: agreement and boundaries

- Arnold, Falk, Guzman and Tsogtgerel (2014),
  [On the consistency of the combinatorial codifferential](https://sites.math.rutgers.edu/~falk/papers/dstar.pdf),
  supplies counterexamples and a numerical inconsistency example for two-forms
  in three dimensions even with regular subdivision. This supports separating
  exterior-derivative compatibility from strong adjoint consistency. Their
  result is not a theorem about our curved meshes.
- Arnold and Chen (2017),
  [FEEC for parabolic problems](https://www.numdam.org/item/10.1051/m2an/2016013.pdf),
  analyzes mixed Galerkin Hodge heat discretizations. Solution convergence is
  a different assertion from our instantaneous generator test on canonical
  exact-edge inputs. Switching to positive-time trajectories or projected
  initial data would change the question, not close BW05.
- Arnold and Brezzi (1985),
  [Mixed and nonconforming finite element methods](https://www.numdam.org/item/M2AN_1985__19_1_7_0.pdf),
  describes multiplier enforcement and variable elimination in mixed methods.
  This supports using an explicit constrained variational problem and its
  reduced energy. It does not certify the candidate below.

These are primary sources, not claims that published convergence theorems
apply to an implementation with different spaces, inputs or norms.

## Final candidate: remove the fitted target, not the measured data

Use ONLY the already verified RF2F BDM1 mass H (aitch), original edge mass M
(em), original incidence D (dee), and original face fluxes f=D a. C sums the
three moments on a face. The same J distributes f/3 and Z parametrizes the
two zero-sum moments, so C J=I and C Z=0.

Define the physical reduced energy by an actual minimum:

    E(a) = min_{C c=D a} 1/2 c^T H c.

With S=Z^T H Z and T=Z^T H J,

    u=S^-1 T f, c=J f-Z u=R f,
    E=1/2 c^T H c, gradient_a=D^T R^T H c,
    velocity=-M^-1 gradient_a.

This is the Schur-complement reduced energy. Its auxiliary stationarity is
Z^T H c=0, hence the exact envelope derivative is D^T J^T H c. Evaluate the
full transpose expression as well; agreement is a numerical check, not an
excuse for dropping derivatives in the previous nonstationary fit.

Unlike RF2F, there is no patch target Q, polynomial fit, ring radius, penalty
parameter, spectral filter or oracle field in this energy. Every measured
face flux remains exact. Positive energy and compatibility follow from the
finite-dimensional constrained problem; strong consistency does NOT follow.
This tests whether the previous nonstationary fitting target was the dominant
additional problem. Failure is a valid stopping outcome, not a reason to tune.

## Frozen gates and spending limit

Keep mapping, original mass, clock1, meshes n=1,2,3,5 and BW05 unchanged:
both finest rates >=0.5; final full relative error <5%; numerical differences
<spatial error/10. No change to existing evidence/source files.

One candidate only. Reuse hash-verified matrices: do not regenerate meshes,
rebuild Rust, run broad suites, or search parameters. First prove independent
dense KKT agreement, actual reduced-energy derivative, minimum energy,
constraints and null input; behavioral RED before implementation. Remove
the auxiliary minimization and require failure. Reject forged evidence and
clear stale verification before refusal.

Then evaluate all four Abelian levels at q4/q6, with original full error and
action/load/mass split. Compare the new code with the existing general RF2F
recovery at Q=0 as a separate algebraic implementation. Use residual-correction
solve estimates; no success flag alone. A hard 180-second experiment timeout
is a computational limit, not an automatic mathematical failure.

STOP after this one Abelian experiment and focused verification. On failure
or unresolved evidence, park the repair. Even on an Abelian pass, report it
as Abelian-only and do not start an open-ended nonlinear Berger implementation
in this turn. No gate close, commit, deployment or chapter claim follows.
