# Ricci flow on homogeneous three-manifolds: RF1

Gate status is recorded in `RICCI_FLOW_SU2_RESULTS.md`. This is a Ricci-flow feature,
not graph smoothing, a Wilson-action proxy, or a general topology classifier.

## Purpose and boundary

Give GIGI an independent, actual Ricci-flow trajectory against which the
Davis-Wilson/Ricci dictionary can eventually be tested. The existing
`pc_001_formal_equivalence.py` computes action and curvature proxies on a
periodic gauge lattice; it does not independently solve the Ricci equation.
No correspondence is established merely because two observables decrease.

RF1 solves all diagonal left-invariant SU(2) metrics in a bounded numerical
regime, not just round or Berger fixtures. SU(2) is the smooth three-sphere.
The input is an explicit analytic manifold and metric, not a point cloud or
a guessed metric extracted from records. The same local solution descends to
Gamma\SU(2) for a finite subgroup acting by LEFT translations. Such actions
are free and preserve every left-invariant metric. Right multiplication is
not substituted for left multiplication in anisotropic examples.

The demonstration pairs this flow with GIGI's generator-closure query and an
independent exact algebraic certificate for the supplied binary icosahedral
group. The native closure remains numerical; the exact certificate belongs to
the evidence harness. Neither result recognizes an arbitrary input manifold.
The finite quotient groups are DECK groups, not Levi-Civita holonomy groups.
A curved simply connected three-sphere has nontrivial Levi-Civita holonomy;
the identity deck-group control must never be called flat parallel transport.

## Mathematics and conventions

Use a fixed left-invariant frame with [e1,e2]=2 e3 cyclically, dual forms
theta_i, and g=A theta_1^2+B theta_2^2+C theta_3^2, A,B,C>0. Simultaneously
reversing all bracket signs gives the same curvature. A,B,C are squared
lengths, not lengths. Unit diagonal gives sectional curvature 1, scalar
curvature 6 and cover volume 2*pi^2. Time has units of squared length.

In the fixed frame, cyclically:

    Ric_11 = 2*(A*A-(B-C)*(B-C))/(B*C)
    lambda_1 = Ric_11/A
    R = lambda_1+lambda_2+lambda_3
    K_12 = (lambda_1+lambda_2-lambda_3)/2
    V_cover = 2*pi^2*sqrt(A*B*C)

Unnormalized Hamilton flow: dg/dt=-2 Ric, hence

    A' = -4*(A*A-(B-C)*(B-C))/(B*C)                 (cyclic)

Volume-normalized flow: dg/dt=-2 Ric+(2/3)R*g. Homogeneity makes scalar
curvature equal its spatial average. No numerical volume projection is used.

Checks independent of trajectory implementation:

- Round: A=B=C=a0-4t; extinction time a0/4. Normalized round is stationary.
- Berger: B=C=b, A'= -4*A*A/b^2, b'=-8+4*A/b.
- d(log V)/dt=-R unnormalized, 0 normalized.
- R'=2*sum(lambda_i^2), or 2*sum(lambda_i^2)-(2/3)*R^2 normalized.
- Scaling: g(t;c*g0)=c*g(t/c;g0). Frame-axis permutations commute with flow.
- Normalized positive metrics approach the round metric with diagonal
  (A0*B0*C0)^(1/3); finite runs only test their reported time interval.

References: Landsman, *Ricci Flow: A First Introduction*, section 6.1,
https://www.math.ru.nl/~landsman/RicciFlow2019.pdf (bracket 1 there; multiply
curvature by 4 for our bracket 2); Calegari, *Ricci Flow*, section 1.2.4,
https://math.uchicago.edu/~dannyc/courses/ricci_2019/ricci_flow.pdf (Berger
normalization agrees directly). The independent oracle derives curvature
using the Koszul connection and Riemann contraction, not a copy of our formula.

## Native contract

    RICCI_FLOW SU2 METRIC (A,B,C) TIME t STEPS n
      [NORMALIZED] [TOLERANCE epsilon];

`gauge` feature required. Same kernel in embedded and streaming GQL. No
bundle-name pre-resolution; missing feature must refuse, never status:ok.
Only this manifold family is accepted. Unknown/repeated options refuse.

- Initial coefficients: finite [1e-6,1e6], max/min <=1e4.
- Time: finite, >0, <=100*min(initial coefficients).
- STEPS: integer 1..4096; output exactly n+1 rows, including initial state.
- TOLERANCE: finite [1e-12,1e-3], default 1e-8.
- Fixed classical RK4 on n and 2n steps, common coarse time points. Return
  the 2n-step trajectory. Work is bounded; no hidden adaptive retry.
- Internally scale metric and time by max(initial coefficients).
- Every RK stage must stay finite and above 1e-10 times the smallest initial
  coefficient; otherwise refuse the WHOLE calculation and recommend a shorter
  interval/more steps. Do not step through a singularity or invent surgery.
- Error diagnostic: max_i |g_2n[i]-g_n[i]|/(15*max(g0)), at each output time.
  Refuse if the maximum exceeds epsilon. This is a Richardson estimate, not
  an interval enclosure, proof of convergence, or guaranteed error bound.

Rows carry t,A,B,C, fixed-frame Ricci components, Ricci eigenvalues, scalar and
sectional curvatures, cover volume, anisotropy=max/min, error estimates,
normalization flag, equation, method, bracket convention and scope. Every row
states `surgery_performed=false`, `topology_inferred=false` and
`error_bound_certified=false`. No quotient volume or fundamental-group order
is guessed by this kernel.

## Strict gates (all required for RF1 closure)

RF00: behavioral RED native grammar/execution, bad contracts, feature-off
refusal. No compile error counts as a red mathematical test.

RF01: round metric/time/curvature/volume against closed form; normalized
stationarity; refusal at and beyond round extinction.

RF02: triaxial Ricci components against an independent Koszul/Riemann tensor
oracle over a deterministic varied grid; Berger sectional-curvature oracle.

RF03: normalized volume and scalar evolution, unnormalized scalar evolution,
axis permutation and scaling. No assertions accepting alternative outcomes.

RF04: three step resolutions demonstrate fourth-order convergence away from
round fixed/exact fixtures. Independently integrated Koszul-based trajectories
must agree; label the reference integrator and tolerances.

RF05: anisotropic and initially negative-scalar-curvature examples. Same
general-purpose code; no fixture-ID branches or precomputed trajectories.

RF06: positivity, overflow/nonfinite, low-step and excessive error refusals;
never a partial successful trajectory. Explicit bounded-work checks.

RF07: live rebuilt private server; actual returned rows/numbers; exact query,
source manifest, binary hash and response retained. Native 120 vs 1 closure
alongside equal local Ricci flow is a mandatory counterexample to
"Ricci extinction alone recognizes S3". Exact group oracle stays outside engine.

RF08: deliberately remove flow update, Ricci cross terms, normalization,
positivity guard and error gate in isolated source/build caches. Each must
cause its named behavioral test to fail, not merely compilation to fail.

RF09: evidence verifier rejects wrong times, coefficients, curvature, missing
cases, mismatched binary/source, and forged PASS; removes stale PASS before
reading/parsing evidence. Coherent raw-and-parsed tampering must be rejected.

RF10: all-target default and halcyon regression suites; no commit/deployment
before these pass on the final source plus RF00..RF09. Existing monodromy
overall gate is still open after its build-isolation repair; do not relabel
its live-only recovery receipt as a complete release gate.

## What remains outside RF1

General triangulated-manifold ingestion and recognition; nonhomogeneous
3D Ricci PDE; neckpinch resolution; surgery; Perelman entropy calculations;
certified numerics; inference of pi_1 from a single representation.

The NEXT correspondence gate requires a declared map from the SAME smooth
metric to lattice geometry and connection, a fixed time/observable mapping,
held-out metrics, mesh/time refinement, round and nonround controls, and
residuals against the independently solved Ricci equation. Fitting separate
time scales to every trajectory or comparing two outputs of the Wilson
calculation is not evidence of equivalence. A torsion-free metric connection
and gauge-invariant comparison must be specified before implementing it.
The map must also retain metric scale or explicitly restrict to a fixed-scale
normalized comparison: constant rescaling leaves the Levi-Civita connection
unchanged, but changes lengths, volume and the unnormalized extinction time.
Matching the mapped vector fields, with a fixed time conversion and held-out
metrics, is the relevant test; matching two decreasing scalar summaries is not.

RF1 makes this missing reference computation possible; it does not by itself
establish the Davis-Wilson/Ricci equivalence or reproduce Perelman's theorem.
