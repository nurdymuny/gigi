# Bounded review of the incoming two grain potential

Review target: `52146ec0`. This arrived during the separately registered
projected-tail audit. It does not change that audit's source or measurements.

The incoming 11 tests pass. Review additions check the frame derivative
against group motion, independent colour and spatial rotations, and the
nontrivial zero-cross-term case where both families share one colour axis.
The bidegree diagnostic must refuse an underdetermined request rather than
call a least-squares fit exact: its five-by-five sampling grid supports the
specified degree-four model only. Demonstrate this refusal RED before fixing
it and remove the guard in a mutation afterward.

This is a finite-potential and local-geometry review, not a spectral run.
No new held-out physical coupling or eighteen-mode excitation is measured.
The published quadrature rule can be algebraically exact for its degree
class while its NumPy evaluation remains floating point. A local zero-point
offset is not a measured global excitation gap. The full-theory gauge-copy
comparison needs a common regulated operator and boundary domain; it is not
a computed zero in these tests.

Commands, from the repository root:

```powershell
python -m unittest discover -s scripts -p test_ym_two_grain_potential.py -v
python scripts/check_ym_two_grain_mutations.py research/two-grain-review-20261124/mutations.json
```

Retain RED, GREEN and mutation logs here. The earlier dome audit remains in
`research/dome-local-review-20261124/audit-93b3b569`.

## Outcome

Fifteen tests pass; five deliberate defects are rejected by assertions.
The degree-five request first failed its refusal test, then passed after
the fixed-grid contract was enforced. The baseline physical formula and
degree-four results are unchanged. New geometric checks pass without
changing the frame derivative or curvature formula.

The prose now records the nonzero-family, zero-interaction counterexample;
distinguishes finite floating-point checks from exact arithmetic; and keeps
the approximately 3.37 local harmonic offset separate from a global
excitation gap. A nonnegative-decomposition argument supplies the vacuum
exhaustion missing from a test that only samples the known vacua.
