# Boundary extension: verified incompatibility with the fixed exterior

2026-11-17. **Checker verified; boundary extension NOT accepted.**

Spec: `RICCI_BOUNDARY_EXTENSION_SPEC.md`. Same six cells, exact signed input
data, original metric/chart/cover, Whitney tangent mass, clock and thresholds.
No new cells, outside reconstruction changes, degree/radius escalation, global
flow solve, native edit, full suite, commit or deployment.

## Smallest construction tested

The common quadratic curvature has26 closed coefficients and12 independent
face-flux constraints. We implemented a minimum-energy correction restricted
to its14 flux-neutral directions. Thus the correction cannot change any of
the18 face integrals, and the common field retains internal agreement.
Recovering the potential and exact vertex-gradient correction preserves all
19 original edge integrals as well.

The boundary equations include both tangential metric-dual curvature and
pointwise normal curvature flux. The latter matters: preserving only its
integral would not ensure a closed splice to an exterior field. The spec was
clarified to include this before implementation and the real-data run.

This attempt holds the existing outside reconstructions fixed. That is the
smallest extension of the preceding construction, not a claim that every
possible coupled-neighbor construction has been exhausted.

## Real-data obstruction, independent of polynomial degree

Two boundary faces meeting on one edge must prescribe the same component along
that physical edge if they come from one smooth common field. We evaluated
every boundary edge in its original global orientation, using six Gauss points
plus three fixed additional points. All18 edges contain incompatible targets.

At global edge613, with edge parameter0.9662347571015759:

| Outside source | Patch boundary face | Unit-tangent component |
| --- | ---: | ---: |
| Original cell457 | 12 | 0.24368429460572172 |
| Original cell461 | 16 | 0.0759265385083391 |

Their difference is0.16775775609738264. For any common scalar value, at least
one error must be at least half that difference: **0.08387887804869132**.
The unchanged acceptance tolerance is1e-9.

These are incompatible prescriptions at the SAME point and direction, not
values at two different positions or unaligned cell components. Independent
direct-cell evaluation agrees with the face-to-edge route to1.333e-15;
orientation reversal agrees to3.109e-15. The existing common smooth curvature
passes the same compatibility checker as a positive control.

This rules out a smooth strong-trace extension matching these unchanged
outside prescriptions to the required uniform tolerance. Increasing the
polynomial degree inside this patch cannot remove that particular conflict.
It does NOT rule out altered neighboring reconstructions, weaker trace spaces,
or a coupled global method. It is not a general Ricci/Wilson no-go theorem.

The elementary common-edge restriction agrees with the trace compatibility
condition in Chaumont-Frelet, Ern and Vohralik (2020), section2.3:
https://who.rocq.inria.fr/Martin.Vohralik/Files/Pub_CRAS_20.pdf . Their Euclidean
polynomial stability theorem was not applied as a theorem about this curved
metric-dual reconstruction; the numerical witness above is our own data.

## Independent finite-family feasibility certificate

The12 boundary faces supply432 q6 points and1296 equations: two physical
tangential components and one normal-flux component at each point. The
flux-neutral boundary map has rank14. Its exact equations are incompatible.

| Quantity | Measured value |
| --- | ---: |
| Orthogonal residual norm | 1.4568724810956541 |
| Residual orthogonality check | 2.817e-12 |
| Dual-witness identity check | 7.217e-13 |
| Lower bound on maximum nodal group error in this family | 0.049563808319262284 |

The last bound divides the residual norm by sqrt(2 times432), accounting for
the tangential norm and normal-flux absolute error at each point. It is a
lower bound, NOT an achieved corrected mismatch. Neither this bound nor the
edge witness is a replacement for the unchanged1e-9 acceptance threshold.
No least-squares approximation was published as an accepted extension.

## The implemented lift works on compatible inputs

A declared manufactured control uses a nonzero known correction within the
same14 flux-neutral directions. Its boundary data are generated from that
known closed field, not from the real outside observations. The construction
recovers it and checks unseen boundary points independently.

| Compatible-control check | Maximum residual |
| --- | ---: |
| Recovered coefficients | 1.111e-16 |
| All original edge integrals | 2.256e-17 |
| All original face fluxes | 2.082e-17 |
| Coefficient closure | 8.327e-17 |
| Potential curl identity | 2.776e-17 |
| Internal physical trace agreement | 1.161e-14 |
| Internal Whitney moments | 4.943e-17 |
| Off-quadrature boundary tangential agreement | 1.839e-15 |
| Off-quadrature boundary normal-flux agreement | 1.284e-15 |

Small planted algebra cases independently check the weighted minimum, zero
correction, conflicting constraints, metric normalization, orientation and
the complete linear derivative with respect to base and boundary inputs.
These successful controls validate the implementation; they do not turn the
real-data refusal into success.

## Gate receipts

Final focused run:15 tests pass, comprising11 new tests and4 existing patch
tests. Six deliberate mechanism removals trigger assertion failures, not
incidental exceptions: flux-nullspace restriction, incompatible-target refusal,
physical normalization, orientation sign, boundary derivative and stale-marker
cleanup. Seven real-evidence attacks reject altered values, omitted edges,
omitted faces, forged acceptance, source drift, nonfinite numbers and duplicate
keys. Every attack removes its stale VERIFIED marker.

Fresh-process replay confirms:

```text
evidence_verified = true
boundary_extension_passed = false
status = REFUSED_INCOMPATIBLE_BOUNDARY
global_gate = UNCHANGED_OPEN
```

Evidence: `output/ricci-boundary-extension-20261117/`. The diagnostic retains
all edge samples and both source-cell values, all boundary targets, the
orthogonal residual vector, compatible controls, and pinned source/input hashes.
Diagnostic SHA256:
`d8297e815db09ddf2adb89f55cb024bb2b86b16624cec05074159daf5387fe5a`.
Base HEAD: `0a64e2f848693221bf25f330ea4fb19041249106`; the worktree is dirty,
so source hashes identify the tested implementation. Earlier phase receipts
retain their earlier source versions; the final receipt pins the final sources.

Commands used, with RED executed before the corresponding implementation:

```powershell
python scripts/ricci_boundary_tdd.py red
python scripts/ricci_boundary_tdd.py green
python scripts/ricci_boundary_tdd.py red --evidence
python scripts/ricci_boundary_tdd.py green --evidence
python scripts/ricci_boundary_gate.py
```

Recheck retained evidence without overwriting the diagnostic:

```powershell
python scripts/ricci_boundary_extension.py verify
```

## What must change in any future proposal

The old exterior traces cannot all be immutable boundary targets for one
smooth internally agreeing field. A future proposal must allow neighboring
reconstructions to be determined together, or explicitly change the continuity
question. It may still preserve the original measurements; changing a derived
field is not the same as changing its measured edge data. Such a proposal must
also address the new outer boundary and the original strong-adjoint-consistency
question. Adding one collar is NOT established as sufficient by this result.

The prior local construction remains untouched and verified. Its original
3.738 boundary mismatch has not been repaired. Do not repeat a higher-degree
fixed-exterior fit as if it were a new approach. **Stop here; no gate closure.**
