"""sqlite scaling-sweep runner — ROUND 2 ADDENDUM clause 2, in-process.

Times point-query p50/p95 (MICROSECONDS) at N = 10k / 100k / 1M against a
file-backed sqlite database with txn_id PRIMARY KEY (index in place before
timing, from the DDL). Per round-2 clause 2 the sweep times ONLY point
queries — ingest is untimed setup. Warm + sequential; 1 untimed warmup pass +
3 timed passes per N, median reported (round-1 rule, still binding).

Timing purity (clause 4): this in-process leg refuses to run while anything
answers on 127.0.0.1:3143 — the gigi server must be verified dead first.

Datasets + the per-N 2,000-id lists come from gen_scale.py (run it first);
ids(N) = random.Random(seed+1).sample(range(N), 2000), shared with the gigi
HTTP and embedded legs. Database files live in the scale dir under %TEMP%
(never repo/OneDrive).

Output: results_scale_sqlite.json (shared schema from gen_scale.write_scale_results).
"""
from __future__ import annotations

import socket
import sqlite3
import sys
import time
from pathlib import Path

from eval_common import GIGI_HOST, GIGI_PORT, warmup_plus_reps
from gen_scale import (
    SCALE_DIR, SCALE_NS, require_scale_files, scale_csv_path, scale_label,
    scale_point_cell, scale_point_ids, stream_rows, write_scale_results,
)

DDL = """
CREATE TABLE txns (
    txn_id   TEXT PRIMARY KEY,
    merchant TEXT NOT NULL,
    customer TEXT NOT NULL,
    hour     REAL NOT NULL,
    amount   REAL NOT NULL
)
"""
INSERT_SQL = "INSERT INTO txns (txn_id, merchant, customer, hour, amount) VALUES (?, ?, ?, ?, ?)"
POINT_SQL = "SELECT txn_id, merchant, customer, hour, amount FROM txns WHERE txn_id = ?"
INGEST_CHUNK = 20_000  # untimed setup — chunked so 1M rows never sit in memory


def assert_gigi_server_dead() -> None:
    """Round-2 clause 4: server up only for HTTP legs, verified dead before
    in-process legs. Enforced here, not just documented."""
    try:
        s = socket.create_connection((GIGI_HOST, GIGI_PORT), timeout=0.5)
    except OSError:
        return  # nothing listening — good
    s.close()
    sys.exit(
        f"timing purity violation (round-2 clause 4): something is listening "
        f"on {GIGI_HOST}:{GIGI_PORT}. Stop the gigi server and verify it is "
        f"dead before running in-process legs."
    )


def db_path(n: int) -> Path:
    return SCALE_DIR / f"bench_scale_sqlite_{scale_label(n)}.db"


def build_db(n: int) -> sqlite3.Connection:
    """Fresh file DB per N; chunked streamed ingest, UNTIMED (sweep rule).
    PK index is maintained during ingest and thus in place before timing."""
    path = db_path(n)
    for suffix in ("", "-journal", "-wal", "-shm"):
        Path(str(path) + suffix).unlink(missing_ok=True)
    con = sqlite3.connect(path, isolation_level=None)
    con.execute(DDL)
    t0 = time.perf_counter()
    con.execute("BEGIN")
    batch = []
    for row in stream_rows(scale_csv_path(n)):
        batch.append(row)
        if len(batch) == INGEST_CHUNK:
            con.executemany(INSERT_SQL, batch)
            batch.clear()
    if batch:
        con.executemany(INSERT_SQL, batch)
    con.execute("COMMIT")
    cnt = con.execute("SELECT count(*) FROM txns").fetchone()[0]
    assert cnt == n, f"ingest verification failed at N={n}: {cnt}"
    print(f"  ingest (untimed setup): {n:,} rows in {time.perf_counter() - t0:.1f}s")
    return con


def main() -> None:
    assert_gigi_server_dead()
    require_scale_files()
    SCALE_DIR.mkdir(parents=True, exist_ok=True)

    cells = {}
    for n in SCALE_NS:
        print(f"N = {n:,} ({scale_label(n)})")
        con = build_db(n)
        cur = con.cursor()
        ids = scale_point_ids(n)

        # correctness probe, untimed
        row = cur.execute(POINT_SQL, (ids[0],)).fetchone()
        assert row is not None and row[0] == ids[0]

        def point_pass() -> list[float]:
            lats = []
            for tid in ids:
                t0 = time.perf_counter()
                r = cur.execute(POINT_SQL, (tid,)).fetchone()
                lats.append((time.perf_counter() - t0) * 1e6)  # microseconds
                assert r is not None
            return lats

        print("  point queries (2000 lookups; 1 warmup pass + 3 timed passes) ...")
        reps = warmup_plus_reps(point_pass)
        cells[scale_label(n)] = scale_point_cell(n, reps)
        c = cells[scale_label(n)]
        print(f"  p50 {c['p50_us_median']} us   p95 {c['p95_us_median']} us")
        con.close()

    disclosures = [
        "sqlite runs in-process (its real shape); the gigi HTTP leg pays an "
        "HTTP round trip per lookup and the gigi embedded leg is in-process "
        "Rust — shape differences disclosed in each leg's own results file.",
        "Sweep times ONLY point queries (round-2 clause 2); ingest is untimed "
        "setup — chunked executemany in one transaction, fresh file DB per N. "
        "Round-1 results_sqlite.json remains the ingest-throughput cell of "
        "record at 100k.",
        "txn_id PRIMARY KEY declared in the DDL, so the PK index exists before "
        "any timed lookup (same best-foot-forward rule as round 1).",
        "Database files live under the scale dir in %TEMP% (local disk, "
        "outside the OneDrive-synced repo).",
        "In-process purity enforced in code: this runner aborts if anything "
        "answers on 127.0.0.1:3143 (round-2 clause 4).",
        "Per-N latency ratio reported as measured — flat ~1.0x supports O(1) "
        "for sqlite's B-tree only in the observed range; B-tree lookups are "
        "O(log N) in principle and any visible growth is reported, not smoothed.",
    ]
    versions = {
        "sqlite": sqlite3.sqlite_version,
        "storage": {scale_label(n): str(db_path(n)) for n in SCALE_NS},
    }
    write_scale_results("sqlite", versions, cells, disclosures)


if __name__ == "__main__":
    main()
