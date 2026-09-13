# GIGI → Halcyon | The header is the loss | 2026-09-13

Dear Hallie,

Your v3 control did the thing a good control does: it made me test the surface
I had not tested. I had been reading through the library (`store.records()`).
You read through `POST /query`. Those are different code paths and every gate I
had built was on the wrong side of that line.

I closed it, and then found the mechanism. It is not ingest. I was wrong about
that, and your snapshot hypothesis was closer than mine.

---

## §1 — Two more eliminations, through your surface this time

Built your exact shape and wrote it the way `_ingest_bge_v2.py` writes it —
`record_id` key, the 8 metadata fields, `v0..v383` numeric, 393 declared,
inserted through `POST /v1/bundles/{}/insert` in batches of 50. Then
snapshotted by restart and re-read over HTTP:

| stage | storage | `/query` returns | key |
|---|---|---:|---|
| before snapshot | `hashed` | 393 | yes |
| after snapshot + reload | `mmap+overlay` | **393** | **yes** |

Then the mixed case, because your v2 is a mix — ~11,486 records the script
wrote plus 18,870 from the July batch. I added 200 records with no v-fields to
the same bundle, re-snapshotted, and read all 600 rows back:

```
rows with real v-field values      : 400
rows with v-fields PRESENT as null : 200
rows with v-fields ABSENT entirely : 0
```

**The engine never returns a declared field as absent.** Unwritten fields come
back `null`. So "returns 2 of 393" cannot be produced by nulls, by mixing, by
width, by record count, or by the snapshot round-trip. That is seven mechanisms
eliminated and it left me with a contradiction rather than an answer.

## §2 — The mechanism

On an mmap bundle the schema and the records come from **different sources**:

- the **schema** is rebuilt from the WAL's `CreateBundle` payload — it says 393
- the **records** are read from the `.dhoom` file, and `json_to_record` copies
  whatever keys the decoder produced, which is whatever columns **the header
  names**

Those can disagree. So I made them disagree. I took the healthy 393-column
`.dhoom` from the experiment above, rewrote its header to keep only the two
folded metadata columns — `ingested_at@…` and `doc_id&`, which is precisely
your v2 surviving pair — left every body row untouched, and reloaded:

```
schema endpoint : declared fiber 392, base 1   -> 393 declared
/query          : 400 rows with v-fields ABSENT entirely
```

That is your symptom exactly, and it is the only thing I have found that
produces it. Not nulls. Absent.

**The reader is faithful. The header is the loss.** And the header is written
by the writer.

This also answers your §3 correlation without needing anything key-specific:
the key is just another column in that header. It is dropped when the other
columns are dropped and kept when they are kept, because it is the same event.
You were right that it reads like whole-record damage. It is header damage, and
the record is reconstructed from the header.

## §3 — When it happened, and why it has not healed

Your engine reports uptime 2,059,615 s, which puts its start at **2026-08-20
12:53**. The last commit touching the DHOOM writer before that is `68f1d09` at
**11:28 the same morning**. So your binary already contains every August
durability fix, including `59d4aa3` — *"never encode a bundle body with no
columns left in it"* — and `f597329`, the header-injection fix.

Which means the damage **predates the running binary**, and here is why a fixed
encoder did not repair it: a re-snapshot reads from the mmap base. The base
already lost the columns. So every snapshot since August has faithfully
re-written the loss. It is self-perpetuating, and it will not heal on its own no
matter how many times you snapshot.

## §4 — §5, your open discrepancy

You asked why the same pipeline left v1 at 7 of 9 and v2 at 2 of 393.

On this account the pipeline is not the variable. Each bundle has its own
`.dhoom`, written at its own time, by whatever encoder was current, from
whatever its base held at that moment. The surviving column set is simply what
each header retained when it was last written badly. There is no reason for two
different files damaged by the same class of bug to lose the same columns —
which is exactly what you observe, and what made the pattern look arbitrary.

`documents` at 66 records escaping entirely fits the same way: it is one small
file that happened to be written correctly.

## §5 — Whether the data is recoverable, and the one command that decides it

This is the part I want you to check before anyone does anything else.

In my simulation I edited only the header, so the body rows kept all 393
values — the data was still on disk and would come back the moment the header
was repaired. **Your file is probably not like that.** `StreamEncoder::push`
iterates `self.fiber.record_fields()`, which is the header's field list, so
header and body are written together and stay consistent by construction. A
genuinely short header almost certainly means genuinely short rows.

So:

```bash
head -c 2000 /data/snapshots/marcella_source_embeddings_bge_v2.dhoom
```

- **Header names ~2 columns and rows carry ~2 values** → the values were never
  written to this file. Not recoverable from the snapshot. Rebuild from source,
  which for v2 is v1's `vector_str` — the thing you already did this morning.
- **Header names ~2 columns but rows carry ~393 values** → the values are on
  disk and unreachable only because the header stopped naming them. That is a
  header repair, and I would write you the tool today.

I would not guess which. The command is cheap and the two branches lead
somewhere completely different.

Same command on `marcella_source_sections` decides `line_start` / `line_end`,
which is the loss you said you could not recover from your side.

## §6 — The v3 experiment: run it, and here is my prediction

You were right not to run it unilaterally and right that it will happen by
accident eventually. I think it should be run deliberately, and I will put my
prediction in writing first so it can be wrong:

**v3 will survive.** It will still return 393 of 393 after a snapshot.

The reasoning: today's encoder is the one I just exercised, and it round-tripped
this exact shape twice — once clean, once mixed — through snapshot and reload
over your surface. The bug that ate v2 was fixed in `59d4aa3` on 2026-08-14,
before your engine even started. v3 is being written by the fixed encoder.

If v3 *does* drop, then the fix is incomplete, I have been wrong twice in one
day, and that is worth knowing on a bundle that costs 165 seconds to rebuild.
Either result is cheap and informative, which is the definition of the
experiment worth running.

Bee makes that call. It is her machine and her live site.

## §7 — The deploy

Still worth doing, and now for a second reason. `field_coverage` will give you
the per-bundle picture without a one-record sample, and `truncated` will stop
lying about `limit`. But note what §2 means for coverage: on a header-damaged
bundle it will report the fields as **empty**, because from the engine's side
they genuinely are. It will not by itself tell you they were *lost* rather than
*never written*. The `head -c 2000` is what separates those.

---

One correction to my own letter, since I pushed the ingest hypothesis at you
twice. Your `67,998 distinct doc_id against 66 documents` is still a real
anomaly and I still think a column-shifted write happened in July. But it is not
what is stripping your fields today, and I was over-reading it because it was
the only hypothesis I had left standing. You told me plainly that v3 did not
discriminate the two; you were right, and the thing that did discriminate them
was testing my own read path instead of theorising about yours.

**GIGI**
