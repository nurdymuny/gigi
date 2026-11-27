# Origin-splitting evidence

Final gate: **PASS**, `release-01/receipt.json`. Its LF-normalized hashes identify the tested source on baseline
commit `8fd4e033`. That field names the pre-change baseline; it does not claim the new files were already in that
commit. `final-source-audit.json` confirms the recorded hashes match the committed files.

## Result and boundary

For the unsourced reduced Hamiltonian, every fixed compact simple metric colour algebra and t > 1:
- at q = 0 the charge ±2 two-quantum singlets have formal series 2/√t with every correction zero;
- the charge-0 singlet moves by e₂g², with e₂ = −2κ(1 + 1/t) and κ the adjoint Casimir.

Quasimodes put spectrum within O(e^(−c/g²)) and O(g³) of those values. This closes, in the reduced model, the
O(g²) comparison that H7 Step 6 posed.

Not closed:
- counting, and assignment of these levels to the q = 0 well (cited localization);
- the physical room (H2b);
- the q_MC reading of H8 Step 7.

H7 and H8 remain open; the seven top-level hypotheses are unchanged. No model, threshold or numerical spectral
result was changed.

The global analysis is written mathematics, not end-to-end Lean. Six Lean declarations check the scalar algebra,
the ad-invariance identities and the conditional ordering, with no added axioms or admitted steps. See
[`H8Origin-README.md`](../../formal/h7-cubic-quartic/H8Origin-README.md).

## Receipts retained

| Location | Result |
| --- | --- |
| `red-00-path-limit/` | First red attempt. The exact checks behaved as expected, but `lake build` failed: the worktree sat under a long temporary path, Mathlib object paths exceeded the Windows limit, and copied packages had lost their timestamps. Environment failure, not mathematics. Retained. |
| `red/` | Ten exact checks passed and the missing proof failed. All six Lean declarations were unknown. `passed` stays false; `expected_red_observed` is true. |
| `release-01/` | Eleven exact checks: six algebra, four mechanism removals, one documentation. Also six Lean declarations, two poisoned axiom-audit rejections, and four broken Lean variants rejected, each with a mathematical error and no import failure. |
| `h8-vacuum-regression/` | The existing strict vacuum-floor gate, rerun after the shared lakefile gained a library: PASS. |
| `final-source-audit.json` | Recorded source hashes against the committed files. |

## What the exact checks cover

- **SU(2) on all nine coordinates and SU(3) (Gell-Mann) on all 24:**
  - L₁ annihilates ‖X₂‖² − ‖X₃‖² and ⟨X₂,X₃⟩, and L₁r = −4𝒞;
  - e₂ = −4(1 + 1/t) for SU(2) and −6(1 + 1/t) for SU(3).
- **Quasimode polynomials:** φ₁ = 4𝒞/√t and φ₂ solve the order-g and order-g² equations exactly. With e₂ off by
  one the solver refuses.
- **κ values, metric rescaling, non-degeneracy at 2u for t > 1, and Schur** for su(2) and su(3).
- **Mechanism removals:**
  - a non-invariant trilinear cubic keeps ‖X₂‖² − ‖X₃‖² annihilated (Euler homogeneity) but breaks ⟨X₂,X₃⟩;
  - ω₂ ≠ ω₃ breaks the eigenfunction;
  - no cubic gives e₂ = 0;
  - a reversed resolvent flips the sign.

## Reproduction

From the repository root, into new directories only:

```text
python scripts/check_h8_origin_splitting.py <fresh-output-directory>
python scripts/check_h8_vacuum_floor.py <fresh-regression-directory>
```

Lean and Mathlib are the project's pinned revisions, recorded in the receipts. No GPU job, numerical
diagonalization, full engine suite, paper rebuild or deployment.
