# Independent assessment and one bounded spec

2026-11-17, Claude, in response to `CLAUDE_RICCI_HANDOFF_2026_11_17.md`.
**Assessment and spec only. No reconstruction implemented, no flow run, no
native change, no commit.** One read-only diagnostic was added and run.

Everything below is labelled MEASURED (receipt-backed), INFERRED (follows
directly from measured facts), THEORY (standard argument, not proved here), or
OPEN (unknown until measured).

## 1. The fixed-exterior refusal is not about this patch

Receipt: `output/ricci-exterior-conformity-20261117/receipt.json`
(sha256 `25ba2377708bbb64634f84b4d3d240ce3045e45284c589e650d52de543a9f104`),
script `scripts/ricci_exterior_conformity_audit.py`. It reuses the verified
per-cell evaluation from `edge_audit` verbatim, applied to every cell around
every edge of the original level1/n=2 Abelian mesh. It reproduces the published
edge-613 witness to 4.2e-17 before reporting anything.

**MEASURED.** The outside "fields" are `old_flux`: the curl of the lowest-order
Whitney one-form, which is constant on each cell. Around edge 613 there are six
cells, five of them outside the patch. Those five disagree **among themselves**
by 0.258 at the witness point (0.076, 0.167, 0.244, 0.333, 0.334). Cells 457 and
461 are just the two that touch the patch.

| Quantity, same pipeline, all 1124 edges | Curvature (what was spliced) | Potential (its one-form) |
| --- | ---: | ---: |
| Edges whose incident cells disagree beyond 1e-9 | **1124 / 1124** | **0 / 1124** |
| Largest disagreement | 0.435 | 3.1e-15 |
| Median disagreement | 0.106 | — |

The potential column is the mechanism-swap control: the curl of that one-form
equals `old_flux` exactly (error 0.0), so both columns are one field seen at two
levels. The pipeline finds agreement where agreement is guaranteed, so the
1124/1124 is a property of the curvature, not of the evaluation.

**INFERRED.** Lowest-order Whitney curvature is normal-continuous and
tangentially discontinuous by construction. A strong curvature-trace splice at
1e-9 therefore fails against the original exterior at every edge of this mesh,
for every patch. A collar of any width does not change this: its new outer
boundary meets the original exterior at edges that are, by this audit, all
incompatible. A collar relocates the obstruction; it cannot remove it.

## 2. Local patches cannot reach BW05 by themselves

**MEASURED**, rates computed from the published tables (h proportional to 1/n):

| Quantity | n=1→2 | 2→3 | 3→5 |
| --- | ---: | ---: | ---: |
| RF2D original Whitney, strong force | +0.198 | +0.012 | **−0.020** |
| RF2D Rayleigh quotient | +1.916 | +1.976 | +1.991 |
| RF2D energy density | +1.913 | +1.976 | +1.990 |
| RF2G minimum-energy recovery, full error | +0.498 | +0.230 | +0.094 |

The original method's energy converges at order two while its force does not
converge at all; between n=3 and n=5 the force error grows. RF2G's rates fall
towards zero. Both are the signature of an error floor, not slow convergence.

**INFERRED.** BW05 measures the full-mesh force. Any construction that leaves
most cells as original Whitney inherits the RF2D floor near 26% outside the
patch. Local patches are diagnostics of a mechanism. Passing BW05 requires the
mechanism on every cell.

**THEORY**, consistent with the RF2D spectral finding that a tiny amount of
high-mode mass is magnified into nearly all of the residual. The force is
compared in the Whitney mass norm, a dual norm over every Whitney variation
tau (tau), including the roughest. Its error is bounded by the curvature
interpolation error, of order h, times the largest gain of curl over those
variations, of order one over h by the standard inverse inequality. Order h
times order one-over-h is order one.

**MEASURED, added later the same day** (`output/ricci-pg-curvature-20261117/s3_modes.json`):
the squared gain, the largest Whitney eigenvalue against the mass, is 2585.5 at
n=1 and 9247.6 at n=2, ratio 3.58, against the factor 4 that one-over-h-squared
scaling predicts. The n=1 value equals the 2585.51465983 reported in the RF2D
HHE section, so that "142-dimensional coarse scalar operator" is this Abelian
Whitney operator (level 0 has 142 edges). An earlier draft of this paragraph
cited it, then withdrew the citation on the mistaken belief that it was a
different operator; the identification is now confirmed numerically. Energy does not
test against rough variations, so it keeps order two. This is why energy
agreement never implied force agreement, here or in RF2F.

## 3. The continuity was imposed on the wrong object

**INFERRED, and the main recommendation.** The boundary extension required the
*curvature* to be tangentially continuous across the splice. Neither a
well-defined energy nor a consistent force needs that. They need:

- **Energy**, one half the integral of the squared curvature of R(a), where R
  (are) reconstructs a potential from edge data a: requires only that curvature
  be square-integrable.
- **Integration by parts in the force**: requires the reconstructed *potential*
  to be tangentially continuous across faces. Its curvature is then
  automatically normal-continuous, and may jump tangentially.

The table in section 1 is exactly this distinction. At the potential level the
original exterior is already a single field at every edge.

## 4. The fork: this is a contract decision for Bee

Two routes, with opposite strengths. Neither is established.

**Route V, variational (Bee's stated preference).** Force is the gradient of an
energy built from a linear reconstruction R. Since R is linear, its complete
derivative applied to a variation is just R applied to the variation, so the
gradient is R-transpose times curl-transpose times mass times curl times R a.
Splitting R as Whitney plus an enrichment N (en), this force equals the Route PG
force below **plus N-transpose applied to an approximately smooth field**.

- Stability: the flow decreases a genuine energy, so it is stable by construction.
- **OPEN, and it is precisely the handoff's strong-adjoint-consistency question:**
  whether N-transpose applied to smooth fields is small in the mass norm.

  **Correction, later 2026-11-17: the sufficient condition first stated here
  was wrong.** It read "every enrichment field has zero mean on every cell."
  That bounds the transpose term by the curvature's deviation from its cell
  mean, order h, times the gain of rough variations, order one over h, which is
  order one, not small. It omitted the very gain this document is about. It is
  also incompatible with accuracy: zero mean keeps Whitney's cell means, which
  are first-order accurate (orders 0.93, 0.98; `cell_means.json`). Bee chose
  option 1 on the strength of the wrong sentence; it was then run as a
  diagnostic and failed BW05 (`RICCI_ZERO_MEAN_RESULTS.md`).
- Boundary: needs the potential, not the curvature, to be conforming. If tested
  locally, splice on the potential's tangential trace, which section 1 shows is
  single-valued against the original exterior.

**Route PG, Petrov-Galerkin.** Reconstructed curvature, tested against the
**original** Whitney variations unchanged.

- Consistency: error is bounded by cellwise curvature error times the order
  one-over-h gain. Curvature accurate to order h to the 1.5 gives rate 0.5; there
  is no test-variation term by construction.
- No conformity is needed anywhere. The existing local curvature reconstruction
  is usable as is. The fixed-exterior obstruction disappears.
- **Cost:** the force is not the gradient of an energy, which departs from the
  variational-first route. The semi-discrete flow is non-symmetric, so stability
  must be **checked**: every eigenvalue of mass-inverse times the PG operator
  needs a positive real part. Not assumed.

## 5. Bounded spec, applicable to either route

**Static only.** One instantaneous force evaluation per mesh. No time stepping,
no Berger, no SU(2) nonlinearity, no native change, no full suite. Abelian
control, the same n = 1, 2, 3, 5 meshes and q4/q6 checks, original signed edge
data, original metric, mass, clock and BW05 thresholds unchanged.

The reconstruction is applied on **every cell**, because section 2 shows nothing
less can pass BW05. R is local and sparse, so this is a sparse static
computation, not a global flow solve.

| Step | Measures | Stop if |
| --- | --- | --- |
| S0 | RED tests: planted polynomial reproduction, linearity, edge-integral preservation, pure-gauge invariance, zero data | — |
| S1 | Curvature reconstruction error and its observed order m, all four meshes | **m below 1.5**: neither route can reach rate 0.5 with this R |
| S2 | PG force error and rates in the BW05 norm, against **all** original Whitney variations | — |
| S3 | PG stability: real parts of the spectrum of mass-inverse times the PG operator | any non-positive real part |
| S4 | Route V only: gradient against central finite differences; then N-transpose applied to the manufactured smooth field, in the mass norm, across meshes | the N-transpose term does not decay |
| S5 | Mechanism removals: replace R by Whitney (must reproduce the RF2D floor); drop the enrichment; corrupt one edge datum | any removal still passes |

S5's first removal is the essential control: with R set back to Whitney, S2 must
reproduce RF2D's flat 26%. If it does not, S2 is not measuring the BW05 quantity.

S1 is first because it is cheap and decides both routes at once.

## 6. Not claimed

No Ricci-flow correspondence, no lattice convergence, no repair of the patch
boundary, no claim that either route works. The section 4 sufficient condition
for Route V is a design target, not a result. Nothing here was run through the
GIGI engine; all numbers come from the Python research scripts, same methods as
the previous attempts. The upper bound in section 2 is a standard argument that
matches the measured rates; a matching lower bound on these meshes is not shown.

## Decision needed from Bee

1. **Route V or Route PG**, or both through S1 before choosing. S1 is common
   to both and decides whether either is reachable.
2. Authorization to apply a reconstruction on **every cell** for a static
   evaluation. This exceeds the previous patch scope; it is not a global flow.
