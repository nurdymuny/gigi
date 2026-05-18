# GIGI Sheets — Sprint Spec (v0.1)

> A spreadsheet/Airtable-shaped front-end for GIGI bundles, aimed at
> non-technical operators. Renders bundles as live, editable grids while
> keeping every geometric property (κ, λ₁, holonomy, Betti, encrypted fields)
> first-class in the UI.

The mockup at [`mockups/sheets.html`](mockups/sheets.html) is the **visual + interaction
contract** this spec operationalizes. Every interaction in the mockup that
involves computation is wired to a verb that already exists in the engine
(`SECTION`, `INTEGRATE`, `CURVATURE`, `SPECTRAL`, `HOLONOMY`, `TRANSPORT`,
`BETTI`) — the front-end is a thin, opinionated rendering of those verbs.

---

## 1 · Goals and non-goals

### Goals

- Operators with **zero GQL** can read, edit, filter, sort, and group a
  bundle as if it were an Airtable base.
- The **geometry never hides**: every row carries κ; every selection shows
  confidence/capacity/λ₁; outliers are visible before you ask for them.
- Every UI gesture has a **deterministic GQL preview** — the footer always
  shows the query the user just expressed by clicking. The grid is a
  query builder.
- **Schema is mutable** from the UI: add / rename / drop fields,
  add rows, change types — all backed by GQL `ALTER BUNDLE` /
  `CREATE BUNDLE` / `SECTION` writes.
- **Encrypted fields** (Encrypt v0.2) are first-class: never decrypted in
  the UI, queryable on equality when `INDEXED`, gauge-transformed where
  `AFFINE`. Lock-icon column adornment is required.

### Non-goals (v0.1)

- No collaborative real-time cursors (Linear-style).
- No spreadsheet formulas. Cells are values, not expressions. We may add
  a `COMPUTED` field type backed by `INTEGRATE` / `CURVATURE` later.
- No multi-bundle joins in the grid — that ships with the
  Geometry-and-Joins surface in a later spec.
- No mobile-first layout. Desktop ≥ 1280 px is the target.

---

## 2 · Personas

| Persona | Pain | Sheets answer |
|---|---|---|
| **Field operator** (no SQL) | "Which sensors are off today?" | Anomalies-only filter chip, status pills, κ bar in the row gutter |
| **Domain analyst** | Wants to slice but not to write GQL | Click column headers to sort, drag chips to filter, Insights drawer summarizes |
| **GIGI power user** | Mixes UI exploration with raw GQL | GQL tab + footer preview round-trip ↔ grid view |
| **Compliance / auditor** | Needs to know nothing decrypts in the UI | Encrypted-field lock affordance, INDEXED/OPAQUE/AFFINE badge per column, audit log per cell mutation |

---

## 3 · Information architecture

```
Workspace
└── Bundle (= "table" to the user)
    ├── Saved view  (filter + sort + group + visible columns)
    │   ├── Grid     (the spreadsheet)
    │   ├── Geometry (2D fiber scatter)
    │   ├── Charts   (count-by, histograms, κ/conf)
    │   ├── Kanban   (group by categorical field)
    │   └── GQL      (editor + result)
    └── Schema (fiber field list, encryption mode, indexed flag)
```

A **row** is a `SECTION` of the bundle. The row gutter renders κ. The
inspector panel always describes the currently-selected row.

---

## 4 · Surface area / modules

```
sheets/
├── src/
│   ├── app/                # router, layout, workspace state
│   ├── lib/
│   │   ├── gigi-client.ts  # thin SDK wrapper (typed verbs)
│   │   ├── gql.ts          # AST + serializer; UI builds AST, renders GQL
│   │   └── geometry/       # κ / spectral / transport helpers for offline preview
│   ├── views/
│   │   ├── Grid.tsx        # virtualized data grid
│   │   ├── Geometry.tsx    # SVG scatter, transport overlay
│   │   ├── Charts.tsx
│   │   ├── Kanban.tsx
│   │   └── GqlEditor.tsx
│   ├── components/
│   │   ├── Inspector.tsx   # gauges + verb runner
│   │   ├── VerbCard.tsx
│   │   ├── Filter/Sort/Group chips
│   │   ├── EncryptedBadge.tsx
│   │   └── Insights.tsx
│   └── server/             # optional BFF — auth, view persistence
└── tests/
    ├── unit/               # vitest
    ├── component/          # @testing-library/react
    └── e2e/                # playwright (extends repo's e2e/)
```

Engine-side: **no new modules**. Sheets sits entirely above the existing
HTTP/WS surface of `gigi-server` (port 3142). Where engine support is
needed it is called out explicitly in the sprint plan and added as a
separate engine sprint with its own tests.

---

## 5 · Sprint plan (TDD-first)

Every sprint follows the repo convention: **write the test, then make it
pass, then land the spec section**. Each sprint is sized to ≤ 1 working
week and exits with green `cargo test` + green `vitest` + green
`playwright`.

### Sprint S0 — Foundation

**Goal:** Renderable shell. No interactivity. Just proves we can read a
bundle and render rows.

- Stand up `sheets/` with Vite + React + TS, no router-overkill.
- Implement `gigi-client.ts` with one method: `section(bundle, opts) →
  Row[]` against `GET /bundles/:name/section?…`.
- Render the top bar / sidebar / grid shell. No editing, no inspector,
  no overlays.

**Tests (must exist before code):**

- `tests/unit/gigi-client.section.test.ts`
  - happy: parses a SECTION response into `Row[]`.
  - error: 4xx → typed error, no UI crash.
  - timeout: 30 s → typed error.
- `tests/component/Grid.boot.test.tsx`
  - renders header cells in schema order.
  - renders one row per response item.
  - shows skeleton while loading.
- `e2e/sheets_boot.spec.ts` (Playwright; needs running `gigi-server`)
  - load `/sheets/sensors` → see ≥ 1 row.

**Exit:** Open `localhost:5173/sheets/sensors` against a live server,
see the bundle.

---

### Sprint S1 — Inline edit + WAL durability

**Goal:** The killer demo. Edit a cell, watch κ recompute and persist
across server restart.

- Cell-level edit with optimistic UI.
- Single-cell write hits `PATCH /bundles/:name/section` (engine work
  item — see below).
- Every commit appends to WAL (`src/wal.rs` — exists). After server
  restart, the value survives.
- On commit success, the response carries the updated κ for the row
  and `affected_kappa: [(id, k')]` for cohort recompute. UI applies it.

**Engine work item (separate Rust PR, lands first):**
- Add `PATCH /bundles/:name/section` returning the updated row + the
  cohort κ deltas. Tests live in `tests/http_patch_section.rs`.
- Tests:
  - `tests::http_patch_persists_through_restart` — write, kill, restart, read.
  - `tests::http_patch_emits_kappa_delta` — response includes affected rows.
  - `tests::http_patch_rejects_encrypted_field_plaintext` — 400.

**UI tests:**

- `tests/component/Grid.edit.test.tsx`
  - click numeric cell → input rendered with value selected.
  - Enter commits, Esc cancels.
  - failure rolls back the optimistic value and toasts the error.
- `tests/unit/applyKappaDelta.test.ts`
  - given a row set and a delta payload, rows update in place.
- `e2e/sheets_edit_persists.spec.ts`
  - edit a value, hard-reload, value is still there.

**Exit:** The single demo line: *"change S-0117's temp to 45 and watch
its κ-bar light up red."*

---

### Sprint S2 — Geometry overlay + κ in the gutter

**Goal:** The grid always carries κ; the overlay toggle tints anomalies.

- κ-bar in row gutter (CSS gradient driven by `--p`).
- `data-overlay="on|off"` body attribute → CSS handles tinting.
- Row class derived from κ thresholds: `bad ≥ 2.0`, `warn ≥ 0.8`.

**Tests:**

- `tests/unit/kappaClass.test.ts` — pure function. Threshold table.
- `tests/component/Gutter.test.tsx` — given κ=4.2, bar is `.bad`,
  width >= 80%.
- `tests/component/OverlayToggle.test.tsx` — clicking the toggle
  flips `data-overlay` and the row bg.

**Exit:** Visual diff against the mockup baseline.

---

### Sprint S3 — Inspector + geometric verbs

**Goal:** Selection drives an inspector that runs `SPECTRAL`,
`TRANSPORT`, `HOLONOMY`, `BETTI` against the live engine.

- Right panel: gauges (κ, conf, capacity, λ₁).
- Verb cards. Each clicks → POST to engine → render result card.
- Result card formats are fixed per verb (see mockup):
  - SPECTRAL: top 3 eigenvalues + plain-English read.
  - TRANSPORT: 2×2 matrix + θ (rad and deg) + interpretation.
  - HOLONOMY: signed angle + interpretation.
  - BETTI: b₀ / b₁ / b₂ + χ.

**Tests:**

- `tests/unit/verbCard.spectral.test.ts` — given `{l1, l2, l3, n}`,
  renders bars proportionally.
- `tests/unit/verbCard.transport.test.ts` — given θ, renders matrix
  exactly with cos/sin and degree conversion.
- `tests/component/Inspector.select.test.tsx` — selecting a row updates
  all four gauges and the explainer copy.
- `e2e/sheets_verbs.spec.ts` — click TRANSPORT, see a matrix card; the
  GQL footer updated to a valid `TRANSPORT FROM … TO …` query.

**Engine work item:** none. The verbs all exist.

---

### Sprint S4 — Geometry tab (2D fiber scatter)

**Goal:** Second view. Same bundle, plotted in (fiber\_x, fiber\_y).

- Default fiber projection: the first two numeric fields, user-selectable.
- Click point → select row. Anomalies have halos. Selected row drawn
  with the transport line to its nearest healthy peer.
- Sidebar: per-site λ₁ bars, cover stats.

**Tests:**

- `tests/unit/projection.test.ts` — given fiber spec + rows, produces
  the right (x, y) ranges.
- `tests/component/Geometry.test.tsx`
  - 13 rows → 13 circles.
  - selected row has an extra ring.
  - clicking a circle dispatches `select(id)`.
- `e2e/sheets_geometry.spec.ts` — switch to Geometry tab, click a
  point, the GQL footer changes.

---

### Sprint S5 — Schema mutation from the UI

**Goal:** Add row / add field / rename field / drop field, all via UI.

- "+" at end of column list opens a "new field" dialog (name, type,
  encryption mode, indexed). On submit → `ALTER BUNDLE ADD FIELD`.
- Right-click column header → rename / drop / change type.
- "New row" inserts a draft row with placeholder values and opens the
  first numeric field for edit.

**Engine work item:**
- `ALTER BUNDLE` GQL verb if not yet present. Audit current parser
  before scoping. Tests:
  - `parser::alter_bundle_add_field`
  - `parser::alter_bundle_rename_field`
  - `parser::alter_bundle_drop_field`
  - `engine::alter_bundle_replays_through_wal`

**UI tests:**

- `tests/component/AddField.test.tsx` — submit calls client with the
  right GQL string.
- `tests/component/AddField.encrypted.test.tsx` — choosing
  `INDEXED · CMAC` results in `ENCRYPTED INDEXED` in the GQL.
- `e2e/sheets_alter.spec.ts` — add a field, edit a value into it,
  reload, field still there.

---

### Sprint S6 — Saved views

**Goal:** A "view" is filter + sort + group + visible-cols + chosen
projection. URL-shareable. Persisted server-side.

- Sidebar "Saved views" list.
- View serialization: a single JSON object. Stable.
- Round-tripping: clicking a view sets state; mutating state offers
  "Save changes" / "Save as new view".

**Tests:**

- `tests/unit/viewSerde.test.ts` — round-trip property test (fast-check).
- `tests/component/ViewBar.test.tsx` — clicking a view applies it;
  modifying anything shows "Save changes".
- `e2e/sheets_views.spec.ts` — create a view, copy URL, open in a
  fresh browser, same state.

**Backend work item (BFF):**
- `GET/PUT /api/views/:bundle` — minimal, can sit in `sheets/server/`.
- Tests in TS via supertest. No engine change.

---

### Sprint S7 — GQL editor view

**Goal:** First-class GQL editor on the same surface, with the same
client. ⌘↵ runs; results render in-app, not opening a separate REPL.

- Re-use the `playground/` syntax highlighting if portable; otherwise
  ship a Prism profile.
- Format button uses a deterministic GQL formatter.
- Result panel: meta row (rows, κ̄, elapsed, plan) + a small grid.

**Tests:**

- `tests/unit/gqlFormatter.test.ts` — formatter is idempotent;
  fuzzed property test.
- `tests/component/GqlEditor.test.tsx` — ⌘↵ submits.
- `e2e/sheets_gql.spec.ts` — type a query, run, see results.

---

### Sprint S8 — Encrypted fields surface

**Goal:** Encrypt v0.2 is visible in the schema editor + grid. No
plaintext ever appears in the DOM.

- Schema editor: dropdown for `NONE / OPAQUE / INDEXED / AFFINE` per
  field. Disabled in places where the engine forbids it.
- Grid: lock icon column adornment. Cell renders `▒▒▒▒▒` for
  `OPAQUE`; renders a short stable hash for `INDEXED`; renders the
  affine-transformed value for `AFFINE` with a tooltip explaining the
  gauge.
- Equality filter on an `INDEXED` column works; range filter is
  disabled with a tooltip.

**Tests:**

- `tests/component/EncCell.test.tsx` — opaque rows have no
  decryptable text in `el.textContent`.
- `tests/component/SchemaEditor.encryption.test.tsx` — toggling
  modes emits the right GQL.
- `e2e/sheets_encrypted.spec.ts` — turn a column ENCRYPTED INDEXED,
  filter by equality, only the right rows return.

**Engine work item:** none. Encrypt v0.2 already supports these modes.

---

### Sprint S9 — Insights drawer

**Goal:** Auto-generated narrative — "what's interesting in this view
right now?" — sourced from the engine, not the client.

- Engine adds a `WHAT IS INTERESTING` (or `INSIGHTS bundle [view]`)
  verb that returns ranked observations with explanations and a GQL
  trail per observation.
- UI renders them as cards with a tag (`anomaly | watch | geometry`),
  body, and a copy-able GQL footer per card.

**Engine work item:**
- New verb `INSIGHTS` (whatever the final name) — accepts an
  optional filter, returns `Vec<{tag, body, gql, score}>`.
- Tests:
  - `engine::insights_finds_anomaly_concentration_per_site`
  - `engine::insights_finds_highest_kappa_row`
  - `engine::insights_ranks_deterministically_for_fixed_seed`

**UI tests:**

- `tests/component/InsightsDrawer.test.tsx` — drawer opens, renders
  the cards from the response.

---

## 6 · Cross-cutting concerns

### Performance budget

- 50k rows in the grid at 60 fps on a mid-range laptop. Use virtualization
  (`@tanstack/react-virtual` or hand-rolled). Tests should assert that
  ≤ 60 DOM rows exist for a 5k-row dataset.
- Initial route → first paint ≤ 200 ms on a hot SDK cache.
- Cell edit → optimistic update ≤ 16 ms; commit confirmation ≤ 200 ms.

### Observability

- Every grid action emits a DHOOM-event-shaped log line (see
  `GIGI_OBSERVABILITY_SPEC.md`):
  `{verb, bundle, rows, kappa_mean, elapsed_ms, ui_action}`.
- The footer's GQL preview must always be runnable on the engine's
  GQL endpoint. CI test: scrape the preview, post it, expect 200.

### Accessibility

- Keyboard-first navigation. Arrows move selection. ⌘K focuses search.
  Enter starts edit. Esc cancels. Tab cycles inspector verbs.
- Color is never the only signal — every anomaly row also has an icon
  in the status pill.

### Error contract

- Optimistic UI **must** roll back on server rejection and surface the
  engine's error string in a toast. Tests assert this for each kind of
  mutation (cell edit, new row, alter bundle).

---

## 7 · Test coverage map

| Layer | Tooling | Required for green |
|---|---|---|
| Engine (Rust) | `cargo test` | Every engine work item from S1/S5/S9 |
| GQL parser (Rust) | `cargo test --test parser` | S5 |
| Client lib (TS) | `vitest` | Every sprint |
| Components (React) | `vitest + @testing-library/react` | Every sprint with UI |
| End-to-end | `playwright` (extends `e2e/`) | S0, S1, S3, S4, S5, S6, S7, S8 |
| Visual regression | Playwright + image diff | S2 (overlay), S4 (scatter) |
| Property tests | `fast-check` | S6 (view round-trip), S7 (formatter) |

CI gate: PR cannot merge unless every layer for the touched sprint
passes. The single sprint-spec file gets updated with the status of each
sprint, mirroring the existing TDD rhythm in this repo.

---

## 8 · Open questions / call-outs

1. **Schema discovery.** The grid needs to know fiber-field types. Does
   `GET /bundles/:name` already return a structured schema, or do we
   need to add it? (Audit before S0.)
2. **κ-delta on write.** Does `PATCH /bundles/:name/section` currently
   return cohort κ updates? If not, this is the only required engine
   change in S1.
3. **`ALTER BUNDLE` scope.** Some engine modules may freeze schema
   after first insert. Need to confirm what's mutable post-creation.
4. **Insights verb naming.** `INSIGHTS` vs `INTERESTING` vs reusing
   `INTEGRATE` with a `RANK BY κ` clause. Decide in S9 design pass.
5. **Sheets vs `dashboard/`.** Sheets is a separate app, not a tab in
   the operator dashboard. Both apps share `gigi-client.ts`. We may
   move common pieces to a shared `sdk/js-ui/` later.

---

## 9 · Done means

- A non-technical operator can open `sheets.davisgeometric.app/sensors`,
  edit a cell, watch the row turn red, click "Insights" to see why,
  and never have written a query.
- Every interaction has a green test at the right layer.
- The footer GQL preview, when copied into the GQL tab and run, returns
  the exact rows shown above.

The mockup at [`mockups/sheets.html`](mockups/sheets.html) is the visual contract.
Anything that ships must pass a side-by-side diff against the relevant
mockup view at the end of each sprint.
