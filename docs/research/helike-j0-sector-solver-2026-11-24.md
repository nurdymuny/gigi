# The scalar and tensor sectors on their own: solvers that converge tighter

24 November 2026. **At registration: software built and tested; runs NOT YET DONE.** The numbers in the last section come
from a development check on the review laptop. They are not receipts.

**Review amendment:** [bounded execution and numerical corrections](sector-review-protocol-2026-11-24.md).
The first authorized run is both sectors through cutoff 24, with 60-digit assembly and residual contractions,
a float64 eigensolve, and 100-digit reassembly of the top cutoff. Examples below use this reviewed scope.

## Why

In the six-control pilot the scalar's residual was the slow one. At g² = 1 it fell by 2.18 times from cutoff 10 to
12, then by only 1.43 times from 12 to 14. If that rate held, the 9-variable basis would need about cutoff 24 to meet
the inherited gate. The basis would have more than 50,000 functions, and the assembly was already at 4.4 GiB at
cutoff 14. But the scalar is a colour singlet with spin 0. Such a state is invariant under colour and space
rotations, so it lives in a much smaller space.

## What it is

`scripts/ym_helike_j0.py`. A spin-0 colour-singlet polynomial is a polynomial in three invariants:
- T = tr(qqᵀ);
- P = tr((qqᵀ)²);
- d = det q.

The invariant ring is free in these three. H maps that ring, times the Gaussian, to itself. So the
spin-0 part of the cutoff-N basis is exactly

  { T^a P^b d^c e^(−x·x/2) : 2a + 4b + 3c ≤ N },  x = √α q.

That is 47 functions at cutoff 14, against 2,178 for all spins; 249 at 28; 632 at 40.

Inside this invariant subspace the Ritz values are still upper bounds on the exact spin-0 levels. The residual is
still the full-operator residual. So the Temple rule of the crossing registration applies unchanged.

The parts are exact:
- **Gradient table.** The gradient inner products and Laplacians of T, P and d, written back in T, P and d.
  For example, ∇P·∇P = 16 tr(Q³) = −8T³ + 24TP + 48d², and ΔP = 28T.
- **Moments.** Gaussian moments are exact rationals, from integration by parts along ∇T, ∇P and ∇d.
- **Precision.** Assembly and orthonormalization run in mpmath at 60 digits; the eigensolve uses float64:
  it loses about 6 digits at cutoff 14 and 13 at 28. The residual is read from the exterior Gram K̃ − H̃², so a
  small residual is not lost to cancellation. The reviewed calculation adds the returned eigenpair's projected
  error and contracts both terms in mpmath; it refuses materially negative values instead of clipping them to zero.

## An exact structure found on the way

The potential is a perfect square:

  V = ½|∇W|²,  W = ½T − g det q.

det q is harmonic and ΔW = 9, so H = ½A†A + 9/2 with A = ∇ + ∇W. Three consequences:
- **An exact floor for this domain and energy origin.** H >= 9/2; a normalizable eigenfunction cannot attain
  equality when g != 0, because e^(-W) is not normalizable. This is not a vacuum-to-excitation gap, and strict
  non-attainment alone does not establish a strict bound on the spectral infimum. This is a
  new exact gate, and every computed ground energy passes it. E₀ − 9/2 is 0.9304 at g² = 1 and 1.7480 at g² = 2.
- **Exact vacua.** Both vacua are the critical points of W: q = 0 and the orbit q = R/g with R ∈ SO(3).
  W differs between them by exactly 1/(2g²). This is the action bound associated with the superpotential;
  identifying a tunnelling action also requires a path saturating that bound, not just the endpoint difference.
- **Known structure.** This is the Chern–Simons gradient structure of Yang–Mills on the three-sphere, here in
  the homogeneous sector.

The scalar carries the larger ⟨det q⟩, which points toward the second vacuum, and the barrier falls as g grows.
That may be why the scalar converges slowly in a Gaussian centred at the origin. I have not shown it.

## Tests (12 pass; 6 of 6 mutants rejected by assertion)

`scripts/test_ym_helike_j0.py`:
- Every gradient inner product and Laplacian, checked against finite differences.
- The superpotential identity at random points.
- Exact moments against an independent 9-variable symbolic expansion.
- The free oscillator, g = 0 with α = 1: levels are 9/2 plus the degree, with zero residual.
- The floor E₀ > 9/2 at four couplings.
- Ritz values do not rise with the cutoff.
- 40 and 90 digits agree.
- An explicit assertion when the solver refuses a non-symmetric operator.
- Agreement with all six saved GPU controls from the 9-variable solver. Energies agree to 2×10⁻¹⁴ and residuals
  to 8×10⁻¹⁵, for the vacuum, the scalar and the next scalar at cutoffs 10, 12 and 14. These are two independent
  implementations.

`scripts/check_ym_helike_j0_mutations.py` makes six mutations:
- a wrong ΔP;
- a sign flip in ∇d·∇d;
- a wrong ∇P·∇d;
- a dropped d² in ∇P·∇P;
- a dropped cubic term in H;
- promoting the separation to verified.

Each one fails a test assertion. None produces an unexpected error.

## How to run it

```
python -m unittest discover -s scripts -p test_ym_helike_j0.py
python scripts/check_ym_helike_j0_mutations.py OUT/mutations.json
python scripts/run_ym_helike_j0.py OUT --cases 1:1.7 2:2.3 --cutoffs 10 12 14 16 18 20 22 24
```

The runner needs only the CPU, with no GPU and little memory. The widths are the pilot's frozen widths.
The receipt says RUNNING, then COMPLETE or FAILED. It records:
- every level and residual;
- the floor;
- monotonicity;
- conditioning;
- the scalar's Temple quantities, with `certified: false` and `separation_verified: false`;
- the inherited residual gate;
- a recomputation of the top cutoff at 100 digits.

Cutoff 28 took about 140 seconds on the laptop. Cutoff 40 will take several times longer.

## Development check, g² = 1, α = 1.7 (laptop; not a receipt)

**Unverified historical table:** the N=14 scalar energy below is inconsistent with the saved control
7.7004303148103945 and the listed N=16 value. The text's first-pass claim at N=24 is also inconsistent with
its own N=20 residual 0.0116 < 0.0227. Retained as development history, not accepted numerical evidence.

| Cutoff | n | E_s | ε_s | ε_s′ (next scalar) | Temple τ_s |
| --- | --- | --- | --- | --- | --- |
| 14 | 47 | 7.70004 | 1.11×10⁻¹ | 1.65×10⁻¹ | — |
| 16 | 64 | 7.7000470 | 4.97×10⁻² | 9.19×10⁻² | 1.5×10⁻³ |
| 20 | 108 | 7.6999908 | 1.16×10⁻² | 2.70×10⁻² | 7.9×10⁻⁵ |
| 24 | 169 | 7.69998797 | 2.77×10⁻³ | 7.85×10⁻³ | 4.4×10⁻⁶ |
| 28 | 249 | 7.69998786 | 7.28×10⁻⁴ | 2.26×10⁻³ | 3.0×10⁻⁷ |

The stated allowance is 0.0227; the table would already satisfy that scalar-only test at N=20, if verified. The decay is about
four times per four degrees, so the slowdown seen at cutoff 14 did not persist. E₀ settles to 5.43038142, with
ε₀ = 1.5×10⁻⁴ at cutoff 28. The Temple width of the scalar is now far below any expected |D|. A tensor-lower
verdict therefore depends almost entirely on the tensor's upper value.

## The spin-2 sector (added the same day)

`scripts/ym_helike_j2.py`. Colour invariants are generated by G = qᵀq and det q. A spin-2 colour-singlet state is
a traceless symmetric tensor field, and those covariants are a free module over the same invariant ring. Its
generators are A₁ = dev G (degree 2) and A₂ = dev G² (degree 4), because Cayley–Hamilton reduces G³. So the
spin-2 part of the cutoff-N basis is { m(T,P,d)·A_k e^(−x·x/2) : deg m + 2k ≤ N }. That is 57 multiplets at
cutoff 14, 375 at 28 and 1,029 at 40.

The exact tables:

| Quantity | Value |
| --- | --- |
| ∇T·∇A₁, ∇T·∇A₂ | 4A₁, 8A₂ |
| ∇P·∇A₁, ∇P·∇A₂ | 8A₂, 16(T A₂ − e₂A₁) |
| ∇d·∇A₁, ∇d·∇A₂ | 0, 4dA₁ |
| ΔA₁, ΔA₂ | 0, 22A₁ (the trace of ΔG² = 22G + 2T·I gives ΔP = 28T, as before) |
| A₁:A₁ | P − T²/3 |
| A₁:A₂ | tr G³ − TP/3 |
| A₂:A₂ | tr G⁴ − P²/3 |

Inner products sum the Frobenius contraction over the five members. By Schur's lemma the residual is then the
per-member full-space residual, the operator norm of the 5×5 residual Gram.

Tests: `scripts/test_ym_helike_j2.py`, 12 tests.
- Every table entry against finite differences.
- The free oscillator is exact.
- Every level is above 9/2.
- Ritz values do not rise with the cutoff; 40 and 90 digits agree.
- The branch passes on the measured residual.
- **Completeness.** The module dimension equals the number of spin-2 multiplets in the 9-variable basis at
  cutoffs 10, 12 and 14 (24, 38, 57). So A₁ and A₂ generate the whole sector, not just part of it.
- **Agreement with the GPU controls.** Tensor and next-tensor energies agree to about 10⁻¹⁴ and residual
  operator norms to about 5×10⁻¹⁵.
- **The pilot's verdicts, reproduced.** Feeding both sector solvers into `quantum_ordering.ordering` reproduces
  every conditional verdict of the six-control pilot. That includes BOUNDS_OVERLAP at cutoff 10 and
  SEPARATION_SENSITIVE at cutoff 12 for g² = 2.

`scripts/check_ym_helike_j2_mutations.py`: seven mutants (wrong ΔA₂, ∇P·∇A₂, ∇d·∇A₂, ∇P·∇A₁, two wrong
contractions, a zeroed tensor residual). Each fails by assertion. The six scalar mutants still do.

## Both sectors together

```
python -m unittest discover -s scripts -p test_ym_helike_j0.py
python -m unittest discover -s scripts -p test_ym_helike_j2.py
python scripts/check_ym_helike_j0_mutations.py OUT/mutations_j0.json
python scripts/check_ym_helike_j2_mutations.py OUT/mutations_j2.json
python scripts/run_ym_helike_sectors.py OUT --cases 1:1.7 2:2.3 --cutoffs 10 12 14 16 18 20 22 24
```

The runner is CPU only and uses little memory. It records, per cutoff:
- both sectors' levels and residuals;
- a shared J=0/J=2 residual allowance, not a verified full-spectrum convergence gate; the vacuum is included;
- D = E_t − E_s;
- the full `quantum_ordering.ordering` output (conditional order and reason, beta and beta_half, the Weinstein
  secondary order, `certified: false`, `separation_verified: false`, unconditional `ORDER_UNRESOLVED`);
- the 9/2 floor;
- monotonicity;
- a recheck of the top cutoff at 100 digits.

It uses the pilot's couplings and frozen widths. It does not choose widths or touch the held-out couplings. The
tensor dominates the time; cutoff 32 is about 550 basis functions.

## What this does not do

- It does not measure the held-out couplings.
- It does not verify the separation.
- It is not interval arithmetic.
- It does not reach the continuum.
- It covers spins 0 and 2 only. A spin-4 or other competitor below both branches is still checked by the full
  spectrum audit of the 9-variable solver, at the cutoffs that solver reaches.
