# gigi-stream deploy — 2026-09-20

Prepared, not executed. `flyctl` is installed and **not authenticated**, so the
deploy itself is yours to run. Everything that does not need the token is done
and recorded below.

Target: `origin/main` at `8ef4dd4`. Deployed today: **v0.1.0, 31 days uptime**,
which predates every commit in this list.

---

## 1 · What goes live

Five commits from today, plus everything already merged and never deployed.

| commit | what |
|---|---|
| `b409707` | the log records how to re-derive the key, not the key |
| `7a8f5e0` | a key rotation is recorded, not only applied |
| `cbe5513` | fiber values sealed before journalling |
| `64fa97f` | bundles declare order, retention, row semantics, units |
| `a96a3d8` | health reports `null` over fields it cannot see |
| `8ef4dd4` | a record with no key is refused, not collapsed |
| `e67719a` | schema records written in the oldest version that fits, keeping rollback open |
| _pending_ | one version, reported from the crate, so the deploy is verifiable |

---

## 2 · Behaviour that changes on the live host

**Read this before deciding, not after.** Each of these previously succeeded
and will now refuse. All are deliberate; none has ever run against production.

1. **An insert whose record omits a declared base field is refused.** This is
   the newest change and the one with live-writer exposure. Any writer relying
   on the old behaviour was silently collapsing its records onto one row, so a
   refusal is strictly better than what it was getting — but it will surface as
   failed writes where it previously surfaced as nothing.
2. **Creating an encrypted bundle without `WITH ENCRYPTION SEED FROM ENV`** is
   refused. Existing encrypted bundles are unaffected; this is creation only.
3. **An order-sensitive verb with no order available** refuses. Stricter than
   before: it now covers memory-mapped bundles, which is most of production.
   TEXTURE, PRECEDENCE and CHANGEPOINTS on an mmap bundle need either a
   declared order field or one named on the call.
4. **CHANGEPOINTS no longer infers a clock from field names.** Callers relying
   on the old substring guess will get a refusal.
5. **An order field with tied values** refuses.
6. **CADENCE on a bundle declaring discrete events** refuses. Nothing declares
   that yet, so this is inert until someone does.

**Wire format is unchanged for healthy bundles.** `confidence` is now
`Option<f64>`; `Some(x)` serialises exactly as before, and `unreadable_fields`
is omitted when empty. Only a bundle whose snapshot header is short returns
`confidence: null` — which is the point.

Item 3 is the one most likely to surprise a dashboard. Worth a grep of live
callers before pressing go.

---

## 3 · Pre-flight

Verified here:

- [x] Release build, **exact image feature set**, compiles clean:
      `kahler imagine sharded transactions patterns causal_states wish halcyon
      post_kahler_phase1`, all three binaries.
- [x] Full suite green on the default feature set: 1552 passed, 0 failed.
- [ ] Full suite under the image's feature set — running at time of writing.
- [ ] `fly auth login` — **yours**.

Still to do on the host, from `DEPLOY_RUNBOOK_2026-08-11.md` §3:

1. **Capture per-bundle record counts and engine totals.** These are the
   comparison for §5. Last known: 5,096 bundles, 12,707,004 records,
   `marcella_source_sections` at 161,795.
2. **Fly volume snapshot.** Block-level copy of `/data`, engine uninvolved.
   This is the durable backup and the rollback of last resort.
3. **Engine snapshot before the restart, and this is not optional.** A deploy
   is a restart. Seven HTTP mutation routes still write only to RAM and the
   `.dhoom`, so a restart without a prior snapshot destroys exactly those
   writes. The runbook's original advice said the opposite and is marked
   obsolete in place; follow the inverted version.

   Note it is **engine-wide** and takes minutes: 5,096 bundles, 12.7M records.
   It blocked reads for roughly ten minutes when it was run earlier today. Plan
   for that window rather than being surprised by it.

---

## 4 · Deploy

```bash
fly auth login
fly deploy --app gigi-stream
```

Boot is the unknown, not the build. Fast mmap path is ~150s measured; the heap
replay fallback was ~11 min at 12.17M records, and the volume has accumulated
WAL since. Fly's readiness grace is 900s (`fly.toml [checks.readiness]`), so a
heap-replay boot is inside the window but not by much.

---

## 5 · Verify

1. `GET /v1/health` reports **`"version":"0.4.1"`**.

   This works only because it was fixed as part of this deploy. Health served a
   hardcoded `"0.1.0"` literal, metrics served the crate version which was also
   0.1.0, and the API document said 0.4.0 — three strings, none tied to the
   binary, so a deploy had no way to prove itself live. All three now come from
   the crate and a test asserts they agree.
2. Bundle count is 5,096 and total records match §3.1 — no bundle lost rows.
3. `marcella_source_sections` still holds 161,795.
4. The new behaviour is actually live, which the version string alone does not
   prove. Cheapest probe: `GET /v1/bundles/marcella_source_sections/curvature`
   should now return `confidence: null` and an `unreadable_fields` list naming
   `section_id` and the other five. If it still says `confidence: 1.0`, the old
   binary is still serving.
5. `davisgeometric.com/marcella` returns 200.

---

## 6 · Rollback

`fly releases --app gigi-stream` then `fly deploy --image <previous>`, per
runbook §6. The Fly volume snapshot from §3.2 is the data rollback and is
independent of the image.

A rollback restores the old binary, which means the refusals in §2 stop
refusing.

**There was a one-way door here and it is closed.** Schema records carry a
version, and the binary in production has no version handling at all — it would
misread the marker as a bundle-name length. Compaction re-emits every schema,
so writing the newest version unconditionally would have made every bundle on
the volume unreadable to the previous binary within one compaction of the
deploy, including the 5,000-odd that never used a new feature.

A schema is now written in the **oldest version that represents it**. A bundle
using none of today's declarations is still written in the original format and
still reads on the deployed binary. You pay the compatibility cost only for a
bundle that actually declares an order field, a retention, row semantics or a
unit — and none exist yet.

So the rollback is clean today and narrows only as the new declarations get
used. Gate: `wal::tests::a_schema_is_written_in_the_oldest_version_that_represents_it`.
