# Smallest boundary extension, fixed six-cell patch

2026-11-17. Spec before implementation and real-data run.

## Scope and preservation

Keep the six cells, saved cover images, exact signed edge observations,
original metric/chart, full Whitney tangent mass, clock, and BW05 thresholds.
Local absolute tolerance remains1e-9, rank cutoff relative1e-12, conditioning
cap1e12. No native change, global solve, full suite, new mesh, degree search,
neighboring-cell modification, commit or deployment. Outside original cells
are read-only boundary targets in this smallest attempt.

The requested extension changes the previous common quadratic curvature
inside its existing freedom only. All19 edge integrals, all18 face fluxes,
coefficient closure, potential/curl identity and six internal trace agreements
must survive. It does not preserve the previous pointwise reconstruction:
requiring that would forbid any boundary correction.

## Smallest candidate: exact flux-neutral boundary lift

Let c0 (see zero) be the accepted26-coordinate closed-curvature fit, Q (cue)
its all-face flux map, and Z (zee) a basis of ker Q (kernel cue). Its dimension
is14, verified from geometry. T (tee) evaluates the original-metric tangential
field AND the physical normal curvature flux on the12 outer faces; h (aitch)
contains those unchanged outside traces. Pointwise normal-flux matching is
necessary for a closed splice: integrated flux alone would not ensure it.

```text
c = c0 + Z u
T Z u = h - T c0
minimize (1/2) (Z u)^T M (Z u), subject to the exact boundary equations.
```

c and u are see and you; M is em, the unchanged q6 physical energy matrix.
This is a minimum physical-energy correction, not regularization. It neither
adds unknowns beyond the existing26 nor changes observed face flux. The shared
closed polynomial retains internal agreement by construction. Recompute its
polynomial potential and vertex-gradient edge correction after an accepted
lift; do not reuse the old potential with changed curvature.

Tangential boundary components are whitened by the original face metric so
their Euclidean two-component norm equals the physical covector norm. Divide
the pulled-back curvature two-form by physical face area density for its
normal component. Each point has three equations; check tangential norm and
absolute normal-flux error separately against1e-9. Use the same q6 nodes as
the previous receipt; verify separately at the same seven off-quadrature points
per face. All faces, not selected favorable ones, must pass. Matching samples
alone is reported as numerical evidence, not an identity over a face.

## Necessary compatibility before any claimed extension

Two outer faces meeting along an edge prescribe the same field component along
that edge if they can be traces of one smooth common field. Use the ORIGINAL
edge ID and orientation, not each face's incidental vertex ordering. Evaluate
six Gauss points and t=0.17,0.37,0.73 on each of the18 boundary edges. Normalize
by physical arc speed. Independently compare a face-to-edge pullback with a
direct evaluation in each original outside cell. Reverse orientations as a
separate control. The common reconstructed field is a compatible positive
control, not a replacement for the outside observations.

If two targets differ by gap g (gee) along the same unit tangent, any common
value has worst error at least g/2. A measured g/2>1e-9 is a necessary-condition
failure for a smooth strong-trace extension with the exterior held fixed.
It is not a no-go for weaker trace spaces, altered outside reconstructions,
or a coupled global method. Passing this necessary check is not sufficient.

One small SVD also checks the exact14-direction boundary equations. Retain
the orthogonal residual r (are), verify (TZ)^T r=0 and r dot rhs=||r||^2.
Its norm divided by sqrt(2 times number of face sample points) is a lower
bound on the best achievable maximum of tangential and normal physical nodal
mismatches in this family. This follows by summing both squared group errors
per point, not by treating each component as an independent norm. A least
squares vector used to form this certificate is NEVER returned as an accepted
extension. No averaging, dropped constraints, increased tolerance or penalty.

The edge witness and linear feasibility certificate may both be evaluated on
this one patch even when one already refuses. After an incompatibility finding,
STOP: no radius/degree escalation or exterior fitting in this turn.

## Strict focused gates

BE00: RED before implementation. Planted compatible and incompatible linear
systems, weighted minimum correction, zero correction, nonfinite refusal,
flux preservation, unit-tangent normalization, orientation reversal, and full
linear reconstruction derivative against finite differences.

BE01: fixed-patch controls. A known flux-neutral closed correction supplies
compatible boundary data. Recover it, retain every edge/flux/closure/curl
identity and all internal traces, and validate boundary values off the fitted
nodes. Also test unchanged common-field boundary data as a zero-correction
control. These are declared manufactured controls, NOT real-data success.

BE02: real exterior targets. Record all12 faces, all18 boundary edges, all
q6 and off-quadrature nodes, direct-cell replay and orientation errors. Native
boundary acceptance requires1e-9, not an improvement over3.738. A refusal is
honest functioning of the checker but is NOT closure of the extension gate.

BE03: retain a checkable incompatibility witness if refused; keep original
accepted reconstruction untouched. If compatible, rebuild potential/gauge,
check original invariants, complete derivative and all boundary/internal traces
before accepting. No full-flow claim even if the local extension passes.

BE04: remove nullspace preservation, exact consistency checking, metric
normalization, orientation handling, full derivative and stale-marker cleanup;
targeted tests must fail by assertions, not incidental crashes. Tampered
numbers, omitted edges/faces, forged acceptance, source drift, NaNs or duplicate
JSON keys must refuse replay and remove a stale VERIFIED marker. A verified
incompatibility receipt explicitly says boundary_extension_passed=false.

## Evidence and bounded budget

Use `output/ricci-boundary-extension-20261117/`. Freeze source/spec/data hashes,
record RED/GREEN and mechanism-removal receipts, then replay from original
inputs without trusting saved conclusions. Limit each worker to180seconds.
Only tiny dense matrices; reuse frozen previous modules read-only. Distinguish
the manufactured success controls, real-data extension result, and checker
verification. No claim of all-suite health or global Ricci correspondence.

## Research basis and limits

Chaumont-Frelet, Ern, Vohralik (2020), section2.3, states common-edge tangential
compatibility for polynomial face traces and requires compatible data before
the extension minimization: https://who.rocq.inria.fr/Martin.Vohralik/Files/Pub_CRAS_20.pdf
and https://arxiv.org/abs/2005.14528 . We use the elementary restriction of one
smooth one-form to a common edge; we do NOT apply their Euclidean polynomial
stability theorem directly to this curved metric-dual field. Their work does
not prove this candidate feasible or establish GIGI's global flow convergence.
