# Reply to Marcella's L13.5 persistence-ratio overflow finding

**From.** Bee Davis + GIGI engine team (Claude pair).
**To.** Marcella team.
**Date.** 2026-05-25 (~23:00 UTC).
**Re.** Your `REPLY_L13_5_FILTER_PROBE_2026-05-25.md` (`f0f64e4`).
**Status.** Unified-floor fix shipped this commit. L13.6 (σ² floor)
+ L13.7 (gap-denominator floor) now both ride the same idiom you
named.

---

## TL;DR

You named the abstraction correctly: **two pathologies, one class,
one idiom**. L13.7 ships the gap-denominator floor with the same
`max(value, ε × scale, 1e-300)` pattern as L13.6 σ², from the
same starting point: a Marcella probe revealed honest-fit failure
on production data; the substrate gained a defensive default; the
fix is in production the same evening.

```
                Idiom                           Where it lives now
─────────────────────────────────────────────────────────────────────────────
σ²-floor          max(σ², ε × median(σ²),       fit_diagonal_gaussian
(L13.6)                3 × DT_DEFAULT, 1e-12)   gigi_stream.rs

gap-floor         max(median, ε × max_gap,      episodic_events_with_floor
(L13.7)                1e-300)                  src/geometry/memory.rs
```

Both shipped. Both contract-tested. Both surface diagnostics in
the response so consumers can see what got floored. **No unified
helper** yet — they're in different modules with different scales
— but the *shape* of the fix is identical, and the next pathology
of this class (if there is one) will probably ride the same
template again.

---

## What changed for `/brain/episodic`

Wire add (one optional field, default = the new floor):

```json
POST /v1/bundles/{name}/brain/episodic
{
  "field": "ingested_at",
  "min_persistence_ratio": 50.0,
  "where_field": "record_kind",            // L13.5 (already shipped)
  "where_value": "section",                //
  "gap_floor_epsilon": 1e-6                ← NEW (default 1e-6)
}
```

Response add:
```json
{
  ...,
  "filter_applied": {...},
  "gap_floor_epsilon_used": 1e-6           ← NEW (echo the floor)
}
```

What `gap_floor_epsilon = 1e-6` does for your bge case:

- max_gap on `ingested_at` = ~234 000 s (65h, as you measured)
- relative floor = 1e-6 × 234 000 = 0.234 s
- new denom = max(median, 0.234, 1e-300) — with batched timestamps
  median is ~0, so denom = 0.234
- new ratio = 234 000 / 0.234 ≈ **1 × 10⁶** (capped, not 10²⁸⁸)

Reported `persistence_ratio` values now top out near `1 / ε`,
which is large enough to fire on any caller's `min_persistence_ratio`
threshold but stays a finite number.

**Trade-off you should know:** with ε=1e-6 floor, you can't
distinguish two gaps both above the cap. For your bge case (gaps
either ~0 or ~234000s), this never matters — they're separated by
9 orders of magnitude. For data with multiple gaps in the
near-overflow regime, the ordering survives but the *ratios*
flatten near 1e6. Pass smaller ε for more headroom or 0 to
disable (escape hatch, only safe on well-spaced data).

---

## Unified-floor template, for the record

Per your note: **same `max(value, ε × scale)` idiom resolves
both**. The template generalizes — if a third class of pathology
shows up (zero-determinant covariance, near-collinear principal
direction, etc.), the fix probably looks the same. We're not
landing a generic helper yet because the scale function differs
(median vs max vs det), but if you spot a third instance, ping —
it's worth factoring out at three.

```rust
fn defensive_denominator(
    raw: f64,
    scale: f64,           // domain-specific: median, max, det, ...
    relative_epsilon: f64, // tunable; 1e-3 for σ², 1e-6 for gaps
    absolute_floor: f64,   // 1e-12 or 1e-300 as makes sense
) -> f64 {
    raw.max(relative_epsilon * scale).max(absolute_floor)
}
```

If/when you want this consolidated, ship `gigi::geometry::math::
defensive_denominator` and have both call sites import. Half a
commit; on-demand.

---

## Acknowledgements (no action needed)

- **You picked the right Option** on the L13.5 base-field caveat:
  Option B (keep per-user `now − last_seen` for greetings, use
  `/brain/episodic` for bundle-wide analytics) preserves the
  per-record-scale FEP primitive you already shipped. Mirroring
  `user_id` into `fiber_fields` (Option A) is correct long-term
  but isn't blocking; do it whenever the schema-evolution pass
  feels right.

- **Filter mechanism working cleanly** confirmed by your second
  test (the bge `record_kind=section` query returning 6864
  post-filter, 4 events, filter_applied echo present). Wire shape
  is locked.

- **"Honest-fit failure beats hidden-fit success"** lands again.
  The unfloored persistence_ratio honestly told you the median
  was near-zero (and the math wouldn't lie about what divide by
  zero means). The fix isn't to silently average; it's to floor
  the denominator with disclosure (`gap_floor_epsilon_used` in
  the response). Same philosophy as L13.6's `fit_floored_indices`.

---

## Status snapshot post-L13.6 + L13.7

| Pathology | Origin | Fix | Status |
|---|---|---|---|
| σ² → 0 (rank-deficient axes) | L13.3 deploy | floor on σ² (L13.6) | shipped `c80c93a` |
| median(gap) → 0 (clustered times) | L13.5 probe | floor on denominator (L13.7) | shipped this commit |
| ATTEND query echoes self at 0.0182 (function-word clusters) | L13 probe | working as designed | no action |
| ATTEND flat on content tokens | L13 probe | needs bge re-ingest | your side |
| SAMPLE T-ratio aggregate 2.0× | L13.3 → L13.6 | per-axis correct; aggregate weighted by floored vs non-floored | re-probe |

---

## What you'd want to verify on re-probe

After this deploy lands (≈10 min from this letter), re-running
your `f0f64e4` probe should give:

1. The 4 bge `ingested_at` events should report finite
   persistence_ratios — probably in the 1e3 to 1e6 range now,
   not 1e288.
2. `gap_floor_epsilon_used` in the response should equal 1e-6
   unless you passed a different value.
3. The diagonal-fit re-probe (L13.6) should show
   `fit_floored_indices` containing the v11_fiber degenerate dims
   (your f12 / f13 / f14), with samples staying finite.

If anything else looks off, ping. We're at a tight feedback rhythm
now — Marcella probe → fix → deploy → re-probe is running on
~hour-scale.

---

## Three v3 paper observations from this cycle

1. **The substrate as diagnostic instrument again.** L13.5's
   overflow surfaced your bge bundle's batch-ingest structure
   (timestamps clustered → median → 0). The math wasn't broken;
   it was correctly reporting "there's no useful gap scale in
   this data." Worth landing in the paper as "FEP primitives
   surface structural facts about the data that aggregate
   summaries hide."

2. **Floor diagnostics as observability.** The
   `gap_floor_epsilon_used` + `fit_floored_indices` response
   fields are basically "the substrate telling you which math it
   defended itself from." If you keep tracking these per-probe
   over time, you get a history of "how degenerate is this
   bundle today" — could be a useful operational metric.

3. **The morning's compound feedback loop.** L13.3 deploy →
   probe → diagnosis (L13.3 explosion) → L13.6 fix → deploy →
   probe → diagnosis (L13.5 overflow, same class) → L13.7 fix →
   deploy. All inside ~6 hours of clock time. That's the cycle
   the substrate was built to support: production-data
   pathology surfaces fast, the fix lands fast, the contract
   test pins it permanently. Worth a sentence in the paper's
   "operational learnings."

---

— Bee + GIGI engine team (Claude pair)

P.S. Your three explicit agreements (lifecycle-tied gap capture,
14-threshold calibration table for the paper, the seconds-ago
negative control) — all logged for the v3 paper queue. The
calibration table specifically is becoming useful enough that we
should write it up as a one-pager that other Marcella features
can import — same shape that "FEP signal X ↔ user copy Y" mapping
will take for any future EPISODIC consumer.
