# The shape verbs — TEXTURE, PRECEDENCE, CADENCE

**A guide for people using GIGI against HELICITY data.**

Three verbs that answer *"what shape is this data in?"* — each without making you
pick a bar size first.

That last part is the point. Almost every regime filter and lead-lag number on a
desk starts by resampling asynchronous events onto a grid whose width somebody
chose. The choice is rarely revisited and it is not free: on live crypto tape,
three of six instrument pairs gave a **different** lead-lag answer under a time
warp that preserved event order, and one **reversed direction entirely**. Same
trades, same sequence, different clock.

These verbs don't have that knob.

---

## 1 · Which one do I want?

| Your question | Verb | What it reads |
|---|---|---|
| Is this tape smooth enough to ride, or am I about to trade chop? | **TEXTURE** | one field, in record order |
| Which of these two moved first — is flow leading price, or chasing it? | **PRECEDENCE** | two fields, in record order |
| Is my data arriving steadily or in gulps — and is my feed even healthy? | **CADENCE** | the timestamps themselves |

**TEXTURE and PRECEDENCE deliberately ignore your timestamps.** They read the
*order* records arrived in and nothing else. That is what makes them immune to
your choice of clock.

**CADENCE reads only the timestamps.** It is the complement — it measures exactly
what the other two throw away. The three don't overlap, and that is structural,
not a coincidence of one dataset.

None of them predicts anything. They describe the data in front of them.

---

## 2 · How you call them: REST, not GQL

**All three are REST endpoints. There is no GQL verb for any of them** — you
will not find `TEXTURE`, `PRECEDENCE` or `CADENCE` in the GQL grammar, and
`POST /v1/gql` will not dispatch them. That is deliberate, not an omission:
they take typed parameters and return structured diagnostics rather than rows,
so they sit in the ML family alongside `/scan`, `/cluster` and `/circulation`.

In practice you use both surfaces together — GQL to shape and load the bundle,
REST to measure it:

```bash
# 1. GQL: create the bundle
curl -s -XPOST $GIGI/v1/gql -H 'Content-Type: application/json' \
     -d '{"query":"CREATE BUNDLE btc_l2 (row_id INT BASE, ts_exch NUMERIC FIBER, mid NUMERIC FIBER, signed_volume NUMERIC FIBER);"}'

# 2. Load it (NDJSON, one record per line)
curl -s -XPOST $GIGI/v1/bundles/btc_l2/ingest \
     -H 'Content-Type: application/x-ndjson' --data-binary @trades.ndjson

# 3. REST: measure the shape
curl -s -XPOST $GIGI/v1/bundles/btc_l2/texture \
     -H 'Content-Type: application/json' -d '{"field":"mid"}'
```

Anything you can do to a bundle in GQL — filtering, `COVER`, sectioning — you
can do before measuring. The shape verbs read whatever the bundle holds at the
moment you call them.

**Discovery:** `GET /v1/ml` lists every ML endpoint including these three, with
their parameters and a one-line description of what each does. That is the
machine-readable index to use. Note that `GET /v1/openapi.json` covers the core
bundle, query and GQL surface but **does not currently include the ML family** —
don't treat its absence there as meaning a route doesn't exist.

---

## 3 · Before you start

All three run against a bundle you have already loaded. Minimum shapes:

| verb | needs | minimum rows | maximum |
|---|---|---|---|
| TEXTURE | one numeric field | 64 | 2,000,000 |
| PRECEDENCE | two numeric fields | 32 | 2,000,000 |
| CADENCE | one numeric **timestamp** field | 64 distinct stamps | 2,000,000 |

Base fields and fiber fields both work — you don't have to care which side of the
schema a column landed on.

Every response carries a **`reads`** array: plain-English sentences interpreting
the numbers. You should not need this document open to act on a result. If a
`reads` line ever contradicts what you expected, trust it over your assumption —
it is generated from the same numbers the verdict is.

---

## 4 · TEXTURE — how rough is this signal?

Returns the self-similarity (Hurst) exponent `H` of one field.

- `H > ½` — **persistent.** Moves tend to continue. A move is evidence of more of
  the same.
- `H = ½` — a random walk. Past movement carries no directional information.
- `H < ½` — **anti-persistent.** Moves tend to reverse. Choppy; trend-following
  on this signal fights itself.

### Call it

```bash
curl -s -XPOST https://your-gigi-host/v1/bundles/btc_l2/texture \
     -H 'Content-Type: application/json' \
     -d '{"field":"mid","order":"seq"}'
```

Only `field` is required. `order` is optional — omit it and record order is used.
`q` (moment order, default 2), `min_lag` (default 1), and `max_lag` (default
`n/8`) are available and rarely need touching.

### Read it

```json
{ "field": "mid", "n": 4096,
  "exponent": 0.312, "r_squared": 0.994, "n_lags": 10,
  "verdict": "ROUGH",
  "reads": ["H = 0.312: anti-persistent — movements tend to REVERSE. Choppy; ..."] }
```

`verdict` is `ROUGH` (H < 0.45) · `RANDOM_WALK` · `SMOOTH` (H > 0.55).

**Watch `r_squared`.** It is not decoration. The exponent is a straight-line fit
in log-log; a low `r_squared` means the signal is **not self-similar** and `H`
should not be read as a single number at all. A confident-looking exponent with
`r_squared` of 0.4 is telling you the model doesn't fit, not that the tape is
rough. Anything above ~0.9 is a clean fit.

### What it looks like on real books

Measured on live L2 tape: BTC and ETH land 0.43–0.71, LTC 0.26–0.39. The thin
book reads rougher, consistently — which is the separation you want, since thin
books are where the losses live.

For calibration: it recovers a known `H` from exact fractional Brownian motion to
within ±0.03 across H = 0.1 to 0.9.

**A limit worth knowing.** The `ROUGH`/`SMOOTH` cutoffs at 0.45 and 0.55 are
conventional, not derived from a noise budget, and the estimator's own sampling
spread is comparable to the band width at modest `n`. Treat a verdict near the
boundary as "inconclusive" rather than as a call, and read `exponent` and
`r_squared` directly when the decision matters. Deriving those bands properly is
open work.

---

## 5 · PRECEDENCE — which of these two moved first?

Returns the normalised signed area enclosed by the joint path of two fields.

```bash
curl -s -XPOST https://your-gigi-host/v1/bundles/btc_l2/precedence \
     -H 'Content-Type: application/json' \
     -d '{"x":"signed_volume","y":"log_mid","order":"seq"}'
```

### Read it

```json
{ "x": "signed_volume", "y": "log_mid", "n": 4096,
  "area": -0.768, "leads": "y", "magnitude": 0.768,
  "reads": ["'log_mid' leads 'signed_volume' ..."] }
```

| `area` | meaning |
|---|---|
| **> 0** | **`x` leads `y`** — x moves first, y follows |
| **< 0** | **`y` leads `x`** |
| ≈ 0 | simultaneous |

Swapping the arguments negates the area exactly. `magnitude` is how pronounced
the lead is, not how confident you should be.

> **On that sign convention.** It was asserted the wrong way round twice during
> development — once in validation, once again in the port — and caught both
> times by a fixture with a *planted* lead. The estimator was right on both
> occasions. If you are ever unsure which way it reads on your data, don't reason
> about it: construct a series with a lead you chose, and measure.

### The property you're paying for

Under time warps that preserve event order (`u²`, `√u`, `u³`), **0 of 6
instrument pairs moved** — identical to six decimal places. The grid-based method
on the same data moved on **3 of 6**, and one pair (`sol-ltc`) flipped from −1 to
+2: it reversed which instrument leads, from the clock alone.

There is no bin width to choose, and therefore none to get wrong.

### On real tape

BTC −0.768, ETH −0.444, SOL −0.034 — flow leading price on the liquid books.
LTC **+0.408** — price leading, flow chasing. LTC is the same instrument TEXTURE
reads as roughest. Two independent measurements, same verdict, neither built to
agree with the other.

**Honest limit.** Grid-free is not the same as robust to missing data. Thin the
events and the answer moves, because deleting vertices genuinely changes the
path. What you are immune to is *your own arbitrary choice of clock* — not gaps
in the feed.

---

## 6 · CADENCE — is this arriving steadily, or in gulps?

This one reads your timestamps, and it returns **two** numbers. Always both.

```bash
curl -s -XPOST https://your-gigi-host/v1/bundles/btc_trades/cadence \
     -H 'Content-Type: application/json' \
     -d '{"time":"ts_exch"}'
```

> ### ⚠ The parameter is `time`, not `order`
>
> TEXTURE and PRECEDENCE take an **ordinal** sort key — any increasing field
> works and the values are ignored. CADENCE needs a **cardinal** clock, where the
> values *are* the measurement.
>
> If you copy a working TEXTURE call and pass a row index, every gap is
> identical, and the statistic is then exactly zero — a maximally damning "your
> feed is metered" verdict on a perfectly healthy stream. CADENCE refuses that
> input by name and points you back at the verb you actually wanted.
>
> Write timestamps as **epoch seconds or milliseconds**. Epoch *nanoseconds*
> exceed the range where a float holds every integer, so a 2026 nanosecond stamp
> is only precise to ~256 ns. CADENCE handles integer stamps correctly by
> rebasing them before conversion, but seconds or millis are less to think about.

### Read it

```json
{ "time_field": "ts_exch", "n_events": 4096, "n_gaps": 4095,
  "index": 1.510, "index_z": 32.6, "null_sd": 0.0156,
  "index_blocked": 1.431, "block_len": 64, "n_blocks": 63,
  "memory": 0.5635, "memory_z": 36.1,
  "verdict": "BURSTY", "memory_verdict": "PERSISTENT",
  "reads": ["index = 1.510 (+32.6 sd): arrivals are UNEVEN ...", "..."] }
```

**`index` — how uneven the spacing is.**

| | |
|---|---|
| ≈ 1 | memoryless arrivals — the ordinary, healthy state |
| **> 1** | **BURSTY** — the stream comes in gulps rather than a steady drip |
| **< 1** | **THROTTLED** — *more regular than random*. See below; this is the one to wire to a pager |

**`memory` — whether the unevenness persists.**

| | |
|---|---|
| **> 0** | **PERSISTENT** — quiet follows quiet, busy follows busy. The stream has a state and stays in it |
| ≈ 0 | **MEMORYLESS** — the unevenness is in the *distribution* of gaps, not their order |
| **< 0** | **ALTERNATING** — a long gap tends to be followed by a short one. Often a queue draining in batches |

`index_z` and `memory_z` are standard deviations from the memoryless null, and
the verdict bands are the null ±2 sd — computed from a closed form at your actual
`n`, not thresholds anyone picked. `null_sd` is reported so you can see the ruler.

### Why two numbers and not one

Because one cannot be honest on its own.

`index` sums over the gaps, and **addition does not care about order**. So it
*provably cannot* distinguish a self-exciting stream from the same gaps in
scrambled order — measured, a clustering process and its own shuffle both read
1.448, a difference of 0.000. That is arithmetic, not a small-sample weakness.

Which means a high `index` alone does **not** mean clustering. An independent
process with a heavy-tailed gap distribution and *zero* memory reads 1.305, where
live BTC tape reads 1.332. You could not tell them apart from that number.

Three streams `index` calls identically bursty, separated by `memory`:

| stream | `index` | `memory` |
|---|---|---|
| long-tailed but independent gaps | BURSTY | MEMORYLESS |
| a batch job on a fixed rhythm | BURSTY | ALTERNATING |
| genuine load waves — busy, then quiet, staying in each | BURSTY | PERSISTENT |

Only the third is a stream where *being busy now tells you anything about next*.

The two are non-overlapping by construction: `index` cannot see order, `memory`
cannot see spread. That's a stronger guarantee than a correlation measured once
on one dataset.

### The reading a desk can use tomorrow

**`index` below 1 is a feed-quality alarm.** Real sources are not more regular
than random. If you see `THROTTLED`, something is metering the stream between the
source and you — a poll interval, a rate limiter, a fixed flush — and every
number computed downstream is describing your own plumbing rather than the market.

It is not subtle. Releasing BTC events on a fixed shared hold collapses the
reading exactly as you'd predict:

| hold | `index` |
|---|---|
| 0.5 s | 0.835 |
| 1 s | 0.588 |
| 5 s | 0.076 |

A per-bar count statistic cannot do this job: the Fano factor of SOL reads
5.2 / 17.6 / 82.2 / 446.5 at 0.25 / 1 / 5 / 30 s bars — entirely an artefact of
the bar you chose. CADENCE's worst parameter sensitivity is 1.73× across a 32×
range of window size.

### `index_blocked` — read this before using it

`index_blocked` averages the statistic over contiguous blocks. It removes drift
in the arrival rate — and, by the same mechanism, **any structure longer than one
block.** It is a high-pass filter whose corner is the block length:

| burst length | retained |
|---|---|
| 4 gaps (well inside a block) | 97.6% |
| 64 gaps (one block) | 62.8% |
| 256 gaps (spanning blocks) | 32.4% |

So `index / index_blocked` separates *structure shorter than a block* from
*structure longer than a block*. It does **not** separate clustering from rate
drift — to this statistic, slow clustering and drift are the same object, and no
reading of the ratio can tell them apart.

`block` is therefore specified in **gaps per block** (default 64), not as a block
count. Fixing the count would let the filter corner drift with your sample size
and silently change what you're measuring between two calls on the same stream.

---

## 7 · Using all three together

They read different things, so they can corroborate each other:

```bash
# Same bundle, three questions.
curl -s -XPOST $GIGI/v1/bundles/ltc_l2/texture \
     -H 'Content-Type: application/json' -d '{"field":"mid","order":"seq"}'

curl -s -XPOST $GIGI/v1/bundles/ltc_l2/precedence \
     -H 'Content-Type: application/json' \
     -d '{"x":"signed_volume","y":"log_mid","order":"seq"}'

curl -s -XPOST $GIGI/v1/bundles/ltc_l2/cadence \
     -H 'Content-Type: application/json' -d '{"time":"ts_exch"}'
```

A sensible order of operations:

1. **CADENCE first.** If it reads `THROTTLED`, stop — you are measuring your own
   pipe, and the other two verbs will faithfully describe an artefact.
2. **TEXTURE next.** Rough tape means chop; size accordingly or stand aside.
3. **PRECEDENCE last**, once you trust the feed and know the regime, to ask what
   is actually leading.

On LTC, TEXTURE reads roughest and PRECEDENCE reads price-leads-flow — two
independent measurements agreeing about the same instrument without being
designed to.

---

## 8 · When they refuse

All three refuse rather than return a number they can't stand behind. **This is
a feature.** The failure mode being avoided is a confident value computed from
something that was never really measured.

| you sent | you get |
|---|---|
| a bundle that doesn't exist | **404**, naming the bundle |
| a field that doesn't exist | **422**, naming *that specific field* |
| a text field | **422** — never a silent coercion to 0.0 |
| too few rows | **422**, stating the requirement *and your actual count* |
| a field that never moves | **422** — no fabricated exponent |
| more rows than the cap | **422**, stating the cap — never an OOM or a silent `null` |
| **CADENCE:** a row index as `time` | **422** — "that is a counter, not a clock", pointing at TEXTURE/PRECEDENCE |
| **CADENCE:** timestamps too coarse | **422**, telling you the ratio measured and the ratio needed |
| **CADENCE:** timestamps on a lattice | **422** — rounded feeds read as *regular* when they're actually bursty |

Rows with missing or non-finite values are **skipped and counted in `notes`**,
never coerced to zero. Check `notes` and `n` if a result surprises you — you may
be measuring fewer records than you loaded.

---

## 9 · What we do not claim

**None of these predicts anything, and none of them is an edge.** They are
measurement instruments with stated noise floors, gates that refuse rather than
guess, and receipts you can reproduce in an afternoon.

**The mathematics is not new and we don't pretend otherwise.** TEXTURE is the
classical scaling estimator in the Mandelbrot–Van Ness lineage. PRECEDENCE is the
Lévy area — the level-2 antisymmetric signature term (Chen 1957; Lyons'
rough-path theory). CADENCE uses the Greenwood statistic (1946) with the standard
exponential-spacings null. What is ours is that all three are queryable on a live
bundle, in one call, with derived bands and honest refusal paths.

**CADENCE in a trading context is diagnostics-only.** Its economic value is
unproven — the test that would establish whether it saves anything on execution
has not been run. Use it to judge your feed and describe your session. Do not
gate entries on it.

**One session is one sample.** Every measured number quoted here comes from
specific tape on specific days and is offered as a receipt you can reproduce, not
as a stable constant.

---

## 10 · Reference

| | TEXTURE | PRECEDENCE | CADENCE |
|---|---|---|---|
| route | `POST /v1/bundles/{name}/texture` | `.../precedence` | `.../cadence` |
| required | `field` | `x`, `y` | `time` |
| optional | `order`, `q`, `min_lag`, `max_lag` | `order` | `block` |
| min rows | 64 | 32 | 64 distinct stamps |
| verdicts | `ROUGH` `RANDOM_WALK` `SMOOTH` | `leads`: `x` `y` `neither` | `BURSTY` `STEADY` `THROTTLED` + `PERSISTENT` `MEMORYLESS` `ALTERNATING` |

Runnable examples with planted answers and deliberate refusals:
[`examples/texture_precedence_walkthrough.py`](../examples/texture_precedence_walkthrough.py) ·
[`examples/cadence_walkthrough.py`](../examples/cadence_walkthrough.py)

Machine-readable index of every ML endpoint, these three included:
`GET /v1/ml` against any running instance.
