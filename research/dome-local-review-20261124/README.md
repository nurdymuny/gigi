# Local geometry and finite reference audit evidence

Review target: `f4807710`, including `613e6e25`. The mathematical experiment
is specified in [the protocol](../../docs/research/dome-local-review-protocol-2026-11-24.md).
This directory preserves tests and the bounded replication, not a new
weak-coupling excitation experiment.

## Focused gates

The initial diagnostic failed four behavioral tests: reversed orders,
unresolved exterior denominators, negative projected squared norms and
missing provenance. A later evidence-replay test also caught an unchecked
cancellation disclosure. RED logs are retained, not replaced by GREEN logs.

The final focused suite contains 7 truncation tests, 11 local-geometry and factorization tests
and 12 evidence/runner tests. Eight mechanism-removal checks must each fail
by assertion, not by an import error or unrelated exception. The small
pipeline fixture is an engineering test, not a measurement of the held-out
physical couplings.

The 11 existing dome-mathematics tests also pass unchanged: 41 focused tests
in total. All nine sources frozen by the earlier registered dome run retain
their recorded SHA-256 hashes.

From the repository root, with Python dependencies already installed:

```powershell
python -m unittest discover -s scripts -p test_ym_helike_dome_truncation.py -v
python -m unittest discover -s scripts -p test_ym_helike_second_vacuum.py -v
python -m unittest discover -s scripts -p test_dome_local_review.py -v
python scripts/check_dome_local_review_mutations.py research/dome-local-review-20261124/gates/mutations.json
```

See `gates/` for captured commands' output. No engine source or route was
changed, and no full Rust suite is claimed.

## Bounded numerical replication

After committing and pushing the protocol and source, run into a new directory:

```powershell
python -u scripts/run_ym_helike_dome_local_review.py research/dome-local-review-20261124/audit-SOURCE
python scripts/run_ym_helike_dome_local_review.py research/dome-local-review-20261124/audit-SOURCE --verify
```

Replace `SOURCE` with the committed source's short revision. The runner
refuses a nonempty directory. It records the full source revision, exact
source snapshots, Python environment, batch metadata, raw norms, overlaps,
Gram matrices, progress events, worker output and sampled resources.

The frozen design has eight batches and 32 rows, one CPU thread, a 45-minute
limit and an 8-GiB memory limit. Existing couplings only; no eigensolve.
`verification.json` checks evidence replay. Its PASS does not certify an
infinite tail, a global spectral gap or the next physical experiment.

The old dome experiment and its `UNRESOLVED_TRUNCATION` verdict remain
unchanged. The future normal-envelope construction, joint-span and
Hamiltonian graph-norm checks remain separate work.
