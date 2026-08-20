# GIGI-STREAM DEPLOY RUNBOOK
**v259 → main @ `0e69430`** · 60 commits (36 touching build inputs) · single machine `683961dbe9ee38`, iad, 13,122,071 records, no redundancy

---

## 1 · GO / NO-GO

**GO on the deploy.** No blocker stands between you and shipping this.

> ## ⚠️ THE NO-GO BELOW IS OBSOLETE AS OF 2026-08-13 — AND IS NOW INVERTED
>
> The paragraph that follows was correct when written and is **dangerous to
> follow now**. `POST /v1/admin/snapshot` was data-destructive on the mmap path
> because the snapshot loop touched only `self.bundles`. That is fixed:
> `snapshot_with_report` (`engine.rs:2726`) and `snapshot_with_chunk_size`
> (`engine.rs:3362`) both dispatch to `mmap_rebase_snapshot` when
> `mmap_bundles` is non-empty. Shipped in `05dd729` (TDD-DUR W1), deployed at
> **v262 on 2026-08-13**. Production has been running it since.
>
> **The advice is now the opposite: snapshot BEFORE a deploy, and that ordering
> is not optional.** A deploy is a restart, and mutations made through the
> non-journalling HTTP routes (TDD-IDX W-IDX-2) live only in RAM and the
> `.dhoom` until a snapshot writes them. Restarting without snapshotting first
> destroys exactly the writes at issue. The two routes the GIGI Sheets UI uses
> were fixed on 2026-08-16; seven others were not, and the fix is
> forward-looking only — it does nothing for what is already in flight.
>
> Left in place rather than deleted, because "the runbook said not to" is the
> kind of thing that gets quoted at 2am, and a deleted paragraph cannot be
> argued with.

**~~NO-GO~~ (OBSOLETE, SEE ABOVE) on one pre-flight step you would otherwise reach for: do not call `POST /v1/admin/snapshot` before this deploy.** That verb is data-destructive when the engine booted through the fast mmap path, which yours almost certainly did. `admin_snapshot` (`src/bin/gigi_stream.rs:12579-12583`) → `snapshot_with_chunk_size_report` (`src/engine.rs:2327-2496`) loops only `for (name, store) in &self.bundles` (`engine.rs:2348`) — `self.mmap_bundles` is never touched — and then compacts the WAL (`engine.rs:2491-2493` → `compact_wal_to_schemas`, `engine.rs:2706-2758`). Writes to an mmap bundle live in a RAM-only overlay whose only durable form is the post-checkpoint WAL. Compaction deletes that WAL while the `.dhoom` base stays at pre-write contents. The call returns `{"status":"ok"}` because RAM is intact. The loss appears on the next boot — which is the boot you are about to trigger. The runtime path does this correctly (`maybe_auto_compact` branches to `mmap_rebase_snapshot`, `engine.rs:3037-3041`); the admin route reaches neither.

Use a **Fly volume snapshot** instead. It copies `/data` at the block level — WAL and `.dhoom` together, engine uninvolved. Step 3 below.

Two things that are risk, not blocker, and that you should go in knowing:

- **Boot time is the only real unknown.** The fast mmap path is ~150s measured; the fallback heap replay was ~11 min at 12.17M records. Fly's readiness grace period is 900s (`fly.toml [checks.readiness]`). The boot code is byte-identical in this range — `open_mmap_fast`, `init_app_bundles` (165L), `init_system_bundles` (125L), `tigris_push`/`tigris_pull` all compare identical; `main()` goes 678L→693L with zero removed lines and exactly 7 added `.route(` lines. So the boot profile matches v259's *plus 22 days of accumulated post-checkpoint WAL*, and nothing compacts that WAL between boots. That accumulation is the one unbounded term, and the safe way to shrink it is not available to you (see the NO-GO above).
- **Eight already-deployed surfaces return different numbers afterward**, silently, with no version marker on the wire: `CONSISTENCY` (GQL, REST, and WS), `POST /scan`, `POST /scan/fit` (previously-fitted lens weights go stale and need re-fitting), `POST /infer` method=svm, spectral head=gmm, and — the wide one — `Bundle::records()` itself now sorts on Hashed and Hybrid-overflow storage (`bundle.rs:2842-2860`), which has ~150 call sites. An unordered `SELECT ... LIMIT n` can return a different subset of rows. This is a compatibility break, not a corruption risk: nothing rewrites the 13.1M stored records. What breaks is recorded baselines, hardcoded thresholds, and any client asserting on row order or on 404 body text (`No bundle: x` → `Bundle 'x' not found`, at 23 sites — and parity is incomplete: `parser.rs:11370, 11373, 11497, 11500` still emit the old string for the two-bundle PULLBACK and join verbs).

---

## 2 · WHAT THIS DEPLOY FIXES THAT IS BROKEN IN PROD RIGHT NOW

These are live defects on the machine serving your customer today.

### 2.1 Records silently invisible to every indexed query — `4fcca32`
The field index addressed roaring bitmaps as `bp as u32`, the low half of the 64-bit base point. Two records agreeing in their low word shared one bitmap entry and the second `bp_reverse.insert` overwrote the first. The losing record stayed perfectly present: `sections()` returned it, `len()` counted it, point lookups found it. It was gone only from paths that go through the index — `COVER … ON f=v`, `neighborhood()`, geometric neighbours, components, SPECTRAL, BETTI. No error, no warning.

Birthday bound, probability of at least one collision:

| records | P(collision) |
|---|---|
| 20,000 | 4.5% |
| 50,000 | 25.3% |
| 100,000 | 68.8% |
| 500,000 | ~100%, ~29 expected |

Any production bundle past ~100k rows very likely has unreachable records right now.

**This one repairs itself on restart, no migration.** The index is in-memory only — `bp_reverse`, the new `bp_forward`, and `next_bp_ordinal` are private `BundleStore` fields (`src/bundle.rs:726, 729, 733`) on a struct that derives only `Debug` (`bundle.rs:698`). No roaring bitmap is ever serialized (`grep 'serialize_into|deserialize_from' src/` → zero hits). Snapshots are streams of records, not indexes (`engine.rs:2631-2646`). The fresh binary re-interns from scratch via `intern_bp` (`bundle.rs:3317-3326`), which is collision-free by construction.

### 2.2 Silent total data loss on FLOAT / NUMERIC BASE bundles — `a458034`
The turbo ingest path keyed on `as_i64()` and `continue`d on anything else. `FieldType::Numeric` covers floats, so a bundle declared `(value NUMERIC BASE, …)` or `FLOAT BASE` routed there and **every record was dropped** — HTTP 200, `count: 0`, `parse_errors: 0`, no error anywhere. Found by customer acceptance testing 2026-08-10 with 1024 valid rows vanished. Fix routes on what the data is, not what the schema permits (`bundle.rs:1488-1515`); integer-keyed bundles keep the fast path unchanged.

**Already-lost rows are not recovered.** Affected bundles go from permanently empty to populated on their next ingest. If your customer has a float-keyed bundle, plan a re-ingest after this deploy.

### 2.3 One anonymous request kills the process — `41ceb37` (H2)
A ~60 KB unauthenticated POST to `/v1/public/gql` with nested `EXISTS` overflows the parser stack and `abort()`s the release binary. `abort()` is uncatchable, so no panic layer helps — and there is no panic layer (`tower-http` is cors-only, `Cargo.toml:143`). `restart_policy = "always"` means a cold restart into WAL replay, which is slower than re-firing the request. There is no rate limit (§7.1), so it is unmetered.

The deployed tree has no depth counter of any kind — `git grep MAX_PARSE_DEPTH 44161e2 -- src/parser.rs` returns nothing. The fix adds `MAX_PARSE_DEPTH = 64` (`parser.rs:2247`) and a `Parser::recurse` wrapper (`parser.rs:2275-2286`) on every recursion cycle, including a fourth (`parse ↔ parse_explain`, `parser.rs:6338`) the security review itself missed, plus threaded depth on the WEIGHT free functions (`parser.rs:8909-8956`). Five regression tests at `parser.rs:18259-18331`.

**This is the single highest-value item in the deploy.** The fix is written, tested, and sitting on main while production is one anonymous request from indefinite outage.

### 2.4 `CONSISTENCY` was reporting curvature as h1 — `f912c22`
GQL `CONSISTENCY <bundle>` returned `store.scalar_curvature()` verbatim — h1 was literally kappa. The REST route's h1 was structurally always 0 (`curvature::holonomy` compares only first vs last loop key, `curvature.rs:1195-1217`, and the loop was `[i,j,m,i]`). The WS text verb was a hardcoded stub. All three now run a shared sampled-contradiction kernel. Caveat: that kernel is heap-only (`store.as_heap()…unwrap_or_default()`, `gigi_stream.rs:8956-8971`), so on mmap-backed bundles the answer goes from kappa to `0.0`.

### 2.5 The three shape verbs
`TEXTURE` / `PRECEDENCE` / `CADENCE` — REST at `gigi_stream.rs:16753-16755`, GQL dispatch at `gigi_stream.rs:15549`. Not feature-gated (`pub mod ml;` is unconditional, `src/lib.rs:171`), so they ship live. They are **not** on the public anonymous allowlist — `validate_public_stmt` is byte-identical to the deployed version and they fall to its wildcard reject arm. Your customer needs an API key or a token.

---

## 3 · PRE-FLIGHT

Run in Git Bash. Every command here is read-only against the engine except step 3, which creates a backup.

### 3.0 · Set up
```bash
cd /c/Users/nurdm/OneDrive/Documents/gigi
export GIGI_KEY='<your GIGI_API_KEY>'
export H='https://gigi-stream.fly.dev'
export STAMP=$(date -u +%Y%m%dT%H%M%SZ)
mkdir -p "preflight_$STAMP"
```

### 3.1 · Settle mmap-vs-heap and capture per-bundle record counts
This is the load-bearing probe. `__bundles__` is virtual, never persisted, and every write verb rejects it — it cannot mutate anything (`src/virtual_bundles.rs`, `reject_virtual_write`).

```bash
curl -s -X POST "$H/v1/gql" \
  -H "x-api-key: $GIGI_KEY" -H 'content-type: application/json' \
  -d '{"query":"COVER __bundles__ ALL;"}' \
  > "preflight_$STAMP/bundles_before.json"

# how many are overlay (mmap) vs heap
grep -o '"type":"overlay"' "preflight_$STAMP/bundles_before.json" | wc -l
grep -o '"type":"heap"'    "preflight_$STAMP/bundles_before.json" | wc -l
```
`__bundles__` carries `type`, `n_records`, `created_ts` (`virtual_bundles.rs:93-95, 138-143`). For overlay bundles `created_ts` is literally the mtime of `snapshots/<name>.dhoom` (`engine.rs:1815-1826`), so `max(created_ts)` over `type='overlay'` is your last successful snapshot.

**If any bundle reports `type: "overlay"`, the admin snapshot NO-GO in §1 applies. Do not run it.**

### 3.2 · Capture engine-level state
```bash
curl -s "$H/v1/health" | tee "preflight_$STAMP/health_before.json"
curl -s -H "x-api-key: $GIGI_KEY" "$H/v1/metrics" > "preflight_$STAMP/metrics_before.json"
flyctl status -a gigi-stream            > "preflight_$STAMP/fly_status_before.txt"
flyctl releases -a gigi-stream --image  > "preflight_$STAMP/fly_releases_before.txt"
```
Expect `bundles: 5056`, `total_records: 13122071` (± drift).

### 3.3 · Take the durable backup — Fly volume snapshot
```bash
flyctl volumes snapshots create vol_4m8qxyxe3qdn06gr
flyctl volumes snapshots list vol_4m8qxyxe3qdn06gr | head -5
```
Wait until the new snapshot shows as created before proceeding. This is a block-level copy of the whole 50 GB volume — WAL and `.dhoom` consistent with each other, no engine cooperation required, and it is the only backup in this runbook that is unaffected by the mmap/overlay problem.

### 3.4 · Capture an indexed-query baseline (proves 2.1 landed)
Pick your largest bundle with an indexed field and a value you know is common. Replace the placeholders:
```bash
curl -s -X POST "$H/v1/gql" -H "x-api-key: $GIGI_KEY" -H 'content-type: application/json' \
  -d '{"query":"COVER <bundle> ON <indexed_field> = <value>;"}' \
  > "preflight_$STAMP/indexed_before.json"
wc -c "preflight_$STAMP/indexed_before.json"
```
Record the row count. After the deploy this should be **≥** the before count — previously-collided records reappear.

### 3.5 · Confirm the build (optional, ~6 min)
The exact prod feature line has been checked and compiles clean:
```bash
cargo check --release \
  --features "kahler imagine sharded transactions patterns causal_states wish halcyon post_kahler_phase1" \
  --bin gigi-stream --bin gigi-convert --bin gigi-edge
```
Expected: exit 0, ~5m42s, 10 `dead_code` warnings, 0 errors.

### 3.6 · Decision point — rate limiting in this deploy or a later one
`GIGI_RATE_LIMIT` is set in neither `fly.toml [env]` nor Fly secrets, so `state.rate_limit` is 0 and `rate_limit_middleware` short-circuits on its first line (`gigi_stream.rs:357-360`, `1626-1628`). It is read once in `StreamState::new`, so turning it on needs a restart — and you are about to pay one.

Honest tradeoff, both directions:
- **Turn it on now** (add `GIGI_RATE_LIMIT` and `GIGI_TRUST_PROXY` to `fly.toml [env]`, commit, deploy once): meters H1 and H3 immediately, one boot instead of two. Costs you clean attribution if something goes wrong.
- **Leave it for later**: this deploy stays a single-variable change and is easier to debug, but you pay a second full boot.

Either way both vars must go on together. With `GIGI_TRUST_PROXY` unset the limiter keys on `ConnectInfo` (`gigi_stream.rs:1637-1641`), which behind Fly's edge is the proxy address — one global bucket, and the first abuser 429s your paying customer. And with trust-proxy on, the map key becomes the attacker-supplied `x-forwarded-for` in an unbounded `HashMap` with no sweep task (`gigi_stream.rs:1650`; entries whose Vec empties are never removed). That is a memory-growth vector you will need to close. It is still net-positive against unmetered H1/H3.

**Do not touch `GIGI_SKIP_BOOT_SNAPSHOT` in this deploy.** Unsetting it triggers its own restart, and it is only read on the slow path (`gigi_stream.rs:17067`) — if you boot fast-mmap it is inert. Address it separately (§7.5).

---

## 4 · DEPLOY

```bash
git log --oneline -1          # expect 0e69430
git status --porcelain        # expect clean, or only intended fly.toml change

flyctl deploy -a gigi-stream --wait-timeout 20m --ha=false
```

`--wait-timeout 20m` matters: the readiness grace period is 900s and flyctl's default wait is shorter than a worst-case boot.

**In a second shell, watch the boot:**
```bash
flyctl logs -a gigi-stream
```

What you want to see (`gigi_stream.rs:17029`):
```
Engine ready — 13122071 records + _gigi_* system bundles (fast path)
```

What means it took the slow path — expect ~11–15 min, not 150s (`gigi_stream.rs:17039`):
```
Fast mmap open failed: … — falling back to heap replay
```

**What means stop and roll back immediately** (`gigi_stream.rs:17049-17055`) — this flips `ready` to true and health returns 200 with zero records:
```
WAL replay error: …
Engine ready (replay failed, using empty state)
```

Every non-health request 503s until `ready` flips (`readiness_middleware`, `gigi_stream.rs:1332-1345`). One machine, one volume — no second machine can serve alongside it, so expect the full boot as downtime.

---

## 5 · VERIFY

```bash
export POST=$STAMP   # keep the same folder
```

### 5.1 · Engine came back whole — PASS: counts match §3.2
```bash
curl -s "$H/v1/health" | tee "preflight_$STAMP/health_after.json"
```
**PASS:** `total_records` within a few hundred of 13,122,071 and `bundles` ≥ 5056. `_gigi_*` system bundles accrue records from log events, so a small upward drift is expected. **FAIL:** `total_records: 0`, or `loading: true` persisting past the grace period.

### 5.2 · Per-bundle comparison — PASS: no bundle lost rows
```bash
curl -s -X POST "$H/v1/gql" -H "x-api-key: $GIGI_KEY" -H 'content-type: application/json' \
  -d '{"query":"COVER __bundles__ ALL;"}' > "preflight_$STAMP/bundles_after.json"

diff <(python -c "import json,sys;d=json.load(open(sys.argv[1]));print('\n'.join(sorted(f\"{r['name']}\t{r['n_records']}\" for r in d.get('records',d.get('rows',[])))))" "preflight_$STAMP/bundles_before.json") \
     <(python -c "import json,sys;d=json.load(open(sys.argv[1]));print('\n'.join(sorted(f\"{r['name']}\t{r['n_records']}\" for r in d.get('records',d.get('rows',[])))))" "preflight_$STAMP/bundles_after.json")
```
**PASS:** differences only in `_gigi_*` bundles, and only upward. **FAIL:** any non-`_gigi_` bundle lost rows, or a bundle disappeared. That is the §1 overlay-loss signature — roll back and restore the volume snapshot.

### 5.3 · The three shape verbs — the reason you are here
```bash
# GQL — this is exactly what returns 400 "Unknown statement: 'TEXTURE'" today
curl -s -X POST "$H/v1/gql" -H "x-api-key: $GIGI_KEY" -H 'content-type: application/json' \
  -d '{"query":"TEXTURE <bundle> ON <numeric_field> ALONG <order_field>;"}'

curl -s -X POST "$H/v1/gql" -H "x-api-key: $GIGI_KEY" -H 'content-type: application/json' \
  -d '{"query":"PRECEDENCE <bundle> ON <field_x>, <field_y> ALONG <order_field>;"}'

curl -s -X POST "$H/v1/gql" -H "x-api-key: $GIGI_KEY" -H 'content-type: application/json' \
  -d '{"query":"CADENCE <bundle> ON <timestamp_field>;"}'

# REST
curl -s -X POST "$H/v1/bundles/<bundle>/texture" \
  -H "x-api-key: $GIGI_KEY" -H 'content-type: application/json' \
  -d '{"field":"<numeric_field>","order":"<order_field>"}'
```
**PASS:** HTTP 200 with `exponent`, `r_squared`, `n_lags`, `n` (`src/ml/texture.rs:67-76`). **FAIL:** 400 `Unknown statement` means the deploy did not land. A 422 naming the bundle and its storage mode is a *correct* refusal, not a failure — TEXTURE/PRECEDENCE need an ordering field on a hash-stored bundle (`docs/SHAPE_VERBS.md:552`). `CADENCE` takes no `ALONG` and needs a real clock, not a row index (`docs/SHAPE_VERBS.md:372-378`). Request structs are `deny_unknown_fields`, so a misspelled key is now a 400 rather than a silently different answer (`src/ml/texture.rs:41-46`).

### 5.4 · The index fix landed — PASS: count went up or stayed equal
```bash
curl -s -X POST "$H/v1/gql" -H "x-api-key: $GIGI_KEY" -H 'content-type: application/json' \
  -d '{"query":"COVER <bundle> ON <indexed_field> = <value>;"}' \
  > "preflight_$STAMP/indexed_after.json"
```
**PASS:** row count ≥ §3.4's baseline. An increase is the collided records coming back. **FAIL:** fewer rows.

### 5.5 · H2 is closed
```bash
python - <<'PY' > /tmp/h2.json
import json
q = "COVER sensors WHERE " + "EXISTS (COVER sensors WHERE " * 300 + "x = 1" + ")" * 300 + ";"
print(json.dumps({"query": q}))
PY
curl -s -X POST "$H/v1/public/gql" -H 'content-type: application/json' -d @/tmp/h2.json
curl -s -o /dev/null -w '%{http_code}\n' "$H/v1/health"
```
**PASS:** a 4xx with a "too deep" style error, and health still 200. **FAIL:** connection reset / empty reply followed by health 000 — that is the process gone. Run this against production only after you have the volume snapshot from §3.3 in hand.

### 5.6 · jg_kv still works
```bash
curl -s -o /dev/null -w '%{http_code}\n' -X POST "$H/v1/bundles/jg_kv/query" \
  -H "x-api-key: $GIGI_KEY" -H 'content-type: application/json' -d '{"limit":1}'
```
**PASS:** 200. All four jg_kv handler bodies (`insert_records`, `update_records_v2`, `filtered_query`, `delete_records_v2`) and the whole auth / namespace / rate-limit middleware stack are byte-identical to the deployed build, so this should be uneventful.

---

## 6 · ROLLBACK

```bash
flyctl deploy -a gigi-stream \
  -i registry.fly.io/gigi-stream:deployment-01KY0AH9Z2Z2EYQP3SHAHQNN3T \
  --wait-timeout 20m
```
That is v259, the image running today.

**What rollback cannot undo.** It swaps the binary. `/data` is a persistent volume that survives the swap, so anything the new binary wrote there is permanent and unreachable by rollback: WAL entries appended after boot, and any `.dhoom` rotation (`rotate_snapshot`, `engine.rs:2271-2320`, keeps exactly *one* prior generation — a second snapshot cycle overwrites `.dhoom.prev`). There is no release-scoped copy of `/data` anywhere.

**Real data rollback** is the volume snapshot from §3.3, restored with `flyctl volumes create --snapshot-id <id>`. That creates a **new volume** — you then have to detach the old one and attach the new one to the machine. It is not a one-liner and it discards everything written since the snapshot. Have the snapshot ID written down before you start.

**What rollback costs you.** You go back to: H2 live and unmetered, records silently invisible to every indexed query on any bundle past ~100k rows, float-keyed bundles silently dropping every ingest, and no shape verbs for the customer. Rollback is the right move only if §5.1 or §5.2 fails.

---

## 7 · AFTER, BEFORE THE CUSTOMER'S DATA LANDS

The deploy is a strict security improvement — H2 closes, no new anonymous surface, no new public verbs, all 7 new routes are `/v1/bundles/{name}/…` and therefore path-gated. But it is not a security release. It fixes exactly one of 33 confirmed findings.

The bigger posture change in this window had nothing to do with code: **`GIGI_JWT_SECRET` is now a live Fly secret**, and the mint at `davisgeometric.com/api/gigi/token` exists. That flipped five findings from latent to reachable.

### 7.1 · Config wins — no code, but each needs a restart
| Item | What | Note |
|---|---|---|
| `GIGI_RATE_LIMIT` + `GIGI_TRUST_PROXY` | Meters H1, H3, M6. Set both or neither (§3.6). | Introduces an unbounded XFF-keyed map, `gigi_stream.rs:1650` — close that after. |
| **Rotate `GIGI_JWT_SECRET`** | Any token signed with the current value verifies. Its exact trimmed length is enumerable by any token holder via `CREATE BUNDLE … SEED FROM ENV` (`gigi_stream.rs:12995`, `13073`; length echoed at `crypto.rs:890-897`). Treat it as disclosed. | Rotate on Vercel and Fly together, invalidating outstanding tokens. |

### 7.2 · Code fix #1 — the tenant-authorization group (H4 + H5 + H6 + M2 + M1), as ONE change
The precondition is met on both sides. A customer holding their own non-owner token can today read, write, and irreversibly `COLLAPSE` **every bundle on the box** — the customer is simultaneously the victim and the credential holder.

- **H4:** `/v1/gql` never reads claims (`gigi_stream.rs:12879-12883`); the path gate falls to `_ => None` because the second segment is `gql` (`gigi_stream.rs:1554`). `ShowBundles` enumerates unfiltered (`13025-13040`); `Collapse` drops unconditionally with no `_gigi_` guard (`13042-13044`).
- **H5:** `/ws` has no `/v1` prefix so namespace enforcement never runs; claims are dropped at upgrade. The insert broadcast frames caller-supplied **pre-encryption** values, so an `encrypted:"opaque"` field goes out in cleartext.
- **H6:** `POST /v1/bundles/<own>/join` takes `right_bundle` from the **body** (`gigi_stream.rs:2669`) and never checks it — direct exfiltration of the full right-hand record, and it survives any `/v1/gql` fix. `/v1/divergence` has the same gap with no `as_heap` gate at all.
- **M2:** admin routes are registered flat with no `route_layer` anywhere in `src/` (`gigi_stream.rs:16617-16620`, handler at `12579`). Any tenant can stall the single node, or silence query logging so a subsequent H4 exercise leaves nothing behind.

Shape: a `state.bundle_for(&claims, name)` accessor as the only way to get a `BundleRef` in the HTTP layer with `engine.bundle()` made crate-private; a `get_bundle_names(&Statement)` walker feeding `allows_bundle` before GQL dispatch; claims threaded into `handle_ws_command`; an owner-only `route_layer` over an `/v1/admin/*` sub-router. Note `get_bundle_name` (`gigi_stream.rs:15588-15610`) is a starting point but insufficient — it returns only `left` for Pullback/Join and has no arm for CreateBundle or Collapse. It *does* already cover the three new shape verbs, so they come along free.

### 7.3 · Code fix #2 — H1, anonymous cross-bundle read
`validate_public_stmt` destructures `S::Cover { bundle, .. }` (`gigi_stream.rs:12766`), discarding `where_conditions`; the executor then resolves the inner `Exists { cover_bundle, … }` against the whole engine with no allowlist in scope (`gigi_stream.rs:13931`). `state.public_bundles` never reaches the executor. No credentials needed, all 5056 bundles reachable, with a demonstrated character-by-character extraction oracle. Fix must walk the condition tree recursively (an Exists can nest another) and cover `excluding` in the same change.

### 7.4 · Code fix #3 — H3, then H7
- **H3:** unbounded attacker-keyed regex cache, `bundle.rs:208-219`, no eviction, no `size_limit`. ~1.94 MB retained per distinct pattern; one 2.6 KB anonymous request parks 162 MB; ~200 requests exhaust the 32 GB VM.
- **H7:** `src/ml/cluster.rs:352`, `if method != "dbscan" && n < 2 * k` with `k` a bare unclamped `usize` from JSON. Release has `overflow-checks=false` (no `[profile]` section in `Cargo.toml`), so `2 * 2^63` wraps to 0. One request permanently wedges query service — reads and writes — recoverable only by restart, never self-clears. Cheapest severe item on the board: fix as `k > n / 2`, clamp `k` first, add `[profile.release] overflow-checks = true`, and move `cluster_records` under `spawn_blocking`.

### 7.5 · Follow-ups
- **Fix `admin_snapshot`** to branch on `mmap_bundles` exactly as `engine.rs:3037-3041` already does. It is still the only HTTP-reachable way to persist heap-only bundles created after boot (`claude_substrate_v0`, `workflow_recruiting`), so the answer is not "stop using it." Second loss mode in the same loop: `if count == 0 { continue; }` (`engine.rs:2353-2355`) skips currently-empty bundles, leaving a stale `.dhoom` while compaction erases the Deletes that emptied it — that one bites in heap mode too.
- **Retire `GIGI_SKIP_BOOT_SNAPSHOT`.** The encoder wedge it dodges was fixed 2026-07-16 (`theory/gigi/DURABILITY_ENCODER_HANG_DIAGNOSIS_2026-07-16.md`), *before* v259 was built. It is now pure downside — it forfeits the post-replay snapshot that makes the next boot fast.
- **H10 + H11**, before encrypted customer data accumulates. Derived AES-256 keys are written verbatim into `gigi.wal` beside their own ciphertext (`src/wal.rs:1447-1454`) and both sync to `gigi-snapshots`. The per-field KDF is a fully invertible 64-bit mixer (`src/crypto.rs:876-887`), so one recovered field key yields every field key under the same seed. Both are wire-format breaks needing a versioned gauge-key marker and a re-key — and re-keying only gets more expensive as customer volume grows. The spec already calls for HKDF-SHA256 and the `hkdf` crate is already used correctly at `src/integrity.rs:101`.
- **`gigi-convert` and `gigi-edge` now ship inside the runtime image** (`Dockerfile`, `49a9eaa`). Nothing starts them — `CMD` is still `gigi-stream` — but `src/bin/gigi_edge.rs` has never been security-reviewed.
- **Public demo surface is already dead.** Of the four `GIGI_PUBLIC_BUNDLES`, `stations` / `chembl` / `tetmesh_demo` return `No bundle:` and `sensors` has `record_count: 0`. The GIGI Builds exercise pages point at these.

---

**Files referenced:** `C:/Users/nurdm/OneDrive/Documents/gigi/src/bin/gigi_stream.rs`, `C:/Users/nurdm/OneDrive/Documents/gigi/src/bundle.rs`, `C:/Users/nurdm/OneDrive/Documents/gigi/src/engine.rs`, `C:/Users/nurdm/OneDrive/Documents/gigi/src/parser.rs`, `C:/Users/nurdm/OneDrive/Documents/gigi/src/crypto.rs`, `C:/Users/nurdm/OneDrive/Documents/gigi/src/wal.rs`, `C:/Users/nurdm/OneDrive/Documents/gigi/src/ml/texture.rs`, `C:/Users/nurdm/OneDrive/Documents/gigi/src/virtual_bundles.rs`, `C:/Users/nurdm/OneDrive/Documents/gigi/fly.toml`, `C:/Users/nurdm/OneDrive/Documents/gigi/docs/SHAPE_VERBS.md`, `C:/Users/nurdm/OneDrive/Documents/gigi/SECURITY_REVIEW_SELF.md`, `C:/Users/nurdm/OneDrive/Documents/gigi/theory/gigi/DURABILITY_ENCODER_HANG_DIAGNOSIS_2026-07-16.md`