# RF2B residual diagnosis and next controlled experiment

Date: 2026-11-17. This is a diagnosis, not a replacement acceptance gate.
The frozen contract remains `RICCI_LATTICE_CORRESPONDENCE_SPEC.md`.
No time integrator, public verb, commit, or deployment is authorized by a
partial operator result.

## What the measurements separate

For Berger `(3,1,1)`, the four levels give these mass-norm errors:

| Level | All edge components | Component in nine homogeneous modes | Orthogonal remainder |
|---|---:|---:|---:|
| 0 | 5.3616286154 | 5.3565853719 | 0.2324963712 |
| 1 | 5.7188141835 | 0.9920859987 | 5.6321044944 |
| 2 | 9.1020221056 | 0.1167316004 | 9.1012735452 |
| 3 | 12.6508267035 | 0.0246692446 | 12.6508026509 |

The nine columns are independent matrix-exponential Frechet derivatives for
all entries of a homogeneous 3-by-3 connection, not just its diagonal.
Projection uses the actual assembled mass matrix. The squared projected
and orthogonal norms sum to the squared full norm, checked numerically.
The orthogonal component is retained in every acceptance test.

This identifies where the discrepancy lives. It does not yet identify a
unique cause, and does not prove that an evolved lattice trajectory fails
to converge. Instantaneous operator consistency and time-evolution
convergence are different mathematical questions.

## A consistency theorem is not the missing strong-norm theorem

Theorem 2 in Christiansen and Halvorsen,
[A simplicial gauge theory](https://arxiv.org/pdf/1006.2059), controls first
variation consistency using the norm `||v|| = ||dv||_L2 + ||v||_L2` and
also reports second-order action consistency. It does not provide our
requested all-link L2-gradient convergence theorem on curved spherical
meshes with the specified tangent metric.

An inference explaining the gap: an estimate of the form

    |R_h(v_h)| <= C h (||d v_h||_L2 + ||v_h||_L2)

combined only with an inverse estimate `||d v_h|| <= C_inv h^-1 ||v_h||`
does not force the L2-dual residual to vanish. It gives at best an O(1)
bound. A small action error or small residual on smooth homogeneous test
directions therefore cannot replace BW05. This is an explanation of why
the cited theorem is insufficient, not a proof of the cause of this run.

## Mesh quality is a second unresolved factor

The original twelve-child refinement is exactly deck-equivariant, closes
every face, and retains the correct manifold. Nevertheless, its measured
worst flat-tetrahedron diameter/inradius is:

| Level | Tetrahedra | Maximum diameter/inradius |
|---|---:|---:|
| 0 | 600 | 4.8989794856 |
| 1 | 7,200 | 6.6160640190 |
| 2 | 86,400 | 10.6701801644 |
| 3 | 1,036,800 | 19.6703467316 |

Positive cell volume is not a uniform shape-regularity bound. These four
measurements do not prove eventual degeneration, but they do prohibit
silently assuming a level-independent stability constant.

## Next experiment: isolate rather than tune

The smallest justified next work is a controlled discretization study, not
a rescaling of time or a fitted correction to the measured residual.

1. Keep the present run and BW05 failure immutable. Freeze new mesh choices
   before measuring the operator on them. Require conformity, the same S3
   cover and 120-fold quotient, exact deck permutations, and a justified
   shape-regularity bound in addition to measured shape statistics.
2. Change only the refinement family first. Keep the connection map,
   action, full mass, time convention, oracle, and all-link error fixed.
   A mesh-only improvement must reproduce on every predeclared metric and
   held-out data, not only Berger.
3. If the strong residual persists, investigate a gauge-compatible stable
   projection or a higher-order consistent variational construction.
   Specify it mathematically before implementation. It may not read the
   target continuum velocity, fit its coefficients to these results, or
   discard transverse degrees of freedom. A changed initial map must be
   disclosed and independently shown to approximate the same connection.
4. A separate weak-consistency-plus-stability route could establish
   evolution convergence even when the present instantaneous gate fails.
   It needs its own justified error bound and predeclared space/time tests,
   including initial transients. It cannot retroactively close BW05.
5. Repeat native derivative, gauge, reversed-edge, full-cover, quadrature,
   mutation, evidence-tampering and regression gates for every candidate.

The success condition remains convergence to an independently evaluated
continuum object. An attractive plot, a fitted clock, a three-mode result,
or a finer mesh by itself is not that result.
