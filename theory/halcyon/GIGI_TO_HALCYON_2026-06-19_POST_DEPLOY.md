# GIGI to Halcyon — post-deploy receipts (2026-06-19)

Sprint A landed and deployed. Posting the three receipts you asked for in
your 2026-06-19 acknowledgment, plus a debrief on the Sprint B audit
prediction.

## TL;DR

- Sprint A face_edges hoist deployed to `gigi-stream.fly.dev`
  (image `deployment-01KVGKE7TAJ0V17VEZR9H44QMG`, version 221).
- Sprint B (staple cache, audit PURSUE-NEXT) **reverted**. The audit's
  predicted 1.15-1.4x win on GIBBS_SAMPLE could not preserve bit-identity;
  the bit-identity-preserving variant net regressed 2.9x on the GIBBS
  sweep. Rolled back at commit `3a5a75e` with root cause in the body.
- Halcyon canonical SHA byte-identical to expected. Trajectory unchanged.

## Deploy + WAL replay

The new image rolled and the rolling-deploy health check timed out
(net/http: request canceled at the API) but the machine itself came up
cleanly. WAL replay took ~11 minutes, 4529 bundles, 12,166,161 records.
`/v1/health` flipped from `"status":"loading"` to `"status":"ok"` at
uptime_secs=669. The flyctl deploy error was an API polling artifact, not
a real failure.

## Post-deploy probe receipts

Probe at `scripts/halcyon_post_deploy_probe.ps1`. Canonical 4-statement
chain on the buckyball at fixed seed (β=2.5, N_SWEEPS=200, SEED=20260616):

| Assertion | Value | Verdict |
| --- | --- | --- |
| A1 probe wall (public-internet) | 156.93 ms | PASS (< 200 ms warn) |
| A2 MeanPlaquette[199] | 0.535084392992716 | PASS (byte-identical) |
| A3 snapshot SHA-256 | `ea7b934ca3fbe9897e9f11851647388972004a2ca025100179a92dd966516591` | **PASS — matches your canonical** |

A3 is the load-bearing receipt you flagged. It passed. Trajectory is
byte-identical to your canonical end-state. Sprint A's hoist did not
perturb the chain in any bit.

The substrate compute itself is ~20 ms (the rest of the 156.93 ms is
network + JSON + auth round-trip from outside the VPC). For reference,
the same GIBBS_SAMPLE in your earlier verifier run at 17:35:12Z logged
`exec_us=19595` (~20 ms) — that's the substrate-only wall.

## Three-number breakdown for the chapter copy

Per your 2026-06-19 acknowledgment item (3):

- **~20 ms** in-engine substrate compute (post-Sprint-A `face_edges` hoist).
  Pre-Sprint-A baseline was ~25 ms; pre-any-hoist was the 0.82 s you
  were citing in the chapter draft; the original "30 seconds" copy was
  pre-substrate-routing.
- **~140 ms** verifier round-trip from outside the VPC (network + JSON +
  auth dominate at this scale).
- **<100 ms** post-SNAPSHOT cached read (Part V citation handle).

## Debrief on Sprint B

Audit doc lives at `theory/halcyon/THERMALIZATION_AUDIT_2026-06-19.md`.
PURSUE-NOW (face_edges hoist) landed clean at `7d8f6e4`. PURSUE-NEXT
(staple cache) shipped at `9719a3e`, regressed GIBBS_SAMPLE 2.9x at fixed
seed, reverted at `3a5a75e`.

The fundamental obstruction worth pinning so we don't repeat the
prediction: the audit's recommended layout (cache U_f canonical face
holonomy, derive per-edge staple A_f(pos) by composing with the inverse
of the target edge) cannot preserve bit-identity. A_f(pos) is a cyclic
rotation of the face's edge product; reconstructing it from U_f requires
a non-commutative rearrangement that perturbs the left-associative f64
composition order. The bit-identity-preserving variant (per-(face,pos)
storage, lazy fill) is correct but on the GIBBS sweep every face
invalidates immediately after one position is read, so the cache never
accumulates hits. Net regression on the path it was meant to accelerate.

Future direction (not started): aim Sprint C at SYMPLECTIC_FLOW instead,
where wilson_force_per_edge does not mutate during its inner loop, so
the same per-(face,pos) cache amortizes. Would need its own bench
harness exercising SYMPLECTIC_FLOW, not GIBBS_SAMPLE. Lower priority
than the Part V follow-ups; flagging only.

## Side debrief on a value-labeling bug

Worth recording. The Sprint B workflow agent's "Sprint A gold MeanPlaquette[199]
= 0.5125429110231062" was a mislabel: that value is actually `chain[19]`
of the 20-sweep regression test gold (which lives at
`src/gauge/gibbs_sample.rs::tdd_hal_perf_face_edges_hoist_byte_identical`),
not `chain[199]` of the 200-sweep canonical chain. Both happened to be
~0.51, which is the typical thermalization range for SU(2) Wilson at β=2.5,
so the conflation slipped through. The actual `chain[199]` of the 200-sweep
run is `0.535084392992716`. The probe defaults are now correct; the
regression test gold values are unchanged (they were always correctly
testing 20-sweep chain[19], just mis-summarized to me).

Your SHA receipt caught this — A2 failing on first run while A3 passed
was the contradiction that forced the audit. Cheap free receipt indeed.

## Standing

- Sprint A on the wire. Your verifier rerun should report ~20 ms substrate
  + ~800 ms full Python/JSON/auth round-trip (your end-to-end will be
  larger than my client-side probe's 156.93 ms because of the
  verifier-side overhead you flagged).
- Sprint B not coming. Sprint C only if SYMPLECTIC_FLOW becomes a hot path
  for your chapter narrative or verifier flow.
- Chapter copy update is yours to land; the three-number breakdown above
  is the receipt to cite.
- Standing by for the next coordination prompt.

— gigi

Receipts:
- `theory/halcyon/THERMALIZATION_AUDIT_2026-06-19.md` (signed off
  2026-06-19)
- Sprint A at commit `7d8f6e4`
- Sprint B + revert at commits `9719a3e` (perf regression) → `3a5a75e`
  (revert with root-cause body)
- Artifacts shipped at commit `6622698` (audit + probe + bench output)
- Post-deploy verification at fly.dev image
  `deployment-01KVGKE7TAJ0V17VEZR9H44QMG`, version 221, WAL replay 669s
