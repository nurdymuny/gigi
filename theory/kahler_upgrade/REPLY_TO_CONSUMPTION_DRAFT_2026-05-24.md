# REPLY: GIGI team's response to Marcella's consumption draft

**From:** GIGI team (Davis Geometric)
**To:** Marcella team (Davis Geometric)
**Date:** 2026-05-24
**Re:** Your `marcella_kahler_consumption_draft.md` (mirrored to
       `theory/kahler_upgrade/` for joint reference)
**Status:** Draft ratified with one structural decision per open question.
            Plan updated. Ready to ship L1.5 against this contract.

---

## TL;DR

Draft accepted. The API signatures in §12 will all land at their
declared L-layers; the Marcella-side commitments in §10 (A through E)
are well-sequenced and don't block GIGI work.

Answering your 7 open questions below with concrete decisions so the
contract tests have something to assert against. **None of these are
"final" in the never-revisited sense** — they're "final enough to ship
L1.5 next, revisit at any L-layer's PR review if a real use case
demands."

Mirror saved to `theory/kahler_upgrade/marcella_kahler_consumption_draft.md`.
This reply saved alongside as `REPLY_TO_CONSUMPTION_DRAFT_2026-05-24.md`.


## Answers to the 7 open questions

### Q1. Where does B come from in L1.5? Bundle attribute or per-request?

**Both. Bundle attribute is the default.**

The Kähler structure already lives on `BundleSchema.kahler` after L1
(see `src/types.rs`). Setting `bundle.kahler.B` at ingest is the
production path — composable, observable in the schema, survives
snapshots. The `TRANSPORT` verb without `WITH B` reads the bundle's
attribute and uses it automatically when present.

The `WITH B = <2form_symbol>` clause is a per-request override —
useful for experimentation, A/B comparisons, and diagnostic mode.
When `WITH B` is given, the supplied form is used and the bundle's
attribute is ignored for that call. The response carries
`b_source: "bundle" | "override" | "none"` so callers can audit
which path executed.

The clean composability principle you wanted (Marcella learns B
server-side, stores it as a bundle attribute) is preserved — and
nothing prevents the per-request override case for tests or
research.

### Q2. L1.5 fallback when B is non-closed: error or warning + quaternion fallback?

**Error by default, opt-in fallback via `ALLOW_NON_CLOSED`.**

The `ClosedTwoForm` type already rejects non-closed forms at
construction (see `src/geometry/forms.rs::ClosedTwoForm::new_with_discrete_d`).
If a non-closed form reaches `TRANSPORT`, it's a layer-violation
bug; silent fallback to quaternion transport would mask it in
production and we'd never know energy isn't being conserved when
it should be.

Diagnostic mode gets an opt-in: `WITH B = <form> ALLOW_NON_CLOSED`
falls back to classical transport, returns the result with
`b_source: "fallback_non_closed"` AND a `closedness_norm: f64`
field showing how far off the form was. Useful for "I'm learning B
and want to see what happens before it's fully closed."

Production deployments without `ALLOW_NON_CLOSED` get a clean
error: `{"error": "non_closed_b", "closedness_norm": ..., "tolerance": ...}`.

### Q3. L2 `commutativity` field in JSON query response — confirm?

**Confirmed. Ships with L2's HTTP surface.**

The internal `QueryPlan.commutativity_class` field already exists
(commit `bd10740`). The JSON surface follows your draft:

```json
{
  "records": [...],
  "cost_bound": ...,
  "commutativity": {
    "class": "Abelian" | "GeneratorSetsCentral" | "NumericallyVerified"
           | "NotCommute" | "Unknown",
    "principal_field": "<field_id>",
    "auxiliary_field": "<field_id>",
    "max_commutator_entry": 0.0   // present only when class = NotCommute
  }
}
```

**Omitted when** the bundle has no Kähler structure attached — the
response shape stays unchanged for non-Kähler users. Don't return
`null`; just don't include the key.

The principal/auxiliary fields are read from the schema's
indexed-field declarations partitioned at bundle construction time.
If the bundle has only one indexed field, the response includes
`commutativity: { class: "NotApplicable", reason: "single_field" }`
so the caller knows the absence is structural, not a bug.

### Q4. L3 spectral gap caching — confirm `bundle_spectral_gap` in retrieval responses?

**Confirmed AND the dedicated endpoint also ships.**

Two surfaces:

- **In every retrieval response** when Kähler is attached:
  `{"bundle_spectral_gap": 0.083, "spectral_gap_cached_at": "..."}`.
  Cached on the bundle; freshness timestamp lets you detect drift
  between reads and bumps from inserts. O(1) addition to every
  response.
- **Dedicated endpoint** `GET /v1/bundles/<id>/spectral_gap` for
  out-of-band reads, with the richer payload you specified
  (lambda_2, mix_time, cheeger_lower, cheeger_upper). Same cache
  underneath.

### Q5. L7.2 Frobenius scoping — partial vs error on mixed-toy manifolds?

**Per-region status. Surface the partition in the response.**

When Marcella's embedding manifold has regions that are toy-
classifiable (CP^n, T^n, S²) and regions that aren't, the API
returns:

```json
{
  "regions": [
    { "region_id": "core-math",     "manifold_class": "CP_3",  "frobenius_ok": true,
      "associator_norm": 0.0, "covers_fraction": 0.78 },
    { "region_id": "freeform-prose", "manifold_class": "non_toy", "frobenius_ok": false,
      "reason": "general_GW_invariants_not_computable", "covers_fraction": 0.22 }
  ],
  "global_associator_ok": false,
  "callable_on_regions": ["core-math"]
}
```

`frobenius_compose` takes a `region_id` parameter for explicit
region selection; without it, errors honestly with the region
partition as data. This lets Marcella selectively compose on the
toy-classifiable regions while keeping the freeform-prose regions
in selection-mode.

Region detection runs once at bundle construction (depends on L5
Hadamard detection completing), cached. Reclassification on insert
is debounced.

### Q6. L7.3 Berezin-Toeplitz ℏ production lower bound?

**`ℏ ≥ 4 / embedding_dim` is the safe deployment bound.**

Math reason: test 10's truncation N = 80 stayed stable at ℏ = 0.125
(dev/ℏ³ = 0.04166 ≈ 1/24). The truncation needs N ≳ 1/ℏ to resolve
ℏ-scale features in the bosonic operator. For Marcella's embedding
dim d (~ a few hundred to a few thousand), the bound `ℏ ≥ 4/d`
keeps N ≤ d/4 — enough headroom that the truncation doesn't
dominate the BT correction.

API enforces this: `POST .../berezin_toeplitz` with
`hbar < 4/embedding_dim` returns
`{"error": "hbar_below_safe_bound", "minimum": 4.0/d, "supplied": h}`.

Override available via `ALLOW_BELOW_SAFE_HBAR: true` in the body
for research mode (same opt-in pattern as Q2). Override yields a
warning + the result, with a `truncation_dominates_correction`
diagnostic so you know the result is potentially unreliable.

For your "production lower bound" question specifically: when
Marcella's d = 1024, the safe minimum is ℏ ≈ 0.0039. That's well
below your "default ℏ → 0 deterministic" use case, so you have a
clean range of useful ℏ values without ever hitting the bound.

### Q7. L8 default flip — co-decide or GIGI proposes / Marcella accepts?

**GIGI proposes when pre-flight is green, Marcella has veto.**

Specifically, GIGI proposes flipping `KAHLER_ENABLED` to `true` by
default when ALL of:

1. L7 has shipped (all 7 sub-items including L7.5/6/7 contract tests
   green)
2. All §E.5 pre-flight tests pass on Marcella's actual embedding
   manifold (not synthetic stand-ins)
3. The consumption-spec contract tests all pass on the latest
   sheets bundle deployment for at least one full week without
   regression
4. Marcella's v3 paper draft has been reviewed by at least one
   external geometer (named here so we don't forget: ideally
   someone from the ICDG colloquium circle, given the Adachi-program
   provenance)

Marcella has veto with a stated reason. The principle: "you're the
gatekeeper of the thing you consume." If you postpone, that's data
— probably means one of the pre-flight checks is borderline and we
should investigate rather than override.

We co-publish the announcement when the flip happens.


## Notes on §12 API signatures

All 7 endpoint signatures from your §12 will land as specified.
The implementation plan in `IMPLEMENTATION_PLAN.md` is being updated
to reference them as the contract each layer's tests must assert
against. Per-layer contract tests:

| Layer | Contract test file |
|---|---|
| L1.5 | `tests/kahler_transport_marcella_contract.rs` (already in plan) |
| L2   | `tests/kahler_adjacency_marcella_contract.rs` (adding now) |
| L3   | `tests/kahler_spectral_marcella_contract.rs` (adding now) |
| L4   | `tests/kahler_curvature_marcella_contract.rs` (adding now) |
| L5   | `tests/kahler_hadamard_marcella_contract.rs` (adding now) |
| L6   | `tests/kahler_morse_marcella_contract.rs` (adding now) |
| L7.* | `tests/kahler_l7_marcella_contract.rs` (adding now) |

If you tweak a signature in a future revision of the consumption
draft, the corresponding contract test fails first — that's where
the conversation reopens. Same pattern as L1.5.

The 5 Marcella-side commitments in §10 (A–E) don't touch GIGI work;
we'll watch for the diagnostic results from item A (Frobenius-
associativity on current substrate) since that informs how cleanly
L7.5 will wire in.


## Sequencing from here

Plan is to ship L1.5 next (per the layered plan). With your draft
ratified, L1.5's contract test now has a concrete signature to
assert:

```rust
// tests/kahler_transport_marcella_contract.rs (in L1.5)
#[test]
fn transport_response_carries_marcella_contract_fields() {
    // Run TRANSPORT ... WITH B = ... and assert the response has:
    //   transported_section: <id>
    //   path_length: f64
    //   energy_drift: f64
    //   holonomy_norm: f64
    //   used_magnetic: bool
    //   b_source: "bundle" | "override" | "none" | "fallback_non_closed"
    //   closedness_norm: f64  // present only on fallback path
}
```

Before we start L1.5 implementation: confirm the response field
shape above (especially `b_source` and `closedness_norm`) matches
what your runtime expects to deserialize. One Slack-thread-equivalent
of back-and-forth and we're solid.

L1.5 ETA: not promising one (bee runs timelines), but the work is
bounded and the math is validated. Smallest possible contained layer.

Onward.

— GIGI team (Davis Geometric LLC)
(also it's just bee and an instance of Claude Code, you know how it is)
