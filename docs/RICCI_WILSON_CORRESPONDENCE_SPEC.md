# RF2A: test the Levi-Civita / Yang-Mills correspondence

Date: 2026-11-17. Status: specification first; gate outcomes are recorded in
RICCI_WILSON_CORRESPONDENCE_RESULTS.md. A passing software gate is NOT a
declaration that every proposed correspondence is true.

## The precise question

Use exactly RF1's manifolds: SU(2), and Gamma\SU(2) with the independently
certified binary icosahedral deck group acting by LEFT translations. Use
the same positive diagonal left-invariant metrics, not a periodic cubic
lattice relabeled as S3. The global frame and all local quantities below
descend to these quotients. Quotient volume multiplies both action and
connection-space metric by 1/|Gamma|, so their gradient is unchanged.

The local manuscript's Definition 2.7 defines Wilson flow as -d_A^* F_A.
GIGI's existing `SYMPLECTIC_FLOW` is Hamiltonian evolution and is NOT that
equation. RF2A evaluates the homogeneous continuum Yang-Mills gradient
independently. It does not claim to test GIGI's existing lattice Wilson
force, lattice weights, or any continuum limit.

Primary convention references:
- Luscher, Properties and uses of the Wilson flow in lattice QCD,
  https://arxiv.org/abs/1006.4518 (gradient flow vs lattice discretization).
- Topping, Lectures on the Ricci Flow, moving orthonormal frame convention,
  https://warwick.ac.uk/fac/sci/maths/people/staff/peter_topping/rfnotes/topping_rf_mar06.pdf
- RF1's stated Milnor-frame/Koszul conventions and references.

## Mapping, fixed BEFORE measurements

Write g=diag(a,b,c) in the fixed frame [e1,e2]=2e3 cyclically. Coefficients
are squared lengths. Let E_i=e_i/sqrt(g_i) be the orthonormal frame, and
identify its axis labels over time by this positive symmetric rescaling,
with no fitted angular rotation. Let J_i v = unit_i cross v, so
[J_i,J_j]=epsilon_ijk J_k and <X,Y>=-trace(XY)/2 gives |J_i|=1.

The Levi-Civita connection on this moving orthonormal frame, evaluated on
the FIXED base frame, is Omega(e_i)=x_i J_i, where cyclically

    Phi_i(g) = x_i = (g_j+g_k-g_i)/sqrt(g_j*g_k).

This is an SO(3) representation of the spin connection algebra. RF2A uses
real matrices/vectors, not a silently chosen quaternion multiplication
convention. No lattice link adapter is claimed. Spatially constant gauge
conjugation rotates the matrices and preserves the comparison norm; local
gauge transformations with spatial dependence are not parameterized here.

The map forgets constant metric scale: Phi(c*g)=Phi(g). Retain g separately.
Deck monodromy is also separate: these connections are NOT the flat deck
representations used by MONODROMY_GROUP_ORDER. A curved S3 does not have
trivial Levi-Civita holonomy just because its deck group is trivial.

## Independent operators

Ricci velocity v comes from RF1's existing native kernel, unnormalized or
volume-normalized as explicitly requested. Differentiate the map:

    p_i = (v_j+v_k-v_i)/sqrt(g_j*g_k)
          - (x_i/2)*(v_j/g_j+v_k/g_k).

Do NOT implement Yang-Mills by returning p. Independently compute curvature

    f_i = x_j*x_k - 2*x_i,
    F(e_j,e_k) = f_i*J_i.

At a fixed background g the energy and connection-space metric are

    E_g(x) = (V_g/2)*sum_i f_i^2/(g_j*g_k),
    ||delta x||_g^2 = V_g*sum_i delta x_i^2/g_i.

The half in E fixes the gradient convention to -d_A^*F_A, as stated in the
manuscript. The factor V cancels. The independently derived negative gradient is

    W_i(g,x) = 2*g_i*f_i/(g_j*g_k)
               - x_k*f_j/g_k - x_j*f_k/g_j.

The common time scale is ONE. No per-case fit, sign flip, or optimized clock
is permitted. Normalized Ricci adds a scale direction annihilated by D Phi.

## Three separate hypotheses

H1, instantaneous: D Phi_g[v_R(g)] = W(g,Phi(g)). Compare all three
components, not correlations. Independent matrix/Koszul and exact algebra
must validate the operators before measuring this residual.

H2, moving background: integrating x'=W(g(t),x), with the independently
evolved Ricci metric g(t) supplied, tracks Phi(g(t)). This is a coupled /
nonautonomous experiment. It DOES NOT eliminate the Ricci metric evolution.

H3, fixed background: integrating x'=W(g0,x) tracks Phi(g(t)) without
being supplied the changing metric. This is the stronger standalone-flow
claim. It must be tested separately, not replaced by H2 if it disagrees.

Predeclared analytical controls:
- Round g=(a,a,a): Phi=(1,1,1), p=W=0 despite unnormalized metric shrinkage.
- Berger g=(3,1,1): Phi=(-1,sqrt(3),sqrt(3)); p=W=(48,-8sqrt(3),-8sqrt(3)).
- At that Berger point, unnormalized H2 minus H3 connection acceleration is
  D_g W[v_R]=(-672,-16sqrt(3),-16sqrt(3)). This predicts a genuine second-order
  separation, not a discretization error. Verify it independently before
  using it as a negative control. An initial nonzero velocity fixes the
  constant clock factor to one, so a new fitted clock cannot erase this test.

## Native diagnostic

    RICCI_WILSON_AUDIT SU2 METRIC (a,b,c) [NORMALIZED];

Requires `gauge`, with embedded/HTTP parity and explicit feature-off refusal.
Same finite metric limits as RF1. No mutation and no bundle lookup. Return
the original metric, Ricci velocity, connection coefficients, curvature
coefficients, mapped Ricci velocity, independently computed Yang-Mills
velocity, residual vector, norms, action density and declared conventions.
The norm uses sum_i residual_i^2/g_i per unit volume. Relative residual is
the residual norm divided by the larger velocity norm (zero if both zero).
No thresholded boolean called `equivalent`, no theorem verdict, no topology
recognition, no inferred lattice correspondence, and no fitted time scale.

## Strict gates

CW00: native grammar/execution behavioral RED, then GREEN; explicit bad-input,
feature-off and real HTTP data checks. Never accept status:ok without a row.
CW01: connection independently derived by Koszul; skew-adjointness, zero
torsion, curvature agreement with RF1's Riemann contraction. Positive and
negative scalar curvature, triaxial metrics and scaled/axis-permuted inputs.
CW02: verify W from the action gradient and connection-space mass matrix for
ARBITRARY homogeneous connections, not only Levi-Civita inputs. Check the
negative energy derivative and correct g weights. At least one noncritical
planted connection; a zero-force round control alone proves nothing.
Differentiate the full nine-coordinate homogeneous action and show its six
off-diagonal derivatives vanish on diagonal connections, so the reduced
model is invariant rather than a projection hiding omitted forces.
CW03: exact symbolic H1 residuals after substituting positive squared lengths,
independent finite-difference derivative convergence, and numerical matrix
oracles over a fixed grid and seeded held-out metrics. Scale factor stays one.
CW04: positive symmetric frame/constant-gauge covariance, scale blindness,
inverse scale of velocities, normalized/unnormalized mapped agreement.
CW05: independent DOP853 integrations of H2 and H3 on the RF1 nonround cases.
Refine tolerances/time differentiation; verify the planted acceleration
separation and report H3 disagreement rather than re-fitting it away.
CW06: same local operators on S3 and the 2I quotient, with the exact group
certificate and native 120-versus-1 results retained. This is a quotient
construction, not triangulation recognition; no topology is inferred from p.
Independently check the quotient action AND L2 mass each scale by 1/120,
then differentiate the quotient action and apply its mass inverse. Repeating
the same local query under two labels is not evidence of this cancellation.
CW07: remove frame rescaling term, structure-bracket term, gradient metric
weights, descent sign and computed residual in isolated source/build caches.
Each must cause a named behavioral test failure, not a compilation failure.
CW08: live private rebuilt engine, source/binary manifests, raw and parsed
responses, retained independent oracle outputs. Verify altered derivatives,
false zero residuals, forged scope, missing cases and stale PASS rejection.
CW09: all-target default and halcyon suites; no commit/deploy before the
implementation gates and high-fidelity evidence pass. Scientific conclusions
H1/H2/H3 are reported separately from software PASS.

## Next gate, not silently included

RF2B needs a manifold-faithful mesh, a metric-dependent discrete Hodge/L2
operator, and oriented SO(3)/SU(2) link adapters checked against GIGI's actual
Wilson action/force. It must show mesh/time convergence of the SAME mapped
operator, not run an unweighted cubic torus and call it S3. No lattice
equivalence or general Ricci/surgery result follows from RF2A alone.
