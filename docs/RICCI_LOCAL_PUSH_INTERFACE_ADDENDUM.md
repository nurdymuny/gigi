# Final bounded separation: cell interiors and shared faces

2026-11-17, declared after the first attribution pass, before this calculation.
The six-cell star and its outside remainder nearly cancel. Raw element forces
include boundary terms, so a large element contribution is NOT evidence of
a bad element. Keep the first receipt and its sources unchanged.

One further diagnostic pass on the SAME level1/q6 case, using the two SAVED
adjoint readouts: zero additional solves, no refinement, no repair. Maximum
960 cells, 3840 face occurrences; timeout120 seconds. Publish the six-cell
patch and its common-face cancellations, retaining the outside remainder.

For the original scalar Whitney field, its exterior derivative is constant
in reference coordinates inside each tetrahedron. Let f (eff) denote its
three flux components and H (aitch) = g f / sqrt(det g) its metric-dual
one-form. For the adjoint test one-form u (you), integrate by parts:

```text
- integral H dot curl(u)
    = - integral u dot curl(H) + boundary integral u dot (normal cross H).
```

Use analytic metric derivatives for the interior term and direct triangular
quadrature for faces. Outward normals are reference-coordinate area vectors,
not unit physical normals; no omitted Jacobian or independently chosen frame.

Controls before interpretation: analytic curl versus finite differences;
direct cell energy derivative versus interior plus boundary; removing boundary
terms and flipping outward normals must break the identity. On retained data,
match the first pass's actual per-cell contributions and preserve whole-readout
closure within absolute1e-9. As a separate oracle, use the smooth control's
H=-2 theta1 (minus two theta one): its shared-face contributions must cancel.
No oracle is fed into the operator or used as a correction.

Interpretation remains local to this Abelian example. A jump of the metric-dual
curvature across a face is not a missing topological connection. Neither
individual face magnitude nor its cancellation establishes convergence or
justifies altering the frozen mapping, mass, norm or thresholds.
