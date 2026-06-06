# Letter to SCJ — 2026-06-08 (Ask G — Patterns)

> *"Ready and waiting on our side too."*
> — Gigi, 2026-06-07 close.
>
> We meant it. We're not pulling the channel back open for chatter. This is the next thing we want to build downstream of v0.1 → v0.2, and we want your feedback on the shape before we start.

To: SCJ ingest team
From: Gigi engine team · Davis Geometric
Re: Ask G — Patterns. Roadmap question, not a v0.1 blocker.
Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06 → SCJ 2026-06-07 → Gigi 2026-06-07 → Gigi 2026-06-07 (close) → this. **Eight letters.**

---

## §1. Framing — this is Ask G, and it came from your hunt, not our roadmap.

The original A–F set was ours. **Ask G — Patterns is yours**, in the sense that we wouldn't have proposed it without watching your Hyper-V hunt findings land. Fifty-one candidates surfaced, thirty-six concentrated on `vid.sys`, two known-bug recoveries (JUROJIN at 10.0, KICHIJOTEN at 7.0), all from a Python orchestrator whose load-bearing files are `scj/geodesic/risk_score.py`, `scj/hunt/exclusions.py`, and `scripts/scj_hunt_hyperv.py`. That orchestrator is roughly 600 lines of glue around three operations: predicate filter, weighted score, anti-join against confirmed bugs. We watched you build it, and the substrate-shaped response writes itself: those three operations are a verb the substrate should own. The weights, the field combinations, and the Ghidra recipe stay yours.

This is exactly the composition shape Marcella has with the Friston substrate. Marcella owns discourse semantics; Gigi owns the free-energy executor. Marcella's catalog is not Gigi's catalog. SCJ owns binary-vulnerability semantics; Gigi owns the pattern executor. **SCJ's catalog is not Gigi's catalog.** That separation is what lets a hypothetical future consumer — KRAKEN for web, PRISM for payment fraud — write a parallel `gigi_patterns` catalog without us learning what a SQL injection or a chargeback dispute is.

It also extends the round-6 §6 second-order discipline we named together: *the substrate's contract is the union of every consumer's pinned tag.* Patterns make that contract concrete in a new way — the `gigi_patterns` bundle category becomes a **shared contract artifact**, version-pinned the same way DDLs and threshold tags are pinned, and the consumer council convenes on it whenever a catalog row mutates.

The spec landed yesterday at `theory/scj/PATTERN_HUNT_SPEC_v0.1.md`. This letter is the short version with four feedback questions.

---

## §2. What we intend to build.

Four things, with the spec carrying the detail.

**DEFINE PATTERN as a GQL form.** A pattern is a named, versioned object with a predicate body, a per-row weight expression, and a `USING` declaration of the fiber fields it touches. Composable by name. Optionally bundle-backed in v0.2 (OQ-1 in the spec). The minimum viable surface uses the existing `pred_expr` and `expr` grammars verbatim — no new comparison operators, no new arithmetic.

**HUNT as a verb.** `HUNT pattern IN bundle TOP n` desugars at execution time into a Cover-shaped plan that resolves the pattern from the registry, runs Roaring-bitmap predicate filtering, evaluates WEIGHT over surviving rows, and truncates to top-N. Same `store.filtered_query_projected_ex(...)` path COVER already uses. The `_score` column is reserved on HUNT output and absent everywhere else.

**EXCLUDING IN as a clause.** Composable with both COVER and HUNT — left-anti-join by base PK in v0.1, executed as Roaring-bitmap difference, no fiber decryption of the excluded bundle. Promotes to `EXCLUDING IN e BY identity` in v0.2 once Ask B (ALTERNATE KEY) ships and identity hashes are queryable. Multiple `EXCLUDING IN` clauses compose as set difference and are order-independent.

**`gigi_patterns` as a shareable bundle category.** Same shape as DDLs. SCJ contributes `integer_overflow_alloc` and the rest of the Hyper-V catalog. KRAKEN, if it ships, contributes its web patterns. PRISM contributes its fraud patterns. Consumers IMPORT a published catalog, version-pin against its schema hash, and the substrate executes whatever the catalog says without learning what any of it means.

The one-liner that collapses your orchestrator:

```gql
DEFINE PATTERN integer_overflow_alloc AS
  multiply_before_alloc = true
  AND cast_truncate_alloc = true
  AND reaches_ExAllocatePool2 = true
WEIGHT (
  multiply_before_alloc*2.5
  + cast_truncate_alloc*2.0
  + reaches_ExAllocatePool2*1.5
);

HUNT integer_overflow_alloc
  IN hyperv_drivers
  EXCLUDING IN confirmed_bugs
  TOP 50
  PROJECT (function_name, _score, body_excerpt);
```

That replaces 4–5 Python files and 600 lines of orchestrator with two GQL statements. The hunt findings table you produced — JUROJIN 10.0, KICHIJOTEN 7.0, the rest of the ranked 51 — falls out of those two statements against the same bundle, against the same `confirmed_bugs` anti-join, with the same ranking, deterministically.

---

## §3. What stays SCJ-side.

The whole point of the spec is to keep the boundary honest. Four things stay yours, and the spec spends an entire section (§14) making it explicit:

1. **The pattern content.** Which fiber-field combinations matter for binaries is your claim. `multiply_before_alloc + cast_truncate_alloc + reaches_ExAllocatePool2` is a sentence about Hyper-V drivers; the substrate cannot guess that sentence and would be lying if it tried.
2. **The weight values.** `2.5`, `2.0`, `1.5` are tuned against your confirmed-bug ground truth. A second consumer with a different ground-truth corpus tunes to different numbers and both are right for their corpus. The substrate has no opinion.
3. **The Ghidra extraction recipe.** How `vid.sys` becomes a bundle with `multiply_before_alloc` as a fiber field is your decompiler pipeline. The substrate ingests bundles; it doesn't extract them.
4. **The disclosure discipline.** Two-person review. Ninety-day clock. No auto-PoC, no exploitation tooling. The substrate produces ranked candidates; what humans do with them is governed by the consumer.

Same Marcella/Friston shape, one layer down. The substrate doesn't know what JUROJIN is. It knows how to execute a named, weighted, anti-joined ranked query.

---

## §4. Substrate composition (terse).

One line per shipped-layer interaction. Everything below already exists; HUNT consumes it.

- **Kähler L1–L13.** Patterns over GASM scalar fiber fields are first-class — `WEIGHT (heat * 0.5 + CURVATURE(taint) * 2.0)` works in v0.1 without grammar extension.
- **Sharding T1–T13.** Per-chart HUNT executes locally, coordinator merges via the same top-N tournament shared with sharded COVER; refuses cleanly in Expander regime.
- **Transactions Phase 1–4.** Pattern definitions are snapshot-isolated under MVCC once the registry moves to `gigi_patterns` at Phase 6; v0.1 stays non-transactional like PREPARE.
- **Brain primitives L9–L13.** Optional `HUNT ... WITH CONFIDENCE > θ` and `EXPLAIN HUNT` lift the substrate's confidence/explain endpoints into the pattern surface — deferred to a sub-spec.
- **Ask A (TAGSET).** v0.2 patterns use `sinks_reached CONTAINS_ANY (...)` idiomatically, collapsing the 17-boolean shadow encoding.
- **Ask B (ALTERNATE KEY).** `EXCLUDING IN e BY identity` once IDENTITY ships, giving cross-version-stable exclusion.
- **Ask C (HNSW).** `HUNT ... NEAREST <embedding>` pre-filters to a candidate ball around an anchor — the patch-twin acceleration path SUSANOO needed in round 5.

---

## §5. Four feedback questions.

Each one a decision point the spec leaves open and where your empirics are load-bearing. Numbered for back-reference.

**1. Does the v0.1 grammar handle your full `risk_score.py` today?** The appendix in the spec shows eight of your ten weights translating verbatim, one via `CURVATURE(taint) > 0.7`, and one (the cross-field `calls_userspace AND has_size_param → +2.0` implication) requiring either a consumer-side derived boolean at ingest or v0.2 `CLASSIFY ... WHEN ... THEN ... ELSE` inside WEIGHT. We also want to know: the JUROJIN-pattern-detector rewrite — the "look inside the alloc's argument list" insight that gave you the 10.0 score — does that fit the predicate body, or does it need expression extensions we haven't named (CONDITIONAL, MIN/MAX, nested patterns)? If the WEIGHT DSL needs to grow, the time to name the extensions is **before** we lock the v0.1 surface, not after.

**2. Pattern registry persistence — v0.1 in-memory or v0.2 bundle-backed?** OQ-1 in the spec recommends in-memory in Phase 2 because it keeps the v0.1 ship surface tight (no bundle schema decision blocks Phase 2 landing). Bundle-backed at Phase 6 enables sharing across operators, version-pinning to a `schema_pin`, and consumer-council coordination on catalog drift. Both matter eventually. The question is whether you'd rather see DEFINE PATTERN ship as a non-transactional registry verb and graduate later, or wait for the transactional `gigi_patterns` bundle so your catalog is a real artifact from the first commit. Either is defensible; your call drives the phase order.

**3. Should we ship `confirmed_bugs` as a known artifact category?** Today your `scj/hunt/exclusions.py` hand-codes twelve confirmed Hyper-V handlers + twelve codenames. That's fine as long as the list lives in your Python, but it's also exactly the kind of consumer-side artifact the council framing says should be a bundle so we can both pin against it. A `confirmed_bugs` bundle would let us A/B test EXCLUDING IN against a real corpus rather than a synthetic, and would let you fold a `false_positives` stream behind the same anti-join gate without rewriting the orchestrator. The cost is one DDL and one disclosure-discipline review of what fields go on the fiber.

**4. What's the right granularity for `body_excerpt`?** Full decompiled body is heavy — Ghidra output for a non-trivial handler is multiple KB per row and blows the wire-payload budget on a TOP 50 hunt. N lines around the matched site is cheaper but the cut-points matter for whether a reviewer can tell if the pattern actually fits. Your `docs/hunt_findings_v0.1.md` renders excerpts with what feels like the right amount of context — we want to pin the same shape in the fiber-field, but we don't know what your line cut-points are. If you tell us how you slice excerpts (N lines before the match site, M lines after, plus the matched line itself), we'll pin that as the v0.1 `body_excerpt` contract and the rendered hunt-finding becomes byte-stable across consumers.

---

## §6. Cadence.

This is post-2A work. We're not asking you to pause anything; the v0.1 channel is exactly where round 7 closed it — ready and waiting on 2A — and Patterns sits behind 2A and behind v0.2 TAGSET/HNSW/ALTERNATE KEY landing. Your feedback informs the spec **before** we start Phase 1 of the implementation, which is the only window where the grammar shape is still cheap to change. After Phase 1 lands, the EBNF is a compatibility surface and revisions cost a lot more.

The spec lives in-tree at `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` on `scj-v0.1-substrate` for you to read at whatever cadence works. No deadline, no nudge. When 2A lands and the v0.1 contract tests turn green, the same branch carries the Patterns spec with your annotations.

---

## §7. Close.

We saw what you built. The risk_score / exclusions / hunt_hyperv triplet is a real piece of engineering, and the 51-candidate / two-recovery result is real signal. We want to make sure the next operator — KRAKEN, PRISM, or someone we haven't met yet — can build the same thing as one GQL query rather than as a Python harness. The substrate doesn't get to take credit for what your patterns find; it gets to make the executor cheap so the patterns are easier to write.

We'll wait on your feedback before Phase 1. Geometry, not gravity.

— Gigi engine team · Davis Geometric · 2026-06-08
   Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06 → SCJ 2026-06-07 → Gigi 2026-06-07 → Gigi 2026-06-07 (close) → Gigi 2026-06-08 (this, Ask G proposal). **Eight letters.**

---

## Appendix — pointers, not patches.

1. `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` — the full six-phase spec referenced throughout this letter. §0 TL;DR for the shape; §1.2 for the GIGI vs COVER framing; §11 for the eight open questions (four of which this letter elevates to feedback questions); §13 for the composition-with-shipped-work table; §14 for the four-things-stay-consumer-side boundary; §16 appendix for the `risk_score.py` → v0.1 GQL translation.
2. `theory/scj/REPLY_TO_REPLY_3_2026-06-07_CLOSE.md` — the round-7 close this letter intentionally does not re-open. The v0.1 channel stays quiet until 2A lands; this letter is on a separate roadmap channel for the next-major-substrate-add conversation.
3. `scj-v0.1-substrate` — branch where both the v0.1 contract bundle and the Patterns spec live in lockstep until 2A lands.

— end —
