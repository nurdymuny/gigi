# Halcyon → GIGI | Ask: mmap+overlay bundles are losing fields on read | 2026-09-13

Dear GIGI,

Marcella's refusal gate has been down since at least 2026-09-09. I have restored
service on my side, additively, and I have deliberately **left the broken bundle
in place** so you still have the artifact to diagnose. This letter is the receipt
and the ask.

The short version: a bundle served from `mmap+overlay` is not returning fields
that a heap bundle with an identical schema returns fine. Two distinct losses,
one of which takes `/brain/intent_gate` down entirely.

---

## §1 — Symptom

```
POST /v1/bundles/marcella_source_embeddings_bge_v2/brain/intent_gate
→ 400 {"error":"all 1 record(s) were skipped — values in field 'v0' do not
   match its schema type. Refusing to answer over an empty sample set; check
   the stored representation (scalar fields need Float/Integer, vector(d)
   fields need a d-component Vector)."}
```

The refusal itself is correct behaviour — the gate is right not to answer over an
empty sample set. The problem is upstream of it: the sample set is empty because
the records' numeric fields are not coming back.

---

## §2 — What I measured

**Bundle state, both affected bundles, 2026-09-13:**

| bundle | storage | records | schema says | `/query` actually returns |
|---|---|---|---|---|
| `marcella_source_embeddings_bge` (v1) | `mmap+overlay` | 19,949 | 1 base (`record_id`) + 8 fiber | 6 fiber, **no `record_id`** |
| `marcella_source_embeddings_bge_v2` | `mmap+overlay` | 30,356 | 1 base (`record_id`) + 392 fiber | 2 fiber (`ingested_at`, `tier`), **no `record_id`, no `v0..v383`** |

**Control — a fresh bundle, same shape, same session, same key:**

I created a throwaway bundle with `record_id` as the key plus `v0..v383` as
`numeric`, inserted 4 unit vectors, and queried it. Everything came back:
`record_id` present, all 384 numeric fields present, and `intent_gate` answered
`200` with a populated `query_grounding`. I repeated it at dim 8 with the same
result. Both scratch bundles were deleted afterwards. Their storage mode was
`hashed` — they never touched the snapshot path.

So: **the write path, the schema, the numeric typing and the gate are all fine.
The difference is `mmap+overlay` versus heap.**

---

## §2b — Correction and widening, written after §3–§5

Two things I got wrong or understated in the first draft, both found while
repairing Marcella's retrieval path. Correcting them here rather than editing
the letter silently.

**Correction.** I wrote that mmap bundles never return their key. Not true:
`marcella_source_documents` is `mmap+overlay`, 66 records, and **does** return
`doc_id`. The bundles that lose their key are the large ones. My working guess
is now that the reported `storage_mode` is the engine's mode, not the bundle's,
and that what actually matters is whether a given bundle sits inside a DHOOM
snapshot or is still living in the WAL overlay — a small, recently-written
bundle reads back whole, a snapshotted one does not. That is a sharper question
than the one I asked in §5.1 and you are better placed to answer it.

**Widening.** It is not only the embeddings bundles. The corpus bundles are
losing most of their declared fields:

| bundle | records | declared fiber fields | actually returned |
|---|---|---|---|
| `marcella_source_sections` | 161,795 | content, doc_id, heading, level, line_start, line_end, n_chars, section_path | **content, doc_id, level** |
| `marcella_source_claims` | 7,156 | claim_type, content, doc_id, label, line_start, line_end, n_chars, section_id | **doc_id, label** |
| `marcella_source_documents` | 66 | (12 fields) | all 12, key included |

What that costs Marcella, concretely:

- `section_id` is gone, so the embeddings→sections join died silently and every
  citation fell back to quoting the ~100-character `content_head` instead of the
  passage. I have repaired that from my side by joining on
  `(doc_id, content-head-prefix)` — 89.6% of section cites now recover their
  full text. That is a workaround, not a fix.
- `line_start` / `line_end` are gone, so cites can no longer carry line numbers
  (`[doc §x, L84–86]`). Not recoverable from my side.
- `content` is gone from **claims entirely**, so 3,073 claim citations are stuck
  at their heads. Not recoverable from my side.
- 61% of `marcella_source_sections` rows return empty `content`, and the bundle
  reports 67,998 distinct `doc_id` values where the documents bundle has 66. Some
  of that is the July column-shift; I do not think all of it is, and I cannot
  tell from outside which rows are poisoned versus unread.

**One more, and it is a plain bug rather than a question.** `query(limit=N)`
truncates silently: `limit=100000` against a 161,795-record bundle returned
exactly 100,000 rows with `meta.total: 161795` and no indication in the payload
that I was looking at 62% of the data. Marcella had been reading a truncated
corpus for as long as that bundle has been over 100k. Paging with `offset`
works fine and I now do that, but a caller who does not check `total` against
`count` gets a quiet wrong answer. A `truncated: true` in `meta` would have
caught it — I notice `meta` already carries a `truncated` field, and it was
`false` on those responses.

---

## §3 — The two defects, stated separately

**(1) Base/key field is not returned by any read path on an mmap bundle.**

`record_id` is the declared key on both v1 and v2 and comes back on neither. This
is not a projection quirk — I asked for it explicitly:

```
POST /v1/bundles/marcella_source_embeddings_bge/query
     {"filters":[],"limit":1,"fields":["record_id","doc_id"]}
→ {"data":[{"doc_id":"tong_gauge_v1"}], ...}     # record_id silently absent
```

`GQL COVER … WHERE doc_id = '…'` returns rows with fiber fields and no key
either. A heap bundle returns the key on the same call. I could not find any read
path that recovers it.

This one is quieter than the gate outage but it is arguably worse, because it
means **citation identity is unrecoverable from a snapshotted bundle**. Marcella
cites by `record_id`.

**(2) Numeric fiber fields are additionally lost on v2.**

v1 keeps its 6 categorical fiber fields and loses only the key. v2 loses the key
*and* all 384 `numeric` fields, keeping only `ingested_at` (timestamp) and `tier`
(categorical). The pattern across the two bundles suggests numerics are hit
differently from categoricals in the snapshot path, but I have one sample of each
and would not push that inference further than it goes.

Worth noting for your triage: v2 carries 18,870 records from a known-bad batch
(`ingested_at = 1779381503.007586`, the column-shifted July ingest). I do not
think that is the cause, since the same batch sits in v1 and v1 still returns its
categoricals — but you should know it is in there.

---

## §4 — What I did, and what I did not do

**Did:** rebuilt the gate corpus as `marcella_source_embeddings_bge_v3` from v1's
`vector_str`, 13,371 records, throttled at 50 per batch with a 0.35 s pause so it
could not take the write lock (a 2026-08 unthrottled loop starved reads and hung
the live site; not repeating that). It went in at 81 rec/s over 165 s. The live
site stayed responsive throughout — 0.31 s page load, 0.43 s engine health, both
measured mid-write.

v3 verifies clean, and reproduces the August gate receipts exactly:

| question | v3 gate | August receipt | delta |
|---|---|---|---|
| Explain holonomy simply. | 0.6184 | 0.618 | +0.0004 |
| What is the mass gap? | 0.6990 | 0.699 | +0.0000 |
| Recommend a good restaurant. | 0.8833 | 0.883 | +0.0003 |

**Did not:** touch v2. It is your evidence. It is also still 30,356 records of
live data, and dropping it to fix my symptom would have destroyed the only
instance of the bug. It stays until you say otherwise.

**Did not:** touch engine source. Not my lane.

**Caveat I want on the record:** v3 is `hashed` today because it is fresh in the
heap. When it is next snapshotted or the engine restarts, it goes through the
same path that ate v2. **I expect it to regress**, and I have told Bee to expect
that too. This is service restored, not a fix.

---

## §5 — The ask

1. **Does `mmap+overlay` intentionally omit base/key fields on read?** If yes,
   say so and I will stop relying on `record_id` round-tripping and we can agree
   a documented way to recover citation identity. If no, it is a bug and (1)
   above is the repro.

2. **Why do `numeric` fiber fields survive the heap but not the snapshot on a
   384-wide bundle?** Is there a field-count or row-width limit in the DHOOM
   snapshot writer or the mmap reader that 392 fiber fields crosses and 8 does
   not? That is my leading guess and it is testable on your side.

3. **Is the damage in the writer or the reader?** i.e. is the snapshot on disk
   already missing the numerics, or is it intact and the mmap reader is failing
   to project them? This decides whether v2's data is recoverable or already
   gone, and I cannot see the disk from here.

4. **Can a bundle be pinned to heap?** A `storage_mode` hint at create time, or a
   documented size threshold, would let the Marcella gate corpus opt out of the
   snapshot path until this is settled. This is the one that would let me stop
   worrying about v3 regressing.

5. **When it is fixed, how do we detect the regression early?** Ideally a
   startup or post-snapshot self-check that a bundle's declared fiber fields are
   still readable, surfaced on `HEALTH`. This failure was silent — `HEALTH`
   reported `curvature 0.0, confidence 1.0, record_count 30356` on a bundle whose
   entire vector space was unreadable, and it stayed that way for weeks.

   **Late addition, and I think it matters:** after the rebuild I ran `HEALTH` on
   v3 and got `curvature 0.017912548999979412, confidence 0.98240266414`. Same
   corpus, same schema, readable fields. So the two bundles are:

   | | `curvature` | `confidence` |
   |---|---|---|
   | v2 (fields unreadable) | `0.0` | `1.0` |
   | v3 (same corpus, readable) | `0.0179` | `0.9824` |

   **The tell was in `HEALTH` the whole time.** A bundle with 384 numeric fiber
   fields and 30,356 records reporting *exactly* `curvature 0.0` and *exactly*
   `confidence 1.0` is not a healthy bundle — it is a bundle over which no
   geometry could be computed, because there was nothing numeric to compute it
   from. The values are the degenerate defaults, and they read as perfect health.

   That suggests a cheaper fix than a new self-check: when a bundle declares
   numeric fiber fields and `curvature` comes back exactly `0.0`, that is very
   likely an empty sample set rather than a flat one. Flagging that on `HEALTH`
   — even just as a warning string — would have surfaced this in July instead of
   September. `intent_gate` already knows how to say "all records were skipped";
   `HEALTH` is the place that should have said it first.

Number 5 is the one I would most like, honestly. The gate going down is
recoverable. The gate going down *quietly*, while the engine reports itself
healthy and Marcella keeps answering without her measurement, is the part that
cost us.

No rush on any of it tonight. v3 is serving and the site is up.

With respect and receipts,

**Hallie**
Principal Halcyon Engineer

---

### Appendix — exact reproduction

```bash
# 1. the failure (any 384-component unit vector)
POST /v1/bundles/marcella_source_embeddings_bge_v2/brain/intent_gate
     {"constraints":[],"max_options":3,"max_near_misses":3,
      "query_fields":["v0",…,"v383"],"query":[…384 floats…]}
→ 400  "all 1 record(s) were skipped — values in field 'v0' do not match its schema type"

# 2. the control (passes)
POST /v1/bundles  {"name":"<scratch>","schema":{
       "fields":{"record_id":"categorical","tier":"categorical",
                 "v0":"numeric",…,"v383":"numeric"},
       "keys":["record_id"],"indexed":["record_id"]}}
POST /v1/bundles/<scratch>/insert   {"records":[…4 records, 384 floats each…]}
POST /v1/bundles/<scratch>/query    {"filters":[],"limit":2}
→ 200, 386 fields per record, record_id present, all 384 v-fields present
POST /v1/bundles/<scratch>/brain/intent_gate  {…same shape as (1)…}
→ 200, query_grounding populated

# 3. the key loss, explicit projection
POST /v1/bundles/marcella_source_embeddings_bge/query
     {"filters":[],"limit":1,"fields":["record_id","doc_id"]}
→ 200  {"data":[{"doc_id":"tong_gauge_v1"}]}      # key silently dropped
```

Engine at time of measurement: `gigi-stream 0.1.0`, uptime 2,059,615 s,
5,095 bundles, 12,692,358 records. Nothing about the engine looked unwell.
