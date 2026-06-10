# Record-Vector Endpoint Shipped — IMAGINE Phase 2 Unblocked

**From:** GIGI team
**To:** Marcella team
**Date:** 2026-06-10
**Subject:** `−pv̂` placeholder retired — `seed_vec` now fetchable from substrate

---

You asked for the record's actual fiber coordinates so the walk direction
`along = normalize(prompt_vec − seed_vec)` could be geometrically honest
instead of the `−pv̂` proxy. Shipped today on `gigi-stream.fly.dev`.

Chose your **Option B** (dedicated endpoint), not Option A
(`include_vectors` on `/query`). Reasons:

1. **Single-record use case.** IMAGINE Phase 2 already has the nearest
   record's id from `load_substrate_catalog`. Pulling the one vector by
   id is what you actually need — adding an opt-in flag to `/query`
   would pay a payload cost on every catalog scan.
2. **Cacheable.** Vectors are immutable per (record, field) — a
   dedicated GET is trivially HTTP-cacheable. A flag on `/query` mixes
   payload modes and isn't.
3. **Clean ambiguity handling.** When records carry multiple Vector
   fields, the endpoint takes `?field=<name>` explicitly. On
   `/query`, the projection layer would have to guess.

## Contract

```
GET /v1/bundles/{name}/record/{id}/vector[?field=<name>]
X-API-Key: $GIGI_API_KEY

→ 200 OK
{
  "data": {
    "id": 42,                   // record's base-field value (typed)
    "field": "embedding",        // name of the Vector field returned
    "vector": [0.13, -0.27, …],  // the embedding, as f64[]
    "dims": 768                  // vector length
  },
  "meta": { "confidence": …, "curvature": …, "capacity": …, "count": 1 }
}
```

**Field selection rule.** If `?field=` is omitted, the endpoint returns
the **first Vector field in schema fiber-declaration order** (not
HashMap iteration order — that would be nondeterministic). So if your
substrate schema declares `embedding` before any other Vector field,
omitting the query param is safe.

**Pin-by-name path.** Pass `?field=embedding` for forward stability
against future schema additions. Recommended for production.

## Error envelope

| Status | Cause                                                  |
| ------ | ------------------------------------------------------ |
| 400    | Bundle has >1 base field (composite key). Use `/get?{k}={v}`. |
| 400    | `?field=<name>` resolved but the field is not a Vector |
| 401    | Missing/invalid API key                                |
| 404    | Bundle not found                                       |
| 404    | Record not found at that id                            |
| 404    | No Vector field on the record (and no `?field=` hint)  |
| 404    | `?field=<name>` not present on the record              |

## Constraints

- **Single-base-field bundles only.** Substrate catalog qualifies
  (single `id` base). Composite-key bundles return 400 with the
  fallback path in the error message.
- **No auth bypass for this endpoint** — same `X-API-Key` middleware
  as every other `/v1/bundles/*` route.

## Your diff

Apply the two-line swap you mocked up — the API surface matches your
proposal exactly:

```python
# load_substrate_catalog still works as before; it returns ids.
# For Phase 2:

seed_vec = client.get_record_vector(
    bundle="substrate",
    record_id=nearest.id,
    field="embedding",  # explicit; future-stable
)
starting_from = seed_vec
along = normalize(prompt_vec - seed_vec)
```

## Verification

- **Lib suite:** 1354/1354 green with the new helper
  `gigi::types::first_vector_field`.
- **Contract tests:** `tests/record_vector_endpoint.rs` — 6/6 green:
  - schema-order first-Vector selection
  - `point_query` round-trips `Value::Vector` unchanged
  - `None` when no vector field present (→ 404)
  - `None` for missing record (→ 404)
  - composite-key arity rejection (→ 400)
  - response wire shape has exactly `{id, field, vector, dims}`
- **Release build:** clean, 9 unrelated dead-code warnings.
- **Deploy:** `gigi-stream` machine healthy; image
  `deployment-01KTRZ3MRBZV48F4WB1ZYPNE93`.

## What this does NOT do

- Does **not** add bulk vector retrieval. If IMAGINE Phase 3 wants a
  batch (e.g. all k nearest seeds at once), tell me and I'll add
  `POST /v1/bundles/{name}/records/vectors` taking `{ids: [...]}`.
- Does **not** change `/query` behavior. If you ever want vectors in
  the catalog scan response itself, Option A is still on the table —
  it just wasn't the cleanest fit for IMAGINE Phase 2.
- Does **not** decrypt encrypted fibers. If your substrate bundle
  has `embedding` under an `Affine`/`Isometric` encryption mode, the
  vector returned is the **gauge-encrypted** representation. That's
  the same thing `point_query` would surface today — if IMAGINE
  needs the plaintext, that's a follow-up question about gauge keys.

The "starting_from = seed_vec" line is finally honest. Ship Phase 2
whenever you're ready.

— GIGI team
