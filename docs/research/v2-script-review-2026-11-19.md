# Review of the V2 script (ba1e028), and an n = 3 probe of the hybrid base (19 November 2026)

This is the Mac's response to the Windows machine's handoff (`from-the-windows-machine-2026-11-19.md`)
and to `scripts/ricci_variational_v2.py`.

**No V2 run was started here**, as the handoff asks. The only new computation is the C = 0 base probe at
n = 3. It is a separate script and a separate receipt: `research/variational-baseprobe-20261119/L2_three_bases.json`.

## Verdict

**The script implements the spec and all three amendments faithfully. No defect changes a number, so
the running jobs should not be stopped.** I read it line by line against `VARIATIONAL_RECOVERY_V2_SPEC.md`:

- **The arms** are linear, quad and hybrid, each with N₂ and with N₁.
- **Z** is the null space of the own-face rows.
- **N₂** is the left null space of [X_lin | X_quad], and **N₁** that of X_lin.
- **The constraint rows** are V1's per-edge column condition, on the same 23 generators.
- **The solve:**
  - `lsqr` runs from x₀ = 0 with atol = 0 and btol = 10⁻⁹. Only stopping rule 1, ‖r‖ ≤ 10⁻⁹‖b‖, can fire,
    so the solve is the minimum-norm one Amendment 2 asks for.
  - The dense eigh of AAᵀ is kept as a cross-check, and the smaller residual is used.
- **Gate thresholds:** 10%, Whitney's 29.8% and 26.0%, and gauge counts of 21 and 163.

### The two-ring C = 0 receipt is a confirmation, not an anomaly

`L0_rings2_c0.json` shows the quad and hybrid bases at two rings with amplification ≈ 1,490 and
reconstruction terms of 437% and 446%. That is exactly the trap the count predicted:

- 26 unknowns are fitted from 26 clues at n = 1, so the fit is a square solve on a nearly singular system.
- The third ring is what makes these bases usable (amplification 26.8 and 27.6 at three rings).

The linear base at two rings reproduces the paper (45.405%, PG 6.223%), so C-repro passes.

### The hybrid's own fluxes are exact on all data, not only on smooth fields

The probe measured own-flux exactness only on the 26 smooth fields (5 × 10⁻¹⁶). It holds for every
data vector. Let f be data and f_own its own-face part.

- A closed field's fluxes through a cell's boundary sum to zero, so every row of A_quad is a closed
  own-flux vector. Rank(A_lin) = 3, the dimension of that closed space, so A_quad M₂₆[11:] f lies in
  range(A_lin).
- A_lin pinv(A_lin) is the projector onto range(A_lin), so it leaves that vector unchanged:
  A_lin M_hyb f = A_lin M₂₆[:11] f + A_quad M₂₆[11:] f = [A_lin | A_quad] M₂₆ f.
- The equality-constrained fit makes that last expression equal f_own whenever f_own is closed, which
  data always are.

**Consequence for C-exact.** The receipts check only the *change* ΔM, on the own faces, the linear
fields and the quadratic fields. Combined with the base's exactness (the probe receipts, plus the argument
above), this covers the whole of C-exact for V2 and V2h. The results document should say so explicitly,
because the receipts alone do not.

### Small issues (none affects a number)

| # | issue | effect | fix |
| --- | --- | --- | --- |
| a | `left_is = "minus"` is hard-coded; the comment says it is "checked again below", but no check runs | none: all four V1/V1b receipts (n = 1, 2) detected `minus` | an `assert` against V1's C-span test on cell 0, or correct the comment |
| b | the refuse-to-overwrite check runs **after** an arm finishes | a rerun into the same directory computes for hours, then exits without writing | check every arm's output path before the cells are built |
| c | the solution C is not saved | any later look (the PG force with the change, a spectrum, a per-cell ‖C‖ map) needs a rerun | `np.save` x beside each receipt (about 5 MB at n = 2) |
| d | with atol = 0, an inconsistent system (V1 at n = 2 stalled at 6.8 × 10⁻⁵) runs the full 10⁵ iterations | time only; this is what Amendment 2 specifies | none; this note is here so the run time is not mistaken for a hang |
| e | C-quad is computed on [X_lin \| X_quad] together; the spec states it per block | a small block's relative error can hide behind the larger one | report the two ratios separately |
| f | `pg_error_unchanged_base` reports the PG force with C = 0 only, but T′ = M′D also changes the PG force | a missing number, not a wrong one | assemble K_p from T′ too (cheap: already done for K_v) |

**Recommendation.** Leave the running n = 1 and n = 2 jobs alone. Before any further run (n = 3, a
rerun), apply b and c; e and f are one line each.

## What I can add: does the hybrid PG lead survive at n = 3?

The hybrid note found, with C = 0, a Petrov–Galerkin force error of 5.47% at n = 1 and 0.85% at n = 2.
The published two-ring force gives 6.22%, 3.83%, 2.79% and 1.45% at n = 1, 2, 3, 5.

This lead is independent of whether V2 passes: it uses no change at all. The PG force is not
variational, so it does not replace V2. But if the lead holds, it is a better *Reading the Curve*
force with the same own-face principle. **n = 3 is the first test** that a two-level coincidence
cannot pass.

**Predictions, fixed before the n = 3 number exists** (C = 0, three rings, hybrid base):

1. **H-beat.** The hybrid PG error at n = 3 is below the published two-ring 2.79%.
2. **H-order.** It is below the hybrid's own n = 2 value of 0.85%, so it is still falling, not at a floor.
3. **H-amp.** The median amplification stays at or below its n = 2 value (16.2) × 1.25.

- If H-beat fails, the n = 2 number was a pre-asymptotic accident, and the lead is dropped.
- If H-beat passes and H-order fails, the hybrid has a floor near 1%, which matters for n = 5.

The probe is running (C = 0, three bases, two and three rings). Results will be appended below,
unedited.

## Status of the other Mac jobs

S3c's two-ring force at n = 2, on the 5× window (T = 0.1), is still running:

- **N = 40:** at t = 0.08.
- **N = 80:** at t = 0.06.

Both are slowing (Newton takes 9 to 11 iterations late in the window). Native and the n = 1 runs are done.
P-long (two-ring) and P-cross are judged when both land.

## n = 3 result (appended unedited after the run; 198 s)

Receipt: `research/variational-baseprobe-20261119/L2_three_bases.json`. C = 0 throughout.

**Controls reproduce the paper at n = 3:**

- The published force (linear base, two rings) gives a PG error of **2.788%**; the paper has 2.79%.
- Its variational error is **330.2%**; the paper has 330.2%.

**PG force with the hybrid base, three rings:**

| n | h_max | published two-ring PG | order | **hybrid PG** | **order** | hybrid amplification (median) |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 0.3881 | 6.22% | | 5.47% | | 26.8 |
| 2 | 0.2017 | 3.83% | 0.73 | 0.85% | 2.85 | 16.2 |
| 3 | 0.1355 | 2.79% | 0.80 | **0.401%** | **1.89** | 15.5 |

Orders are computed from the relative errors. For the published column, this reproduces the paper's 0.73
and 0.80.

**Gates:**

- **H-beat: passes.** 0.401% is below 2.79%, by a factor of 7.
- **H-order: passes.** 0.401% is below 0.85%; the error is still falling, with no floor in sight.
- **H-amp: passes.** 15.5 is at most 16.2 × 1.25.

**Everything else at n = 3** (three rings, C = 0):

| base | PG | variational | reconstruction | test variation | curvature error | amplification (median / max) | own-flux defect |
| --- | --- | --- | --- | --- | --- | --- | --- |
| linear | 3.80% | 229% | 3.54% | 228% | 0.44% | 22.0 / 35.2 | 4.5×10⁻¹⁶ |
| quadratic-aware | 0.56% | 270% | 0.64% | 270% | 0.18% | 15.8 / 21.8 | 4.8% |
| **hybrid** | **0.40%** | **116%** | **0.43%** | 116% | **0.16%** | **15.5 / 21.3** | **4.8×10⁻¹⁶** |

**Two-ring quad and hybrid at n = 3:** amplification max 8.2 × 10⁵ and variational error 6.7 × 10⁵ %.
The 26-from-26 trap again. The third ring is necessary, not optional.

## What this means

1. **For *Reading the Curve*:**
   - The hybrid three-ring PG force runs about 2.9 → 1.9 in observed order, against the published
     force's 0.7–0.8.
   - It is 7× more accurate at n = 3, keeps the own-face principle, and has lower amplification.
   - It is a candidate for the next version. Before any claim, it needs, unchanged from the published
     force's checks:
     - n = 5;
     - the BW05 gate function;
     - the q4/q6 difference;
     - the S2 stability spectrum (decomposed eigenvalues, numerical range) at n ≤ 3;
     - the SU(2) flow.
   - Nothing here touches the variational question.
2. **For V2:** the hybrid base's test-variation term does not decay with n (111% to 116%). It still needs
   the change, as the spec assumes. Its reconstruction term (0.43%) and amplification (15.5) are the best
   starting point V2 could have. If V2h's change keeps the reconstruction term near the base's, P-recon
   passes with a wide margin.
3. **Proposed next step (not started):** a spec for the hybrid PG force at n = 5, with the paper's
   stability and BW05 gates. It is cheap at n ≤ 3 and needs the S2 spectrum machinery.
