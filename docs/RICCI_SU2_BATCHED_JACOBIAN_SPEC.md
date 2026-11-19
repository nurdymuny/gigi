# B5: the linearised SU(2) force (the matrix K), batched (spec)

2026-11-19. Written before any batched-Jacobian code ran. Research script
`scripts/ricci_su2_batched.py` (new mode `buildK`), **not an engine run**. It changes how K is
computed, not what K is.

## In plain English

K is the force's response to nudging each link: one column per link and direction. It is what
the stability spectra (S2b), the numerical range (S2c) and the Cayley step's Newton solve (S3b)
are built on. Today it is built cell by cell in a Python loop: at n = 3 that took 6,672 s on
twelve processes, and it has never been built at n = 5. This step builds the same K with the
batched force of `docs/RICCI_SU2_BATCHED_FORCE_SPEC.md`, checks it against the loop, and then
builds it at every level, so that the stability work (B6) and the n = 5 flow (C9) can run.

## What K is (unchanged)

As in `scripts/ricci_su2_pg_stability.py` (`Setup.column`): at the stored initial links and the
Berger metric g(0) = (3, 1, 1), column (g, k) is the central difference

  K[:, (g, k)] = [ grad(U with U_g ← U_g exp(+ε τ_k)) − grad(U with U_g ← U_g exp(−ε τ_k)) ] / 2ε,

with ε = 10⁻⁶, right-trivialised, k = 1, 2, 3. Only the cells that read link g change, so only
their contributions are recomputed. The face-defect derivative inside the gradient is the exact
one (`docs/RICCI_SU2_EXACT_DEFECT_DERIVATIVE_SPEC.md`), so this K has no finite-difference
noise from the gradient itself, only the central difference of the column.

## The batched build

- **Rows.** One row per (cell c, link g that c reads, direction k, sign ±): about 43 links per
  cell for the two-ring force, so about 840,000 rows at n = 3.
- **One row = one cell's contribution with one link overridden.** The batched gradient's
  pieces (local links, defects, the exact defect derivative, the transport tree, the face
  holonomies, the lift) take per-row link arrays in which every occurrence of link g is replaced
  by its perturbed value, including the lift −R(U_g) of a reversed edge.
- **Chunks.** Rows are processed in chunks of fixed size, so memory stays bounded at n = 5.
- **Assembly.** Each row's six 3-vectors, times ±1/2ε, are scattered into column (g, k) at the
  cell's six edges; the result is stored as a sparse matrix.
- **Metric pieces** come from the batched class at g(0); the loop uses the per-cell quadrature
  at the same metric. They agree to rounding (P-metric of the batched-force spec: 2 × 10⁻¹⁵).

## Parity gates (fixed now)

The loop's `Setup.column` on the current code (exact defect derivative) is the reference.
Relative errors are Frobenius norms, per column or per matrix.

- **P-K1.** n = 1, every column, native, two-ring and one-ring: ‖K_batched − K_loop‖/‖K_loop‖
  ≤ 10⁻⁸. The central difference turns gradient rounding (~10⁻¹⁶ relative) into column noise
  of order 10⁻¹⁶/ε = 10⁻¹⁰; 10⁻⁸ leaves a hundredfold margin.
- **P-K2.** n = 2 and n = 3, 60 columns per force chosen by a fixed seed (`default_rng(0)`),
  each column ≤ 10⁻⁸ relative to the loop column.
- **P-pattern.** At every level checked, the batched and loop columns have the same set of
  nonzero rows, with entries below 10⁻¹⁴ × max|column| counted as zero in both.

**Checked, not gated:** the n = 3 matrices already on disk
(`outputs/su2-cayley/L2_{native,pg}_K.npz`, built by the loop for S3b) against the batched
build, whole-matrix relative difference reported.

## What is built after the gates pass

K at n = 1, 2, 3, 5 for native, two-ring and one-ring, saved as sparse `.npz` with SHA-256 in a
receipt under `research/su2-batched-K-20261119/`, together with dimensions, nonzero counts and
build time. n = 5 is built only after P-K1 and P-K2 pass.

## Cost (measured, not gated)

Seconds to build each K, one process. The loop's n = 3 build (6,672 s on twelve processes) is
the comparison.

## Not done here

The spectra and numerical ranges from these matrices (B6, its own spec); the n = 5 flow (C9).

## Amendment 1 (2026-11-19): the pattern threshold was below the method's own noise

**Written after the first P-K1 run failed, before any other gate was run.** No other threshold
changes.

**What happened.** At n = 1, two-ring, every column agrees with the loop to 5.9 × 10⁻¹¹ (median
1.6 × 10⁻¹¹), within P-K1's 10⁻⁸. P-pattern failed: in 2 of 61 columns inspected, an entry is
exactly zero in one build and about 10⁻¹² × the column's largest entry in the other (for example
−7.6 × 10⁻¹¹ against 0). These are entries whose true value is zero, where the central
difference of two nearly equal sums leaves rounding noise in one build and cancels exactly in
the other. The spec put the zero line at 10⁻¹⁴ × max, below the method's own noise floor of
about 10⁻¹¹ × max (gradient rounding over ε = 10⁻⁶). The failed receipt is kept in
`research/su2-batched-K-20261119/failed-pattern-1e-14/`.

**Change.** In P-pattern an entry counts as zero below 10⁻⁹ × the column's largest entry: a
hundred times above the noise floor and ten times below P-K1's tolerance, so an entry that P-K1
would allow to differ cannot hide a structural difference.
