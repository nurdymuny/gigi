# Review of the flux background implementation

Review target: `0b330274`, pulled after `24356819` on 24 November 2026.
This review runs the newly added focused tests and adversarial controls.
It does not authorize or consume the expensive registered physical grid.

Review priorities are the mathematical reduction, approximation domain,
level identification, convergence decisions, failure handling and retained
evidence. Any execution-contract correction must be documented before a
future registered run; the original registration is retained as history.

Test output and behavioral RED/GREEN receipts are saved here. Engine code
and routes are unchanged; no full Rust suite is claimed.

## Reproduce the focused checks

From the repository root, with the existing NumPy/SciPy/SymPy environment:

```powershell
$env:OMP_NUM_THREADS='1'
$env:OPENBLAS_NUM_THREADS='1'
$env:MKL_NUM_THREADS='1'
python -m unittest discover -s scripts -p 'test_helike_*.py' -v
python -m unittest discover -s scripts -p 'test_ym_two_grain_*.py' -v
python -m unittest discover -s scripts -p test_flux_review.py -v
python scripts/check_flux_review_mutations.py OUT/review-mutations.json
```

Create the output parent directory first. The seven supplied mutation
scripts are `check_helike_curl_mutations.py`,
`check_helike_berger_mutations.py`, `check_helike_condensate_mutations.py`,
`check_helike_condensate_pt_mutations.py`,
`check_helike_condensate_fock_mutations.py`,
`check_ym_two_grain_molien_mutations.py` and
`check_ym_two_grain_mutations.py`, all under `scripts/`. Each accepts an
output JSON filename. A surviving mutant or an incidental exception is
a failed mutation gate.

## Evidence interpretation

- `baseline-*.txt`: supplied tests before implementation corrections.
- `review-red*.txt`, `runner-red.txt`: the deliberately failing controls.
  The refined RED separates an ambiguous-label failure from an indexing
  crash; it records 11 assertion failures and one malformed-length error.
- `green-*.txt`: 49 + 25 + 24 passing focused tests.
- `*-mutations.json`: exact mechanism-removal outcomes.
- `domain-witness.json`: direct evaluated negative-kinetic witness, not
  a physical point admitted by G=qq^T.

The small planted gaps printed by `test_flux_review.py` exercise reporting
and refusal behavior. They are deliberately synthetic test values, never
experimental measurements. The supplied mathematical suites perform their
actual numerical constructions. See the
[review report](../../docs/research/helike-flux-review-results-2026-11-24.md)
for the scientific scope and open domain requirement.

Final focused result: **98 tests pass; 43 of 43 deliberate mechanism
removals are caught by assertions, with zero incidental test errors.**
This is a software/mathematical-control result, not closure of the physical
flux-background gate.

## Actual runner check

After source `384e1226` was committed and pushed:

```powershell
python scripts/run_helike_flux_background.py OUT/smoke --smoke
python scripts/run_helike_flux_background.py OUT/smoke --verify
python scripts/run_helike_flux_background.py OUT/hold
```

The first two commands returned 0. The final command intentionally returned
2 with HOLD_DOMAIN and zero cases. Saved folders `smoke-384e1226/` and
`hold-384e1226/` include the exact text snapshots and the declared portable
hash policy. The smoke is one-rung, low-order and uncertified; competitor
weights do not meet the registered 0.9 threshold. Its local ordering change
does not close the physical gate. Replay checks receipt consistency only.

The original failing unittest logs retain their emitted whitespace,
including three trailing-space lines flagged by `git diff --check`.
