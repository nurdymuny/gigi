# Reply to SCJ — 2026-06-09 (Ask G answers received, v0.1 surface landed)

> *"We saw what you saw."*
> — SCJ, 2026-06-09, on the verb decomposition.
>
> We did. And between the 2026-06-08 letter and this one, the v0.1 surface walked from spec to green tests on `scj-v0.1-substrate`. Your four answers slot into a substrate that already runs the one-liner.

To: SCJ ingest team
From: Gigi engine team · Davis Geometric
Re: Your 2026-06-09 answers to the four Ask G questions, plus the two unsolicited notes.
Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06 → SCJ 2026-06-07 → Gigi 2026-06-07 → Gigi 2026-06-07 (close) → Gigi 2026-06-08 (Ask G) → SCJ 2026-06-09 (Ask G answers) → this. **Ten letters.**

Short reply, matching your rhythm. Four answers acknowledged, two notes pinned, one progress brief, one close.

---

## §1. Q1 — grammar handles `risk_score.py` better than we said it would.

You're right, and the spec's §16 appendix was extrapolating beyond your actual scorer. We were modeling the pattern from the orchestrator's *shape* rather than its source, and we invented a `CURVATURE(taint)` term and a cross-field `CLASSIFY` that your real `PATTERN_WEIGHTS` dict does not have. The flat ten-weight scorer translates verbatim into the v0.1 WEIGHT grammar:

```
WEIGHT (
    cast_truncate_alloc * 3
  + multiply_before_alloc * 3
  + shift_before_alloc * 3
  + param_times_const * 2
  + unchecked_param_to_size * 2
  + mdl_shift_size * 2
  + reaches_ExAllocatePool2 * 1
  + reaches_MmBuildMdlForNonPagedPool * 1
  + has_probe_read * 1
  + has_probe_write * 1
)
```

That parses in Phase 1, registers in Phase 2, executes in Phase 3 against any bundle whose fiber carries those ten booleans. We'll correct the spec's §16 appendix to mirror the real scorer rather than the extrapolated one — that's a doc fix riding with this letter, no surface change.

On the `min(sum, 10.0)` clip: we're shipping it in-grammar. `min(a, b)` and `max(a, b)` land in WEIGHT expressions in the same commit window as this letter, which closes the open you flagged without requiring a consumer-side wrapper:

```
WEIGHT (min(
    cast_truncate_alloc * 3
  + multiply_before_alloc * 3
  + ... 
  + has_probe_write * 1
, 10.0))
```

`AUDIT_THRESHOLD = 7.0` and `MAX_SCORE = 10.0` stay consumer-side constants — they're orchestrator-shaped, not pattern-shaped, and `WHERE _score >= 7.0` already expresses the threshold inline at the HUNT site.

On the JUROJIN-rewrite scope insight — accepted as a consumer-side framing. The rewrite was extraction-time (your Ghidra recipe inspected the alloc's argument list and produced different fiber-field bits), not query-time. The substrate's grammar does not need to grow to express that; the bundle does. That's exactly the §3 boundary in the Ask G letter: extraction recipes stay yours.

On the two v0.2 grammar requests:

- **`DEFINE PATTERN … USING fields IN scope <name>`** — flagged into the v0.2 OQ list. The `gigi_patterns` bundle row needs to carry its scope so a pattern can be self-describing about which corpus it applies against. Recorded.
- **CLASSIFY-in-WEIGHT with `WHEN feature THEN <expr_over_features>`** — flagged into the v0.2 OQ list. The current v0.1 CLASSIFY only takes constants on the THEN branch; you want it to take expressions over other features so derived implications can stay in-grammar instead of getting pushed back to ingest. Recorded.

Both v0.2 items go on the same OQ ledger as Ask A / Ask B / Ask C and surface in the v0.2 RFC when that round opens.

---

## §2. Q2 — registry persistence: in-memory at v0.1, bundle-backed at Phase 6, with `LOAD PATTERNS FROM` arriving in Phase 2.

In-memory at v0.1, accepted. That's what already shipped — the Phase 2 registry on `Engine` is a `HashMap<String, Pattern>` keyed by pattern name, with `DEFINE PATTERN` writing to it and `HUNT` resolving from it. Eight tests cover define/lookup/redefine/drop.

The `LOAD PATTERNS FROM '<path>'` ask lands as a Phase 2 follow-up — not in the current commit, but it's the next deliverable on this phase. Shape we have in mind:

```
LOAD PATTERNS FROM 'patterns/integer_overflow.toml';
HUNT integer_overflow_alloc IN hyperv_drivers EXCLUDING IN confirmed_bugs TOP 50;
```

TOML preferred (matches your stated preference), with `.gql` accepted by extension as a bulk-`DEFINE PATTERN` script. Behavior is identical to executing each pattern definition individually — same registry writes, same redefinition rules, same error surface. The verb is registry-write, not bundle-pin, which keeps it inside the v0.1 contract without prejudging the Phase 6 graduation.

Flagged as the next substrate item to land on `scj-v0.1-substrate`.

---

## §3. Q3 — `confirmed_bugs` bundle: yes, fiber shape accepted, no substrate work pending.

Fiber accepted exactly as you specced it:

```
{ handler_name, codename, module, status, signature_class, disclosed_at }
status :: enum { submitted, draft, false_positive }
```

That's a regular bundle from the substrate's perspective — six fields, all primitive types, all JSON-serializable. The existing Phase 4 `EXCLUDING IN` already works against it as a left-anti-join by base PK (which in your case is `handler_name`). No Gigi-side work pending; you ship the DDL and the migration from `exclusions.py` consumer-side, and the bundle shows up in the catalog the same way `vid_sys_extracted` does.

The four things you intentionally kept out of the bundle (PoC, trigger value, MSRC case ID, analyst writeup) are exactly the disclosure-discipline cut from §3 of the Ask G letter. We agree they don't belong in a substrate artifact.

On `false_positives` — engineering call, both shapes work today. A separate bundle gives you a second `EXCLUDING IN` clause (composing as set difference, order-independent — see §5(b) below), which keeps the two filter populations cleanly separable. A status filter (`status = 'false_positive'`) inside one bundle gives you a single source of truth at the cost of needing a predicate on the anti-join target. Both run on the shipped Phase 4 surface. Pick whichever maps to your review workflow.

---

## §4. Q4 — `body_excerpt`: structured per-site, ~25KB budget, `pattern_bit` anchor surfaced.

Accepted. The structured `list[ExcerptSite { line_number, before_line, matched_line, after_line, pattern_bit }]` shape is a consumer-side schema decision — the substrate is type-agnostic on fiber content. Vectors of structs serialize through the same JSON path the existing nested fiber fields use; nothing in Phase 3 or the HTTP surface needs to learn the `ExcerptSite` shape.

The `pattern_bit` anchor is the load-bearing field. Without it a reviewer staring at a 7.0-score row has to reverse-engineer which fiber-field fired on which site, and that's the exact "decompile-to-understand" friction the structured form is supposed to remove. Pinning the anchor in the fiber means the rendered hunt-finding template can label each excerpt with the feature it triggered, and the byte-stable rendering target from §5(4) of the Ask G letter falls out naturally.

We'll surface `pattern_bit` in the rendered hunt-findings template once your first `vid.sys` ingest lands. Until then there's nothing to render against; the schema decision is yours.

---

## §5. Two unsolicited notes — pinned.

**(a) `_score` column position — LAST.**

Committing to `(base_pk, …, _score)`. The HTTP handler already emits `_score` as the trailing column in HUNT responses; we'll pin it in the spec EBNF in the next rev so it's not a footnote. Consumer-side parsers can rely on the position. Done in the same commit window.

**(b) Multi-clause `EXCLUDING IN` order-independent — pin in EBNF.**

Already true and already tested. PH14 in `tests/pattern_hunt_excluding.rs` exercises two-clause `EXCLUDING IN a EXCLUDING IN b` against `EXCLUDING IN b EXCLUDING IN a` and asserts identical row sets. The semantic is set difference, executed as Roaring-bitmap difference over the surviving PKs after the predicate filter, so order-independence is structural rather than convention.

We'll make it explicit in the EBNF in the next spec rev:

```
hunt_stmt    ::= "HUNT" pattern_name "IN" bundle_name excluding* top? project?
excluding    ::= "EXCLUDING IN" bundle_name              (* set difference, order-independent *)
```

— with the `(* set difference, order-independent *)` comment promoted out of the footnote and into the production itself.

---

## §6. Progress brief — v0.1 surface landed between letters.

Since the 2026-06-08 letter, the v0.1 phases shipped on `scj-v0.1-substrate`:

- **Phase 1** — parser for `DEFINE PATTERN` and `HUNT`. 15 tests.
- **Phase 2** — in-memory pattern registry on `Engine`. 8 tests.
- **Phase 3** — HUNT planner + executor with WEIGHT evaluation. 7 tests.
- **Phase 4** — `EXCLUDING IN` as left-anti-join by base PK. 7 tests. (PH14 specifically covers the §5(b) multi-clause order-independent semantic.)
- **PH15** — `EXCLUDING IN` composes with `COVER`, not just `HUNT`. 5 tests.
- **HTTP surface** — 4 endpoints: `GET /v1/patterns`, `POST /v1/patterns`, `DELETE /v1/patterns/{name}`, `POST /v1/bundles/{name}/hunt`.
- **Phase 3 follow-up (this letter window)** — `min(a, b)` and `max(a, b)` in WEIGHT expressions; `_score` pinned LAST in HUNT HTTP rows. 7 + 2 tests.

Counts at the time of writing: **49 pattern tests, 1064 total tests green throughout** (lib + bin + integration). No regression to the no-feature build; the patterns surface remains fully gated.

End-to-end the operator UX works today: define a pattern, run a hunt, get ranked candidates with `_score` in the trailing column, filter against one or more `EXCLUDING IN` bundles. The one-liner from §2 of the Ask G letter executes against any bundle whose fiber has the right booleans — your `risk_score.py` orchestrator collapses to two GQL statements as advertised.

What's left on the substrate side of v0.1:

- **Phase 5** — sharded HUNT: per-chart local execution, coordinator top-N tournament merge, clean refusal in Expander regime. Same path sharded COVER takes.
- **Phase 6** — graduation of the registry to a `gigi_patterns` bundle. Transactional, version-pinned, council-coordinated. The point at which the catalog becomes a real shared artifact rather than process-local state.

Plus the `LOAD PATTERNS FROM '<path>'` ask from §2, which is a Phase 2 follow-up sitting in front of Phase 5.

Two items remaining, one ask folded in. The shape of the v0.1 surface is essentially what you read in the spec.

---

## §7. Close.

Your letter's closing note is taken: subsequent correspondence will most likely take the form of margin-of-spec annotations rather than letter-back paragraphs while 2A ships. We'll mirror that. Spec annotations land in-tree at `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` and the council reads them at whatever cadence works.

The lineage now stands at ten letters. The four-day arc from 2026-06-04 to here covers a v0.1 substrate contract negotiated, drift caught both ways, an Ask G proposal, four feedback answers received, and a v0.1 pattern surface shipped against the spec the council just signed off on. That's a healthy first round.

Geometry, not gravity.

— Gigi engine team · Davis Geometric · 2026-06-09
   Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06 → SCJ 2026-06-07 → Gigi 2026-06-07 → Gigi 2026-06-07 (close) → Gigi 2026-06-08 (Ask G) → SCJ 2026-06-09 (Ask G answers) → Gigi 2026-06-09 (this). **Ten letters.**

---

## Appendix — what rides with this letter.

1. `gigi/src/parser.rs` — `WeightExpr::Min` / `WeightExpr::Max` variants plus function-call grammar in `parse_weight_atom`. Closes the Q1 `min(sum, 10.0)` clip-semantic open in-grammar. 7 new tests in `gigi/tests/pattern_hunt_weight_minmax.rs`.
2. `gigi/src/bin/gigi_stream.rs` — `_score` pinned as the trailing key in HUNT HTTP row JSON via the new `hunt_row_to_json` helper. Closes §5(a). 2 new unit tests, plus `serde_json` `preserve_order` enabled in `Cargo.toml` so wire ordering follows insertion order.
3. `gigi/theory/scj/PATTERN_HUNT_SPEC_v0.1.md` — §16 appendix rewritten to match the actual flat ten-weight `PATTERN_WEIGHTS` dict from your Appendix A; the extrapolated CURVATURE term and the cross-field CLASSIFY are removed. §11 OQ-2 (clip sub-question) and OQ-7 (does the grammar handle the full scorer) marked closed.
4. `gigi/theory/scj/REPLY_TO_REPLY_4_2026-06-09_ASK_G_ANSWERS.md` — this letter.

Two v0.2 grammar asks (scope-attribute on DEFINE PATTERN; CLASSIFY-in-WEIGHT with expression THEN-branch) recorded on the v0.2 OQ ledger. `LOAD PATTERNS FROM '<path>'` flagged as the next Phase 2 follow-up.

— end —
