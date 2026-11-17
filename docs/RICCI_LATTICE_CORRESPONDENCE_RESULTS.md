# RF2B: actual lattice / Ricci correspondence experiment

Date: 2026-11-17.

**Scientific result: NO-GO for this discretization. BW05 fails. The full
correspondence and release gates remain OPEN. No commit or deployment.**

The mapping was specified before implementation in
`RICCI_LATTICE_CORRESPONDENCE_SPEC.md`. Its opening NOT RUN status records
the specification freeze; this document records the subsequent experiment.
The existing RF2A continuum identity is not invalidated by this result.

## What was actually built

- An exact native Levi-Civita connection-to-SU(2)-link adapter with declared
  path, orientation, half-angle convention and input refusals.
- Native 3D Whitney edge and face mass matrices for curved radial
  tetrahedra, retaining all off-diagonal entries and the anisotropic metric.
- A covariant simplicial Wilson-loop action, its complete 18-component
  cell derivative, and a gauge-covariant tangent mass.
- A native assembly executable, not a public GQL route. The diagnostic
  linear solve uses SciPy CG and explicitly reports that fact.
- Exact deck-orbit reduction of the group-averaged cover operator,
  independently compared with complete cover assembly at level zero.

The legacy Hamiltonian force was not replaced or retuned. Direct native
action/drift/force tests establish its existing normalization. The new
metric-coupled action is explicitly an extension, not a claim that the
old unweighted staple already represented this curved 3D geometry.

There is a concrete reason an extension is needed: multiplying the metric
by seven leaves these connection links and the legacy force unchanged,
while the continuum velocity scales by one seventh. The assembled new
mass/action obey the proper tensor scaling, checked without fitting time.
The native legacy-force directional-derivative check has maximum absolute
error `2.8466062840237782e-11`.

## Same manifolds, actual cells

The cover is the radially projected boundary of the regular 600-cell in
R4, constructed from the binary icosahedral group. Its quotient uses the
same 120-element left deck action. No cubic torus is relabeled as S3.

| Level | Tetrahedra | Cover edges | Maximum angular edge length |
|---|---:|---:|---:|
| 0 | 600 | 720 | 0.628318530718 |
| 1 | 7,200 | 8,640 | 0.326366221807 |
| 2 | 86,400 | 103,680 | 0.164833703214 |
| 3 | 1,036,800 | 1,244,160 | 0.082627469629 |

The finest operator has 31,104 independent quotient tangent components,
representing 3,732,480 cover components. This is not a three-parameter
homogeneous projection. Every cell orbit and its ordering multiplicities
are retained. Reversed edges include the required adjoint, not just a sign.

The independent continuum target is the matrix-exponential Frechet
derivative of the actual link map applied to the RF2A velocity. The native
action/gradient receives no target velocity. No clock or coefficient was
fitted to improve agreement.

## The decisive result

Five metrics, four mesh levels and two quadrature orders give 40 recorded
native assemblies. The noncritical fixtures all fail the predeclared
all-link convergence criterion. Values below use quadrature order six.

For Berger `(3,1,1)`:

| Tetrahedra | Relative action-density error | Relative full-flow error |
|---|---:|---:|
| 600 | 24.8213% | 17.2137% |
| 7,200 | 5.34751% | 17.1581% |
| 86,400 | 1.03591% | 26.9455% |
| 1,036,800 | 0.192224% | 37.3101% |

The action approaches its continuum value while the full gradient does
not satisfy the required refinement behavior. Reporting only the action
or a projected curve would therefore give the wrong conclusion.
For the finest Berger fixture, the mass-norm change between quadrature
orders four and six is `1.3835693543e-7`, compared with the full continuum
discrepancy `12.6508267035`. This measured quadrature change cannot account
for that discrepancy; it is not a rigorous bound on all quadrature error.

At the finest level:

| Metric | Relative action-density error | Relative full-flow error |
|---|---:|---:|
| Berger `(3,1,1)` | 0.192224% | 37.3101% |
| Triaxial `(0.5,1,2)` | 0.181612% | 35.1572% |
| Negative-scalar-curvature `(5,1,1)` | 0.218735% | 48.9747% |
| Held-out `(1.3,0.8,2.1)` | 0.141588% | 36.7116% |

The round fixture is a zero-velocity control, so no relative flow error
is invented for it. Its finest absolute mass-norm error is 0.0233491386.
It cannot substitute for the four noncritical fixtures.

## What the failure does and does not establish

The full Berger error is increasingly outside the nine homogeneous
connection modes. The homogeneous component falls from 5.35658537 to
0.02466924, while the orthogonal component reaches 12.65080265. Those
components are measured with the same mass matrix, and their squared
norms add to the squared total norm.

The mesh also has worsening observed shape quality. Neither observation
alone proves a unique cause. This failure does not prove that no lattice
scheme, or no evolved lattice trajectory, can converge to the continuum.
It establishes that this candidate does not meet the specified strong
instantaneous-operator gate. Space/time integration was not promoted past
that failed gate.

See `RICCI_LATTICE_RESIDUAL_ANALYSIS.md` for the separate consistency-norm
issue, measured shape quality, and the next controlled experiments. The
original acceptance threshold has not been weakened.

## Verification and evidence

- 17 native kernel tests pass. Behavioral RED logs precede each new kernel.
- 18 Python tests cover mesh construction, orbit mapping, independent
  matrix derivatives, full-cover assembly, scaling, and evidence refusals.
- Nine isolated native mechanism-removal mutants are detected by actual
  test failures, not compilation errors.
- Ten evidence-tampering cases plus malformed/missing/false-proof inputs
  are refused. An incomplete linear solve is refused. Stale PASS and stale
  verification receipts are removed before checking evidence.
- Complete all-target regression: default **1,622 passed, 24 ignored**;
  Halcyon **2,303 passed, 40 ignored**; zero failures in both.

The numerical correspondence test is separate from those green software
tests. Its failure cannot be hidden by their counts.

Evidence root: `output/ricci-lattice-20261117/`.
The raw 40 cases are in `four-level-operator/`; `diagnostic.json` records
input/output hashes and numerical values. The fail-closed verifier replays
the pinned native executable, reconstructs the manifold fixtures, recomputes
the independent comparisons, and writes `VERIFIED_DIAGNOSTIC.json` only
after success. **All 40 native replays matched byte for byte**, and the
independent numerical comparisons reproduced the saved results. The final
artifact records `evidence_valid: true`, `operator_numeric_gate: FAIL`,
`correspondence: NOT_ESTABLISHED`, and `release_gate: OPEN`.
The `--assert-operator` command exited 1 with the named BW05 failure.
No `gate.PASS.json` remains. A verified failed experiment is not a passed
correspondence gate.

Spatial time evolution, a native time integrator, an analytical stability
and continuum-limit proof, and public server integration remain uncompleted.
They are not hidden behind the successful kernel/regression counts.

Source baseline: `0a64e2f848693221bf25f330ea4fb19041249106`, with uncommitted
working-tree changes. This is not a committed release. Source hashes are
included in the run and verification receipts.

Native operator executable SHA-256:
`71fb30dbbb5fbfcd01e7a516f72849bef9303cbae8b046f7fbd2fe7824b196ad`.
A byte-identical retained executable is in `bin/ricci_operator_probe.exe`
under the evidence root. It avoids relying on a subsequently rebuilt cache.

## Reproduce

From the GIGI repository in PowerShell:

```powershell
$env:CARGO_PROFILE_DEV_OPT_LEVEL='1'
$env:CARGO_PROFILE_DEV_DEBUG='0'
cargo build --features halcyon --example ricci_operator_probe
python e2e/probes/ricci_lattice_convergence.py --binary target/debug/examples/ricci_operator_probe.exe --output output/rf2b-reproduction --max-level 3 --orders 4 6
python e2e/probes/ricci_lattice_evidence.py output/rf2b-reproduction --assert-operator
```

The final command must exit unsuccessfully for the current candidate:
the evidence can verify while BW05 fails. A successful exit here would
require investigation, not a claim that the correspondence was proved.

To replay the retained run rather than rebuild it:

```powershell
python e2e/probes/ricci_lattice_evidence.py output/ricci-lattice-20261117/four-level-operator --binary output/ricci-lattice-20261117/bin/ricci_operator_probe.exe --assert-operator
```

The verifier also checks source hashes. Changed source is an explicit
refusal, not permission to label an earlier receipt current. No general
Ricci-flow theorem, surgery result, or continuum-limit proof is claimed.
