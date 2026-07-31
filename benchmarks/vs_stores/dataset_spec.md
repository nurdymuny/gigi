# Dataset spec — vs_stores benchmark (LOCKED PROTOCOL, dataset clause)

Deterministic seed: **20260731** (single `random.Random(20260731)` instance, stdlib only;
every draw — merchant, hour, customer, amount, subset sampling, anomaly planting —
comes from this one stream in fixed program order, so the files are byte-reproducible
by rerunning `gen_dataset.py`).

## Files

| file | rows | contents |
|---|---|---|
| `data.csv` | 100,000 | `txn_id,merchant,customer,hour,amount` |
| `labeled_subset.csv` | 20,000 | subset of `data.csv` (identical rows, same columns) |
| `labels.json` | 20,000 keys | `txn_id -> 0/1`; exactly 100 ones (0.5%) |

Same files are consumed by all three systems (gigi, sqlite, duckdb).

## Schema

- `txn_id` — string key `T000000` … `T099999` (unique).
- `merchant` — one of 12 categories, drawn uniformly.
- `customer` — `C0000` … `C1999` (2,000 customers), drawn uniformly.
- `hour` — hour-of-day float in [0, 24), 3 decimals: wrapped normal
  `N(peak_m, hour_sd_m) mod 24` per merchant.
- `amount` — USD float, 2 decimals: log-normal with a merchant-AND-hour dependent
  mean (the cohort pattern):
  `log(amount) ~ N( log(base_m) + amp_m * cos(2*pi*(hour - peak_m)/24), log_sd_m )`.

## Merchant parameters (hardcoded)

| merchant | peak_hour | hour_sd | base_$ | log_amp | log_sd |
|---|---|---|---|---|---|
| coffee | 8 | 2.0 | 5.5 | 0.35 | 0.18 |
| grocery | 17 | 3.0 | 62.0 | 0.30 | 0.20 |
| restaurant | 19 | 2.5 | 48.0 | 0.45 | 0.22 |
| gas | 12 | 4.0 | 38.0 | 0.15 | 0.15 |
| electronics | 15 | 3.0 | 240.0 | 0.40 | 0.25 |
| airline | 11 | 5.0 | 420.0 | 0.25 | 0.30 |
| pharmacy | 14 | 3.5 | 22.0 | 0.20 | 0.20 |
| streaming | 21 | 3.0 | 12.0 | 0.30 | 0.10 |
| hotel | 16 | 4.0 | 180.0 | 0.35 | 0.28 |
| clothing | 14 | 3.0 | 75.0 | 0.30 | 0.24 |
| hardware | 10 | 2.5 | 55.0 | 0.25 | 0.20 |
| bookstore | 13 | 3.0 | 18.0 | 0.30 | 0.18 |

## Labeled subset

20,000 row indices sampled without replacement from the 100,000 rows
(`rng.sample`), sorted ascending. The subset rows are the SAME rows as in
`data.csv` (same txn_ids, same values).

## Planting mechanism (combination anomalies)

Exactly 100 rows (0.5% of the labeled subset, `rng.sample` of the subset
indices) are turned into **combination anomalies**, processed in ascending row
order:

1. The row keeps its merchant B, its customer, and its **hour** — the hour was
   drawn from B's own wrapped-normal hour distribution, so the hour is normal
   for the row's own merchant.
2. A **donor** merchant A != B and a donor hour `h_A ~ N(peak_A, hour_sd_A) mod 24`
   are drawn (rejection-sampled) until the donor cohort's expected log-amount
   differs from the row's own (B, hour) cohort expectation by at least
   **DELTA_LOG = 1.2** in absolute value (a >= e^1.2 ~ 3.3x
   multiplicative gap between cohort means).
3. The planted amount is drawn from the donor cohort:
   `log(amount) ~ N( mu_A(h_A), log_sd_A )` **truncated to |z| <= 1.5**,
   so the amount is comfortably in-distribution for merchant A.

Result: amount is normal for SOME merchant (the donor A), hour is normal for
SOME merchant (the row's own B), but the (merchant, hour, amount) combination
sits >= 1.2 - 1.5*log_sd_A away (in log space) from the
row's own merchant-hour cohort mean — i.e. far off that cohort pattern, while
no single column value is out of distribution on its own. Labels: planted rows
-> 1, all other labeled-subset rows -> 0.

## Verification (this run)

- positives in labeled subset: **100 / 20,000 = 0.50%** (exact)
- global normal amount range: [3.46, 1509.41]; all 100 planted amounts inside: **True**
- every planted amount inside its donor merchant's observed normal amount range: **True**
- every planted amount inside SOME merchant's observed normal amount range: **True**
- every planted hour inside its own merchant's observed normal hour range: **True**
- planted |log(amount) - own-cohort log-mean| gap: min **0.993**, max **4.379**
