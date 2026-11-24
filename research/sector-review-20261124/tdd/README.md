# Sector review: focused TDD evidence

Review target `0bc5bd4d`. No held-out coupling is evaluated by this suite.

- `upstream-baseline.txt`: all 24 imported tests pass, with zero skips. This does not cover the review defects.
- `red-source/` and `red.txt`: retained pre-fix code, behavioral stubs for new boundaries, and tests;
  10 tests produce 14 assertion failures (including subtests), zero unexpected errors.
- `test_sector_review.txt`: 13 passing review tests, including negative exterior Gram refusal, retained
  eigensolver error, tiny residual preservation, frozen inputs, common allowance and tampered evidence.
- `test_ym_helike_j0.txt` and `test_ym_helike_j2.txt`: 12 + 12 passing imported tests after the corrections.
- `test_quantum_ordering.txt` and `test_quantum_order_controls.txt`: 22 + 10 existing tests pass unchanged.
- `mutations.json`: all 19 deliberate removals cause assertions, not unrelated errors. The original 13 scalar
  and tensor mutations are exercised by targeted tests, plus six review-mechanism removals.
- `green.json`: exact source hashes and gate status. Total: 69 focused tests, no skips. No Rust suite.

One initial mutation removed the global exterior-PSD check but was still caught by the selected-state negative
contraction check. It therefore did not establish that the global check was necessary. The strengthened fixture
places the negative direction outside the reported states; that mutant now fails by assertion. This correction
does not change the numerical policy or accept a negative residual.

The runner repeats the prerequisites and records a separate set of receipts before numerical measurements.
The original operator identities and frozen couplings/widths remain unchanged.
