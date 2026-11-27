# Strict vacuum-floor evidence

Final gate: **PASS**, `release-03/receipt.json`. Its LF-normalized hashes
identify the tested source, on baseline commit
`1c31cb6bd533a3685e02681735d89004e68513fd`. The source revision field names
that pre-change baseline; it does not assert that uncommitted tested files
were already in that commit. `final-source-audit.json` confirms that all
23 recorded source hashes match the files being committed.

## Result and boundary

For the unsourced reduced Hamiltonian, every fixed compact simple metric
colour algebra of dimension n, t > 0 and g > 0 has E0 > nS/2. The written
proof uses the earlier confinement result, factorization, weak-kernel
rigidity and a positive-measure cone of non-normalizability. This is not
the excitation gap. H7 and H8 remain open; seven top-level hypotheses
remain. No model, threshold or numerical spectral result was changed.

The global analysis is written mathematics, not an end-to-end Lean proof.
Seven Lean declarations check the stated scalar algebra and conditional
logic without added axioms or admitted steps. See
[`H8Vacuum-README.md`](../../formal/h7-cubic-quartic/H8Vacuum-README.md).

## Receipts retained

| Location | Result |
| --- | --- |
| `red/` | Six exact algebra/control tests passed; the missing proof failed. All seven absent Lean declarations failed. `passed` stays false; `expected_red_observed` is true. |
| `release-01/` | Document marker failed because a line break split a phrase. No mathematical failure; fixed the marker check to normalize whitespace. This failed receipt is retained. |
| `release-02/` | First complete pass, including all four broken variants. The source-manifest coverage and register edits were being finalized; use release-03 as the authoritative final snapshot. |
| `release-03/` | Seven checks (six algebra/control, one documentation), seven Lean declarations, two poisoned dependency-audit rejections, and four rejected broken Lean variants. |
| `h7-regression/` | All 12 exact confinement/prerequisite checks passed. |
| `h8-regression.log` | All 14 existing inter-well exact/document checks passed after the H8 note edit. |
| `reuse-rejection.*` | An attempted run into an existing output directory exited nonzero and left the original receipt byte-identical. |
| `final-source-audit.json` | All 23 final source hashes match. |

The four source mutations change the factorization shift sign, remove
attainment, remove kernel exclusion, or overstate the cone-tail bound.
Every one fails compilation for the changed mathematics/proof dependencies,
not an unavailable import. Separate exact controls give Gaussian equality
at zero coupling, a zero-area ray, and free-particle energies 1/(4L^2)
approaching zero without an attained ground state.

## Reproduction

From the repository root, choose new directories; never reuse a receipt:

```text
python scripts/check_h8_vacuum_floor.py <fresh-output-directory>
python scripts/check_h7_cubic_quartic.py <fresh-regression-directory>
```

The existing H8 algebra/document regression can be run from `scripts`:

```text
python -m unittest check_h8_interwell -v
```

Lean and Mathlib revisions are pinned in the existing formal project and
recorded in the receipts. No GPU jobs, numerical diagonalizations, full
engine suite, paper/PDF rebuild, or deployment were performed.
