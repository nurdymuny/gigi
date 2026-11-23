# Ricci / Davis-Wilson lattice correspondence: PARKED

**2026-11-17. Bee explicitly stopped this work. Read this before restarting.**
Do not automatically rerun experiments, search parameters, build another
recovery, run broad suites, or advance to Berger. No release gate is closed.
No new lattice claim is approved for the chapter. No commit or deployment.

## Established, with scope

- **RF2A:** exact symbolic instantaneous Ricci/Yang-Mills agreement for the
  declared positive diagonal left-invariant SU(2) metrics. Tested
  moving-background trajectories agree; fixed-background equivalence fails
  in the tested nonround cases. Not a general Ricci-flow or lattice theorem.
- **RF2D:** native differentiation agrees with its own action. Independent
  Stokes/incidence, mass and quadrature checks pass. The measured full-flow
  discrepancy is dominated by action/load inconsistency, not solve error.
  It already occurs in an infinitesimal Abelian control.
- Signed double-cover tests found no local sign/branch loss explaining these
  data. HHE/mass whitening checked operator-norm behavior, not spatial
  consistency. Neither result rules out phase transitions in other settings.
- Energy accuracy, exact face fluxes, polynomial reproduction and a correct
  energy derivative are insufficient to establish the required full flow.
  Finite numerical checks are evidence, not general convergence proofs.

## Attempts not to repeat

| Attempt | Finest full relative error | Disposition |
|---|---:|---|
| RF2B original lattice, Berger | 37.31% | FAIL |
| RF2C shape-regular refinement, Berger | 25.51% | FAIL |
| RF2D original Abelian Whitney control | 26.13% | FAIL |
| RF2F fitted closed-affine / compatible BDM1 recovery | 80.91% | Verified FAIL |
| RF2G minimum physical energy at exact face fluxes | 20.58% | Verified FAIL; STOP |

RF2F's energy is 1.99999058 against 2, yet its flow fails. RF2G's four errors
are 33.4652%, 23.7022%, 21.5955%, 20.5839%; final rates 0.22989 and 0.09301.
Its finest quadrature difference is 1.78e-9 and solve-error estimate 4.27e-12.
These do not explain the remaining 0.82297 absolute full-mass error.

**Separate lead, not a pass:** RF2E regularization recorded a 1.4291% Abelian
score. It changes the discrete energy, was excluded after variational-first
direction, and lacks completed independent acceptance and a Berger extension.
Do not relabel it as an authorized repair or repeat it as a new discovery.

## Exact question before more implementation

**Can a computable, data-only, energy-derived discretization achieve strong
adjoint consistency on the ORIGINAL exact links with the ORIGINAL Whitney
tangent mass, or is a stated part of that contract incompatible with the
desired convergence?**

For a declared smooth connection class containing the controls, construct a
candidate energy E_h and justify a bound of the following kind:

```text
sup over eta != 0 of
  | dE_h(I_h A)[eta] + <W(A), T_h eta>_L2(g) | / ||eta||_B_h
    <= C(A,g) h^s,     s >= 1/2,     C independent of mesh size h.
```

Here I_h (eye sub aitch) is the frozen exact-link map; W (double-you) is the
continuum negative energy gradient; T_h (tee sub aitch) is the ORIGINAL
transported Whitney trial field; B_h (bee sub aitch) is its ORIGINAL mass;
eta (ay-tuh) ranges over **all** discrete variations, not only homogeneous
or smooth ones. E_h (ee sub aitch) must be built without the known target
force, fitted clock, removed error modes or substituted input projection.
Together with a justified mass/interpolation consistency bound, this controls
the full velocity error. A different rigorous route to that same full-error
conclusion is acceptable; energy agreement alone is not.

Russian Geom Work identified two terms to control: curvature reconstruction
error AND the mismatch between reconstructed test variations and original
Whitney variations. Differentiation can amplify small errors by an inverse
mesh scale. No unfiltered construction with the needed bound is currently
established. Their review was advisory, not an independent numerical replay.

## Conditions for reopening

Bee must explicitly resume. First supply a defensible consistency argument
or obstruction analysis addressing the question above; agree a small budget
before implementation. Increasing degree/radius or refining again without
that argument is not justified by current evidence.

Keep BW05: SAME n=1,2,3,5 meshes, signed links, mass, clock1 and q4/q6;
both finest observed rates >=0.5, final FULL relative error <5%, numerical
errors <spatial error/10. An asymptotic bound alone does not pass this finite
gate. Abelian before Berger. Focused TDD and mechanism-removal checks first.
Changing the input projection, norm, clock, or testing positive-time solution
convergence is a NEW research question requiring explicit agreement, not a
retroactive pass. Report to Russian Geom Work before chapter changes.

## Where to resume reading, not running

- `docs/RICCI_FINAL_BOUNDED_ATTEMPT.md`: synthesis, primary research links and
  last pre-run spec. In particular Arnold et al. (2014) document Whitney
  codifferential inconsistency; Arnold-Chen (2017) treat parabolic FEEC.
  Neither paper proves GIGI's proposed repair or a no-go for these manifolds.
- `docs/RICCI_WILSON_CORRESPONDENCE_RESULTS.md`: supported continuum scope.
- `docs/RICCI_OPERATOR_CONSISTENCY_RESULTS.md`: diagnosis and controls.
- `docs/RICCI_VARIATIONAL_RECOVERY_RESULTS.md`: RF2F derivation and failure.
- `docs/RICCI_FINAL_BOUNDED_RESULTS.md`: RF2G derivation, failure and commands.
- `docs/RICCI_RECONSTRUCTION_EXPLORATORY_STATUS.md`: excluded RF2E lead.
- Evidence: `output/ricci-consistency-20261117/diagnostic/`,
  `output/ricci-variational-20261117/abelian/`, `output/ricci-final-20261117/`.

RF2F: 18 focused tests, five removal mutants, eight case replays, six evidence
attacks. RF2G: four focused tests, minimization-removal failure, eight replays,
three evidence attacks. Bad evidence cannot retain a stale verified/PASS file.
Preserve all prior sources and receipts: manifests contain exact dirty-tree
source/matrix hashes; HEAD alone is insufficient. Base HEAD was
`0a64e2f848693221bf25f330ea4fb19041249106`.
RF2G diagnostic SHA256:
`f41afff646bd4165527700bf316f8ca3c2239305f755009d73c6241cac7c1968`.
Final status: **evidence valid; numerical gate FAIL; correspondence NOT
ESTABLISHED; Berger NOT RUN; release OPEN; research PARKED.**

## Visual follow-up only (2026-11-17)

At Bee's request, inspected retained RF2D/RF2F Abelian velocities on one fixed
physical slice and crossing, selected without looking at the errors. No new
flow solve, operator change, threshold change or gate closure. RF2G was NOT
visualized; this is not a nonlinear Ricci trajectory or a global error test.

Pictures show cell-scale striping and sideways motion. Crucially, the known
reference drawn through the SAME Whitney representation is nearly smooth;
the display/representation alone does not explain the larger visible zigzags.
This suggests inspecting the curvature-to-motion transfer, not another
curvature-only fit or refinement. It does NOT establish which replacement
works. Existing cells are already coupled; do not misdiagnose missing gluing.
Do not smooth the pictures or remove modes to claim a repaired full flow.

Figures, sampled fields, source hashes and same-renderer control receipts:
`output/ricci-visual-review-20261117/`. Scripts: `ricci_visual_review.py`,
`ricci_visual_closeup.py`, `ricci_visual_target_control.py` under `scripts/`.
The exact consistency question and conditions for reopening above remain.

## Bounded individual-push follow-up (2026-11-17)

Bee authorized attribution only, not another repair. On one fixed point of
the original Abelian n=2 mesh, a complete six-cell edge star was visualized,
with the rest of the mesh explicitly retained. Two cached-mass adjoint solves
attribute the saved velocity; an independent integration-by-parts pass uses
no further solves. No mapping, operator, norm, threshold or production change.

**New localization:** shared-face contributions dominate this point's sideways
error. Smooth-reference contributions cancel across all1920 shared faces to
4.80e-14; discrete-curvature contributions leave remainders. Interior and
interpolation contributions are small by comparison. This is not a missing
connection, a rogue-cell diagnosis, or evidence that deleting face terms is
valid. The latter breaks the exact energy-derivative accounting.

Read `docs/RICCI_LOCAL_PUSH_RESULTS.md` and
`output/ricci-local-push-20261117/shared-face-pushes-v3.png` before repeating
this work. Eight focused tests and four mechanism-removal failures; prior
gates unchanged. The full-consistency question above remains open. STOP.

## Shared-face constraint checker (2026-11-17)

Bee authorized a spec and focused TDD on the SAME six-cell patch. The condition
is common-chart tangential equality of the metric-dual curvature, with all
three original Whitney face-edge moments checked separately. Normal-only
differences are not failures; moment-only agreement is not strong equality.

All six smooth-reference faces match; all six frozen original faces mismatch.
The new moments independently replay prior observed face contributions to
6.242e-15. Twenty focused tests pass; eight mechanism removals fail by assertion;
six evidence attacks reject and remove stale verification. Zero new solves.

Read `docs/RICCI_SHARED_FACE_CONSTRAINT_SPEC.md` and
`docs/RICCI_SHARED_FACE_CONSTRAINT_RESULTS.md`; evidence is under
`output/ricci-shared-face-20261117/`. This is a verified CHECKER, not enforcement
or a repaired operator. Mapping, original field/mass, clock and BW05 unchanged.
No independent face-value freedom exists in the frozen lowest-order field;
averaging a substitute face trace is not a justified construction. STOP.

## Shared-curvature construction on the same patch (2026-11-17)

Bee authorized the smallest construction in a bounded polynomial family, not
a global repair. One common CLOSED quadratic curvature field on the same six
cells now preserves all nineteen signed edge integrals and eighteen oriented
face fluxes, including the boundary. A recovered potential plus an exact
vertex-gradient correction supplies the connection. No trace averaging or
analytic-control fitting. The original mapping, mass and thresholds remain.

Geometry-only ranks select degree2: constant/linear/quadratic closed fields
span 3/11/12 face-flux directions; the patch requires12. This is minimal only
within that declared family. All six internal face traces agree to1.136e-14;
edge/flux errors are5.551e-17/1.648e-17. Complete energy differentiation passes
on every edge to5.457e-13. Three arbitrary vectors and a pure gauge also pass.

**Critical limitation:** exterior pointwise mismatch against unchanged
neighbors increases from0.1096544 to3.7380949, despite preserved integrated
flux. Do NOT splice this patch field into the old mesh or claim full-flow
repair. A compatible global extension and strong adjoint consistency remain
unanswered. This is a verified LOCAL construction, not a BW05 pass.

Read `docs/RICCI_SHARED_CURVATURE_PATCH_SPEC.md` and
`docs/RICCI_SHARED_CURVATURE_PATCH_RESULTS.md`. Evidence:
`output/ricci-shared-curvature-20261117/`. Fifteen focused tests, six mechanisms
removed and caught by assertions, six real-evidence attacks rejected with
stale PASS removed; fresh-process replay verified. No full suite, native
operator change, new global solve, Berger run, commit or deployment. STOP.

## Fixed-exterior boundary extension (2026-11-17)

Bee authorized the smallest extension on the same patch. We implemented a
closed quadratic correction in the14-dimensional nullspace of all face flux
constraints, with unchanged edge data, mapping, mass and tolerance. Boundary
equations include BOTH tangential metric-dual curvature and pointwise normal
curvature flux. The lift works on planted compatible inputs and preserves
edges, fluxes, closure, potential curl and internal agreement.

**Real-data result: REFUSED_INCOMPATIBLE_BOUNDARY.** The unchanged exterior
traces conflict along all18 boundary edges. At edge613, outside cells457/461
prescribe0.24368429460572172 and0.0759265385083391 in the same physical tangent
direction at the same point. Any common value has error>=0.08387887804869132
on at least one side, versus the unchanged1e-9 threshold. Independent direct
cell replay error1.333e-15; orientation check3.109e-15. Thus a higher-degree
smooth fixed-exterior fit cannot solve this particular incompatibility.

The14-direction boundary equations independently refuse, with a certified
numerical lower bound0.0495638 on maximum nodal group mismatch. This is NOT
an achieved correction. No approximate output was accepted. The original
local construction and its3.738 exterior mismatch are unchanged.

Read `docs/RICCI_BOUNDARY_EXTENSION_SPEC.md` and
`docs/RICCI_BOUNDARY_EXTENSION_RESULTS.md`. Evidence under
`output/ricci-boundary-extension-20261117/`:15 focused tests, six removals
caught by assertions, seven corrupted-evidence refusals with stale markers
removed, and fresh replay. VERIFIED records evidence=true, extension=false.
No full suite, new mesh/cells, native edit, global solve, commit or deployment.

Any future candidate must reconsider immutable outside reconstructions and
determine compatible neighboring fields together while preserving observations;
neither one extra collar nor global convergence is proved sufficient here.
Do not repeat a fixed-exterior polynomial fit. BW05 remains OPEN. STOP.
