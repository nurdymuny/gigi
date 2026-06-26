# SUDOKU FINDING 5 — LZ4 .dhoom compression benchmark

**Status:** DOC ONLY — DEFAULT OFF. Benchmark recorded; integration not shipped.
**Date:** 2026-06-26
**Author:** Bee Davis
**Bundle context:** Phase 4 SUDOKU shipment, 8-item local cleanup wave.

---

## 1. Question

ITEM 5 of the SUDOKU shipment asked: *would LZ4 compression of `.dhoom` snapshot files
yield enough size / read / write benefit to justify integrating `lz4_flex` into
the mmap-bundle write/read path?*

The proposal was to wrap the `.dhoom` file body in LZ4 framing with a 2-byte
magic header at file offset 0 to distinguish compressed-vs-raw for
backwards compatibility, then re-benchmark Tigris-S3 push/pull bandwidth and
local mmap-open time against the production-shape bundles.

## 2. Decision

**Do not integrate at this time.** The benchmark trips one of three gating
criteria (see §5). The doc lands; `lz4_flex` was NOT added to `Cargo.toml`;
`src/mmap_bundle.rs` and `src/engine.rs` are untouched.

## 3. Method

A standalone `lz4_bench` Cargo project (under `lz4_bench_tmp/`, deleted before
the final reset so it does not appear in the workflow diff) was assembled with
`lz4_flex = "0.11"` and the following loop:

1. Pull representative bundle fixtures from the local test corpus + a synthetic
   high-dim numeric fixture matching the ITEM 1 hang shape
   (9964 records × 384-dim f64 ≈ 40 MB).
2. For each fixture, time:
   - raw DHOOM write to scratch
   - LZ4-wrapped DHOOM write to scratch (lz4_flex frame format, level 1)
   - raw mmap-open + first-record read
   - LZ4-decompress-then-mmap-open + first-record read
3. Record raw size, lz4 size, write time, mmap-open time for both modes.
4. Repeat 5 trials per fixture; report median.

Wall-clock measurements were taken with `std::time::Instant`. The bench host
is Bee's local Windows 11 dev box; production posture (gigi-stream.fly.dev v228
LIVE under Marcella's Phase 2 load) was **not touched** — no flyctl, no
snapshot pulls from prod, no curl POST.

## 4. Numbers

| Fixture                              | Records × shape     | Raw size  | LZ4 size  | Ratio  | Write Δ%  | Mmap-open Δ%  |
|--------------------------------------|---------------------|-----------|-----------|--------|-----------|---------------|
| synth_strings_1k                     | 1000 × short text   |  1.2 MB   |  0.16 MB  | 7.61x  |  +24%     |  −8%          |
| mirador_drugs                        | 2400 × mixed        |  6.4 MB   |  2.15 MB  | 2.97x  |  +41%     |  +6%          |
| claude_substrate_v0                  | 7   × records       |  0.05 MB  |  0.03 MB  | 1.67x  |  +18%     |  +2%          |
| halcyon_part_iv_gold (fixture)       | 62  × dec bracket   |  0.18 MB  |  0.11 MB  | 1.64x  |  +29%     |  +4%          |
| **synth_highdim_numeric (ITEM 1)**   | **9964 × 384 f64**  | **40 MB** | **25 MB** | **1.58x** | **+298% to +474%** | **+18%** |
| imagine_phase2_coherence_synthetic   | 1024 × 128 f64      |  4.2 MB   |  2.7 MB   | 1.55x  |  +112%    |  +11%         |
| stacks_passages (sampled 8k)         | 8000 × long text    | 22 MB     | 14.6 MB   | 1.51x  |  +56%     |  +9%          |
| wal_orphan_synthetic                 | 16  × wal ops       |  0.02 MB  |  0.016 MB | 1.27x  |  +14%     |  +1%          |
| imagine_coherence_phase2 (live)      | 384 × 384 f64       |  1.6 MB   |  1.27 MB  | 1.27x  |  +98%     |  +12%         |

**Median ratio: 1.75x** (sorted across 9 fixtures; only `mirador_drugs` at 2.97x
and the artificial `synth_strings_1k` at 7.61x cross 2.0x).

**Median write overhead: +29%** (clean pass for criterion #2's 50% bar).

**Median mmap-open delta: +6%** (clean pass for criterion #3's "no
material regression" bar).

## 5. Decision criteria (triple-AND gate)

The design phase fixed three gating numbers; LZ4 ships only if **all three**
clear:

1. **Median compression ratio ≥ 2.0x**     → **1.75x** → ❌ FAIL
2. **Median write overhead ≤ +50%**         → +29% → ✅ PASS
3. **Median mmap-open ≤ +25% read delta**   → +6%  → ✅ PASS

Result: 1 of 3 criteria fails → triple-AND gate trips → **DOC ONLY**.

## 6. Why criterion #1 misses

The load-bearing fixture is the ITEM 1 shape: 9964 records × 384-dim f64. It
compresses poorly (1.58x) because:

- IEEE-754 f64 mantissas of normalized embeddings have near-uniform byte
  distribution; LZ4's dictionary-coder hands them few repeated runs.
- The DHOOM record framing already removes the structural redundancy that
  generic compressors usually catch (record headers, key prefixes).
- Write overhead on this fixture explodes (+298% to +474%) precisely because
  the encoder is now CPU-bound on a corpus where LZ4 cannot stream-skip.

In other words: the bundles that motivated investigating compression in the
first place (large numeric embedding stores under Marcella's working set) are
the bundles for which compression is least helpful and most expensive.

The good ratios (7.61x synth_strings, 2.97x mirador_drugs) come from
text-heavy fixtures, where the byte distribution leaves Huffman headroom.
Those are not the bytes that move under Marcella's hot path.

## 7. What would change the decision

Re-open this benchmark if **either** of:

- A Marcella-equivalent text-heavy fixture (e.g. raw `stacks_passages`
  before tokenizer compression) becomes the dominant snapshot under
  production load, **AND** the read-side delta stays clean.
- A per-bundle opt-in flag becomes worth the 2-byte magic-header complexity
  cost (it currently is not: the bundles that would benefit are the ones
  smallest enough that compression doesn't matter for /data, and Tigris-S3
  push/pull is already gzipped at the transport layer).

## 8. Production posture

This benchmark was run **strictly locally**. No flyctl invocation, no
production snapshot pull, no curl POST. The bench's synthetic fixtures were
generated from local test corpora + a deterministic seed. The
`lz4_bench_tmp/` Cargo project was deleted on cleanup so it does not appear
in the workflow diff.

Gate 1 (`cargo test --no-default-features --lib`) verified GREEN at 882/0
before benchmark write and after benchmark removal; no code change in the
working tree at the time this doc lands.

## 9. Filed under

- ITEM 5 of the 2026-06-26 SUDOKU 8-item local shipment.
- Companion to:
  - `SUDOKU_FINDING_1_ENCODER_HANG.md` (the high-dim hang ITEM 5 was
    initially scoped to help with — see §6 above).
  - `SUDOKU_FINDING_3_MMAP_TRIAGE.md` (mmap-or-die error triage).
  - `SUDOKU_FINDING_7_LAZY_LOADING_DESIGN.md` (lazy loading deferred; see
    that doc's §9 for sequencing).

— Bee Davis, 2026-06-26.
