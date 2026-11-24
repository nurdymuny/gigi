# Pre-registration: a second envelope on the second vacuum (the "dome" test)

24 November 2026. **REGISTERED BEFORE MEASUREMENT. Code and tests committed with this document. Not run.**

## Why

The razor observation: a straight edge works on a curved face when it rides on a dome whose curvature is matched
to the face, because contact sets the frame locally. The sector solvers already did half of this. They measure each
state in the frame its own symmetry dictates, and the residual fell from 0.11 to 0.003.

The other half is position. Every basis function so far is centred at q = 0. W = T/2 − g det q has a second critical
point on the orbit {R/g : R ∈ SO(3)}, and the logistic path q(t) = a(t)R/g is an exact gradient-flow line from 0 to
that orbit, with ΔW = 1/(2g²). The scalar has the larger ⟨det q⟩, which points along that path, and in the 9-variable
basis it was the slow one to converge. My hypothesis is that the scalar has tunnelling character: it reaches toward
the second vacuum. If so, functions seated on the second vacuum should help the scalar much more than the vacuum.

## The dome functions

The Haar average over SO(3) of a Gaussian of width β centred on the orbit, times a low-degree invariant m:

  Φ_m(q) = m(T,P,d) · e^(−βT/2) · F_K(κq),  κ = β/g,  F(X) = ∫ dR exp(tr RᵀX).

F has an exact series in the invariants. With λ = (s₁+s₂+s₃, s₁−s₂−s₃, −s₁+s₂−s₃, −s₁−s₂+s₃) and the
Dirichlet(½,½,½,½) law of the squared quaternion components:

  F = Σₖ cₖ/(k+1)!,  Σₖ cₖ tᵏ = (1 − 2T t² − 8d t³ + (2P − T²) t⁴)^(−1/2).

The truncation F_K (K = 30) is used as given. Every dome function is therefore a specific invariant polynomial
times a Gaussian, and every matrix element is exact (Gaussian moments with mixed widths). Ritz values stay upper
bounds, and residuals stay full-operator residuals. Truncation changes the trial function, not the validity of
the bounds. Code: `scripts/ym_helike_dome.py`.

## Fixed design

| Item | Frozen choice |
| --- | --- |
| Model, sector | The registered Hamiltonian; J = 0 colour singlets only |
| Couplings and widths | The controls: g² = 1 with α = 1.7, g² = 2 with α = 2.3 (frozen, as before) |
| Origin cutoffs | N = 12, 14, 16, 18 (comparators up to N + 2 = 20) |
| Dome sizes | n_d = 0, 3, 4: m of degree ≤ n_d, giving 1, 3 and 5 dome functions |
| Series truncation | K = 30; sensitivity K = 36 at every N = 18 point |
| Dome width β | Grid {0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0}, chosen per coupling at N = 10, n_d = 0, by the smallest second J = 0 Ritz value (the lowest-excited-scalar upper bound, a valid variational criterion). Ties go to the smaller β. A grid-edge choice refuses that coupling. |
| Precision | 80 digits for assembly, orthonormalisation and residual contractions; float64 eigensolve; every N = 18 point repeated at 120 digits |

**Exposure, disclosed.** While timing the code I computed one dome function (n_d = 0) at N = 10, g² = 1, for β = 1
and β = 2. Scalar residual: 0.3479 and 0.3321, against 0.3471 with no dome. Outside-origin fraction of the scalar:
under 10⁻⁴. I looked at nothing else. These values are not used in any registered comparison. The width-selection
rule above uses N = 10 and these were N = 10 values, so the exposure is recorded here rather than hidden.

## Measurements

For each level ℓ ∈ {vacuum, scalar, next scalar}, coupling and N:
- ε_ℓ(N): origin basis alone.
- ε_ℓ(N + dome n_d): origin basis plus the dome functions.
- ε_ℓ(N+2): origin basis at the next cutoff. It adds Δn origin functions: 13, 17, 20, 24 for N = 12, 14, 16, 18.
- The outside-origin fraction 1 − ‖P_origin ψ‖² of each state in the augmented basis: how much of the state the
  origin basis cannot hold.

Gain per added function:

  r_dome(ℓ) = ln(ε_ℓ(N)/ε_ℓ(N + dome)) / n_dome,  r_orig(ℓ) = ln(ε_ℓ(N)/ε_ℓ(N+2)) / Δn.

## Hypothesis and decision rule

Primary comparison: n_d = 4 (five dome functions), at all eight points (four N × two couplings).

- **H1a (efficiency):** r_dome(scalar) > r_orig(scalar). The dome buys more scalar residual per function than
  raising the cutoff does.
- **H1b (specificity):** r_dome(scalar) > r_dome(vacuum). The dome helps the scalar more than the vacuum.

Status:
- `SUPPORTED` if H1a and H1b hold at all eight points;
- `NOT_SUPPORTED` if H1a fails at all eight points;
- `MIXED` otherwise.

Secondary, reported regardless:
- the outside-origin fraction of the scalar against the vacuum's;
- the same comparisons at n_d = 0 and 3;
- the next scalar.

What each outcome means:
- **SUPPORTED:** the scalar has a component that orbit-seated functions represent efficiently. That is consistent
  with tunnelling character. It does not prove it, and it says nothing about the crossing mechanism by itself.
- **NOT_SUPPORTED:** an isotropic Haar-averaged Gaussian is not a good dome. That does not rule out tunnelling
  character. The main alternative is curvature mismatch, the razor's own lesson. At the second vacuum the
  quadratic part of W has frequency 1 for the breathing mode, 2 for the five symmetric-traceless modes and 0 along
  the orbit, while an isotropic dome uses one width for all. A matched, anisotropic dome is the registered next stage.
- **MIXED:** reported as such, with every point shown. No post hoc subset is promoted.

## Validity gates (all must pass, or the status is INVALID)

- **D00** With no dome functions the solver reproduces `ym_helike_j0.solve` (energies and residuals to 1e-12).
- **D01** Adding dome functions never raises a Ritz value (exact min-max property), within 1e-12.
- **D02** Gram conditioning: digits lost < dps − 25 everywhere (the solver refuses otherwise).
- **D03** 120-digit repeats agree: energies to 1e-11, residuals to relative 1e-6.
- **D04** K = 36 does not change any H1a or H1b truth value at N = 18. Otherwise the status is
  `UNRESOLVED_TRUNCATION`.
- **D05** All outside-origin fractions lie in [0, 1].

Tests: `scripts/test_ym_helike_dome.py`.
- Exact Haar coefficients (c₂/3! = T/6, c₃/4! = d/6).
- The λ-polynomial identity at random points.
- The series against a seeded Haar Monte Carlo of the moments.
- Mixed-width moments against the Gamma-function closed form.
- The q-coordinate Hamiltonian action against finite differences.
- D00, D01 and D05 at small cutoffs.
- An explicit assertion on solver refusal.

Mutation check: `scripts/check_ym_helike_dome_mutations.py`.

## Run

```
python -m unittest scripts/test_ym_helike_dome.py
python scripts/check_ym_helike_dome_mutations.py OUT/mutations.json
python scripts/run_ym_helike_dome.py OUT
```

CPU only and small: at most 108 origin functions plus 5 dome functions.

## Limits

- J = 0 only.
- Two control couplings, no held-out couplings.
- Floating point with a float64 eigensolve, not certified.
- The second envelope is one specific dome shape. Its failure is evidence about that shape, not about the state.
