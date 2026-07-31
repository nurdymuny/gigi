"""sqlite runner — LOCKED PROTOCOL, in-process (sqlite's real shape).

Fully self-contained: python stdlib only. The database is a FILE on disk
(default: a `gigi_vs_stores` folder under %TEMP%, deliberately outside the
OneDrive-synced repo so sync I/O cannot contaminate timings; override with
VS_STORES_WORKDIR). Default pragmas — the ingest is one executemany in one
explicit transaction (rule 2A: sqlite's idiomatic bulk path), so there is a
single commit fsync regardless of journal mode.

Cells:
  A INGEST      executemany of 100k rows in ONE transaction. txn_id is
                declared PRIMARY KEY in the DDL, so the key index is
                maintained DURING ingest — same deal as gigi, whose base key
                is indexed at insert (fairness: no system gets to defer its
                key index out of the ingest cell).
  B POINT       2,000 lookups by txn_id (PK index in place before timing —
                best foot forward per rule 2B), warm, sequential.
  C AGGREGATE   SELECT merchant, AVG(amount), population stddev FROM txns
                GROUP BY merchant. sqlite has no STDDEV aggregate; population
                stddev is computed as sqrt(max(E[x^2]-E[x]^2, 0)) in SQL —
                same population convention as duckdb's STDDEV_POP (disclosed).
  D ANOMALY     the honest expert-SQL flat baseline: per-(merchant, 2h-bucket)
                z-score of amount via GROUP BY, scored per row. Cohorts with
                zero variance score 0 (COALESCE). SQL line count is a
                reported metric (shared counting rule).
"""
from __future__ import annotations

import os
import sqlite3
import sys
import tempfile
import time
from pathlib import Path

from eval_common import (
    MERCHANTS, N_ROWS, N_SUBSET, average_precision, cell_aggregate,
    cell_anomaly, cell_ingest, cell_point_query, count_loc, load_labels,
    point_query_ids, read_subset, read_transactions, require_dataset, wall_ms,
    warmup_plus_reps, write_results,
)

WORK_DIR = Path(os.environ.get("VS_STORES_WORKDIR", tempfile.gettempdir())) / "gigi_vs_stores"
DB_PATH = WORK_DIR / "bench_sqlite.db"

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

# population stddev: sqrt(max(E[x^2] - E[x]^2, 0)) — max() guards float dips below 0
AGG_SQL = """
SELECT merchant,
       AVG(amount) AS mean_amount,
       sqrt(max(AVG(amount * amount) - AVG(amount) * AVG(amount), 0.0)) AS stddev_amount
FROM txns
GROUP BY merchant
"""

# The honest expert-SQL flat baseline (rule 2D). Its line count is a metric.
ANOM_SQL = """
WITH b AS (
  SELECT txn_id, merchant, CAST(hour / 2 AS INTEGER) AS hb, amount
  FROM subset
),
stats AS (
  SELECT merchant, hb,
         AVG(amount) AS mu,
         sqrt(max(AVG(amount * amount) - AVG(amount) * AVG(amount), 0.0)) AS sd
  FROM b
  GROUP BY merchant, hb
)
SELECT b.txn_id,
       COALESCE(ABS(b.amount - s.mu) / NULLIF(s.sd, 0.0), 0.0) AS score
FROM b
JOIN stats s ON s.merchant = b.merchant AND s.hb = b.hb
"""


def fresh_connection(con: sqlite3.Connection | None) -> sqlite3.Connection:
    if con is not None:
        con.close()
    for suffix in ("", "-journal", "-wal", "-shm"):
        Path(str(DB_PATH) + suffix).unlink(missing_ok=True)
    con = sqlite3.connect(DB_PATH, isolation_level=None)  # autocommit; explicit BEGIN/COMMIT
    con.execute(DDL)
    return con


def main() -> None:
    require_dataset()
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    try:
        sqlite3.connect(":memory:").execute("SELECT sqrt(4)").fetchone()
    except sqlite3.OperationalError:
        sys.exit("this sqlite build lacks math functions (sqrt) — need CPython >= 3.11")

    rows = read_transactions()  # parsed OUTSIDE the timed window (same as gigi)
    con: sqlite3.Connection | None = None

    # ── A. INGEST ── fresh file DB per rep; timed = BEGIN + executemany + COMMIT
    def ingest_once() -> float:
        nonlocal con
        con = fresh_connection(con)
        t0 = time.perf_counter()
        con.execute("BEGIN")
        con.executemany(INSERT_SQL, rows)
        con.execute("COMMIT")
        dur = time.perf_counter() - t0
        n = con.execute("SELECT count(*) FROM txns").fetchone()[0]  # untimed verify
        assert n == N_ROWS, f"ingest verification failed: {n}"
        return N_ROWS / dur

    print("A. ingest (1 warmup + 3 reps, 100k rows each) ...")
    ingest_reps = warmup_plus_reps(ingest_once)
    print(f"   rows/sec reps: {[round(x) for x in ingest_reps]}")
    assert con is not None  # keep the last rep's DB for the remaining cells

    # ── B. POINT QUERY ── PK index exists from DDL (before timing, rule 2B)
    ids = point_query_ids()
    cur = con.cursor()
    row = cur.execute(POINT_SQL, (ids[0],)).fetchone()  # correctness, untimed
    assert row is not None and row[0] == ids[0]

    def point_pass() -> list[float]:
        lats = []
        for tid in ids:
            t0 = time.perf_counter()
            r = cur.execute(POINT_SQL, (tid,)).fetchone()
            lats.append((time.perf_counter() - t0) * 1000.0)
            assert r is not None
        return lats

    print("B. point queries (2000 lookups; 1 warmup pass + 3 timed passes) ...")
    point_reps = warmup_plus_reps(point_pass)

    # ── C. AGGREGATE ──
    def aggregate_once():
        out = cur.execute(AGG_SQL).fetchall()
        assert len(out) == len(MERCHANTS)
        return out

    print("C. aggregate (mean+stddev by merchant, full 100k) ...")
    agg_reps = warmup_plus_reps(lambda: wall_ms(aggregate_once)[0])
    print(f"   wall ms reps: {[round(x, 2) for x in agg_reps]}")

    # ── D. ANOMALY ── subset table built untimed; timed = the z-score query
    print("D. anomaly — building 20k subset table (untimed) ...")
    subset = read_subset()
    con.execute("DROP TABLE IF EXISTS subset")
    con.execute("""
        CREATE TABLE subset (
            txn_id   TEXT PRIMARY KEY,
            merchant TEXT NOT NULL,
            customer TEXT NOT NULL,
            hour     REAL NOT NULL,
            amount   REAL NOT NULL
        )""")
    con.execute("BEGIN")
    con.executemany("INSERT INTO subset VALUES (?, ?, ?, ?, ?)", subset)
    con.execute("COMMIT")
    # rule 2D allows indexes for the baseline — cohort index in place before timing
    con.execute("CREATE INDEX idx_subset_cohort ON subset (merchant)")
    con.execute("ANALYZE")
    assert con.execute("SELECT count(*) FROM subset").fetchone()[0] == N_SUBSET

    labels = load_labels()
    sql_scores: dict[str, float] = {}

    def anomaly_once() -> float:
        nonlocal sql_scores
        ms, out = wall_ms(lambda: cur.execute(ANOM_SQL).fetchall())
        assert len(out) == N_SUBSET
        sql_scores = {tid: float(s) for tid, s in out}
        return ms

    print("   z-score query (1 warmup + 3 reps) ...")
    anom_reps = warmup_plus_reps(anomaly_once)
    pr_auc = average_precision(labels, sql_scores)
    loc = count_loc(ANOM_SQL)
    print(f"   wall ms reps: {[round(x, 2) for x in anom_reps]}  PR-AUC: {pr_auc:.4f}  SQL lines: {loc}")

    disclosures = [
        "sqlite runs in-process (its real shape); gigi is measured over HTTP — "
        "asymmetry disclosed in the shared protocol block.",
        "Database is file-backed on local disk (%TEMP%/gigi_vs_stores), outside "
        "the OneDrive-synced repo folder, default pragmas (journal_mode=delete, "
        "synchronous=full). Ingest is one transaction, so one commit fsync.",
        "txn_id PRIMARY KEY is declared in the DDL, so the key index is "
        "maintained during ingest — matching gigi, whose base key is indexed "
        "at insert. No system defers its key index out of the ingest cell.",
        "sqlite has no STDDEV aggregate; population stddev computed in SQL as "
        "sqrt(max(E[x^2]-E[x]^2, 0)). Same population convention as duckdb's "
        "STDDEV_POP. The E[x^2]-E[x]^2 form can lose precision vs Welford; at "
        "these magnitudes (amounts < 2000, 64-bit floats) the error is far "
        "below the reported precision.",
        "ANOMALY baseline required hand-chosen grouping (merchant x 2h-bucket) "
        "and domain judgment that gigi's /scan did not get — that expert input "
        "is the point of the comparison and is priced in the SQL-lines metric.",
        "ANOMALY: zero-variance cohorts score 0.0 via COALESCE (a row alone in "
        "its cohort cannot be flagged by a z-score baseline).",
    ]
    versions = {
        "sqlite": sqlite3.sqlite_version,
        "storage": str(DB_PATH),
    }
    cells = {
        "ingest": cell_ingest(ingest_reps),
        "point_query": cell_point_query(point_reps),
        "aggregate": cell_aggregate(
            agg_reps, len(MERCHANTS),
            "SELECT merchant, AVG(amount), sqrt-form population stddev GROUP BY merchant — in-process",
        ),
        "anomaly": cell_anomaly(
            anom_reps, pr_auc, loc,
            "SQL: ANOM_SQL text, non-blank non-comment lines (shared counting rule)",
            "expert SQL flat baseline: per-(merchant, 2h-bucket) |z| of amount",
            N_SUBSET,
        ),
    }
    write_results("sqlite", versions, cells, disclosures)


if __name__ == "__main__":
    main()
