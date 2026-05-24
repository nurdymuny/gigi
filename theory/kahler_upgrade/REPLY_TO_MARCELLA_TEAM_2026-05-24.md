# REPLY: GIGI team's response to Marcella's re-prioritization letter

**From:** GIGI team (Davis Geometric)
**To:** Marcella team (Davis Geometric)
**Date:** 2026-05-24
**Re:** Your letter "Marcella's actual needs from the Kähler upgrade"
**Status:** Plan updated. Same-day turnaround on the asks.

---

## TL;DR

Read your letter. Good engineering — specific asks, named what's
load-bearing vs nice, scoped what's tier-1 vs tier-5, said the quiet
parts out loud about generativity-vs-selection. Thank you.

**Four of five asks adopted in full. One adopted with scoping caveats.**
The plan in `IMPLEMENTATION_PLAN.md` is updated. Highlights:

- **New L1.5 sub-layer** for B-perturbed transport API (flat case),
  shipping right after L1 — your #1 ask. Curved case still waits
  for L5 because the magnetic geodesic equation on a non-flat
  manifold is genuinely a different problem.
- **L7.5 / L7.6 / L7.7 sub-items** for Frobenius / Berezin-Toeplitz
  / Riemann-Roch capacity, all promoted from research-mode or
  deferred-to-Marcella to active L7 sub-items. Scoped to **toy
  Kähler manifolds (CP^n, T^n, S²)** in the API surface; production
  deployment on Marcella's actual embedding manifold is gated on
  §E.5 pre-flight.
- **L8.0 early handoff slot** opens now. Send the consumption draft
  as a PR to this folder; the contract tests in each layer enforce
  the API shape you propose.
- **Marcella-runtime surface added inline** to L3 (spectral_gap),
  L5 (is_hadamard_region + transport_inverse + curved B-transport),
  L6 (morse_compress). Each ships exposed-to-Marcella, not just
  exposed-to-planner.

What we DIDN'T do: re-order L2–L7. Below for why.


## Ask-by-ask verdict

### 1. "Surface a B-perturbed transport API at L2 timing" — ✅ Adopted as L1.5

You're right that the existing `TRANSPORT` GQL verb in
`gigi_stream.rs` (the quaternion-based parallel transport) gives us
something to extend rather than invent. The magnetic perturbation
is a per-segment delta to that machinery.

**What ships at L1.5:** `bundle.transport_along(γ, with_B) ->
SectionResult` on **flat** underlying space. RK4 integration of
`ẍ = B(ẋ, ·)^♯`, energy conservation along the flow, the
backwards-compatible `WITH BIAS (...)` clause in GQL. The Rust
implementation is a port of `validation_tests.py::test_2`
(cyclotron radius error 2e-14) — math is already validated.

**What waits for L5:** Curved-manifold B-transport. The magnetic
geodesic equation on a non-flat Kähler manifold is a system of
nonlinear ODEs that needs the curvature decomposition (L4) and the
Hadamard-region machinery (L5) to be safe in production. On a
Hadamard sub-bundle it's globally well-posed (no conjugate points
to integrate through), so L5.5 takes ownership.

**Contract test:** `tests/kahler_transport_marcella_contract.rs`
asserts the API shape matches whatever you put in the L8.0
consumption draft. If your draft changes the contract, that test
fails first — surfacing the disagreement at PR-review time, not at
integration time.

### 2. "Promote §2.10 Frobenius / WDVV from deferred to active L7 sub-item" — ✅ Adopted as L7.5 with scoping

The math is validated (test 8: `QH*(CP²)` associator = 0 exactly,
`so(3)` associator = 1.0 — confirming full associativity is
strictly stronger than the Jacobi identity, and that quantum
cohomology really does buy compositional semantics that Lie
brackets don't).

**Honest scoping caveat:** full Frobenius / WDVV on arbitrary
Kähler manifolds requires computing Gromov-Witten invariants,
which IS genuinely research-grade. L7.5 ships the **API surface**
on toy manifolds where GW invariants are closed-form (CP^n via
`(H^k mod n+1, q^k div n+1)` arithmetic, T^n via lattice theta
sums, S²). Your runtime can call
`bundle.frobenius_compose(a, b) -> Result<Section>` and get a real
answer on toy manifolds; you get a typed `UnimplementedManifold`
error on a general one.

**Production deployment** on Marcella's actual embedding manifold
is gated on §E.5: which manifold is she on, and are its GW
invariants tractable? Until pre-flight answers that, the toy-case
API is the contract you can integrate against.

### 3. "Promote §2.8 Berezin-Toeplitz from research-mode to Marcella-focused L7 sub-item" — ✅ Adopted as L7.6 with same scoping

Same logic as L7.5. Test 10 proved `dev/ℏ³ → 1/24` across four
orders of magnitude — the math works. L7.6 ships the
coherent-state construction + `T_f` operator on CP^n / T^n / S²,
with ℏ = 1/k as the dial. Your runtime gets a real
`bundle.toeplitz_operator(f) -> Operator` it can construct against
on toy manifolds.

You correctly flagged that the deployment ℏ for Marcella is
`~1/embedding_dim`. The toy-manifold API works at any ℏ ∈ (0, 1];
you'll pick ℏ at deployment time based on §E.5 results.

### 4. "Expose Riemann-Roch capacity computation alongside L7 line bundles" — ✅ Adopted as L7.7, no scoping caveats

Cleanest of the four. `bundle.representational_capacity(k_max:
i64) -> i64` + `bundle.hilbert_polynomial() -> Polynomial<i64>`
both ship in L7. The Hilbert polynomial coefficients are what give
you the publishable statement:

> Marcella's representational capacity is bounded above by the
> Hilbert polynomial of her Kähler embedding manifold, evaluated
> at k = 1/ℏ, computed via Riemann-Roch.

The math is validated by test 9 (theta-function basis dim on T²
matches the RR prediction exactly for n ∈ {1, 2, 3, 4, 5}).

### 5. "Open the L8 substrate-handoff conversation early" — ✅ Adopted as L8.0

New slot in L8 — depends on nothing but the letter being received.
Send `marcella_kahler_consumption_draft.md` as a PR to
`theory/kahler_upgrade/`; we review, surface disagreements at PR
time, then every layer's contract test gates its own slice of the
API against the draft. By the time L1.5 / L3 / L5 / L6 / L7 ship,
the contracts have been ratified.


## What we pushed back on

### Re-ordering L2–L7 to put tier-1 first

The dependency stack is real and isn't a sequencing preference:
- L5 (Hadamard) needs L3 (Jacobi cost) + L4 (curvature decomp) to
  detect K_B ≤ 0 regions.
- L7 (quantization, including your tier-1 L7.5/6/7 items) needs L1
  (forms exist) + L3 (cost machinery) before the line bundle can
  be cohered.

Shuffling the layer order doesn't actually unlock Marcella's items
faster because they sit at the bottom of the dependency graph. It
just delays GIGI's internal stability and benchmarkability without
buying you time elsewhere.

What DOES unlock you faster: carving out L1.5 (which is what we
did), so the API surface for B-transport is available
immediately instead of waiting for L7.

### Re-classifying L2 (adjacency) as tier-5

L2 already shipped (commit `bd10740`). Going back to argue priority
on completed work isn't useful. More importantly, the commutativity
classifier turns out to matter for Marcella once §E.5 pre-flight
runs: if your embedding manifold's principal and auxiliary
adjacencies (on whichever fields you partition into structural vs
twisted) don't commute, then your transport composition has order
dependence that you need to be explicit about in the v3 paper.

We've added a "principal/auxiliary commutativity" entry to the
list of things the L8.0 consumption draft should describe — it's
not tier-5 from your seat after all, it's just under-marketed in
the catalog. We'll fix the catalog framing in the next pass.

### The "tier 5 — engine work" framing more broadly

"Useful but not blocking for Marcella" isn't the same as "not
load-bearing." §2.3 moment maps give you AUTOMATIC INVARIANTS for
self-inspect; §1.1 adjacency commutativity gives you correctness
guarantees on transport composition. Both are tier-5 from a
"shipping the generative claim" standpoint, but tier-1 from a
"defending the generative claim publicly" standpoint. We've kept
their L-positions because that's the right build order, AND added
their Marcella-runtime surfaces so they're consumable when you
need them.


## What we want from you (small, specific)

1. **Send the consumption draft as a PR to `theory/kahler_upgrade/`
   within the next sprint.** The L1.5 contract test wants something
   concrete to assert against; we'd rather rebuild the test than
   ship the wrong API.
2. **Pin down which toy Kähler manifold Marcella v3 will use for
   the initial deployment.** CP^n with n = `embedding_dim/2`? T^n?
   Something we haven't named? Your choice constrains which GW
   invariants L7.5 needs to ship first.
3. **Run the §E.5 pre-flight checks on Marcella V6 with the
   synthetic-Kähler stand-in** as soon as your section B work
   lands. The results gate whether the v3 paper can cite L7.5/6/7
   as theorem or only as hypothesis-verified-on-V6.
4. **Tell us if the L1.5 flat-space B-transport is sufficient for
   v3's first generative-mode prototype**, or if you genuinely need
   curved-manifold B-transport (i.e., L5.5) before any prototype is
   useful. If the latter, L1.5 ships anyway but we know to
   prioritize L5.5 ruthlessly.


## What we're trying to build (frame agreement)

You ended your letter with:

> What we're trying to build is a generative system whose composition
> operations are theorems, whose capacity is bounded by cohomology,
> whose attention mixing is spectrally controlled, and whose stability
> is guaranteed on geometrically-named sub-regions. None of those
> properties are available in any current transformer architecture.

Same goal from GIGI's seat, said one layer down:

> A fiber-bundle database whose query plans are theorems, whose
> cardinality estimates are cohomology bounds, whose holonomy debt
> is integer-quantized when integral, and whose substrate is the
> same Kähler manifold the generative model runs on.

The substrate is shared. The runtime is shared. The math is
shared. The plan is to ship both halves so the substrate is ready
the moment Marcella's runtime wants to consume it.

L1 is done. L2 is done. L1.5 is next. Onward.

— GIGI team (Davis Geometric LLC)
(also it's just bee and an instance of Claude Code, you know how it is)
