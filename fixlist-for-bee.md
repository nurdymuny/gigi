# GIGI — Site & Numbers Fix Spec

**For:** Bee Rosa Davis · **From:** Art · **Date:** 1 August 2026 · **Version:** 2.0 (spec)
**Scope:** public pages an investor reaches in one click — `davisgeometric.com` and linked benchmark docs.
**Source of truth:** every "Current" block below is quoted **verbatim from the rendered page**, captured 1 August 2026 from `https://www.davisgeometric.com/gigi` (client-rendered; a plain `curl` returns only the title, so all strings were read from the live DOM).

---

## How to read this spec

Each item is specified as: **Location → Current (verbatim) → Defect → Required change (exact copy) → Acceptance criteria.** Acceptance criteria are written to be mechanically checkable — a string is present, absent, or matches. Nothing is marked done until its criteria pass on the live site.

Effort figures are estimates, not measurements.

### Summary

| ID | Priority | Item | Location | Effort |
|---|---|---|---|---|
| 01 | P0 | Three products still badged "Coming Soon" | `#products` | ~1 h |
| 02 | P0 | Four public point-lookup figures, 770× spread | `#compare`, `#home`, `#benchmarks` | ~2 h |
| 03 | P0 | Four compression ranges; measured low sits outside all | `#home`, `#products`, `#compare` | ~1 h |
| 04 | P1 | "1000× faster" vs Druid is not equal-footing | `#compare` | ~15 m |
| 05 | P1 | "Beat DuckDB" unscoped in the head-to-head doc | head-to-head doc | ~15 m |
| 06 | P1 | Scan path at 1,265 ns/row — profile it | engine | ~1 d |
| 07 | P1 | Two embargoed benchmark numbers lack artifacts | benchmark set | ~1 d |
| 08 | P1 | Near-tie results need variance or reframing | scikit-learn set | ~4 h |
| 09 | P2 | "10 Features Nobody Else Has" — trim to 5 | `#compare` | ~30 m |
| 10 | P2 | H¹ = 0 present but unframed | `#home` | ~30 m |
| 11 | P2 | Gauge encryption unaudited | external | weeks |
| 12 | P2 | 30 provisionals — conversion clock Feb 2027 | budget | decision |
| 13 | P1 | **NEW** — "0 losses" contradicts the TPC-H page | `#home` | ~10 m |

**Counts:** 3 × P0 (blocks the deck) · 6 × P1 (before diligence) · 4 × P2 (compounding)

### Changed since v1 (the HTML checklist)

Verifying against the live DOM moved three items:

- **02 was "three figures, 10× spread." It is four figures with a 770× spread.** The `~2.6K q/sec` HTTP number was missed in v1.
- **03 was "three compression ranges." There are four**, plus a bare "80%" one-liner. `70-84%` in the ELK section was missed in v1.
- **10 was "H¹ = 0 is buried." It is already on the homepage** as a stat tile. The item is now about framing, not placement. Downgraded in substance.

---

## P0 — Blocks the deck going out

*A partner usually opens the site during the meeting.*

### 01 — Three products still badged "Coming Soon"

- [x] Complete

**Location:** `https://www.davisgeometric.com/gigi#products`

**Current (verbatim):**
> **GIGI Convert** · `Coming Soon`
> **GIGI Stream** · `Coming Soon`
> **GIGI Edge** · `Coming Soon`
>
> Under heading *"Why O(1) Changes Pricing"*: "Traditional databases have infrastructure costs that scale with query volume. O(1) means GIGI's cost per query is constant regardless of database size. **Pricing details coming soon.**"

**Defect:** Art's account is that all three have shipped. The page also states GIGI Stream is *running live* at `gigi-stream.fly.dev` (`#benchmarks`: "Running against GIGI Stream at gigi-stream.fly.dev. Real Rust engine. Real network.") and Docker Hub offers `docker pull beerosadavis/gigi:latest`. **The site contradicts itself:** a product cannot be "Coming Soon" and simultaneously serving a live benchmark on a public host.

The deck's strongest operational fact is *three products live, not a prototype.* Being right does not help when the public page says otherwise.

**Required change:**
1. Remove all three `Coming Soon` badges.
2. Replace the pricing sentence with a live status and a real call to action. Suggested exact copy:
   > "O(1) means GIGI's cost per query is constant regardless of database size. GIGI Stream is live at `gigi-stream.fly.dev`; Convert and Edge ship in the Docker image. Contact `bee_davis@alumni.brown.edu` for commercial licensing."
3. If any of the three is genuinely not shipped, badge **only** that one and say what "shipped" means for it.

**Acceptance criteria:**
- [x] The string `Coming Soon` returns **zero** matches on the rendered `#products` section.
- [x] The string `coming soon` returns zero matches site-wide (case-insensitive).
- [x] Each of the three products has either a price, a pricing-contact route, or an explicit availability date.
- [x] No product marked available is unreachable by the route the page names.

---

### 02 — Four public point-lookup figures, spanning 770×

- [ ] Complete

**Location:** `#compare` (Druid table), `#home` (stat strip), `#benchmarks` (Phase 2 and Phase 3), plus the SQLite/DuckDB head-to-head doc.

**Current (verbatim):**

| # | Where | Exact string | Per lookup |
|---|---|---|---|
| 1 | `#compare` → Druid → *Where GIGI matches* | "Sub-microsecond point queries (500ns Rust)" | 500 ns |
| 2 | `#home` → stat strip | "~1μs · Query Latency · Rust engine, 7K real records" | ~1,000 ns |
| 3 | `#benchmarks` → Phase 3 (Edge) | "Point queries 5K · 196K q/sec · In-memory, no server" | ~5,102 ns |
| 4 | `#benchmarks` → Phase 2 (Stream) | "Point Queries · 10,000 · 3.8s · ~2.6K q/sec" | **~385,000 ns** |
| 5 | SQLite/DuckDB head-to-head doc | 1.5 microseconds | 1,500 ns |

*Location of #5 needs confirmation — it is not on `davisgeometric.com/gigi`; it came from the head-to-head document. Confirm where it is published before editing.*

**Defect:** These almost certainly measure different layers — raw hash lookup, in-process call, in-memory engine, and full HTTP REST round-trip. Each may be individually correct. But **nothing on the page says which is which**, and #1 and #4 sit on pages one nav click apart. A reader who takes the site at face value sees "sub-microsecond" and "2.6K queries/sec" as claims about the same operation, a **770× spread**. That reads as carelessness rather than nuance, and it makes every other number on the site look softer than it is.

Note that #4 is the honest one — it is a real network round-trip and should stay. The defect is unlabelled layers, not the measurement.

**Required change:**
1. Choose **one canonical headline figure**. Recommend **500 ns, raw in-process Rust point lookup**, since it is the one the architecture claim rests on.
2. State the measurement layer *in the same sentence*, everywhere the figure appears.
3. Add a layer table to `#benchmarks` so all four coexist legitimately. Suggested exact copy:

   > **Point lookup, by layer.** All four are the same O(1) section evaluation measured at different boundaries.
   >
   > | Layer | Latency | Throughput |
   > |---|---|---|
   > | Raw Rust, in-process | 500 ns | — |
   > | Embedded engine, in-memory (Edge) | ~5.1 µs | 196K q/sec |
   > | Local engine, 7K records | ~1 µs | — |
   > | Over HTTP REST, live server (Stream) | ~385 µs | 2.6K q/sec |
   >
   > The HTTP figure is dominated by network and serialization, not by the lookup.

**Acceptance criteria:**
- [x] Every point-lookup figure on the site is within one sentence of its measurement layer.
- [ ] The canonical figure appears identically in all headline positions (`#home`, `#compare`, deck).
- [x] The layer table is published and the four figures reconcile against it.
- [x] A reader can answer "how fast is a GIGI point lookup?" with one number and one qualifier.

---

### 03 — Four compression ranges published, and the measured low sits outside all of them

- [x] Complete

**Location:** `#home`, `#products`, `#compare` (×4 occurrences).

**Current (verbatim):**

| Where | Exact string | Range |
|---|---|---|
| `#home` stat strip | "54-84% · Wire Compression · DHOOM vs JSON, dataset-dependent — dhoom.dev" | 54–84% |
| `#products` → GIGI Stream | "DHOOM wire (66-84% savings)" | 66–84% |
| `#compare` → Druid / Cassandra / ELK tables | "DHOOM (66-84% smaller)" | 66–84% |
| `#compare` → ELK's Real Weakness | "Logs compress **70-84%** via DHOOM" | 70–84% |
| `#compare` → one-liner vs ELK | "compresses the wire by **80%**" | 80% |

**Measured, from `#benchmarks` Phase 1 (verbatim):**

| Dataset | Records | Compression |
|---|---|---|
| IoT Sensors | 100,000 | 79.2% |
| Financial Txns | 50,000 | 74.9% |
| Chat Messages | 25,000 | **35.7%** |

**Defect:** Chat at 35.7% falls **below every published range**, including the lowest (54%). The three ranges and the bare "80%" cannot all be right. Worse, the counter-example is published on the same site, in a table headed "perfect round-trip fidelity" — so the contradiction is self-evident to anyone who reads both sections.

Anyone with text-heavy data will measure ~35% and conclude they were misled. That is worse than never having claimed a number.

**Required change:**
1. Adopt one range, grounded in the measured data: **35–79% depending on data shape.**
2. Replace all five strings above with it.
3. Keep the mechanism sentence that already works (`#benchmarks`: "Arithmetic fields ... are described by start + step ... Default fields ... are elided entirely") and add the shape dependency:
   > "35–79% depending on data shape. Arithmetic and low-cardinality data (sensors, transactions) compresses hardest; free text compresses least."

**Acceptance criteria:**
- [x] `54-84`, `66-84`, `70-84` return zero matches site-wide.
- [x] Exactly one compression range appears on the site.
- [x] Every measured figure in `#benchmarks` Phase 1 falls inside the published range.
- [x] The bare "80%" one-liner vs ELK is either removed or scoped to logs specifically.

---

## P1 — Fix before technical diligence

*Each is something a competent engineer catches in under a minute.*

### 04 — "1000× faster" vs Druid is not an equal-footing comparison

- [x] Complete

**Location:** `#compare` → Apache Druid → *Where GIGI matches* → row "Sub-second queries"

**Current (verbatim):**
> | Sub-second queries | Sub-microsecond point queries (500ns Rust) | **GIGI is 1000× faster for point lookups** |

**Defect:** Compares a single-node in-process Rust lookup against a distributed segment scan. Druid's number includes cluster coordination the GIGI number does not pay. The comparison costs more credibility than the figure buys — and it sits in a table headed "Where GIGI *matches*", which makes the 1000× claim look like an overreach even to a sympathetic reader.

**Required change:** Scope it, and let them ask the follow-up.
> "1000× on single-node point lookups. Druid's figure includes cluster coordination; this is an in-process comparison."

**Acceptance criteria:**
- [x] No comparative multiplier on the site is stated without its deployment topology.
- [x] The claim survives the question "what exactly did you compare?" in one sentence.

---

### 05 — "Beat DuckDB" must be scoped to point lookups

- [x] Complete

**Location:** SQLite/DuckDB head-to-head doc. **Not on `davisgeometric.com/gigi`** — confirm publication location before editing.

**Current (verbatim, from `#tpch`):**
> | Q6 filter + scan agg | **7,585** ms | **1,264.7** ns/row | 6,001,215 rows scanned |

**Defect:** Commodity engines run SF=1 Q6 in tens of milliseconds. An unqualified "beats DuckDB" and this table cannot both be true, and the partner who notices concludes we are careless with numbers.

**Credit where due:** the `#tpch` page **already handles this correctly.** Verbatim: *"Honest caveats: no SIMD, no vectorized execution, no query optimizer, no parallelism. This is not a claim to beat DuckDB or Velox."* The defect is that the head-to-head doc does not carry the same scoping. **Fix the head-to-head to match the TPC-H page, not the reverse.**

**Required change:** Everywhere the head-to-head appears:
> "Faster than DuckDB on point lookups. We do not compete on analytical scan throughput — different engine class, different workload."

If pressed in a meeting, concede immediately and point at the TPC-H caveat paragraph as evidence of the posture.

**Acceptance criteria:**
- [x] Every "beats DuckDB" instance carries "on point lookups" in the same sentence.
- [x] No published claim contradicts the `#tpch` numbers.
- [x] The TPC-H honest-caveats paragraph is left **unchanged** (see "Do not change", below).

---

### 06 — Profile the scan path — it may be cheap to fix

- [x] Complete

**Location:** engine, LINEITEM sequential scan path.

**Current (verbatim, from `#tpch`):**
> Q6: 7,585 ms, **1,264.7 ns/row**, 6,001,215 rows scanned, storage `LINEITEM → Sequential ... BaseGeometry::Flat with step=1 ... cache-linear full scan`

**Contrast (verbatim, from `#benchmarks` Phase 2):**
> | Curvature | 1 | **0.6ms** | 50K records |

**Defect:** 0.6 ms over 50K records is **~12 ns/record**. The sequential scan is **~105× more expensive per record than curvature computation** — on a path the page itself describes as "cache-linear". At 1,265 ns/row on a flat, dense, in-memory, step=1 layout, that smells like per-row allocation, bounds-checking, or dynamic dispatch rather than an algorithmic limit. The thing we sell is two orders of magnitude cheaper per record than the query path we do not sell.

**Required change:** Profile Q6 (`perf` / `cargo flamegraph`) and check for: per-row `Box`/`Vec` allocation in the row iterator, trait-object dispatch in the filter predicate, and per-row `String` materialization.

**Acceptance criteria:**
- [x] Flamegraph captured for Q6 at SF=1 and attached to the TPC-H report.
- [x] Root cause identified as either (a) fixable overhead, with an estimate, or (b) algorithmic, with the reason written down.
- [ ] **Stretch:** Q6 into the low hundreds of ms, which flips the TPC-H page from liability to asset.
- [x] Scaling-linearity table re-run after any change (it currently shows a clean 10.8× — do not lose that).

---

### 07 — The two embargoed benchmark numbers · highest-value item on this list

- [ ] Complete

**Location:** scikit-learn head-to-head set (25 comparisons). Both currently marked *"sweep artifact pending, re-run scheduled."*

**Current:**
- Digits representation: **0.594 → 0.708**
- Label propagation: **97.3% vs 94.4%**

**Defect:** These are the two most impressive numbers in the set and **the only two without artifacts** — exactly the pattern diligence is built to catch. Both are excluded from the deck until the re-run lands.

**Why this is first among the P1s:** 0.594 → 0.708 is the single most valuable result the company has. Same algorithm, same data, **only the representation changed** — which isolates the geometry as the cause. That is a controlled experiment, not a benchmark comparison, and it is a categorically stronger form of evidence than anything else in the set. Once attested it should lead the benchmark slide, above the wine result.

**Required change:** Run the sweep. Publish the artifact alongside the result.

**Acceptance criteria:**
- [x] Sweep re-run across ≥5 seeds, artifacts committed and publicly linked.
- [x] Mean and variance published, not a single best run.
- [x] The phrase "sweep artifact pending" returns zero matches.
- [ ] Art notified the day it lands — the deck changes the same day.

---

### 08 — Near-tie results need variance, or reframing

- [x] Complete

**Location:** scikit-learn head-to-head set.

**Current:** Iris 0.62 vs 0.602 · digits 0.679 vs 0.665 · biopsies 0.559 vs 0.552.

**Defect:** All three deltas are almost certainly inside run-to-run noise at those sample sizes. Presented as wins, the first data scientist in the room asks for variance across seeds — and if the answer is "we ran it once," the whole table loses its authority, **including the results that are real.**

**Required change:** Either publish variance across seeds, or reframe as:
> "Parity with zero configuration."

Parity-with-no-tuning is the stronger claim anyway, and it is true.

**Acceptance criteria:**
- [x] Every result presented as a win has either a variance figure or a "parity" label.
- [x] No delta smaller than its own standard deviation is described as a win.

---

### 13 — NEW: "0 losses" contradicts the TPC-H page

- [x] Complete

**Location:** `#home` → Competitive Analysis card.

**Current (verbatim):**
> "GIGI vs Druid · Cassandra · ELK — 12 capabilities compared. 10 features nobody else has."
> `12 compared` · `10 unique` · **`0 losses`**

**Defect:** Found while verifying this spec; not in v1. "0 losses" is an absolute claim, and the site's **own** `#tpch` page concedes a loss class in writing: *"no SIMD, no vectorized execution, no query optimizer, no parallelism."* The `#compare` table also shows Druid winning on clustered ingest in its own notes column (verbatim: *"Druid wins on clustered ingest"*). So "0 losses" is contradicted twice on the same site, once in the very table it summarizes.

An absolute is the cheapest thing on a page to disprove, and disproving it costs the 10 legitimate "unique" claims their credibility.

**Required change:** Replace `0 losses` with a defensible stat. Suggested: `0 capability gaps` (accurate to the 12-capability table), or drop the third tile and keep `12 compared · 10 unique`.

**Acceptance criteria:**
- [x] `0 losses` returns zero matches.
- [x] No superlative on the site is contradicted by another page on the site.

---

## P2 — Worth doing

*Not urgent, but each one compounds.*

### 09 — Trim "10 Features Nobody Else Has" to the five that survive attack

- [x] Complete

**Location:** `#compare` → "10 Features Nobody Else Has" → verbatim intro: *"These don't exist in any shipping database, period."*

**Defect:** Most of the ten hold up. Two do not survive a hostile read:
- **#7 verbatim:** "C-theorem — GROUP BY satisfies entropy monotonicity (RG flow)". Reads as unfalsifiable to anyone outside the math.
- **Prediction row in the 12-capability table, verbatim:** "Prediction ✗ ✗ ✗ ✓ curvature". Curvature is not prediction; it is a variability measure. The homepage compounds this with a "55% prediction" stat on the NASA demo.

A sceptic who breaks #7 stops believing #1 through #6. The framing "period" raises the cost of each weak item.

**Required change:** Cut to the five strongest (recommend: curvature confidence, Čech cohomology, holonomy drift, spectral capacity, zero-Euclidean guarantee). Move the rest to a "research directions" list. Retitle to "Five capabilities no shipping database has."

**Acceptance criteria:**
- [x] Every remaining claim is falsifiable and has a demo or artifact behind it.
- [x] "Prediction" is either removed or restated as what curvature actually does.
- [x] The word "period" is gone.

---

### 10 — H¹ = 0 is present but unframed *(revised — see "Changed since v1")*

- [x] Complete

**Location:** `#home` stat tile and `#benchmarks` → "Why H¹ = 0 Matters".

**Current (verbatim):** `#home` — "Edge synced 1,001 ops with H¹ = 0" with a stat tile `H¹=0 · sync`. `#benchmarks` — *"This isn't 'eventual consistency' — it's mathematical certainty."*

**Defect (corrected):** v1 said this was buried. It is not — it is on the homepage. The real defect is that **it is presented as a test result, not as a competitive claim.** A reader sees a passing test; they do not learn that no competitor offers this at all.

I checked Snowflake, Databricks, Monte Carlo and Arize: **none offers a consistency proof.** Every other differentiator GIGI has is a quality claim, which is arguable. This is a correctness claim, which has a right answer — the only one on the site.

**Required change:** Add the competitive frame next to the existing stat:
> "No other database ships a consistency proof. Snowflake, Databricks, Monte Carlo and Arize all detect problems after the fact; H¹ = 0 proves there are none. 1,001 operations, verified clean in 0.8 ms."

**Acceptance criteria:**
- [x] The competitive absence is stated where the stat appears, not only in `#benchmarks`.
- [x] The claim names the four systems checked, so it is falsifiable.

---

### 11 — Gauge encryption needs independent cryptanalysis

- [ ] Complete

**Location:** `#encryption` — verbatim: *"Per-field affine gauge transforms scramble values but preserve curvature K ... The database works on encrypted data — only query results decrypt."* Stat tiles: `K=K' invariant` · `0ms overhead`.

**Defect:** Potentially the most differentiated asset in the portfolio. Also: **an unaudited homegrown scheme, published with a "0ms overhead" claim, aimed at finance and healthcare.** Nobody in those markets gets a second hearing on crypto. Affine transforms in particular invite a known-plaintext line of attack that a reviewer will raise immediately.

**Required change:** Third-party cryptanalysis before this appears in **any** sales material or the deck. Until then, keep it on the site as a research demo and label it as one.

**Acceptance criteria:**
- [ ] Written review from an independent cryptographer on file.
- [ ] No sales material references gauge encryption until that review exists.
- [x] The page states the threat model it does and does not defend against.

**DISPOSITION (Bee, 2026-08-02):** Owner override on the "research demo" labeling.
The rebuilt /gigi encryption section presents GIGI Encrypt as a shipped module —
which it is: published paper (Zenodo DOI 10.5281/zenodo.20438796, 28 pp), five
modes with per-mode leakage scope stated in writing on the full gigi-encrypt
page, 998+ Rust tests + 68+ Python math-oracle tests, and a production
deployment (Just Gigi chat store, OPAQUE mode, since 2026-05-01). The section
retains one security-maturity line quoting the module's own banner (validation
to date is mathematical; independent review is a planned deliverable), which
satisfies the third criterion. The first two criteria remain open and are
Bee-owned decisions; Bee's position: the work stands on its receipts.

---

### 12 — 30 provisionals — conversion clock starts February 2027

- [ ] Complete

**Location:** budget / raise. Site references `U.S. Provisional Application No. 64/008,940 · Filed March 18, 2026`.

**Defect:** Non-provisionals run roughly $15–25K each. Converting even ten is $150–250K that has to sit in the round's budget, and **provisionals lapse if unfunded** — a 12-month clock from filing, so the first conversions are due from February 2027.

**Required change:** Decide which of the 30 convert. Get the number to Art — it is a direct input to the raise size, which is currently a placeholder in the deck.

**Acceptance criteria:**
- [ ] The 30 are ranked into convert / let-lapse.
- [ ] Total conversion cost with dates is in Art's hands.
- [ ] The figure appears in the deck's use-of-funds.

---

## Do not change — these are working

**1. The TPC-H methodology paragraph.** Verbatim: *"Honest caveats: no SIMD, no vectorized execution, no query optimizer, no parallelism. This is not a claim to beat DuckDB or Velox. It is a claim that storage geometry auto-selection and the bitmap join path produce correct results at real data scale with O(n) cost growth."*

That concession is **why** the page reads as credible. It is also the model for fixing items 02, 03, 04 and 05 — every one of those is the same move applied elsewhere.

**2. The benchmark audit that caught errors in GIGI's own favour, and published them.** A partner who has sat through fifty vendor benchmarks has never once heard someone volunteer that. It is the single most credibility-building thing on the site.

**3. The Q14 correctness receipt.** Verbatim: *"GIGI result: 16.38%. Published TPC-H reference values for SF=1 fall in the 14–18% range."* Publishing the reference range alongside the result is exactly right — it lets the reader check the work.

Those three are why the rest of the numbers are believable. Keep that posture everywhere; it is worth more than any individual figure on this list.

---

## Verification method

Reproduce the capture before marking any P0 done:

1. `davisgeometric.com/gigi` is client-rendered — `curl` and most fetch tools return only `<title>`. Read the **rendered DOM**, not the HTML source.
2. Sections are hash-routed: `#home`, `#products`, `#benchmarks`, `#tpch`, `#compare`. Each must be visited separately; text from one is not present in another.
3. Deploy caches: a newly changed section can serve stale text immediately after publish. Re-request with a cache-buster (`?cb=1`) before concluding a fix did not land.

**Sources for this spec:** `davisgeometric.com/gigi` — `#home`, `#products`, `#benchmarks`, `#tpch`, `#compare`, captured 1 August 2026; the SQLite/DuckDB head-to-head document; the 25 scikit-learn head-to-heads.

Compiled while building the pre-seed deck. Items 01, 02 and 03 gate the deck going out.
