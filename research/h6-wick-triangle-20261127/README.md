# Wick triangle: exact evidence and publication record

Baseline commit: `36eb3b4f9e240f2fa3d226dc11f2111dcf3a8c6d`.
That commit identifies the incoming note, not the new uncommitted implementation.
The receipts' LF-normalized source hashes identify the code actually checked.

## Result and scope

The original H6 Step 6 prediction that the leading free triangle is nonzero is
**refuted**. The all-configurations Hodge argument gives zero at pairwise distinct
points of free Euclidean R4. The documented four-point alternative has connected
coefficient `676 n / (27 pi^8 L^16)` at `0, L e0, 2L e0, 3L e0`, with
`n = dim(g)` and canonical `O = sum_{a,mu,nu} :F_mu,nu^a F_mu,nu^a:`.

This settles one existing sub-conjecture against its prediction. It does not
close H6 or any of the seven top-level hypotheses. No new auxiliary lemma is
counted as one of the originally documented conjectures. Nonzero composite
correlations do not alone establish non-triviality of the full theory.

## Math gate

- `red/`: the tests preceded implementation. Nine engine calls failed with
  the expected NotImplementedError, the missing proof failed its document gate,
  and all five Lean declarations were absent. `passed` is false, with an explicit
  `expected_red_observed` marker.
- `algebra-development.log`: nine exact tests passed before the document gate.
- `release-01/`: rejected by a case-sensitive prose marker (`contact terms`
  versus `Contact terms`); it remains a failed receipt. The checker now normalizes
  prose case and whitespace, not mathematical data.
- `release-02/`: ten checks pass. Four actual source mutations are rejected by
  assertion failures, not just import/runtime errors: a missing derivative term,
  always-zero output, wrong observable scaling, and missing pairing.
- The same release builds five finite-matrix Lean declarations and audits their
  dependencies. Only `propext`, `Classical.choice` and `Quot.sound` are allowed.
  Three proof mutants fail: remove the third exchange, remove the involution,
  or reverse the conjugation sign. Two poisoned axiom reports are also rejected.
- `mathlib-cache.log` and `lean-development.log`: local dependency preparation
  and compiler evidence. The unused-section-variable warning is a linter warning,
  not an admitted proof. The exact pinned versions are in the release receipt.

The proof note spells out the formalization boundary. Lean checks the trace
theorem with explicit hypotheses; it does not construct Maxwell or Yang--Mills
quantum field theory. The independent derivative/reflection covariance formulas,
Wick enumeration, and nonzero two/scalar-three/Maxwell-four controls are exact
SymPy checks, not floating-point samples used to infer a universal theorem.

## Publication gate

`paper-before.tex` and `paper-after.tex` freeze the one authorized replacement.
`publication-red` records the old preservation gate rejecting it. The amendment
helper recognizes that exact passage and leaves H6 and all unrelated claims
unchanged. Final preservation tests also reject a wrong coefficient, a modified
baseline and an unauthorized status promotion.

`pdf-build-01.log` through `pdf-build-04.log` retain the successful build history.
The first revision spilled two lines onto an otherwise empty page. Availability
prose was condensed; the final PDF has 27 pages and no LaTeX warnings or overfull/
underfull boxes. The mathematical passage is unchanged by that layout repair.
The `final-page-*.png` files are renders of pages 19-27 inspected for layout,
including the new calculation, conditional status, provenance, tables and references.
An initial shell invocation used a nonexistent log path and did not provide a
valid build record; the four logs here use the corrected absolute path.

Final document receipts live under `final-format`, `final-review` and
`final-doublet`. These are preservation/replay checks, not proof certificates for
the central conjecture. `publication-01` is an intermediate check against the
earlier PDF; use the final receipts for the delivered artifact.

## Reproduce

Use fresh output directories, from the repository root:

```text
python scripts/check_h6_wick_triangle.py <new-math-directory>
python scripts/check_lowest_note_format.py <new-format-directory>
python scripts/check_lowest_note_review.py <new-review-directory>
python scripts/check_lowest_note_doublet.py <new-doublet-directory> --artifacts
```

Build `docs/papers/ym-theory-statement/statement.tex` with pdflatex until references
stabilize before the artifact checks. The checker refuses an existing output
directory instead of overwriting a receipt or presenting an old PASS as a new run.
No large eigensolve, simulation, GPU run or full engine test suite was used.
