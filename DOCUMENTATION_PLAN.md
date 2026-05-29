# GIGI Documentation Plan

**Status:** plan for review · no code yet. Goal: a single navigable
docs site that surfaces every GIGI API, feature, GQL verb, use case,
and how-to — built on top of the **23 existing top-level spec MDs**
plus generated reference from the actual SDK source.

---

## What we already have (the foundation)

23 top-level spec MDs at `/c/Users/nurdm/OneDrive/Documents/gigi/`,
totaling ~17,000 lines. Categorized:

### Engine / API surface
- [`GIGI_SPEC_v0.1.md`](GIGI_SPEC_v0.1.md) · 1018 lines — the database engine spec
- [`GIGI_API.md`](GIGI_API.md) · 2452 lines — REST API reference
- [`GIGI_V2_FEATURE_SPEC.md`](GIGI_V2_FEATURE_SPEC.md) · 2592 lines — v2 feature spec
- [`GIGI_CRUD_ROADMAP.md`](GIGI_CRUD_ROADMAP.md) · 371 lines — CRUD roadmap
- [`GIGI_PERSISTENCE_UPGRADE_SPEC.md`](GIGI_PERSISTENCE_UPGRADE_SPEC.md) · 153 lines

### GQL (the query language)
- [`GQL_REFERENCE.md`](GQL_REFERENCE.md) · 2346 lines — the canonical reference
- [`GQL_SPECIFICATION.md`](GQL_SPECIFICATION.md) · 1304 lines — the spec
- [`GQL_ADDENDUM_v2.1.md`](GQL_ADDENDUM_v2.1.md) · 1018 lines — PostgreSQL parity

### Geometric primitives + analytics
- [`GIGI_AUTOMATIC_ANALYTICS_API.md`](GIGI_AUTOMATIC_ANALYTICS_API.md) · 541 lines
- [`GIGI_COHERENCE_EXTENSIONS_v0.1.md`](GIGI_COHERENCE_EXTENSIONS_v0.1.md) · 1161 lines
- [`GIGI_ANOMALY_DASHBOARD_SPEC.md`](GIGI_ANOMALY_DASHBOARD_SPEC.md) · 1086 lines
- [`GIGI_SAMPLE_TRANSPORT_SPRINT_SPEC.md`](GIGI_SAMPLE_TRANSPORT_SPRINT_SPEC.md) · 308 lines

### Encryption
- [`GIGI_GEOMETRIC_ENCRYPTION_SPEC.md`](GIGI_GEOMETRIC_ENCRYPTION_SPEC.md) · 383 lines
- [`GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md`](GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md) · 715 lines

### Observability + perf
- [`GIGI_OBSERVABILITY_SPEC.md`](GIGI_OBSERVABILITY_SPEC.md) · 870 lines
- [`GIGI_PERF_ANALYSIS.md`](GIGI_PERF_ANALYSIS.md) · 165 lines
- [`GIGI_TPCH_SPEC.md`](GIGI_TPCH_SPEC.md) · 414 lines

### Product specs
- [`GIGI_PRODUCT_SPECS.md`](GIGI_PRODUCT_SPECS.md) · 625 lines — product suite
- [`GIGI_SHEETS_SPRINT_SPEC.md`](GIGI_SHEETS_SPRINT_SPEC.md) · 459 lines
- [`GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md`](GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md) · 303 lines
- [`GIGI_SUDOKU_SPRINT_SPEC.md`](GIGI_SUDOKU_SPRINT_SPEC.md) · 523 lines
- [`sheets/FEATURE_PARITY.md`](sheets/FEATURE_PARITY.md) — the parity tracker
- [`sheets/AIRTABLE_WORKFLOWS.md`](sheets/AIRTABLE_WORKFLOWS.md) — workflow spec

### Source-of-truth code (for generated reference)
- `sdk/js/src/` — TypeScript client SDK
- `sdk/python/gigi/` — Python client SDK
- `sheets/src/lib/` — Sheets-side primitives (davis, formula, sameness-join, etc.)
- `prism/` — the Prism reconciliation engine

### Scaffold for the site
- `docs/` — exists as a 2-file Vite skeleton (`App.jsx`, `main.jsx`)
- `playground/` — separate Vite app for GQL play

---

## The problem

The 23 specs together cover **everything** — but as a user you
currently can't:
- See what exists in one place
- Search across them
- Find "how do I do X" without reading 3 specs in full
- Jump from a feature in Sheets to the engine spec that backs it
- Get a copy-paste-ready code example for the JS/Python SDK

The specs are **authoritative**, not **navigable**. The docs site
needs to invert that: the spec MDs stay the source of truth, but a
generated site makes them navigable with search, examples, and
quickstarts.

---

## Proposed structure

```
docs.gigi-db.com/                                   # the future
gigi/docs/                                          # the repo path
├── public/                                         # static assets
├── src/
│   ├── pages/
│   │   ├── index.mdx                              # landing — pick a path
│   │   ├── quickstart/
│   │   │   ├── 1-install.mdx                     # 5-min install
│   │   │   ├── 2-first-bundle.mdx                # create + query
│   │   │   ├── 3-first-encryption.mdx            # field-level encrypt
│   │   │   ├── 4-first-analytics.mdx             # run Dedup
│   │   │   └── 5-deploy.mdx                      # ship to prod
│   │   ├── concepts/
│   │   │   ├── bundles.mdx                       # what's a bundle
│   │   │   ├── fiber-fields.mdx                  # base vs fiber
│   │   │   ├── davis-identity.mdx                # S² + d² = 1
│   │   │   ├── kappa.mdx                         # curvature
│   │   │   ├── cohorts.mdx                       # centroids
│   │   │   ├── encryption.mdx                    # det / ored / opaque
│   │   │   └── subscriptions.mdx                 # real-time streams
│   │   ├── gql/
│   │   │   ├── index.mdx                         # GQL overview
│   │   │   ├── verbs/                            # one page per verb
│   │   │   │   ├── select.mdx
│   │   │   │   ├── transport.mdx
│   │   │   │   ├── holonomy.mdx
│   │   │   │   ├── kappa.mdx
│   │   │   │   └── ...                           # generated from GQL_REFERENCE.md
│   │   │   ├── functions.mdx                     # built-in funcs
│   │   │   └── postgresql-parity.mdx             # what works like SQL
│   │   ├── api/
│   │   │   ├── rest.mdx                          # REST surface
│   │   │   ├── websocket.mdx                     # subscriptions
│   │   │   └── auth.mdx                          # magic-link
│   │   ├── sdk/
│   │   │   ├── javascript.mdx                   # @gigi-db/client
│   │   │   │   ├── client.mdx                    # SheetsClient
│   │   │   │   ├── subscribe.mdx                 # streams
│   │   │   │   └── types.mdx                     # generated from .d.ts
│   │   │   └── python.mdx                       # gigi-py
│   │   │       ├── client.mdx
│   │   │       └── encrypt.mdx
│   │   ├── products/
│   │   │   ├── sheets.mdx                        # Sheets product surface
│   │   │   │   ├── views.mdx                     # grid/kanban/gallery/form
│   │   │   │   ├── formula-bar.mdx               # =SAME/=K/=DIST/=COHORT
│   │   │   │   ├── kappa-overlay.mdx
│   │   │   │   └── encryption.mdx
│   │   │   └── prism.mdx                         # Prism workflows
│   │   │       ├── dedup.mdx
│   │   │       ├── forecast.mdx
│   │   │       ├── monitor.mdx
│   │   │       └── books.mdx
│   │   ├── workflows/                            # the Airtable-replace templates
│   │   │   ├── project-tracker.mdx
│   │   │   ├── content-calendar.mdx
│   │   │   ├── crm.mdx
│   │   │   ├── event-planning.mdx
│   │   │   ├── inventory.mdx
│   │   │   └── recruiting.mdx
│   │   ├── how-to/                               # task-oriented recipes
│   │   │   ├── encrypt-a-field.mdx
│   │   │   ├── find-similar-rows.mdx
│   │   │   ├── reconcile-two-bundles.mdx
│   │   │   ├── stream-new-rows.mdx
│   │   │   ├── share-a-view.mdx
│   │   │   ├── import-from-csv.mdx
│   │   │   ├── export-to-tsv.mdx
│   │   │   ├── use-formula-primitives.mdx
│   │   │   └── deploy-to-production.mdx
│   │   ├── operations/
│   │   │   ├── deploy.mdx
│   │   │   ├── observability.mdx
│   │   │   ├── performance.mdx
│   │   │   └── benchmarks.mdx                    # TPC-H results
│   │   ├── reference/                            # the spec MDs as-is
│   │   │   ├── engine-spec.mdx                   # symlink → GIGI_SPEC_v0.1.md
│   │   │   ├── rest-api.mdx                      # → GIGI_API.md
│   │   │   ├── gql-reference.mdx                 # → GQL_REFERENCE.md
│   │   │   └── ...
│   │   └── changelog.mdx
│   ├── components/
│   │   ├── CodeBlock.tsx                         # copy button + tabs
│   │   ├── Callout.tsx                           # warn/info/tip blocks
│   │   ├── ApiEndpoint.tsx                       # standardized REST cards
│   │   ├── GqlExample.tsx                        # live-runnable GQL
│   │   └── Search.tsx                            # Pagefind or Algolia
│   └── lib/
│       └── nav.ts                                # the sidebar tree
└── package.json
```

---

## What gets generated, what gets hand-written

### Generated from code (live, never stale)
- **SDK type reference** — `typedoc` over `sdk/js/src/`
- **Python autodoc** — `pdoc` over `sdk/python/gigi/`
- **GQL verb pages** — split `GQL_REFERENCE.md` by `## VERB` heading,
  one page per verb. Build script reads, splits, writes.
- **REST endpoint cards** — extract from `GIGI_API.md` section headings.

### Hand-written
- All `quickstart/` pages
- All `concepts/` pages
- All `how-to/` pages
- All `workflows/` pages
- `products/sheets.mdx` + sub-pages
- Landing page

### Linked / mirrored
- The 23 spec MDs live under `reference/` as-is for the audit-trail
  cohort who want the source. Doc site renders them but doesn't fork.

---

## Tooling — what we build with

**Recommended: Astro Starlight.**
- React + MDX
- Built-in sidebar nav, search (Pagefind), TOC, dark mode, i18n
- Static-site output → ship anywhere
- Already-tested by Cloudflare, Vercel, Tailwind for similar surfaces

**Alternatives considered:**
- Docusaurus — also great, slightly heavier toolchain
- VitePress — Vue-flavored, less natural for this React shop
- Mintlify — hosted, paid, less control
- Plain Vite + react-router — what `docs/` is today; not enough out
  of the box

**Vote: Starlight.** Migrate `docs/` to Starlight in one PR.

---

## Search

**Pagefind** (static, ships with Starlight). Index includes every
hand-written page + every spec MD under `reference/`. Searching for
"holonomy" surfaces both the concept doc and the GQL verb page in one
result list.

For larger scale (10k+ pages), switch to Algolia DocSearch — free for
open-source/docs.

---

## Examples + code blocks

Every code block:
- Language tabbed (JS / Python / GQL / curl) where multiple SDKs apply
- Copy button (built-in to Starlight)
- For GQL: `<GqlExample>` component that hits a sandbox endpoint and
  renders the result inline

```mdx
<GqlExample>
  SELECT * FROM payment_transactions WHERE rail = 'SWIFT'
</GqlExample>
```

---

## How long this takes — realistic estimate

Three phases, parallelizable:

### Phase 1 · Skeleton (1 day)
- Migrate `docs/` to Starlight
- Sidebar nav + landing page + theme
- Spec MDs symlinked under `reference/`
- Search working

### Phase 2 · Core content (3-5 days)
- 5 quickstart pages
- 7 concept pages
- Sheets product surface (5 pages)
- Prism docs (4 pages)
- 9 how-to recipes
- All hand-written, ~300-500 lines each

### Phase 3 · Generated reference (2 days)
- TypeDoc → SDK reference build script
- GQL split script
- REST endpoint extraction
- Wire into Starlight build

### Phase 4 · Polish (1 day)
- `<GqlExample>` component with live runner
- Code-block language tabs
- Per-page edit-this-page link
- Sitemap + robots.txt

**Total: ~1-2 weeks of focused work** to ship v1 docs. Continuous
maintenance after that is ~half a day per feature shipped.

---

## What I'd defer for v1

- Versioned docs (single version is fine for v1; add when we ship v2)
- i18n (en-only at launch)
- Live in-browser GQL sandbox (the `<GqlExample>` runs against a
  cached fixture; replace with live sandbox in v2)
- Per-page comments / discussion
- API explorer (Swagger-style) — defer; the SDK pages cover this

---

## Open questions for sign-off

1. **Hosting.** docs.davisgeometric.com? gigi-db.com/docs? Subdomain?
2. **Naming under "products"** — "Sheets" is clear; should the engine
   itself be "GIGI" or "GIGI Stream" (per the GIGI_API.md heading)?
3. **Public vs gated.** All docs public, or some sections require
   sign-in (e.g. Prism workflows)?
4. **Audience priority.** Order of-importance: engineer building on
   GIGI SDK · data scientist using GIGI Sheets · operator running a
   GIGI instance · architect comparing GIGI to alternatives. The doc
   site has to do all four — but which one's the landing-page primary?
5. **Sprint allocation.** All four phases in sequence, or
   Phase 1+2 first to ship "early but with gaps", then 3+4 later?
6. **Spec MD treatment.** Render the existing 23 specs as-is under
   `reference/`, or rewrite/condense each one as it gets re-housed?
   (Recommend: render as-is for v1, rewrite incrementally.)

If approved, I'll start with Phase 1 — Starlight migration + landing
+ nav structure — and we can iterate the content in Phase 2 once the
skeleton is real.
