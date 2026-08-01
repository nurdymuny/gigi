"""Round 4 — CONTRACT-MATCHED INGEST, sqlite + duckdb lanes (LOCKED ADDENDUM).

Rounds 1-3 compared gigi syncing per 1,000-row batch (100 durability acks per
100k rows) against sqlite committing once (1 ack) — asymmetric durability
contracts inside the same cell. Round 4 runs BOTH matched pairs on the same
100k dataset. This runner times the four store lanes; the two gigi lanes come
from the release-built example binary:

    vs_stores_embedded data.csv 2000 3 --batch-rows 100000   # ONE-ACK lane
    vs_stores_embedded data.csv 2000 3                       # MANY-ACK lane
                                                             # (1,000-row default,
                                                             # the round-3b cell)

Lanes here (all in-process, fresh file DB per rep, 1 untimed warmup + 3 timed
reps, median — the round-1 discipline via eval_common.warmup_plus_reps):

  ONE-ACK   sqlite: one BEGIN + executemany(100k) + COMMIT  (round-1 cell, re-timed)
            duckdb: one INSERT INTO .. SELECT FROM read_csv (round-1 shape, re-timed)
  MANY-ACK  sqlite: BEGIN + executemany(1,000) + COMMIT per chunk — 100
            transactions, journal defaults (journal_mode=delete,
            synchronous=full) disclosed
            duckdb: BEGIN + executemany(1,000) + COMMIT per chunk — 100
            transactions

Same records, same order, for every lane (rows parsed from data.csv outside
every timed window, except duckdb ONE-ACK whose read_csv parse is inside the
clock — its round-1 shape, disclosed then, disclosed again here).

Timing purity: strictly serial; refuses to run while anything answers on
127.0.0.1:3143 (round-2 clause 4). No engine/server involvement — every lane
is in-process.

Output: results_round4_ingest_contracts.json (ingest-only schema — this file
deliberately does NOT use eval_common.write_results, which asserts the full
four-cell round-1 schema).
"""
from __future__ import annotations

import json
import socket
import sqlite3
import sys
import time
from pathlib import Path

from eval_common import (
    GIGI_HOST, GIGI_PORT, HERE, INSERT_BATCH, N_ROWS, env_info, median,
    read_transactions, require_dataset, warmup_plus_reps,
)

# Reuse the round-1 runners' DDL/SQL/db-file plumbing verbatim — the point of
# this round is contract matching, so the per-lane mechanics must be the
# round-1 code paths, not re-implementations.
import run_sqlite  # DDL, INSERT_SQL, fresh_connection, DB_PATH, WORK_DIR

OUT_PATH = HERE / "results_round4_ingest_contracts.json"
CHUNK = INSERT_BATCH  # 1,000 — the locked many-ack chunk size


def refuse_if_gigi_up() -> None:
    """Round-2 clause 4: in-process legs run only with the server verified
    dead. Enforced here, not just documented."""
    try:
        s = socket.create_connection((GIGI_HOST, GIGI_PORT), timeout=0.5)
    except OSError:
        return  # nothing listening — good
    s.close()
    sys.exit(
        f"something answers on {GIGI_HOST}:{GIGI_PORT} — stop the gigi server "
        "before the in-process lanes (timing purity, round-2 clause 4)"
    )


def chunks(rows: list, size: int):
    for i in range(0, len(rows), size):
        yield rows[i : i + size]


def cell(reps: list[float], acks: int, mechanism: str) -> dict:
    return {
        "rows": N_ROWS,
        "unit": "rows_per_sec",
        "reps": [round(x, 1) for x in reps],
        "median": round(median(reps), 1),
        "durability_acks": acks,
        "mechanism": mechanism,
    }


# ── sqlite lanes ─────────────────────────────────────────────────────────────
def sqlite_lanes(rows) -> tuple[dict, dict]:
    con: sqlite3.Connection | None = None

    def one_ack() -> float:
        nonlocal con
        con = run_sqlite.fresh_connection(con)
        t0 = time.perf_counter()
        con.execute("BEGIN")
        con.executemany(run_sqlite.INSERT_SQL, rows)
        con.execute("COMMIT")
        dur = time.perf_counter() - t0
        n = con.execute("SELECT count(*) FROM txns").fetchone()[0]  # untimed verify
        assert n == N_ROWS, f"sqlite one-ack verification failed: {n}"
        return N_ROWS / dur

    def many_ack() -> float:
        nonlocal con
        con = run_sqlite.fresh_connection(con)
        t0 = time.perf_counter()
        for chunk in chunks(rows, CHUNK):
            con.execute("BEGIN")
            con.executemany(run_sqlite.INSERT_SQL, chunk)
            con.execute("COMMIT")
        dur = time.perf_counter() - t0
        n = con.execute("SELECT count(*) FROM txns").fetchone()[0]  # untimed verify
        assert n == N_ROWS, f"sqlite many-ack verification failed: {n}"
        return N_ROWS / dur

    print(f"sqlite ONE-ACK  (1 warmup + 3 reps, {N_ROWS} rows, 1 txn) ...")
    one = warmup_plus_reps(one_ack)
    print(f"   rows/sec reps: {[round(x) for x in one]}")
    print(f"sqlite MANY-ACK (1 warmup + 3 reps, {N_ROWS} rows, {N_ROWS // CHUNK} txns) ...")
    many = warmup_plus_reps(many_ack)
    print(f"   rows/sec reps: {[round(x) for x in many]}")
    if con is not None:
        con.close()
    return (
        cell(one, 1, "one BEGIN + executemany + COMMIT (round-1 cell, re-timed)"),
        cell(
            many, N_ROWS // CHUNK,
            f"BEGIN + executemany({CHUNK}) + COMMIT per chunk — "
            f"{N_ROWS // CHUNK} transactions, journal defaults",
        ),
    )


# ── duckdb lanes ─────────────────────────────────────────────────────────────
def duckdb_lanes(rows) -> tuple[dict, dict, dict]:
    try:
        import duckdb
    except ImportError:
        sys.exit("duckdb is not installed — pip install duckdb")
    import run_duckdb  # DDL, ingest_sql, fresh_connection (needs duckdb importable)

    con = None
    insert_sql = "INSERT INTO txns VALUES (?, ?, ?, ?, ?)"

    def one_ack() -> float:
        nonlocal con
        con = run_duckdb.fresh_connection(con)
        sql = run_duckdb.ingest_sql()
        t0 = time.perf_counter()
        con.execute(sql)
        dur = time.perf_counter() - t0
        n = con.execute("SELECT count(*) FROM txns").fetchone()[0]  # untimed verify
        assert n == N_ROWS, f"duckdb one-ack verification failed: {n}"
        return N_ROWS / dur

    def many_ack() -> float:
        nonlocal con
        con = run_duckdb.fresh_connection(con)
        t0 = time.perf_counter()
        for chunk in chunks(rows, CHUNK):
            con.execute("BEGIN TRANSACTION")
            con.executemany(insert_sql, chunk)
            con.execute("COMMIT")
        dur = time.perf_counter() - t0
        n = con.execute("SELECT count(*) FROM txns").fetchone()[0]  # untimed verify
        assert n == N_ROWS, f"duckdb many-ack verification failed: {n}"
        return N_ROWS / dur

    print(f"duckdb ONE-ACK  (1 warmup + 3 reps, {N_ROWS} rows, 1 bulk insert) ...")
    one = warmup_plus_reps(one_ack)
    print(f"   rows/sec reps: {[round(x) for x in one]}")
    print(f"duckdb MANY-ACK (1 warmup + 3 reps, {N_ROWS} rows, {N_ROWS // CHUNK} txns) ...")
    many = warmup_plus_reps(many_ack)
    print(f"   rows/sec reps: {[round(x) for x in many]}")
    versions = {"duckdb": duckdb.__version__, "storage": str(run_duckdb.DB_PATH)}
    if con is not None:
        con.close()
    return (
        cell(one, 1, "INSERT INTO txns SELECT FROM read_csv(...) — round-1 shape, "
                     "CSV parse inside the clock (disclosed)"),
        cell(
            many, N_ROWS // CHUNK,
            f"BEGIN + executemany({CHUNK}) + COMMIT per chunk — "
            f"{N_ROWS // CHUNK} transactions, row-wise prepared statements",
        ),
        versions,
    )


def main() -> None:
    require_dataset()
    refuse_if_gigi_up()
    run_sqlite.WORK_DIR.mkdir(parents=True, exist_ok=True)

    rows = read_transactions()  # parsed OUTSIDE every timed window

    sq_one, sq_many = sqlite_lanes(rows)
    dk_one, dk_many, dk_versions = duckdb_lanes(rows)

    disclosures = [
        "CONTRACT MATCHING (round-4 clause 1): rounds 1-3 timed gigi under a "
        "100-durability-ack contract against sqlite under a 1-ack contract. "
        "This file carries both matched pairs for the stores; the two gigi "
        "lanes come from the vs_stores_embedded binary (--batch-rows 100000 "
        "for one-ack; default 1000 for many-ack) and are reported side by "
        "side with these, never mixed across contracts.",
        "MANY-ACK 'ack' definitions per system, honestly different in kind: "
        "sqlite COMMIT under journal_mode=delete + synchronous=full "
        "(fsync-backed rollback-journal commit per chunk); duckdb COMMIT of "
        "its WAL per chunk (durability semantics per duckdb defaults, version "
        "recorded); gigi batch_insert = WAL append + one fsync per call. Each "
        "system pays its own real per-ack cost; none is normalized to "
        "another's.",
        "duckdb MANY-ACK uses row-wise prepared-statement executemany per "
        "chunk — NOT duckdb's idiomatic columnar bulk path. That is the "
        "honest shape of a 100-transaction contract from pre-parsed Python "
        "rows; the ONE-ACK lane shows duckdb's idiomatic path (read_csv, CSV "
        "parse inside the clock, as in round 1).",
        "sqlite lanes reuse run_sqlite.py's DDL/INSERT/fresh_connection "
        "verbatim (txn_id PRIMARY KEY maintained during ingest, fresh file DB "
        "per rep, %TEMP% local disk outside the OneDrive-synced tree); duckdb "
        "lanes reuse run_duckdb.py's likewise.",
        "All lanes in-process, strictly serial, GC handled uniformly by "
        "eval_common.warmup_plus_reps; this runner aborts if anything answers "
        "on 127.0.0.1:3143.",
    ]

    out = {
        "round": 4,
        "purpose": "contract-matched ingest — store lanes (gigi lanes: vs_stores_embedded --batch-rows)",
        "protocol": {
            "n_rows": N_ROWS,
            "chunk_rows_many_ack": CHUNK,
            "warmups_per_cell": 1,
            "reps_per_cell": 3,
            "cell_statistic": "median of reps",
            "rows_source": "data.csv parsed pre-window (except duckdb one-ack: read_csv in-clock)",
        },
        "environment": env_info(),
        "versions": {
            "sqlite": sqlite3.sqlite_version,
            "sqlite_storage": str(run_sqlite.DB_PATH),
            **dk_versions,
        },
        "lanes": {
            "one_ack": {"sqlite": sq_one, "duckdb": dk_one},
            "many_ack": {"sqlite": sq_many, "duckdb": dk_many},
        },
        "disclosures": disclosures,
    }
    with open(OUT_PATH, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2)
    print(f"\nwrote {OUT_PATH}")


if __name__ == "__main__":
    main()
