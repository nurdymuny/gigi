# Halcyon → GIGI Substrate, Part V gates (Snapshot persistence)

**Status:** Sprint-locked — request to engine, scope-tight
**Author:** Bee Rosa Davis, with Claude (Anthropic)
**Date:** 19 June 2026
**Companion:** ``HALCYON_PART_I_GATES.md`` (the existing Part I–IV sprint gates), ``../../papers/solves_vol4_ym_mass_gap.tex`` Appendix A.4 (the public-receipt verifier the snapshot verb would close out), ``HALCYON_TO_GIGI_REPLY_2026-06-17.md`` § A2 (the bit-identity contract this builds on)
**Goal:** add the minimum WAL op + GQL verb that lets a thermalized `GAUGE_FIELD` survive a `gigi-stream` restart, so the public-receipt verifier in Solves Vol. 4 Appendix A.4 can hand a reader a one-line `GET` instead of a 30-second thermalization on every request.

**Precondition surfaced 2026-06-19:** `POST /v1/gql` does not dispatch gauge-feature statements on production (`805e0c8`). The `gigi_stream.rs::gql_query` handler only knows about bundle-shaped statements; every `LATTICE`, `GAUGE_FIELD`, `GIBBS_SAMPLE`, `E_FIELD`, `SYMPLECTIC_FLOW` statement falls into the default early-return and gets `{"status":"ok"}` without ever reaching `parser::execute`. The route-resolution receipts in the `805e0c8` deploy log are correct at the route layer — but data flow is severed. Part V is blocked behind this precondition (named **P-1** in §2.5 below) because the snapshot verb would itself fall into the same default. P-1 is also what makes the chapter's Appendix A.4 receipt actually work against production.

---

## 0. Letter

Gigi —

Parts I–IV closed cleanly: every gauge-field phase of `run_validation_report.py` has a GQL home, the Halcyon-side test scaffold pins 53 mock + 20 live gates, the matched-RNG receipt demonstrates byte-equality on demand, and the Solves Vol. 4 chapter renders the worked example as a five-statement GQL block. From the chapter:

> ⟨P⟩ = 0.5068472 ± 0.0014580 over the last 100 of 200 sweeps at β = 2.5

The chapter's Appendix A.4 hands a reader a one-line `verify_canonical_receipt.py` against any reachable `gigi-stream`. It works. But it makes a reader thermalize from `IDENTITY` on every call — about thirty seconds against `gigi-stream.fly.dev` per verification. That cost is real for a paper reader: every cite-check pays it again.

The Part II / Part III WAL ships exactly two ops today, ``OP_LATTICE_DECLARE`` (`0x09`) and ``OP_GAUGE_FIELD_DECLARE`` (`0x0A`). Both persist the *declaration*. Neither persists the *post-thermalization buffer*. On restart, the field re-initializes to `IDENTITY`. The verifier wanting to cite a long-lived canonical can't get one.

The ask is a third op and a thin verb that pairs with it. **`OP_GAUGE_FIELD_SNAPSHOT` (`0x0B`)** logs the SU(2) link buffer of an already-declared field as raw `f64` bytes. **`SNAPSHOT GAUGE_FIELD U;`** is the GQL surface that writes the op. Declarations stay separate from contents; users explicitly snapshot when they want durability of state.

Sized against your existing pattern: a 90-edge × 4-component `f64` buffer is 2,880 bytes per SU(2) field. That's smaller than a single typical bundle insert. The op is additive — declared LATTICEs and GAUGE_FIELDs from existing replays are unaffected; the SNAPSHOT op only fires on explicit `SNAPSHOT` statements. The replay semantics are unambiguous: on `Engine::open`, if a `0x0B` follows the matching `0x0A`, install the saved buffer in place of the INIT spec's re-derivation.

This is one op, one verb, one new test gate. The architectural shape — opt-in durability, declarations-only by default — stays preserved. After it ships, Halcyon's verifier adds `--snapshot` and the chapter's Appendix A.4 promotes from "fire the block and wait" to "hit the cached canonical."

The two questions back at you are at the bottom (Section 7). Same protocol as Part I: spec is a first draft, pushback on every clause is welcome.

—Bee + Claude

---

## 1. Motivation

The Solves Vol. 4 chapter's three-way receipt promises that the same canonical lives in three places:

1. Halcyon's deployed JSON report.
2. GIGI's live engine at the substrate URL.
3. The chapter PDF itself.

Today receipt 2 is *operational* (Appendix A.4 fires the five-statement block and gets the canonical back) but it isn't *addressable*. A reader can't `GET /v1/gauge_field/halcyon_canonical_U/plaquette?reduction=mean` against `gigi-stream.fly.dev` and read the canonical directly — because no thermalized `halcyon_canonical_U` lives long-term in the engine. Every reader thermalizes from scratch.

The architectural cost is that the chapter has to document a recipe ("run this five-statement block") instead of citing a query endpoint. The user-facing cost is the ~30s wall every cite-check pays. The reproducibility cost is that the canonical buffer is never the *same bytes* between cite-checks — every reader gets their own statistically-equivalent draw, which is fine for the science but suboptimal for the citation handle.

What closes the gap is durable thermalized state. Two structurally distinct paths:

- **Snapshot the buffer.** Explicit user action; the buffer becomes a first-class persistable artifact. Matches the existing declarations-only pattern.
- **Replay the GIBBS_SAMPLE statement.** Implicit; every persisted `GIBBS_SAMPLE` survives. Replay re-thermalizes on every cold start, so boot time grows linearly with the number of persisted thermalized fields.

This spec is for the first path. The second path is named in Section 8 (out-of-scope, with the reasons).

## 2. What's already in GIGI (no new work)

Per ``HALCYON_PART_I_GATES.md`` and the Part II/III impl logs:

- **`OP_LATTICE_DECLARE` (`0x09`)** — `engine::declare_lattice_durable` writes; replay re-installs via `lattice_registry::register`. WAL entry carries the lattice payload sized by topology.
- **`OP_GAUGE_FIELD_DECLARE` (`0x0A`)** — `engine::declare_gauge_field_durable` writes; replay re-installs by re-running the INIT spec (IDENTITY / HAAR_RANDOM / FROM_FIELD).
- **The GQL parser already routes `PERSIST` on LATTICE + GAUGE_FIELD declarations to the durable handlers.** Adding `SNAPSHOT` is a third terminal-statement variant; the grammar slot is small.

The constant-time-compared `X-API-Key` middleware applies to every `/v1/gql` POST. The snapshot verb inherits this — the same `state.api_key` gate keeps anonymous writes out of the WAL.

## 2.5  Precondition surfaced during Part V scoping: `/v1/gql` does not dispatch gauge-feature statements

While probing `gigi-stream.fly.dev` to verify the chapter's Appendix A.4 receipt after your `805e0c8` redeploy (image `deployment-01KVG58VXNFQ1E9K1C2H137E3S` with combined `kahler imagine sharded transactions patterns causal_states wish halcyon`), I confirmed that every gauge-feature statement issued to `POST /v1/gql` returns `{"status": "ok"}` with no executor side-effects. Concretely:

- `LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';` → 200, `{"status":"ok"}`. The lattice is **not** in `lattice_registry` afterwards (the subsequent `GET /v1/gauge_field/U/plaquette` returns *"source field is not declared"*).
- `GAUGE_FIELD U ON LATTICE bb GROUP SU(2) INIT IDENTITY;` → same.
- `GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED 20260616;` → 200, `{"status":"ok"}`. No `mean_plaquette` chain in the response.
- `SELECT PLAQUETTE OF U;` → 200, `{"status":"ok"}`. No row data.

The root cause sits in `src/bin/gigi_stream.rs::gql_query`. The `match &stmt {` block has arms only for the bundle-shaped statements (`CreateBundle`, `Insert`, `BatchInsert`, …). Every gauge-feature statement falls into a default early-return at the line that emits `{"status":"ok"}`. The `parser::execute` arms for `Statement::GibbsSample` (line 8586) and `Statement::SymplecticFlow` (line 9388) build proper `ExecResult::Rows` envelopes — but they are never reached from the HTTP layer.

Your route-resolution receipts hold (6 read-only routes resolve with 4xx validation, 3 embedded-only writers correctly 404), but they validate **route layer**, not **data flow**. The HTTP handler dispatches the right shape on the wire and stops there.

This is a precondition for Part V's snapshot verb in two ways:

1. `SNAPSHOT GAUGE_FIELD U;` issued via `/v1/gql` would itself fall into the same default early-return path unless the dispatcher learns about gauge statements first.
2. The snapshot is only useful if a `GIBBS_SAMPLE` (or `SYMPLECTIC_FLOW`) executed beforehand actually mutated state on the engine. Today neither does over HTTP.

### P-1  `/v1/gql` gauge-feature dispatch (precedes P0)

Add a feature-gated match prefix to `gql_query` before the bundle-aware path:

```rust
// src/bin/gigi_stream.rs::gql_query
#[cfg(feature = "gauge")]
match &stmt {
    Statement::Lattice { .. }
    | Statement::GaugeField { .. }
    | Statement::ShowGaugeField { .. }
    | Statement::GibbsSample { .. }
    | Statement::EField { .. }
    | Statement::SymplecticFlow { .. }
    | Statement::ShowEField { .. }
    | Statement::SelectHTotal { .. }
    | Statement::SelectGaussResidualMax { .. }
    // ... and SNAPSHOT once Part V P0.2 lands
    => {
        let result = gigi::parser::execute(&stmt);   // executor already returns ExecResult::Rows
        let dur = t0.elapsed().as_micros() as u64;
        let (status, resp) = match result {
            Ok(r) => exec_result_to_response(r),
            Err(e) => (StatusCode::BAD_REQUEST,
                       Json(serde_json::json!({"error": e}))),
        };
        // ...metrics + emit_quick as usual...
        return (status, resp);
    }
    _ => {}
}
```

The gauge executors do not need a bundle handle (they operate over `gauge_registry` + `lattice_registry`, which are process-global singletons today). So the dispatch is straight from `parser.rs::execute` through `exec_result_to_response` — no bundle resolution at all.

### Receipts for P-1

`tdd_hal_v_0_gql_dispatches_gauge_statements`:

1. `POST /v1/gql LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';` → 200 with the LatticeView Rows envelope.
2. `GET /v1/lattice/bb` → 200 with the same LatticeView (proves the declaration **landed**, not just was acknowledged).
3. `POST /v1/gql GAUGE_FIELD U ON LATTICE bb GROUP SU(2) INIT IDENTITY;` → 200 with the GaugeFieldCreateResponse.
4. `POST /v1/gql GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 10 MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED 20260616;` → 200 with a 10-element `mean_plaquette` Vector column.
5. `POST /v1/gql SELECT PLAQUETTE OF U;` → 200 with the per-face values.

Then Halcyon's existing Phase B test (`test_G_LIVE_B2_production_thermalization_pass_criterion`) passes against `gigi-stream.fly.dev` on the first try.

P-1 size: roughly the same shape as the existing bundle-statement dispatch arm. One match block, one return, one round of metrics. The gauge executor and the response serializer both already exist — this is the wire that connects them.

---

## 3. Sprint asks (P0 → P2)

### P0 — `OP_GAUGE_FIELD_SNAPSHOT` (`0x0B`)

The minimum WAL op that lets a thermalized buffer survive restart.

#### P0.1 WAL op definition

```rust
// src/wal.rs
const OP_GAUGE_FIELD_SNAPSHOT: u8 = 0x0B;

#[derive(Serialize, Deserialize)]
struct GaugeFieldSnapshotPayload {
    /// The declared field's name. Must match a previously-declared
    /// GAUGE_FIELD's name; replay errors loudly if absent.
    name: String,
    /// The group tag — matches the declared field's group. Carried for
    /// validation; replay errors if it disagrees with the declared
    /// field's group (catches a snapshot-against-wrong-field bug).
    group: Group,
    /// Row-major (n_edges, repr_dim) f64 bytes. For SU(2) on the
    /// buckyball: 90 * 4 * 8 = 2880 bytes. Native endianness is fine
    /// IF the WAL is reread on the same architecture; otherwise emit
    /// little-endian.
    buffer: Vec<f64>,
    /// SHA-256 of the buffer bytes, computed before the entry is
    /// written. Replay rejects entries with bad checksums.
    sha256: [u8; 32],
}
```

The op is **append-only on top of** the matching `OP_GAUGE_FIELD_DECLARE`. Replay walks WAL in order: the `0x0A` re-installs the declaration, the subsequent `0x0B` (if any) overwrites the link buffer with the snapshot's bytes.

If a snapshot is logged before its declaration, replay rejects the WAL with a typed `WalError::OrphanedSnapshot(name)` — same shape as other typed replay errors today.

#### P0.2 GQL surface: the `SNAPSHOT` verb

Grammar:

```ebnf
snapshot_stmt
  : "SNAPSHOT" "GAUGE_FIELD" ident
    [ "PERSIST" ]                  // default: PERSIST (the whole point of the verb)
    ";"
  ;
```

Semantics:

- `SNAPSHOT GAUGE_FIELD U;` (no clause) — write to the WAL, ack on response. This is the production path.
- `SNAPSHOT GAUGE_FIELD U PERSIST;` — explicit, identical to the default. Carried for spec-clarity.
- *(possible future)* `SNAPSHOT GAUGE_FIELD U TRANSIENT;` — copy the buffer to an in-memory snapshot slot for fast `RESTORE`, no WAL write. Not in this sprint; named so the parser knows where to grow if it's ever wanted.

#### P0.3 Verb math

The handler:

1. Resolves the field through the dyn surface (`gauge_registry::get`).
2. Validates group erasure (`Group::SU2` only at launch).
3. Copies out the buffer (`handle.as_dense_buffer().to_vec()`).
4. Computes SHA-256 of the buffer bytes.
5. Calls `engine.snapshot_gauge_field_durable(name, group, buffer, sha256)`.
6. Returns a single-row Rows envelope:

```json
{
  "rows": [{
    "field": "U",
    "n_edges": 90,
    "repr_dim": 4,
    "sha256": "<hex>",
    "wal_offset": <u64>
  }]
}
```

The SHA-256 in the response is the same one written to the WAL. It is also the citation handle the Solves Vol. 4 chapter wants — readers can cite the snapshot SHA as the canonical.

### P1 — Replay restoration

`Engine::open` walks the WAL in order. The existing replay loop already handles `0x09` and `0x0A`. Add:

```rust
OP_GAUGE_FIELD_SNAPSHOT => {
    let payload: GaugeFieldSnapshotPayload = decode(entry)?;
    let handle = gauge_registry::get(&payload.name)
        .ok_or(WalError::OrphanedSnapshot(payload.name.clone()))?;
    if handle.group() != payload.group {
        return Err(WalError::SnapshotGroupMismatch { … });
    }
    if !verify_sha256(&payload.buffer, &payload.sha256) {
        return Err(WalError::SnapshotChecksumMismatch { name: payload.name });
    }
    handle.replace_buffer(payload.buffer);
}
```

`replace_buffer` is the new internal method on the dyn surface. For `SU2GaugeField` it's a single buffer copy. The replay is idempotent — if multiple `0x0B` ops exist for the same field, the last one wins (the WAL is the source of truth; latest write is current state).

### P2 — Test gates

Three TDD gates, mirroring the Part II receipts:

#### P2.1 — `tdd_hal_v_1_snapshot_writes_and_replays`

Declare → `INIT IDENTITY` → `GIBBS_SAMPLE` thermalization → `SNAPSHOT` → close engine → reopen → read buffer → assert byte-identical to pre-close buffer.

#### P2.2 — `tdd_hal_v_2_snapshot_checksum_rejection`

Manually corrupt the SHA-256 in a WAL entry. Replay must reject with `WalError::SnapshotChecksumMismatch`.

#### P2.3 — `tdd_hal_v_3_snapshot_orphan_rejection`

Manually delete the `OP_GAUGE_FIELD_DECLARE` entry preceding an `OP_GAUGE_FIELD_SNAPSHOT`. Replay must reject with `WalError::OrphanedSnapshot`.

(P2.4 — gauge-leak prevention.) Out of scope; the SU(2)-only group guard already covers this.

---

## 4. What Halcyon gets back

Once Part V ships, Halcyon's `verify_canonical_receipt.py` adds a single flag:

```bash
python -m inertia_damping.scripts.verify_canonical_receipt \
    --base-url https://gigi-stream.fly.dev \
    --api-key $GIGI_API_KEY \
    --snapshot
```

Adding `--snapshot` appends a single statement after the thermalization:

```sql
SNAPSHOT GAUGE_FIELD halcyon_canonical_U;
```

After it runs once, the canonical lives in the production WAL. Subsequent verification calls skip thermalization and just read the cached buffer:

```bash
GET /v1/gauge_field/halcyon_canonical_U/plaquette?reduction=mean
→ {"reduction": "mean", "value": 0.5068472...}
```

The chapter's Appendix A.4 promotes from a 30-second receipt to a sub-100ms receipt. The SHA-256 in the snapshot response becomes the canonical citation handle.

For the test scaffold: Halcyon adds a Phase E test (`test_gigi_live_phase_e_snapshot.py`) that snapshots, simulates a restart via re-introspection round-trip, and asserts byte-identity. One new gate, opt-in like all the live tests.

## 5. Reproducibility and bit-identity contract

Same shape as ``HALCYON_TO_GIGI_REPLY_2026-06-17.md`` § A2:

- **Snapshot at time T₁ ↔ replay at time T₂** in the same process → byte-identical buffer.
- **Snapshot ↔ replay across processes on the same OS / same BLAS** → byte-identical.
- **Cross-OS** → up to 2 ULPs in trig reductions if the snapshot was produced under different FMA / SIMD lane order. Documented, not enforced — same caveat as A2.
- **The snapshot's SHA-256 is the citation handle.** A reader can cite *that specific 256-bit hash* and any independent verifier can confirm.

Note: the SHA-256 is over the **buffer bytes**, not the GQL statement that produced them. Two different `GIBBS_SAMPLE` invocations producing the same end state (theoretically possible if seeds align) produce the same SHA-256. This is correct — the citation is about the state, not the path that reached it.

## 6. Out-of-scope, deliberately

- **`SNAPSHOT E_FIELD`** — same shape will be wanted for symplectic-flow trajectory caching, but defer to a follow-up sprint. Halcyon's current public-verifier doesn't need it; Part V ships gauge-field snapshots first.
- **`OP_GIBBS_SAMPLE`** — the alternative path (WAL-log the GQL statement, replay re-thermalizes). Tracked here so the choice is named; not requested because (a) replay cost grows linearly with thermalization length, (b) snapshot is the cleaner architectural fit with declarations-only persistence.
- **Compression of the snapshot buffer.** For SU(2) on the buckyball it's 2,880 bytes — irrelevant. If larger lattices land later (10⁴ or 10⁵ edges) it becomes a real concern; orthogonal to this sprint.
- **A `RESTORE GAUGE_FIELD U FROM SNAPSHOT <sha>;` verb.** Powerful and almost trivial once Part V ships (the buffer is already addressable by SHA in the WAL), but not on the critical path; tracked for the next sprint after this one.
- **Multi-engine federation of snapshots** (e.g., snapshot on one node, replay on another). Not relevant; gigi-stream is a single-engine deploy today.

## 7. Open questions back at you

1. **Endianness in the WAL payload.** Native or explicit little-endian? Fly.io's `gigi-stream` runs x86_64 Linux today, so native works, but if the WAL ever needs to be readable on ARM (cross-arch tests, future Macs, etc.) explicit LE is the right call. My suggestion: little-endian, documented at the WAL op site.
2. **`SNAPSHOT` over the HTTP write surface, or `/v1/gql` only?** The locked decision D5 from Part III put `GIBBS_SAMPLE` on `/v1/gql` only (mutation is *not* on the REST write surface). I'd suggest `SNAPSHOT` follow the same precedent: only addressable via `/v1/gql`, no `POST /v1/gauge_field/{name}/snapshot` route. Consumer code calls `/v1/gql` for any state-changing op; REST routes stay read-only. If you'd rather have a dedicated route for snapshot, name it now so the chapter cites consistently.
3. **What's the right place for the SHA-256 in the verifier's response?** I had it land in the `verify_canonical_receipt.py` JSON output under `snapshot_sha256`, alongside the existing `P_chain_sha256`. Two different SHAs — one for the measurement chain (what the chapter currently cites), one for the post-snapshot buffer (what the WAL records). The verifier surfaces both; the chapter cites whichever is the canonical handle of record. I'd suggest the snapshot SHA for the chapter, since it's also the WAL replay receipt. Open to either.

---

## 8. Targets (what this unblocks)

- **`papers/solves_vol4_ym_mass_gap.tex` Appendix A.4** — the public-receipt verifier promotes from a 30-second thermalization to a sub-100ms cached read against the production substrate.
- **The citation handle.** The chapter currently cites the Halcyon JSON's commit hash + the GIGI deploy hash. With Part V, the canonical adds a third hash — the snapshot SHA-256 of the thermalized buffer itself. That's a state-level fingerprint independent of code commits or deploys, and is the strongest publicly-citable receipt the architecture can offer.
- **Marcella's gauge-corpus query pattern** — already named as read-only in the Part II audit. With persistent thermalized fields, her geometric channel can query a stable corpus instead of triggering thermalization on every read. Off-critical-path but real.

---

**Pushback on every clause is invited.** If `OP_GAUGE_FIELD_SNAPSHOT` is the wrong shape and you'd rather ship `OP_GIBBS_SAMPLE` + statement-replay, say so — the snapshot path is what the chapter wants but the engineering decision is yours. If the SHA-256 should be over the `(declaration + buffer)` pair rather than the buffer alone, that's a real call. If the `PERSIST` clause should be required rather than default, that's also a real call.

—Bee + Claude
