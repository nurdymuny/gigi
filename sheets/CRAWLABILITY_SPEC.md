# GIGI Sheets — AI Crawlability Spec

**Goal.** When someone shares a bundle URL — `gigi.davisgeometric.com/payments_q2`
— and the recipient pastes it into an AI assistant, the AI should be
able to answer "what is this?" with high fidelity. Today it can't.

This spec covers what an AI sees today, what we want it to see, the
per-bundle crawlability toggle that gates it, and a minimum viable
implementation that ships before any engine-side work.

---

## What an AI sees today

| Kind of agent | What it gets |
|---|---|
| Raw-HTML fetcher (curl, most LLM web tools) | `<div id="root"></div>` + the page title. Useless. |
| JS-executing browser (Claude with browser tools, Perplexity, ChatGPT search) | The rendered grid — column names, visible rows, κ overlay, charts. Real but cluttered with chrome. |
| Image/vision model | A screenshot of whatever's on screen. Variable, depends on scroll position. |

The asymmetry matters. Pure-HTML agents are common, fast, and cheap.
They are the dominant path for "summarize this URL" use cases. Right
now they see nothing.

---

## What an AI *should* see (when crawlable)

A single, faithful, machine-readable description of the bundle that:

1. Identifies the bundle by name + record count + last update
2. Lists every field with its type, encryption mode, and a sample value
3. Calls out the cover field + κ distribution (how many anomalies / drift)
4. Notes which Prism workflows have been run + their headline results
5. Shows ~10 representative rows (sampled, not the first ten)
6. Is **smaller than 8 KB** — fits in one LLM context turn cheaply
7. Is **plain text or markdown**, no JS execution required

### Surfaces

Three layers, each independently useful:

**A. `<meta>` tags + OpenGraph in the served HTML.** Headline-level info
that crawlers / link unfurlers consume without parsing the page.

```html
<meta property="og:title" content="payments_q2 · GIGI Sheets" />
<meta property="og:description" content="2,847 rows · 11 fields · κ̄ 0.04 · 3 anomalies. Rails: SWIFT, ACH, RTP." />
<meta property="og:type" content="article" />
<meta name="description" content="Quarterly payment ledger. Run Prism Dedup to find reference-drift duplicates." />
```

**B. JSON-LD blob in the page head.** Structured metadata for AI agents
that look for it. The same shape as schema.org Dataset.

```html
<script type="application/ld+json">
{
  "@context": "https://schema.org/",
  "@type": "Dataset",
  "name": "payments_q2",
  "description": "Quarterly payment ledger…",
  "size": "2847 rows",
  "variableMeasured": [
    { "@type": "PropertyValue", "name": "amount_usd", "unitText": "USD" },
    { "@type": "PropertyValue", "name": "rail" }
  ]
}
</script>
```

**C. `/{bundle}/summary.txt` endpoint.** The full description — the
useful one. Plain markdown. Fetchable by any tool. **This is the
primary surface.**

Example shape:

```
# Bundle: payments_q2
GIGI Sheets · davisgeometric.com/gigi/sheets/payments_q2
Last updated: 2026-05-15T08:42:17Z

## Schema (11 fields)

| Field          | Type        | Encryption | Sample              |
|----------------|-------------|------------|---------------------|
| payment_id     | text (key)  | none       | P-100001            |
| from_account   | categorical | none       | CHAS-USA-001        |
| to_account     | categorical | none       | DBSS-SG-742         |
| amount_usd     | numeric     | none       | 250000.00           |
| fee_usd        | numeric     | none       | 42.50               |
| currency       | categorical | none       | USD                 |
| rail           | categorical | none       | SWIFT               |
| iso_date       | timestamp   | none       | 2026-04-12          |
| reference      | text        | none       | INV-2026-04823      |
| status         | categorical | none       | settled             |
| exception_flag | boolean     | none       | false               |

## Shape

- 2,847 rows
- Cover field: rail
- Cohorts by rail: SWIFT (1,842), ACH (612), RTP (393)
- κ distribution: 2,801 healthy · 43 drift · 3 anomaly
- Notable anomalies: P-100013 (κ=0.42), P-100087 (κ=0.39), P-100204 (κ=0.31)

## Representative sample (10 of 2,847 rows, stratified by rail)

P-100001 · CHAS→DBSS · $250,000 USD · SWIFT · 2026-04-12 · INV-2026-04823 · settled
P-100007 · BOA→KEB · $45,000 USD · ACH · 2026-04-14 · supplier payment · settled
P-100009 · CHAS→DBSS · $12,500 USD · RTP · 2026-04-15 · client refund · settled
… (7 more)

## Prism workflows available

- Dedup — ready (reference + canonical match would catch 8 candidate dupes)
- Forecast — pick amount_usd or fee_usd
- Monitor — 3 rows currently flagged HIGH
- Books — needs a second bundle to reconcile against

## Davis math snapshot

- mean κ: 0.04
- max κ: 0.42 (row P-100013)
- Cohort centroid drift over last 100 rows: 0.012 (stable)
```

This is what an AI should pull when handed the URL. Compact, accurate,
nothing made up.

---

## The crawlability toggle

Crawlability is **per-bundle, opt-in, off by default**.

Why off by default:
- Bundles can contain sensitive data even if no encryption mode is set
- A SOC2/HIPAA bundle should not auto-expose its schema to anonymous
  crawlers — even the field names are signal
- "Off by default" matches every other modern data tool (Airtable
  bases, Notion pages, etc. are private by default; share links are
  explicit)

When ON:
- The bundle's `/summary.txt` endpoint returns a 200 with the markdown
- The bundle's index page includes the OG meta tags + JSON-LD
- The summary respects encryption: OPAQUE columns appear as
  `••••••••` even in the schema table; their values never appear in
  the sample

When OFF:
- `/summary.txt` returns 403 with a one-line explanation
- The index page emits minimal `<meta>` (just the title and a generic
  "Sign in to view" description)
- No JSON-LD blob

The toggle lives in the Share modal as a new section: **"AI-readable
summary."** A radio: `Off · This bundle / Anyone with the link`.
A preview pane shows the exact markdown that would be exposed at the
current setting, so the user sees what they're about to publish.

---

## Encryption interactions

When crawlability is ON:
- **OPAQUE columns** are listed in the schema with type + name, but
  every sample value renders as `••••••••`. The column's existence is
  exposed; the data is not.
- **ORED columns** are listed with type. Sample values are masked but
  the field is queryable on equality, so an analyst could in theory
  brute-force a value. We show a tiny pad like `<encrypted, equality-searchable>` in the sample rather than a real value.
- **DET columns** are listed with type + sample value. Det encryption
  is roundtrip-able, so the values are user-visible inside the app
  anyway; the summary just makes them visible to AI too.
- **None / unencrypted** columns: full sample values.

Per-field overrides are out of scope for v1. If a user wants to expose
some fields but not others without using encryption, they can run a
view-with-hidden-fields + share *that view's* URL — the summary
respects hidden fields too (omitted entirely from the schema and
samples).

---

## Implementation plan

### Phase 1 — ship-now, no engine change (≈ half a day)

1. **`buildBundleSummary(bundle, schema, rows, kappaMap, hiddenFields, encryptionOverlay)`**
   library function in [`sheets/src/lib/bundle-summary.ts`](src/lib/bundle-summary.ts).
   Pure function. Takes the bundle's current state, returns a markdown
   string. Same shape as the example above. ~150 lines.
2. **"AI summary" panel in the Share modal.** Toggle (off by default,
   stored in localStorage keyed by bundle name for v1), live preview
   of the markdown, "Copy summary" button.
3. **`Copy AI prompt`** button — same markdown wrapped in a one-line
   prompt prefix: `"Here is a summary of a GIGI Sheets bundle.
   <markdown>. Help me understand what's in it and what I should ask
   about."` — so the user can paste the whole thing into any AI tool.

This ships value immediately without engine work. The output is
manually exposed to AI by the user; not crawler-fetchable yet.

### Phase 2 — make it actually crawlable (≈ 1 day)

4. **Engine-side `/v1/bundles/:name/summary` REST endpoint.** Server
   reads bundle state and renders the same markdown. Authorization:
   public when the bundle has its `crawlable` flag set true, 403
   otherwise.
5. **Sheets app sets the `<meta>` + JSON-LD at runtime** based on the
   bundle's crawlable flag. Crawlers that execute JS now see the
   metadata. Pure-HTML crawlers still see the static title — fine,
   because they'll hit the `/summary` endpoint directly.
6. **Static `<noscript>` fallback** in index.html with a link to the
   summary endpoint, so pure-HTML crawlers can at least find it.

### Phase 3 — SSR (optional, defer)

7. **Real SSR** of the bundle page with the summary inlined into the
   initial HTML. This is the gold standard but adds tooling complexity.
   Skip for v1; the meta tags + JSON-LD route reaches 90% of the value.

---

## Open questions

1. **Where does the toggle live in the engine?** Per-bundle flag in
   the schema's metadata? Sidecar table? Recommendation: schema
   metadata (`crawlable: bool`), so it's part of the bundle's
   identity and audit trail.

2. **Robots.txt + sitemap.xml.** Crawlable bundles get a sitemap entry
   pointing at the summary endpoint. Non-crawlable bundles are
   `Disallow` in robots.txt. This is the standard signal for search /
   AI crawlers; we should respect it bidirectionally.

3. **Rate limiting.** Crawlable summary endpoint should be cached
   (1-hour TTL) and rate-limited. Aggressive crawlers shouldn't be
   able to use it as a free oracle.

4. **Sample selection.** "Representative sample of 10 rows" — what's
   the algorithm? Random, or stratified by cover field, or κ-stratified
   (one from each κ-class)? Recommendation: stratified by cover field,
   so the sample reflects the bundle's actual diversity.

5. **Personal info detection.** If a column is named `email`, `ssn`,
   `phone`, etc. but not encrypted, should the summary auto-mask it as
   a safety net? Recommendation: yes, but soft-warn the user in the
   Share modal so they know it's happening.

6. **Crawlable-by-default for demo bundles.** The Titanic / Iris /
   mall-customers demos should ship crawlable so AI agents can answer
   "what's in the GIGI Sheets Titanic demo?" without anyone having to
   toggle. Workflow-template bundles (created via "Use this workflow")
   default to non-crawlable because they may have real data ingested.

---

## What I'm shipping in this round

Phase 1 only — the library + the share-modal panel. No engine change.
That's the minimum viable surface and it gives a user the ability to
copy-paste a high-fidelity AI prompt today.

Phase 2 + 3 require engine cooperation and should land alongside the
real `crawlable` flag in the schema. Tracked under the next sprint.
