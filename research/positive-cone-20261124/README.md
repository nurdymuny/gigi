# Positive-cone basis controls

Baseline `9c17cc33`. Spec written before implementation. Preserve the
negative-kinetic RED witness, focused control logs and mechanism removals
here. No larger registered run is authorized by these receipts.

## Before the Mq=6 refinement

The first two direct domain-witness tests failed on the legacy linear map,
then passed on the log-Cholesky chart. All six geometry and four moment/
reference-transform tests pass. Mq=4 does NOT pass the unchanged control
gate: the extrapolated competitor coefficient error is 0.041367 > 0.03.

The initial logarithmic-coordinate weights were not the original reference
observable. The corrected fixed linear-Gram projectors give competitor
weights 0.9812470 and 0.9907386, which pass 0.95; basis-coordinate weights
are retained separately. The 8/10 overlap rule refused at 1.36e-6; 10/12
resolves the same observable to 1.70e-7 and 2.37e-8 without changing the
1e-6 requirement. All earlier failures remain recorded.

This is an experimental source checkpoint, not a closed physical gate.
The Mq=6 refinement of the SAME two weak-coupling controls is specified
before its first run. Commit and push this source first. The full physical
grid stays held; the singular-boundary realization is unresolved.

Earlier development receipts retain matrices and source hashes, but not
complete source snapshots. Subsequent clean-commit runs retain both.

## Refined controls and final verification

`controls-ad4536e4/receipt.json` records `PASS_CONTROLS` on the same two
backgrounds, with Mq=6. Source/spec commit `ad4536e4` was pushed before that
run. The receipt has clean declared sources, source snapshots, binary-hashed
matrices and saved eigenvectors. Extrapolated coefficients are -0.5308701642
and -2.7511293048; both pass the unchanged 0.03 error requirement. Original
reference weights all exceed 0.95. Log-coordinate weights remain distinct.

`green.txt` records 18 new focused tests, including independent kinetic
integration and saved-matrix/weight replay. `mutations.json` and `mutations.txt`
record all eight deliberate defects caught by assertions, not test crashes.
The existing focused regression logs record 24 refusal/evidence tests and
four perturbative tests. These are research-script checks, not a claim that
the entire GIGI test suite ran.

The full physical gate remains open: the competitor gap changes by
0.0034559546 between Mq=4 and 6 at g squared=0.004, above 0.003. Boundary
realization and exact charge-sector identification remain unresolved. No
larger registered grid ran. See the
[full results note](../../docs/research/helike-positive-cone-basis-results-2026-11-24.md).

From the repository root, with NumPy, SciPy and SymPy installed:

```powershell
$env:OPENBLAS_NUM_THREADS='1'
python -m unittest discover -s scripts -p test_helike_positive_cone.py -v
python -m unittest discover -s scripts -p test_flux_review.py -v
python -m unittest discover -s scripts -p test_helike_condensate_pt.py -v
# Use a new scratch directory; the controls refuse a nonempty output folder.
New-Item -ItemType Directory -Path tmp/cone-replay
python scripts/check_positive_cone_mutations.py tmp/cone-replay/mutations.json
python scripts/run_helike_positive_cone_controls.py tmp/cone-replay/controls --cutoff 6
```

The saved-state tests replay the committed `controls-ad4536e4` artifacts.
The last command makes a new run with its own source revision and hashes;
it does not overwrite historical evidence. `saved-state-replay.txt` is an
earlier standalone replay log; the final combined run is `green.txt`.
