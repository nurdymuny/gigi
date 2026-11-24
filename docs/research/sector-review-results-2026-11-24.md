# The sector solvers clear the measured residual bottleneck

24 November 2026. **COMPUTED: verified existing-control results, not a certified spectral theorem.**

The scalar and tensor sector formulation now passes the numerical residual check that the cutoff-14
all-spin calculation could not reach. For both frozen controls, the vacuum, lowest scalar excitation and
lowest tensor multiplet first pass their shared allowance at cutoff 20. All three also pass the stricter
supplementary 0.01 check from cutoff 22 onward.

The run completed 16 coupling/cutoff cases, saved both sectors at every point, and repeated each control's
top cutoff at higher assembly precision. All four higher-precision comparisons passed. Saved-matrix replay
passed for all 36 archives, covering 216 reported residuals. No held-out coupling was evaluated.

## Review findings and repairs

Pulled review target: `0bc5bd4d`, following the scalar solver at `93b47d0b`.
The imported 24 tests passed before any changes. The invariant derivative tables and exact Gaussian moments
were not changed. Their agreement with the six existing nine-variable GPU controls survives the corrections.

| Finding | Correction and evidence |
| --- | --- |
| The residual used only the exterior term and clipped negative contractions to zero | Include the approximate eigensolver's projected error; contract in mpmath; reject materially negative exterior data. A planted rotated eigenpair with zero exterior Gram now has the required nonzero residual. |
| Each channel used its own gap for an allegedly inherited allowance | Both sectors now share the minimum available excitation gap. A planted case that incorrectly passed the larger scalar allowance is rejected. The vacuum is checked too. |
| Precision wording implied all linear algebra was high precision | Explicitly report 60-digit assembly, orthonormalization and residual contractions, but a float64 eigensolve. The 100-digit check reassembles the same mixed-precision pipeline. |
| The runners could overwrite evidence, lacked bounded supervision, and could finish with a success exit after failed checks | New output directories only; frozen inputs; source snapshots and drift checks; finite JSON; saved matrices; recomputation; immediate logs; resource supervision; nonzero failure exits; stale-PASS invalidation. Both entry points share the workflow. |
| Development prose disagreed with its own numbers | Preserve the original table as unverified history; identify its incorrect N=14 energy and inconsistent first-passing-cutoff statement. Use the new receipts for results. |

The [review protocol](sector-review-protocol-2026-11-24.md), corrections and tests were committed and pushed
as `abd81f978faf4b3c29fbc44d5ac6d7df02a75aae` BEFORE execution. Behavioral RED evidence records 14 assertion
failures and zero unexpected errors. GREEN evidence contains **69 focused tests with no skips** and **19
assertion-rejected mechanism removals**. The run repeats those prerequisites. No full Rust suite was run;
no database engine or HTTP route changed.

## The unchanged mathematical problem

The model, domain, coupling values, width choices and conditional-separation rule remain those of the
registration. The scalar solver works with invariant polynomials in T, P and d. The tensor solver uses
their products with the two traceless symmetric covariant generators. This removes irrelevant sectors from
each calculation; it does not claim to retain every spin in the full problem.

At cutoff 24, the scalar space has 169 basis functions and the tensor module has 244 multiplets. The
five tensor members remain represented by rotational covariance, not discarded arbitrarily. Low-cutoff
agreement tests compare their energies and full residual norms against the earlier complete quintets.

For the returned approximate eigenpair, the reviewed residual is

\[
 r^2=\frac{v^T Xv+\|\widetilde H v-Ev\|^2}{v^Tv},
 \qquad X=\widetilde K-\widetilde H^2.
\]

Both terms are contracted at assembly precision. Material negative values refuse; tiny roundoff handling
has an explicit recorded tolerance. This is floating-point error accounting, not interval certification.
The `gram_digits_lost` field is a Cholesky-pivot diagnostic, not a certified condition-number bound.

## Measured residuals

Controls: coupling squared 1 at alpha 1.7; coupling squared 2 at alpha 2.3. Widths are fixed across all
states and cutoffs. The table reports the lowest scalar excitation and lowest tensor multiplet only.

| Cutoff | Scalar, coupling 1 | Tensor, coupling 1 | Scalar, coupling 2 | Tensor, coupling 2 |
| --- | --- | --- | --- | --- |
| 10 | 0.347055237031 | 0.388349277737 | 0.416758575293 | 0.427874949460 |
| 12 | 0.158879704627 | 0.179695932818 | 0.223010314720 | 0.203614513158 |
| 14 | 0.111475664736 | 0.083441917815 | 0.126114635141 | 0.117382683244 |
| 16 | 0.049673666985 | 0.048996817286 | 0.060669496628 | 0.060551078746 |
| 18 | 0.019698009586 | 0.025808376011 | 0.033672796654 | 0.031669076594 |
| 20 | 0.011625707732 | 0.012368223431 | 0.017270563448 | 0.016596753247 |
| 22 | 0.006026220194 | 0.006402133679 | 0.009041716759 | 0.008717436769 |
| 24 | 0.002773801367 | 0.003421505416 | 0.004793443248 | 0.004551948676 |

The common allowance is 0.01*max(1,min(gap_scalar,gap_tensor)): at N=24 it is 0.022696065519 for coupling
squared 1 and 0.030192984714 for coupling squared 2. The vacuum residuals there are 0.000633453520 and
0.000649067748. The first joint pass, including the vacuum, is N=20 for both controls. The scalar alone
already passes at N=18 for coupling squared 1.

At N=22 and N=24 every target residual is also below 0.01, the minimum possible numerical allowance under
the inherited formula. This stronger supplementary observation is not dependent on choosing the larger
of the two channel gaps. It does not identify the global lowest excitation.

## Next levels and conditional ordering

The next-level errors remain measured and included, not replaced by zero:

| Coupling squared, N=24 | Next scalar residual | Next tensor residual | Tensor energy minus scalar energy | Conditional order |
| --- | --- | --- | --- | --- |
| 1 | 0.007851343894 | 0.015631922500 | +0.354777732948 | Scalar lower |
| 2 | 0.015267452591 | 0.036781034455 | -0.075701850201 | Tensor lower |

Both top-cutoff orderings survive the registered halved-separation check. Some higher reported states have
larger errors; the passing residual statement is about the vacuum and the two target levels, not every
one of the six reported levels in each sector. The earlier coupling-squared-2 unresolved decisions at
N=10 and N=12 are reproduced, not overwritten retrospectively.

All outputs retain `separation_verified=false`, `certified=false`, and unconditional `ORDER_UNRESOLVED`.

## Precision, replay and resources

The run assembles and contracts at 60 decimal digits; the NumPy eigensolve remains float64. At N=24, each
sector and control was reassembled at 100 digits. All six reported energies and residuals per sector agree
exactly after conversion to the stored output precision: recorded maximum absolute energy differences and
maximum relative residual differences are zero. This is not a claim of 100-digit eigenvalue accuracy.

All 36 archives retain the high-precision matrix entries as decimal strings, the returned eigenvectors,
energies, residuals, basis labels and precision metadata. Replay recomputes the eigenspectrum and full
residual contractions from those saved matrices, then regenerates the reported gates and ordering. It is
not a second independent reconstruction of every matrix from the physical operator; the separate top-cutoff
reassemblies and the low-cutoff nine-variable comparisons address different parts of that validation.

Execution status: COMPLETE. Verification: PASS. Overall scientific status: UNRESOLVED.

- 16 cases, two sectors each, plus four high-precision repeats.
- 828.343 seconds, about 13.8 minutes, including prerequisites and verification.
- Peak sampled worker RSS: 302,891,008 bytes, about 288.9 MiB.
- CPU-only mpmath Python backend, one BLAS thread; no GPU solve in this run.
- Fixed limits of 45 minutes and 8 GiB respected. No threshold or budget was raised.

These timings describe this run, not a matched hardware benchmark against the earlier GPU calculation.

## Superpotential clarification

The factorization gives a floor relative to this Hamiltonian's energy origin; it is not the vacuum-to-first-
excitation gap. The original endpoint difference in W can also be supported by an explicit path in this
finite model: q(t)=a(t)R/g, R in SO(3), a(t)=1/(1+exp(-(t-t0))). Then a'=a(1-a), so
q'=q-g cofactor(q)=grad W. It connects 0 to R/g and saturates the square-completion action bound,
with W(R/g)-W(0)=1/(2g^2). This supplies the path that an endpoint-only argument would leave implicit;
it does not assert a new result about arbitrary four-dimensional fields.

## Scope and reproduction

The local residual bottleneck is cleared for these measured target states. Higher-spin competitors at the
new cutoffs, probe-tail gates, state-geometry requirements, certified separation, held-out crossing tests
and the continuum problem remain outside this run. Neither COMPLETE nor verification PASS closes those
scientific gates. The through-32 sweep was not run; this was the predeclared bounded first pass through 24.

Evidence: [`controls-abd81f97`](../../research/sector-review-20261124/controls-abd81f97/).
TDD: [`tdd`](../../research/sector-review-20261124/tdd/).
Both contain the logs, receipts and source hashes; the run also contains byte-preserving source snapshots,
resource samples and matrix archives. Nothing was deployed to a live GIGI service.

With the pinned source files retained, replay without running a new experiment:

```powershell
python scripts/run_ym_helike_sectors.py research/sector-review-20261124/controls-abd81f97 --verify
```

Use a fresh output directory to repeat the registered run. Existing evidence cannot be overwritten, and a
source mismatch must be addressed by using the pinned revision, not by editing saved hashes.
