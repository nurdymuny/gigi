# Helike quantum probe: convergence amendment (draft Amendment 2)

**Bee Rosa Davis | Davis Geometric**
23 November 2026. Draft amendment to the
[quantum-probe registration](helike-quantum-probe-preregistration-2026-11-23.md). **Status: NOT RUN** as a
registered survey. The model, probes, couplings and acceptance thresholds are unchanged.

## Why

The first survey (`20261124T021903Z-c326a3c5`) ended **UNRESOLVED**. The finite matrices were solved accurately and
cross-checked; the gate that failed was HP08. Successive cutoffs had not settled, and the full-space residuals of
the reported levels were 30 to 70 times the allowance (for example 0.899 against 0.023 at g² = 1, N = 12).

The cause is visible in the construction. Every basis function carries the fixed Gaussian exp(−q·q/2) of the free
oscillator, while the interacting ground state is wider: the cubic term −3g det q and the quartic term pull it
toward the second classical vacuum at q = I/g. A polynomial basis of fixed width needs high degree to follow that
shape. The registration's own test, the action of the full polynomial Hamiltonian beyond the cutoff, measures exactly
this. It is the right gate; the basis is what needs to change.

## What changes

1. **Basis width.** Use exp(−x·x/2) with q = x/√α. The Hamiltonian becomes
   H_α = α H₀ + (1/(2α) − α/2) tr − 3g α^(−3/2) det + g² α^(−2) V₄,
   which is the registered operator in rescaled coordinates. α = 1 reproduces the registered matrix exactly. Only
   the basis changes: the untruncated spectrum does not depend on α.
2. **Frozen choice of α.** For each coupling, α* minimises the ground energy over the grid
   {0.8, 0.9, 1.0, 1.1, 1.2, 1.35, 1.5, 1.7, 2.0} at cutoff 10. Only E₀ is read; no excited state, probe or gap enters
   the choice, and the full E₀(α) table is published. If α* falls on the edge of the grid, that is reported, and the
   grid is not extended within this run.
3. **Cutoffs.** N = 8, 10, 12, 14, 16 at α*, and the registered α = 1 at the same cutoffs beside it. N = 18 and 20 only
   by explicit option (dimensions 6,721 and 11,011); a run that would exceed the GPU budget refuses rather than
   falling back.
4. **Spin labels and exact zeros.** Each complete level carries its spatial spin from ⟨J²⟩ = J(J+1) and checks that
   its multiplicity is 2J + 1. From a spin-0 vacuum the H2 channel reaches only J = 2 and H0 only J = 0. A lowest
   level of another spin is reported as **SELECTION_RULE_ZERO**, a symmetry zero, rather than as a tiny or unresolved
   overlap (the first survey's weights of order 10⁻²⁹ are rounding of exact zeros).
5. **Residual enclosures.** For each reported level, with ε the full-space residual of the normalised state:
   - **Weinstein, rigorous:** an eigenvalue of the untruncated operator lies in [E − ε, E + ε];
   - **Kato–Temple, conditional:** E − ε²/(b − E) ≤ λ ≤ E + ε²/(E − a), if no other true eigenvalue lies in (a, b).
     Here a and b are neighbouring Ritz values, which are not proven bounds on the true neighbours, so the interval
     is labelled conditional.

   Upper Ritz bounds λ_k ≤ E_k hold by min–max. A rigorous lower bound for the gap needs a proven lower bound on the
   second eigenvalue (Temple–Lehmann with a certified separation); that is the step after this one, not claimed here.

Everything else is as registered: thresholds 10⁻³ (energies and gaps) and 0.01 (weights) between two successive
cutoff increments, residual allowance 10⁻² max(1, gap), tail fraction 10⁻³, robust detection weight 10⁻⁴.

## What had been seen when this was written

- The registered survey and its UNRESOLVED verdict.
- Code validation on CPU at N = 4 and 6 (not a survey): α = 1 reproduces the registered gaps to all printed digits
  (2.867846745506, 2.037058771365, 4.351929591164 at N = 4); the spin labels reproduce the registered "lowest spin"
  column; at g² = 2, α = 1.25 lowers E₀ at N = 4, 6, 8 (by 0.183, 0.128, 0.064), and in a smoke test with
  α ∈ {1, 1.25, 1.5}, α = 1.5 reduced the worst residual at N = 6 from 5.0 to 2.0. No result at N ≥ 8 with α ≠ 1, and
  no probe weight with α ≠ 1, was examined.

## Gates added

| Gate | Positive check | Defect that must fail |
| --- | --- | --- |
| HP11 Width | α = 1 matrix equals the registered matrix; scaled exterior equals the registered exterior at α = 1 | Omit the trace term from the exterior action at α ≠ 1; scale the probes but not the Hamiltonian |
| HP12 Spin | Every level's ⟨J²⟩ within 10⁻⁶ of J(J+1), multiplicity 2J + 1, vacuum J = 0 | A level split across a degenerate multiplet; a vacuum of nonzero spin |
| HP13 Enclosure | Weinstein interval from the full residual of a normalised state; Kato–Temple only when ε is below both neighbour distances | Use the in-basis residual; drop the exterior part; call the conditional interval rigorous |

## How to run

```text
python scripts/run_ym_helike_convergence.py research/helike-quantum-probe-20261123 --device cuda
```

Each run writes a new `convergence-<run id>/` folder with a RUNNING receipt first and an atomic COMPLETE or FAILED
receipt. The registered runner, `scripts/run_ym_helike_quantum.py`, is unchanged.
