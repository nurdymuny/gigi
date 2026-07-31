"""duckdb runner — LOCKED PROTOCOL, in-process (duckdb's real shape).

Fully self-contained (needs `pip install duckdb`). The database is a FILE on
disk (default: a `gigi_vs_stores` folder under %TEMP%, outside the
OneDrive-synced repo; override with VS_STORES_WORKDIR). Default settings —
including duckdb's default multi-threading, which is its out-of-the-box shape
(recorded in versions, disclosed).

Cells:
  A INGEST      INSERT INTO txns SELECT FROM read_csv(...) — duckdb's
                idiomatic bulk path (rule 2A: "bulk insert/appender"). The
                timed window therefore INCLUDES duckdb's own CSV parse,
                whereas gigi/sqlite parse the CSV outside their timed windows
                (disclosed; it is duckdb doing strictly more work inside the
                clock, via its own fastest honest path). txn_id is PRIMARY KEY
                in the DDL, so the ART key index is maintained DURING ingest —
                same deal as gigi and sqlite.
  B POINT       2,000 lookups by txn_id (PK/ART index in place before timing,
                rule 2B), warm, sequential.
  C AGGREGATE   SELECT merchant, AVG(amount), STDDEV_POP(amount) FROM txns
                GROUP BY merchant (population stddev — same convention as the
                sqlite runner's sqrt-form).
  D ANOMALY     the honest expert-SQL flat baseline: per-(merchant, 2h-bucket)
                z-score of amount via GROUP BY, scored per row. floor() is
                used for the bucket (duckdb CAST rounds; sqlite CAST
                truncates — floor makes the buckets identical). SQL line count
                is a reported metric (shared counting rule).
"""
from __future__ import annotations

import os
import sys
import tempfile
import time
from pathlib import Path

try:
    import duckdb
except ImportError:
    sys.exit("duckdb is not installed — pip install duckdb")

from eval_common import (
    MERCHANTS, N_ROWS, N_SUBSET, TXN_CSV, average_precision, cell_aggregate,
    cell_anomaly, cell_ingest, cell_point_query, count_loc, load_labels,
    point_query_ids, read_subset, require_dataset, wall_ms, warmup_plus_reps,
    write_results,
)

WORK_DIR = Path(os.environ.get("VS_STORES_WORKDIR", tempfile.gettempdir())) / "gigi_vs_stores"
DB_PATH = WORK_DIR / "bench_duckdb.db"

DDL = """
CREATE TABLE txns (
    txn_id   VARCHAR PRIMARY KEY,
    merchant VARCHAR NOT NULL,
    customer VARCHAR NOT NULL,
    hour     DOUBLE NOT NULL,
    amount   DOUBLE NOT NULL
)
"""
POINT_SQL = "SELECT txn_id, merchant, customer, hour, amount FROM txns WHERE txn_id = ?"

AGG_SQL = """
SELECT merchant,
       AVG(amount) AS mean_amount,
       STDDEV_POP(amount) AS stddev_amount
FROM txns
GROUP BY merchant
"""

# The honest expert-SQL flat baseline (rule 2D). Its line count is a metric.
ANOM_SQL = """
WITH b AS (
  SELECT txn_id, merchant, CAST(floor(hour / 2) AS INTEGER) AS hb, amount
  FROM subset
),
stats AS (
  SELECT merchant, hb,
         AVG(amount) AS mu,
         STDDEV_POP(amount) AS sd
  FROM b
  GROUP BY merchant, hb
)
SELECT b.txn_id,
       COALESCE(ABS(b.amount - s.mu) / NULLIF(s.sd, 0.0), 0.0) AS score
FROM b
JOIN stats s ON s.merchant = b.merchant AND s.hb = b.hb
"""


def ingest_sql() -> str:
    path = str(TXN_CSV).replace("'", "''")
    return f"""
        INSERT INTO txns
        SELECT * FROM read_csv('{path}', header = true, columns = {{
            'txn_id': 'VARCHAR', 'merchant': 'VARCHAR', 'customer': 'VARCHAR',
            'hour': 'DOUBLE', 'amount': 'DOUBLE'
        }})
    """


def fresh_connection(con):
    if con is not None:
        con.close()
    for suffix in ("", ".wal"):
        Path(str(DB_PATH) + suffix).unlink(missing_ok=True)
    con = duckdb.connect(str(DB_PATH))
    con.execute(DDL)
    return con


def main() -> None:
    require_dataset()
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    con = None

    # ── A. INGEST ── fresh file DB per rep; timed = the bulk INSERT..read_csv
    def ingest_once() -> float:
        nonlocal con
        con = fresh_connection(con)
        sql = ingest_sql()
        t0 = time.perf_counter()
        con.execute(sql)
        dur = time.perf_counter() - t0
        n = con.execute("SELECT count(*) FROM txns").fetchone()[0]  # untimed verify
        assert n == N_ROWS, f"ingest verification failed: {n}"
        return N_ROWS / dur

    print("A. ingest (1 warmup + 3 reps, 100k rows each) ...")
    ingest_reps = warmup_plus_reps(ingest_once)
    print(f"   rows/sec reps: {[round(x) for x in ingest_reps]}")
    assert con is not None  # keep the last rep's DB for the remaining cells

    # ── B. POINT QUERY ── PK/ART index exists from DDL (before timing, rule 2B)
    ids = point_query_ids()
    row = con.execute(POINT_SQL, [ids[0]]).fetchone()  # correctness, untimed
    assert row is not None and row[0] == ids[0]

    def point_pass() -> list[float]:
        lats = []
        for tid in ids:
            t0 = time.perf_counter()
            r = con.execute(POINT_SQL, [tid]).fetchone()
            lats.append((time.perf_counter() - t0) * 1000.0)
            assert r is not None
        return lats

    print("B. point queries (2000 lookups; 1 warmup pass + 3 timed passes) ...")
    point_reps = warmup_plus_reps(point_pass)

    # ── C. AGGREGATE ──
    def aggregate_once():
        out = con.execute(AGG_SQL).fetchall()
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
            txn_id   VARCHAR PRIMARY KEY,
            merchant VARCHAR NOT NULL,
            customer VARCHAR NOT NULL,
            hour     DOUBLE NOT NULL,
            amount   DOUBLE NOT NULL
        )""")
    con.executemany("INSERT INTO subset VALUES (?, ?, ?, ?, ?)", subset)
    assert con.execute("SELECT count(*) FROM subset").fetchone()[0] == N_SUBSET

    labels = load_labels()
    sql_scores: dict[str, float] = {}

    def anomaly_once() -> float:
        nonlocal sql_scores
        ms, out = wall_ms(lambda: con.execute(ANOM_SQL).fetchall())
        assert len(out) == N_SUBSET
        sql_scores = {tid: float(s) for tid, s in out}
        return ms

    print("   z-score query (1 warmup + 3 reps) ...")
    anom_reps = warmup_plus_reps(anomaly_once)
    pr_auc = average_precision(labels, sql_scores)
    loc = count_loc(ANOM_SQL)
    print(f"   wall ms reps: {[round(x, 2) for x in anom_reps]}  PR-AUC: {pr_auc:.4f}  SQL lines: {loc}")

    threads = con.execute("SELECT current_setting('threads')").fetchone()[0]

    disclosures = [
        "duckdb runs in-process (its real shape); gigi is measured over HTTP — "
        "asymmetry disclosed in the shared protocol block.",
        "Database is file-backed on local disk (%TEMP%/gigi_vs_stores), outside "
        "the OneDrive-synced repo folder, default settings including duckdb's "
        f"default multi-threading (threads={threads}); gigi and sqlite cells "
        "use whatever parallelism their default shapes provide.",
        "INGEST timed window includes duckdb's own CSV parse (read_csv is its "
        "idiomatic bulk path); gigi/sqlite parse the CSV to Python rows "
        "outside their timed windows. duckdb does strictly more work inside "
        "the clock here, via its own fastest honest path.",
        "txn_id PRIMARY KEY declared in the DDL, so the ART key index is "
        "maintained during ingest — matching gigi and sqlite. No system "
        "defers its key index out of the ingest cell.",
        "AGGREGATE/ANOMALY use STDDEV_POP — same population-stddev convention "
        "as the sqlite runner's sqrt-form.",
        "ANOMALY bucket uses floor(hour/2) explicitly because duckdb CAST "
        "rounds while sqlite CAST truncates — buckets are identical across "
        "the two SQL baselines.",
        "ANOMALY baseline required hand-chosen grouping (merchant x 2h-bucket) "
        "and domain judgment that gigi's /scan did not get — that expert input "
        "is the point of the comparison and is priced in the SQL-lines metric.",
        "ANOMALY: zero-variance cohorts score 0.0 via COALESCE.",
    ]
    versions = {
        "duckdb": duckdb.__version__,
        "threads": str(threads),
        "storage": str(DB_PATH),
    }
    cells = {
        "ingest": cell_ingest(ingest_reps),
        "point_query": cell_point_query(point_reps),
        "aggregate": cell_aggregate(
            agg_reps, len(MERCHANTS),
            "SELECT merchant, AVG(amount), STDDEV_POP(amount) GROUP BY merchant — in-process",
        ),
        "anomaly": cell_anomaly(
            anom_reps, pr_auc, loc,
            "SQL: ANOM_SQL text, non-blank non-comment lines (shared counting rule)",
            "expert SQL flat baseline: per-(merchant, 2h-bucket) |z| of amount",
            N_SUBSET,
        ),
    }
    write_results("duckdb", versions, cells, disclosures)


if __name__ == "__main__":
    main()
