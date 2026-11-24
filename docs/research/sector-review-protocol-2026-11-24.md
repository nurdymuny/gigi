# Sector-solver review and bounded execution

24 November 2026. Review target: `0bc5bd4d`. Specified before the new numerical run.

The invariant formulas and low-cutoff agreement tests are retained. The upstream 24-test suite passes locally,
including all six saved controls. This review corrects numerical and execution contracts, not the Hamiltonian.

## Corrections and tests

1. Residuals: for the returned approximate eigenvector v and energy E, use
   r^2 = [v^T X v + ||Htilde v - E v||^2] / (v^T v), X = Ktilde - Htilde^2.
   Contract in the requested mpmath precision. The old exterior-only expression omitted the finite eigensolver
   error, and clipping every negative value could report zero for invalid data. Refuse materially negative
   residual squares, nonfinite inputs and exterior eigenvalues below the existing -1e-12 gate. Report the
   arithmetic roundoff tolerance for tiny negative contractions; do not claim interval certification.
   A planted inexact eigenpair with X=0 must have NONZERO full residual. An invalid K must refuse.
2. Precision: assembly, Cholesky, transformations and residual contractions use 60 decimal digits; the NumPy
   eigensolve uses float64. A 100-digit reassembly is a precision check of that mixed-precision pipeline, not
   a 100-digit eigensolve. Save Htilde, X, eigenvectors and recomputation diagnostics for the reported levels.
3. Allowance: both sectors use 0.01*max(1,min(gap_scalar,gap_tensor)), including the vacuum residual.
   A planted case must fail where the old, larger per-channel allowance passed. Above the audited full-basis
   cutoffs, other spins have not been excluded: label this a J=0/J=2 sector-only check, NOT a verified global
   residual/convergence gate. Also report the sufficient conservative residual check <=0.01 separately.
   Scalar-only runs cannot infer a full-spectrum allowance and must say so.
4. Evidence: no output-directory reuse; exact frozen coupling/width identities; increasing registered cutoffs;
   finite JSON; source revision, hashes and byte-preserving snapshots; startup and per-sector event logs;
   stdout/stderr, sampled RSS and elapsed-time supervision; saved arrays; no source drift or stale PASS;
   failed gates exit nonzero. Precision comparison refuses missing/short/nonfinite arrays. Prerequisite tests
   may not silently skip missing evidence. Verification checks exact case identities and replays the saved
   finite matrices, residual contractions, and derived reports. Full high-precision reassembly at the top
   cutoff is separately retained. Matrix replay alone is not an independent high-precision reconstruction.

## Frozen first pass

- Existing controls only: g squared 1 / alpha 1.7 and g squared 2 / alpha 2.3.
- Both sectors at N = 10,12,14,16,18,20,22,24; six reported levels per sector.
- Assembly/residual precision 60; reassemble each control's N=24 at 100 digits.
- CPU-only mpmath pipeline, float64 eigensolve; one BLAS thread. No backend or dependency changes.
- Same ceilings: 8 GiB worker RSS and 45 minutes including prerequisites and verification. Stop and retain
  partial evidence on exhaustion. Do not run the unbounded/default-through-32 sweep afterward automatically.
- All conditional-separation labels and `scientific_status=UNRESOLVED` remain unchanged.
- No held-out couplings, no new width optimization, no continuum or full-state-geometry claim.

RED fixtures precede fixes, GREEN focused tests and assertion-rejected mechanism removals precede execution.
Commit and push the reviewed implementation first. Keep every result and log under
`research/sector-review-20261124/`, with separate directories for TDD and each run.

## Documentation corrections

The imported scalar development table's N=14 energy 7.70004 contradicts its saved control
7.7004303148103945 and its own N=16 entry. Mark the table unverified instead of treating it as a receipt.
Its stated residual 0.0116 at N=20 is already below 0.0227, so 'first passes at N=24' is unsupported.
Actual basis counts: scalar/tensor = 47/57 at N=14, 249/375 at N=28, 632/1029 at N=40.
The shifted ground-energy bound 9/2 is relative to this Hamiltonian's energy origin, not an excitation gap.
Strict non-attainment by a normalizable eigenfunction does not alone prove a strict spectral-infimum bound.
Any instanton/Bogomolny action interpretation needs a path saturating the bound; a difference of W alone
does not establish that action. These distinctions do not change the registered model or energy computation.

The published model also discusses boundary extensions; this run retains the earlier Cartesian Friedrichs
domain, not those extensions: [Acharyya and Balachandran](https://arxiv.org/abs/1702.06430).
The eigensolver interface is documented by [NumPy](https://numpy.org/doc/stable/reference/generated/numpy.linalg.eigh.html).
