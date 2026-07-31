"""gigi EXPERT-PARITY anomaly runner — ROUND 2 addendum, arm 3.

Round 1 pitted expert SQL (hand-chosen merchant x 2h-bucket z-score) against
novice gigi (zero-config /scan). This arm grants gigi the SAME domain
knowledge the SQL author had — and NOTHING more:

    GRANTED:  cohort structure is (merchant x 2h-hour-bucket);
              amount is the value channel.
    WITHHELD: labels (never sent to gigi, ever); donor mechanics; anything
              about HOW the anomalies were planted.

Both top candidate plays are implemented, measured, and reported — no
cherry-picking; the better PR-AUC is labeled best, both go in the report.

  PLAY 1  WEIGHTED FIELD-SCOPED /scan
          The expert bundle carries a derived categorical `cohort` field
          ("merchant|hourbucket", the granted cohort structure, computed
          client-side at ingest). One POST /scan scopes the geometry to
          (cohort, amount) by excluding every other field, and puts all
          fusion weight on the `context:cohort` lens — gigi's cohort-local
          curvature z, i.e. the granted knowledge expressed in gigi's own
          idiom. Weights are chosen a priori from the granted knowledge,
          not tuned on labels.

  PLAY 2  GQL COHORT Z (INTEGRATE server-side moments + client z)
          One GQL INTEGRATE ... OVER cohort MEASURE avg(amount),
          avg(amount_sq), count(*) pulls per-cohort moments (population
          stddev via E[x^2]-E[x]^2 — the SAME convention the SQL arm used),
          then the per-row |z| is computed client-side. Its LOC is counted
          by the shared rule, like the SQL text was.

The round-1 zero-config cell stands unchanged in the report as the
zero-config row (carried verbatim from results_gigi.json).

Modes:
    python run_gigi_expert.py               timed run (server on :3143, rule 4)
    python run_gigi_expert.py --smoke       untimed end-to-end smoke on a tiny
                                            throwaway bundle (server on :3142;
                                            falls back to offline shape checks)
    python run_gigi_expert.py --offline     offline request-shape validation only

This runner starts NOTHING. Timed mode assumes a gigi-stream server already
listening at 127.0.0.1:3143 and verified alone (rule 4: builds done, nothing
concurrent).
"""
from __future__ import annotations

import inspect
import json
import math
import sys

from eval_common import (
    GIGI_BASE, HERE, HOUR_BUCKET_HOURS, N_SUBSET, SCAN_BUDGET,
    average_precision, cell_anomaly, count_loc, env_info, load_labels,
    protocol_info, read_subset, require_dataset, wall_ms, warmup_plus_reps,
    write_results,  # noqa: F401  (imported to prove the shared module loads whole)
)
from run_gigi import Gigi, extract_rows

EXPERT_BUNDLE = "bench_anom_expert"
INSERT_BATCH = 1_000

# Same 2h bucketing as the SQL arm's CAST(hour / 2 AS INTEGER).
assert HOUR_BUCKET_HOURS == 2.0

EXPERT_SCHEMA = {
    "fields": {
        "txn_id": "text",
        "merchant": "text",
        "customer": "text",
        "cohort": "text",       # derived: the granted cohort structure
        "hour": "numeric",
        "amount": "numeric",
        "amount_sq": "numeric",  # derived: lets INTEGRATE reach population stddev
    },
    "keys": ["txn_id"],
}

RESULTS_PATH = HERE / "results_gigi_expert.json"
ROUND1_GIGI = HERE / "results_gigi.json"


# ── the granted-knowledge derivation (LOC counted into BOTH plays, the way the
#    SQL arm's CAST(hour/2 AS INTEGER) was counted inside ANOM_SQL) ───────────
def expert_record(t: str, m: str, c: str, h: float, a: float) -> dict:
    return {
        "txn_id": t, "merchant": m, "customer": c, "hour": h, "amount": a,
        "cohort": f"{m}|{int(h // HOUR_BUCKET_HOURS)}",
        "amount_sq": a * a,
    }


# ── PLAY 1: weighted field-scoped /scan (its LOC is a reported metric) ───────
def play_scan_cohort_weighted(g: Gigi, bundle: str) -> dict[str, float]:
    status, doc = g.request("POST", f"/v1/bundles/{bundle}/scan", {
        "budget": SCAN_BUDGET, "limit": 0,
        "exclude": ["merchant", "customer", "hour", "amount_sq"],
        "weights": {"context:cohort": 1.0},
    })
    assert status == 200, doc
    return {r["txn_id"]: r["score"] for r in doc["results"]}


# ── PLAY 2: GQL cohort z — INTEGRATE moments server-side, |z| client-side
#    (population stddev = sqrt(max(E[x^2]-E[x]^2, 0)), the SQL arm's exact
#    convention; zero-variance cohorts score 0, also matching the SQL arm) ────
def play_gql_cohort_z(g: Gigi, bundle: str, rows) -> dict[str, float]:
    q = (f"INTEGRATE {bundle} OVER cohort "
         f"MEASURE avg(amount) AS mu, avg(amount_sq) AS mu_sq, count(*) AS n;")
    status, doc = g.request("POST", "/v1/gql", {"query": q})
    assert status == 200, doc
    stats = {}
    for r in extract_rows(doc):
        mu = float(r["mu"])
        sd = math.sqrt(max(float(r["mu_sq"]) - mu * mu, 0.0))
        stats[r["cohort"]] = (mu, sd)
    out = {}
    for t, m, _c, h, a in rows:
        mu, sd = stats[f"{m}|{int(h // HOUR_BUCKET_HOURS)}"]
        out[t] = abs(a - mu) / sd if sd > 0.0 else 0.0
    return out


def play_loc(play_fn) -> int:
    """Shared counting rule; the derivation function is priced into every play
    that relies on it (the SQL arm's bucketing expression was inside ANOM_SQL
    and therefore counted — same deal here)."""
    return count_loc(inspect.getsource(play_fn)) + count_loc(inspect.getsource(expert_record))


# ── plumbing (not a play; not LOC-counted, same as round 1's HTTP client) ────
def fresh_expert_bundle(g: Gigi, name: str) -> None:
    g.request("DELETE", f"/v1/bundles/{name}")  # 404 fine on first run
    g.ok("POST", "/v1/bundles", {"name": name, "schema": EXPERT_SCHEMA})


def insert_expert(g: Gigi, name: str, rows) -> None:
    records = [expert_record(*r) for r in rows]
    for i in range(0, len(records), INSERT_BATCH):
        g.ok("POST", f"/v1/bundles/{name}/insert",
             {"records": records[i:i + INSERT_BATCH]})


def record_count(g: Gigi, name: str) -> int:
    return int(g.ok("GET", f"/v1/bundles/{name}/schema").get("records", -1))


def verify_cohort_lens(g: Gigi, name: str) -> list[str]:
    """Untimed pre-flight: the cohort contextual lens must actually build and
    the fusion must actually be weighted — otherwise play 1 would silently
    measure something else."""
    status, doc = g.request("POST", f"/v1/bundles/{name}/scan", {
        "budget": SCAN_BUDGET, "limit": 1,
        "exclude": ["merchant", "customer", "hour", "amount_sq"],
        "weights": {"context:cohort": 1.0},
    })
    assert status == 200, doc
    assert "context:cohort" in doc["lenses"], (
        f"context:cohort lens did not build — lenses={doc['lenses']} notes={doc.get('notes')}")
    assert doc["fusion"] == "weighted", doc
    return doc.get("notes", [])


# ── offline request-shape validation (no server needed) ──────────────────────
def validate_shapes_offline() -> None:
    scan_body = {"budget": SCAN_BUDGET, "limit": 0,
                 "exclude": ["merchant", "customer", "hour", "amount_sq"],
                 "weights": {"context:cohort": 1.0}}
    # ScanRequest (src/ml/scan.rs) fields: budget, weights, limit, exclude.
    assert set(scan_body) <= {"budget", "weights", "limit", "exclude"}
    json.dumps(scan_body)
    rec = expert_record("T000001", "coffee", "C0001", 8.437, 5.12)
    assert set(rec) == set(EXPERT_SCHEMA["fields"]), "record/schema field drift"
    assert rec["cohort"] == "coffee|4" and rec["amount_sq"] == 5.12 * 5.12
    # boundary: hour 23.999 -> bucket 11; hour 0.0 -> bucket 0
    assert expert_record("t", "m", "c", 23.999, 1.0)["cohort"] == "m|11"
    assert expert_record("t", "m", "c", 0.0, 1.0)["cohort"] == "m|0"
    gql = (f"INTEGRATE {EXPERT_BUNDLE} OVER cohort "
           f"MEASURE avg(amount) AS mu, avg(amount_sq) AS mu_sq, count(*) AS n;")
    # parser grammar (src/parser.rs parse_integrate): AVG/COUNT in the AggFunc
    # set, one field or * per measure, optional AS alias — all satisfied.
    for frag in ("OVER cohort", "avg(amount)", "avg(amount_sq)", "count(*)", "AS mu"):
        assert frag in gql
    # labels never appear anywhere in either play's request surface
    for blob in (json.dumps(scan_body), gql, json.dumps(rec)):
        assert "label" not in blob.lower()
    print("offline shape validation: PASS")
    print(f"  scan body : {json.dumps(scan_body)}")
    print(f"  gql query : {gql}")
    print(f"  play LOC  : scan_weighted={play_loc(play_scan_cohort_weighted)} "
          f"gql_cohort_z={play_loc(play_gql_cohort_z)} (shared counting rule)")


# ── smoke: tiny throwaway bundle, untimed, obvious planted combo anomalies ───
def smoke(port: int) -> None:
    import random
    g = Gigi(port=port)
    try:
        g.ok("GET", "/v1/health")
    except Exception as e:
        print(f"no dev server on 127.0.0.1:{port} ({e}) — falling back to offline checks")
        validate_shapes_offline()
        return
    print(f"smoke against 127.0.0.1:{port} (untimed, throwaway bundle)")
    rng = random.Random(7)
    merchants = {"espresso": 6.0, "avionics": 900.0, "grocer": 60.0, "inn": 200.0}
    rows, planted = [], []
    for i in range(240):
        m = list(merchants)[i % 4]
        h = round(rng.uniform(8.0, 16.0), 3)          # 4 two-hour buckets
        a = round(merchants[m] * rng.uniform(0.8, 1.25), 2)
        rows.append((f"S{i:04d}", m, f"C{i % 40:04d}", h, a))
    for i in (17, 133):                                # combo plant: espresso row
        t, m, c, h, _ = rows[i]                        # priced like avionics
        rows[i] = (t, "espresso", c, h, round(900.0 * rng.uniform(0.9, 1.1), 2))
        planted.append(t)
    name = EXPERT_BUNDLE + "_smoke"
    fresh_expert_bundle(g, name)
    insert_expert(g, name, rows)
    assert record_count(g, name) == len(rows)
    notes = verify_cohort_lens(g, name)
    print(f"  lens pre-flight OK; notes: {notes}")

    s1 = play_scan_cohort_weighted(g, name)
    s2 = play_gql_cohort_z(g, name, rows)
    for tag, s in (("scan_weighted", s1), ("gql_cohort_z", s2)):
        assert len(s) == len(rows), f"{tag}: scored {len(s)}/{len(rows)}"
        top5 = sorted(s, key=lambda k: -s[k])[:5]
        hit = all(p in top5 for p in planted)
        print(f"  {tag:14s} scored {len(s)} rows; planted in top5: {hit} "
              f"(top5={top5})")
        assert hit, f"{tag}: planted rows {planted} not in top5 {top5} — play is broken"
    g.request("DELETE", f"/v1/bundles/{name}")
    print("smoke: PASS (both plays end-to-end; labels never sent)")


# ── timed run (protocol rule 4 — server on :3143, serial, builds done) ───────
def main() -> None:
    require_dataset()
    if not ROUND1_GIGI.exists():
        sys.exit("results_gigi.json (round-1 zero-config row) missing — the report "
                 "carries that cell verbatim; run round 1 first.")
    with open(ROUND1_GIGI, encoding="utf-8") as f:
        zero_config_cell = json.load(f)["cells"]["anomaly"]

    g = Gigi()
    try:
        health = g.ok("GET", "/v1/health")
    except Exception as e:
        sys.exit(
            f"no gigi server at {GIGI_BASE} ({e}).\n"
            "This runner starts NOTHING. Start the server yourself first, e.g.:\n"
            '  $env:PORT="3143"; cargo run --release --bin gigi_stream\n'
            "then re-run: python run_gigi_expert.py"
        )
    print(f"gigi server: {json.dumps(health)}")

    print("loading 20k labeled subset into expert bundle (untimed prep) ...")
    subset = read_subset()
    fresh_expert_bundle(g, EXPERT_BUNDLE)
    insert_expert(g, EXPERT_BUNDLE, subset)
    assert record_count(g, EXPERT_BUNDLE) == N_SUBSET
    notes = verify_cohort_lens(g, EXPERT_BUNDLE)
    print(f"lens pre-flight OK; notes: {notes}")

    labels = load_labels()
    cells = {"anomaly_zero_config_round1": zero_config_cell}
    aucs = {}

    plays = [
        ("expert_scan_weighted",
         "PLAY 1: POST /scan, geometry scoped to (cohort, amount) via exclude, "
         "all fusion weight on context:cohort (a-priori weights, no labels)",
         lambda: play_scan_cohort_weighted(g, EXPERT_BUNDLE),
         play_scan_cohort_weighted),
        ("expert_gql_cohort_z",
         "PLAY 2: GQL INTEGRATE OVER cohort MEASURE avg(amount), avg(amount_sq), "
         "count(*) server-side + per-row |z| client-side (population stddev, "
         "SQL-arm convention)",
         lambda: play_gql_cohort_z(g, EXPERT_BUNDLE, subset),
         play_gql_cohort_z),
    ]
    for key, method, run, fn in plays:
        print(f"{key} (1 warmup + 3 reps) ...")
        scores = {}

        def once() -> float:
            nonlocal scores
            ms, s = wall_ms(run)
            assert len(s) == N_SUBSET, f"{key}: scored {len(s)}/{N_SUBSET}"
            scores = s
            return ms

        reps = warmup_plus_reps(once)
        auc = average_precision(labels, scores)
        loc = play_loc(fn)
        aucs[key] = auc
        print(f"   wall ms reps: {[round(x, 1) for x in reps]}  "
              f"PR-AUC: {auc:.4f}  LOC: {loc}")
        cells["anomaly_" + key] = cell_anomaly(
            reps, auc, loc,
            "python: play function source + expert_record derivation source, "
            "non-blank non-comment lines (shared counting rule)",
            method, N_SUBSET,
        )

    best = max(aucs, key=lambda k: aucs[k])
    out = {
        "system": "gigi_expert",
        "arm": "ROUND 2 addendum arm 3 — expert-parity anomaly",
        "granted_knowledge": [
            "cohort structure is (merchant x 2h-hour-bucket)",
            "amount is the value channel",
            "NOTHING more: no labels sent, no donor mechanics",
        ],
        "protocol": protocol_info(),
        "environment": env_info(),
        "versions": {
            "gigi_server_health": health,
            "gigi_version": health.get("version", "unknown"),
            "transport": f"HTTP/1.1 keep-alive to {GIGI_BASE}",
        },
        "cells": cells,
        "best_expert_play": {"cell": "anomaly_" + best, "pr_auc": round(aucs[best], 4),
                             "note": "both plays reported; best labeled, none dropped"},
        "disclosures": [
            "EXPERT PARITY: gigi was granted exactly the SQL author's domain "
            "knowledge — cohort = (merchant x 2h bucket), amount = value channel "
            "— and nothing else. Labels were never sent to gigi.",
            "Both candidate plays are reported with their measured PR-AUCs; the "
            "best is labeled best. No cherry-picking.",
            "The expert bundle carries two client-derived fields (cohort, "
            "amount_sq) computed during UNTIMED ingest prep; the SQL arm computes "
            "its bucket inside the timed query. The wall-time asymmetry favors "
            "gigi and is disclosed; the derivation's LOC is priced into every "
            "play's LOC (as the SQL arm's CAST was inside ANOM_SQL).",
            "PLAY 1 weights ({context:cohort: 1.0}) are fixed a priori from the "
            "granted knowledge, not fitted — /scan/fit was NOT used (it needs "
            "labels, which the protocol withholds).",
            "PLAY 1's context:cohort lens is gigi's cohort-local curvature "
            "kappa=|amount-mu|/range z-scored within cohort then rank-normalized "
            "globally — the granted knowledge in gigi's idiom, not a verbatim "
            "z-score reimplementation.",
            "PLAY 2 computes the per-row |z| client-side from server-side "
            "INTEGRATE moments (GQL has no stddev aggregate — round-1 finding "
            "stands); population-stddev convention identical to the SQL arm; "
            "zero-variance cohorts score 0 like the SQL arm's COALESCE.",
            "The round-1 zero-config /scan cell is carried verbatim as "
            "anomaly_zero_config_round1 (PR-AUC 0.0848 as measured in round 1).",
            "All expert cells measured over HTTP at 127.0.0.1:3143 (gigi's real "
            "deployment shape), persistent keep-alive, strictly serial.",
        ],
    }
    with open(RESULTS_PATH, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2)
    print(f"\nwrote {RESULTS_PATH}")


if __name__ == "__main__":
    if "--offline" in sys.argv:
        validate_shapes_offline()
    elif "--smoke" in sys.argv:
        idx = sys.argv.index("--smoke")
        port = int(sys.argv[idx + 1]) if len(sys.argv) > idx + 1 and sys.argv[idx + 1].isdigit() else 3142
        smoke(port)
    else:
        main()
