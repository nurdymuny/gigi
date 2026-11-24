# Existing-control ordering pilot

23 November 2026, Pacific time. Authorized bounded pilot; commit and push the implementation before execution.
This pilot fixes the four review issues. It does not implement all geometry gates or the held-out crossing survey.

## Frozen scope

- Same Hamiltonian, coordinates, domain and probes as the crossing registration. No changed physical terms.
- Existing couplings only: g squared 1 at alpha 1.7; g squared 2 at alpha 2.3. The latter is the expanded
  exploration grid's ground-energy optimum, not the older grid's 2.0. No per-level width selection.
- Cutoffs 10, 12, 14 in that order: six cases. No held-out coupling, N=16 construction, or enlarged grid.
- For each case: ground, lowest scalar excitation, NEXT DISTINCT scalar excitation, lowest complete spin-2
  multiplet, NEXT DISTINCT spin-2 multiplet. Save all finite eigenvalues and selected subspaces.
- CUDA solve, double precision; small CPU reference tests before launch. No automatic CPU fallback.
- Unchanged ceilings: 8 GiB worker RSS, 4 GiB CUDA allocation, 45 minutes including replay. Sample host RSS
  every half second; log startup, phases, failures and completed results immediately. Sampling may overshoot.
- Compute one full residual block at a time to bound temporary memory. A resource refusal is a recorded result,
  not permission to weaken precision, discard rows, or change the budget.

## Mathematical and decision contract

For a metric-orthonormal selected block V, B=V-dagger H V and R=H V-V B in full oscillator coordinates.
Keep the residual Gram matrix R-dagger R, its operator norm, energy endpoints and spin validation separately.
Do not add dimensionless spin tolerances to energy-valued residuals. Proper metric and complex conjugation
are required. Repeated unresolved same-spin copies refuse; a mixed scalar/tensor energy group is split by
Casimir without losing either multiplet.

For each target, beta = next_energy_low - next_full_residual. Its label is always ASSUMED_FROM_RITZ.
The conditional lower estimate is rho_low - residual^2/(beta-rho_low), only when beta > rho_low+residual.
Repeat with beta_half=rho_low+(beta-rho_low)/2, including the guard. A conditional order requires the same
inequality under both choices. Only the lower estimate needed for that inequality must be usable.
Variational upper endpoints use rho_high. eta=1e-8*max(1,abs(scalar_rho_high),abs(tensor_rho_high)).

Every result carries conditional_order, unconditional_order=ORDER_UNRESOLVED, separation_verified=false,
certified=false, and scientific_status=UNRESOLVED. A pilot is not a convergence or crossing acceptance.
No fixture can prove that Ritz data alone detect every missed exact eigenvalue.

## TDD gates

1. Known-spectrum Temple arithmetic and positive/negative/overlap examples, including strict separation guards.
2. Hidden scalar spectrum [0,1,2,4], captured [0,2,4], tensor at 1.5: conditional tensor-lower is allowed,
   but verified separation/unconditional tensor-lower MUST NOT be emitted, even at zero residual.
3. An unused unusable bound does not block the other conditional inequality. Half-separation sensitivity refuses.
4. Full quintets and next distinct same-spin levels, including mixed-spin degeneracy, nonidentity metric and
   complex rotations. Incomplete, missing, repeated or invalid-spin blocks refuse explicitly.
5. Full Hamiltonian action agrees with an independent direct ladder-action oracle at small cutoff; removing
   exterior action, coordinate scaling or conjugation must fail. One block's action must not depend on how
   other blocks were batched. Existing residual measurements agree when applied to the same subspace.
6. Spin leakage is checked independently and never used as an additive energy offset.
7. Width-summary regression on the committed raw table: expanded-grid maximum per-level gain 1.1089676171;
   minimum excited residual 0.3470552370; the older width comparison has its own named baseline.
8. Runner and verifier reject incomplete/corrupt/source-mismatched evidence, invalidate a previous PASS before
   replay, retain failure reasons, enforce budgets, and never manufacture a held-out result.

Save behavioural RED assertion failures before implementation, then GREEN receipts and deliberate mechanism
removal failures. Import errors and unrelated exceptions are not accepted red/mutation evidence. Run only
focused Python tests; no full Rust suite. All scientific gates remain separate from these software checks.

## Evidence and continuation

Use `research/quantum-order-controls-20261123/` for test receipts and immutable `controls-<UTC-id>/` runs.
Each run retains source/spec/test snapshots and hashes, stdout/stderr, resource samples, per-case arrays,
selected subspace identities, residual Grams, conditional decisions and independent replay results.
Verification recalculates the residuals from the saved states, not just the printed JSON. Missing data is not zero.
Failed replay replaces current verification with FAIL; historical files stay available but are not current PASS.

Execution COMPLETE means six controls and replay completed, never that the full experiment passed. Report
conditional feasibility, separation failures and original residual allowances point by point. Do not extrapolate
a result at g squared 2 to a measurement at 1.90. The full crossing run still requires all QC00--QC14 gates,
bounded N=16 construction and the specified state-geometry implementation; this pilot cannot waive them.
