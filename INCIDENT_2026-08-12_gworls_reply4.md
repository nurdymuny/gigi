# gworls — the two open defects are closed

Follow-up to the 2026-08-13 correction. Nothing in this note changes anything
we told you about your data; that message stands. This one closes the two
defects it left open.

Both are fixed, both have a test that goes red without the fix, and both are
deployed.

---

## 1 · A delete against an mmap-backed bundle was lost on restart

**What you would have seen.** Delete a record. It disappears — immediately,
and from every lookup you would naturally try. Restart the engine and it is
back. No error at any point.

**Why.** A deleted row is hidden by a tombstone, and the tombstone set had two
groups of callers encoding the key two different ways:

```
Engine::pk_string             -> [("id", Integer(1))]
OverlayBundle::tombstone_key  -> Integer(1)
```

`Engine::delete` wrote under the first form and `Engine::point_query` read
under the same first form, so a delete verified correct the moment you made
it. Everything else — the record count, full scans, and both paths that write
a new `.dhoom` — asked the second question and was told the row was still
there.

The consequence worse than the resurrection: a snapshot taken while a
tombstone was live copied the deleted row into the new base file. The delete
was not merely forgotten, it was actively undone on disk.

**Fix.** The overlay now derives the key itself, from the record, in the one
place that owns the tombstone set. No caller is in a position to hold a
different opinion about the encoding.

**Scope for you.** Any delete you issued against a bundle large enough to have
been snapshotted may not have taken. Deletes are worth re-checking; nothing
needs restoring, since the failure mode kept data rather than losing it.

---

## 2 · `snapshot()` could write a `.dhoom` with an empty body

**What you would have seen.** A snapshot reports the correct record count.
The bundle reloads empty.

**Why.** The encoder folds a column into the file header whenever it can
describe it in closed form — a counter, a constant, a repeated prefix.
Nothing stopped it folding *every* column. When it did, each record encoded
to an empty line and the reader found no rows to count.

Measured, one bundle, two columns:

| records | file | reloaded |
|---|---|---|
| 1 | 21 B | **0** |
| 10 | 30 B | **0** |
| 50 | 354 B | 50 |

At 50 the values stopped being uniform enough to fold and the bundle
survived. Small and regular was the dangerous shape — the more compressible
the bundle, the more completely it vanished. A wide bundle with real variation
in it was never at risk, which is why this did not show up in the counts we
sent you.

**Fix.** If the encoder finds nothing left to write, it puts one column back,
choosing the cheapest one to give up.

---

## What this does and does not mean

**Does:** the two defects we listed as open in the last note are closed, and
the class of failure behind tonight's incident — a success reported over an
operation that did nothing — now has gates in the test suite at four separate
points in the write path.

**Does not:** claim the engine is now free of defects. It means these two are
fixed and cannot regress silently.

If you have deletes from before this deploy that you need to be certain of,
tell us which bundles and we will verify them against the live engine rather
than asking you to trust the fix.
