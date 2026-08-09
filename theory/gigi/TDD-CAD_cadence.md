# TDD-CAD — CADENCE

**Status: gates green, 18/18; suite 1019 passing (1001 pre-existing untouched).**

| | |
|---|---|
| kernel | `src/ml/cadence.rs` |
| route | `POST /v1/bundles/{name}/cadence` |
| example | `examples/cadence_walkthrough.py` |
| origin | HELICITY's GUST proposal (`helicity/docs/GUST_FOR_REVIEW.md`), reviewed and amended before porting — see §5 |

The third member of the shape family. TEXTURE asks how rough a signal is,
PRECEDENCE asks which of two moves first, and **both deliberately read record
order and discard the timestamps**. CADENCE reads exactly what they throw away.
That complementarity is structural, not a coincidence of one dataset.

---

## 0 · What it measures, in general terms

Given a numeric timestamp field, CADENCE returns **two** numbers.

**`index` — how uneven the spacing is.** With `tau_1..tau_n` the gaps between
consecutive events,

```
R_n     = sum(tau^2) / (sum tau)^2                 (Greenwood 1946)
index^2 = (n+1)/(n-1) * (n * R_n - 1)
```

Under memoryless arrivals the normalised gaps are Dirichlet(1,…,1) **for every
rate, exactly, at finite n**, so `E[R_n] = 2/(n+1)`, `Var[R_n] =
4(n-1)/[(n+1)²(n+2)(n+3)]`, and `E[index²] = 1` identically. No simulation, no
asymptotics, and no dependence on how fast events arrive.

**`memory` — whether the unevenness has memory.** Lag-1 autocorrelation of
`ln tau`, null sd `1/sqrt(n)`.

### Why two numbers and not one

`R_n` is a **symmetric function of the gaps.** Permuting them cannot change it.
Self-excitation is an *ordering* property, so `index` has **exactly zero power
against it** — this is provable from the formula, not a finite-sample weakness.

Measured, n = 512, Hawkes at μ=0.4/α=0.7/β=1.6:

| | `index` | blocked, K = 8 |
|---|---|---|
| Hawkes, time-ordered | 1.448 | 1.428 |
| **same gaps, shuffled** | 1.448 | 1.426 |
| difference | **0.000** | 0.001 |

And a process with *no memory whatsoever* can read as high as a real market:

| stream (zero self-excitation by construction) | `index` |
|---|---|
| iid lognormal gaps, σ = 0.6 | 0.657 |
| **iid lognormal gaps, σ = 1.0** | **1.305** |
| iid lognormal gaps, σ = 1.4 | 2.216 |

BTC measured 1.332 on live tape. An independent renewal process reads 1.305.
A verb shipping `index` alone could not tell them apart, so **CADENCE does not
ship `index` alone.**

The same fact makes the pairing clean: because `index` cannot see order and
`memory` cannot see spread, the two are **orthogonal by construction** rather
than by measured correlation on one session. `memory` separates all three
regimes that `index` collapses:

| stream | `index` | `memory` |
|---|---|---|
| heavy-tailed, independent gaps | BURSTY | MEMORYLESS |
| strictly periodic burst rhythm | BURSTY | ALTERNATING |
| mode-switching (stays fast, then stays slow) | BURSTY | PERSISTENT |

CAD-5 pins exactly that table.

---

## 1 · API

```
POST /v1/bundles/{name}/cadence
{
  "time":  "ts",      // REQUIRED, numeric. No default.
  "block": 64         // optional, gaps per block; default 64
}
```

Returns:

```json
{ "bundle": "...", "time_field": "ts", "n_events": 4096, "n_gaps": 4095,
  "index": 1.612, "index_z": 39.2, "null_sd": 0.0156,
  "index_blocked": 1.431, "block_len": 64, "n_blocks": 63,
  "memory": 0.548, "memory_z": 35.1,
  "verdict": "BURSTY", "memory_verdict": "PERSISTENT",
  "reads": ["index = 1.612 (+39.2 sd): arrivals are UNEVEN ...", "..."],
  "notes": ["...", "null sd of index at n = 4095 is 0.0156; verdict bands are
             the memoryless null +-2 sd, derived not chosen", "..."] }
```

`verdict` ∈ `BURSTY` · `STEADY` · `THROTTLED`
`memory_verdict` ∈ `PERSISTENT` · `MEMORYLESS` · `ALTERNATING`

### The parameter is `time`, not `order` — and that is load-bearing

TEXTURE and PRECEDENCE take an **ordinal** sort key: any monotone field works
and the values are ignored. CADENCE needs a **cardinal** clock, where the values
are the entire content.

A caller who copies a working TEXTURE call and passes a row index gets perfectly
uniform gaps, hence `R_n = 1/n`, hence `index² = (n+1)/(n-1)·(n·(1/n) − 1) = 0`
**exactly** — a maximally damning "your feed is metered" verdict on a stream that
was never measured at all. CAD-10 refuses that input by name and points at the
verb the caller actually wanted.

### Verdict bands are derived, not chosen

```
null_sd(index) = n / sqrt((n-1)(n+2)(n+3))       (delta method on Var[index^2])
bands          = 1 +- 2 * null_sd
memory_z       = memory * sqrt(n)
```

Checked against simulation:

| n | closed form | measured | `1/sqrt(n)` | measured `memory` sd |
|---|---|---|---|---|
| 64 | 0.12126 | 0.11663 | 0.12500 | 0.12045 |
| 512 | 0.04402 | 0.04392 | 0.04419 | 0.04467 |
| 2048 | 0.02208 | 0.02216 | 0.02210 | 0.02225 |
| 8192 | 0.01105 | 0.01116 | 0.01105 | 0.01112 |

This is deliberate contrast with TEXTURE, whose `H < 0.45` / `H > 0.55` bands
were **chosen by convention and are not gate-defended** — a gap this port made
visible and which is filed separately. Nothing in CADENCE's verdict layer is a
number anyone picked.

### Ergonomics

- One required parameter. `block` has a default that works.
- Base or fiber fields both accepted.
- `reads` explains **both** numbers and states that they are independent, so a
  caller cannot mistake a high `index` with no `memory` for clustering.
- Refusals name the field, the actual measurement, and the requirement.

---

## 2 · Gates

Numbered CAD-*. All blocking, all green.

### Axiom gates — does it measure what it claims?

| gate | asserts |
|---|---|
| **CAD-1** | memoryless arrivals read `index ≈ 1` — `abs(z) < 3` and verdict `STEADY` |
| **CAD-2** | **rate invariance.** The same arrivals at 1000× the rate give the *same* `index` and `memory` to < 1e-9. The Dirichlet null is exactly rate-free; a close-but-not-equal answer would mean the implementation is not |
| **CAD-3** | affine invariance — `t ↦ 7.5t + 1.6e9` moves nothing |
| **CAD-4** | bursty streams read `BURSTY`, and out-rank a steady one |

### The orthogonality gates — the reason this verb ships two numbers

| gate | asserts |
|---|---|
| **CAD-5** | three streams that `index` calls equally `BURSTY` are separated by `memory` into `MEMORYLESS` / `ALTERNATING` / `PERSISTENT` |
| **CAD-6** | shuffling the gaps leaves `index` **bit-identical** while destroying `memory` (z > 2 → \|z\| < 2). This pins the blindness rather than asserting it |

CAD-6 exists because a clustering claim backed only by `index` is a claim **no
fixture can support**. Any gate of the form "Hawkes reads above threshold" passes
equally on shuffled Hawkes. That defect was found in the source proposal's own
gate list during review and is not inherited here.

### Refusal gates — never answer from nothing

| gate | asserts |
|---|---|
| **CAD-7** | a fixed release grid is **refused by name** as quantised. Asserts the *specific* branch — see CAD-17 for why "either outcome is fine" was a defect |
| **CAD-8** | duplicate timestamps are **coalesced**, leaving the answer unchanged and disclosing the fraction. Duplicates are one observation of the clock, not zero-length gaps |
| **CAD-9** | `median(tau) < 10q` → **422** stating the ratio measured and the ratio required |
| **CAD-10** | a counter passed as a clock → **422** saying "counter, not a clock" and naming TEXTURE/PRECEDENCE |
| **CAD-11/12** | missing bundle → **404** naming it; missing field, non-numeric field, undersized `block` → **422** naming the problem |
| **CAD-13** | too few distinct timestamps → **422** reporting the actual count |

`q` is estimated as the **smallest positive gap, deliberately not a float gcd.**
A gcd collapses toward zero the moment one stamp is not an exact multiple of the
quantum, which makes the gate pass everything — a silent failure in the
permissive direction, which is the one that cannot be afforded.

Non-finite and non-numeric stamps are **skipped and counted into `notes`**, never
coerced to 0.0 — a coerced stamp lands at the epoch and manufactures one enormous
gap. Same inheritance refused as TXP-13.

### Determinism + limits

| gate | asserts |
|---|---|
| **CAD-14** | same bundle, same params → identical bits, twice |
| **CAD-15** | the blocked form attenuates structure **longer** than a block more than structure shorter than one |

### Gates added by adversarial review — each one a defect the first 15 missed

| gate | asserts |
|---|---|
| **CAD-16** | one off-grid record cannot open a quantised feed |
| **CAD-17** | **the `THROTTLED` verdict is reachable** — a metered stream on a fine clock is measured, not refused |
| **CAD-18** | nanosecond epoch stamps keep their structure through the `f64` cast |

**CAD-17 is the one that mattered.** The granularity refusal, written without a
quantisation guard, made `THROTTLED` **structurally unreachable** — and
`THROTTLED` is the commercially useful half of this verb, the feed-quality
alarm. `median/q` is a property of the gap *distribution*, not of clock
resolution: for any stream whose gaps are bounded away from zero — which is what
a poll interval or a rate limiter produces — the ratio is a small constant no
matter how fine the clock is. A 100 ms poller with ±5% jitter, stamped in f64
seconds with a 1e-17 ULP, has `median/q = 1.05` and was refused as *"too coarsely
stamped"*: a false statement about its clock, suppressing a true reading of
`index = 0.029`, deeply throttled. Measured refusal at 5%, 30%, 70% and 90%
jitter.

The fix requires evidence of quantisation before refusing — every gap an exact
multiple of `q`. Off-lattice measure: 0.105–0.500 on the poller feeds (all now
pass), 0.000000 on a genuine 1 s grid (still refused).

**CAD-7 is why it survived to review.** As first written it accepted *either* a
refusal *or* a `THROTTLED` verdict, so the suite could not distinguish "refused
because the clock is coarse" from "reported because the stream is regular" — and
the second branch had become unreachable. It now asserts the specific branch.
A gate with two acceptable outcomes tests neither.

**CAD-16** covers the same class as the gcd criticism made upstream, turned back
on our own estimator: `q = smallest positive gap` is *one record's opinion*. A
bursty stream (true `index` 3.611) rounded onto a 1 s grid reads 0.912 — the sign
of the answer inverted — and a single injected stamp 0.001 off the grid takes
`median/q` from 2.0 to 2000 and lets it through. The second check keys on
something no single record can move: a quantised clock puts gaps on a lattice,
and lattice gaps repeat *exactly*. Modal-gap fraction measures 0.22–0.33 on a
coarse feed regardless of injections (still 0.221 at +100) against 0.000–0.016 on
Poisson and lognormal at σ = 1.0 and 2.0, where `1/n` is the arithmetic floor.
The threshold sits in the empty decade between those populations.

This is **not** the tie-fraction gate rejected upstream — that one keyed on
duplicate *timestamps* and was rate-dependent. Its counterexample (a slow book on
a 1 s feed passing at a duplicate fraction of 0.397 and then reporting the
opposite sign) is refused here, because its *gaps* are lattice-valued whatever
its duplicate fraction is.

**CAD-18.** `Value::Timestamp` is a nanosecond epoch integer in this engine, and
a 2026 value (~1.75e18) lies in [2^60, 2^61) where the f64 ULP is exactly **256
ns**. A naive cast merges every pair of events closer than that, and the
coalescing step would then report the merge as *the caller's* duplicate stamps —
a false statement about their data. Measured before the fix: 3300 events with
sub-microsecond sweeps became 3057 events, 243 falsely reported as coalesced, and
the verdict flipped BURSTY → STEADY. Integer stamps are now rebased in `i64`
space before the cast, which is exactly the origin shift CAD-3 already proves
cannot move anything measured.

---

## 3 · The blocked form is a filter, not a cleanup

`index_blocked` averages `index` over contiguous blocks. This removes drift in
the arrival rate — and by the same mechanism removes any structure longer than
one block. Measured attenuation, bursts of length L, block = 64 gaps:

| burst L | vs block | `index` | blocked | retained |
|---|---|---|---|---|
| 4 | inside a block | 2.932 | 2.862 | **97.6%** |
| 16 | inside a block | 5.215 | 4.679 | 89.7% |
| 64 | equals block | 8.053 | 5.054 | 62.8% |
| 128 | spans blocks | 8.402 | 3.440 | 40.9% |
| 256 | spans blocks | 6.518 | 2.113 | **32.4%** |

So it is a **high-pass filter with its corner at the block length.** Two
consequences are stated in `notes` rather than left for the caller to discover:

1. `index / index_blocked` separates *structure shorter than a block* from
   *structure longer than a block*. It does **not** separate clustering from rate
   drift — slow clustering and drift are the same object to this statistic and no
   reading of the ratio can tell them apart.
2. Because the corner **is** the block length, the API exposes `block` in **gaps
   per block**, not a block count. Fixing the count would let the corner drift
   with `n` and quietly change what is being measured between two calls on the
   same stream.

---

## 4 · What is NOT claimed

Greenwood (1946) for the statistic; the Dirichlet spacings null is the standard
exponential-order-statistics result. Lag-1 autocorrelation is lag-1
autocorrelation. **No novelty is claimed for any of it.** The contribution is
that both are queryable on a live bundle, in one call, with derived bands and
refusal paths.

`index` is **not** a clustering or self-excitation detector — see §0, and CAD-6.
It is a measure of gap dispersion. Read it as cardinal near 1 and **ordinal above
about 1.5**: the estimator is downward-biased under heavy tails (a true CV of 4
reads about 2.89).

Neither number predicts anything. Both describe the stream in front of them.

---

## 5 · Provenance — what changed between the proposal and this verb

Ported from HELICITY's GUST proposal after a GIGI-side adversarial review
(`helicity/docs/GUST_REVIEW_RESPONSE.md`). The Dirichlet argument survived every
attack — verified across six orders of magnitude in rate, at n = 64/512/2048,
with variance matching the closed form. Four things did not survive and are
different here:

| proposal | shipped as | why |
|---|---|---|
| single statistic sold as clustering / self-excitation | two statistics; `index` sold as dispersion only | `R_n` is symmetric in the gaps and provably cannot see ordering |
| gate "Ĝ_K ≥ 1.20 on Hawkes with branching 0.4" | CAD-5 + CAD-6 | the original passes on shuffled Hawkes, so it does not gate the mechanism it names |
| block **count** `K` | block **length** in gaps | the corner is the block length; fixing `K` lets it drift with `n` |
| `q` from gcd of durations | `q` from smallest positive gap | float gcd fails permissively on one non-multiple stamp |
| parameter named alongside `order` | `time`, required, no default | ordinal vs cardinal; the row-index trap returns `index = 0` on a healthy stream |

The granularity gate `median(tau) ≥ 10q` is adopted **unchanged** — it was
derived from a stated bias budget, which is more than TEXTURE's own bands can
say.
