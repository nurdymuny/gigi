# Reply to SCJ — 2026-06-07

> *"Honest arithmetic on both sides keeps the geodesic honest."*
> — SCJ, 2026-06-07, §9.

To: SCJ ingest team
From: Gigi engine team · Davis Geometric
Re: Your 2026-06-07 reply
Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06 → SCJ 2026-06-07 → this.

Caught us back on the 4-row partition — and rightly. Three reservations at 0.005 sum to 0.015, not 0.020; we'd written one number in the table and a different number in the narrative. **The four-row partition with `r_residual = 0.010` is the right correction**, and it's a structural improvement on what we proposed: naming residual is what absorbs the unmodeled correlations that *will* exist whether or not we enumerate them. The partition is the v0.1 contract. Ack.

Short reply mirroring yours. Eight items.

---

## §1. The 4-row partition — ack, with one tightening.

The corrected table is the contract:

| Term | Bound (observed worst-case) | Reservation (committed headroom) |
|---|---|---|
| `E_backend` | ≤ 1e-4 | `r_backend = 0.005` |
| `E_recall` | ≤ 0.05 (1 − recall@200 ≥ 0.95) | `r_recall = 0.005` |
| `E_holonomy` | TBD (§10 PR gate; ±1% planted phase) | `r_holonomy = 0.005` |
| `r_residual` | unmodeled correlations | `0.010` |
| **Sum reserved** | | **0.025** |
| Vacuity slack | | `0.005` |
| **δ_indep target** | | **0.030** |

Sum: 0.005 + 0.005 + 0.005 + 0.010 + 0.005 = 0.030. ✓

**One tightening:** the engine surfaces these as **named constants in `src/spectral.rs`** rather than scattered comments, with a unit test that asserts the partition sums to the target. If anyone touches a reservation in code, the test fails until both teams ack the change. That's the substrate-side enforcement of the contract; your side enforces it via `V01_TACTICAL_CHECKLIST.md` C2. Both sides green = the contract holds.

Constants now live at:
- `gigi/src/spectral.rs::R_BACKEND_SLACK = 0.005`
- `gigi/src/spectral.rs::R_RECALL_SLACK = 0.005`
- `gigi/src/spectral.rs::R_HOLONOMY_SLACK = 0.005`
- `gigi/src/spectral.rs::R_RESIDUAL_SLACK = 0.010`
- `gigi/src/spectral.rs::DELTA_INDEP_VACUITY_SLACK = 0.005`
- `gigi/src/spectral.rs::DELTA_INDEP_TARGET = 0.030`

Plus unit test `delta_indep_partition_sums_to_target()` in the same module.

Two zero-impact pre-existing constants stay alongside as bounds (not reservations): `E_BACKEND_BOUND_SPECTRAL = 1.0e-4` and `E_RECALL_BOUND_HNSW = 0.05`. **Bound and reservation now have distinct names everywhere they appear.** Renaming the old constants to make the bound/reservation distinction visible at every callsite.

Patch in next commit on `main` (and cherry-picked onto `scj-v0.1-substrate`).

---

## §2. `[]` empty sentinel + `TOP 200` truncate-to-50 + recall_mode + schema-hash-mismatch — ack.

All four landed:

- **`[]` sentinel.** Confirmed. NULL = field absent, `[]` = field present and empty. Contract tests assert this distinction.
- **`TOP 200` engine / `[:50]` hunt-side truncate.** Right design — the post-filter robustness argument is airtight, and recall@200 is the operationally useful signal. Engine contract: surface 200 by default for `SIMILAR`. `SIMILAR EXACT` ignores TOP per our prior agreement.
- **`recall_mode: "approximate" | "exact"`** in the result envelope. Hunt scripts can never accidentally cross-contaminate.
- **`USING SCHEMA <name>` rejects on schema-hash mismatch.** Loud failure is the only correct mode. We'll surface the expected vs actual hash in the error body so reviewers can tell whether the spec drifted or the engine drifted.

---

## §3. Calibration sweep restatement — confirmed clean.

The corrected statement reconciles. Steady-state ~50 ms/query warm-cache + ~30 s cold-cache first-query + ~1 minute wall-clock single-threaded on the workstation-class box. Pinning the bench against the corrected number in `tests/test_calibration_sweep_perf.py` when 2A lands.

One implementation note: the bench should include both cold-cache and warm-cache passes on the same run, reporting both numbers. A 50% page-cache hit rate is realistic in a real analyst's monthly workflow; the bench should reflect that distribution rather than report only steady-state.

---

## §4. HNSW machine spec — confirmed; one substrate note.

The cardinality breakdown is exactly what we expected — `ntoskrnl.exe` at ~30K is the ceiling, everything else is well inside. The ~6 s wall-clock for the worst-case 20-shard Patch Tuesday delta is plausible. Bench commitment same as §3.

One substrate note: HNSW is built per-bundle today, not per-shard. When sharding lights up the single-field vector path (Ask C, downstream of 2A), each shard rebuilds its local HNSW independently. The wall-clock projection above assumes that's how it lands; if the per-shard graph topology turns out to need a cross-shard gluing step (we don't think it will, but the seam isn't fully closed), the projection re-derives. We'll surface it before it bites.

---

## §5. OOD tripwire + "minimum across spectral clusters" — accepted, with one observation.

The four-band rule lands as the contract. The minimum-across-clusters refinement is **the right call** — a single bad cluster with 5 OOM gap is a hole in the embedding even if the global median is 184 OOM. Stronger than the global-median tripwire we proposed; pin yours.

One observation that may matter for the bench: a single small cluster (say n=12 functions in some weird outlier corner) can have noisy OOM measurements just from sample size. Suggest the tripwire fires on `min(gap_by_cluster | cluster_size >= 50)` to prevent a tiny-but-coherent cluster from looking broken when it's just noisy. Take or leave; we don't have your cluster-size distribution yet.

---

## §6. Spec bug fix follow-ups — three sweep results acked.

**6.1 — 4A sink encoding.** `reaches_unknown_sink BOOLEAN INDEX` + reserved vocabulary position 0 for `"_unknown"` in v0.2 — the right shape. IAT thunk confirmation noted. The MATCHES sweep coming back clean means we can close the pattern. ✓

**6.2 — 4B (module, rva) sweep — two more genuine bugs caught.** Good catches. The §4.6 `pre_*` field issue is exactly the same shape as the DIFF — the Option-(b) restriction does cover both, but as you note it deserves an explicit call-out in the new §4.6 text. The §4.8 PHASE-along-ingest_date catch is the subtler one: temporal series accidentally stitching together two different functions is a class of bug that's *invisible* in the result — there's no error to throw, just a wrong number. The footnote + within-version scope restriction is the right v0.1 response. When `BY identity_hash` ships in v0.2, lift the restriction.

Confirming §4.3 PULLBACK queries are within-snapshot joins — RVA stability is automatic there, no patch needed. ✓

**6.3 — 4C λ_max sweep — clean.** Residual + Lanczos + safety floor + warnings + 4-gate A/B contract all accepted. The two soft cases you named (`boundary_hops RANGE 16`, `cve_count RANGE 10000`) are exactly the right kind of "empirical-not-derived" placeholders — defensible at this stage, refinable as data lands. Don't patch yet.

---

## §7. Consumer council — accepted, with the convening mechanism named.

The "consumer council" framing is sharper than what we said. You're right that with three consumers under one umbrella we don't need a formal mechanism *yet*. But the convening discipline is worth naming now so it's not retrofitted later:

> **Substrate breaking changes require simultaneous re-pin agreement from every affected consumer. The substrate team convenes the council; consumers vote with their pinned tags.**

For Marcella + SCJ + KRAKEN-next, "the council" is two letters and a `git rebase`. For a fourth consumer (especially external), the council becomes a real coordination artifact — likely a `theory/substrate_council/` directory with one folder per breaking change, capturing the proposal + consumer responses + outcome.

Until then, the in-band discipline is: every PR on `main` runs all pinned-consumer-tag contract suites in CI; a red consumer freezes the merge until the proposal is amended or the consumer cuts a new tag.

---

## §8. Close.

Four corrections across four letters (yours: 17× sweep, target slip; ours: table arithmetic, 600× sweep-per-query). All four converged before either side committed contract bugs to the substrate or the ingest. That's what the bound/reservation discipline buys us — the slack absorbs unmodeled error, but only when both teams *do the arithmetic*.

Two corrections each way means the channel works. Three consumers downstream of one bundle means the geodesic is now a shared constraint, not a per-pair contract.

Geometry, not gravity.

— Gigi engine team · Davis Geometric · 2026-06-07
   Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06 → SCJ 2026-06-07 → Gigi 2026-06-07.

---

## Appendix — substrate-side patches landing alongside this letter

1. `gigi/src/spectral.rs` — refactor: rename `E_*_SLACK_*` constants to `E_*_BOUND_*` (the bounds); add `R_*_SLACK` reservation constants; add `DELTA_INDEP_TARGET` and `DELTA_INDEP_VACUITY_SLACK`; add `delta_indep_partition_sums_to_target()` unit test asserting the partition closes.
2. `gigi/theory/scj/REPLY_FROM_SCJ_2026-06-07.md` — your reply mirrored in-tree for posterity (this side; ignored on yours per .gitignore policy).
3. `gigi/theory/scj/REPLY_TO_REPLY_2_2026-06-07.md` — this letter.

Cherry-picked onto `scj-v0.1-substrate` once they land on `main`. Both refs stay in lockstep until 2A lands and the contract tests turn live.

— end —
