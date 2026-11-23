# RF2C: shape-regular mesh experiment

Date: 2026-11-17. Numerical outcome: **NO-GO for BW05**.
Evidence replay: **8/8 exact native replays**. Focused checks: **24 passed,
0 failed, 0 skipped**, including adversarial evidence and mechanism-removal tests.
Correspondence remains NOT ESTABLISHED; release gate remains OPEN.

## Question answered

Does removing the old mesh's deteriorating shapes, without changing the
mapping or the numerical acceptance criteria, fix the full link-flow error?

**Not on the prescribed four-level experiment.** The new family has a
level-independent shape bound, and all four measured meshes pass the fixed
quality cap. Nevertheless, the finest Berger relative full-flow error is
25.5066%, not below the required 5%. The last two observed orders are
negative, not at least 0.5.

This rules out mesh-quality replacement alone as a sufficient fix in this
experiment. It is not a theorem of nonconvergence, does not rule out every
other refinement, and does not identify which remaining operator mechanism
causes the discrepancy. No constants, clocks, targets or thresholds were fit.

## What stayed fixed

- Same S3 cover, binary-icosahedral left quotient and Berger metric `(3,1,1)`.
- Same metric-to-connection mapping and exact short-path links.
- Same native action, every link-gradient component and full tangent mass.
- Same independently computed continuum reference and volume-normalized mass norm.
- Same CG solve, physical time scale, quadrature orders 4/6, and BW05 thresholds.
- Same native executable, SHA256
  `71fb30dbbb5fbfcd01e7a516f72849bef9303cbae8b046f7fbd2fe7824b196ad`.

Only the mesh construction and its input adapter are new. The adapter matches
the old adapter's entire output exactly on old levels 0 and 1. A relabeling
of the new coarse mesh plants reversed deck edges without changing geometry;
the full-cover assembly then agrees with the reduced assembly, and removing
the reversed-edge adjoint is detected. Physical metric scaling is also tested.

The diagnostic uses the retained native Rust assembly executable and the
unchanged external SciPy solver/oracle. This is not a newly deployed GQL verb
or a live-server claim. The repository HEAD is
`0a64e2f848693221bf25f330ea4fb19041249106`; it is a dirty worktree, so the
manifest additionally binds the actual source and executable hashes.

## Mesh result

The frozen [specification](RICCI_SHAPE_REGULAR_SPEC.md) defines one affine
barycentric flag subdivision followed by edgewise refinement at n=1,2,3,5.
Integer barycentric node identities are used before a single radial projection.
It gives a uniform chord diameter/inradius bound below 43, independent of n;
the stricter predeclared measured cap is 14. The bound also controls the
nondegeneracy of the unchanged radial cell chart.

| n | Cover tetrahedra | Maximum angular edge h (radians) | Worst diameter/inradius | Quotient link DOF |
|---|---:|---:|---:|---:|
| 1 | 14,400 | 0.3881395154 | 9.955009 | 426 |
| 2 | 115,200 | 0.2016619534 | 10.759009 | 3,372 |
| 3 | 388,800 | 0.1354592602 | 10.787264 | 11,358 |
| 5 | 1,800,000 | 0.0815947846 | 10.810998 | 52,530 |

Every level checks closed oriented face incidence, connected cover and vertex
links, Euler counts, exact node identity, and free 120-element deck orbits.
The 301,200 vertices of the finest cover are checked; its full lifted link
space has 6,303,600 components. The quotient uses all its link components,
not a nine-mode projection.

The old finest mesh had h=0.0826274696 and worst shape ratio 19.670347.
Thus the endpoint spacings are close, but the meshes have different cell and
DOF counts. This is not a matched-cost performance comparison.

## Full-flow result

Numbers below use quadrature order 6 and the unchanged independent oracle.

| n | Absolute full mass error | Relative full-flow error | Relative action error |
|---|---:|---:|---:|
| 1 | 8.5464514386 | 26.245894% | 4.654073% |
| 2 | 8.3069497153 | 24.744196% | 0.829461% |
| 3 | 8.4299282389 | 24.958594% | 0.323853% |
| 5 | 8.6420526517 | 25.506615% | 0.104666% |

Observed orders use actual measured h, not an assumed halving:
`0.04341004, -0.03693132, -0.04902663`.

BW05 requires both final orders >=0.5 AND final relative error <0.05.
Both conditions fail. The old finest relative error was 37.310095%; this
replacement reduces the endpoint discrepancy but does not resolve it.
The more accurate action value cannot substitute for the failed full-flow gate.

The original mass solves converge in 35,48,53,58 iterations at order 6;
their recomputed relative linear residuals are below 1e-12. The verifier also
measures order-4 versus order-6 velocity differences in the full mass norm
and separately estimates solve error by a residual-correction solve. This
diagnostic correction does not change any reported velocity or tolerance.

| n | Order-4/order-6 full mass difference | Linear correction mass estimate |
|---|---:|---:|
| 1 | 7.9027409e-6 | 2.0258068e-11 |
| 2 | 1.0585917e-6 | 3.5948537e-11 |
| 3 | 2.4765590e-7 | 3.2850281e-11 |
| 5 | 3.6575110e-8 | 3.8059752e-11 |

Both numerical error estimates are well below the required one-tenth of the
spatial discrepancy at every level. They do not explain the failure. The
quadrature comparison is an estimate, not an all-orders certified bound;
the linear correction is likewise a diagnostic estimate, not an interval proof.

## Evidence and gates

Evidence directory: `output/ricci-shape-regular-20261117/berger/`.

- `manifest.json`: frozen RF2B sources, original evidence, RF2C sources, spec,
  binary and exact experimental contract. Preparatory mesh unit tests preceded
  this manifest; the complete four-level construction and all new flow runs
  followed it. RF2B source hashes match the earlier archived run.
- `diagnostic.json`: all four mesh certificates and eight measured cases.
- `level*_berger_q*.input.json` / `.output.json`: exact native requests/results.
- `VERIFIED_DIAGNOSTIC.json`: written only after regeneration, native replay,
  independent numerical recomputation and source rechecks succeed.
- `FOCUSED_TESTS.json`: 24 focused checks, source hashes and commands' scope;
  its PASS concerns those checks only, never the failed numerical/release gate.
- `logs/`: retained behavioral RED logs, subsequent GREEN logs, native run,
  replay, planted-reversal fixture correction and final focused test output.

Adversarial cases reject missing levels, altered thresholds, invented proof
status, changed or removed source identities, changed binary identity, incorrect
mesh counts, invented zero errors, altered native gradients and partial runs.
Each attack starts with fake PASS receipts and verifies their removal. Removal
of the oriented-complex check and of the provenance guard is separately shown
to turn the corresponding tests red. The planted reversal test also detects
omission of the reversed-edge adjoint in the lifting rule.

All eight replay cases use the actual native executable. To avoid repeating
eight complete runs per adversarial test, those tests use the real executable
for the attacked coarse case and copy the already-replayed unaffected outputs;
they do not mock mesh reconstruction, numerical recomputation or source checks.
This distinction is explicit in `test_ricci_shape_attacks.py`.

The first complete geometry-plus-eight-case diagnostic took 212.76 seconds
on this machine. That excludes preparatory tests and the independent replay;
it is a run receipt, not a general benchmark.

| Gate | Status |
|---|---|
| SR00: source/binary/threshold freeze | PASS for the recorded experiment |
| SR01: exact reference subdivision and shape argument checks | PASS |
| SR02: four complete mesh certificates | PASS |
| SR03: old-adapter equality, native cover/quotient and scale checks | PASS |
| SR04: four-level Berger full-flow acceptance | FAIL |
| SR05: other four metrics | NOT RUN: staged fast-fail rule |
| SR06: replay, removal checks and adversarial evidence | PASS |
| SR07: full default/Halcyon regressions | DEFERRED at user request |

No commit or deployment. Existing RF2B evidence is preserved unchanged.

## Reproduction

From the GIGI repository root, use a fresh output directory:

```powershell
$env:OPENBLAS_NUM_THREADS='1'
$env:OMP_NUM_THREADS='1'
python e2e/probes/ricci_shape_experiment.py output/ricci-shape-reproduction/berger
python e2e/probes/ricci_shape_evidence.py output/ricci-shape-reproduction/berger --assert-operator
python scripts/ricci_shape_focused_gate.py output/ricci-shape-reproduction/berger
```

The second command must exit nonzero for this numerical outcome, while
retaining a receipt that says the evidence is valid and the numerical gate
failed. It must never leave a release PASS. The retained native binary must
be present at the pinned path; replacing it is refused.

## Smallest justified next investigation

Keep this shape-regular family and diagnose the remaining operator residual:
separate weak consistency from full mass-inverse flow consistency, and test
the local transport/ordering behavior with independently planted fields.
Do not alter the continuum correspondence, time scale or acceptance criteria
on the strength of these data. A separate spec and gate are needed before
changing the operator. Improved mesh quality alone did not close BW05.
