# The scalar sector on its own: a solver that converges tighter

24 November 2026. **Software built and tested. Registered runs NOT YET DONE.** The numbers in the last section come
from a development check on the review laptop. They are not receipts.

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
- **Precision.** Linear algebra runs in mpmath at 60 digits, because the monomial Gram matrix is ill-conditioned:
  it loses about 6 digits at cutoff 14 and 13 at 28. The residual is read from the exterior Gram K̃ − H̃², so a
  small residual is not lost to cancellation.

## An exact structure found on the way

The potential is a perfect square:

  V = ½|∇W|²,  W = ½T − g det q.

det q is harmonic and ΔW = 9, so H = ½A†A + 9/2 with A = ∇ + ∇W. Three consequences:
- **An exact floor.** Every eigenvalue exceeds 9/2 when g ≠ 0, because e^(−W) cannot be normalised. This is a
  new exact gate, and every computed ground energy passes it. E₀ − 9/2 is 0.9304 at g² = 1 and 1.7480 at g² = 2.
- **Exact vacua.** Both vacua are the critical points of W: q = 0 and the orbit q = R/g with R ∈ SO(3).
  W differs between them by exactly 1/(2g²), the Bogomolny action of a tunnelling path.
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
python -m unittest scripts/test_ym_helike_j0.py
python scripts/check_ym_helike_j0_mutations.py OUT/mutations.json
python scripts/run_ym_helike_j0.py OUT --cases 1:1.7 2:2.3 --cutoffs 10 12 14 16 18 20 22 24 26 28 30 32 36 40
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

| Cutoff | n | E_s | ε_s | ε_s′ (next scalar) | Temple τ_s |
| --- | --- | --- | --- | --- | --- |
| 14 | 47 | 7.70004 | 1.11×10⁻¹ | 1.65×10⁻¹ | — |
| 16 | 64 | 7.7000470 | 4.97×10⁻² | 9.19×10⁻² | 1.5×10⁻³ |
| 20 | 108 | 7.6999908 | 1.16×10⁻² | 2.70×10⁻² | 7.9×10⁻⁵ |
| 24 | 169 | 7.69998797 | 2.77×10⁻³ | 7.85×10⁻³ | 4.4×10⁻⁶ |
| 28 | 249 | 7.69998786 | 7.28×10⁻⁴ | 2.26×10⁻³ | 3.0×10⁻⁷ |

The allowance is 0.0227. The scalar passes the inherited residual gate from cutoff 24 on. The decay is about
four times per four degrees, so the slowdown seen at cutoff 14 did not persist. E₀ settles to 5.43038142, with
ε₀ = 1.5×10⁻⁴ at cutoff 28. The Temple width of the scalar is now far below any expected |D|. A tensor-lower
verdict therefore depends almost entirely on the tensor's upper value.

## What this does not do

- It does not solve the spin-2 sector. The tensor still comes from the 9-variable solver, or from a spin-2 module
  over the same invariant ring, which is the next build.
- It does not measure the held-out couplings.
- It does not verify the separation.
- It is not interval arithmetic.
- It does not reach the continuum.
