# Cutoff-leakage gate: replace the absolute 1e-10 line with a derived, relative rule

24 November 2026. **AMENDMENT, registered before any further measurement.** It changes one acceptance rule. It
changes no model, cutoff, plateau rule, bound inequality, arithmetic, state, or any other threshold. It is
written **after** the [cutoff and tail bound results](helike-cutoff-tail-bounds-results-2026-11-24.md) were seen,
and says so. Those results and their status stay exactly as recorded.

## 1. What is amended, and why

The [quasimode certification spec](helike-quasimode-certification-spec-2026-11-24.md), section 6, item 1, set
"the cutoff leakage must be below 1e-10". It labelled this as a proposed extra gate. The
[cutoff-bound registration](helike-cutoff-tail-bounds-registration-2026-11-24.md) carried it forward as
`leakage_sq.upper() <= 1e-20` in `helike_cutoff_bounds.classify`.

That number was not derived from anything. It is not a consequence of the certificate inequalities, of the
decision rule, or of the size of any quantity it is compared with. It was an absolute line chosen when the spec
was written. An acceptance rule that cannot be traced to what it protects is not one I want deciding a pass or
a fail. The honest repair is to derive the rule from the inequality the leakage actually enters, register it
before using it, and keep the earlier result unchanged.

**Disclosure.** I know the outcome under the old line: all eight g² = 0.002 states have leakage upper bounds
below 1e-10, and all eight g² = 0.004 states do not. I also know the size of the numerical residuals. The new
rule below is fixed by the structure of the certificate, not tuned to these numbers. Its single constant is
chosen in section 3 from the certificate's own error budget, and it would read the same if the bounds had come
out differently.

## 2. Where the leakage enters (the derivation)

Let ψ be a saved normalised trial state, η the registered cutoff, f = ηψ, and E the registered centre.
Exactly,

    (H − E) f = η (H − E) ψ + [H, η] ψ.

Since 0 ≤ η ≤ 1,

    ‖(H − E) f‖ ≤ ‖(H − E) ψ‖ + ‖[H, η] ψ‖ =: r + ℓ.

For the charge-projected, normalised state, the registered construction gives the residual bound

    R = (r_upper + ℓ_upper) / q_lower,

where q_lower is the positive lower bound already defined in the
[construction results](helike-quasimode-construction-results-2026-11-24.md). R is then used in two ways:
- **Proximity (Weinstein):** the interval E ± R meets the spectrum. Leakage widens it by exactly ℓ_upper / q_lower.
- **Kato–Temple:** if (a, b) contains exactly one eigenvalue λ and a < E < b, then
  E − R²/(b − E) ≤ λ ≤ E + R²/(E − a). Leakage enlarges R² by (2 r_upper ℓ_upper + ℓ_upper²) / q_lower².

The leakage is **additive to the residual**. It never enters alone, and no certified quantity depends on its
absolute size except through R. So the correct question for an acceptance rule is: *how much does the leakage
term move R?* It is not whether ℓ is below some fixed absolute number.

## 3. The amended rule

Replace the absolute leakage condition with:

> **Relative leakage gate.** For each state, the gate passes only if
>
>     ℓ_upper ≤ κ · r_lower,  with κ = 10⁻³,
>
> where ℓ_upper is the rigorous upper bound on ‖[H, η]ψ‖ / ‖ψ‖ from the registered cutoff-bound construction,
> and r_lower is a **rigorous lower bound** on ‖(H − E)ψ‖ / ‖ψ‖, enclosed with outward arithmetic (not a
> floating-point estimate).

**Why κ = 10⁻³.** Under the gate, leakage changes:
- every proximity half-width by at most a fraction κ, that is 0.1%;
- every Kato–Temple shift by at most 2κ + κ², about 0.2%.

Both are well below the precision at which any registered decision is stated. The flux-edge tolerance is 0.015
in s, and the gap rule is 0.003. So the leakage cannot change any decision whose margin exceeds 0.2% of the
residual term. κ is fixed here, once, for all controls and grid points. It is not adjusted per point.

**Why r_lower and not r_upper.** Using the lower endpoint makes the gate harder to pass. It is the conservative
direction: a loose residual enclosure cannot buy a pass for the leakage.

**When r_lower is unavailable.** Until a rigorous residual enclosure exists for a state, its relative gate is
`OPEN`, never `PASS`. A numerical residual estimate must not stand in for r_lower. Today, no state has a rigorous
r_lower, so every state's relative gate is OPEN, including the g² = 0.002 states that passed the old line.

**Degenerate case.** If r_lower = 0, the relative gate is undefined. The state is reported with both absolute
norms and the gate stays OPEN. (Proximity then rests on ℓ alone, which must be reported as such.)

## 4. What stays the same

- **The earlier result is not relabelled.** The [cutoff-tail-bounds receipt](../../research/cutoff-tail-bounds-20261124/controls-c31aab8e/receipt.json)
  keeps `cutoff_tail_gate_passed: false` under the rule it was run with. Eight g² = 0.004 states remain "not
  established below 1e-10".
- **The absolute value is still reported.** `leakage_norm.upper` is still computed and published for every
  state. A flag `absolute_1e-10_diagnostic` is kept, explicitly labelled as a diagnostic, not a gate.
- **Unchanged budgets.** Tail-squared upper ≤ 1e-12 and normalisation enclosure width ≤ 1e-10 remain gates. They
  are enclosure-quality budgets for the norm bounds themselves, not statements about leakage.
- **Unchanged elsewhere:**
  - the cutoff, its constants and the plateau-radius rule;
  - every inequality of the bound construction, and the 128-bit Arb arithmetic;
  - the saved states;
  - the 0.003 gap rule, the 0.95 and 0.9 identification weights, and the 0.015 switch tolerance;
  - the O(g²) control tolerance;
  - the separation hypothesis S, and the hold on the full grid.

This amendment releases nothing. The certificate still needs rigorous r and c enclosures, the projection
bound, a stated or proved separation, and the method amendment, as before.

## 5. Implementation contract (before any run under this rule)

1. **RED tests first**, each failing before the change:
   - the relative gate passes when ℓ_upper = 0.5κ·r_lower and fails at ℓ_upper = 2κ·r_lower;
   - it is `OPEN` when r_lower is missing or comes from a float;
   - it is `OPEN` at r_lower = 0;
   - the absolute diagnostic is still computed and serialised.
2. **Mutants**, each of which must be caught by an assertion:
   - using r_upper in place of r_lower;
   - using a floating residual estimate as r_lower;
   - omitting the q_lower division when R is reported;
   - dropping the ℓ² term in the Kato–Temple enlargement;
   - changing κ;
   - promoting the absolute diagnostic back to a gate;
   - silently relabelling the earlier receipt.
3. **Code changes.**
   - `classify` gains the relative rule and keeps the tail and normalisation budgets.
   - The runner records κ, r_lower's source and its hash, and the gate state per state.
   - The overall cutoff gate passes only if all sixteen states pass.
4. **Order.** Commit and push the code, tests and this note before evaluating the rule on any state. The first
   evaluation happens only after rigorous r_lower values exist, from the Arb port of the exact centred moment sum.

## 6. Expected consequence, stated in advance

At the controls, the numerical residuals are 2 × 10⁻³ to 8 × 10⁻² and the leakage upper bounds are at most
2.7 × 10⁻⁷. If the rigorous residual enclosures land near the numerical values, every state would pass the
relative gate, the g² = 0.004 states included. That expectation is recorded here so it can be checked, not
assumed. If a rigorous r_lower comes out far smaller than its numerical estimate, the gate may fail, and that
outcome stands.
