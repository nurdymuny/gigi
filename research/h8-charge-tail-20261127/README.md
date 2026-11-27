# Charge-tail evidence

Final gate: **PASS**, `release-02/receipt.json`. It was run after the register entry was added, so its
LF-normalized hashes cover the committed sources, including `CLOSED-SET.md`. `source_revision` names the
pre-change baseline commit `c3947e6f`; it does not claim the new files were already in that commit.

## Result and boundary

These results hold for the unsourced reduced Hamiltonian, every compact simple metric colour algebra, every
t > 0 and every g:
- **Horizontal barrier:** |∇W|² ≥ |X_hor|²/t at every q. This covers every rank stratum and both determinant
  sheets. It is sharp, so b_m(g) = |m|/√t exactly. This is route Step 2.
- **Charge floor:** inf spec on charge m is at least σ + m²/(4tσ) for |m| ≤ 2√t σ, with σ = nS/2. At t = 3 this
  is (225 + m²)/(30√3).

**Not closed:**
- any upper bound on E_{2,0} (Step 3), which is needed before any charge is actually removed;
- the remaining sectors (Step 4);
- H8 itself;
- H2b.

No eigenvalue is computed or enclosed. H8 remains open; the seven hypotheses are unchanged.

## Receipts retained

| Location | Result |
| --- | --- |
| `red/` | Thirteen exact checks passed and the missing proof failed. All five Lean declarations were unknown, checked through a temporary declarations file. `passed` stays false; `expected_red_observed` is true. |
| `release-01/` | PASS before the register edit. Its hashes predate the `CLOSED-SET.md` change. Retained. |
| `release-02/` | PASS on final sources: 14 exact checks (8 algebra, 5 mechanism removals, 1 document). It also covers 5 Lean declarations with standard axioms only, 2 poisoned audit rejections, and 4 broken Lean variants rejected. |
| `origin-regression/` | The origin-splitting gate, rerun after the shared lakefile gained a library: PASS. |
| `vacuum-regression/` | The strict vacuum-floor gate, rerun for the same reason: PASS. |

How the four Lean variants fail:
- `missing_small_charge` and `doubled_ruler` fail as false statements; `linarith` finds no proof.
- `missing_antisymmetry` and `missing_twisted_bound` fail at the step that used the removed hypothesis. Both
  weakened statements are false: an arbitrary bracket need not satisfy [x, x] = 0, and σ = 1, c = 1/4, P = 1/2,
  E = 1 violates the floor without the twisted bound.

## Reproduction

From the repository root, into new directories only:

```text
python scripts/check_h8_charge_tail.py <fresh-output-directory>
```
