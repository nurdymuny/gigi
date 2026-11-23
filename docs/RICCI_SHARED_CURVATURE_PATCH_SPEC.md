# Smallest shared-curvature patch construction

2026-11-17. Spec before construction. Original Abelian six-cell patch only;
no global repair, new mesh, Berger run, broad suite, commit or deployment.

## Meaning of preservation

The input mapping, signed edge observations, original round metric, original
Whitney tangent mass, clock and BW05 thresholds remain unchanged. Preserve
ALL patch edge integrals and ALL oriented face fluxes, including exterior
faces. Curvature is the derivative of a reconstructed connection and remains
closed. The pointwise interior Whitney approximation is allowed to change;
holding that approximation fixed would make the known trace mismatch fixed.

This is a new reconstruction of the same observations, not an alternative
input projection or an averaged face field. The metric-dual field is ALWAYS
computed from the reconstructed curvature and original metric.

## One shared field, minimal declared family

Use the same cells0,2,3,456,458,459 and their retained physical cover images.
Map them to the original flag's gnomonic chart. Translate by the mean of unique
patch vertices and scale isotropically by their maximum distance from that
mean. Geometry alone chooses this normalization; no observed answer enters.

Use one polynomial two-form on the ENTIRE patch, not six separate fields.
In three coordinates it is represented by a three-component flux polynomial
f (eff). Closure is the coefficient identity div f = 0 (divergence eff equals
zero). Its metric-dual one-form H (aitch) is determined by the ORIGINAL metric.
One smooth common f and metric supply common tangential traces of H at all
six INTERNAL shared faces, not just at fitted quadrature points.

Examine degrees0,1,2 ONLY, with respectively3,11,26 closed coefficients.
Choose the smallest degree whose face-flux map represents the ENTIRE discrete
closed-flux space on this patch, not merely this one observed vector. Geometry
and matrix rank determine the choice, before fitting data. If degree2 is
insufficient, REFUSE and stop. No automatic higher degree or radius search.
"Smallest" refers only to this declared common-polynomial family.

Build the oriented patch incidence matrices directly from original edge IDs,
signs and retained cover vertices. Check vertex/edge/face consistency and
boundary-of-boundary zero. Verify the expected contractible edge-star ranks;
do not assume exactness simply because a patch looks small.

## Exact data constraints and physical objective

Let a (ay) be the original edge-integral vector, D (dee) the face-edge
incidence, C (see) the divergence-free coefficient basis, Q (cue) the map
from those coefficients to all face fluxes, and M (em) their physical L2
energy matrix over the six cells, using original radial geometry and q6.

```text
b = D a
c = argmin_c (1/2) c^T M c, subject to Q c = b.
```

b is bee; c is see; superscript T is transpose. This selects a unique physical
curvature inside the declared finite family. It is not interpolation to a
known analytic control. All input a are supported if they induce closed
fluxes; the solver also rejects independently supplied nonclosed/incompatible
face data. Redundant face equations may be eliminated for the linear solve,
but ALL original equations must pass afterward.

Use rank-revealing SVD/QR at relative1e-12; cap numerical conditioning at1e12.
No diagonal regularization, soft constraints, fitted clock, target force,
known smooth curvature, or analytic solution in the construction. Absolute
data, closure, trace and derivative acceptance remains1e-9. BW05 remains
rates>=0.5, full relative error<5%, numerical errors<spatial/10 and OPEN.

## Connection and complete derivative

Recover a polynomial potential A0 (ay zero) by the polynomial homotopy:
each homogeneous flux term of degree r (are) contributes
f_r(y) cross y divided by (r+2). Its curl must equal f coefficient-by-coefficient.
y is why. The primary polynomial de Rham/Koszul identity supports this step.

The difference between the original edge data and A0's edge integrals must
be a discrete vertex gradient. Recover vertex values psi (sigh), with one
fixed zero anchor, and define A_rec = A0 + d psi (ay rec equals ay zero plus
dee sigh), using the continuous piecewise-linear nodal psi on the patch.
This preserves EVERY signed edge integral while leaving curl A_rec=f.
No edge correction is allowed unless its zero curl is independently verified.

Since the reconstruction is a geometry-only linear map c=R a (are ay), its
physical energy derivative is R^T M c. Test the complete derivative on every
patch edge using central differences; do not treat reconstructed coefficients
as constant. Check symmetry, pure-gauge invariance and minimum energy along
the remaining constraint-null directions. Do NOT compute or claim full flow.

## Boundary honesty

All EXTERIOR integrated fluxes and edge observations are preserved. Exterior
pointwise traces need not equal the old outside Whitney field. Measure and
report that mismatch. This candidate CANNOT be silently spliced into the
unchanged outside field: doing so may introduce distributional curvature
closure/interface defects. Global extension and patch-energy overlap are
separate, unattempted work. Passing this gate means a compatible reconstruction
ON this bounded patch, not a drop-in globally compatible repair.

## Fixed focused gates

PC00: failing algebra fixtures before implementation. Divergence-free basis,
homotopy curl identity, constrained minimum, exact full flux constraints,
nonclosed-data refusal and complete derivative; no oracle used for fitting.

PC01: geometry-only rank audit for0/1/2. Generic closed data must be
representable at the chosen degree; a lucky fit of the real control is not
enough. The previous incompatible original field remains the negative control.

PC02: preserve every original edge integral and every face flux to1e-9,
with independent Stokes checks. Show closure coefficient-by-coefficient and
the potential curl identity. Include three fixed-seed arbitrary edge vectors
and a pure vertex-gradient vector, not just the original control.

PC03: on the six original shared faces, independently evaluate the candidate
from both owner charts, applying the full two-form pullback and each original
cell metric. Use the existing trace_report unchanged. Require both sampled
physical trace and all three Whitney moments<=1e-9. Also check seven fixed
off-quadrature barycentric points per internal face. H must remain the metric
dual of the actual reconstructed curvature at face and interior probes.

PC04: physical energy gradient agrees on every original patch edge to1e-9
relative-to-max(1,derivative); reconstructing/finite-differencing must use
the same exact constraints. Zero input and pure gauge give zero curvature.

PC05: removing closure, flux enforcement, the zero-curl gauge correction,
proper face pullback, or the reconstruction derivative must break targeted
tests. Tampered coefficients, omitted constraints or forged agreement must
be rejected by independent replay with no stale VERIFIED marker.

## Budget, records, and stop rule

Only six cells, at most26 curvature unknowns, and the patch incidence data.
No global mass solves, global eigenanalysis, native build or refinement sweep.
Hard180-second limit per construction/replay worker; focused tests only.
Preserve all prior evidence and sources. Freeze spec/source hashes before the
real-data result. If infeasible, ill-conditioned or a gate fails, retain the
failure and stop; do not relax thresholds or enlarge the family this turn.

Record source/data hashes, degree/rank decisions, all constraint residuals,
energy/derivative evidence, raw coefficients, positive/negative controls and
external-boundary mismatches. Report LOCAL construction separately from
global correspondence. No automatic continuation beyond this patch.

## Sources

Arnold, Falk, Winther, polynomial de Rham exactness and Koszul homotopy,
equations4.5-4.7: https://sites.math.rutgers.edu/~falk/papers/nacomplexes.pdf
and https://sites.math.rutgers.edu/~falk/papers/bull1278.pdf . These justify
the algebraic identities, not global convergence of this proposed recovery.
Prior shared-reference motivation: Davis (2026),
https://zenodo.org/records/20123423 . This remains an analogy, not an
application of the paper's stochastic construction theorem.
