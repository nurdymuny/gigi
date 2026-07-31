"""Scaling-sweep dataset generator + shared sweep module — ROUND 2 ADDENDUM.

ROUND 2 LOCKED PROTOCOL ADDENDUM (2026-07-31), clause 2 (SCALING SWEEP):
point-query p50 at N = 10,000 / 100,000 / 1,000,000 rows for (a) gigi
embedded, (b) gigi over HTTP, (c) sqlite in-process — the O(1) claim measured
as a CURVE, plus the ratio latency(1M)/latency(10k) per system. Flat ~1.0x
supports O(1); growth is reported as measured (and for gigi embedded flagged
as a REGRESSION finding, not smoothed — that flagging lives in the embedded
lane's runner).

This module does three jobs:

1. EMITS THE THREE DATASETS into %TEMP%/gigi_vs_stores_scale — NEVER into the
   repo or any OneDrive-synced folder (guarded in code, the script refuses):

     data_100k.csv  byte-for-byte copy of the round-1 data.csv (header + row
                    count verified = 100,000 before copying; if data.csv is
                    missing run `python gen_dataset.py` first).
     data_10k.csv   PREFIX RULE (locked): the byte-exact first 10,000 data
                    rows of data_100k.csv — data.csv is written in
                    generation-index order, so these are exactly txn_ids
                    T000000..T009999 in file order.
     data_1m.csv    1,000,000 rows from the SAME generative model as
                    gen_dataset.py (identical per-row draw order — merchant,
                    hour, log-amount, customer — from a fresh
                    random.Random(20260731)), streamed to disk. NO anomaly
                    plant and NO labeled subset: the sweep times point lookups
                    only, values are never scored. Because the draw order is
                    identical and the round-1 plant is a post-pass, rows
                    0..99,999 of data_1m.csv coincide byte-for-byte with
                    data.csv EXCEPT the 100 planted amounts — verified below
                    (exactly 100 mismatching lines, all planted txn_ids,
                    amount field only).

2. WRITES THE PER-N POINT-QUERY ID FILES (2,000 ids each, one per line, file
   order = query order):

     ids(N) = random.Random(20260731 + 1).sample(range(N), 2000)

   — the same seed+1 stream rule as round 1; at N=100,000 the list is
   asserted equal to eval_common.point_query_ids(). The Rust embedded-lane
   example does NOT read these files: it re-derives the ids in-code via a
   bit-exact MT19937/sample() reimplementation (audit-verified equal to
   these files at all three Ns). The files are the cross-language
   determinism RECEIPT the cross-check runs against.

3. IS THE SHARED SWEEP MODULE imported by run_scale_sqlite.py and
   run_scale_gigi_http.py: scale constants, per-N id derivation, streaming
   CSV reader, the microsecond point-query cell builder and the
   results_scale_<system>.json writer — one definition, so no runner can
   drift onto a private version of any metric.

scale_manifest.json (row counts + SHA-256 of every emitted file) is the
byte-reproducibility receipt.
"""
from __future__ import annotations

import csv
import hashlib
import json
import math
import os
import random
import shutil
import sys
import tempfile
import time
from pathlib import Path

import gen_dataset  # round-1 generative model: MERCHANTS, wrapped_hour, cohort_log_mean
from eval_common import (
    CSV_HEADER, HERE, N_POINT_QUERIES, REPS, SEED, env_info, median,
    percentile, point_query_ids, require_dataset, txn_id,
)

# ── locked sweep constants ───────────────────────────────────────────────────
SCALE_NS = [10_000, 100_000, 1_000_000]
_LABELS = {10_000: "10k", 100_000: "100k", 1_000_000: "1m"}

# %TEMP%/gigi_vs_stores_scale by default; VS_STORES_WORKDIR relocates the
# parent (same env knob as round 1). Never the repo, never OneDrive.
SCALE_DIR = (
    Path(os.environ.get("VS_STORES_WORKDIR", tempfile.gettempdir()))
    / "gigi_vs_stores_scale"
)
MANIFEST = SCALE_DIR / "scale_manifest.json"

ROUND1_DATA = HERE / "data.csv"


def scale_label(n: int) -> str:
    return _LABELS[n]


def scale_csv_path(n: int) -> Path:
    return SCALE_DIR / f"data_{scale_label(n)}.csv"


def scale_ids_path(n: int) -> Path:
    return SCALE_DIR / f"point_ids_{scale_label(n)}.txt"


def scale_point_ids(n: int) -> list[str]:
    """The 2,000 point-lookup ids for dataset size n — deterministic, seed
    SEED+1, same construction as round 1 (identical list at n=100,000).
    Same ids in the same order for every system at a given N."""
    rng = random.Random(SEED + 1)
    return [txn_id(i) for i in rng.sample(range(n), N_POINT_QUERIES)]


def assert_outside_repo_and_onedrive(p: Path) -> None:
    """Round-2 addendum: scale data NEVER goes into the repo or OneDrive."""
    rp = p.resolve()
    repo_root = Path(__file__).resolve().parents[2]
    if rp == repo_root or repo_root in rp.parents:
        sys.exit(f"refusing: scale dir {rp} is inside the repo {repo_root}")
    if "onedrive" in str(rp).lower():
        sys.exit(f"refusing: scale dir {rp} is inside a OneDrive-synced path")


def require_scale_files(ns=SCALE_NS) -> None:
    missing = [scale_csv_path(n).name for n in ns if not scale_csv_path(n).exists()]
    if missing:
        sys.exit(
            f"scale dataset files missing in {SCALE_DIR}: {missing} — run "
            f"`python gen_scale.py` first (deterministic, seed {SEED})"
        )


def stream_rows(path: Path):
    """Yield (txn_id, merchant, customer, hour, amount) tuples one at a time —
    the runners ingest 1M rows without materializing them all in memory."""
    with open(path, newline="", encoding="utf-8") as f:
        r = csv.reader(f)
        header = next(r)
        assert header == CSV_HEADER, f"{path.name}: unexpected header {header}"
        for t, m, c, h, a in r:
            yield t, m, c, float(h), float(a)


# ── shared sweep cell builder + results writer (both runners import these) ───
def scale_point_cell(n: int, lat_us_reps: list[list[float]]) -> dict:
    """Per-N point-query cell. Latencies in MICROSECONDS (round-2 unit)."""
    reps = [
        {"p50_us": round(percentile(l, 50), 2), "p95_us": round(percentile(l, 95), 2)}
        for l in lat_us_reps
    ]
    return {
        "n_rows": n,
        "n_queries": N_POINT_QUERIES,
        "unit": "us",
        "reps": reps,
        "p50_us_median": round(median([r["p50_us"] for r in reps]), 2),
        "p95_us_median": round(median([r["p95_us"] for r in reps]), 2),
    }


def scale_protocol_info() -> dict:
    return {
        "round": 2,
        "arm": "scaling_sweep",
        "seed": SEED,
        "scale_ns": SCALE_NS,
        "n_point_queries": N_POINT_QUERIES,
        "reps_per_cell": REPS,
        "warmups_per_cell": 1,
        "cell_statistic": "median of reps",
        "unit": "microseconds",
        "id_rule": (
            "per-N ids = random.Random(seed+1).sample(range(N), 2000) — same "
            "stream rule as round 1; the N=100k list equals round 1's "
            "point_query_ids (asserted in gen_scale.py); id files written to "
            "the scale dir for the Rust embedded lane"
        ),
        "timing": (
            "only point queries are timed in the sweep; ingest is untimed "
            "setup (round-2 addendum clause 2 / 3). warm + sequential + "
            "strictly serial (clause 4)."
        ),
        "interpretation": (
            "ratio latency(1M)/latency(10k) per system: flat ~1.0x supports "
            "O(1); growth is reported as measured, never smoothed; for gigi "
            "embedded any clear growth is a REGRESSION finding (flagged in "
            "the embedded lane's runner)"
        ),
        "data_dir": str(SCALE_DIR),
    }


def write_scale_results(system: str, versions: dict, cells: dict,
                        disclosures: list[str]) -> Path:
    required = {scale_label(n) for n in SCALE_NS}
    assert set(cells) == required, f"cells must be exactly {required}, got {set(cells)}"
    r50 = cells["1m"]["p50_us_median"] / cells["10k"]["p50_us_median"]
    r95 = cells["1m"]["p95_us_median"] / cells["10k"]["p95_us_median"]
    out = {
        "system": system,
        "protocol": scale_protocol_info(),
        "environment": env_info(),
        "versions": versions,
        "cells": cells,
        "scaling": {
            "p50_ratio_1m_over_10k": round(r50, 3),
            "p95_ratio_1m_over_10k": round(r95, 3),
            "note": "flat ~1.0x supports O(1); growth reported as measured, not smoothed",
        },
        "disclosures": disclosures,
    }
    path = HERE / f"results_scale_{system}.json"
    with open(path, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2)
    print(f"\nwrote {path}")
    print(f"  p50 ratio 1M/10k: {r50:.3f}x   p95 ratio 1M/10k: {r95:.3f}x")
    return path


# ── generation ───────────────────────────────────────────────────────────────
def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _count_data_lines(path: Path) -> int:
    with open(path, "rb") as f:
        return sum(1 for _ in f) - 1  # minus header


def _copy_100k() -> None:
    """data_100k.csv = byte-for-byte copy of round-1 data.csv, verified first."""
    with open(ROUND1_DATA, newline="", encoding="utf-8") as f:
        header = next(csv.reader(f))
    assert header == CSV_HEADER, f"data.csv: unexpected header {header}"
    n = _count_data_lines(ROUND1_DATA)
    if n != 100_000:
        sys.exit(f"data.csv has {n} rows, expected 100,000 — rerun gen_dataset.py")
    shutil.copyfile(ROUND1_DATA, scale_csv_path(100_000))
    print(f"data_100k.csv: copied from data.csv ({n:,} rows, verified)")


def _write_10k_prefix() -> None:
    """PREFIX RULE (locked, stated here and in the module docstring):
    data_10k.csv = the byte-exact first 10,000 data rows of data_100k.csv,
    i.e. txn_ids T000000..T009999 in file order."""
    src, dst = scale_csv_path(100_000), scale_csv_path(10_000)
    with open(src, "rb") as fin, open(dst, "wb") as fout:
        for i, line in enumerate(fin):
            if i > 10_000:  # header + 10,000 data rows
                break
            fout.write(line)
    with open(dst, "rb") as f:
        lines = f.readlines()
    assert len(lines) == 10_001
    assert lines[1].startswith(b"T000000,") and lines[-1].startswith(b"T009999,"), \
        "prefix rule violated: data.csv is not in generation-index order"
    print("data_10k.csv: byte-exact 10,000-row prefix of data_100k.csv "
          "(txn_ids T000000..T009999)")


def _generate_1m() -> None:
    """Same generative model + per-row draw order as gen_dataset.main, fresh
    random.Random(SEED), extended to 1,000,000 rows, streamed. No plant."""
    rng = random.Random(SEED)
    n_rows = 1_000_000
    path = scale_csv_path(n_rows)
    t0 = time.perf_counter()
    with open(path, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(CSV_HEADER)
        for i in range(n_rows):
            # EXACT draw order of gen_dataset.main: merchant, hour, log-amount,
            # customer — one shared stream, so the first 100k rows reproduce
            # data.csv's pre-plant draws.
            midx = rng.randrange(gen_dataset.N_MERCHANTS)
            name, peak, hour_sd, base, amp, log_sd = gen_dataset.MERCHANTS[midx]
            hour = round(gen_dataset.wrapped_hour(rng, peak, hour_sd), 3)
            log_amt = rng.gauss(gen_dataset.cohort_log_mean(midx, hour), log_sd)
            amount = round(math.exp(log_amt), 2)
            customer = f"C{rng.randrange(gen_dataset.N_CUSTOMERS):04d}"
            w.writerow([txn_id(i), name, customer, hour, amount])
            if (i + 1) % 200_000 == 0:
                print(f"  data_1m.csv: {i + 1:,} rows "
                      f"({time.perf_counter() - t0:.1f}s)")
    print(f"data_1m.csv: {n_rows:,} rows in {time.perf_counter() - t0:.1f}s")


def _verify_1m_prefix_vs_data_csv() -> None:
    """Receipt: rows 0..99,999 of data_1m.csv equal data.csv byte-for-byte
    EXCEPT the 100 round-1 planted amounts (plant = post-pass in
    gen_dataset.py). Exactly 100 mismatching lines, all planted txn_ids,
    first four fields identical (amount-only difference)."""
    with open(HERE / "labels.json", encoding="utf-8") as f:
        planted = {k for k, v in json.load(f).items() if int(v) == 1}
    assert len(planted) == 100
    mismatches = []
    with open(ROUND1_DATA, "rb") as f100k, open(scale_csv_path(1_000_000), "rb") as f1m:
        for i, (a, b) in enumerate(zip(f100k, f1m)):
            if a != b:
                fa, fb = a.decode().split(","), b.decode().split(",")
                assert fa[0] == fb[0] and fa[:4] == fb[:4], \
                    f"line {i}: non-amount field differs between data.csv and data_1m.csv"
                assert fa[0] in planted, \
                    f"line {i}: unplanted row {fa[0]} differs — generative model drifted"
                mismatches.append(fa[0])
    ok = len(mismatches) == 100
    print(f"  [{'PASS' if ok else 'FAIL'}] data_1m.csv rows 0..99,999 == data.csv "
          f"except the {len(mismatches)} planted amounts (expected exactly 100)")
    if not ok:
        raise SystemExit("SCALE SANITY CHECK FAILED")


def _write_id_files() -> dict:
    """Per-N id files — the cross-language determinism receipt. The Rust
    embedded lane re-derives the ids in-code (bit-exact MT19937/sample()
    reimplementation, verified equal to these files at all Ns); it does not
    read them. One id per line; file order = query order."""
    r1 = point_query_ids()
    assert scale_point_ids(100_000) == r1, \
        "id-rule drift: scale_point_ids(100k) != round-1 point_query_ids"
    print("point ids: scale_point_ids(100k) == round-1 point_query_ids  [PASS]")
    out = {}
    for n in SCALE_NS:
        ids = scale_point_ids(n)
        p = scale_ids_path(n)
        p.write_text("\n".join(ids) + "\n", encoding="utf-8")
        out[scale_label(n)] = {
            "path": str(p), "n": len(ids), "sha256": _sha256(p),
            "rule": f"random.Random({SEED}+1).sample(range({n}), {N_POINT_QUERIES})",
        }
        print(f"  point_ids_{scale_label(n)}.txt: {len(ids)} ids "
              f"[{ids[0]} .. {ids[-1]}]")
    return out


def main() -> None:
    assert_outside_repo_and_onedrive(SCALE_DIR)
    require_dataset()  # round-1 data.csv + labels.json needed (copy + receipt)
    SCALE_DIR.mkdir(parents=True, exist_ok=True)
    print(f"scale dir: {SCALE_DIR} (outside repo/OneDrive, guarded)")

    _copy_100k()
    _write_10k_prefix()
    _generate_1m()
    _verify_1m_prefix_vs_data_csv()
    id_meta = _write_id_files()

    files_meta = {}
    for n in SCALE_NS:
        p = scale_csv_path(n)
        rows = _count_data_lines(p)
        assert rows == n, f"{p.name}: {rows} rows, expected {n}"
        files_meta[scale_label(n)] = {"path": str(p), "rows": rows, "sha256": _sha256(p)}
        print(f"  {p.name}: {rows:,} rows  sha256={files_meta[scale_label(n)]['sha256'][:16]}…")

    manifest = {
        "round": 2,
        "arm": "scaling_sweep",
        "seed": SEED,
        "generated_at_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "prefix_rule": (
            "data_10k.csv = byte-exact first 10,000 data rows of data_100k.csv "
            "(txn_ids T000000..T009999 in file order)"
        ),
        "data_100k_rule": "byte-for-byte copy of round-1 data.csv (row count verified)",
        "data_1m_rule": (
            "same generative model + per-row draw order as gen_dataset.py, fresh "
            "random.Random(seed), 1,000,000 rows, NO anomaly plant / labels; rows "
            "0..99,999 == data.csv except the 100 round-1 planted amounts (verified)"
        ),
        "files": files_meta,
        "point_ids": id_meta,
    }
    with open(MANIFEST, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)
    print(f"wrote {MANIFEST}")


if __name__ == "__main__":
    main()
