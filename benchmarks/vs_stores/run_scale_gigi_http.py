"""gigi HTTP scaling-sweep runner — ROUND 2 ADDENDUM clause 2.

Times point-query p50/p95 (MICROSECONDS) at N = 10k / 100k / 1M over HTTP at
127.0.0.1:3143 — gigi's deployment shape, persistent keep-alive client, same
GET /v1/bundles/<b>/get?txn_id= endpoint as round 1. Per round-2 clause 2 the
sweep times ONLY point queries; ingest is untimed setup. Warm + sequential;
1 untimed warmup pass + 3 timed passes per N, median reported.

This script starts NOTHING. It assumes an operator-started gigi-stream server
is already listening at http://127.0.0.1:3143 with its data dir on local disk
(NOT inside OneDrive — see run_all.md). If the server is not up it aborts
with instructions.

Bundle reuse rule (setup-time convenience, safe by construction): each N gets
bundle bench_scale_<label>; if the bundle already holds exactly N records it
is reused instead of re-ingested — keys are the deterministic T%06d for
0..N-1 regardless of dataset version, and the sweep never reads values, only
looks up keys (correctness probe asserts the first id round-trips). Set
VS_STORES_SCALE_FRESH=1 to force drop + full re-ingest.

Datasets + the per-N 2,000-id lists come from gen_scale.py (run it first).

Output: results_scale_gigi_http.json (shared schema from gen_scale.write_scale_results).
"""
from __future__ import annotations

import json
import os
import sys
import time
import urllib.parse

from eval_common import GIGI_BASE, INSERT_BATCH, warmup_plus_reps
from gen_scale import (
    SCALE_NS, require_scale_files, scale_csv_path, scale_label,
    scale_point_cell, scale_point_ids, stream_rows, write_scale_results,
)
from run_gigi import Gigi, fresh_bundle, record_count

FORCE_FRESH = os.environ.get("VS_STORES_SCALE_FRESH", "") == "1"


def bundle_name(n: int) -> str:
    return f"bench_scale_{scale_label(n)}"


def ensure_bundle(g: Gigi, n: int) -> bool:
    """Untimed setup: make bundle_name(n) hold exactly the N scale rows.
    Returns True if an existing bundle was reused (count already == N)."""
    name = bundle_name(n)
    if not FORCE_FRESH:
        try:
            if record_count(g, name) == n:
                print(f"  bundle {name}: exists with {n:,} records — reused "
                      f"(VS_STORES_SCALE_FRESH=1 forces re-ingest)")
                return True
        except RuntimeError:
            pass  # bundle absent — ingest below
    fresh_bundle(g, name)
    t0 = time.perf_counter()
    batch = []
    posted = 0
    for t, m, c, h, a in stream_rows(scale_csv_path(n)):
        batch.append({"txn_id": t, "merchant": m, "customer": c, "hour": h, "amount": a})
        if len(batch) == INSERT_BATCH:
            g.ok("POST", f"/v1/bundles/{name}/insert", {"records": batch})
            posted += len(batch)
            batch = []
            if posted % 100_000 == 0:
                print(f"  bundle {name}: {posted:,}/{n:,} rows "
                      f"({time.perf_counter() - t0:.1f}s)")
    if batch:
        g.ok("POST", f"/v1/bundles/{name}/insert", {"records": batch})
    cnt = record_count(g, name)
    assert cnt == n, f"ingest verification failed at N={n}: {cnt}"
    print(f"  ingest (untimed setup): {n:,} rows in {time.perf_counter() - t0:.1f}s")
    return False


def main() -> None:
    require_scale_files()
    g = Gigi()

    # server must already be up — this runner starts nothing
    try:
        health = g.ok("GET", "/v1/health")
    except Exception as e:
        sys.exit(
            f"no gigi server at {GIGI_BASE} ({e}).\n"
            "This runner starts NOTHING. Start the server yourself first "
            "(data dir on local disk, NOT OneDrive — see run_all.md), e.g.:\n"
            '  $env:PORT="3143"; cargo run --release --bin gigi_stream\n'
            "then re-run: python run_scale_gigi_http.py"
        )
    print(f"gigi server: {json.dumps(health)}")

    cells = {}
    reused = {}
    for n in SCALE_NS:
        print(f"N = {n:,} ({scale_label(n)})")
        reused[scale_label(n)] = ensure_bundle(g, n)
        name = bundle_name(n)
        ids = scale_point_ids(n)
        quoted = [urllib.parse.quote(t) for t in ids]

        # correctness probe, untimed
        doc = g.ok("GET", f"/v1/bundles/{name}/get?txn_id={quoted[0]}")
        assert doc["data"]["txn_id"] == ids[0], doc

        def point_pass() -> list[float]:
            lats = []
            for q in quoted:
                t0 = time.perf_counter()
                status, _ = g.request("GET", f"/v1/bundles/{name}/get?txn_id={q}")
                lats.append((time.perf_counter() - t0) * 1e6)  # microseconds
                assert status == 200
            return lats

        print("  point queries (2000 lookups; 1 warmup pass + 3 timed passes) ...")
        reps = warmup_plus_reps(point_pass)
        cells[scale_label(n)] = scale_point_cell(n, reps)
        c = cells[scale_label(n)]
        print(f"  p50 {c['p50_us_median']} us   p95 {c['p95_us_median']} us")

    disclosures = [
        "All lookups measured over HTTP at 127.0.0.1 (persistent keep-alive "
        "connection) — gigi's real deployment shape. Every latency includes "
        "request serialization, the HTTP round trip, response read and JSON "
        "deserialization. The embedded lane measures the same engine with the "
        "HTTP layer removed; the sqlite leg is in-process.",
        "Sweep times ONLY point queries (round-2 clause 2); ingest is untimed "
        "setup in 1,000-row batches. Round-1 results_gigi.json remains the "
        "HTTP ingest-throughput cell of record at 100k.",
        "Bundle reuse rule: a scale bundle is reused iff its record count "
        "already equals N (keys are deterministic T%06d; values never read by "
        "the sweep). VS_STORES_SCALE_FRESH=1 forces re-ingest. Reuse per N is "
        "recorded in versions.bundles_reused.",
        "Server is operator-started; the runner cannot verify the server's "
        "data-dir placement (must be local disk outside OneDrive) — "
        "operator-attested per run_all.md.",
        "Per-N latency ratio reported as measured — growth, if any, is "
        "reported, not smoothed. (REGRESSION flagging for clear growth "
        "applies to the embedded lane per the addendum; this HTTP leg "
        "additionally carries per-request transport jitter.)",
    ]
    versions = {
        "gigi_server_health": health,
        "gigi_version": health.get("version", "unknown"),
        "transport": f"HTTP/1.1 keep-alive to {GIGI_BASE}",
        "bundles_reused": reused,
    }
    write_scale_results("gigi_http", versions, cells, disclosures)


if __name__ == "__main__":
    main()
