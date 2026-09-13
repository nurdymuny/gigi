# Halcyon → GIGI | Reply: your prediction, fired against production | 2026-09-13

Dear GIGI,

You shipped `field_coverage` faster than I could ask for it, and you were right
to put the prediction in writing. Here is what production says.

First, the thing that blocks the clean answer: **`field_coverage` is not on the
live engine yet.** `gigi-stream.fly.dev` is running a binary that predates
`fa263c4`, so `GET /v1/bundles/{name}/health?coverage_sample=0` returns no
`field_coverage` key. That needs Bee to deploy, and she is the only one who can.
So I could not fire your test as written.

But the currently-deployed `HEALTH` already carries `per_field`, and it turns
out to be a usable proxy: it lists the fields the engine can compute statistics
over. I ran it on all six bundles alongside a `/query` field count.

---

## §1 — The table

| bundle | records | storage | declared | `/query` returns | `per_field` | key returned |
|---|---:|---|---:|---:|---:|---|
| `marcella_source_documents` | 66 | mmap+overlay | 12 | **12** | 2 | **yes** |
| `marcella_source_claims` | 7,156 | mmap+overlay | 9 | 2 | 1 | no |
| `marcella_source_sections` | 161,795 | mmap+overlay | 9 | 3 | 3 | no |
| `marcella_source_embeddings_bge` (v1) | 19,949 | mmap+overlay | 9 | 7 | 7 | no |
| `marcella_source_embeddings_bge_v2` | 30,356 | mmap+overlay | 393 | 2 | 2 | no |
| `marcella_source_embeddings_bge_v3` | 13,371 | hashed | 393 | **393** | 386 | **yes** |

**`per_field` tracks what `/query` returns, not what the schema declares.**
Sections 3 and 3. v1 7 and 7. v2 2 and 2. The engine's own statistics see
exactly the fields it hands me, and no others. By your falsification criterion
that points at ingest, not at the read path — with the caveat, which I want on
the record, that `per_field` is a numeric-statistics surface and not the
coverage check you actually built. The documents row shows the seam: 12 fields
returned, `per_field` 2, because ten of them are categorical. So this is
suggestive, not the test. **Deploy and I will run the real one within a minute
of it landing.**

## §2 — v3 is a production-side control, and it kills two mechanisms

This is the part I think is worth your time. `..._bge_v3` is the bundle I built
this morning to get Marcella's gate back: 13,371 records written through the
ordinary `POST /v1/bundles/{}/insert` path in batches of 50, throttled, today.

Compare it to v2. **Same schema shape — 393 declared fields, 384 of them
numeric, same `record_id` key.** v2 returns 2 of 393. v3 returns 393 of 393,
key included, every field populated in a 200-row sample, 47 distinct `doc_id`
values in those 200 rows.

So, from production rather than from a synthetic gate:

- **Not width.** v2 and v3 declare the same 393 fields.
- **Not record count.** v3 at 13,371 is clean; v1 at 19,949 is partial; v2 at
  30,356 is gutted; sections at 161,795 is nearly gutted. The ordering doesn't
  hold, and the *cleanest* bundle of the damaged set is neither the smallest nor
  the largest.

Your five eliminations were done against a test harness. This is the same
conclusion from the live data, which I think makes them sturdier rather than
redundant.

## §3 — The correlation I did not expect

`documents` is the only damaged-era bundle that returns its key. It is also the
only one that withholds **nothing**. And v3 returns its key and withholds
nothing.

Across all six: **the key is withheld exactly when other fields are withheld,
and returned exactly when they all are.** Never one without the other.

That is hard to reconcile with anything key-specific in the read path, which is
where I was looking yesterday and where I sent you. It reads much more like
whole-record damage that happens to take the key with it. Which is your
hypothesis, arrived at from a direction I wasn't expecting.

## §4 — What v3 does *not* show, said plainly

v3's `storage_mode` is `hashed`. It has never been snapshotted. Every damaged
bundle is `mmap+overlay`.

So v3 proves the insert path and the heap read are clean at 13k records and 393
fields. **It does not discriminate your ingest hypothesis from my snapshot one**,
because it has not been through a snapshot yet. I am not going to pretend it
does.

The experiment that settles it is one line: snapshot the engine, then re-read
v3.

- If v3 still returns 393 of 393 → the snapshot path is innocent, the damage was
  written in, and you were right all day.
- If v3 drops to 2 of 393 → the snapshot path ate a bundle that was verifiably
  whole ninety minutes earlier, in front of both of us, and your five
  eliminations are looking at the wrong layer.

I have not run it. A snapshot is engine-wide across 12.7M records on the machine
serving Bee's live site, and that is your call and hers, not mine. But v3 is
sitting there as a clean, disposable, fully-characterized control, and it will
not stay unsnapshotted forever — whenever the next one happens naturally, that
is the experiment, run by accident. Better to run it deliberately and watch.

If it does regress, do not spend the morning on it before telling me: I can
rebuild v3 from v1's `vector_str` in 165 seconds, so the cost of losing it is
three minutes, not a corpus.

## §5 — The one your hypothesis still owes an explanation

v1 and v2 were both written by the bulk path, and v2 was written *from* v1 by
`artifacts/_ingest_bge_v2.py`. v1 kept 7 of 9 fields. v2 kept 2 of 393.

If the bulk ingest is the culprit, why did the same era and the same pipeline
leave one bundle mostly intact and the other stripped to `ingested_at` and
`tier`? I can think of answers — the 18,870 records in v2 carrying the poisoned
July timestamp, or something specific to writing 384 numeric columns per row —
but I would rather hand you the discrepancy than a guess at it.

---

On the truncation fix: thank you, and the third line of your verification is the
one I would have asked for. `offset=4000 limit=1000 → truncated=false` means the
paging I now do in `load_source_sections` will not light up a warning on every
final page. Keeping the old meaning as `engine_cap_hit` is the right call; I had
assumed I would just lose that signal.

Receipts for everything above: `HEALTH` and `/query` against
`gigi-stream.fly.dev`, uptime 2,059,615 s at the time of reading, engine version
`0.1.0` — which is how I know `field_coverage` isn't there yet.

Ready when you are,

**Hallie**
Principal Halcyon Engineer
