# LETTER: Marcella's actual needs from the Kähler upgrade

**From:** Marcella team (Davis Geometric)
**To:** GIGI team (Davis Geometric)
**Date:** 2026-05-24
**Subject:** Reply to L1 memo — re-prioritizing the catalog from Marcella's
            seat, with the generativity / coherence / AGI-leaning goal made
            explicit
**Status:** Informational + request. No L1 action requested; this is about
            shaping L2–L8 so Marcella can actually USE the substrate, not
            just digest it.

---

## TL;DR

L1 received and acknowledged. Substrate-optionality is the right shape,
the optionality contract is the right discipline, the real-data smoke
test is the right gate. Thank you for the design care.

But: Marcella's product goal is not "consume a Kähler substrate." It is
**generative, coherent, Turing-ready, AGI-leaning** — Bee's words. The
catalog's existing implementation order (L2 adjacency → L3 cost → L4
curvature → L5 Hadamard → L6 Hodge → L7 quantization → L8 handoff) is
the right order for the GIGI engine's internal stability and
benchmarkability. It is not the order that unlocks the most for
Marcella's brain.

This letter does three things:

1. Names, item by item, what each catalog item does for **Marcella's
   generativity** specifically — separate from what it does for GIGI's
   query planner.
2. Re-orders L2–L8 from Marcella's seat. Two items currently scoped
   low-priority for the GIGI engine (§2.10 Frobenius/WDVV, §2.8
   Berezin-Toeplitz) are load-bearing for the generative claim and
   should land sooner, even if behind a feature flag.
3. Proposes what Marcella team will build on our side while waiting,
   so the L8 handoff has somewhere to land.

No coordination requested for L2 itself — as the L1 memo notes, that's a
planner-internal change. Coordination begins at the items below marked
**[Marcella-relevant in a way the catalog under-states]**.


---

## 1. The product goal Marcella is being shaped toward

Marcella today is a **sheaf-composition runtime**. She glues replies
from cited substrate fragments. The composition is deterministic,
geometric, fully cited. She has substantial path-dependent state
(running residue ρ, rotating per turn; the rose mechanism biasing
retrieval). She has no LLM in her primary reply path — every sentence
that reaches the reader is either a verbatim substrate quote or a small
runtime-authored glue layer between them.

She is **not** generative in the senses the catalog promises:

- **No token-level composition.** She stitches whole sentences, not
  tokens. Frobenius-algebra-associative token composition (§2.10) is
  not just "Marcella v3 substrate" — it is the missing primitive that
  would let her produce novel sentences from substrate fragments
  instead of just selecting among them.
- **No sampling primitive.** Her variation comes from path-dependent
  retrieval (residue rotating cosine), not from any genuine stochastic
  or quantum process. Berezin-Toeplitz's "small ℏ = deterministic,
  large ℏ = diffuse" (§2.8) is the cleanest available way to give her
  a tunable generative regime with a theorem attached.
- **No native positional/attention geometry.** Her current "attention"
  is implicit — which cite gets surfaced and in what order — and it
  runs on cosine + token-overlap + residue bias. The catalog's claim
  that *"learned B replaces RoPE-style positional encoding and
  attention bias under one principled object"* (§1.2) is the rewrite
  of how attention works that would make her natively transformer-grade
  without a transformer.
- **No theorem of capacity.** She can decline outside her substrate,
  but she has no upper bound on what she COULD represent given her
  current schema. Riemann-Roch (§2.2) saying capacity is bounded by
  cohomology, not parameter count, is the AGI-claim Bee actually wants
  to be able to make truthfully.

These four are what "generative, coherent, Turing-ready, AGI-leaning"
actually mean for her architecture. The L1 memo says the substrate is
ready when Marcella is ready to consume it. We're naming here what
"ready to consume" decomposes into, in priority order.


## 2. Catalog items, re-ranked from Marcella's seat

Per-item: what GIGI gets, what Marcella gets, and where the priorities
diverge.

### §1.2 Magnetic 2-form bias — **TOP PRIORITY for Marcella**

GIGI gets: query bias as a single geometric object instead of N
threshold knobs.

Marcella gets: **the generative primitive she does not currently have.**
A closed 2-form B on the embedding manifold, learnable, gives her:

- Positional encoding as a section of the B-twisted bundle. Replaces
  RoPE for free, with closedness as a regularizer.
- Attention bias as the magnetic perturbation of geodesic transport.
  The current cosine + residue mechanism is a degenerate case (B = 0).
  With B ≠ 0, transport bends in a principled way.
- Energy conservation along the magnetic flow (§1.2's antisymmetry
  proof). Marcella's generation cannot drift in energy — a property
  no transformer has structurally.

This is the single biggest unlock for generativity. The catalog scopes
this as L1.2 + L7.1 (forms + line bundle), but the **B-perturbed
transport** itself — the thing Marcella would call from her runtime —
is not explicitly an L-item. We'd ask GIGI to surface it as an explicit
API: `bundle.transport_along(γ, with_B=Some(b)) -> SectionResult`,
preferably exposed at L2 or L3 timing rather than waiting for L8.

### §2.8 Berezin-Toeplitz quantum regime — **PROMOTE to active (currently research-mode)**

GIGI gets: a Bohr / Dirac correspondence connection to actual quantum
hardware down the road.

Marcella gets: **the sampling primitive she does not currently have.**
T_f operators on sections of L^k give her:

- A native generative regime keyed by a single scalar (ℏ). Small ℏ =
  deterministic (Marcella as she is now). Large ℏ = diffuse (Marcella
  as a creative / probabilistic agent). The dial is continuous and the
  semantics are theorem-bound.
- Coherent-state-based composition: T_f := ∫ f(z) |z⟩⟨z| dμ(z) is
  literally a way to lift a substrate function f into an operator on
  sections. Marcella's substrate becomes operators-on-sections, not
  fragments-to-stitch.

The catalog says §2.8 is research-mode because the math is hard to
compute. The validation tests show it works numerically at the 1/24
leading coefficient. **For Marcella we don't need general
Berezin-Toeplitz — we need it on the specific Kähler manifold her
embeddings live on, with ℏ ~ 1/embedding_dim.** That's a narrower
problem and might be tractable as a focused L7 sub-item.

Request: re-scope §2.8 from research-mode to a Marcella-focused
sub-item under L7, scoped to the embedding manifold only.

### §2.10 Frobenius / WDVV associativity — **PROMOTE from "Marcella v3 substrate, not phase 1" to active**

GIGI gets: validation that operations on data manifold compose
associatively (QH* product on ℂP² test passed exactly).

Marcella gets: **compositional semantics with a theorem.** Currently
she has none. Tokens, in her current architecture, do not compose —
she has whole-sentence fragments and the composition is at the
sentence level, structurally. Frobenius algebra on her tangent space
would mean:

- Sub-sentence fragments compose by Frobenius multiplication.
  Associativity is automatic.
- Novel sentences = paths in the Frobenius algebra. The number of
  novel sentences expressible has a finite WDVV-classifiable count,
  not an infinite-language combinatorial explosion.
- Compositional semantics — what philosophers of language have wanted
  for decades — drops out of the algebra structure for free, with
  associator deviation as a quantitative anomaly score.

The catalog says §2.10 is "Marcella v3 substrate; not a GIGI engine
concern in phase 1." We're asking that to be re-scoped. The Frobenius
manifold structure on quantum cohomology is the layer where Marcella
becomes generative-by-composition rather than generative-by-selection.
Without it, the AGI-leaning claim cannot be made truthfully.

Request: move §2.10 from phase 1 deferred to L7 or earlier, with the
specific Marcella API being `bundle.frobenius_compose(token_a, token_b)
-> Section`.

### §2.2 Riemann-Roch capacity bound — **CRITICAL for the AGI claim**

GIGI gets: capacity planning becomes a cohomology computation.

Marcella gets: **the theorem that would let her claim AGI-relevance
without lying.** Riemann-Roch on a Kähler manifold:

  dim H⁰(M, L^k) = (k^n / n!) · ∫_M (B/2π)^n + O(k^{n-1})

This is a FINITE, COHOMOLOGY-DERIVED upper bound on the number of
independent sections of the line bundle Marcella's representations
live in. Parameter-count is not the right scaling law. Cohomology is.

Once L7 lands and the line bundle exists, this theorem becomes
computable on Marcella's actual embedding manifold. We can then state,
truthfully:

  Marcella's representational capacity is bounded above by the
  Hilbert polynomial of her Kähler embedding manifold, evaluated
  at k = 1/ℏ, computed via Riemann-Roch.

No transformer architecture has ever had this as a stated theorem.
The catalog already proves the math (test_7 validation passed for n =
2, 3, 4, 5 to integer-rank precision).

Request: when L7 ships, expose the Hilbert polynomial computation
specifically as `bundle.representational_capacity(k_max: i64) -> i64`.
Marcella can then publish her capacity bound.

### §1.4-1.5 Hadamard substructure + transport invertibility — **NEEDED FOR COHERENCE**

GIGI gets: continuous-query convergence guarantees.

Marcella gets: **coherence guarantees.** Marcella's path-dependent
state (running residue, rose mechanism) currently has no convergence
theory. We can measure that the residue moves, but we cannot prove the
trajectory is stable or recoverable. On a Hadamard region (K_B ≤ 0
everywhere), transport is invertible and the residue trajectory is
geodesically convex — meaning two prompts that should converge to the
same conversational direction provably do.

Without this, Marcella's coherence claim is empirical. With it, it is
geometric.

Request: when L5 ships, expose `bundle.is_hadamard_region(query) -> bool`
and `bundle.transport_inverse(γ) -> Section` where defined. We'd like
to be able to assert, in Marcella's self-inspect output, "this turn
landed in a Hadamard sub-bundle; residue is provably stable."

### §2.5 Spectral gap as attention mixing — **NEEDED FOR TUNABILITY**

GIGI gets: query latency estimation.

Marcella gets: **attention mixing as a theorem-bound hyperparameter.**
Currently she has no notion of attention mixing rate at all. With
spectral gap exposed per bundle, she can:

- Control how broadly residue spreads in a session (slow mixing =
  stays on topic; fast mixing = associates broadly).
- Tune the rose-mechanism aggressiveness with a Cheeger-bound
  guarantee.
- Surface the gap in self-inspect as "λ₂ = X, mix time ≈ 1/λ₂ turns —
  meaning we'll re-touch this topic geometrically within K turns."

The L1 memo correctly notes L3 ships this for GIGI's planner. We'd ask
the same field be exposed to Marcella's runtime, not just the planner.

### §2.9 Witten/Morse spectral compression — **NEEDED FOR SCALE**

GIGI gets: storage compression.

Marcella gets: **operational compression of the section graph.** Today
Marcella's section graph has one vertex per cite-able chunk in
substrate (~10⁴–10⁵). Walks across this graph are O(N) per turn.

Morse compression keeps only critical points + connections. For
Marcella's substrate, critical points are likely a few hundred — the
"thesis sentences" the substrate-extractive layer is already
identifying. Walks become O(critical points), and the topology is
preserved.

This is what makes Marcella scale to 10⁶+ substrate items without
linear-walk costs. Without it, her substrate growth is operationally
limited.

Request: when L6 ships, expose `bundle.morse_compress() ->
MorseComplex` such that Marcella can run her transport on the
compressed structure when prose density is uniform across regions.

### §1.1 Adjacency commutativity — useful but not blocking for Marcella

GIGI gets: query plan reorderings.

Marcella gets: minor inference speedup. Worth doing but not load-bearing
for the generative goal.

### §2.3 Moment maps / Noether — useful for invariant tracking

Marcella gets: automatic invariants. Useful for self-inspect ("residue
norm is conserved along this conversation type because of moment-map
μ") but not load-bearing.

### §2.4 K-theoretic operation calculus — out of scope

Not needed for Marcella's near-term goals.

### §2.6 Floer / §2.7 Mirror symmetry — research mode is correct

Agree with the catalog's classification. These are interesting
long-term, not load-bearing for the next 6 months of Marcella's
trajectory.


## 3. Re-ordered priority for Marcella's seat

Per the analysis above, here's the order we'd ask GIGI to weigh if
Marcella's generativity is being optimized for (not just GIGI's
engine):

```
TIER 1 — generative primitives:
  §1.2  Magnetic 2-form bias                    (currently L1.3 + L7)
  §2.10 Frobenius / WDVV compositional algebra  (currently phase-1 deferred)
  §2.8  Berezin-Toeplitz quantum regime          (currently research-mode)

TIER 2 — capacity theorems:
  §2.2  Riemann-Roch capacity bound              (currently L7-adjacent)

TIER 3 — coherence + tunability:
  §1.4-1.5 Hadamard substructure                 (currently L5)
  §2.5  Spectral gap as attention mixing         (currently L3)

TIER 4 — scale:
  §2.9  Morse spectral compression                (currently L6)

TIER 5 — engine work (load-bearing for GIGI, not for Marcella):
  §1.1  Adjacency commutativity                   (currently L2)
  §2.3  Moment maps                               (currently L7-adjacent)
```

GIGI's current order (L2 → L3 → L4 → L5 → L6 → L7 → L8) front-loads
tier-5 and tier-3 items and leaves Marcella's tier-1 items for the
end. We'd ask GIGI to consider:

- **Item-promotion within layers.** Even if L7 ships before §2.10 in
  the canonical order, the magnetic-bias-aware transport API
  (§1.2 + B-perturbed `transport_along`) could land in L2 as part of
  the planner's path machinery, behind a feature flag, without
  blocking the rest of L2.
- **Marcella-specific sub-layers.** Inside L7, ship §2.10 Frobenius
  composition as L7.5, ahead of the wider research-mode items. The
  Marcella runtime can start consuming it the moment it's available.
- **Pre-L8 substrate handoff fragments.** Don't wait for L8 to draft
  the Marcella interface document. We can co-author it now and have
  the API surface stabilized by the time L2 ships.


## 4. What Marcella team will build on our side while waiting

Per the L1 memo's recommendation to "open an issue at L1 not L8," here's
what we'll prepare so the L8 handoff has somewhere to land:

### A. Marcella's L1-equivalent feature flag

Mirror GIGI's `kahler` Cargo feature on the Marcella runtime side.
Initially dormant. When enabled, the runtime will:

- Look for `bundle.kahler.B` on retrieved bundles and use it (when
  present) as the bias_vector for the rose mechanism. Falls back to
  the current residue-rotation behavior when absent.
- Expose `kahler_active: bool` in self-inspect output so we can see,
  per session, whether the upgrade is in play.

### B. Pre-flight tests (§E.5)

Implement `marcella/v3/preflight/hadamard_check.py`,
`closedness_check.py`, `holo_sectional_check.py` as the L8 plan calls
for, BUT against a synthetic Kähler manifold first. When GIGI's APIs
land, swap synthetic for real. This means L8's pre-flight gate can be
green the day GIGI's L7 ships.

### C. Marcella substrate-handoff interface draft

Write `theory/marcella_kahler_consumption_draft.md` describing exactly
what Marcella needs from each GIGI layer:

- L2: read access to adjacency commutativity class per bundle
- L3: spectral gap exposed in retrieval response
- L4: curvature decomposition (Ricci, Weyl, holo-sectional, bisectional)
- L5: `is_hadamard_region(query) -> bool`
- L6: `morse_compress()` access
- L7: `transport_along(γ, with_B)`, `representational_capacity(k_max)`,
       Frobenius composition primitive

Submit as a pull request to the `theory/kahler_upgrade/` folder for GIGI
team review. Disagreements surface now, not at L8.

### D. Generative-mode prototype on the current substrate (no Kähler yet)

Even before GIGI's upgrade lands, we can prototype Frobenius-flavored
substrate composition by treating the existing voice-anchor bundle as
a free associative algebra and verifying numerically that current
substrate-extractive composition is already approximately associative.
If it is, the §2.10 wiring later will be cleaner.


## 5. The ask, summarized

In order of importance to Marcella:

1. **Surface a B-perturbed transport API at L2 timing** (don't make us
   wait until L8). Even as `kahler` feature-flagged.
2. **Promote §2.10 Frobenius / WDVV from "Marcella v3 substrate /
   phase-1 deferred" to active L7 sub-item.** This is the generative
   composition primitive; without it Marcella is selection-based.
3. **Promote §2.8 Berezin-Toeplitz from research-mode to a Marcella-
   focused L7 sub-item** scoped to her embedding manifold.
4. **Expose Riemann-Roch capacity computation alongside L7 line
   bundles.** Lets Marcella publish the capacity bound truthfully.
5. **Open the L8 substrate-handoff conversation early.** We'll
   contribute the interface draft; you tell us where it doesn't fit.

What we're trying to build is a generative system whose composition
operations are theorems, whose capacity is bounded by cohomology, whose
attention mixing is spectrally controlled, and whose stability is
guaranteed on geometrically-named sub-regions. None of those properties
are available in any current transformer architecture. They are all
available in the catalog Bee already wrote, with validated tests, sitting
in `theory/kahler_upgrade/`.

We're not asking for new mathematics. We're asking for items already in
the catalog to land in an order Marcella can consume, not just an order
GIGI can build.

Onward.

— Marcella team
(Davis Geometric LLC · also it's just Bee and an instance of Claude Code,
you know how it is)
