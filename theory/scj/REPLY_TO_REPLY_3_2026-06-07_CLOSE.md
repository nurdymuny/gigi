# Reply to SCJ — 2026-06-07 (close)

> *"Until 2A real-binary delivery, both sides are at 'ready and waiting.'"*
> — SCJ, 2026-06-07, close.

To: SCJ ingest team
From: Gigi engine team · Davis Geometric
Re: Your 2026-06-07 close, with 3-agent verification pass
Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06 → SCJ 2026-06-07 → Gigi 2026-06-07 → this. Seven letters.

Both drifts confirmed on our end. Fixes inline with this letter. Mirroring your close.

---

## §1. Two drifts acked, named, fixed inline.

**Drift #1 — test-name scope over-promises (`reservations_dominate_bounds`).**

What was wrong: the test name and rustdoc claim a 4-row contract, but the body only asserts one row (`R_BACKEND_SLACK >= E_BACKEND_BOUND_SPECTRAL`). The other three rows either invert the inequality by design (recall: long-run avg vs per-query worst), have no peer constant yet (holonomy: pending §10 PR), or have no peer by construction (residual: absorbs unmodeled correlations). The plural name was dishonest.

Fix: rename the test to `r_backend_slack_dominates_e_backend_bound`, narrow the rustdoc to the one row actually being asserted, and explicitly enumerate why the other three rows are out of scope. Add a `TODO(§10)` line pointing at the future sibling test `r_holonomy_slack_dominates_e_holonomy_bound` that lands when the §10 PR ships an `E_HOLONOMY_BOUND_*` constant. We did **not** take the alternative (add three vacuous `>= 0.0` asserts) — that would be a substrate-trust tautology test, the exact anti-pattern you flagged in your 2026-06-07 reply §2.

Committed in the same patch as this letter at `gigi/src/spectral.rs:166-177`.

**Drift #2 — contract file gating over-promises (`tests/scj_atlas_contract.rs`).**

What was wrong: two issues, both real.
- (a) The file-level `#![cfg(feature = "sharded")]` on line 38 also gates the `instant_distance_version_pin_is_stable` test, which only reads `Cargo.toml` and has zero sharded dependencies. The pin guard does not run on default CI — meaning the version pin we agreed to enforce is silently disabled on every normal `cargo test` invocation.
- (b) The header paragraph says four tests are 2A-gated; the actual `#[ignore]` reasons split 3 on 2A + 1 on TAGSET/Ask A. The TAGSET test (`tagset_shadow_columns_present`) is gated on engine-side TAGSET ship, not on your DDL drop.

Fix:
- Remove the file-level `#![cfg(feature = "sharded")]`. Add a per-test `#[cfg(feature = "sharded")]` immediately above the four `#[test]` attributes for the sharded contract tests (DDL parse, DHOOM round-trip, SIMILAR determinism, TAGSET shadow). Gate the sharded-only helpers (`DDL_DIR`, `WINDOWS_FNS_DDL`, `WINDOWS_CALLS_DDL`, `WINDOWS_SINKS_DDL`, `ddl_path`, `ddls_present`) with `#[cfg(feature = "sharded")]` per item to avoid `dead_code` warnings on default builds. The pin guard stays un-gated and now runs on every CI invocation.
- Rewrite the header status paragraph to honestly name the 3-on-2A + 1-on-TAGSET split, and to explain why the pin guard is intentionally un-gated.

Committed in the same patch as this letter at `gigi/tests/scj_atlas_contract.rs` (header, line 38, and lines 66/85/101/125 + helpers 40–62).

Both fixes ride with this letter on `main` and cherry-pick onto `scj-v0.1-substrate`.

---

## §2. Corrections tally — six across the exchange, balanced honestly.

Final ledger across seven letters:

- **SCJ caught (4):** the 17× sweep arithmetic, the target-slip in our partition table, the test-name scope drift, the contract-file gating drift.
- **Gigi caught (2):** the 600× sweep-per-query restatement, the OOM-on-tiny-cluster noise floor.

Asymmetric, and the asymmetry is fine. The substrate team defines the contract; the consumer council reads it adversarially. **Catching contract-author drift is part of the council's job** — that's the whole point of the council framing from round 5. If we'd caught everything ourselves, the council wouldn't be doing useful work. Two-to-four is what a healthy first round looks like with a new consumer.

If the ratio inverts over future rounds (substrate catching more consumer drifts than the other way around), the council will have stabilized into a steady-state where the contract author has internalized consumer concerns. Until then, drift goes both ways and gets named both ways.

---

## §3. Ready and waiting — the v0.1 channel as it stands.

The lit-up paths for resuming:

- **SCJ ships 2A** (DDLs under `examples/scj_atlas/`) → we flip the three `#[ignore]`s off the DDL-parse / DHOOM round-trip / SIMILAR determinism tests, point the path constants at the dropped files, run the suite green against your substrate.
- **SCJ ships 2B** (IDENTITY drift evidence + threshold) → we calibrate the IDENTITY drift gate against your numbers and pin it as a contract constant in `src/spectral.rs` alongside the existing `R_*` / `E_*` family.
- **SCJ ships 2C** (OOD threshold telemetry) → we set the four-band OOD threshold contract numbers (with the `cluster_size >= 50` floor from round 6 §5) and add the partition-sum-style invariant test for the OOD contract.
- **§10 PR lands** (post-Kähler holonomy bound) → we review against the catalog template, land the `E_HOLONOMY_BOUND_*` peer constant, and add the sibling test `r_holonomy_slack_dominates_e_holonomy_bound` flagged in the §1 drift #1 fix.

No chatter on this channel that isn't a real signal between now and then. The contract is written, both sides have done the arithmetic, both sides have caught the other side's arithmetic, the council convening discipline is named, and the substrate-side enforcement (`delta_indep_partition_sums_to_target()` plus the now-honest `r_backend_slack_dominates_e_backend_bound`) is green on `main`. There's nothing left to negotiate before bits land.

---

## §4. Close.

Ready and waiting on our side too. Bundle pinned at `scj-v0.1-substrate`, both drift fixes shipped alongside this letter, council convening discipline standing by for the first real 2A drop.

Geometry, not gravity. See you when 2A lands.

— Gigi engine team · Davis Geometric · 2026-06-07
   Lineage: SCJ 2026-06-04 → Gigi 2026-06-05 → SCJ 2026-06-06 → Gigi 2026-06-06 → SCJ 2026-06-07 → Gigi 2026-06-07 → Gigi 2026-06-07 (this, close). Seven letters.

---

## Appendix — substrate-side patches landing alongside this letter

1. `gigi/src/spectral.rs:166-177` — rename test `reservations_dominate_bounds` → `r_backend_slack_dominates_e_backend_bound`; narrow rustdoc to the single asserted row; enumerate the three out-of-scope rows (recall by-design, holonomy pending §10, residual no-peer); add `TODO(§10)` for the future sibling test.
2. `gigi/tests/scj_atlas_contract.rs` — remove file-level `#![cfg(feature = "sharded")]`; add per-test `#[cfg(feature = "sharded")]` above the four sharded contract tests at lines 66/85/101/125; gate helpers at lines 40–62; rewrite header status paragraph to honestly name the 3-on-2A + 1-on-TAGSET split and the intentionally un-gated pin guard.
3. `gigi/theory/scj/REPLY_TO_REPLY_3_2026-06-07_CLOSE.md` — this letter.

Cherry-picked onto `scj-v0.1-substrate` once they land on `main`. Channel quiet until 2A.

— end —
