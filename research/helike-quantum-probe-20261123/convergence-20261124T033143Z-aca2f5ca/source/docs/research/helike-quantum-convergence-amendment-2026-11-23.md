# Helike quantum probe: convergence amendment (draft Amendment 2)

**Bee Rosa Davis | Davis Geometric**
23 November 2026. Draft amendment to the
[quantum-probe registration](helike-quantum-probe-preregistration-2026-11-23.md). **Status: NOT RUN** as a
registered survey. The model, probes, couplings and acceptance thresholds are unchanged.

## Why

The first survey (`20261124T021903Z-c326a3c5`) ended **UNRESOLVED**. The finite matrices were solved accurately and
cross-checked; the gate that failed was HP08. Successive cutoffs had not settled, and the full-space residuals of
the reported levels were 30 to 70 times the allowance (for example 0.899 against 0.023 at g² = 1, N = 12).

One hypothesis is that the fixed Gaussian exp(−q·q/2) is a poor finite-basis envelope for the interacting states.
The cubic and quartic terms change the potential, which also has a classical vacuum at q = I/g. That does not
establish that the ground state is wider, or that width alone explains the residuals. Alpha above one narrows the
Gaussian in q; alpha below one broadens it. This amendment tests whether choosing that width reduces the measured
truncation error. The original exterior-residual gate remains the judge; improvement is not assumed.

## What changes

1. **Basis width.** Use exp(−x·x/2) with q = x/√α. The Hamiltonian becomes
   H_α = α H₀ + (1/(2α) − α/2) tr − 3g α^(−3/2) det + g² α^(−2) V₄,
   which is the registered operator in rescaled coordinates. α = 1 reproduces the registered matrix exactly. Only
   the basis changes: the untruncated spectrum does not depend on α. Physical observables transform too:
   H2(x) / α and H0(x) / α². Their kernels and excited norms therefore carry factors α⁻² and α⁻⁴.
   Normalized weights and discarded fractions cancel these factors; absolute responses do not.
2. **Frozen choice of α.** For each coupling, α* minimises the ground energy over the grid
   {0.8, 0.9, 1.0, 1.1, 1.2, 1.35, 1.5, 1.7, 2.0} at cutoff 10. Only E₀ is read; no excited state, probe or gap enters
   the choice, and the full E₀(α) table is published. If α* falls on the edge of the grid, that is reported, and the
   grid is not extended within this run.
3. **Cutoffs.** N = 8, 10, 12, 14, 16 at α*, and the registered α = 1 at the same cutoffs beside it. N = 18 and 20 only
   by explicit option (dimensions 6,721 and 11,011); a run that would exceed the GPU budget refuses rather than
   falling back.
4. **Spin labels and symmetry zeros.** Diagonalize J² inside each complete energy level, rather than rounding its
   average. A level can contain several spins and repeated irreducible representations. Each spin's multiplicity
   must be an integer multiple of 2J + 1. Check Casimir eigenvalues and leakage between energy levels to 10⁻⁶;
   refuse unresolved or incomplete multiplets and a non-scalar vacuum. From a spin-0 vacuum H2 reaches only J = 2
   and H0 only J = 0. Check that the vacuum-subtracted probe response obeys that rule (relative amplitude residual
   at most 10⁻⁶). Declare **SELECTION_RULE_ZERO** only if the entire lowest level excludes the allowed spin and
   its measured weight is at most 10⁻¹². Preserve the measured number. The exact statement comes from the
   registered operator's symmetry; the numerical check is not itself a proof of exact symmetry.
5. **Residual enclosures.** For each reported level, with ε the full-space residual of the normalised state:
   - **Weinstein, numerical estimate:** the exact theorem places spectrum within [E − ε, E + ε] for an exact
     full-space residual. Here the residual is floating-point, not a certified upper bound. Report
     `certified: false`; no rigorous computer enclosure is claimed. Include the energy-group spread in the radius
     when reporting a common center for a multiplet;
   - **Kato–Temple, conditional:** E − ε²/(b − E) ≤ λ ≤ E + ε²/(E − a), if no other true eigenvalue lies in (a, b).
     Here a and b are neighbouring Ritz values, which are not proven bounds on the true neighbours, so the interval
      is labelled conditional and uncertified. Enforce the stated ε < neighbour-distance guard, including the
      upper neighbour for the ground state. With no upper neighbour, do not emit this interval.

   Upper Ritz bounds λ_k ≤ E_k hold by min–max. A rigorous lower bound for the gap needs a proven lower bound on the
   second eigenvalue (Temple–Lehmann with a certified separation); that is the step after this one, not claimed here.

Everything else is as registered: thresholds 10⁻³ (energies and gaps) and 0.01 (weights) between two successive
cutoff increments, residual allowance 10⁻² max(1, gap), tail fraction 10⁻³, robust detection weight 10⁻⁴.

## What had been seen when this was written

- The registered survey and its UNRESOLVED verdict.
- Code validation on CPU at N = 4 and 6 (not a survey): α = 1 reproduces the registered gaps to all printed digits
  (2.867846745506, 2.037058771365, 4.351929591164 at N = 4); the spin labels reproduce the registered "lowest spin"
  column; at g² = 2, α = 1.25 lowers E₀ at N = 4, 6, 8 (by 0.183, 0.128, 0.064), and in a smoke test with
   α ∈ {1, 1.25, 1.5}, α = 1.5 reduced the worst residual at N = 6 from 5.0 to 2.0. The N = 8 ground-energy
   observation above was seen; the earlier blanket statement that no N ≥ 8 result was seen was incorrect.
   No larger-cutoff width survey or alpha-dependent probe-weight survey is reported here.

## Gates added

| Gate | Positive check | Defect that must fail |
| --- | --- | --- |
| HP11 Width | α = 1 reproduces the registered matrix/exterior; independent scaled action and physical probe norms agree | Omit the exterior trace term; omit either observable's scale factor |
| HP12 Spin | Resolve all Casimir components, complete spin copies, scalar vacuum, correct probe response | Average mixed spins into a false zero; split a multiplet; invalid vacuum or probe |
| HP13 Enclosure | Finite full residual, group-spread allowance, uncertified labels, neighbour guards | Drop exterior action; omit ground upper-neighbour guard; label floating estimates certified |

## How to run

```text
python scripts/run_ym_helike_convergence.py research/helike-quantum-probe-20261123 --device cuda
```

## Review hold and evidence contract

**Do not launch the survey until review of this correction commit.** Focused regression and deliberate
mechanism-removal tests are permitted; they are not the registered survey. No changed thresholds, model,
couplings, alpha grid, or alpha-selection rule are authorized.

The new runner must execute HP11--HP13 tests and mutation checks before selecting alpha. Each case checks spin
and observable validity, saves physical H2/H0 matrices, H, S, J², energies, eigenvectors, alpha, and coupling in a
hashed NPZ, and immediately records its result. Replay checks the arrays and the reported responses. Source
snapshots include the amendment, tests and verifier, not just the two Python files. Source drift, missing gates,
corrupted arrays, partial runs and failed checks cannot produce an accepted convergence status.

Restore the original 45-minute / 8-GiB host-memory supervisor and 4-GiB CUDA allocation limit. Estimate dense
and exterior-space storage before construction; these estimates are not hard bounds on Python or solver
overhead, so retain the supervisor and report measured peaks. Host monitoring is sampled and can overshoot
between checks. Keep only one assembled cutoff at a time. No CPU fallback is allowed.

Every run starts with RUNNING and ends atomically as COMPLETE, FAILED or RESOURCE_STOP. COMPLETE means the
requested sequence finished, not that it converged. Numerical convergence additionally requires all amendment
prerequisites and the unchanged HP08 thresholds. This convergence experiment does not run the HP09 utility
comparison and does not close the original survey's full acceptance gate. The original runner is unchanged.

## Execution note: eigenpair ordering

After authorization to run, attempt `20261124T032926Z-92f909f6` passed the prerequisites and selected alpha,
then stopped before saving its first convergence case. A bounded reproduction at N=8, g2=1, alpha=1 found
GPU energies 19.85429364793703 followed by 19.854293647937027, an inversion of 3.552713678800501e-15.
All energies and residuals were finite; the solver residual was 1.1882286578357973e-16.

Before retrying: stable-sort the eigenvalues and apply exactly the same permutation to eigenvector columns,
before residual validation and energy grouping. Do not round, merge, average or perturb any eigenvalue;
do not weaken the ordered-input guard or change any threshold. Plant both reversed distinct eigenpairs
and a one-ULP inversion in a nonidentity metric. Removing the sort must fail that test. Preserve the worker's
specific failure reason in the terminal receipt instead of replacing it with a generic exit message.
The first attempt and its source snapshot remain intact. This is an ordering correction, not a physics result.
