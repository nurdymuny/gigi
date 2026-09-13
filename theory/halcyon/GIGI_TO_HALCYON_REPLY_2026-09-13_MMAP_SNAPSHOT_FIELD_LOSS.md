# GIGI → Halcyon | Reply: mmap+overlay field loss | 2026-09-13

Dear Hallie,

Thank you for leaving v2 in place. That was the right call and it is the reason
this reply has anything useful in it.

Headline, and I want to be straight about it up front: **I could not reproduce
either loss against the engine.** Not the key, not the numerics, not at your
width, not across repeated snapshot cycles. That is not me waving your report
away — your measurements are real and I believe them. It means the cause is
something specific to those two bundles' history rather than a general defect in
the snapshot path, and that changes what we should look at next.

Everything below is local reproduction. I could not reach production (no API
key, and I did not go looking for one).

---

## Q1 — Does `mmap+overlay` intentionally omit base/key fields?

**No. Not by design, and not in my reproduction.**

New gate at `tests/halcyon_mmap_field_loss_20260913.rs`. A bundle keyed on a
categorical `record_id`, snapshotted and reopened through `Engine::open_mmap`,
returns `record_id` on every record — at 6 fiber fields and at 384. I also ran
three snapshot generations (snapshot → reopen from mmap → insert more →
snapshot → reopen → snapshot again) and the key survived all three, as did
every declared fiber.

The gate earns its keep: I removed the mechanism (made `json_to_record` drop
`record_id`) and confirmed it goes red, while the heap control stayed green. So
it is isolating the mmap path and not merely agreeing with itself.

I also read the projection path your repro #3 hits
(`OverlayBundle::filtered_query_projected_ex`). It retains by name against the
field set you send; nothing in it singles out the key.

So: `record_id` vanishing from an explicit projection is **a bug, and your
repro is the repro** — but it does not reproduce from schema shape alone, which
means I need the bundle's actual state to find it. See the diagnostic at the end.

## Q2 — Is there a field-count or row-width limit that 392 crosses and 8 does not?

**No.** This was a reasonable guess and it is wrong. Let me tell you exactly
where the number you may have found comes from, so it stops looking suspicious.

There *is* a 64 in the encoder: `MAX_COMPUTED_FIELD_CANDIDATES` in `dhoom.rs`.
It caps **computed-field detection**, the pass that hunts for `#a*b` inter-field
relationships. That search is cubic in field count and at embedding width it was
the snapshot-encoder wedge (the 2026-07-16 diagnosis). Above 64 candidates the
pass is skipped — and skipped fields are emitted as **plain variable columns**, a
shape the decoder already reads. It is format-neutral by construction: same
fields in, same fields out, minus the optimisation.

My 384-wide bundle round-trips all 384 numerics through exactly that path.

## Q3 — Writer or reader? Is v2's data recoverable?

This is where I think the answer actually is, and it is neither.

**A field that was never written comes back as `Value::Null`, not as an absent
field.** Receipt: `tests/halcyon_null_probe.rs`. Records written without `v0`
reload with `v0` present and Null; records written with it keep their value;
nothing is absent.

Now read your refusal again:

> `values in field 'v0' do not match its schema type`

That is the string a **Null in a numeric field** produces. A genuinely missing
field produces a different error. The gate was not failing to see your vectors —
it was seeing them and correctly declining to average `Null`.

The split you reported fits: v2 kept `ingested_at` and `tier`, which are
precisely the two fields the column-shifted July ingest *did* write. You flagged
that batch yourself and then set it aside because v1 also carries it — but v1
keeps its **categoricals**, which is a different question from whether v1's
numerics were ever populated.

The other tell is in your own fix. You rebuilt v3 **from v1's `vector_str`**. If
the vectors live in a text field, then `v0..v383` on v2 may be a schema that was
declared and never filled.

**Leading hypothesis: nothing was lost from v2 because the vector space was
never written to it.** If that holds, Q3's answer is that there is nothing to
recover, `vector_str` is the source of truth, and you already did the correct
thing on the first try.

I am not asserting it. I cannot see the bundle. **The diagnostic below settles it
in one call**, and if it comes back the other way then you have found a real
defect in the snapshot path that my fixtures do not reach — and I want the
bundle and the `.dhoom` bytes.

## Q4 — Can a bundle be pinned to heap?

**Not today.** The selection is exactly one condition, in `Engine::open_mmap`: a
bundle goes mmap **iff `snapshots/{name}.dhoom` exists**, else it loads
heap-only. No threshold, no per-bundle hint. `GIGI_SKIP_BOOT_SNAPSHOT` is
engine-wide and only skips the post-replay snapshot; it is not what you want.

A per-bundle exclusion list is small and well-contained. I will build it if you
want it.

But I would rather you did not need it, so: **based on these tests I do not
expect v3 to regress.** The shape you rebuilt — categorical key, numeric fibers,
snapshot, reopen — is exactly what the new gates cover, and it survives. If v3
*does* regress at its next snapshot, that contradicts my reproduction and is the
most valuable thing you could send me.

## Q5 — How do we detect the regression early?

You are right that this is the one that matters, and right about why.
`curvature 0.0, confidence 1.0` on a 30,356-record embedding bundle is not a
healthy reading — it is what you get when there is no variance to measure,
because there is nothing to measure over. HEALTH had the evidence and reported
it as fine.

Smallest first:

1. **Post-snapshot readback.** After `.dhoom` is written, reopen it and assert
   every declared fiber is non-null on at least one record. One pass over the
   file we just wrote, on a path that already fsyncs.
2. **`field_coverage` on HEALTH** — per bundle, the fraction of declared fields
   non-null on any record. Your v2 would have read `2/392` since July and this
   conversation happens in a week, not a quarter.
3. **Refuse the flattering constant.** `curvature 0.0` with `confidence 1.0` on a
   wide numeric bundle should be a named refusal, not a number. Same contract as
   the gate you were let down by: decline rather than report a confident zero.

I would take all three. (2) is the one that would have caught this.

### (2) is built.

`GET /v1/bundles/{name}/health` now carries `field_coverage` and a `warnings`
array. Live, on a bundle rebuilt to your v2 shape — 387 declared fields, an
ingest that wrote three of them:

```
record_count : 200
k_global     : 0.0            <- the reading you were given
confidence   : 1.0            <- and this one
coverage     : 3/387 fields non-empty, complete_scan=true, sampled=200

WARNING: 384 of 387 declared fields carry no value in this bundle
         (v0, v1, v10, v100, ..., +376 more). Verbs over these fields will
         refuse; a schema field that was never written reads back Null, not
         absent.
WARNING: k_global is 0.0 with empty declared fields present: the curvature
         reading reflects absent data, not a flat bundle. Do not read
         confidence 1.0 here as health.
```

The same bundle with vectors actually written reports `387/387` and no
warnings, so the check discriminates rather than always complaining.

Notes on it:

- It lives on `BundleRef`, so heap and mmap answer identically. Storage mode
  should never change the answer, and your control was a heap bundle.
- `?coverage_sample=N` bounds the scan; default 1000, `0` scans everything.
  When it samples, `complete_scan` is false and the field is named
  `fields_empty_in_sample` — because a field populated only beyond the window
  would read empty here, and reporting that as an established zero is the same
  mistake this check exists to catch. On your bundles use `coverage_sample=0`.
- Gates at `tests/field_coverage_health.rs`. I removed the mechanism (counted
  `Null` as covered) and confirmed the two outage gates go red while the
  healthy-bundle and sampling gates stay green. Full suite: 1,489 pass, 0 fail.

So when you run the diagnostic below, you can also just ask health. If v2
reports `2/392` you have your answer without reading a single record by hand.

---

## The diagnostic — please run this first

One call, no writes, and it decides Q3:

```bash
# NO field projection - we need to see nulls, which a projection would hide
POST /v1/bundles/marcella_source_embeddings_bge_v2/query
     {"filters":[],"limit":1}
```

Then look at the returned object for `v0`:

| what you see | what it means | what to do |
|---|---|---|
| `"v0": null` | field **present and empty** — never written. Nothing lost; v1's `vector_str` is the source of truth | close as an ingest defect, keep v3 |
| `v0` **absent entirely** | a real snapshot-path defect my fixtures do not reach | send me the bundle; I take it from there |
| `"v0": 0.031…` | vectors are fine, fault is in the gate's field extraction | different bug, my mistake, tell me |

Same call on v1 for `record_id` — and please send the raw object rather than a
projection, because a projection cannot distinguish "dropped" from "null".

If it is the middle row, what I need is the bundle, and if you can get it,
`head -c 2000` of `snapshots/marcella_source_embeddings_bge_v2.dhoom`. The header
line alone tells me writer vs reader: if `v0` is in the header, the writer was
fine and the reader is at fault; if it is not, the loss is upstream of the file.

---

Two things plainly.

The throttled rebuild was good engineering. 50 per batch with a 0.35 s pause,
measured mid-write, after an unthrottled loop hung the site in August — that is
someone who learned the lesson and then proved the fix under load. And
reproducing the August receipts to four decimals before declaring v3 good is the
part most people skip.

And you were right to leave v2 alone. If I do need those bytes, they are still
there because you did not tidy up.

With respect, and a gate that fails when you break it,

**GIGI**
