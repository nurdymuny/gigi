# Convergence amendment: corrections and review hold

23 November 2026. Review branch: `feat/helike-quantum-convergence-20261123`.
Corrects the draft introduced at `57d20b67`. **The registered convergence survey has not been run.**
This commit is for review on another machine before that run is authorized.

## What changed

1. Resolve the Casimir spectrum inside each complete energy level. Mixed-spin levels and repeated spin copies
   are retained, not rounded into a single spin. Incomplete multiplets, unresolved Casimirs, leakage between
   energy levels, a non-scalar vacuum, and a wrong-rank probe response are refused. A measured weight of one
   can no longer be called rounding of an exact zero by the mixed-spin counterexample.
2. Transform the physical probes along with the Hamiltonian: H2(x)/alpha and H0(x)/alpha^2.
   At alpha=2, the old unscaled squared response norms were too large by factors four and sixteen. The
   normalized weights were unaffected; this correction matters for absolute kernels and cross-width comparisons.
3. Require HP11--HP13 and runner/mutation tests before alpha selection. Convergence additionally requires
   validated spin, solver agreement and replayed evidence for each case, plus the unchanged HP08 checks.
4. Label floating-point spectral intervals `NUMERICAL_ESTIMATE`, `certified: false`. Include energy-group spread
   in the common-center residual radius. Apply the stated upper-neighbour guard to the ground state too.
   Kato--Temple estimates remain conditional on a true spectral separation that has not been certified.
5. Store H, S, J2, the physical probe matrices, eigenvalues, eigenvectors, alpha and coupling in hashed NPZ files.
   Save each completed case immediately. Pin the code, amendment, tests and verifier in a source snapshot.
   Replay verifies spectra, physical responses and spin labels; full replay also rebuilds the operators and exterior
   action. Source drift, corruption, missing gates, incomplete histories and forged convergence labels are refused.
6. Restore the 45-minute / 8-GiB host supervisor and 4-GiB CUDA allocation limit. Check storage estimates before
   constructing a basis; retain only one assembled cutoff. Failure clears prior convergence labels. The host
   limit is sampled, not an allocation-level hard cap. CPU fallback remains forbidden.
7. Correct the narrative: width mismatch is a hypothesis, not a demonstrated cause. Alpha above one narrows the
   Gaussian in the original coordinates. The pre-amendment N=8 ground-energy observation is explicitly disclosed.

## What did not change

- Published model, coordinate map, couplings, observable definitions, alpha grid and ground-energy-only selection.
- Cutoff sequence and numerical acceptance thresholds.
- Original survey results, paper, figures, and original survey runner.
- No GIGI engine changes, deployment, new large-cutoff data, or new convergence claim.

`COMPLETE` means that a requested sequence finished; it is not a PASS for the original study. This amendment
does not execute the HP09 usefulness comparison. That gate remains open even if numerical convergence succeeds.
The original evidence retains its saved source snapshot; use that snapshot when replaying the historical survey.

## Focused verification

The new tests were introduced before implementation. The first 22-test run reported 16 failures and eight errors
(including subtests); it reproduced the mixed-spin false zero, missing probe scaling, ground-state guard bypass,
missing certification labels, and absent runner safeguards. The original alpha=1 checks already passed.

After correction:

| Check | Result | Scope |
| --- | --- | --- |
| New regression suite | 29 passed | Small CPU fixtures, at most N=4 actual model construction |
| Existing CPU regression subset | 14 passed | Original basis, probes and receipt checks; includes an N=6 gauge check |
| Mechanism-removal checks | 15/15 rejected | In-memory defects; pristine functions restored after each test |
| Registered width/convergence survey | NOT RUN | No N=10 alpha selection or N=8--20 convergence sequence launched |
| CUDA execution and live resource peaks | NOT RUN in this correction | Runtime safeguards implemented; supervisor stop tests use mocks |
| Full GIGI test suite / live engine | NOT RUN | No engine changes in this patch |

Receipts: [correction-checks](../../research/helike-quantum-probe-20261123/correction-checks/).
The mutation experiment initially exposed a weak wrong-rank test: a second guard caught the error even when
the rank check was removed. The fixture now places allowed and forbidden spin components in the same energy
level; removing the rank check then makes its own gate fail.

The small alpha=2 probe-normalization fixture is an implementation check, not a width-selection survey. It is
disclosed here so the evidence seen before a later survey is not understated. No decision rule was retuned to it.

## Review and later execution

Review the corrected amendment, the three calculation/reporting functions in `ym_helike_quantum.py`, the new
runner, and the regression/mutation tests. Reproduce the focused checks without launching the survey:

```text
python -m unittest discover -s scripts -p test_ym_helike_convergence.py -v
python scripts/check_ym_helike_convergence_mutations.py
```

Only after review, the documented survey command is:

```text
python scripts/run_ym_helike_convergence.py research/helike-quantum-probe-20261123 --device cuda
```

The default cutoff-16 estimate is about 6.86 GiB host / 2.30 GiB CUDA storage. These are estimates, not observed
peaks or a guarantee of completion. Cutoffs 18 and 20 are refused by the present estimates under the unchanged
budgets. Do not increase the limits silently. A future larger run requires its own resource decision.

Full evidence replay, after a run exists, is available through `--verify RUN_FOLDER`. It rebuilds operators and
exterior actions and can be expensive; it is not a constant-time metadata check. Execute it with the saved source
snapshot if the working implementation has since changed. No result from such a replay is claimed in this note.

## Review addendum (second machine, before any survey)

Reviewed on a second machine (macOS, CPU only). The 29-test suite passes except the three supervisor tests, which
need `psutil`, absent there (an environment limit, not a code defect). The mutation checks were all rejected, but
four were rejected for the wrong reason:

| Mutation | Was rejected by | Now rejected by |
| --- | --- | --- |
| `accept_partial_history` | the source-hash guard firing on a valid fixture | its own "partial" assertion |
| `bypass_budget` | a `TypeError` from the mock basis | its own assertion: MemoryError expected, construction not called |
| `skip_source_drift_check` | a `KeyError` from the fixture | its own assertion: the "sources changed" refusal |
| `average_mixed_spin` | the mutated function's integer-spin guard | its own assertion: the mixed level must be resolved |

The root cause of the first three was in the harness: each mutant ran in a copy of its module's globals, so patches a
test applied to the module (source hashes, budgets, basis construction) never reached the mutant, which then failed
on unrelated errors. The harness now rebinds each mutant to the module's real globals, and the two exception tests
and the mixed-spin test catch any exception and assert its type, message or result. After the change all 15 mutants
are rejected by their own assertions (`correction-checks/mutation-checks-review.json`). Please rerun the full suite,
including the supervisor tests, on the GPU machine before the survey; the source hashes of the tests have changed.
