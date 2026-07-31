"""Deterministic synthetic transaction dataset generator for the vs_stores benchmark.

Implements the DATASET clause of the LOCKED BENCHMARK PROTOCOL exactly:

  - deterministic seed 20260731
  - 100,000 rows: txn_id (key), merchant (12 categories), customer (2,000),
    hour (0-23 float, i.e. hour-of-day in [0, 24)), amount (float)
  - planted anomalies: 0.5% combination-anomalies in a 20,000-row labeled subset;
    each anomalous row has values individually in-distribution (amount normal for
    SOME merchant, hour normal for SOME merchant) but the (merchant, hour, amount)
    combination is far off that merchant-hour cohort pattern
  - labels.json maps txn_id -> 0/1
  - same files consumed by all three systems

Outputs (written next to this script):
  data.csv            100,000 rows: txn_id,merchant,customer,hour,amount
  labeled_subset.csv  20,000 rows (a subset of data.csv, same columns)
  labels.json         {txn_id: 0 or 1} for the 20,000 labeled rows (exactly 100 ones)
  dataset_spec.md     precise description of the generative model + planting mechanism,
                      including observed verification numbers from this run

Stdlib only (csv/json/math/random) — no numpy — so determinism depends only on
CPython's random module, which is stable across platforms for a fixed seed.
"""

import csv
import json
import math
import random
from pathlib import Path

SEED = 20260731
N_ROWS = 100_000
N_LABELED = 20_000
N_ANOM = 100          # exactly 0.5% of the labeled subset
N_CUSTOMERS = 2_000
DELTA_LOG = 1.2       # minimum |cohort log-mean gap| between the row's own
                      # merchant-hour cohort and the donor cohort (>= e^1.2 ~ 3.3x)
DONOR_Z_CLIP = 1.5    # donor amount draws truncated to |z| <= 1.5 so the planted
                      # amount is comfortably in-distribution for the donor merchant
MAX_DONOR_TRIES = 500

OUT_DIR = Path(__file__).resolve().parent

# Per-merchant generative parameters (hardcoded so the spec can list them exactly).
# (name, peak_hour, hour_sd, base_amount_usd, log_amp, log_sd)
MERCHANTS = [
    ("coffee",      8.0, 2.0,   5.5, 0.35, 0.18),
    ("grocery",    17.0, 3.0,  62.0, 0.30, 0.20),
    ("restaurant", 19.0, 2.5,  48.0, 0.45, 0.22),
    ("gas",        12.0, 4.0,  38.0, 0.15, 0.15),
    ("electronics",15.0, 3.0, 240.0, 0.40, 0.25),
    ("airline",    11.0, 5.0, 420.0, 0.25, 0.30),
    ("pharmacy",   14.0, 3.5,  22.0, 0.20, 0.20),
    ("streaming",  21.0, 3.0,  12.0, 0.30, 0.10),
    ("hotel",      16.0, 4.0, 180.0, 0.35, 0.28),
    ("clothing",   14.0, 3.0,  75.0, 0.30, 0.24),
    ("hardware",   10.0, 2.5,  55.0, 0.25, 0.20),
    ("bookstore",  13.0, 3.0,  18.0, 0.30, 0.18),
]
N_MERCHANTS = len(MERCHANTS)
assert N_MERCHANTS == 12


def wrapped_hour(rng: random.Random, peak: float, sd: float) -> float:
    """Hour-of-day drawn from a wrapped normal around the merchant's peak hour."""
    return rng.gauss(peak, sd) % 24.0


def cohort_log_mean(midx: int, hour: float) -> float:
    """Expected log-amount for merchant midx at hour-of-day `hour` (the cohort pattern)."""
    _, peak, _, base, amp, _ = MERCHANTS[midx]
    return math.log(base) + amp * math.cos(2.0 * math.pi * (hour - peak) / 24.0)


def truncated_gauss(rng: random.Random, mu: float, sd: float, zclip: float) -> float:
    while True:
        z = rng.gauss(0.0, 1.0)
        if abs(z) <= zclip:
            return mu + z * sd


def main() -> None:
    rng = random.Random(SEED)

    # ------------------------------------------------------------------ generate
    # rows[i] = [txn_id, merchant_name, customer, hour, amount, merchant_idx]
    rows = []
    for i in range(N_ROWS):
        midx = rng.randrange(N_MERCHANTS)
        name, peak, hour_sd, base, amp, log_sd = MERCHANTS[midx]
        hour = round(wrapped_hour(rng, peak, hour_sd), 3)
        log_amt = rng.gauss(cohort_log_mean(midx, hour), log_sd)
        amount = round(math.exp(log_amt), 2)
        customer = f"C{rng.randrange(N_CUSTOMERS):04d}"
        rows.append([f"T{i:06d}", name, customer, hour, amount, midx])

    # ------------------------------------------------------- labeled subset + plant
    labeled_idx = sorted(rng.sample(range(N_ROWS), N_LABELED))
    anom_idx = set(rng.sample(labeled_idx, N_ANOM))

    donors = {}  # row index -> donor merchant idx (for verification)
    for i in sorted(anom_idx):
        row = rows[i]
        b_idx = row[5]
        hour = row[3]                      # kept: drawn from B's own hour distribution
        mu_b = cohort_log_mean(b_idx, hour)
        planted = False
        for _ in range(MAX_DONOR_TRIES):
            a_idx = rng.randrange(N_MERCHANTS)
            if a_idx == b_idx:
                continue
            a_name, a_peak, a_hour_sd, a_base, a_amp, a_log_sd = MERCHANTS[a_idx]
            h_a = wrapped_hour(rng, a_peak, a_hour_sd)   # an hour normal for donor A
            mu_a = cohort_log_mean(a_idx, h_a)
            if abs(mu_a - mu_b) >= DELTA_LOG:
                log_amt = truncated_gauss(rng, mu_a, a_log_sd, DONOR_Z_CLIP)
                row[4] = round(math.exp(log_amt), 2)
                donors[i] = a_idx
                planted = True
                break
        if not planted:
            raise RuntimeError(f"no donor cohort found for row {i} (merchant {b_idx})")

    labels = {rows[i][0]: (1 if i in anom_idx else 0) for i in labeled_idx}

    # ------------------------------------------------------------------ write files
    def write_csv(path: Path, indices) -> None:
        with path.open("w", newline="", encoding="utf-8") as f:
            w = csv.writer(f)
            w.writerow(["txn_id", "merchant", "customer", "hour", "amount"])
            for i in indices:
                w.writerow(rows[i][:5])

    write_csv(OUT_DIR / "data.csv", range(N_ROWS))
    write_csv(OUT_DIR / "labeled_subset.csv", labeled_idx)
    with (OUT_DIR / "labels.json").open("w", encoding="utf-8") as f:
        json.dump(labels, f)

    # ------------------------------------------------------------------ verification
    normal_amounts = [r[4] for j, r in enumerate(rows) if j not in anom_idx]
    gmin, gmax = min(normal_amounts), max(normal_amounts)

    per_merchant_amt = {m[0]: [] for m in MERCHANTS}
    per_merchant_hour = {m[0]: [] for m in MERCHANTS}
    for j, r in enumerate(rows):
        if j not in anom_idx:
            per_merchant_amt[r[1]].append(r[4])
            per_merchant_hour[r[1]].append(r[3])
    amt_range = {m: (min(v), max(v)) for m, v in per_merchant_amt.items()}
    hour_range = {m: (min(v), max(v)) for m, v in per_merchant_hour.items()}

    n_pos = sum(labels.values())
    checks = []
    checks.append(("exactly 0.5% positives in labeled subset",
                   n_pos == N_ANOM and n_pos / N_LABELED == 0.005,
                   f"{n_pos}/{N_LABELED} = {n_pos / N_LABELED:.4%}"))

    in_global = all(gmin <= rows[i][4] <= gmax for i in anom_idx)
    checks.append(("every anomalous amount within global normal amount range",
                   in_global, f"global normal range [{gmin}, {gmax}]"))

    in_donor = all(
        amt_range[MERCHANTS[donors[i]][0]][0] <= rows[i][4] <= amt_range[MERCHANTS[donors[i]][0]][1]
        for i in anom_idx)
    checks.append(("every anomalous amount within its DONOR merchant's normal amount range",
                   in_donor, "amount normal for SOME merchant"))

    in_some = all(any(lo <= rows[i][4] <= hi for lo, hi in amt_range.values())
                  for i in anom_idx)
    checks.append(("every anomalous amount within SOME merchant's normal amount range",
                   in_some, ""))

    hour_ok = all(hour_range[rows[i][1]][0] <= rows[i][3] <= hour_range[rows[i][1]][1]
                  for i in anom_idx)
    checks.append(("every anomalous hour within its OWN merchant's normal hour range",
                   hour_ok, "hour normal for the row's own merchant"))

    # combination-gap check: recompute the realized gap for each planted row
    gaps = []
    for i in sorted(anom_idx):
        b_idx = rows[i][5]
        mu_b = cohort_log_mean(b_idx, rows[i][3])
        gaps.append(abs(math.log(rows[i][4]) - mu_b))
    checks.append((f"every planted (merchant,hour,amount) combination far off its own cohort "
                   f"(|log-amount - cohort log-mean| listed; enforced donor gap >= {DELTA_LOG})",
                   min(gaps) >= DELTA_LOG - DONOR_Z_CLIP * max(m[5] for m in MERCHANTS),
                   f"min observed |log gap| = {min(gaps):.3f}, max = {max(gaps):.3f}"))

    all_pass = all(ok for _, ok, _ in checks)
    print(f"rows written: {N_ROWS}  labeled subset: {N_LABELED}  positives: {n_pos}")
    for name, ok, detail in checks:
        print(f"  [{'PASS' if ok else 'FAIL'}] {name}" + (f"  ({detail})" if detail else ""))
    if not all_pass:
        raise SystemExit("SANITY CHECK FAILED")

    # ------------------------------------------------------------------ dataset_spec.md
    merchant_table = "\n".join(
        f"| {m[0]} | {m[1]:.0f} | {m[2]:.1f} | {m[3]:.1f} | {m[4]:.2f} | {m[5]:.2f} |"
        for m in MERCHANTS)
    spec = f"""# Dataset spec — vs_stores benchmark (LOCKED PROTOCOL, dataset clause)

Deterministic seed: **{SEED}** (single `random.Random({SEED})` instance, stdlib only;
every draw — merchant, hour, customer, amount, subset sampling, anomaly planting —
comes from this one stream in fixed program order, so the files are byte-reproducible
by rerunning `gen_dataset.py`).

## Files

| file | rows | contents |
|---|---|---|
| `data.csv` | {N_ROWS:,} | `txn_id,merchant,customer,hour,amount` |
| `labeled_subset.csv` | {N_LABELED:,} | subset of `data.csv` (identical rows, same columns) |
| `labels.json` | {N_LABELED:,} keys | `txn_id -> 0/1`; exactly {N_ANOM} ones ({N_ANOM / N_LABELED:.1%}) |

Same files are consumed by all three systems (gigi, sqlite, duckdb).

## Schema

- `txn_id` — string key `T000000` … `T{N_ROWS - 1:06d}` (unique).
- `merchant` — one of 12 categories, drawn uniformly.
- `customer` — `C0000` … `C{N_CUSTOMERS - 1:04d}` ({N_CUSTOMERS:,} customers), drawn uniformly.
- `hour` — hour-of-day float in [0, 24), 3 decimals: wrapped normal
  `N(peak_m, hour_sd_m) mod 24` per merchant.
- `amount` — USD float, 2 decimals: log-normal with a merchant-AND-hour dependent
  mean (the cohort pattern):
  `log(amount) ~ N( log(base_m) + amp_m * cos(2*pi*(hour - peak_m)/24), log_sd_m )`.

## Merchant parameters (hardcoded)

| merchant | peak_hour | hour_sd | base_$ | log_amp | log_sd |
|---|---|---|---|---|---|
{merchant_table}

## Labeled subset

{N_LABELED:,} row indices sampled without replacement from the {N_ROWS:,} rows
(`rng.sample`), sorted ascending. The subset rows are the SAME rows as in
`data.csv` (same txn_ids, same values).

## Planting mechanism (combination anomalies)

Exactly {N_ANOM} rows (0.5% of the labeled subset, `rng.sample` of the subset
indices) are turned into **combination anomalies**, processed in ascending row
order:

1. The row keeps its merchant B, its customer, and its **hour** — the hour was
   drawn from B's own wrapped-normal hour distribution, so the hour is normal
   for the row's own merchant.
2. A **donor** merchant A != B and a donor hour `h_A ~ N(peak_A, hour_sd_A) mod 24`
   are drawn (rejection-sampled) until the donor cohort's expected log-amount
   differs from the row's own (B, hour) cohort expectation by at least
   **DELTA_LOG = {DELTA_LOG}** in absolute value (a >= e^{DELTA_LOG} ~ 3.3x
   multiplicative gap between cohort means).
3. The planted amount is drawn from the donor cohort:
   `log(amount) ~ N( mu_A(h_A), log_sd_A )` **truncated to |z| <= {DONOR_Z_CLIP}**,
   so the amount is comfortably in-distribution for merchant A.

Result: amount is normal for SOME merchant (the donor A), hour is normal for
SOME merchant (the row's own B), but the (merchant, hour, amount) combination
sits >= {DELTA_LOG} - {DONOR_Z_CLIP}*log_sd_A away (in log space) from the
row's own merchant-hour cohort mean — i.e. far off that cohort pattern, while
no single column value is out of distribution on its own. Labels: planted rows
-> 1, all other labeled-subset rows -> 0.

## Verification (this run)

- positives in labeled subset: **{n_pos} / {N_LABELED:,} = {n_pos / N_LABELED:.2%}** (exact)
- global normal amount range: [{gmin}, {gmax}]; all {N_ANOM} planted amounts inside: **{in_global}**
- every planted amount inside its donor merchant's observed normal amount range: **{in_donor}**
- every planted amount inside SOME merchant's observed normal amount range: **{in_some}**
- every planted hour inside its own merchant's observed normal hour range: **{hour_ok}**
- planted |log(amount) - own-cohort log-mean| gap: min **{min(gaps):.3f}**, max **{max(gaps):.3f}**
"""
    (OUT_DIR / "dataset_spec.md").write_text(spec, encoding="utf-8")
    print("wrote data.csv, labeled_subset.csv, labels.json, dataset_spec.md")


if __name__ == "__main__":
    main()
