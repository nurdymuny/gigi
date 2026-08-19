# TDD-GAUGE — the market-bundle instrument

**Status: G0 RUN AND PASSED (2026-08-08). The instrument is calibrated.**
Results in §G0-RESULTS below; artifacts in `helicity/integration/gauge/`.
G1 is **not** started — G0 established a data requirement that changes it.
**Supersedes:** the daily-bar experimentation line (`helicity/integration/daily/`).
**Motivating survey:** *Differential Geometry in Machine Learning for Financial
Markets: A Rigor-Filtered Survey*, §(b) open gaps, line 124.

---

## 0. Why the previous line was going in circles

Every result produced on the day-shape substrate — regime taxonomy, HERALD
displacement, lead-lag circulation — lands in the survey's crowded-and-weak
columns (Threads 1, 3, 5), and independently reproduced the literature's own
verdict: strong in-sample structure, honest nulls, no demonstrated
out-of-sample edge.

The cause is not effort. **The substrate had no bundle structure.** Six ETFs ×
41 daily columns is a point cloud in ℝ⁴¹: no base space, no fiber, nothing to
transport *along*. Every "geometric" quantity computed on it was multivariate
statistics wearing a geometric name. This is also the mechanism behind the verb
audit's finding that `DEPTH`/`HORIZON`/`SPECTRAL`/`BETTI` are structurally dead
on that bundle — a substrate mismatch, not a defect.

GIGI's actual differentiator — connection, holonomy, curvature 2-form,
characteristic class — was unreachable by construction.

## 0.1 The reframe

**Stop predicting. Build an instrument.**

Every thread the survey marks as failed, failed at *prediction*. The single
largest open gap is *measurement*: no tooling computes connection, curvature
and holonomy on a real market bundle. The product is analysis and diagnosis;
a calibrated instrument is the deliverable, and a measured quantity with a
stated noise floor is defensible in a way a lift ratio is not.

This also dissolves the survey's strategic caveat — "open partly because the
payoff is unproven." The payoff of an instrument is not "it predicts returns."
It is "it measures a quantity that is known to exist, to a stated accuracy,
and then measures it where nobody has looked."

## 0.2 Why FX, exactly and not by analogy

Let `S_ij` be the price of one unit of currency *j* in units of *i*. Define the
connection on the edge *i→j* as `A_ij = log S_ij`. Then:

- `A_ij = −A_ji` — antisymmetric, by the definition of an exchange rate.
- Holonomy around a closed loop `γ = (i→j→k→i)` is `H(γ) = Σ_{e∈γ} A_e`.
- `H(γ) = 0` ⟺ no triangular arbitrage on that loop.
- A **global gauge section** (Farinelli's numéraire) exists ⟺ there is a
  potential `φ` on currencies with `A_ij = φ_j − φ_i` ⟺ the connection is a
  pure gradient ⟺ every loop holonomy vanishes.

That is the Hodge decomposition of the edge flow, and it is exact, not a
resemblance. `POST /circulation` already computes it: `potential` is φ (the
numéraire, up to the additive constant Hodge fixes per component),
`circulation_ratio` is the fraction of the log-rate structure that **no
numéraire can explain**, and `cyclic_edges` names which pairs carry it.

**BRIDGE ALTITUDE: STRUCTURAL.** This is not incidence or resonance. The
Hodge/gauge identification is an equality of definitions, not a family
resemblance. The deferred check is §G0.

**Prior art, cited rather than claimed.** Ilinski (1997; Wiley 2001) first
framed FX and discounting as parallel transport with arbitrage as curvature;
Young (1999) gave the lattice-gauge-theory form; Farinelli (SSRN 1113292,
arXiv:0910.1671) and Farinelli–Takada (*Axioms* 10(4):242, 2021) supply the
rigorous principal-bundle formulation, NFLVR ⟹ zero curvature, and the
holonomy-group parameterisation of arbitrage strategies via Ambrose–Singer.
Ilinski's *dynamics* are formally critiqued as unstable (Sornette 1998; *EPJ B*
2010) and are **not** used here. Malaney–Weinstein index numbers are likewise
excluded (Nguyen, arXiv:2112.03460). **No priority is claimed for the framing.
The claim is tooling: none of the above has a computational instantiation.**

## 0.3 What the engine already has, and what it does not

| capability | status | citation |
|---|---|---|
| Hodge decomposition of directed edge flow | **SHIPPED** | `src/ml/circulation.rs:68-220`, route `gigi_stream.rs:16564` |
| SU(2) holonomy around a named lattice cycle | **SHIPPED** | `src/holonomy_cycle.rs`, walker `gauge::holonomy::walk_loop` (group-erased) |
| Discrete Chern/Pontryagin from plaquette holonomies | **SHIPPED** | `src/chern_weil.rs`, `F = −i log U_p` |
| Global-section obstruction for principal G-bundles | **SHIPPED** | `src/obstruction.rs` |
| B-perturbed parallel transport, RK4, energy-audited | **SHIPPED** | `src/geometry/transport.rs` |
| Integrated autocorrelation time / effective sample | **SHIPPED** | `src/aggregation.rs:224-296` |
| **U(1) gauge group** | **PANICS** | `src/gauge/group_element.rs:36-40` — compiles, every method `unimplemented_for_group!("U1")` |

**U(1) does not need implementing.** The abelian phase embeds in SU(2) as the
σ₃ subgroup, `exp(iθ) ↦ (cos θ, 0, 0, sin θ)` in the scalar-first quaternion
layout — which is already exactly how `holonomy_cycle.rs` builds its twisted
boundary condition `Ω = exp(2πi·q·σ₃/p)`. No new group math, matching that
module's existing constraint.

**Compactification caveat, stated up front.** The natural FX gauge group is
(ℝ₊, ×) ≅ (ℝ, +), which is non-compact. Embedding in U(1) ⊂ SU(2) is valid only
when total loop holonomy ≪ 2π. **Guard: refuse when |H(γ)| > 0.1 rad**
(≈10% log-arbitrage, six orders of magnitude above anything real). For the
abelian case the sum-of-logs is exact and the group machinery adds nothing —
it earns its keep only at §G2, where transport stops commuting.

---

## G0 — AXIOM GATE. The instrument must read zero where zero is the truth.

Per the standing rule: run the axiom on the simplest fixture and ship GATED
with named blocking preconditions.

**Fixture.** Three independently quoted crosses forming one closed triangle:
`EURUSD=X`, `USDJPY=X`, `EURJPY=X` (yfinance, daily close, ≥10y). Independent
quoting is essential — a synthetic cross closes the triangle by construction
and would make G0 vacuous.

**G0.1 — zero on a no-arbitrage loop.** Build the 3-node edge bundle, call
`/circulation`. Assert `circulation_ratio < ε₀` and `|H(triangle)| < ε₀`.

`ε₀` is **not** chosen. It is *measured* and reported as the instrument's noise
floor: the daily-close residual is dominated by **non-synchronous quoting**
(three pairs stamped at three different instants), not by arbitrage. Report the
residual distribution and state plainly that it is a quote-synchronicity
measurement. An instrument with an unstated noise floor is not an instrument.

**G0.2 — recovery of an injected arbitrage.** Perturb one edge by a known
`δ ∈ {1, 10, 100} bp`. Assert recovered `|H(γ)| = δ` to 1e-9 and that
`cyclic_edges` ranks the perturbed edge first. **This is the gate that proves
the instrument measures what it claims.** Nothing downstream may ship until it
passes.

**G0.3 — gauge invariance.** Rescale one currency's units by an arbitrary
factor (a gauge transformation `φ → φ + c` on that node). Assert every loop
holonomy and `circulation_ratio` are **bit-identical**, and that `potential`
shifts by exactly `c`. A quantity that is not gauge-invariant is not a
physical observable and must not be reported.

**G0.4 — SU(2) embedding agrees with the abelian sum.** Compute the same loop
both ways; assert agreement to 1e-12 and that the |H| > 0.1 rad guard fires.

**Blocking preconditions, named:** loop must be closed; all three legs
independently quoted; all quotes same session; |H| < 0.1 rad. Refuse, do not
degrade.

---

## G0-RESULTS — run 2026-08-08

Fixture: 4,320 common sessions, 2010-01-01 → 2026-08-07, three independently
quoted crosses (`EURUSD=X`, `USDJPY=X`, `EURJPY=X`, yfinance daily close).

| gate | verdict | evidence |
|---|---|---|
| **G0.1** zero where zero is true | **PASS** | median \|H\| **0.88 bp**, 55.1% of sessions under 1 bp |
| **G0.2** recovers injected arbitrage | **PASS** | 1/10/100 bp recovered to **4.45e-17** — machine precision — and `cyclic_edges` ranked the perturbed edge first |
| **G0.3** gauge invariance | **PASS on H** | redenominating JPY by 100× and 0.001×: **\|dH\| ≈ 1e-16** |
| **G0.4** SU(2) σ₃ embedding | **PASS** | agrees with abelian sum to **3.37e-16**; \|H\|>0.1 rad guard fires |
| **G0.5** systematic-offset calibration | **PASS (rolling only)** | constant offset FAILS at z = −16.71; rolling median PASSES at every window 21–504 sessions |

### Three findings the gate produced that the spec did not anticipate

**1. `circulation_ratio` is NOT gauge-invariant.** Measured spread 1.3e-4
across redenominations of a single currency. The reason is structural:
`ratio = ‖cyclic‖²/‖flow‖²`, and a gauge transformation adds a gradient, which
leaves the numerator alone and changes the denominator. **The loop holonomy is
the observable; `circulation_ratio` is a property of the market AND the units
and must never be reported as a market property.** This applies retroactively
to the lead-lag circulation work in `VERB_AUDIT.md` §8 — the *ranking* there
is fine (Hodge potential is gauge-covariant, shifting by a constant), but the
ratio is only comparable between graphs in the same units.

**2. The first G0.3 pass was vacuous and had to be redone.** Both pre- and
post-gauge `circulation_ratio` values came back `0.0` and were scored
"invariant." They were equal because the verb rounds to 5 decimals and the
true ratio on the unperturbed triangle is ~1e-11 — cyclic energy `H²/3` with
`H ≈ 4e-5`, against `‖A‖² ≈ 52` dominated by `log(USDJPY) ≈ 5.06`.
Equal-because-rounded is not equal-because-invariant. Re-run with a 500 bp
injection, the answer inverted.

**3. The feed carries a systematic, drifting one-sided offset.** Raw residual
is positive in **89.0%** of sessions and in *every* year from 2010 (79.7%) to
2026 (97.3%). Pure quote non-synchronicity would be symmetric. Median offset
drifts from **+1.91 bp (2016) to +0.34 bp (2026)**, so a single constant
cannot remove it. This is a bid/ask convention artifact in the feed, at the
scale of a retail FX spread component.

**This is precisely the false positive the gate exists to catch.** Uncaught, it
reads as *"persistent triangular arbitrage in EUR/USD/JPY on 89% of days,
every year for sixteen years."*

### The instrument, as calibrated

```
observable          loop holonomy H = Σ log S over a closed currency loop
                    gauge-invariant to 1e-16, exact to 4e-17
calibration         rolling 63-session median of H (feed convention, drifts)
noise floor         p99 |H| = 5.9 bp after calibration
CAN see             triangular inconsistency faster than the calibration window
CANNOT see          any persistent offset — removed by construction
DO NOT report       circulation_ratio as a market property
```

### What G0 established about the data, which changes G1

Real triangular arbitrage in liquid FX is sub-basis-point and lives for
milliseconds. **The measured noise floor on free daily-close data is 5.9 bp —
roughly three orders of magnitude larger.** The instrument is correct; the data
is too coarse to see the thing the instrument was built to measure.

That is a successful calibration outcome, not a failure: a gate that tells you
the required resolution *before* you build on it has done its job. G1 as
originally written would measure quote synchronicity across a 7-currency graph
and report it as market curvature. **G1 is therefore rewritten below.**

---

## G1 — MEASUREMENT. The currency graph, where the answer is not known.

Seven currencies (USD EUR JPY GBP CHF AUD CAD) → up to 21 edges and a cycle
space of dimension `E − V + 1 = 15` independent loops. Now there is a genuine
lattice with many plaquettes and a *distribution* of loop holonomies.

**G1.1 — is the inconsistency concentrated or spread?** Hodge-decompose the
full graph daily. Report `circulation_ratio` as a time series with `tau_int`
and `n_eff` from `WITH JACKKNIFE ALONG date` — the audit established that
volatility-like series carry ~4.5× fewer effective observations than their
raw count, and this series will autocorrelate.

**G1.2 — does the numéraire potential move?** `potential` is the Hodge
ranking of currencies. Its rotation over time is a measured object with, as
far as the survey shows, no existing computational instantiation. Requires the
block-bootstrap null from the verb audit before any rotation number ships.

**G1.3 — falsifier.** If `circulation_ratio` is statistically
indistinguishable from the G0.1 non-synchronicity floor at every date, then
daily-close free data cannot see FX curvature, and the honest conclusion is
that the instrument needs intraday quotes. **Say so and stop.** Do not
reach for a weaker claim.

### G1 AS REWRITTEN AFTER G0 (2026-08-08)

G1.3 has effectively **already fired** — G0 measured the floor at 5.9 bp
against a target three orders of magnitude smaller. Running G1 on daily closes
would measure the feed, not the market, and reporting it as curvature would be
the exact error G0.5 was built to catch. **G1 on daily data is cancelled.**

Two honest successors, in order of cost:

**G1′ (free, small) — the instrument as a DATA-QUALITY diagnostic.** The
7-currency graph over 21 edges has a 15-dimensional cycle space. Hodge-decompose
it daily and ask where the inconsistency concentrates. This measures quote
synchronicity and relative feed quality per currency pair — a real, useful,
honestly-named result, and the correct product for this data. It is *not* a
market-curvature claim and must not be dressed as one. Note the gauge caveat
from finding 1: compare holonomies, not ratios, across graphs.

**G1″ (paid, decisive) — intraday quotes.** Sub-second synchronised FX
quotes bring the synchronicity floor down by orders of magnitude and make the
original G1 answerable. This is the point at which the instrument measures
markets rather than feeds. Cost is the gate, not capability — G0 proved the
machinery is exact.

**Recommendation:** run G1′ for the receipt, and treat G2 (currency × tenor,
CIP plaquette) as the real destination — the CIP basis is *tens of basis
points*, comfortably above even the daily-close floor measured here, which
makes it the one genuinely non-abelian target reachable without tick data.

## G2 — THE ONLY PLACE THE GAUGE MACHINERY EARNS ITS KEEP

G0 and G1 are abelian: holonomy is a sum of logs and a spreadsheet does it.
The SU(2)/Chern–Weil machinery is justified **only** where transport fails to
commute.

Non-commutativity requires a fiber of dimension ≥ 2 and two base directions.
The candidate with published ground truth is the **currency × tenor** bundle:
moving along currency (an FX conversion) and along tenor (a discount) do not
commute, and the plaquette is exactly **covered interest parity**. The CIP
basis is measured, published, and famously non-zero since 2008 — a real
non-vanishing curvature with an independent reference value.

**G2 is GATED and unscheduled.** It requires forward points or cross-currency
basis quotes, which are not free, and it must not be started until G0 passes
and G1 has either produced a signal above its noise floor or been honestly
killed by G1.3.

### G2 DATA SCOPING — RUN 2026-08-08. The paid-data assumption was WRONG.

Every input is free and every source below was tested reachable from this
machine, not assumed:

| input | source | status |
|---|---|---|
| spot FX, all 14 crosses | yfinance | **have it** — 4,318 sessions 2010–2026 |
| FX forwards (CME futures) | yfinance `6E=F 6J=F 6B=F 6C=F 6A=F 6S=F` | **OK** — 2,916 rows each, 2015-01-02 → 2026-08-07 |
| USD short rate | yfinance `^IRX`, `ZQ=F` | **OK** — 2,915 rows |
| EUR €STR | ECB Data Portal `data-api.ecb.europa.eu` | **OK** — CSV, no key |
| GBP SONIA | Bank of England IADB CSV | **OK** — CSV, no key |
| JPY / CHF / CAD / AUD | BoJ / SNB / BoC / RBA | same class, untested |
| ~~FRED~~ | `fred.stlouisfed.org` | **BLOCKED from this sandbox** (RemoteDisconnected) — not needed, the central banks publish the same series at source |

**The gate on G2 was my assumption, not a fact. Cost is zero.** Six currencies
against USD, ~2,900 sessions, from four public providers.

**One structural limit to design around, found while scoping.** Only
USD-crossed futures are freely quoted, so a non-USD forward cross (e.g. forward
EUR/JPY) can only be built synthetically as `6E/6J` — which closes the forward
triangle *by construction* and makes any forward-triangle holonomy vacuous.
This is exactly the trap G0.1 was written to avoid (independent quoting is what
makes the test non-empty). Therefore:

- **In scope:** the CIP basis per currency **against USD**, which is the genuine
  currency × tenor plaquette and needs an independent foreign short rate — hence
  the central-bank series above. This is the real non-abelian target.
- **Out of scope on free data:** forward-triangle consistency among non-USD
  crosses. Do not compute it from synthetic forwards and report it as a
  measurement.

---

## G2-RESULTS — run 2026-08-08. **FAILED on free data. Cause identified.**

Built the plaquette for EUR/USD and GBP/USD: spot (yfinance), forward (CME
front-month future), USD leg `^IRX`, foreign leg €STR / SONIA.
`H = (s − f) + (i_USD − i_X)·τ`, basis `= −H/τ` in bp p.a.

**First run produced a FALSE PASS and it had to be caught.** The March-2020
gate reported PASS with a 3,244 bp peak. The eight largest deviations were
2022-03-04, 2025-03-05, 2025-03-04, 2020-03-06, 2020-03-02, 2023-03-01,
2021-03-04, 2023-03-02 — **every one in the first week of March.** That is the
quarterly futures roll, which happens every year. The gate fired on a calendar
artifact. Magnitudes also gave it away: median −135 bp for EUR against a true
basis of −20 to −50, peaks of 32%.

**Decisive re-test with the roll excluded (τ ∈ 60–85 days):**

| year | n | median \|basis\| bp | max |
|---|---|---|---|
| 2019 | 5 | 78.6 | 198.6 |
| **2020 (COVID)** | 19 | **146.9** | 438.3 |
| 2021 | 18 | 138.8 | 379.3 |
| 2022 | 18 | 163.2 | 398.7 |
| 2023 | 11 | 221.4 | 738.9 |
| 2025 | 16 | 169.6 | 599.7 |
| 2026 | 10 | 239.2 | 337.8 |

**March 2020 is below four other years.** The largest CIP dislocation in a
decade is invisible. **G2 FAILS its validation gate.**

### The cause, quantified rather than guessed

Noise standard deviation by tenor band: **2517 → 477 → 297 → 205 bp** for
τ = 5–20, 20–40, 40–60, 60–85 days. Falling as 1/τ is the exact signature of a
**fixed price error divided by τ**.

The error is that **Yahoo's FX spot close and the CME settle are hours apart**,
so `s` and `f` are not sampled at the same instant. At τ ≈ 70 days a ~40 bp
snapshot mismatch becomes ~210 bp of basis noise — matching the measured 205.
Against a true basis of 20–50 bp, signal-to-noise ≈ **0.15**.

### The fix, and it is not "buy expensive data"

**The FX market quotes forward POINTS, not forward prices.** `f − s` is a single
quoted number, and quoting it that way eliminates the timing mismatch *by
construction* — the same reason G0.1 insisted on independently quoted legs.
Reconstructing `f − s` from two separately-timestamped prices reintroduces
exactly the error the market's own convention removes.

**Revised data requirement:** forward points (or same-snapshot spot+forward),
not forward prices. This is a different ask from "a paid feed" — it is a
*convention* requirement. Sources to check before spending anything: central
bank forward-rate publications, BIS locational statistics, and the ECB/BoE
reference-rate series that already proved free and reachable.

### What did NOT fail

G0 remains passed: the holonomy machinery is exact to 4e-17 and gauge-invariant
to 1e-16. **The failure is entirely attributable to a named, measured property
of the input data, not to the construction.** That distinction is the whole
value of having run G0 first.

---

## Pre-registered kill criterion

Per the survey's own discipline and the standing rules of this line of work:

> If the measured loop holonomy on liquid FX is indistinguishable from the
> quote-synchronicity noise floor, and the currency-graph `circulation_ratio`
> carries no structure above its jackknife-corrected error bar, then the
> gauge-theoretic construction is a *reformulation* with no measurement value
> at daily resolution. Record that, stop, and reallocate to survey Stage 1
> (elliptope-constrained Fréchet mean — Marti et al.'s explicitly stated open
> computational problem).

This is written before any data is pulled, and it is binding.

## Honest cost

G0 is small: one loader, one bundle, four gates, no engine change — the
Hodge verb ships today. G1 is one loader and a time loop. G2 requires paid
data and is not scheduled. **No engine code is required to falsify the whole
programme**, which is the point: the cheapest possible test of the most
expensive possible claim.

## What this does not claim

Not a prediction system. Not an edge. Not a novel framing — Ilinski, Young and
Farinelli own that, cited above. The claim is a calibrated instrument for a
quantity the literature defines and no one measures, with its noise floor
stated and its failure mode pre-registered.
