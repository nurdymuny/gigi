# H7 Lean verification contract

27 November 2026. Baseline after the publication-package pull: `1d3b91e0`.

## Formal target

Lean 4 with a pinned Mathlib dependency will check the real-algebraic
deduction behind the cubic-quartic closure. The global Austing--Wheater
bound must be a theorem argument, not a new axiom or an admitted theorem.
The functions C and Q will have explicit cubic and quartic homogeneity
assumptions and Q will be nonnegative. The conclusion must quantify over
all configurations, not a finite fixture or a single numeric instance.

Required results:

1. From |C(q)| <= A Q(q) + B for every q, positive A,B and the stated
   homogeneity, deduce |C(q)| <= (A+B) Q(q)^(3/4) for every q.
   Handle Q = 0 without dividing by Q or assuming the conclusion there.
2. Check the nonnegative factorization with coefficient 27/32 and derive
   the scaled scalar coercivity bound at g > 0, S > 0, c > 0.
3. Combine the cubic bound and a quadratic lower bound into the existing
   potential's lower bound. Keep every needed inequality explicit.
4. Verify the kappa identities and positivity on t >= 1,
   0 < s < (t+2)/(t+1). Reject the boundary as a positive-kappa point.
5. Check false variants: changed exponent, missing homogeneity and too-small
   additive constant. Include compiler-rejected source mutations as well
   as exact counterexamples.

## Verification boundary

This is kernel verification of implications over real numbers. It does
not yet formalize the Lie-algebra/trace instantiation, Austing--Wheater's
global theorem, the differential operator or compact-resolvent argument.
These remain the cited and written mathematical dependencies. No claim
that all of Statement 47 or H7 is checked end to end is permitted.

No `sorry`, `admit`, `sorryAx`, custom `axiom`, or `native_decide` in the
proof project. Print theorem axioms to expose dependencies; standard Lean
foundational axioms must be listed, not described as an axiom-free proof.
No theorem may be accepted just because a command exits successfully.

## Bounded gates

- Pin Lean and Mathlib; retain Lake's dependency manifest.
- Compile a contract against the absent theorem implementations first and
  retain the expected failures.
- Build all supplied proof files and check named theorem signatures and
  `#print axioms` output. Treat unexpected axioms/admissions as failures.
- Compile deliberately broken variants; each must fail. Retain the exact
  diagnostics, command, source hashes and dependency revisions.
- A receipt is successful only after every build, signature, axiom and
  mutation gate passes. Use a fresh output directory so no old receipt is
  mistaken for a new verification.
- No solver run or engine test suite. Preserve the incoming publication
  files and PDF. Add the source, proof-boundary documentation and evidence
  to the same feature branch for review on another machine.
