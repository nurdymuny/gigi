# Focused test receipts

23 November 2026, Pacific time. These are software checks, not a crossing or continuum result.

- `red-source/`: initial behavioral stubs and their tests, retained without overwriting.
- `red.txt`: 19 tests, 20 assertion failures including subtests, zero unexpected errors.
- `runner-red.txt`: six tests, eight assertion failures including subtests, zero unexpected errors.
- `identity-red.txt`: the additional array-identity check fails before its implementation.
- `ordering_tests.txt`: 22 passing mathematical tests, including direct ladder-action comparison.
- `runner_tests.txt`: 10 passing evidence/resource tests, including recomputation after coherent tampering
  with both the reported residual and its derived ordering, and corruption of a saved residual Gram matrix.
- `original_tests.txt` and `amendment_tests.txt`: 20 and 31 passing existing tests respectively.
- `mutations.json`: 18 deliberate removals; every one rejected by an assertion, with zero unexpected errors.
- `green.json`: gate results and exact source hashes at the green run.

No full Rust suite was run: the engine and HTTP endpoints are unchanged. The numerical control pilot has its
own source-pinned run directory and repeats these prerequisites before measurement.

## Fixture corrections during development

The initial half-separation fixture tested an unused bound, so its expected unresolved result was wrong.
It was corrected to make that bound necessary to the requested inequality; the final half-guard mutant is
rejected. This did not require weakening the ordering rule.

The first source-check mutant hit missing mock evidence rather than an assertion. Its fixture now uses the
integrity-only path with complete mocked identities to isolate source validation. The first residual-replay
mutant was caught by a separate ordering-consistency check, which left that test passing. The fixture now
alters the derived ordering too, proving that recomputing the saved state, not just comparing two JSON fields,
is necessary. Neither initial mutation result counted as gate evidence.

The retained red-source tests reflect their original state; the green tests include these documented corrections
and additional regressions. No held-out coupling was measured by this suite.
