# H8 inter-well path: frozen proof contract

27 November 2026. Baseline `f67812e5`. This is a bounded continuation of
H8 Step 6, not a new model or a numerical spectral run.

## Target and normalization

For every t > 0 put u = 1/sqrt(t) and p = g q, g > 0. Use the existing
Frobenius metric on real 3 by 3 matrices and

W_1(p) = (1/2) sum_k omega_k |p_k|^2 - det(p),
omega = (1/u, u, u), V_1 = |grad W_1|^2/2.

Prove a negative-gradient orbit from P = diag(u,1,1) to 0, with endpoints
at infinite flow time. Deduce that the zero-energy Agmon distance from 0
to P SO(3), defined by inf integral sqrt(2 V_1) |dp|, is exactly u/2.
In q coordinates the corresponding action is u/(2 g^2). No factor of
sqrt(2), no change of kinetic convention, no numerical fit.

The invariant plane p = diag(x,y,y) inherits dx^2 + 2 dy^2. Its equations
must be x' = y^2 - x/u, y' = (x-u)y, not the ordinary coordinate gradient
for dx^2 + dy^2. The forward region is [0,u] times [0,1].

## Required proof steps

1. Derive the plane, metric and vector field from all nine matrix entries.
2. Verify hyperbolicity at (u,1) and an unstable tangent with both
   coordinates pointing into the open rectangle.
3. Establish forward invariance and existence for all positive time.
4. Establish convergence to 0, not merely boundedness or declining W.
   A comparison argument with M = max(x0, u y0^2) < u should give decay.
5. Prove the action lower bound for every ambient path, not only plane paths.
6. Integrate the gradient trajectory and justify finite-endpoint
   connectors as their lengths tend to zero; take the infimum over paths.
7. Preserve the original straight-line upper bound. Test its excess:
   L_straight^2 - (u/2)^2 = (u^2-1)^2/18.

## Gates and formal boundary

- Write this contract and checks before the proof. Retain an absent-proof
  red gate, then a successful release receipt with source hashes.
- Exact SymPy checks: full-space restriction; inherited metric; flow;
  dissipation; equilibria; unstable direction; rectangle; decay comparison;
  action normalization; straight-line excess; round-sphere explicit orbit;
  coupling scaling. No ODE simulation counts as existence evidence.
- Mechanism controls must reject a forgotten repeated coordinate,
  ordinary instead of weighted gradient, reversed flow, and wrong action
  normalization. Lean source mutations must also be compiler-rejected.
- Lean verifies the stated local algebra, derivative and inequalities.
  The unstable-manifold theorem, global continuation/convergence and
  improper-integral/path-infimum argument are written proofs using
  standard analysis, NOT claimed to be end-to-end Lean verified.
- No admissions, custom axioms or native-evaluation shortcuts. Audit
  theorem dependencies and retain the existing H7 verification as a
  focused regression. Never treat build success alone as a proof audit.
- Update only the inter-well sub-result in H8 and the research register.
  H8 carrier ordering, quantum splitting/prefactors, fast-mode dressing
  and the continuum claims stay open. Do not rebuild or alter the paper's
  publication files in this bounded change.
- Commit and push the proof, source and receipts on the existing branch.
