# Thermalization audit — GIBBS_SAMPLE flow + O(1) opportunities (2026-06-19)

## Summary

The current production receipt — `GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 SEED 20260616` on the
truncated-icosahedron buckyball — runs in 0.82 s engine wall on my machine, and a fresh release
bench against the same call lands at 0.047 s (47 ms) when I time only the inner sweep with `Instant`.
Both numbers are honest; they disagree by ~17x because 0.82 s is the receipt I measured
end-to-end including the registry boundary, and 47 ms is the isolated sweep loop. The "30 seconds"
the chapter copy currently quotes is neither of these — it is almost certainly the verifier
round-trip (Python + JSON + HTTP + auth) or a pre-optimization stale figure. I need to fix that
copy.

Where the time actually goes, the bench is unambiguous: the staple-sum walker is ~73% of total,
the measurement epilogue (at `MEASURE_EVERY 1`) is ~14%, the Kennedy-Pendleton heatbath kernel is
only ~7%, mutex acquisition is <0.02%. Any optimization plan that targets the heatbath sampler is
chasing the wrong 7%.

Three things to ship, in order: (1) SNAPSHOT (Part V P0–P2, already in flight) — this is the
O(1)-after-warmup lever for the verifier and the right thing for the chapter to cite. (2) A 25-LOC
patch hoisting the per-face `face_edges` allocation and pre-allocating `measurement_history` —
~1.3–1.8x net wall, zero semantic risk. (3) A process-local deterministic cache keyed by
`(β, lattice, seed, n_sweeps)` as a follow-up — bit-identity clean, ~800x on the repeat-call path.
The ensemble cache (any equilibrium sample, no specific seed) is the wrong shape for the chapter
workload and I'm deferring it; if it ships it gets a new verb (`DRAW_EQUILIBRIUM_SAMPLE`), not a
retrofit onto `GIBBS_SAMPLE`. Direct equilibrium draw is theoretically infeasible for SU(2) Wilson
on a non-tree graph and I'm rejecting it explicitly so it doesn't lurk as a phantom item.

## 1. Where the 0.82 seconds goes

I added `examples/bench_thermalization.rs` to time the canonical call in isolation plus four
sub-benches (staple, KP, mutex pattern, measurement). Three back-to-back release runs at
42.8 / 51.7 / 47.3 ms; mean ~47 ms, ~10% variance which is normal on Windows for sub-50 ms work.
Per-phase split is consistent across all three:

| phase                       | share | per-call         | calls per receipt |
|-----------------------------|-------|------------------|-------------------|
| `staple_sum_at_edge`        | ~73%  | ~1.9 µs          | 18,000            |
| `plaquette_mean` (epilogue) | ~14%  | ~35 µs           | 200               |
| `sample_su2_link` (KP)      | ~7%   | ~0.20 µs         | 18,000            |
| mutex acquire/release       | <0.02%| ~30 ns           | 1 (per call, see §2) |
| residual                    | ~6%   | —                | RNG cold cache, qmul tail, in-place buffer write, refresh epilogue, `build_edge_face_incidence` prologue |

Per-edge work, traced through the code:

- **Staple walker** (`src/gauge/staple.rs:85–140`) walks `inc[edge]` — the 2 incident faces (closed
  surface invariant, asserted at `staple.rs:162–168`). Per edge on the buckyball, the average is
  2 × ((17/3) − 2) = 22/3 ≈ 7.33 quaternion multiplies. Per sweep ≈ 660 qmuls. Per receipt ≈
  132,000 qmuls plus ~72,000 `edge_element` calls through `&dyn EdgeConnection`. The arithmetic
  itself (16 muls + 12 adds per qmul ≈ 2.1M f64 muls total) is ~0.4 ms theoretical on a 5 GFLOP/s
  scalar path. The 34 ms it actually takes is dominated by `GroupElement::compose`'s match-and-
  dispatch, trait-object indirection, and the per-call `face_edges(lat, fidx)` allocation at
  `staple.rs:100` (a fresh `Vec<(EdgeId, EdgeOrientation)>` 36,000 times per receipt).

- **KP heatbath** (`src/gauge/kennedy_pendleton.rs:106–144`) at β = 2.5 with k ≈ 2 gives ξ ≈ 5,
  which is in the regime where `sample_kp_x0` accepts on the first attempt overwhelmingly. Steady
  state: ~6 `rng.uniform()` draws per edge (~108,000 total), 4 sqrts, 1 sin, 1 cos, 4 divs, one
  final qmul + renormalize. Fast on modern hardware and the bench confirms it: 0.20 µs/call, 7% of
  the wall.

- **Mutex acquisition** is a non-issue. `gibbs_sample` at `src/gauge/gibbs_sample.rs:225` acquires
  the `SU2GaugeField` mutex ONCE and holds the guard across the entire `for s in 0..n_sweeps { for
  e in 0..n_edges { … } }` block, including the measurement epilogue. The sweep is mutex-free after
  the initial lock. Bench: 30 ns × 200 = 6 µs total = 0.01% of wall. Even the worst-case "lock per
  edge" pattern would be 0.5 ms. The architecture has runway.

- **`republish_su2` at the epilogue** (`gibbs_sample.rs:263` → `registry.rs:225–240`) is a full
  `SU2GaugeField::clone()` — 4 × 90 × 8 = 2,880 bytes of `buffer.data` memcpy plus metadata clones —
  plus three sequential HashMap mutex inserts. Single-digit microseconds. Not on the hot path.
  Note for anyone reading the code: `gibbs_sample` uses `republish_su2`, **not**
  `refresh_dyn_from_su2_mut`; both do the same deep clone, but `republish_su2` is robust against a
  parallel `registry::clear()` landing mid-sweep.

- **Measurement** is the surprise. At `MEASURE_EVERY 1`, `plaquette_mean` walks all 32 faces per
  call × 200 calls = 6,400 face-walks, costing ~7 ms (14% of wall). Moving to `MEASURE_EVERY 10`
  would save ~6 ms for free with no semantic change. Also, `measurement_history` is initialized as
  an empty `HashMap` at `gibbs_sample.rs:222` with no capacity hint; each per-observable `Vec<f64>`
  starts at capacity 0 and reallocs at 4/8/16/32/64/128 for an n=200 chain. Negligible cost, but a
  trivial cleanup.

**The "30 seconds" question.** At 0.82 s engine wall and 47 ms isolated sweep, the 30 s in the
chapter is end-to-end verifier round-trip, not engine compute. I should measure that path once to
confirm, then fix the copy. See §6.

## 2. Constant-factor wins (no semantic change)

Five candidates considered. Source-verified against the current tree.

**(1) Hoist `face_edges` + pre-allocate `measurement_history`** — PURSUE.
File: `src/gauge/gibbs_sample.rs:217, 222` and `src/gauge/staple.rs:100`. LOC: ~25. The sleeper:
`face_edges(lat, fidx)` at `staple.rs:100` calls into `holonomy.rs:44–57` and allocates a fresh
`Vec` per face per edge per sweep — 200 × 90 × 2 = 36,000 small allocations per `gibbs_sample`
call. Precompute once into a parallel `Vec<Vec<(EdgeId, EdgeOrientation)>>` indexed by
`face_idx`, built alongside `EdgeFaceIncidence` at `gibbs_sample.rs:217`. Pre-allocate
`measurement_history` with capacity. Estimated net: 1.3–1.8x. Bit-identity: unchanged (no
arithmetic touched). Risk: zero. **This is the first thing to ship.**

**(2) Staple cache (across-edge partial product reuse)** — DEFER until (1) lands.
File: `src/gauge/staple.rs:85–140` + new cache + invalidation hook in `gibbs_sample.rs:243`.
LOC: ~120. The framing in the prompt overstates the win: the current code already only walks 2
incident faces per edge, not 30 — the naive "O(F · k) → O(2 · k)" is already in the code. The real
opportunity is across-edges-within-a-sweep partial-product reuse, but on the buckyball each face
touches 5–6 edges so locality is poor; almost every face gets dirtied before its 2nd edge visit.
Honest estimate: 1.15–1.4x net, not 5–10x. Risk: HIGH for bit-identity. Every mutation site for
`field.buffer.data` (current and future — `republish_su2`, persistence replay, SNAPSHOT restore,
etc.) needs an invalidation hook; miss one and post-warmup state silently drifts. Worth doing only
after (1) clears the allocation noise so the cache benchmark is honest, and only with a regression
gate against TDD-HAL-III.5.

**(3) Mutex-acquire-once-per-sweep** — NO ACTION NEEDED, already done.
File: `gibbs_sample.rs:225`. The current code already acquires the SU2 field mutex once in an
outer scope and holds it across both loops (`let mut field = field_arc.lock()...; for s in
0..n_sweeps { for e in 0..n_edges { ... } }`). The lattice is also resolved once at `:214`, and
`EdgeFaceIncidence` is built once at `:217`. Bench confirms 0.01% wall from locking. Flag this in
the audit so the false premise gets corrected — there's no work to do.

**(4) Pre-allocate `measurement_history`** — folded into (1) above.
File: `gibbs_sample.rs:222`. LOC: ~5. Single-digit ms, free, contract-clean.

**(5) `#[inline]` on KP / staple entry points** — VERIFY FIRST, likely no-op.
File: `kennedy_pendleton.rs:67, 106`, `staple.rs:85`, `holonomy.rs:44`. LOC: 2–4. `qmul` at
`kennedy_pendleton.rs:155` already has `#[inline]`. LLVM with default release flags + lto=thin
almost certainly inlines these single-call-site cross-module functions already. Worth checking
with `cargo rustc --release -- --emit=asm` before adding the attributes; premature inline hints
are noise.

**(6) SIMD quaternion ops** — REJECT (for now).
File: `kennedy_pendleton.rs:155–164` and `group_element.rs:59–79`. Estimated 2–3x on qmul, ~1.5–2x
net wall. Breaks the bit-identity contract grounding locked decision D1 and the TDD-HAL-III.5
byte-identity test at `gibbs_sample.rs:365–421`: AVX2 FMA fuses adds-then-multiply into one
rounding step where the scalar code path performs two. Revisit only if I explicitly relax the
byte-identity invariant for a SIMD-gated production build with its own test matrix.

## 3. Memoization paths (semantic implications)

Three layers. Two of them are honest fits; one is the wrong shape for the workload that's
actually shipped.

**Layer A — deterministic `(β, lattice, seed, n_sweeps)` cache.** Shape: a sibling singleton to
`gauge::registry` holding `Arc<CachedThermalization>` entries (buffer + measurement chain +
SHA-256). Hit path: RwLock-read, Arc bump, sub-ms. Miss path: run the existing
`gibbs_sample`, insert. Mirror `VectorMatrixCache`'s single-flight per-key `Mutex<()>` so two
callers don't race on a cold key. Cap at 1000 entries (≈ 3 MB), LRU evict, no WAL durability —
recompute after restart is 0.82 s. Implementation: ~150–200 LOC in a new
`src/gauge/thermalization_cache.rs`, ~30 LOC wrap in `gibbs_sample::gibbs_sample`. **Semantic
risk: LOW.** The bit-identity contract (locked decision D1) is the cache's invariant —
`gibbs_sample(β, seed, lat, n_sweeps)` is a pure function of its arguments by design, so the
cache is observationally identical to recomputation. Receipt bytes, SHA-256 citation, measurement
chain — all identical. Payoff: on the verifier path, 100% hit after the first request. ~800x
wall-clock on repeat calls (0.82 s → < 1 ms). On a β-scan workload, 0% hit, no regression.

**Layer B — ensemble `(β, lattice, n_sweeps)` cache.** Shape: ring buffer of up to 10 thermalized
samples per key, round-robin pick on read. Key omits `seed`. Implementation: ~250–300 LOC. **Semantic
risk: HIGH.** The bit-identity surface the entire Part II/III pass criterion rests on is no longer
visible at this call site. The wire envelope's `diagnostics.seed` becomes ambiguous — whatever
value I publish either lies about what produced these bytes or violates the user's request. The
verifier in Solves Vol 4 Appendix A.4 wants the SPECIFIC seed because the seed is the citation
handle in printed text. Cross-machine reproducibility of the SHA-256 dies. **Honest call:** if a
workload arrives that wants "any equilibrium sample" (browser playground, ensemble statistics for
Marcella), ship it under a NEW verb (`DRAW_EQUILIBRIUM_SAMPLE U BETA 2.5 N_SWEEPS 200`) that does
not take a SEED and does not publish one in diagnostics. Do not retrofit onto `GIBBS_SAMPLE`. For
the chapter workload this layer's hit rate is 0%. **Defer until a use case actually emerges.**

**Layer C — SNAPSHOT-as-cache.** Already in flight as Part V P0–P2 (workflow wqzov991f).
`OP_GAUGE_FIELD_SNAPSHOT = 0x0B`, `SNAPSHOT GAUGE_FIELD U PERSIST` verb, SHA-256 over LE buffer
bytes as the caller-addressed citation handle. After `Engine::open` the snapshot's buffer is
restored via `SU2GaugeField::replace_buffer` during replay; subsequent reads are at
registry-lookup cost. **Semantic risk: LOW (arguably zero).** The contract is EXPLICIT — the
caller chose to SNAPSHOT, knows what the SHA-256 addresses, the chapter cites the handle as a
citation, not a recipe. The receipt the verifier prints is more honest than "thermalized in 0.82
s" because the SHA-256 becomes the load-bearing object instead of the wall-clock time. Caveat:
SNAPSHOT restores the FIELD, not the chain; for `GIBBS_SAMPLE`'s wire envelope the verifier needs
to call `SELECT MEAN(PLAQUETTE) FROM U` after restore (cheap, ms) to regenerate observables as
one-shots. Or, future work, a parallel `OP_MEASUREMENT_CHAIN_SNAPSHOT = 0x0C`. Out of scope here.

## 4. The O(1) path — composition

The architectural target is **C + A composed**, not either alone.

- **C handles cross-restart.** After `Engine::open` rehydrates the field from the WAL, the
  thermalized buffer is in the registry. The verifier reads observables one-shot (`SELECT
  MEAN(PLAQUETTE) FROM U`), sub-100 ms. The chapter can honestly cite a SHA-256 handle instead of
  a wall-clock figure.

- **A handles intra-process repeats.** A verifier process that gets two requests for the same
  `(β, seed, n_sweeps)` tuple — chapter exercise replay, Bee re-running the same scan, automated
  cite-check — pays full thermalization once and sub-ms thereafter. The cache is process-local
  and not WAL-durable on purpose: recompute is 0.82 s, durability is C's job.

- **B stays deferred.** If/when an AI/Marcella caller wants "an equilibrium sample at β=2.5" with
  no seed contract, ship `DRAW_EQUILIBRIUM_SAMPLE` as its own verb. Both `GIBBS_SAMPLE` (with
  seed) and `DRAW_EQUILIBRIUM_SAMPLE` (without) can coexist; one preserves bit-identity, the other
  is explicit about not promising it.

The seed-deterministic contract on `GIBBS_SAMPLE` stays intact for all paths the chapter and
verifier care about.

## 5. Direct equilibrium draw (theoretical)

Worth naming so it doesn't lurk as a phantom item. For SU(2) Wilson on the buckyball, the
equilibrium distribution `p(U) ∝ exp(-β S_Wilson(U))` is mathematically defined, but the
partition function does not factor on a non-tree graph: 32 plaquettes couple 90 edges, and the
coupling is the entire content of lattice gauge theory. The Kennedy-Pendleton heatbath IS the
local exact sampler — it draws from the exact conditional `p(U_e | rest)` and the Markov chain
converges to the global stationary distribution. There is no closed-form direct sample for
non-Abelian groups on non-tree graphs; the published exact-sampling machinery (coupling-from-the-past
and friends) does not exist for SU(2) Wilson. At β = 2.5 we are past strong coupling, so even the
character expansion is not useful.

**Reject this path.** The right "O(1) substitute" is layer C (cite the SHA-256 of a previously-
thermalized buffer, not the buffer itself).

## 6. Chapter copy update

The current "30 seconds" copy is wrong against every measurement I have. Options:

- **(a) Engine-internal compute: 0.82 s** — accurate for the receipt I cite end-to-end, used when
  naming the substrate's compute cost.
- **(b) Inner-loop sweep: 47 ms** — accurate for the isolated thermalization loop without the
  registry boundary. Useful for the "the kernel itself is fast" framing.
- **(c) Verifier round-trip: TBD** — needs one direct measurement of the Python + JSON + auth +
  HTTP path. Almost certainly where "30 s" came from. Honest number for "what a reader pays per
  cite-check today, pre-SNAPSHOT."
- **(d) Post-SNAPSHOT cached read: sub-100 ms** — accurate for the verifier cost after C ships and
  the chapter's canonical receipt has been snapshotted once.

**Recommended copy:** cite (a) for the substrate compute ("the inner sweep computes in 0.82
seconds") and (d) for the verifier cost after SNAPSHOT lands ("readers cite via a sub-100 ms
cached read against the SHA-256 handle"). Frame the two numbers as what they are: the substrate
does the work once; the citation reads a handle. The "30 s" line gets retired.

## 7. Recommended next moves

In order:

1. **SHIP: SNAPSHOT (Part V P0–P2).** Already in flight, gates V.1–V.5. Closes the verifier
   O(1)-after-warmup path and gives the chapter the SHA-256 it wants to cite.
2. **SHIP: 25-LOC patch.** Hoist `face_edges` precompute alongside `EdgeFaceIncidence` at
   `gibbs_sample.rs:217`; pre-allocate `measurement_history` with capacity at `:222`. 1.3–1.8x
   net, bit-identity unchanged, zero risk. New TDD gate to assert the hoisted table matches the
   per-call computation.
3. **SHIP: chapter copy fix.** Update "30 seconds" to "0.82 s substrate, sub-100 ms cached read"
   once SNAPSHOT lands. Before SNAPSHOT lands, just "0.82 s." Measure the verifier round-trip
   once so I know what I'm replacing.
4. **CONSIDER: Layer A deterministic cache.** Follow-up to SNAPSHOT, new module
   `src/gauge/thermalization_cache.rs`, ~150–200 LOC, TDD-HAL-V.6 or VI.1. ~800x on the repeat-
   call path. Bit-identity clean.
5. **DEFER: Layer B ensemble cache.** Revisit only when a use case arrives that doesn't want a
   specific seed. When it does, ship it as `DRAW_EQUILIBRIUM_SAMPLE`, not as a `GIBBS_SAMPLE`
   retrofit.
6. **DEFER: Staple across-edge cache.** After (2) lands, re-bench. If the staple share is still
   the dominant 60–70%, consider — but the win is 1.15–1.4x, not 5–10x, and the invalidation
   surface is real.
7. **DEFER: `#[inline]` hints.** Verify with asm first; likely no-op under lto=thin.
8. **REJECT: SIMD quaternion ops.** Bit-identity contract risk against D1 and TDD-HAL-III.5.
   Revisit only with an explicit decision to relax byte-identity for a SIMD-gated build.
9. **REJECT: direct equilibrium draw.** Theoretically infeasible for SU(2) Wilson on a non-tree
   graph.

## 8. What this audit does not claim

- Bench numbers are from my local machine (Windows 11, release build, default lto / codegen-units),
  not production CI. The 10% run-to-run variance is normal for sub-50 ms work on Windows; the
  mean is stable across three runs but I have not run it on Linux or under a controlled CI
  harness. The decomposition shape (staple 73%, measurement 14%, KP 7%, mutex <0.02%) is robust;
  the absolute numbers can shift on different hardware.
- Bit-identity verification for any constant-factor change is a follow-up gate, not assumed by
  this document. The 25-LOC patch in (2) above touches no arithmetic but still needs the
  TDD-HAL-III.5 byte-identity gate to pass before merge.
- The ensemble cache framing in §3 (layer B) assumes a workload that does not yet exist. If a
  caller emerges that wants seedless equilibrium draws, that's an explicit decision I make then,
  with its own verb and its own wire envelope.
- The 30 s → verifier-round-trip attribution in §6 is my best guess from the gap (0.82 s engine
  vs 30 s chapter); I have not measured the round-trip directly. One measurement closes the gap.
- The Halcyon Python reference path remains read-only; nothing in this document changes that.
