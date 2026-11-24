# Cartesian boundary matching evidence

24 November 2026. Baseline `351007b0`. No larger spectral run.

Specification: [boundary spec](../../docs/research/helike-qspace-boundary-spec-2026-11-24.md).
The original Cartesian registration fixes regularity across the generic
rank-two interface; it does not select an autonomous wall at det(q)=0.

## Checkpoint tests

- `contracts-red.txt`: the tests were written first and failed because the
  new interface module did not exist. This is an API RED, not a demonstrated
  numerical defect in the old code.
- `first-implementation.txt`: first 15 boundary tests passed.
- `green.txt`: 16 focused tests pass, including direct Cartesian operator
  oracles and regular invariant probes at the same two control couplings.
- `mutations.json` and `mutations.txt`: nine deliberate defects cause assertion
  failures, with zero incidental errors. Dirichlet and Neumann counterexamples
  are recorded here as well as wrong orientation, measure, metric, drift,
  signed cubic, value matching and half-normal sign.
- `interior-controls-regression.txt`: all 18 existing positive-cone tests
  pass, including the unchanged coefficient/weight controls and saved-state
  replay. These remain interior tests, not completed physical spectra.

## Reproduce

From the repository root, with NumPy, SciPy and SymPy installed:

```powershell
$env:OPENBLAS_NUM_THREADS='1'
python -m unittest discover -s scripts -p test_helike_qspace_boundary.py -v
python -m unittest discover -s scripts -p test_helike_positive_cone.py -v
New-Item -ItemType Directory -Path tmp/boundary-replay
python scripts/check_qspace_boundary_mutations.py tmp/boundary-replay/mutations.json
python scripts/run_helike_qspace_boundary.py tmp/boundary-replay/controls
```

Use a new output directory. The runner refuses nonempty evidence directories,
retains declared source snapshots and writes each control before proceeding.
`PASS_LOCAL_MATCHING` cannot release the registered grid or certify a spectrum.
The previous 0.0034559546 versus 0.003 refinement failure is unchanged.
