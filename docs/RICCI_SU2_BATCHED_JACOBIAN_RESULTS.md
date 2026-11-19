# B5: the linearised SU(2) force (the matrix K), batched (results)

2026-11-19. Spec: `docs/RICCI_SU2_BATCHED_JACOBIAN_SPEC.md` (with Amendment 1). Research script
`scripts/ricci_su2_batched.py` (modes `kparity`, `kbuild`), **not an engine run**. Receipts in
`research/su2-batched-K-20261119/`.

## In plain English

The batched code builds the same matrix K as the old cell-by-cell loop, agreeing to about
6 × 10⁻¹¹ at worst, for every force and every level checked. It builds K at n = 5 for the first
time. At n = 3 it takes about 2 minutes on one process; the loop took 6,672 s on twelve.

## Gates

Reference: the loop's `Setup.column` on the current code (exact defect derivative). Relative
Frobenius errors.

| Gate | n | Force | Columns | Worst column | Matrix | Pattern | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P-K1 | 1 | native | all 426 | 2.2 × 10⁻¹¹ | 3.6 × 10⁻¹² | identical | PASS |
| P-K1 | 1 | two-ring | all 426 | 5.9 × 10⁻¹¹ | 1.3 × 10⁻¹¹ | identical | PASS |
| P-K1 | 1 | one-ring | all 426 | 5.3 × 10⁻¹¹ | 2.6 × 10⁻¹² | identical | PASS |
| P-K2 | 2 | native | 60 | 6.0 × 10⁻¹² | 1.1 × 10⁻¹² | identical | PASS |
| P-K2 | 2 | two-ring | 60 | 1.7 × 10⁻¹¹ | 3.6 × 10⁻¹² | identical | PASS |
| P-K2 | 2 | one-ring | 60 | 1.5 × 10⁻¹¹ | 1.4 × 10⁻¹² | identical | PASS |
| P-K2 | 3 | native | 60 | 2.3 × 10⁻¹² | 4.3 × 10⁻¹³ | identical | PASS |
| P-K2 | 3 | two-ring | 60 | 9.8 × 10⁻¹² | 1.7 × 10⁻¹² | identical | PASS |
| P-K2 | 3 | one-ring | 60 | 6.6 × 10⁻¹² | 9.9 × 10⁻¹³ | identical | PASS |

The tolerance is 10⁻⁸; the worst case uses less than one percent of it. "Matrix" is over the
checked columns.

**Amendment 1.** The first P-K1 run failed P-pattern at the original 10⁻¹⁴ × max zero line. In
2 of 61 columns, a true-zero entry came out as rounding noise near 10⁻¹² × max in one build and
as exact zero in the other. The amendment moved the line to 10⁻⁹ × max before any other gate
ran. The failed receipt is in `failed-pattern-1e-14/`.

**Assembly change, then rerun.** After all nine gates passed, I changed how `build_K` assembles
the matrix: one direction k at a time, summed, rather than all at once. This keeps memory
bounded at n = 5. It changes only the order of a sparse sum. Because it was a code change after
the gates, all nine were rerun on the new code. Every number reproduced to the last printed
digit. Receipts from before the change are in `before-assembly-change/`; those from after are
in the folder root.

## Checked, not gated: the n = 3 matrices from S3b

Against `outputs/su2-cayley/L2_{native,pg}_K.npz` (built by the loop for S3b), the batched K
differs by 1.03 × 10⁻⁷ (native) and 1.29 × 10⁻⁷ (two-ring), whole-matrix relative. That is about
ten thousand times the loop–batched disagreement on the current code. The likely cause is that
the S3b n = 3 build started before the exact defect derivative was pulled, so its columns carry
the old finite-difference derivative's noise. **This is not confirmed.** It does not touch any
S3b result: in S3b, K₀ only preconditions the quasi-Newton solve. The residual is always the
full force, and every step converged to the 10⁻⁸ tolerance.

## The matrices

Built after the gates passed, at g(0) = (3, 1, 1), ε = 10⁻⁶. Saved as sparse `.npz` in
`outputs/su2-batched-K/`; that folder is gitignored, at 189 MB. The receipts `kbuild_L*.json`
carry each file's SHA-256, dimensions and nonzero count. One process.

| n | Force | Unknowns | Nonzeros | Setup (s) | Build (s) |
| --- | --- | --- | --- | --- | --- |
| 1 | native | 426 | 20,712 | 0.2 | 0.5 |
| 1 | two-ring | 426 | 91,188 | 4.8 | 5.0 |
| 1 | one-ring | 426 | 46,920 | 1.7 | 1.5 |
| 2 | native | 3,372 | 165,629 | 1.0 | 3.1 |
| 2 | two-ring | 3,372 | 861,979 | 39.3 | 48.0 |
| 2 | one-ring | 3,372 | 399,194 | 11.5 | 12.2 |
| 3 | native | 11,358 | 558,938 | 3.1 | 10.0 |
| 3 | two-ring | 11,358 | 2,995,172 | 75.9 | 52.3 |
| 3 | one-ring | 11,358 | 1,362,749 | 15.3 | 15.3 |
| **5** | native | 52,530 | 2,587,562 | 5.6 | 26.3 |
| **5** | two-ring | 52,530 | 14,076,356 | 192.9 | 266.1 |
| **5** | one-ring | 52,530 | 6,346,241 | 100.8 | 60.0 |

"Setup" builds the force's fits and plans; "build" is the K assembly itself.

**Cost.** For the two-ring force at n = 3, setup plus build is 128 s on one process, against
the loop's 6,672 s on twelve processes. That is about 52× in wall time, and about 600× in
process time. The batched time is a single run on this machine, with nothing else measured
alongside it.

## What this unblocks

- **B6.** Stability spectra and numerical ranges from exact-derivative K at n ≤ 3, and at n = 5
  as far as the eigensolver allows. These remove the paper's finite-difference caveat.
- **C9.** The n = 5 flow now has its K₀ for the Newton preconditioner.

## Not done here

The spectra and ranges themselves (B6, its own spec). No n = 5 matrix has been checked against
the loop: the loop has never run at n = 5. The n = 5 builds rest on the gates at n ≤ 3 holding
for the same code.
