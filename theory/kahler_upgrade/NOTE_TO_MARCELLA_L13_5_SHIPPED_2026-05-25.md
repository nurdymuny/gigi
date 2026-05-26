# Note to Marcella team — congrats on the `bee_session_recap` ship + L13.5 filter

**From.** Bee Davis + GIGI engine team (Claude pair).
**To.** Marcella team.
**Date.** 2026-05-25 (~22:00 UTC).
**Re.** Your `334a01b` (gap-aware session recap) + the per-key
EPISODIC filter you named as the upgrade path.

---

## TL;DR

Two things:

1. **Your ship is the right architectural call.** Per-user
   `now - last_seen` at restore time IS the FEP primitive at
   single-record scale; calling `/brain/episodic` for the
   bundle-wide aggregate on every session start would be doing
   the wrong work. The 14-threshold humanize calibration tied to
   the EPISODIC probe output is a clean translation between the
   topological signal and the user-facing greeting.

2. **L13.5 ships your named upgrade path.** `/brain/episodic`
   now accepts optional `where_field` + `where_value`. When
   supplied, the endpoint pre-filters records to those whose
   `where_field` matches `where_value` before running
   change-point detection. Lets you use `persistence_ratio`
   directly on a single user's history without precomputing the
   per-user time series yourself.

---

## What changed

```json
POST /v1/bundles/{name}/brain/episodic
{
  "field": "last_seen",
  "min_persistence_ratio": 50.0,
  "where_field": "user_id_fiber",   // ← NEW: filter records
  "where_value": 42                  //   before computing events
}
```

Response gains a `filter_applied` echo when a filter is supplied:

```json
{
  "events": [...],
  "n_records": 47,           // post-filter count
  "threshold_used": 50.0,
  "filter_applied": {
    "field": "user_id_fiber",
    "value": 42
  }
}
```

Filter accepts numeric (i64 / f64), string, and bool match values
— whatever's in the JSON body matched loosely against the stored
`Value` variant.

---

## One caveat (relevant to your schema)

`/brain/episodic` filtering currently supports **fiber fields only**.
A `where_field` that points to a `base_field` returns a 400 with
the explicit message:

> where_field 'user_id' is a base_field; /brain/episodic per-key
> filter currently supports fiber_fields only. To filter on a
> base_field, query records by that key on your side and POST the
> resulting per-cohort time series.

For your specific `persistent_memory` schema: if `user_id` lives
in `base_fields`, two ways to proceed:

- **Option A** (cleaner long-term): mirror `user_id` into
  `fiber_fields` as a categorical/numeric column (one-time schema
  evolution). Then `where_field: "user_id"` Just Works on the
  endpoint.
- **Option B** (zero-schema-change): your current approach — pull
  the user's records via base-field lookup on your side, then
  POST the time series for change-point detection via either
  (a) a future `/v1/brain/episodic` endpoint that takes raw
  values (let us know if useful — small follow-up), or
  (b) keep the current per-user `now - last_seen` for greetings,
  use `/brain/episodic` for bundle-wide analytics.

We can ship base-field hashed filtering as L13.6 if it would
unblock you; otherwise your current architecture is correct and
this filter just expands the option space.

---

## Verification

`tests/kahler_brain_endpoints_contract.rs::brain_episodic_filter_recovers_per_cohort_change_point`
exercises the filtered path: a synthetic bundle with two
interleaved cohorts (one stable, one with a 1→10 value jump
half-way through). Without filtering, the bundle-wide series
shows the jump diluted by cohort 0 noise. With
`where_field: "cohort", where_value: 1.0`, the filtered cohort 1
returns a change-point at gap ≈ 9 and persistence ratio > 100×.

Smoke script (`examples/brain_endpoints_smoke.py`) now exercises
the filter alongside the unfiltered call.

---

## On your ship architecturally

Three things to flag explicitly:

1. **`_session_resume_gap_seconds` is the right place for the
   capture** — populating it inside `_restore_from_persistent_memory`
   ties it to the actual data-load lifecycle, not the
   conversation-start lifecycle. If a restore fails, the gap
   isn't stale. Good defensive design.

2. **The 14 calibrated thresholds map to the EPISODIC ratio
   regime in a way that's worth documenting once.** The
   `< 6h → ""` skip-band corresponds to "below the noise floor
   that EPISODIC's persistence-ratio default of 50× would catch."
   The `6h–24h` band is where persistence ratios start to look
   like real events; the `1d–3d` band is where the probe's 179×
   hit lives; the longer bands are the "definitely a discrete
   episode" regime. If you write a one-pager mapping
   `(humanize_gap, persistence_ratio_floor, days_elapsed)` it
   becomes a reusable calibration table for any other place you
   want to translate FEP signal → user copy.

3. **The "Bee literally just talked to Marcella seconds ago" no-op
   case is the right negative control.** Confirms the gap-clause
   doesn't fire on within-session continuation. That's the path
   that would have leaked weird "Welcome back" greetings into
   normal multi-turn flow.

---

## Status post-L13.5

| Item | Status |
|---|---|
| Finding 1 (state_corrections topology — dashboard idea) | yours when you want it |
| Finding 2 (EPISODIC trigger in session_recap) | **shipped your side `334a01b`** |
| Finding 3 (diagonal Gaussian fit) | shipped our side `ed3cb3c` (24h promise → 90 min) |
| Finding 4 (bge re-ingest for SELF-MONITOR gate) | yours; schema sketch in earlier reply |
| Finding 5 (syntactic vs semantic ATTEND) | observation captured; no action needed |
| Per-key EPISODIC filter (your named upgrade path) | **shipped this commit** |
| `GET /brain/semantic/history` (Betti trajectory) | on-demand |
| Holo-bisectional bandwidth opt-in | on-demand |
| Base-field filtering for /brain/episodic (L13.6) | on-demand if Option A above doesn't fit |
| Raw-values `/v1/brain/episodic` endpoint | on-demand follow-up |

---

— Bee + GIGI engine team (Claude pair)

P.S. Congrats on the calibration. 14 thresholds tied to the same
topological signal that drives EPISODIC is the kind of detail
that makes the v3 paper land — concrete demonstration that the
FEP primitive translates into user-facing behavior without
hand-tuning.
