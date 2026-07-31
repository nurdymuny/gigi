"""Shared plumbing for the gigi vs sqlite vs duckdb store benchmark.

LOCKED PROTOCOL (2026-07-31). Every runner imports its constants, timing
helpers, the ONE shared average-precision implementation, and the results-JSON
writer from here so no system can drift onto a private definition of any
metric.

Dataset files (written by gen_dataset.py, seed 20260731, consumed identically
by all three systems — see dataset_spec.md):

    data.csv             100,000 rows: txn_id,merchant,customer,hour,amount
    labeled_subset.csv    20,000 rows (subset of data.csv, same columns)
    labels.json           txn_id -> 0/1 for the labeled subset (exactly 100 ones)

The 2,000 point-query ids are derived IN CODE from an independent deterministic
stream (seed SEED+1) — same ids, same order, for every system.

Cell schema (identical for every system, enforced by the cell_* builders):

    results_<system>.json
    ├── system            "gigi" | "sqlite" | "duckdb"
    ├── protocol          locked constants used for the run
    ├── environment       host, CPU, OS, python, git sha, timestamp
    ├── versions          per-system version strings
    ├── cells
    │   ├── ingest        {rows, unit:"rows_per_sec", reps[3], median}
    │   ├── point_query   {n_queries, unit:"ms", reps[3]{p50_ms,p95_ms},
    │   │                  p50_ms_median, p95_ms_median}
    │   ├── aggregate     {unit:"ms", n_groups, method, reps_wall_ms[3],
    │   │                  wall_ms_median}
    │   └── anomaly       {unit:"ms", method, reps_wall_ms[3], wall_ms_median,
    │                      pr_auc, client_lines_of_code, loc_basis,
    │                      n_scored, n_positive}
    └── disclosures       every honest asymmetry, as measured, no softening
"""
from __future__ import annotations

import json
import math
import os
import platform
import random
import subprocess
import sys
import time
from pathlib import Path

# ── locked protocol constants ────────────────────────────────────────────────
SEED = 20260731
N_ROWS = 100_000
N_SUBSET = 20_000
N_ANOM = 100                 # 0.5% of the labeled subset
N_POINT_QUERIES = 2_000
INSERT_BATCH = 1_000         # gigi HTTP batch size (rule 2A)
HOUR_BUCKET_HOURS = 2.0      # SQL baseline hour bucketing (rule 2D)
SCAN_BUDGET = 0.05           # gigi /scan default budget (does not affect PR-AUC)
REPS = 3                     # every timed cell: 1 untimed warmup + 3 timed
GIGI_HOST = "127.0.0.1"
GIGI_PORT = 3143
GIGI_BASE = f"http://{GIGI_HOST}:{GIGI_PORT}"

# 12 merchant categories — must match gen_dataset.py / dataset_spec.md.
MERCHANTS = [
    "coffee", "grocery", "restaurant", "gas", "electronics", "airline",
    "pharmacy", "streaming", "hotel", "clothing", "hardware", "bookstore",
]

HERE = Path(__file__).resolve().parent
TXN_CSV = HERE / "data.csv"
SUBSET_CSV = HERE / "labeled_subset.csv"
LABELS_JSON = HERE / "labels.json"

CSV_HEADER = ["txn_id", "merchant", "customer", "hour", "amount"]


def txn_id(i: int) -> str:
    return f"T{i:06d}"


# ── dataset loading (identical files for all three systems, rule 1) ──────────
def require_dataset() -> None:
    missing = [p.name for p in (TXN_CSV, SUBSET_CSV, LABELS_JSON) if not p.exists()]
    if missing:
        sys.exit(
            f"dataset files missing: {missing} — run `python gen_dataset.py` first "
            f"(deterministic, seed {SEED})"
        )


def _read_csv(path: Path):
    import csv

    with open(path, newline="", encoding="utf-8") as f:
        r = csv.reader(f)
        header = next(r)
        assert header == CSV_HEADER, f"{path.name}: unexpected header {header}"
        return [(t, m, c, float(h), float(a)) for t, m, c, h, a in r]


def read_transactions():
    """All 100k rows as tuples (txn_id, merchant, customer, hour, amount).

    CSV parse happens here, OUTSIDE any timed window, so gigi and sqlite start
    their timed ingest from identical in-memory Python rows. (duckdb's timed
    ingest reads the CSV itself — its idiomatic bulk path — disclosed.)
    """
    rows = _read_csv(TXN_CSV)
    assert len(rows) == N_ROWS
    return rows


def read_subset():
    """The 20k labeled-subset rows (same values as data.csv)."""
    rows = _read_csv(SUBSET_CSV)
    assert len(rows) == N_SUBSET
    return rows


def load_labels() -> dict[str, int]:
    with open(LABELS_JSON, encoding="utf-8") as f:
        labels = {k: int(v) for k, v in json.load(f).items()}
    assert len(labels) == N_SUBSET, f"labels: expected {N_SUBSET}, got {len(labels)}"
    assert sum(labels.values()) == N_ANOM, "labels: planted-anomaly count drifted"
    return labels


def point_query_ids() -> list[str]:
    """The 2,000 point-lookup txn_ids — deterministic (seed SEED+1), an
    independent stream from the dataset's own RNG so the anomaly plant cannot
    leak into the lookup set. Same ids in the same order for every system."""
    rng = random.Random(SEED + 1)
    return [txn_id(i) for i in rng.sample(range(N_ROWS), N_POINT_QUERIES)]


# ── timing (rule 3: 1 untimed warmup + REPS timed, report median) ────────────
def warmup_plus_reps(fn, reps: int = REPS) -> list:
    """Run fn once untimed (warmup, result discarded), then `reps` times;
    return the rep results.

    fn does its own perf_counter measurement and returns whatever the cell
    records (rows/sec, latency list, wall ms, ...). GC is collected then
    disabled around every invocation, uniformly for all systems, so collector
    pauses cannot land in one system's cell and not another's.
    """
    import gc

    out = []
    for i in range(reps + 1):
        gc.collect()
        gc.disable()
        try:
            res = fn()
        finally:
            gc.enable()
        if i > 0:
            out.append(res)
    return out


def wall_ms(fn):
    """(elapsed_ms, fn_result)."""
    t0 = time.perf_counter()
    out = fn()
    return (time.perf_counter() - t0) * 1000.0, out


def median(xs) -> float:
    s = sorted(xs)
    n = len(s)
    mid = n // 2
    return float(s[mid]) if n % 2 else (s[mid - 1] + s[mid]) / 2.0


def percentile(xs, p: float) -> float:
    """Linear-interpolation percentile (same convention for every system)."""
    s = sorted(xs)
    if not s:
        raise ValueError("empty sample")
    k = (len(s) - 1) * (p / 100.0)
    lo, hi = math.floor(k), math.ceil(k)
    if lo == hi:
        return float(s[int(k)])
    return s[lo] + (s[hi] - s[lo]) * (k - lo)


# ── the ONE shared PR-AUC (average precision, step interpolation) ────────────
def average_precision(labels: dict[str, int], scores: dict[str, float]) -> float:
    """AP = sum over ranked positives of (precision at that rank) / n_pos.

    Step interpolation — same definition as sklearn's
    average_precision_score. Every labeled id MUST have a score (silent drops
    would be a fairness hole — a system cannot improve PR-AUC by declining to
    score hard rows). Ties are broken deterministically by txn_id; the same
    policy applies to every system (disclosed: within a tie the order is
    arbitrary but identical across systems).
    """
    missing = [k for k in labels if k not in scores]
    if missing:
        raise ValueError(
            f"{len(missing)} labeled ids have no score (e.g. {missing[:3]}) — "
            "every system must score every labeled row"
        )
    ranked = sorted(labels, key=lambda k: (-scores[k], k))
    n_pos = sum(labels.values())
    if n_pos == 0:
        raise ValueError("no positive labels")
    ap = 0.0
    tp = 0
    for rank, k in enumerate(ranked, 1):
        if labels[k] == 1:
            tp += 1
            ap += tp / rank
    return ap / n_pos


# ── lines-of-code metric (rule 2D) ───────────────────────────────────────────
def count_loc(src: str) -> int:
    """Non-blank, non-comment lines. The SAME counting rule is applied to the
    gigi python client function and to the expert SQL baseline text."""
    n = 0
    for line in src.splitlines():
        t = line.strip()
        if t and not t.startswith("#") and not t.startswith("--"):
            n += 1
    return n


# ── environment + results writer (rule 5) ────────────────────────────────────
def env_info() -> dict:
    repo_root = HERE.parent.parent
    try:
        sha = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=repo_root, text=True
        ).strip()
    except Exception:
        sha = "unknown"
    return {
        "hostname": platform.node(),
        "os": f"{platform.system()} {platform.release()} ({platform.version()})",
        "cpu": os.environ.get("PROCESSOR_IDENTIFIER", platform.processor()),
        "machine": platform.machine(),
        "python": sys.version.split()[0],
        "gigi_git_sha": sha,
        "timestamp_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "single_node": True,
        "note": "single Windows laptop, serial runs, nothing else running (operator-attested)",
    }


def protocol_info() -> dict:
    return {
        "seed": SEED,
        "n_rows": N_ROWS,
        "n_subset": N_SUBSET,
        "n_anomalies": N_ANOM,
        "n_point_queries": N_POINT_QUERIES,
        "gigi_insert_batch": INSERT_BATCH,
        "hour_bucket_hours": HOUR_BUCKET_HOURS,
        "scan_budget": SCAN_BUDGET,
        "reps_per_cell": REPS,
        "warmups_per_cell": 1,
        "cell_statistic": "median of reps",
        "pr_auc": "average precision, step interpolation, shared eval_common.average_precision",
        "timing_shape": "gigi over HTTP at 127.0.0.1 (its real deployment shape); "
        "sqlite/duckdb in-process (their real shape) — asymmetry disclosed, not hidden",
    }


# ── cell builders: identical keys per system, by construction ────────────────
def cell_ingest(rows_per_sec_reps: list[float]) -> dict:
    return {
        "rows": N_ROWS,
        "unit": "rows_per_sec",
        "reps": [round(x, 1) for x in rows_per_sec_reps],
        "median": round(median(rows_per_sec_reps), 1),
    }


def cell_point_query(latency_reps_ms: list[list[float]]) -> dict:
    reps = [
        {"p50_ms": round(percentile(lats, 50), 4), "p95_ms": round(percentile(lats, 95), 4)}
        for lats in latency_reps_ms
    ]
    return {
        "n_queries": N_POINT_QUERIES,
        "unit": "ms",
        "reps": reps,
        "p50_ms_median": round(median([r["p50_ms"] for r in reps]), 4),
        "p95_ms_median": round(median([r["p95_ms"] for r in reps]), 4),
    }


def cell_aggregate(wall_ms_reps: list[float], n_groups: int, method: str) -> dict:
    return {
        "unit": "ms",
        "n_groups": n_groups,
        "method": method,
        "reps_wall_ms": [round(x, 2) for x in wall_ms_reps],
        "wall_ms_median": round(median(wall_ms_reps), 2),
    }


def cell_anomaly(
    wall_ms_reps: list[float],
    pr_auc: float,
    loc: int,
    loc_basis: str,
    method: str,
    n_scored: int,
) -> dict:
    return {
        "unit": "ms",
        "method": method,
        "reps_wall_ms": [round(x, 2) for x in wall_ms_reps],
        "wall_ms_median": round(median(wall_ms_reps), 2),
        "pr_auc": round(pr_auc, 4),
        "client_lines_of_code": loc,
        "loc_basis": loc_basis,
        "n_scored": n_scored,
        "n_positive": N_ANOM,
    }


def write_results(system: str, versions: dict, cells: dict, disclosures: list[str]) -> Path:
    required = {"ingest", "point_query", "aggregate", "anomaly"}
    assert set(cells) == required, f"cells must be exactly {required}, got {set(cells)}"
    out = {
        "system": system,
        "protocol": protocol_info(),
        "environment": env_info(),
        "versions": versions,
        "cells": cells,
        "disclosures": disclosures,
    }
    path = HERE / f"results_{system}.json"
    with open(path, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2)
    print(f"\nwrote {path}")
    return path
