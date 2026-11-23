# Shared-face constraint: bounded diagnostic specification

2026-11-17. Spec first. This implements a constraint CHECKER on the existing
six-cell patch, not an enforcement solve or a new flow candidate.

## Plain-English contract

Neighboring cells must express the same along-face part of their metric-dual
curvature when evaluated at the same physical positions in the same face
coordinates. Components pointing out of the face need not agree for this
particular cancellation. Equal magnitudes are not enough: signs matter.

Sharing vertex positions, an edge integral, a face flux, or a scalar energy
does not by itself impose this additional condition. Nor do two cancelling
readouts test every possible original Whitney variation on that face.

Bee's pyramidal-control paper, sections6 and10.2, motivates checking against a
shared reference rather than relying on local accuracy. This is an analogy,
not an application of its stochastic sequential-control theorem: our mass
solve is already globally coupled. DOI10.5281/zenodo.20123423.

## Mathematical quantity

For cell T (tee), A_T (ay sub tee) is the ORIGINAL Whitney one-form from the
frozen signed edge data. F_T=d A_T (eff sub tee equals dee ay sub tee) is its
curvature two-form. H_T=*g F_T (aitch sub tee equals Hodge-star gee eff sub
tee) is the metric-dual one-form, using the ORIGINAL round metric.

For a common physical face S (ess), pull both H fields back to one oriented
two-dimensional face chart. Let h_plus and h_minus (aitch plus/minus) be those
two-component covectors. The strong interface condition is

```text
j_S = h_plus - h_minus = 0.
```

j_S is pronounced jay sub ess. This is tangential-trace equality, not full
three-component equality, zero curvature, or globally trivial holonomy.
Deck transformations and vertex correspondences are taken from the frozen
signed quotient topology. A future non-Abelian extension must also align
fiber frames with the declared transport; it is OUT OF SCOPE here.

If U_e (you sub ee) is the common trace of an ORIGINAL Whitney basis field
for one of the three face edges, and s (ess) is the first owner's outward
orientation relative to that common chart, define

```text
m_e = -s integral_S U_e wedge j_S.
```

m_e is pronounced em sub ee; wedge is the oriented two-form product. These
THREE signed moments determine the interface contribution for EVERY original
lowest-order Whitney face test, by linearity. Their weighted combination with
the saved adjoints must reproduce BOTH prior observed face contributions.

Strong trace equality implies these moments vanish. The converse fails for
general fields. Report two separate findings:

- `sampled_trace_match`: metric-invariant pointwise jump at q6 nodes <=1e-9.
  This is a sampled diagnostic, NOT a proof of equality between nodes.
- `whitney_moments_match`: all three signed moments <=1e-9 in absolute value.
  The moment computation uses the unchanged q6 numerical integration policy.

Also report physical face L2 jump norm and all residuals, with no averaging,
sign folding or renormalization of inputs. A moment-only match is never
promoted to a strong trace match. No aggregate cancellation across DIFFERENT
faces substitutes for these per-face checks.

## Frozen scope and thresholds

Same n=2/q6 original Abelian data, cells0,2,3,456,458,459; internal faces
0,3,8,9,1009,1012. Same position and original signed links, metric, mass,
readouts and mapping. All twelve outside-facing cell faces remain outside
this check; do not claim whole-mesh compatibility.

Zero new flow solves, adjoint solves, eigensolves, refinements or native builds.
At most the twelve owner-face traces plus chart-reversal controls per replay.
One bounded diagnostic subprocess per run/verification; timeout90 seconds.
No production changes, full suite, commit or deployment.

The existing absolute1e-9 accounting tolerance is retained for cancellations
and replay; 1e-9 is declared here BEFORE runs for the new sampled physical
trace check. Neither diagnostic tolerance replaces or relaxes BW05: both
finest rates >=0.5, full relative error <5%, numerical errors <spatial/10.
Those global gates remain OPEN/unchanged. Zero face jump is not asserted
necessary for every possible convergent discretization; a future controlled
nonzero jump needs the original full dual-mass consistency argument.

## Constitutive and provenance safeguard

The real-data checker computes H from frozen A, curvature and metric itself.
It does not accept user-supplied shared/averaged face values or acceptance
flags. The smooth reference is explicitly `oracle_control`, not a candidate
and never fed back into the engine. Given these fixed lowest-order fields,
there is no independent face-value freedom to enforce agreement. Averaging H
would change the representation or its relation to the stored data.

Source/evidence hashes must match. Verification clears its prior verified
receipt BEFORE reading evidence, independently recomputes all six faces and
both kinds, and compares the full payload. Corrupt, partial, forged-compatible
or nonfinite evidence must fail without a stale verified receipt.

## Strict focused gates

SF00: RED before implementation. Identical tangential traces pass; nonzero
normal-only differences pass; one tangential mismatch and equal-magnitude
opposite signs fail. All input/tolerance/frame checks fail closed.

SF01: common-chart pullback and physical norm agree under coordinate changes;
the shared test traces agree from both owners; common-face orientation reversal
preserves global-edge moments. Do not compare raw local coordinate arrays.

SF02: three-moment completeness, not just the two prior readouts. A planted
quadratic tangential jump q(u)=u^2-(4/5)u+1/10 has zero affine test moments
over the reference triangle but nonzero sampled trace. It must NOT be labeled
strongly compatible. q is pronounced cue; u is you.

SF03: all six smooth-control faces match. Original-data verdicts are derived
without retuning; each prior paired-face readout is independently replayed
from the three new moments within1e-9. Report every face, including any match.

SF04: deliberate removal of tangential projection, orientation/sign handling,
the physical metric norm, or the strong-trace check must break targeted tests.
Forging shared averages must fail original-data replay, not become a repair.

SF05: evidence attacks remove a face, forge a match, alter a moment, introduce
NaN, or drift a source hash. Each must refuse and remove a pre-existing stale
verified receipt. Focused tests only. Implementation checks can pass while
the original discretization fails the mathematical constraint.

## Primary references

- Arnold, Falk, Winther, Finite Element Exterior Calculus (2010), trace and
  integration-by-parts treatment, equations44-46:
  https://sites.math.rutgers.edu/~falk/papers/bull1278.pdf
- DefElement, Nedelec first kind: covariant Piola mapping and tangential
  facet continuity: https://defelement.org/elements/nedelec1.html
- Davis (2026), prior control-topology analogy:
  https://zenodo.org/records/20123423

The first two support the trace identity, not a repair of this operator.
Passing the checker tests is NOT proof of a Ricci/Davis-Wilson correspondence.
