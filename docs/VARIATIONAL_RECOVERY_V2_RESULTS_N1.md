# V2 results at n = 1 (all six arms)

2026-11-19. Spec: `docs/VARIATIONAL_RECOVERY_V2_SPEC.md` with Amendments 1–4. Receipts:
`research/variational-v2-20261119/L0_{linear,quad,hybrid}_{N1,N2}.json` (and `.log`, `.C.npy`).
Research script runs, not engine runs. The four arms that ran out of memory the first time were rerun
two at a time; their first logs are in `crashed-oom/`.

## Controls

- **C-copies:** 0 cells see two copies of an edge, so n = 1 is admissible for three rings.
- **C-quad:** ‖N₂ᵀX‖/‖X‖ ≤ 4.5e-16 (linear and quadratic).
- **C-exact:** own fluxes, closed linear and closed quadratic fields reproduced to 2.3e-14 after the
  change.
- **C-solve:** passes on the three N1 arms (relative residual 1.5–2.8e-9, by eigh). It **fails on the
  three N2 arms** (6.8e-5 to 1.8e-4). With N₂ only 24 live unknowns per cell remain (39 with N₁), and
  the column condition cannot be met in least squares to the threshold. Per Amendment 4 this is
  reported, not hidden; their numbers below are for the best least-squares C.

## Gates

| base | exclusion | BW05 before (C = 0) | BW05 after | reconstruction after | test variation after | amplification (median) | V2-stable |
|---|---|---|---|---|---|---|---|
| linear | N1 | 0.390 | 1.353 | 1.381 | 0.079 | 21.1 → 48.4 | pass |
| linear | N2 | 0.390 | 0.891 | 0.886 | 0.057 | 21.1 → 132.0 | pass |
| quadratic-aware | N1 | 0.536 | 1.483 | 1.510 | 0.071 | 27.6 → 47.6 | pass |
| quadratic-aware | N2 | 0.536 | 5.369 | 5.389 | 0.111 | 27.6 → 378.8 | pass |
| hybrid | N1 | 0.607 | 1.026 | 1.057 | 0.072 | 26.8 → 46.4 | pass |
| hybrid | N2 | 0.607 | 1.008 | 1.026 | 0.038 | 26.8 → 151.8 | pass |

- **P-recon (reconstruction ≤ 10%): fails on every arm**, at 89% to 539%. Before the change the
  three-ring reconstruction terms were 1.5% (linear), 6.9% (quadratic-aware) and 4.3% (hybrid).
- **V2-acc (BW05 below Whitney's 29.8%): fails on every arm**, at 89% to 537%.
- **V2-stable: passes on every arm**: 21 small eigenvalues (the gauge dimension), none negative, the
  smallest physical eigenvalue 3.97–4.00.

## Reading

1. **The change does what it was built to do, and that is the problem.** The test-variation term falls
   from 0.38–0.61 to 0.04–0.11 on every arm. The reconstruction term, which the count predicted the
   exclusion would protect, grows by a factor of 15 to 80. The minimum-norm correction is large
   (median ‖C‖ about 225 on the hybrid N2 arm), and the reading's amplification grows with it.
2. **The exclusion is not what decides it.** N1 against N2 at n = 1: 1.38 vs 0.89 (linear), 1.51 vs
   5.39 (quadratic-aware), 1.06 vs 1.03 (hybrid). There is no consistent direction. The spec's removal
   is defined at n = 2 (N1 at least 3× N2), but n = 1 shows nothing for it to find.
3. **The count's prediction failed at n = 1.** Leaving the second-order part of the clue space alone
   did not keep the reconstruction small once the column condition is enforced at three rings. This
   is the failure mode of V1, and the one Lemma C of the magnet note describes: the poke term the
   column condition asks C to cancel is paid for in the reconstruction. V1c and M0 are the tests built
   on that explanation.

## n = 2

The gates are defined at n = 1 and n = 2. P-recon fails at n = 1 by a factor of 9 to 54, on every arm.
Each n = 1 arm took about an hour and a half of solve time (`lsqr_seconds` up to 5,187), and n = 2 is
much larger. **Recommendation: do not run V2 at n = 2.** The variational route continues through M0
(the seamless reading), which addresses the mechanism this result points at. This is Bee's call.
