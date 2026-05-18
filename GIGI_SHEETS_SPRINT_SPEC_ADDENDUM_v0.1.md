# GIGI Sheets — Sprint Spec Addendum v0.1 (audit answers)

**Date:** 2026-05-14
**Audits:** [`src/bin/gigi_stream.rs`](src/bin/gigi_stream.rs), [`sdk/js/`](sdk/js), [`playground/`](playground), [`site/`](site), [`GIGI_CRUD_ROADMAP.md`](GIGI_CRUD_ROADMAP.md)
**Companion to:** [`GIGI_SHEETS_SPRINT_SPEC.md`](GIGI_SHEETS_SPRINT_SPEC.md) §8 (Open questions)

This addendum resolves the four open questions from the parent spec, plus the
two practical choices (app location, grid library). The net result: **all four
unknowns are de-risked**, none of them blocks S0, and the engine work
originally scoped for S1 / S5 is materially smaller than I estimated.

---

## Discovery summary

The real database server is **`gigi-stream`**, not `gigi-server`. The latter
is a small JSON↔DHOOM conversion service on port 3141. `gigi-stream` is the
~8.8k-line axum app on port **3142** that the README refers to. The README
should be corrected separately; the spec needs to target `gigi-stream`.

The HTTP surface is dense (≥ 47 routes). The JS SDK (`sdk/js/`) already
wraps essentially all of it with a `client.bundle(name).<verb>()` shape.
This means Sheets's data layer is almost entirely off-the-shelf — we
import `@gigi-db/client` and consume the existing methods.

---

## Q1 — Schema discovery: **resolved ✓**

**Question:** Does `GET /bundles/:name` return a structured schema?

**Answer:** Yes. The endpoint is `GET /v1/bundles/{name}/schema` and the
response shape is:

```json
{
  "name": "sensors",
  "base_fields":  [{ "name": "...", "type": "...", "weight": 1.0 }],
  "fiber_fields": [{ "name": "...", "type": "...", "weight": 1.0 }],
  "indexed_fields": ["sensor_id"],
  "records": 1000,
  "storage_mode": "..."
}
```

The JS SDK already exposes `client.bundle(name).schema()` returning this
verbatim.

**Gap:** The response **does not** include encryption mode per field, nor a
distinguished `keys` field. For Sheets we need both:

- For the lock affordance + INDEXED/OPAQUE/AFFINE badge (S8).
- For knowing which fields are keys vs fiber (the row gutter, point
  queries, schema editor all need this).

**Implication for S0:** Render whatever the schema endpoint returns
today — base_fields, fiber_fields, indexed_fields. **No engine change
needed for S0.**

**New engine work item (slot before S8 starts):**

> **E-S8a — extend `/schema` response with encryption per field.** Add
> `encryption: "none" | "opaque" | "indexed" | "affine"` to each
> field descriptor. Tests:
> - `tests::schema_returns_encryption_mode_per_field`
> - `tests::schema_legacy_bundle_defaults_to_none`

Small (`[S]`), no breaking change.

---

## Q2 — PATCH κ-deltas: **partially resolved**

**Question:** Does single-section update return cohort κ updates?

**Answer (partial):** The endpoint exists as
`POST /v1/bundles/{name}/update` with this response:

```json
{
  "status": "updated",
  "data": { ... },        // present when returning: true
  "total": 1000,
  "curvature": 0.425,     // BUNDLE-LEVEL κ at time of write
  "confidence": 0.891
}
```

WebSocket subscribers also receive a `SubscriptionEvent`:

```
EVENT <bundle> update <record_json> K=0.425000 C=0.8910
```

**What we have:** Per-record curvature on write. Bundle-level κ on write.
WebSocket broadcast on every mutation. Optimistic-concurrency via
`expected_version`.

**What we don't have:** **No cohort κ deltas.** The response says nothing
about how κ changed for the *other* rows in the affected cover.

**Implication for S1:** Three options, in order of preference:

| Option | Cost | Behavior |
|---|---|---|
| **A. Client-side cohort recompute.** UI knows the cohort definition (site or whatever the chosen cover key is), refetches the cover after a write, and recomputes κ locally using the same kernel as `mockups/sheets.html`. | None on engine; ~80 LOC in `sheets/lib/kappa.ts`. | Correct under single-writer assumption. Wrong if another client mutated mid-flight. |
| **B. Engine returns the cover.** Update response includes `affected: [{id, kappa, confidence}]` — the engine knows the cohort because it just recomputed it. | Small engine PR. New tests: `tests::update_returns_affected_cohort_kappa`. | Single source of truth. |
| **C. Hybrid.** Option A for the cell-edit demo in S1, swap in Option B as an engine sprint after Sheets ships v0.1. | None upfront. | Lets S1 land without engine work; revisit before S2 if jitter is visible. |

**Recommendation: Option C.** S1 ships with client-side cohort recompute.
Engine sprint to add `affected` to the response runs in parallel as a
standalone PR. **S1 is unblocked.**

The WebSocket subscription means we *also* get the "other client mutated"
case for free in S1 by subscribing to the bundle and re-running the local
κ kernel whenever an `EVENT` arrives.

---

## Q3 — `ALTER BUNDLE`: **resolved, mostly ✓**

**Question:** What does schema mutation look like today?

**Answer:** Two of three operations exist as dedicated routes:

| Operation | Endpoint | Status |
|---|---|---|
| Add field | `POST /v1/bundles/{name}/add-field` | ✓ shipped |
| Drop field | `POST /v1/bundles/{name}/drop-field` | ✓ shipped |
| Add index | `POST /v1/bundles/{name}/add-index` | ✓ shipped |
| Rename field | — | **missing** |
| Change type | — | missing (large; deferred) |

The JS SDK already exposes `client.bundle(name).addField(...)` and
`.addIndex(...)`.

**Implication for S5:** Most of S5 is **already done at the engine layer**.
The sprint becomes UI-only for add/drop. Rename is a small engine
addition. Type-change stays deferred to v0.2.

**Reduced engine work for S5:**

> **E-S5a — `POST /v1/bundles/{name}/rename-field`.** Body: `{ from: string, to: string }`. WAL replay test required. Tests:
> - `tests::rename_field_persists_through_restart`
> - `tests::rename_field_rejects_to_existing_name`
> - `tests::rename_field_404_unknown`

Single-day work.

---

## Q4 — Real-time / WebSocket: **resolved ✓**

**Question:** Should the grid subscribe to changes? What's the shape?

**Answer:** Yes, easily. The engine already broadcasts `SubscriptionEvent`s
for every mutation:

```
EVENT <bundle> <op> <record_json> K=<kappa> C=<confidence>
NOTICE <bundle> lagged=<count>
```

Where `op ∈ {insert, update, delete, upsert, bulk_update, bulk_delete}`.
Subscribers can filter at subscribe-time:

```
SUBSCRIBE <bundle> [WHERE field op value [AND ...]]
SUBSCRIBE <bundle> ON K [> threshold]
```

The JS SDK already wraps this as `client.bundle(name).where(...).subscribe(cb)`.

**Implication for Sheets:**

- **S0 stays request-response.** Defer realtime to a dedicated sprint.
- **New sprint S2.5 (slot between S2 and S3): Realtime grid.** Subscribe
  on mount, apply incoming events to the local row map, retrigger κ
  recompute. Tests:
  - `tests/unit/applyEvent.test.ts` — given event + rowmap, rowmap is correct after.
  - `tests/component/Grid.realtime.test.tsx` — mock WS emits `EVENT`, grid row updates with no user input.
  - `e2e/sheets_two_clients.spec.ts` — two browser contexts; client A edits, client B's grid reflects within 500 ms.
- **Lag handling:** Show a small "behind by N" pill in the topbar when
  `NOTICE … lagged=N` arrives. Pure UI; no engine change.

---

## Q5 — App location and stack: **resolved**

**Existing React/UI projects share a tight convention:**

| Project | Framework | Build | Lang | Port |
|---|---|---|---|---|
| `playground/` | React 18.3.1 | Vite 5.4.2 | **JSX** (no TS) | 5174 |
| `site/` | React 18.3.1 | Vite 5.4.2 | **JSX** (no TS) | 5176 |
| `dashboard/` | React 18.3.1 | Vite 5.4.2 | (per repo pattern) | TBD |
| `sdk/js/` | — | tsc | **TypeScript** | n/a |

**Decision:** `sheets/` as a sibling at the repo root. Match the React/Vite
versions and Vite plugin config exactly. **Deviate** on TypeScript:

> Sheets ships in **TypeScript**, not JSX. Rationale: the grid carries a
> non-trivial state shape (selection, edits, filters, history, overlays,
> verb results), the SDK already exports types, and the spec requires
> typed test cases. The deviation is contained — both `playground/` and
> `site/` stay JSX, only Sheets adopts TS.

**Configuration:**
- Base path: `/gigi/sheets/`
- Dev port: `5177`
- Test runner: **Vitest** + `@testing-library/react` (new dependency in repo)
- E2E: extend the existing `e2e/` Playwright harness with a `sheets_*.spec.ts` set

---

## Q6 — Grid library: **decision**

**Decision: hand-rolled virtualized grid + `@tanstack/react-virtual` for
windowing only.**

Rationale: every full-featured grid library (AG Grid, glide-data-grid,
TanStack Table headless mode) makes assumptions about cell rendering and
row chrome that fight what we need:

- The κ-bar gutter in the row-header cell is custom.
- Anomaly halos that need to span both the row tint and the (Geometry tab)
  scatter point.
- Inline editor with a per-cell type-aware control (numeric vs categorical
  vs encrypted-readonly).
- Conditional formatting driven by geometric properties (κ, λ₁), not just
  cell values.

`@tanstack/react-virtual` solves only windowing, which is the one thing
we don't want to write ourselves. Everything else is straightforward
JSX + CSS grid.

**Test:** A `tests/perf/grid.bench.ts` Vitest benchmark asserts ≤ 80 DOM
rows present when rendering a 5,000-row dataset. Gates the merge of S0.

---

## Net effect on the sprint plan

| Sprint | Original scope | After audit |
|---|---|---|
| **S0** Foundation | Read schema, render rows | **Unchanged.** All endpoints exist. |
| **S1** Inline edit + WAL | Engine work: PATCH + κ-deltas | **Reduced.** Engine already supports update with curvature. Cohort κ via Option C (client-side, swap for engine `affected` in v0.2). |
| **S2** Overlay | CSS + threshold | Unchanged. |
| **+ S2.5** Realtime grid | (was not scoped) | **NEW.** Subscribe to mutations from other clients via the existing WS. ~3 days. |
| **S3** Inspector verbs | Wire to existing verbs | Unchanged. |
| **S4** Geometry tab | New view | Unchanged. |
| **S5** Schema mutation | Big engine PR | **Reduced.** ADD/DROP already shipped. Only RENAME is new (1 day). |
| **S6** Saved views | BFF + UI | Unchanged. |
| **S7** GQL editor | Reuse playground bits | Unchanged. |
| **S8** Encrypted fields | UI + (small) engine | Small engine PR to extend `/schema` with encryption mode (E-S8a). |
| **S9** Insights | New engine verb | Unchanged. |

**Net engine work** across the whole plan, post-audit:

1. **E-S1a** — *optional, deferrable.* Add `affected: [{id, kappa}]` to update response.
2. **E-S5a** — `POST /rename-field` + WAL replay tests.
3. **E-S8a** — encryption mode in `/schema` response.
4. **E-S9a** — `INSIGHTS` verb (already scoped in parent spec §5/S9).

Total engine surface: **two small PRs (E-S5a, E-S8a) before sprint kickoff,
two larger PRs (E-S1a optional, E-S9a) landing in parallel with UI work.**

---

## Corrections to the parent spec

1. **§4 "Surface area":** `gigi-server` → `gigi-stream`. The doc claim that
   "Sheets sits entirely above the existing HTTP/WS surface" is correct,
   but the binary name was wrong.
2. **§5 S1 "Engine work item":** the `PATCH` endpoint exists as
   `POST /v1/bundles/{name}/update`. Use the existing verb, not a new
   one. The κ-delta response shape is an optional engine PR
   (E-S1a, this addendum).
3. **§5 S5 "Engine work item":** ADD/DROP field are already shipped.
   Only RENAME is new (E-S5a).
4. **§3 IA, §4 surface, all e2e tests:** add subscription path
   (`client.bundle(...).where(...).subscribe(...)`) as a first-class
   data source. Introduce S2.5.
5. **§4 directory tree:** language is **TypeScript**, deliberate
   deviation from `playground/`/`site/`. Documented above.

---

## Ready to kick off S0

All open questions are answered. The engine work that the spec implied
exists, exists. The smaller new engine bits (E-S5a, E-S8a) are ≤ 1 day
each and don't block S0/S1.

**S0 kickoff checklist:**

- [ ] Scaffold `sheets/` (Vite + React 18.3.1 + TypeScript + Vitest + Playwright)
- [ ] Add `@tanstack/react-virtual` as the sole grid dependency
- [ ] Link to `sdk/js/` workspace; the existing `client.bundle().schema()` is the data source
- [ ] First test: `tests/unit/gigi-client.section.test.ts` (mocking the SDK response shape we just verified)
- [ ] First commit: empty scaffold + green test suite

Spec is unblocked.
