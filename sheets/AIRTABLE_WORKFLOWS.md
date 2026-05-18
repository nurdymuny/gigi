# Airtable Workflows in GIGI Sheets — Spec

**Status:** spec for review · no code yet. Goal: identify the canonical
Airtable use cases, then design "workflows" in GIGI that match them and
add at least one differentiator each.

A **workflow** in this spec is a *named template*: a bundle schema +
suggested views (kanban / calendar / form) + a Prism workflow wired to
the right column, packaged so a user can pick it from a list and have
a usable workspace in one click.

---

## What is a "classic Airtable workflow"?

Airtable's homepage organizes its templates into ~10 buckets. The
recurring six that *every* SaaS-vs-Airtable comparison cites:

| # | Workflow | What Airtable ships | Why people use it |
|---|---|---|---|
| 1 | **Project tracker** | Tasks table with status / owner / due-date / priority + Kanban by status | Replaces Jira for small teams |
| 2 | **Content calendar** | Posts with publish-date / channel / status + Calendar view | Replaces Trello+spreadsheet for marketing |
| 3 | **CRM** | Contacts + Companies + Deals (linked records) + Pipeline kanban by stage | Replaces Salesforce-lite |
| 4 | **Event planning** | Attendees + Tasks + Vendors (linked), forms for RSVPs | Wedding/conference workhorse |
| 5 | **Inventory management** | Items + Suppliers + Orders (linked), stock-level filter | Replaces SKU spreadsheet |
| 6 | **Recruiting pipeline** | Candidates with stage + skills tags + interview notes + form intake | Replaces ATS for early-stage teams |

The pattern across all six:
- **Single bundle** with categorical "stage/status" column → **Kanban**
- **A date column** → **Calendar**
- **A form** for new records → **Form view**
- **Linked records** between bundles → **sameness-join**
- **Filtering by status** → **per-view filter state**

We already have Kanban, Form view, sameness-join, and per-view state.
**A "workflow" in GIGI = a pre-baked bundle template that wires all of
these together for a specific use case.**

---

## GIGI's edge — what each workflow gets that Airtable can't do

Every workflow inherits the GIGI substrate, so it ships with these for
free:

| Capability | Airtable | GIGI workflow |
|---|---|---|
| Anomaly detection | bolt-on app | κ overlay on every row, always |
| Real-time sync | poll every 15s | streaming subscription |
| Field encryption | only at-rest | per-column det/ored/opaque, queryable while encrypted |
| Audit trail | Enterprise-only event log | every row signed, every plan verifiable |
| ML on the data | needs Zapier + external service | Prism Dedup / Forecast / Monitor inline |
| Cross-table join | exact FK match | sameness-join (survives typos/drift) |
| Formula bar | full Excel-style | subset + `=SAME`, `=K`, `=DIST`, `=COHORT` |

---

## The 6 workflows · proposed design

For each one: **what it is**, **GIGI schema**, **suggested views**,
**Prism wire-up**, **the GIGI-better moment**, and the **shippable
demo bundle**.

### 1 · Project tracker

**Bundle schema** (`workflow:projects`)
- `task_id` (pk text)
- `title` (text)
- `assignee` (categorical)
- `status` (categorical: backlog / in-progress / review / done)
- `priority` (categorical: P0 / P1 / P2 / P3)
- `due_date` (timestamp)
- `created_at` (timestamp)
- `estimate_hrs` (numeric)
- `actual_hrs` (numeric)
- `tags` (text, multi-select)

**Views**
- **Kanban** grouped by `status` (default)
- **Calendar** grouped by `due_date`
- **Form** for new task intake
- **Grid** for full edit

**Prism wire-up**
- **Monitor** flags stalled tasks (high κ on `actual_hrs / estimate_hrs` ratio)
- **Dedup** catches duplicate ticket-filings via canonical `title` match

**GIGI-better moment.** "Tasks stuck in review for 3+ days" surfaces as
κ-drift on the lifecycle vector — no need to write a Zap. Same data, no
extra tooling.

**Demo bundle.** `demo_projects` — 40 tasks across a fake sprint, with
2 planted duplicates and 3 stalled tasks for Monitor to catch.

---

### 2 · Content calendar

**Bundle schema** (`workflow:content_calendar`)
- `post_id` (pk text)
- `title` (text)
- `channel` (categorical: blog / twitter / linkedin / newsletter)
- `author` (categorical)
- `publish_date` (timestamp)
- `status` (categorical: draft / scheduled / published / archived)
- `word_count` (numeric)
- `target_audience` (text)
- `topic_tags` (text)

**Views**
- **Calendar** by `publish_date` (default)
- **Kanban** by `status`
- **Form** for content briefs
- **Gallery** for visual content (post thumbnails)

**Prism wire-up**
- **Forecast** on `posts/week` to project publishing cadence
- **Dedup** on near-duplicate `title` strings (canonical match)

**GIGI-better moment.** Calendar view tints each day by mean κ of the
posts on it — if a Tuesday has 3 unusually long posts queued, it lights
up yellow before publication.

**Demo bundle.** `demo_content_calendar` — 50 posts across 3 months,
showing publishing cadence + 2 near-duplicate titles for Dedup.

---

### 3 · CRM (contacts + deals)

**Bundle schema** (`workflow:crm_contacts` + `workflow:crm_deals`)

`crm_contacts`:
- `contact_id` (pk)
- `name` (text)
- `email` (text)
- `company` (categorical)
- `role` (text)
- `last_contacted` (timestamp)

`crm_deals`:
- `deal_id` (pk)
- `contact_id` (text, links to crm_contacts)
- `stage` (categorical: lead / qualified / proposal / closed-won / closed-lost)
- `value_usd` (numeric)
- `probability_pct` (numeric)
- `expected_close` (timestamp)

**Views**
- **Kanban** on deals by `stage` (default)
- **Calendar** on `expected_close`
- **Form** for new lead intake
- **Grid** for contact list

**Prism wire-up**
- **Books** to reconcile deals across two sources (CRM ↔ accounting)
- **Monitor** flags stale deals (no movement in 14+ days → κ-drift)

**GIGI-better moment.** Sameness-join `crm_deals.contact_id` ↔
`crm_contacts.contact_id` handles email-typo'd contacts that an exact-FK
CRM would orphan.

**Demo bundle.** `demo_crm_contacts` (30 contacts) + `demo_crm_deals`
(25 deals) — paired bundles with 3 deliberate contact-id typos.

---

### 4 · Event planning

**Bundle schema** (`workflow:event_attendees`)
- `attendee_id` (pk)
- `name` (text)
- `email` (text, opaque-encrypted)
- `rsvp` (categorical: pending / yes / no / maybe)
- `dietary` (text, multi-select)
- `arrival_date` (timestamp)
- `table_assignment` (text)
- `plus_one` (boolean)

**Views**
- **Form** for public RSVP intake (default landing)
- **Kanban** by `rsvp`
- **Grid** for seating chart

**Prism wire-up**
- **Dedup** on canonical `email` match for double-RSVPs

**GIGI-better moment.** Field-level encryption: `email` is OPAQUE, so
event organizers can query by name without ever seeing raw addresses
in the grid view. Privacy by default.

**Demo bundle.** `demo_event_rsvps` — 40 attendees, encrypted email,
3 RSVP duplicates from different email formats.

---

### 5 · Inventory management

**Bundle schema** (`workflow:inventory`)
- `sku` (pk text)
- `product_name` (text)
- `category` (categorical)
- `supplier_id` (text, links to suppliers)
- `quantity_on_hand` (numeric)
- `reorder_threshold` (numeric)
- `unit_cost_usd` (numeric)
- `last_restocked` (timestamp)

**Views**
- **Grid** with filter for `quantity_on_hand < reorder_threshold`
- **Kanban** by `category`
- **Form** for stock-take entries

**Prism wire-up**
- **Forecast** on `quantity_on_hand` per SKU
- **Monitor** flags SKUs with unusual stock velocity (κ-anomaly)

**GIGI-better moment.** Forecast tells you *which SKUs will hit
reorder-threshold by day 7* before they do, with the √step
confidence band — no plug-in needed.

**Demo bundle.** `demo_inventory` — 50 SKUs across 4 categories, 5 at or
near reorder threshold, 2 with unusual velocity for Monitor.

---

### 6 · Recruiting pipeline

**Already shipped as `job_applicants` demo.** This is the existing
40-row ATS-shaped dataset. Wiring the workflow surface adds:

**Views**
- **Kanban** by `stage` (Applied → Phone Screen → Onsite → Offer → Hired/Rejected)
- **Form** for new applicant intake
- **Grid** for full review
- **Gallery** for resume cards (when we add attachment field)

**Prism wire-up**
- **Monitor** flags high-score rejected candidates (the planted A-31 anomaly)
- **Dedup** on canonical `(name, email)` for duplicate applications

**GIGI-better moment.** "Find candidates like A-32" (a hired one) is one
formula: `=SAME(A_32, applicant_i) ≥ 0.85`. Airtable can't express this
without a custom script.

---

## "Workflow" picker — the UI

Add a new entry point: **`/workflows`** route + a **"Start with a workflow"**
panel on the landing page and in the bundle picker. Pre-baked
templates the user can one-click instantiate.

```
┌── Start with a workflow ────────────────────────────────────────┐
│                                                                 │
│  📋 Project tracker      📅 Content calendar     💼 CRM         │
│  🎉 Event planning       📦 Inventory            👥 Recruiting   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

Clicking a card:
1. Creates the bundle(s) on the engine with the workflow's schema
2. Loads the seed CSV (small demo data)
3. Applies the default view (kanban / calendar / form depending on workflow)
4. Drops the user straight into the bundle with the workflow tab live

Underneath, a workflow is `{ schema, seed_csv, default_view, prism_wireup }`
in a new `lib/workflow-templates.ts` file. Each one is ~60-100 lines.

---

## Implementation plan (if approved)

| Step | Effort | Deliverable |
|---|---|---|
| 1 · Define `WorkflowTemplate` type + 6 templates | ~half day | `lib/workflow-templates.ts` |
| 2 · Workflow picker component | ~half day | `<WorkflowPicker>` panel for landing + bundle picker |
| 3 · Apply-workflow handler (create bundle + seed + nav) | ~half day | Existing demo-loader + view-apply, plumbed together |
| 4 · Default-view selection per workflow | ~quarter day | Extend `ViewSpec` consumption to honor `defaultView` from workflow |
| 5 · Tests | ~quarter day | Component test per workflow card + smoke test for apply |
| 6 · Doc each workflow on landing page | ~quarter day | Add a "Workflows" section above demos |

**Total: ~2 days, mostly assembly.** No new algorithm work; every
primitive (kanban, form, calendar, sameness-join, Dedup, Monitor) is
already shipped.

---

## Open questions for sign-off

1. **Naming.** "Workflow" vs "Template" vs "Starter" — Airtable uses
   "template," ClickUp uses "template," Notion uses "template." Should
   we say **template** instead?
2. **CRM = two-bundle workflow.** All others are single-bundle. Worth
   the extra UI complexity in v1, or push two-bundle workflows to v2?
3. **Seed data realism.** Should seed data be obviously fake names
   (Acme, Globex) or look like a real company's data (with proper
   anonymization)?
4. **Workflow vs Prism.** The Prism workflows (Dedup, Forecast,
   Monitor, Books) already use the word "workflow." Risk of confusion?
   → If we go with "template" above, this is moot.

If approved, I'll start with the **Project tracker** template (single
bundle, all four views, simplest to validate the pattern) and
generalize from there.
