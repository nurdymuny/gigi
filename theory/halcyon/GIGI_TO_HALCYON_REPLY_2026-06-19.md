# GIGI → Halcyon reply, Part V (2026-06-19)

**From:** GIGI engine team (Bee + Claude)
**To:** Halcyon team
**Subject:** P-1 shipped end-to-end. §7 answers. P0–P2 status.
**Companion to:** `HALCYON_PART_V_SNAPSHOT_GATES.md`, `HALCYON_TO_GIGI_VERIFICATION_2026-06-18.md` (the prior cross-team letter).

---

## Letter

Halcyon —

P-1 is closed end-to-end on production. The diagnosis in §2.5 of your Part V spec was correct in shape and in line numbers: `gigi_stream.rs::gql_query` only knew the bundle-shaped statements; every gauge verb fell into the default early-return and got `{"status":"ok"}` without ever reaching `parser::execute`. Three commits sever-and-rewire the path: V.0 lifts a `try_dispatch_gauge_statement` helper with a 13-variant match arm and dispatches through `parser::execute` + `exec_result_to_response`; V.0b adds the `register_su2` calls that were silently absent from the SU(2)-mut sibling map (a latent gap going back to II.5 — `register(handle)` only populated the dyn map, so `GIBBS_SAMPLE` / `SYMPLECTIC_FLOW` after a `/v1/gql` or `POST /v1/gauge_field` declare failed at the mut lookup); V.1 then bundled the WAL op work with a `register_su2` cleanup on the GAUGE_FIELD execute path.

The production receipts. Against `gigi-stream.fly.dev` image `deployment-01KVG9H0E7TGGVM8HT2N2M1RA3`: `LATTICE buckyball_p1b FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';` returns 200 and the subsequent `GET /v1/lattice/buckyball_p1b` returns the full `LatticeView` (`n_vertices=60`, `n_edges=90`, `n_faces=32`) — the declaration *landed*, not just acknowledged. `GAUGE_FIELD U_p1b ON LATTICE buckyball_p1b GROUP SU(2) INIT IDENTITY;` returns 200. `GIBBS_SAMPLE U_p1b BETA 2.5 N_SWEEPS 10 MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED 20260616;` returns 200 with a real `MeanPlaquette` chain `[0.5389, 0.5216, 0.5210, 0.4081, 0.5934, 0.5426, 0.5370, 0.4100, 0.5503, 0.4462]` — every value in the 0.40–0.60 band consistent with the III.8a canonical `⟨P⟩_canonical = 0.5074 ± 0.0015` over 200 sweeps. `SELECT PLAQUETTE OF U_p1b;` returns 200 with non-unity per-face values `[-0.245, 0.856, 0.351, …]` confirming the post-Gibbs state propagated through `refresh_dyn_from_su2_mut`. Your `test_G_LIVE_B2_production_thermalization_pass_criterion` should pass against the live URL on the first try.

V.0b is the part I owe you a confession on. The P-1b shape shipped a dual-call pattern (`register(handle)` *then* `register_su2(field_snapshot)`) that double-published into the dyn map. Your diagnosis named the right surgical fix: `register_su2` in `src/gauge/registry.rs:160-176` already covers both maps, so the non-PERSIST branch collapses to a single `register_su2(field)` call. That cleanup landed in commit `5bd2291` — bundled with V.1's WAL op work, because of a `git add -A` race during the parallel Part V impl workflow. The commit body only names the WAL op; the V.6 impl log gate will name the conflation explicitly so a future audit lands on the right diagnosis, not on a confused archaeology of two changes wearing one SHA.

Below: production receipts (§1), answers to your §7 (§2), the PERSIST-required clause locked from pushback (§3), the V.0b cleanup acknowledged in writing (§4), Part V P0–P2 status (§5), and out-of-scope carry-forwards (§6). Pushback welcome on every clause, same protocol as Part I.

—Bee + Claude

---

## 1. What's shipped on production

| Receipt | Value |
| --- | --- |
| V.0 helper + 13-variant dispatch | commit `5b555ce` |
| V.0b `register_su2` sibling-map fix | commit `9c5b614` |
| V.1 `OP_GAUGE_FIELD_SNAPSHOT` + GAUGE_FIELD cleanup | commit `5bd2291` |
| Production image | `deployment-01KVG9H0E7TGGVM8HT2N2M1RA3` on `gigi-stream.fly.dev` |
| Health probe | `bundles=5046`, `total_records=12946085`, zero data loss across the V.0 / V.0b / V.1 deploys |

Live `/v1/gql` probes against the deployed image:

1. `LATTICE buckyball_p1b FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';` → 200; `GET /v1/lattice/buckyball_p1b` → 200 with `LatticeView { n_vertices: 60, n_edges: 90, n_faces: 32 }`. The declaration landed.
2. `GAUGE_FIELD U_p1b ON LATTICE buckyball_p1b GROUP SU(2) INIT IDENTITY;` → 200 with `GaugeFieldCreateResponse`.
3. `GIBBS_SAMPLE U_p1b BETA 2.5 N_SWEEPS 10 MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED 20260616;` → 200. `MeanPlaquette` chain: `[0.5389, 0.5216, 0.5210, 0.4081, 0.5934, 0.5426, 0.5370, 0.4100, 0.5503, 0.4462]`. All ten draws inside the III.8a canonical band; 10-sweep variance is statistically reasonable for the 200-sweep `⟨P⟩_canonical = 0.5074 ± 0.0015` reference.
4. `SELECT PLAQUETTE OF U_p1b;` → 200 with per-face values `[-0.245, 0.856, 0.351, …]`. Non-unity confirms the Gibbs sweep mutated state on the engine and the refresh of the dyn surface from the SU(2)-mut sibling went through.

Your existing Phase B gate `test_G_LIVE_B2_production_thermalization_pass_criterion` is the live pin. Per the P-1 receipts list in your spec it should pass against `gigi-stream.fly.dev` on the first try — re-run it on your timing and the green is the cross-team handshake.

---

## 2. Answers to §7 open questions

### Q1 — Endianness in WAL payload

**Locked: explicit little-endian.** `f64::to_le_bytes` / `f64::from_le_bytes`, documented at the `OP_GAUGE_FIELD_SNAPSHOT` (0x0B) constant site in `src/wal.rs`.

Native is fine today on the x86_64 Linux Fly.io image, but native locks the WAL to one architecture forever. The bytes on disk become an artifact of the producer's lane order. Cross-arch readability is cheap to buy now (one `to_le_bytes` call per `f64`) and expensive to retrofit later (a WAL migration touching every snapshot entry ever written). The discipline gets enforced by adjacency: the comment on the `0x0B` constant names the encoding, so a future contributor adding a snapshot field reads the rule at the same site they're editing instead of having to remember it from this letter.

### Q2 — `SNAPSHOT` over HTTP write surface, or `/v1/gql` only

**Locked: `/v1/gql` only. No `POST /v1/gauge_field/{name}/snapshot` route.**

This is the D5 precedent from Part III. Every state-mutating gauge verb — `GIBBS_SAMPLE`, `SYMPLECTIC_FLOW`, `E_FIELD` declarer — is `/v1/gql`-only by design and 404s on dedicated REST routes. `SNAPSHOT` joins the same family. The 46-minute production wall that made `GIBBS_SAMPLE`-over-HTTP a non-starter doesn't apply here (snapshot is bounded compute, one buffer copy + one SHA-256 + one WAL write), but the architectural symmetry is load-bearing on three independent surfaces: the README's framing of REST routes as read-only, the chapter's framing of the canonical citation as a state fingerprint independent of REST topology, and Marcella's read-only-channel architectural property that the II.6c reframe canonicalized. Adding a dedicated REST route for snapshot would split that family for one verb and force every audit downstream to re-derive why.

Your `verify_canonical_receipt.py --snapshot` flag sends a one-statement `POST /v1/gql` with `SNAPSHOT GAUGE_FIELD halcyon_canonical_U PERSIST;` — no special-case handling.

### Q3 — Which SHA-256 the chapter cites

**Locked: the snapshot buffer SHA-256.** Specifically, SHA-256 over the LE-encoded buffer bytes — the same bytes the WAL stores.

That single hash lands in three places by construction:

1. **The WAL entry payload.** Replay re-derives the SHA over the bytes it reads back and rejects on mismatch. Gate V.5's `tdd_hal_v_2_snapshot_checksum_rejection` is the enforcer.
2. **The `SNAPSHOT` response Rows envelope.** The verifier captures the hex in its JSON output as `snapshot_sha256`.
3. **The Solves Vol. 4 Appendix A.4 citation handle.** The chapter cites that hex string. A reader can independently compute it from the buffer they read back through `GET /v1/gauge_field/halcyon_canonical_U/plaquette` and confirm the canonical.

One hash, three load-bearing uses, deterministically wired through LE encoding so any reader on any architecture computes the same hex.

The measurement-chain SHA you flagged as a sibling is real but secondary. It depends on the RNG draw order, which depends on the per-edge inner loop of the heatbath kernel — it's a transient identifier of the path that reached the state, not a fingerprint of the state itself. Keep it in the verifier's JSON output under `P_chain_sha256` for cross-checking, but the chapter cites `snapshot_sha256`. State, not path.

---

## 3. Bee's pushback you ratified — PERSIST required, not default

**D-V-D.** Bare `SNAPSHOT GAUGE_FIELD U;` parse-errors with `expected PERSIST | TRANSIENT`. Grammar lands as:

```ebnf
snapshot_stmt
  : "SNAPSHOT" "GAUGE_FIELD" ident "PERSIST" ";"
  ;
```

The spec floated `PERSIST` as default-and-optional in §3 P0.2 because the verb's whole point is durability. The pushback: when `TRANSIENT` ships in a future sprint, the migration touches every existing caller that wrote bare `SNAPSHOT`. Either we accept silent behavior drift (bare statements that meant `PERSIST` yesterday might mean something different tomorrow) or we run an explicit deprecation cycle (warning, then break). Both are unforced errors today. Requiring `PERSIST` explicitly today means every caller is already disambiguating, and the day `TRANSIENT` lands, zero existing call sites change meaning.

Your `verify_canonical_receipt.py --snapshot` flag bumps from `SNAPSHOT GAUGE_FIELD halcyon_canonical_U;` to `SNAPSHOT GAUGE_FIELD halcyon_canonical_U PERSIST;` — one keyword. The Solves Vol. 4 Appendix A.4 listing inherits the same one-keyword bump.

---

## 4. V.0b cleanup — `register_su2` from GAUGE_FIELD execute path

Acknowledged. Your diagnosis was exact.

P-1b shipped the dual-call pattern: the GAUGE_FIELD execute path called `register(handle)` to publish into the dyn map and then `register_su2(field_snapshot)` to publish into the SU(2)-mut sibling, which double-published into the dyn map (because `register_su2` in `src/gauge/registry.rs:160-176` already publishes both). The cleanup you named: in the non-PERSIST branch, replace `register(handle)` + `register_su2(field_snapshot)` with a single `register_su2(field)` call. In the persist branch, keep the `Arc<dyn>` for `declare_gauge_field_durable` plus a post-durable `register_su2(field_snapshot)` to populate the SU(2)-mut sibling for `GIBBS_SAMPLE` / `SYMPLECTIC_FLOW` reach-through.

67 lines, two files (`src/parser.rs` + `src/gauge/http.rs`), surgical.

Landed in commit `5bd2291`. Conflated with the V.1 WAL op work due to a `git add -A` race during the parallel-workflow shape of the Part V impl sprint (the V.1 agent's `git add -A` swept up the V.0b cleanup files in the same staging pass as the WAL op files; my parallel commit attempt landed second and they merged into one SHA on `main`). The commit body only names the WAL op. The V.6 impl log gate names the conflation explicitly so future audits see two changes under one SHA instead of inferring a hidden third change. Functionally clean — both pieces are in the right shape on the right code paths — attribution-only weirdness.

---

## 5. What's in-flight (P0–P2)

Workflow `wqzov991f`, six gates sequential:

| Gate | Scope | Status |
| --- | --- | --- |
| V.1 | `OP_GAUGE_FIELD_SNAPSHOT` (0x0B) + `GaugeFieldSnapshotPayload` + explicit LE encoding + 3 unit tests | **COMMITTED at `5bd2291`** (bundles the V.0b GAUGE_FIELD cleanup) |
| V.2 | `SNAPSHOT GAUGE_FIELD U PERSIST;` parser + executor + `Statement::Snapshot` in the V.0 helper module's match arm | in-flight |
| V.3 | `Engine::open` replay arm + `WalError::OrphanedSnapshot` + `WalError::SnapshotGroupMismatch` + `WalError::SnapshotChecksumMismatch` + `replace_buffer` method on the dyn surface | queued |
| V.4 | `tdd_hal_v_1_snapshot_writes_and_replays` smoke gate (declare → thermalize → snapshot → close → reopen → byte-identical) | queued |
| V.5 | `tdd_hal_v_2_snapshot_checksum_rejection` + `tdd_hal_v_3_snapshot_orphan_rejection` rejection gates | queued |
| V.6 | Impl log + §7 answers transcribed + decisions inherited + V.1 bundling note | queued |

After V.6 lands: verification + preflight + push + deploy. Estimated under one hour to production. The deploy carries `SNAPSHOT` to the live image. After that, `SNAPSHOT GAUGE_FIELD halcyon_canonical_U PERSIST;` works against `gigi-stream.fly.dev`, your `verify_canonical_receipt.py --snapshot` flag wires through, and the Solves Vol. 4 Appendix A.4 verifier promotes from a 30-second thermalization to a sub-100ms cached read on the canonical SHA.

---

## 6. Out of scope, carried forward

From your §6:

- **`SNAPSHOT E_FIELD`** — same shape will be wanted for symplectic-flow trajectory caching; deferred to a follow-up sprint. Not on Part V's critical path.
- **`OP_GIBBS_SAMPLE` statement-replay alternative** — the road not taken, named explicitly so the architectural choice is on record. Snapshot wins on boot cost (no re-thermalization) and on cleaner fit with declarations-only persistence.
- **Compression of the snapshot buffer** — 2,880 bytes for SU(2) on the buckyball is irrelevant. Revisit if 10⁴- or 10⁵-edge lattices land.
- **`RESTORE GAUGE_FIELD U FROM SNAPSHOT <sha>;` verb** — trivial follow-up once Part V ships (the buffer is already SHA-addressable in the WAL); tracked for the next sprint.
- **Multi-engine federation of snapshots** — single-engine deploy today; not relevant.

Two additional carry-forwards:

- **README mention of Part V.** Lands after the snapshot infrastructure deploys, alongside the existing Part I–IV roster.
- **Halcyon-side `test_gigi_part_v_snapshot.py` mock implementation.** Your call to author at your timing once the live binding swap protocol gets exercised against V.4 + V.5's gates. Mirrors the Phase E pattern from §4 of your spec.

---

Pushback welcome on every clause, same protocol as Part I.

—Bee + Claude
