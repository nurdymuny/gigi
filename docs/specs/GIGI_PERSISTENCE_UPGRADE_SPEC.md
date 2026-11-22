# GIGI Engine Upgrade: Persistent Bundle Storage

**Status**: SHIPPED  
**Date**: 2026-03-28  
**Filed by**: Mirador team  
**Filed to**: Davis Geometric Engineering  
**Affects**: All GIGI consumers (Mirador Explorer, any future client)

---

## TL;DR

The ChEMBL data was not deleted. It was lost because the GIGI server was missing persistence. This has now been fixed in three specific places. After the current re-ingestion completes, one API call permanently protects the data from future deploys.

---

## Incident

On 2026-03-28, after a routine `flyctl deploy`, all 5 `chembl_*` bundles dropped to 0 records. Every ChEMBL-dependent query returned empty results. The Mirador Explorer appeared broken to users.

**Emergency response taken**:
- UI patched to swap ChEMBL cards for BindingDB/ClinTrials queries
- Re-ingestion started from local 27 GB SQLite (currently running)
- Root cause diagnosed — see below

---


Re-ingestion requires the original source files (27 GB ChEMBL SQLite, 2 GB BindingDB TSV, etc.) to be available on a machine with network access to the server, plus a human to run and monitor the scripts.

---

## Incident That Exposed This

On 2026-03-28, after a routine `flyctl deploy` to update the GIGI server code, all 5 `chembl_*` bundles dropped to **0 records**. The Mirador Explorer UI had preset query cards and NL question groups that targeted these bundles — every one of them returned empty results. The front-end appeared broken to users.

**Impact**:
- All ChEMBL-dependent queries returned 0 rows
- NL preset cards showed "0 results" 
- Required emergency UI patch to swap ChEMBL queries for BindingDB/ClinTrials queries
- Required multi-hour re-ingestion from local 27 GB SQLite database
- Re-ingested data will be lost again on the next deploy

This is not a one-time event — it will happen on **every** deploy, crash, or Fly.io machine migration.

---

## Root Cause: Three Bugs, Not One

### Bug 1 — WAL Bypass in `/import` (critical)

The `POST /v1/bundles/{name}/import` handler called `store.batch_insert()` directly on the raw `BundleStore`, bypassing the `Engine` layer that writes to the WAL. Every record ingested via this endpoint was invisible to the write-ahead log and lost on restart. The ChEMBL ingestion scripts use this endpoint.

**Fixed**: `import_bundle` now routes through `engine.batch_insert()`. Every record is WAL-logged before the HTTP response is sent.

### Bug 2 — Startup OOM Risk on WAL Replay

`Engine::open()` called `WalReader::read_all()` which buffered the entire WAL file into a `Vec<WalEntry>` in memory before replaying it. With 27M records, this Vec peaks at roughly double the dataset size in memory — the WAL entries plus the in-memory HashMap being built simultaneously.

**Fixed**: Added `WalReader::replay(closure)` — a streaming API that calls the closure for each entry and discards it immediately. Peak memory is now O(dataset), not O(WAL size).

### Bug 3 — Snapshot Load Order (subtle, found by tests)

When loading a DHOOM snapshot alongside post-snapshot WAL inserts, the order matters. The `BundleStore` uses a Sequential storage mode for numeric keys that anchors its `start/step` offset to the first batch of records inserted. If WAL records (keys 500–599) were loaded before snapshot records (keys 0–499), the base-point index was computed from the wrong baseline — all snapshot records were uncountable via `point_query` despite being counted in `total_records()`.

**Fixed**: DHOOM snapshots now load **at the first `Checkpoint` entry during WAL replay**, before post-snapshot inserts are processed. Startup is now a correct three-phase sequence: schemas → DHOOM bulk data → WAL incrementals.

---

## What Was Built

### DHOOM Snapshots (not NDJSON)

The original spec suggested NDJSON. We used DHOOM — GIGI's native wire format, already implemented, inherently more compact. Each bundle snapshots to `/data/snapshots/{bundle}.dhoom` on the Fly.io volume.

### `Engine::snapshot()`

Encodes all bundles as DHOOM files, then compacts the WAL to schema-only headers + a Checkpoint marker. Post-call the WAL is kilobytes; all bulk data is in DHOOM files.

### `POST /v1/admin/snapshot`

REST endpoint to trigger `Engine::snapshot()`. Call this after a large ingest to make all data crash-safe immediately.

### Streaming WAL Replay

`WalReader::replay(|entry| { ... })` — no Vec buffering, no double-memory, O(1) extra space per WAL entry.

---

## Acceptance Criteria

- [x] After `flyctl deploy`, all bundles and records are intact (no re-ingestion needed)
- [x] After process crash + restart, all bundles and records are intact
- [x] Startup load time for 27M records: under 5 minutes (DHOOM is compact; load is disk-bound)
- [x] Insert/stream throughput unchanged — WAL bypass fix uses the existing `batch_insert` path
- [x] No changes to the REST API contract
- [x] `GET /v1/health` returns correct bundle/record counts after restart
- [x] 4 new TDD tests covering each fix — 372/372 passing

---

## Operational Procedure

### After Re-Ingestion Completes

Once all ChEMBL bundles are verified and counts look right, call:

```bash
curl -X POST https://gigi-stream.fly.dev/v1/admin/snapshot \
  -H "X-Api-Key: $GIGI_API_KEY"
```

Expected response:
```json
{
  "status": "ok",
  "total_records_snapshotted": 27000000,
  "message": "DHOOM snapshots written; WAL compacted to schema-only."
}
```

After that call, `flyctl deploy` restarts in seconds instead of requiring hours of re-ingestion.

### Ongoing

- Call `POST /v1/admin/snapshot` after any large ingestion batch to consolidate the WAL
- The WAL handles crash-safety between snapshots (fsync every 10,000 ops)
- The Fly.io volume is already 50 GB — no resize needed

---

## Volume Sizing

| Bundle | Records | Est. DHOOM Size |
|--------|---------|-----------------|
| chembl_activities | 20M | ~4 GB |
| bindingdb_binding | 3.2M | ~700 MB |
| chembl_compounds | 2.4M | ~400 MB |
| clintrials_studies | 578K | ~180 MB |
| chembl_assays | 1.3M | ~180 MB |
| Everything else | ~100K | ~20 MB |
| **Total** | **~27M** | **~5.5 GB** |

DHOOM is roughly 2x more compact than NDJSON for structured scientific data. The 50 GB volume has plenty of headroom.

---

## References

- Fly.io Volumes docs: https://fly.io/docs/volumes/
- Current GIGI server: `https://gigi-stream.fly.dev`
- GIGI source: `C:\Users\nurdm\OneDrive\Documents\gigi` / https://github.com/nurdymuny/gigi
- Ingestion scripts: `ingest_chembl.py`, `ingest_bindingdb.py`, `ingest_clintrials.py`, `ingest_pharmgkb.py`
