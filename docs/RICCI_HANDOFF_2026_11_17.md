# Claude handoff: GIGI / Davis-Wilson / Ricci correspondence

Prepared for Bee Rosa Davis, 2026-11-17. Bee is transferring this work because
her ChatGPT credits are nearly exhausted. She would like an independent look
and a carefully bounded attempt, not repetition of the previous experiments.

Git checkpoint: `codex/feat-ricci-correspondence-checkpoint`. See the
[checkpoint README](C:/Users/nurdm/OneDrive/Documents/gigi/research/ricci-checkpoint-20261117/README.md)
for archived evidence, exact source snapshots and historical replay caveats.
The scientific receipts retain their pre-checkpoint HEAD and absolute paths;
after the checkpoint commit, a provenance refusal is not a new numerical failure.

## Start here

**We have a verified local curvature reconstruction, but no repaired full
lattice flow. The most recent boundary extension correctly REFUSES the actual
outside fields because their prescribed traces are incompatible.**

Do not spend resources repeating a higher-degree fit against those same fixed
outside traces. First review the evidence and decide what mathematical
construction would actually change the compatibility problem. Neighboring
derived fields may need to be reconstructed together while keeping the
original measured edge data. That is a direction to investigate, NOT a proven
solution. Adding one more ring of cells is not established as sufficient.

The earlier `RICCI_HANDOFF.md` opens with PARKED. That records a historical
stop on broad repair attempts; Bee subsequently authorized the bounded patch
investigations summarized below. Her current request is this handoff to you.
Do not interpret it as authorization for an unlimited global computation.

## Environment and operating rules

- Repository: `C:\Users\nurdm\OneDrive\Documents\gigi`.
- PowerShell on Windows. Python, NumPy and SciPy ran the recent patch work.
- Current/base HEAD: `0a64e2f848693221bf25f330ea4fb19041249106`.
- The worktree is very dirty, with substantial unrelated work. Do not reset,
  revert, clean, stage everything, or assume HEAD identifies the experiment.
  Final evidence manifests pin the actual source and input hashes.
- Read [AGENTS.md](C:/Users/nurdm/OneDrive/Documents/gigi/AGENTS.md).
- Bee requires: spec first, RED tests before implementation, GREEN tests,
  deliberate mechanism-removal failures, then a convincing worked example.
  A refusal is not an engine defect; an example failing its numerical target
  is not a successful feature gate simply because its checker passes.
- **Focused tests only. No full suite until an actual passing result warrants
  it.** The last workers were capped at180seconds each.
- Preserve all previous sources and receipts. Create new scripts/evidence
  directories for a new experiment. Old verifiers check source hashes.
- No commit, deployment, book claim, server build, or Berger run was done in
  these recent patch steps. No native GIGI operator was changed.
- Explain findings plainly to Bee; pronounce mathematical symbols when used.

## Frozen mathematical contract

The objective is an actual discrete-to-continuum relationship on the same
manifolds, not visually similar curves or energy agreement alone.

Keep the original exact signed-link input map, original round/declared metric,
original transported Whitney tangent mass and full-error norm, clock1, and
the same shape-regular meshes. Do not replace signed links by sign-folded
ones, fit a time scale, substitute a projection, discard troublesome modes,
or insert the known target force into the reconstruction.

BW05 remains OPEN: same n=1,2,3,5 meshes and q4/q6 checks; both final observed
convergence rates >=0.5; finest FULL relative error <5%; numerical errors
<spatial error/10. Abelian control before Berger.

Patch tolerances: absolute1e-9 for preserved observations, closure and traces;
rank cutoff relative1e-12; condition cap1e12. Derivative checks use error
relative to max(1, derivative). No tolerances were relaxed.

The central unresolved issue is **strong adjoint consistency**: can an energy
built from the original data produce the correct full force when tested against
ALL original Whitney variations? Accurate curvature alone is insufficient;
the derivative of the reconstruction also changes the test variations.

## What is established, and what failed

The declared positive diagonal left-invariant SU(2) continuum controls have
exact symbolic instantaneous Ricci/Yang-Mills agreement. Tested moving-metric
trajectories agree. This is NOT a general Ricci-flow theorem or lattice result.

Native differentiation agrees with its own discrete action. Independent
incidence/Stokes, mass, quadrature and solve checks localize the main error
to the action/load, already in the infinitesimal Abelian control.

| Previous attempt | Finest full relative error | Status |
| --- | ---: | --- |
| Original lattice, Berger | 37.31% | Failed |
| Shape-regular mesh, Berger | 25.51% | Failed |
| Original Abelian Whitney control | 26.13% | Failed |
| Closed-affine / compatible BDM1 recovery, RF2F | 80.91% | Verified failure |
| Exact-flux minimum physical energy, RF2G | 20.58% | Verified failure |

RF2G's final rates were0.22989 and0.09301, not the required0.5. RF2F's energy
was almost exact despite its poor force. Do not repeat either as a new idea.
RF2E recorded1.4291% using a changed regularized energy, but was excluded after
Bee requested a variational-first route; it lacks completed independent
acceptance and a Berger extension. It is a separate lead, not an approved pass.

Signed double-cover controls did not identify a sign/branch error explaining
these cases. HHE/mass whitening tested operator norms, not spatial consistency.
Keep Bee's double-cover/phase-transition concern in view without asserting
that it explains a discrepancy the controls did not attribute to it.

## The local result that DOES work

Visual attribution localized much of the sideways error at one sampled point
to shared-face terms. The cells were already coupled; this is not a missing
connection or a rogue-cell diagnosis. Deleting face terms breaks the energy
derivative and is not a repair.

Fixed patch: cells `[0, 2, 3, 456, 458, 459]`, around original edge103, on the
original Abelian level1/n=2 geometry. Eight vertices, nineteen edges, eighteen
faces; six internal faces and twelve exterior faces.

One common CLOSED quadratic curvature field over these six cells can retain
all edge integrals and face fluxes while enforcing internal trace agreement.
Constant, linear and quadratic closed fields have flux-map ranks3,11,12;
the patch needs12. Quadratic is minimal within that tested family only.

Construction: choose the minimum q6 physical-energy fit at exact face fluxes;
recover its polynomial potential by homotopy; add an exact vertex-gradient
correction to restore every original edge integral. Apply the full projective
chart Jacobians and two-form pullbacks. The pointwise curvature approximation
changes; the observations and original physical mapping do not.

Measured maxima: internal trace jump1.136e-14; edge error5.551e-17; face-flux
error1.648e-17; complete energy-derivative error5.457e-13. Three arbitrary edge
vectors, pure gauge and zero data pass the controls. Fifteen focused tests,
six removal checks and six evidence attacks passed their respective gates.

**But the boundary mismatch against unchanged outside cells rises from
0.1096544 to3.7380949. This field cannot be inserted as an isolated repair.**

## Latest result: why the smallest boundary lift refuses

The common quadratic has26 closed coefficients minus12 flux constraints,
leaving14 flux-neutral correction directions. We implemented an exact
minimum-energy boundary lift in those directions, preserving curvature
closure, all measured integrals and internal agreement. Boundary equations
include both tangential metric-dual curvature AND pointwise normal curvature
flux, since integral preservation alone would not ensure a closed splice.

It succeeds on manufactured compatible inputs. It does NOT fit the actual
fixed exterior. All18 boundary edges have conflicting outside prescriptions.

An explicit witness, global edge613 at edge parameter0.9662347571015759:

| Original outside cell | Boundary face in patch indexing | Same unit-tangent component |
| --- | ---: | ---: |
| 457 | 12 | 0.24368429460572172 |
| 461 | 16 | 0.0759265385083391 |

Gap0.16775775609738264. Any common value must miss one side by at least
0.08387887804869132, against tolerance1e-9. Direct-cell versus face-to-edge
evaluation agrees to1.333e-15; orientation reversal to3.109e-15.

This obstructs a SMOOTH strong-trace extension with those outside fields held
fixed, regardless of interior polynomial degree. It does not rule out a
coupled reconstruction, a different trace-space formulation, or the research
program as a whole. The conflict concerns derived outside fields; do not
silently turn them into immutable truths about the original measurements.

A separate linear certificate uses432 boundary points /1296 equations and
rank14. Its orthogonal residual norm is1.4568724810956541; the lower bound on
maximum nodal group error is0.049563808319262284. These are infeasibility
witnesses, NOT achieved improved boundary mismatches.

Latest gate:15 focused tests (11 new plus4 prior patch tests), six mechanism
removals caught by assertions, seven real-evidence attacks rejected with stale
verification removed. Fresh-process replay records:

```text
evidence_verified = true
boundary_extension_passed = false
status = REFUSED_INCOMPATIBLE_BOUNDARY
global_gate = UNCHANGED_OPEN
```

## Read these files in order

1. [Latest spec](C:/Users/nurdm/OneDrive/Documents/gigi/docs/RICCI_BOUNDARY_EXTENSION_SPEC.md)
   and [latest results](C:/Users/nurdm/OneDrive/Documents/gigi/docs/RICCI_BOUNDARY_EXTENSION_RESULTS.md).
2. [Successful local construction spec](C:/Users/nurdm/OneDrive/Documents/gigi/docs/RICCI_SHARED_CURVATURE_PATCH_SPEC.md)
   and [local results](C:/Users/nurdm/OneDrive/Documents/gigi/docs/RICCI_SHARED_CURVATURE_PATCH_RESULTS.md).
3. [Historical handoff and exact consistency question](C:/Users/nurdm/OneDrive/Documents/gigi/docs/RICCI_HANDOFF.md).
4. [Prior research synthesis](C:/Users/nurdm/OneDrive/Documents/gigi/docs/RICCI_FINAL_BOUNDED_ATTEMPT.md)
   and [last global attempt](C:/Users/nurdm/OneDrive/Documents/gigi/docs/RICCI_FINAL_BOUNDED_RESULTS.md).
5. [Local push attribution](C:/Users/nurdm/OneDrive/Documents/gigi/docs/RICCI_LOCAL_PUSH_RESULTS.md).

Useful picture:
[Shared-face pushes](C:/Users/nurdm/OneDrive/Documents/gigi/output/ricci-local-push-20261117/shared-face-pushes-v3.png).

## Code and data entry points

| File | Important contents |
| --- | --- |
| [Boundary extension](C:/Users/nurdm/OneDrive/Documents/gigi/scripts/ricci_boundary_extension.py) | `exact_lift`, `boundary_system`, `edge_audit`, `finalize`, `build_receipt`, `verify` |
| [Boundary tests](C:/Users/nurdm/OneDrive/Documents/gigi/scripts/test_ricci_boundary_extension.py) | Planted weighted minimum, refusal, derivative and fixed-patch controls |
| [Boundary gate](C:/Users/nurdm/OneDrive/Documents/gigi/scripts/ricci_boundary_gate.py) | Focused tests, removals, real evidence attacks |
| [Common patch construction](C:/Users/nurdm/OneDrive/Documents/gigi/scripts/ricci_shared_curvature_patch.py) | `geometry`, `rank_audit`, `construct`, `owner_field`, `face_diagnostics` |
| [Polynomial algebra](C:/Users/nurdm/OneDrive/Documents/gigi/scripts/ricci_shared_curvature_math.py) | Closure, potential, curl, exact-degree integration, constrained energy |
| [Unchanged trace checker](C:/Users/nurdm/OneDrive/Documents/gigi/scripts/ricci_shared_face_math.py) | Physical trace norm and three original Whitney face moments |

Frozen inputs:
- [Patch arrays](C:/Users/nurdm/OneDrive/Documents/gigi/output/ricci-local-push-20261117/attributions.npz):
  `patch_ids`, aligned `patch_points`, original `chart`, saved adjoints. Deck
  transforms are already reflected in the retained cover points.
- [Original native geometry](C:/Users/nurdm/OneDrive/Documents/gigi/output/ricci-shape-regular-20261117/berger/level1_berger_q6.input.json):
  960 cells,1124 edges. The folder says Berger; the recent experiment uses the
  original scalar ABELIAN control, not the Berger link values.
- [Original Abelian arrays](C:/Users/nurdm/OneDrive/Documents/gigi/output/ricci-consistency-20261117/diagnostic/level1_q6.arrays.npz):
  `abelian_edge_integrals`, `abelian_velocity`.

Latest [diagnostic](C:/Users/nurdm/OneDrive/Documents/gigi/output/ricci-boundary-extension-20261117/diagnostic.json)
and [final gate](C:/Users/nurdm/OneDrive/Documents/gigi/output/ricci-boundary-extension-20261117/final-gate.json).
Diagnostic SHA256:
`d8297e815db09ddf2adb89f55cb024bb2b86b16624cec05074159daf5387fe5a`.

Successful local construction [diagnostic](C:/Users/nurdm/OneDrive/Documents/gigi/output/ricci-shared-curvature-20261117/diagnostic.json)
SHA256: `b37c290a021504763b4289f687a6f29326c72ce73ddd11f1377656c68e45c0fe`.

## Cheap checks, only if needed

No experiments were rerun to create this handoff. These are optional existing
commands, not a request to run everything:

```powershell
Set-Location 'C:\Users\nurdm\OneDrive\Documents\gigi'
python scripts/ricci_boundary_extension.py verify
python scripts/ricci_shared_curvature_patch.py verify
python -m unittest discover -s scripts -p test_ricci_boundary_extension.py -v
```

Verification recomputes the same small patch and refreshes its VERIFIED file;
it does not change the diagnostic or run a global flow. Source drift can make
replay refuse. Investigate that; do not overwrite evidence or weaken checks.
The RED/GREEN and final-gate runners refuse to overwrite existing receipts.
For new work use separate files/directories rather than deleting receipts.

## Research context and useful next question

[Chaumont-Frelet, Ern, Vohralik, section2.3](https://who.rocq.inria.fr/Martin.Vohralik/Files/Pub_CRAS_20.pdf)
states the common-edge compatibility condition for polynomial tangential
traces. It supports the restriction argument; its Euclidean stability theorem
was not used as a theorem for this curved metric-dual field.

The earlier synthesis links work on Whitney codifferential inconsistency and
parabolic FEEC. Review those primary papers before inventing another recovery.
The Russian Geom Work chat previously flagged both curvature error AND the
mismatch between reconstructed variations and original Whitney variations.
That advice was not an independent replay of the latest numerical work.

**Suggested first deliverable to Bee:** a short independent assessment of the
fixed-exterior obstruction, followed by one bounded spec for jointly compatible
neighboring reconstructions, or a precise explanation of what contract must
change. Explain how the new outer boundary and complete derivative are handled
before coding. Do not promise that an added collar or more polynomial degree
will fix the full flow. Keep the original observations and acceptance contract
unless Bee explicitly chooses a new research question.
