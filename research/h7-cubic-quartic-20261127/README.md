# Cubic-quartic closure: verification record

27 November 2026. Written proof:
`docs/research/h7-cubic-quartic-proof-2026-11-27.md`.
Contract: `docs/research/h7-cubic-quartic-contract-2026-11-27.md`.

## What this record does and does not certify

The mathematical deduction uses Austing--Wheater, arXiv:hep-th/0310170v2,
equation (2.2), followed by an explicit convention translation and scaling.
The scripts check finite exact algebra, declared scope, document preservation
and existing figure artifacts. They do not machine-prove the published global
inequality or the functional-analysis result.

The full H7 conjecture is not promoted. No simulation, eigensolve, solver grid,
engine build or full GIGI test suite ran. These are focused paper/proof checks.

## Sequence

- `red-v2`: 12 checks, 10 pass and two expected document failures, before the
  proof and status edit. The first attempted `red` invocation stopped on a
  Windows stdout encoding error and produced no receipt. That harness issue
  was fixed before the recorded red run.
- `proof-edit`: all 12 checks pass after the proof is written; PDF still old.
- `preservation-red`: eight checks, two expected failures. Existing preservation
  rules reject the new statement/evidence text and the new bibliography entry.
- `preservation-edit`, `review-edit`, `doublet-edit`: intermediate passes after
  narrowly registering the authorized additions. Not the release snapshot.
- `build-1.log`: first LaTeX pass, with expected unresolved new references.
- `build-2.log`: stable 27-page build, no warnings.
- `build-final.log`: final build after distinguishing the cubic-bound constant
  from H7's similarly named gap constant; no warnings or overfull/underfull boxes.
- `release-proof`, `release-format`, `release-review`, `release-doublet`: final
  receipts. Counts 12 + 8 + 26 + 9 = **55 passing focused checks**.

Each receipt includes LF-canonical source hashes and a PDF byte hash. Its
`source_revision` is the pre-commit baseline `1ba75baa762aeab4ca580c6eabe1354a86cb6af3`;
the accompanying source hashes identify the edited working tree actually checked.
Output directories are immutable: the scripts refuse an existing destination.

## Exact and adversarial checks

The new checks cover Hermitian/anti-Hermitian signs, trace normalization,
unordered bracket counting, the epsilon factor of one-half, commuting and
non-regular-limit fixtures, homogeneity, the 27/32 factorization, and the
unchanged positive-kappa window. Five bad contracts are rejected: reversed
cubic sign, omitted bracket, wrong exponent, undersized additive bound and a
promoted full-H7 status. The scalar bound fails at z = 3/2 when its additive
constant is halved. Existing statement and figure mutation controls also pass.

## PDF review

Final PDF: `docs/papers/ym-theory-statement/statement.pdf`, 27 pages.
SHA256: `fbdae866f8f914fdff905239e30d13f2f315fc8b3934e4b826f7a52bbb1a2908`.

All 27 pages were rendered with Poppler. Pages 21-27 were visually reviewed
for the new proof, still-conjectural H7, availability paragraph, table
continuations and references. Pages 21-22 and 26 were checked again after
the final build. A PyMuPDF check found all pages nonblank and every extracted
word box inside its page. The opening-doublet artifact check also passes.
Rendering previews are disposable local files under `tmp/pdfs/h7-cubic-quartic/`.

## Reproduction

From `docs/papers/ym-theory-statement`, run
`pdflatex -interaction=nonstopmode -halt-on-error statement.tex` until references
stabilize. From the repository root, use fresh output paths:

```text
python scripts/check_h7_cubic_quartic.py <new-proof-output>
python scripts/check_lowest_note_format.py <new-format-output>
python scripts/check_lowest_note_review.py <new-review-output>
python scripts/check_lowest_note_doublet.py <new-doublet-output> --artifacts
pdftoppm -png -r 100 docs/papers/ym-theory-statement/statement.pdf <existing-preview-directory>/page
```

Dependencies: Python, SymPy, NumPy, Matplotlib, pypdf; the figure check imports
the existing generator without regenerating its files. The rebuild uses the
paper's existing MiKTeX/NewPX configuration. Visual inspection is separate
from the scripts' algebra checks.
