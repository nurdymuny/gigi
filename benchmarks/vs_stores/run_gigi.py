"""gigi runner — LOCKED PROTOCOL, measured over HTTP at 127.0.0.1:3143.

This script starts NOTHING. It assumes a gigi-stream server is already
listening at http://127.0.0.1:3143 (start it yourself, see run_all.md). If the
server is not up, the script aborts with instructions.

Cells (rule 2, gigi's idiomatic paths):
  A INGEST      POST /v1/bundles/bench/insert in 1,000-row batches over HTTP
  B POINT       GET  /v1/bundles/bench/get?txn_id=... (O(1) point endpoint),
                warm, sequential, 2,000 shared ids
  C AGGREGATE   POST /v1/gql  INTEGRATE bench OVER merchant MEASURE
                avg(amount), stddev(amount)  — probed at runtime; if the
                server's GQL parser rejects stddev (as of sha at run time the
                parser's AggFunc is COUNT/SUM/AVG/MIN/MAX only, although
                GQL_REFERENCE.md claims stddev), the honest fallback is
                INTEGRATE for avg+count (server-side) + one COVER per merchant
                with client-side stddev over the returned rows. The fallback's
                full wall time — including pulling ~100k rows over HTTP — is
                the measured cell. Disclosed, not softened.
  D ANOMALY     one POST /v1/bundles/bench_anom/scan, zero-config, on the 20k
                labeled subset. Metrics: wall ms, client lines of code
                (counted from the actual function source), PR-AUC via the ONE
                shared eval.

Timing includes request serialization, the HTTP round trip, response read and
JSON deserialization — the real cost of talking to gigi in its deployment
shape. A persistent keep-alive connection is used (any real client would).
"""
from __future__ import annotations

import inspect
import json
import math
import sys
import time
import urllib.parse
from http.client import HTTPConnection

from eval_common import (
    GIGI_BASE, GIGI_HOST, GIGI_PORT, INSERT_BATCH, MERCHANTS, N_ROWS, N_SUBSET,
    SCAN_BUDGET, average_precision, cell_aggregate, cell_anomaly, cell_ingest,
    cell_point_query, count_loc, load_labels, point_query_ids, read_subset,
    read_transactions, require_dataset, wall_ms, warmup_plus_reps, write_results,
)

BUNDLE = "bench"
ANOM_BUNDLE = "bench_anom"

SCHEMA = {
    "fields": {
        "txn_id": "text",
        "merchant": "text",
        "customer": "text",
        "hour": "numeric",
        "amount": "numeric",
    },
    "keys": ["txn_id"],
}


class Gigi:
    """Minimal persistent keep-alive HTTP client (stdlib only)."""

    def __init__(self, host: str = GIGI_HOST, port: int = GIGI_PORT):
        self.host, self.port = host, port
        self.conn = HTTPConnection(host, port, timeout=600)

    def request(self, method: str, path: str, body=None):
        payload = None
        headers = {}
        if body is not None:
            payload = body if isinstance(body, (str, bytes)) else json.dumps(body)
            headers["Content-Type"] = "application/json"
        for attempt in (0, 1):  # one silent reconnect if the server closed keep-alive
            try:
                self.conn.request(method, path, payload, headers)
                resp = self.conn.getresponse()
                raw = resp.read()
                break
            except (ConnectionError, OSError):
                if attempt == 1:
                    raise
                self.conn.close()
                self.conn = HTTPConnection(self.host, self.port, timeout=600)
        try:
            doc = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            doc = {"_raw": raw.decode(errors="replace")}
        return resp.status, doc

    def ok(self, method: str, path: str, body=None):
        status, doc = self.request(method, path, body)
        if status >= 300:
            raise RuntimeError(f"{method} {path} -> {status}: {json.dumps(doc)[:400]}")
        return doc


def extract_rows(doc):
    """GQL responses carry row lists under 'data' or 'rows' depending on the
    statement family; accept either (and 'results'), defensively."""
    for key in ("data", "rows", "results"):
        v = doc.get(key)
        if isinstance(v, list):
            return v
    if isinstance(doc, list):
        return doc
    raise RuntimeError(f"no row list in GQL response: {json.dumps(doc)[:300]}")


def fresh_bundle(g: Gigi, name: str) -> None:
    g.request("DELETE", f"/v1/bundles/{name}")  # 404 fine on first run
    g.ok("POST", "/v1/bundles", {"name": name, "schema": SCHEMA})


def insert_batches(g: Gigi, name: str, records: list[dict]) -> None:
    for i in range(0, len(records), INSERT_BATCH):
        g.ok("POST", f"/v1/bundles/{name}/insert",
             {"records": records[i:i + INSERT_BATCH]})


def record_count(g: Gigi, name: str) -> int:
    return int(g.ok("GET", f"/v1/bundles/{name}/schema").get("records", -1))


# ── the minimal, honest anomaly client (its LOC is a reported metric) ────────
def gigi_scan(g):
    status, doc = g.request("POST", f"/v1/bundles/{ANOM_BUNDLE}/scan",
                            {"budget": SCAN_BUDGET, "limit": 0})
    assert status == 200, doc
    return {r["txn_id"]: r["score"] for r in doc["results"]}


def main() -> None:
    require_dataset()
    g = Gigi()

    # server must already be up — this runner starts nothing (rule: files spec)
    try:
        health = g.ok("GET", "/v1/health")
    except Exception as e:
        sys.exit(
            f"no gigi server at {GIGI_BASE} ({e}).\n"
            "This runner starts NOTHING. Start the server yourself first, e.g.:\n"
            '  $env:PORT="3143"; cargo run --release --bin gigi_stream\n'
            "then re-run: python run_gigi.py"
        )
    print(f"gigi server: {json.dumps(health)}")

    rows = read_transactions()
    records = [
        {"txn_id": t, "merchant": m, "customer": c, "hour": h, "amount": a}
        for t, m, c, h, a in rows
    ]

    # ── A. INGEST ── fresh bundle per rep; timed window = the batched POSTs
    #    (json.dumps of each batch happens inside — it is part of the wire path)
    def ingest_once() -> float:
        fresh_bundle(g, BUNDLE)
        t0 = time.perf_counter()
        insert_batches(g, BUNDLE, records)
        dur = time.perf_counter() - t0
        n = record_count(g, BUNDLE)  # verification, outside the timed window
        assert n == N_ROWS, f"ingest verification failed: {n} != {N_ROWS}"
        return N_ROWS / dur

    print("A. ingest (1 warmup + 3 reps, 100k rows each) ...")
    ingest_reps = warmup_plus_reps(ingest_once)
    print(f"   rows/sec reps: {[round(x) for x in ingest_reps]}")

    # ── B. POINT QUERY ── warm, sequential, per-lookup latency
    ids = point_query_ids()
    quoted = [urllib.parse.quote(t) for t in ids]

    # correctness check once, untimed
    doc = g.ok("GET", f"/v1/bundles/{BUNDLE}/get?txn_id={quoted[0]}")
    assert doc["data"]["txn_id"] == ids[0], doc

    def point_pass() -> list[float]:
        lats = []
        for q in quoted:
            t0 = time.perf_counter()
            status, doc = g.request("GET", f"/v1/bundles/{BUNDLE}/get?txn_id={q}")
            lats.append((time.perf_counter() - t0) * 1000.0)
            assert status == 200
        return lats

    print("B. point queries (2000 lookups; 1 warmup pass + 3 timed passes) ...")
    point_reps = warmup_plus_reps(point_pass)

    # ── C. AGGREGATE ── probe stddev support once, untimed, then lock a method
    stddev_q = f"INTEGRATE {BUNDLE} OVER merchant MEASURE avg(amount), stddev(amount);"
    status, doc = g.request("POST", "/v1/gql", {"query": stddev_q})
    server_stddev = status == 200 and "error" not in doc
    disclosures = []

    if server_stddev:
        agg_method = "GQL INTEGRATE OVER merchant MEASURE avg(amount), stddev(amount) — single server-side call"

        def aggregate_once():
            d = g.ok("POST", "/v1/gql", {"query": stddev_q})
            rows_ = extract_rows(d)
            assert len(rows_) >= len(MERCHANTS), d
            return rows_
    else:
        agg_method = (
            "fallback: GQL INTEGRATE OVER merchant MEASURE avg(amount), count(*) "
            "(server-side) + 12x GQL COVER ON merchant + client-side population "
            "stddev over the pulled rows"
        )
        disclosures.append(
            "AGGREGATE: the server rejected stddev() in INTEGRATE (GQL parser "
            "supports COUNT/SUM/AVG/MIN/MAX only, though GQL_REFERENCE.md §V "
            "lists stddev). The measured cell is the honest fallback: one "
            "INTEGRATE for avg+count plus one COVER per merchant pulling all "
            "~100k rows over HTTP with stddev computed client-side. This is "
            "the real cost of getting mean+stddev out of gigi today, and it "
            "is expected to lose to duckdb/sqlite by a wide margin."
        )
        avg_q = f"INTEGRATE {BUNDLE} OVER merchant MEASURE avg(amount), count(*);"

        def aggregate_once():
            d = g.ok("POST", "/v1/gql", {"query": avg_q})
            server_side = extract_rows(d)
            out = {}
            for name in MERCHANTS:
                d2 = g.ok("POST", "/v1/gql",
                          {"query": f"COVER {BUNDLE} ON merchant = '{name}';"})
                amounts = [float(r["amount"]) for r in extract_rows(d2)]
                n = len(amounts)
                assert n > 0, f"COVER returned no rows for merchant {name}"
                mean = sum(amounts) / n
                var = sum((a - mean) ** 2 for a in amounts) / n
                out[name] = {"mean": mean, "stddev": math.sqrt(var), "count": n}
            assert sum(v["count"] for v in out.values()) == N_ROWS
            return {"server_side": server_side, "per_merchant": out}

    print(f"C. aggregate — method: {agg_method[:80]}...")
    agg_reps = warmup_plus_reps(lambda: wall_ms(aggregate_once)[0])
    print(f"   wall ms reps: {[round(x, 1) for x in agg_reps]}")

    # ── D. ANOMALY ── 20k labeled subset in its own bundle; one zero-config
    #    /scan call is the timed cell. Labels are NEVER inserted into gigi.
    print("D. anomaly — loading 20k labeled subset into bundle (untimed) ...")
    subset = read_subset()
    fresh_bundle(g, ANOM_BUNDLE)
    insert_batches(g, ANOM_BUNDLE, [
        {"txn_id": t, "merchant": m, "customer": c, "hour": h, "amount": a}
        for t, m, c, h, a in subset
    ])
    assert record_count(g, ANOM_BUNDLE) == N_SUBSET

    labels = load_labels()
    scan_scores = {}

    def anomaly_once() -> float:
        nonlocal scan_scores
        ms, scores = wall_ms(lambda: gigi_scan(g))
        assert len(scores) == N_SUBSET, f"scan returned {len(scores)} scores"
        scan_scores = scores
        return ms

    print("   /scan (1 warmup + 3 reps) ...")
    anom_reps = warmup_plus_reps(anomaly_once)
    pr_auc = average_precision(labels, scan_scores)
    loc = count_loc(inspect.getsource(gigi_scan))
    print(f"   wall ms reps: {[round(x, 1) for x in anom_reps]}  PR-AUC: {pr_auc:.4f}  LOC: {loc}")

    # ── report ──
    disclosures = [
        "All gigi cells are measured over HTTP at 127.0.0.1 (persistent "
        "keep-alive connection) — gigi's real deployment shape. sqlite/duckdb "
        "run in-process. Point-lookup and per-call numbers therefore include "
        "an HTTP round trip that the embedded stores do not pay; expected "
        "honest loss on point queries.",
        "Ingest timed window includes client-side json.dumps of each 1,000-row "
        "batch (part of the wire path). CSV parsing to Python rows happens "
        "before the timed window, identical to the sqlite runner.",
        "Point-query timing includes response read + JSON deserialization "
        "(the counterpart of the embedded stores' row materialization).",
        "ANOMALY: /scan is zero-config — the only inputs are the bundle name "
        "and the review budget. The labels file is never sent to gigi. The "
        "timed window includes downloading and parsing the full 20k-row "
        "scored response.",
        "ANOMALY LOC counts the actual python client function (source-counted "
        "at run time, shared counting rule in eval_common.count_loc).",
    ] + disclosures

    versions = {
        "gigi_server_health": health,
        "gigi_version": health.get("version", "unknown"),
        "transport": f"HTTP/1.1 keep-alive to {GIGI_BASE}",
    }
    cells = {
        "ingest": cell_ingest(ingest_reps),
        "point_query": cell_point_query(point_reps),
        "aggregate": cell_aggregate(agg_reps, len(MERCHANTS), agg_method),
        "anomaly": cell_anomaly(
            anom_reps, pr_auc, loc,
            "python: gigi_scan() function source, non-blank non-comment lines",
            "POST /v1/bundles/bench_anom/scan {budget: 0.05, limit: 0} — zero-config",
            N_SUBSET,
        ),
    }
    write_results("gigi", versions, cells, disclosures)


if __name__ == "__main__":
    main()
