# TDD-TXP — TEXTURE and PRECEDENCE

**Status: SHIPPED 2026-08-09.** All 16 gates green; suite 1001 passing
(985 pre-existing untouched). Worked example run against a live engine.

| | |
|---|---|
| kernels | `src/ml/texture.rs`, `src/ml/precedence.rs` |
| routes | `POST /v1/bundles/{name}/texture`, `POST /v1/bundles/{name}/precedence` |
| example | `examples/texture_precedence_walkthrough.py` |

**Two things the gates caught, recorded because both were mine:**

1. **The PRECEDENCE sign was inverted in the port.** The doc comment in that
   very file warns that this sign had already been got wrong once by assertion
   in the Python validation — and I then did it again here. TXP-4 failed with
   `x leads => negative area, got +0.517883` and the planted fixture settled it.
   The estimator was right both times; the assertion was a guess both times.
   Convention is now `A > 0 => x leads y`, pinned by a planted-lag fixture and
   by exact antisymmetry under swapping the arguments.

2. **The worked example caught a stale binary that the unit tests could not.**
   `cargo test` rebuilds the lib; the running `gigi-stream` was still serving
   pre-fix logic and reported the wrong leader on data with a known answer.
   This is precisely why §3 requires a live-engine example before merge — a
   green suite against a stale artifact is not a shipped verb.

---

**Original spec follows.** Gates below were blocking; all now pass.
**Family:** `src/ml/` alongside `circulation.rs`.
**Origin:** validated in HELICITY Python (E17/E18) before porting. Every gate
here is a gate that already caught something there.

---

## 0 · What these are, in general terms

GIGI is used for far more than finance. Both verbs are stated over **ordered
numeric sequences**, with no market vocabulary in the API.

**TEXTURE** — how rough is this signal? Returns the self-similarity exponent
*H* of an ordered numeric field. *H* > ½ persistent and smooth (a trend that
keeps going), *H* = ½ a random walk, *H* < ½ anti-persistent and jagged (a
value that keeps reversing). Sensor drift, queue latency, server load, a price,
a patient vital — anything sampled in order.

**PRECEDENCE** — which of these two moved first? Returns the normalised signed
area enclosed by the 2-D path `(x, y)`. This is the level-2 antisymmetric part
of the path signature, i.e. the Lévy area, which is circulation exactly.
It reads the **order of the records**, never a timestamp, so no bin width is
chosen and none can be got wrong.

### Why they belong together

Both answer "what shape is this data in" without imposing a grid. Every
conventional answer to either question requires resampling onto a bar/bucket
of a size the caller picked. That choice changes the answer — measured, E17:
3 of 6 pairs changed their lead-lag under a clock warp and one reversed sign.

---

## 1 · API

### TEXTURE

```
POST /v1/bundles/{name}/texture
{
  "field":   "latency_ms",   // REQUIRED, numeric fiber or base field
  "order":   "seq",          // optional; default = record order
  "q":       2.0,            // optional moment order, default 2
  "min_lag": 1,              // optional, default 1
  "max_lag": null            // optional; default n/8, capped
}
```

Returns:

```json
{ "bundle": "...", "field": "latency_ms", "n": 4096,
  "exponent": 0.312, "r_squared": 0.994, "n_lags": 10,
  "verdict": "ROUGH",
  "reads": ["H = 0.31: anti-persistent — moves tend to reverse. ..."],
  "notes":  ["ordered by 'seq'", "10 lags from 1 to 512"] }
```

`verdict` ∈ `ROUGH` (H < 0.45) · `RANDOM_WALK` (0.45 ≤ H ≤ 0.55) ·
`SMOOTH` (H > 0.55) · and the refusal path returns an error, never a verdict.

### PRECEDENCE

```
POST /v1/bundles/{name}/precedence
{
  "x":     "requests",   // REQUIRED
  "y":     "latency_ms", // REQUIRED
  "order": "seq"         // optional; default = record order
}
```

Returns:

```json
{ "bundle": "...", "x": "requests", "y": "latency_ms", "n": 4096,
  "area": -0.412, "leads": "x", "magnitude": 0.412,
  "reads": ["'requests' leads 'latency_ms' ..."],
  "notes": ["ordered by 'seq'", "area normalised by sqrt(QV_x * QV_y)"] }
```

**Sign convention, measured not assumed** — see the SHIPPED note above; this
table was written the wrong way round in the first draft of this spec and the
planted-lag gate corrected it:

| area | meaning |
|---|---|
| **> 0** | **x leads y** — x moves first, y follows |
| **< 0** | **y leads x** |
| = 0 | simultaneous (exact at zero lag) |

`leads` ∈ `"x"` · `"y"` · `"neither"`.

### Ergonomics

- One required parameter for TEXTURE, two for PRECEDENCE. Everything else has a
  default that works.
- `order` optional everywhere — record order is the default, matching the rest
  of the `ml/` family.
- Both return a `reads` array of plain-English sentences. A caller should not
  need the paper to use the number.
- Both accept **base or fiber** fields, because callers should not have to care
  which side of the schema a column landed on.

---

## 2 · Gates

Numbered TXP-*. All blocking.

### Axiom gates — does it measure what it claims?

| gate | asserts |
|---|---|
| **TXP-1** | TEXTURE recovers a known *H* from exact fBm (Davies–Harte) to within 0.15 for H ∈ {0.2, 0.5, 0.8} |
| **TXP-2** | TEXTURE is monotone in *H* across those three |
| **TXP-3** | PRECEDENCE returns **exactly 0** (< 1e-9) on two identical series — no lag, no area |
| **TXP-4** | PRECEDENCE sign is correct for a known lead in both directions, and the two groups separate cleanly |

### Invariance gates — the property being sold

| gate | asserts |
|---|---|
| **TXP-5** | PRECEDENCE is invariant to rescaling either channel (units must not matter): \|Δ\| < 1e-9 under x → 1000x and under y → 500y |
| **TXP-6** | PRECEDENCE is invariant to the *spacing* of the order field — same record order, different order values, identical answer. This is the clock-invariance claim. |
| **TXP-7** | TEXTURE is invariant to an affine rescale of the field (H is a shape property, not a scale one) |

### Refusal gates — never answer from nothing

| gate | asserts |
|---|---|
| **TXP-8** | missing bundle → **404** naming the bundle |
| **TXP-9** | missing field → **422** naming *that specific field* |
| **TXP-10** | non-numeric field → **422** saying so, not a silent 0.0 coercion |
| **TXP-11** | too few records → **422** stating the requirement and the actual count |
| **TXP-12** | zero-variance field → **422**, never a fabricated exponent |
| **TXP-13** | non-finite values are **skipped and counted into `notes`**, never coerced to 0.0 |

TXP-13 exists because `TYPE_SEAM_AUDIT.md:473` files exactly that defect against
`circulation.rs:96-101`, where `unwrap_or(1.0)` turns a missing weight into a
confident 1.0. Not inherited here.

### Determinism + limits

| gate | asserts |
|---|---|
| **TXP-14** | same bundle, same params → identical output, twice |
| **TXP-15** | above `MAX_TEXTURE_N` / `MAX_PRECEDENCE_N` → **422** stating the cap and the actual count, never an OOM or a silent `null` |

TXP-15 exists because a `MAX_N` overrun on `/circulation` returned `null` for
every field rather than erroring — the failure that cost an hour in E16.

---

## 3 · Honest worked example — required before merge

A runnable script on data with a **known answer**, printing what a customer
would see. It must include:

1. A **synthetic fixture with a planted answer** — fBm at known *H*, and a pair
   with a planted lead — so the reader can check the verbs against truth.
2. **A refusal**, shown deliberately, so the reader sees the verb decline.
3. **A non-financial example**, because the API is not finance-specific.
4. The **exact curl/GQL** a customer would issue.

No worked example, no merge.

---

## 4 · What is NOT claimed

TEXTURE is the classical scaling estimator (Mandelbrot–Van Ness lineage,
standard in rough-volatility work). The signature-native alternative was tried
and **failed its gate** — not monotone in *H*.

PRECEDENCE is the Lévy area, i.e. the level-2 antisymmetric signature term
(Chen 1957; Lyons rough-path theory). No novelty is claimed for either. The
contribution is that both are queryable on a live bundle, gated, with refusal
paths, in one call.

Neither predicts anything. Both describe the data in front of them.
