# Incident: pre-deploy snapshot failure, 2026-08-20 04:43 UTC

**RESOLVED 2026-08-20 ~06:00 UTC. No deploy happened during the incident —
the v264 deploy was aborted when the snapshot failed. Recovery was: restore
the five duplicated bundles' June bases from `.prev`, restart v263, verify.
All verified; resolution section at the bottom.**

## What happened

Preparing to deploy the sheets-durability fix, following the snapshot-first
runbook: `POST /v1/admin/snapshot` against v263 failed after 198s —

```
HTTP 500
Snapshot failed: Invalid arithmetic pattern:
  Invalid step in 'binary_version@v6.7.0+gemini-drift-v08+0'
```

The rebase had already processed 4,091 of ~5,095 bundles when it aborted on
`cfs_psp_v01`. The WAL was untouched (abort preceded compaction — W0/W1
failing toward keeping the WAL, as designed). No deploy was made on the
failure.

## The two bugs (both fixed at `f597329`, both TDD'd red→green)

1. **Header delimiter injection.** The encoder folded a text column whose
   values contain `+` (`v6.7.0+gemini-drift-v08`) into an arithmetic header
   the decoder cannot re-split. Only reproducible with the actual production
   records — three synthetic fixtures failed to trigger the fold. The real 400
   served as the fixture; they were briefly committed to this PUBLIC repo by
   mistake (f597329), removed from the tip the same session per the standing
   decision on the TDD-GAUGE sweep, history left alone. The test now skips
   when the fixture is absent; synthetic tests carry the mechanisms in CI. Fix: `header_safe_text()` gating
   all three text folds.

2. **Key-encoding aliasing.** Base records live in JSON, which cannot
   represent `Timestamp` (returns `Integer`) or distinguish `Binary` from its
   b64 text form. Key comparisons built on `format!("{v:?}")` saw two keys for
   one record. Consequences, both directions:
   - the rebase merge wrote base AND overlay copies → five marcella bundles
     physically duplicated on disk (`marcella_source_sections` 155,610 →
     325,227);
   - live counts for Timestamp-keyed mmap bundles were inflated ~2× on every
     boot since June (`openssl_crypto_v05` reported 18,288 over a 9,145-record
     base). The 23 bundles that "shrank" during the incident were the count
     falling to the truth. **The disk was always right.**
   Fix: `Value::key_repr()` — one canonical key encoding, collapsing the JSON
   aliases, used by `pk_string` and all 17 key sites in `mmap_bundle`.

## Why the v264 restart heals rather than harms

Boot replays the WAL after the FIRST checkpoint marker into overlays (the
accumulator never resets on later markers), so overlays hold near-whole-WAL
state. With `key_repr` the overlay now correctly shadows every base row with
the same key — including both copies of a duplicated row. The follow-up
snapshot then writes the deduped merge, healing the disk, and performs the
first successful WAL compaction in this deployment's history.

## Predictions for post-deploy verification

| check | predicted |
|---|---|
| `jg_kv` records | exactly **1,381** (export taken pre-incident) |
| `openssl_crypto_v05` | **9,145** (the true count; NOT the historical 18,288) |
| `wikitext…train_edges…0416` | **19,960** |
| `marcella_source_sections` | **≈161,795** (pre-incident live count; NOT 325,227) |
| bundles | ≈5,095 |
| after admin snapshot | HTTP 200; WAL compacted from 1.35 GB to MBs; `.dhoom` line counts match live counts |

If `marcella_source_sections` comes back at ≈325k, the shadowing analysis is
wrong — stop, do not snapshot, restore `.prev` files for the five duplicated
bundles and reboot.

## Corrections to historical claims

Every count previously reported for Timestamp-keyed mmap bundles (audits,
inventories, the 08-12/08-15 predeploy captures) is suspect of ~2× inflation.
The 12.7M total record count includes this inflation; the true total emerges
after the first fixed snapshot.


## Bug 3, found during recovery verification: body newline shatter

Content-diffing the restored `marcella_genealogy_records` against the
duplicated state showed 12 "missing" strings — which turned out to be
FRAGMENTS of one authored record. The Aug-20 rebase was the first time those
records (WAL-only since authoring) were ever DHOOM-encoded, and the encoder
quoted newline-containing values CSV-style with the newline left literal.
Every reader iterates `body.lines()`, so the record shattered into one
counterfeit record per line: "the cedar chest", "found the name on my
grandmother's nursing degree" — each a line of one story.

All 11 real fragments verified as substrings of the intact restored records
(the 12th was a None artifact); exactly one restored record contains embedded
newlines, confirming the mechanism.

Fix: newline/CR-containing strings are encoded as the existing ``
sentinel + JSON string form (JSON escapes newlines; the row stays one line;
the mmap row index is untouched), routed through the same CSV-quote wrap the
array branch uses so the field splitter round-trips it. The decoder's sentinel
arm now accepts strings as well as arrays. Old files never contain
sentinel-prefixed strings, so nothing existing changes meaning.
`multiline_text_survives_snapshot_as_one_record` is the gate, red before the
fix (6 records where 2 were written).

## Resolution — verified against the live engine

Bee executed the ten restore commands (mv today's `.dhoom` aside as
`.dup20260820`, cp the June `.prev` back) for the five duplicated bundles,
then restarted the machine. Post-restart verification, all against the API:

| check | result |
|---|---|
| the five restored bundles | all EXACTLY at pre-incident counts (161,795 / 19,949 / 127 / 36 / 32) |
| bundles lost | zero (5,095 → 5,095) |
| `jg_kv` | 1,381 → 1,417 during the incident — gworls kept writing, unaffected |
| `cfs_psp_v01` | 400, intact (WAL-resident; it never had a `.dhoom`) |
| genealogy content | 11/11 fragments accounted for as substrings of intact records; the full "Madge Marcella Davis" record whole |
| the halved set | back at historical (double-counted) live values over CORRECT disk — the pre-existing key-aliasing symptom, cured by the key_repr fix when v264 deploys |

**Nothing was lost at any point.** The WAL held the truth throughout (W0/W1's
abort-toward-keeping-the-WAL is what made every recovery path exist), `.prev`
held the bases, and the `.dup20260820` files remain on the volume as evidence
until v264 is verified.

## What the deploy inherits

v264 ships: header-safety guard, key_repr unification, newline sentinel
encoding, the sheets journalling fix, the TDD-IDX schema-durability work, and
the drift detector. The deploy sequence inverts the failed one: deploy FIRST
(restart replays the WAL — proven safe by this incident's own restore), THEN
snapshot on the fixed encoder, then verify counts stay truthful. Running the
snapshot on v263's encoder is what this incident was.
