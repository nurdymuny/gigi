# Ship report to SCJ — 2026-06-09 (Patterns v0.1 live on prod)

> *Companion to the same-day Round 10 reply. That letter answered Q1–Q4
> in spec terms; this one tells you the wire is hot.*

To: SCJ ingest team
From: Gigi engine team · Davis Geometric
Re: Patterns v0.1 surface is now reachable at `https://gigi-stream.fly.dev`. Receipts, call shapes, and the substrate ledger.
Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06 → SCJ 2026-06-07 → Gigi 2026-06-07 → Gigi 2026-06-07 (close) → Gigi 2026-06-08 (Ask G) → SCJ 2026-06-09 (Ask G answers) → Gigi 2026-06-09 (Round 10 reply) → this. **Eleven letters.**

---

## §1. What's live, as of right now.

`gigi-stream@deployment-01KTFJ329J2SCG3QZ1ETVHA3C3` rolled to a good state at 2026-06-09. The Patterns feature flag is enabled in the production build (Dockerfile feature list now reads `"kahler imagine sharded transactions patterns"`). DNS verified, health check green, machine in steady state. You can point your client at it today.

The four HTTP endpoints, in the shape the deployed engine accepts:

```http
GET    /v1/patterns
POST   /v1/patterns                  { name, predicate, weight?, using?, replace? }
DELETE /v1/patterns/{name}
POST   /v1/bundles/{name}/hunt       { pattern, excluding?, top?, project? }
```

Auth: `X-API-Key` header. Body shape verified end-to-end against the live engine in §3.

One field-name note in case you grep for the spec form: the JSON key for the predicate body is `predicate`, not `pred`. The spec EBNF uses `AS <expr>` as the keyword, the Rust struct uses `predicate`, and the v0.1 wire contract is the Rust struct. We'll rev the spec text to mirror in the next pass.

---

## §2. The risk_score.py translation, live.

Using the corrected §16 appendix (your 2026-06-09 Appendix A ratified, flat 10-weight linear sum, no CURVATURE, no cross-field CLASSIFY):

```sql
DEFINE PATTERN scj_vid_v01 AS
    cast_truncate_alloc >= 0
WEIGHT (
    min(
        cast_truncate_alloc                  * 3
      + multiply_before_alloc                * 3
      + shift_before_alloc                   * 3
      + param_times_const                    * 2
      + unchecked_param_to_size              * 2
      + mdl_shift_size                       * 2
      + reaches_ExAllocatePool2              * 1
      + reaches_MmBuildMdlForNonPagedPool    * 1
      + has_probe_read                       * 1
      + has_probe_write                      * 1,
        10
    )
)
USING (cast_truncate_alloc, multiply_before_alloc, shift_before_alloc,
       param_times_const, unchecked_param_to_size, mdl_shift_size,
       reaches_ExAllocatePool2, reaches_MmBuildMdlForNonPagedPool,
       has_probe_read, has_probe_write);

HUNT scj_vid_v01 IN vid_funcs
    EXCLUDING IN confirmed_bugs
    TOP 50
    PROJECT (name, module, _score);
```

That parses, registers, executes, and the in-grammar `min(…, 10)` clip works end-to-end. We verified it on the deployed engine — a 5-row toy bundle with `min(a*5 + b*5, 10)` where `a + b = 11` always clipped every row's `_score` to exactly `10.0`. The clip is the substrate's responsibility now; no consumer-side post-processing.

The orchestrator collapses to one HUNT statement, as advertised. `AUDIT_THRESHOLD = 7` stays consumer-side — apply it as a row filter on the returned `_score` column or hide rows in the TUI.

---

## §3. Receipts.

**Tests** (passed at the time the commits landed):

  - **49 pattern tests** across six test files, all green with `--features patterns`:
    `pattern_hunt_parser` (15), `pattern_hunt_registry` (8), `pattern_hunt_executor` (7), `pattern_hunt_excluding` (7), `pattern_hunt_cover_excluding` (5), `pattern_hunt_weight_minmax` (7).
  - **1064 total tests** green with `--features patterns` (lib + bin + integration).
  - **849 lib tests** green with no features (byte-identical no-feature build — the `patterns` flag is additive, the no-flag build path is unchanged).

**Live probes** against `gigi-stream.fly.dev`:

  | Probe | Result | Surface verified |
  |---|---|---|
  | `postdeploy_smoke.py` | **10/10** | health, existing bundles, SUDOKU + #107 mmap fix, SAMPLE_TRANSPORT, fit_diagnostics, confidence, Cech preflight |
  | `scj_patterns_live_probe.py` (new, ride-along) | **22/22** | DEFINE with `min()`, SHOW, HUNT, `_score` LAST in wire JSON, EXCLUDING IN drops the right rows, TOP-N + PROJECT, DEFINE OR REPLACE collision behavior, max() reorders, DROP |
  | `tx_http_probe.py` | **19/19** | BEGIN/COMMIT across two bundles, STATUS mid-tx, ROLLBACK discards, post-commit refusal, isolation modes, system-bundle 403, malformed/unknown tx_id 400/404 |
  | `sharded_imagine_probe.py` | **16/16** | sharded curvature 4-chart split, trivial + Möbius holonomy loops, imagine_coherence trajectory + provenance audit + honest refusal + routing advisory |
  | `auth_chain_diag.py` | **7/7** | auth wall alive, new key accepted, old key rejected, no whitespace bug |

**Net live result: 74 / 74 on prod surfaces.**

The `_score`-LAST wire contract verified against a live HUNT response — actual JSON observed:

```json
{"id": 1, "a": 1, "b": 10, "_score": 10.0}
```

`_score` is the trailing key in the serialized row. We enabled `serde_json`'s `preserve_order` feature to make this a structural property (insertion order is preserved by `IndexMap`-backed `serde_json::Map`) rather than relying on alphabetical sort, so your TUI parsers can rely on the position.

---

## §4. The new permanent probe.

`e2e/probes/scj_patterns_live_probe.py` is checked in on `main` and `scj-v0.1-substrate`. Twenty-two checks, exits non-zero on any FAIL. Designed as a post-deploy gate that you can run too — if you ever want to verify the surface from your side after a Gigi redeploy:

```bash
GIGI_API_KEY=$YOUR_KEY python e2e/probes/scj_patterns_live_probe.py
```

It creates two ephemeral bundles, exercises the full DEFINE → HUNT → EXCLUDING IN → DROP arc, and cleans up after itself.

---

## §5. What landed, commit by commit.

Eleven commits on `scj-v0.1-substrate`, all also on `main`:

```
072f19e  scj: Ask G — Patterns spec v0.1 + reopening letter
dfe171e  scj/patterns: Phase 1 — parser-only DEFINE PATTERN / HUNT / EXCLUDING IN
4169414  scj/patterns: Phase 2 — in-memory pattern registry on Engine
570ed46  scj/patterns: Phase 3 — HUNT planner + executor with WEIGHT evaluation
6c07595  scj/patterns: Phase 4 — EXCLUDING IN as left-anti-join by base PK
bdcd4a5  scj/patterns: PH15 — EXCLUDING IN composes with COVER, not just HUNT
bff2da0  scj/patterns: HTTP surface — 4 endpoints wrapping the GQL verbs
4d0ec66  scj/patterns: min(a,b) + max(a,b) in WEIGHT — closes SCJ §1 clip semantic
69e7cb4  scj/patterns: pin _score LAST in HUNT HTTP rows — closes SCJ §5(a)
1d29bdd  scj/patterns: spec §16 corrected + Round 10 reply letter
807704e  probes: add SCJ Patterns live probe — gates the full v0.1 HTTP surface
```

Plus two infrastructure commits riding alongside: `bb52c2a` (postdeploy probe fix — fit_diagnostics + confidence smoke now use fiber-only fields) and `c617fd2` (Dockerfile enables `patterns` in the prod build).

That's the full v0.1 ship.

---

## §6. The substrate ledger — what's still pending on our side.

In rough priority order:

  - **`LOAD PATTERNS FROM '<path>'` (Phase 2 follow-up)** — next deliverable. TOML preferred per your stated preference, `.gql` accepted by extension as a bulk-DEFINE script. Identical semantics to running each pattern definition individually — same registry writes, same redefinition rules, same error surface. Targeting the next commit on `scj-v0.1-substrate`.

  - **Phase 5: sharded HUNT** — per-chart local execution + coordinator top-N tournament merge + clean refusal in Expander regime. Same path sharded COVER takes. Unblocks deployment beyond toy corpora.

  - **Phase 6: graduation off the `patterns` feature flag** — registry becomes a `gigi_patterns` bundle. Transactional, version-pinned, council-coordinated. DEFINE/DROP become 2PC participants. HUNT reads pin to the caller's MVCC snap_id. This is the point at which the catalog becomes a real shared artifact rather than process-local state.

The two v0.2 grammar asks from your Round 10 letter (scope-attribute on DEFINE PATTERN; CLASSIFY-in-WEIGHT with expression THEN-branch) sit on the v0.2 OQ ledger alongside Ask A / Ask B / Ask C. They surface in the v0.2 RFC when that round opens.

---

## §7. Close.

The receipts:

  - 11 commits on `scj-v0.1-substrate`
  - 49 pattern tests + 1064 total tests green
  - 74/74 live probe checks on prod
  - 1 production deploy, machine in steady state
  - 0 regressions to the no-feature build

The two SCJ-flagged opens (Q1 clip semantic; §5(a) `_score` column position) are closed in-grammar and on-wire. The spec's §16 appendix mirrors your actual scorer. The Round 10 reply letter ships in-tree at `theory/scj/REPLY_TO_REPLY_4_2026-06-09_ASK_G_ANSWERS.md`. The wire is hot.

Whenever you're ready, your client can issue `POST /v1/patterns` and watch HUNT come back with `_score` in the trailing column. The substrate's done its half; the catalog's yours.

Geometry, not gravity.

— Gigi engine team · Davis Geometric · 2026-06-09 (post-deploy)
   Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06 → SCJ 2026-06-07 → Gigi 2026-06-07 → Gigi 2026-06-07 (close) → Gigi 2026-06-08 (Ask G) → SCJ 2026-06-09 (Ask G answers) → Gigi 2026-06-09 (Round 10 reply) → Gigi 2026-06-09 (ship report, this). **Eleven letters.**

— end —
