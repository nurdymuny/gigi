"""ROUND 3 supplementary check — server-side stddev vs the round-1/2 fallback.

The fix/gql-stddev lane teaches INTEGRATE a population stddev aggregate, so
run_gigi.py's runtime probe now takes the server-side path. This script:

  1. re-times the FALLBACK path (INTEGRATE avg+count + 12x COVER + client-side
     population stddev) with the same warmup(1)+reps(3) rule, against the same
     live server and the same ingested `bench` bundle (run AFTER run_gigi.py);
  2. verifies the fallback's per-merchant mean/stddev numerically MATCH the
     server-side stddev results (max abs + max rel diff reported).

Prints one JSON blob to stdout.
"""
from __future__ import annotations

import json
import math

from eval_common import MERCHANTS, N_ROWS, warmup_plus_reps, wall_ms, median
from run_gigi import Gigi, extract_rows

BUNDLE = "bench"


def main() -> None:
    g = Gigi()
    g.ok("GET", "/v1/health")

    # server-side stddev (values only, untimed here — the timed cell of record
    # is run_gigi.py's aggregate cell)
    stddev_q = f"INTEGRATE {BUNDLE} OVER merchant MEASURE avg(amount), stddev(amount);"
    doc = g.ok("POST", "/v1/gql", {"query": stddev_q})
    server_rows = extract_rows(doc)
    assert len(server_rows) >= len(MERCHANTS), doc
    server = {}
    for r in server_rows:
        avg_key = next(k for k in r if k.startswith("avg"))
        sd_key = next(k for k in r if k.startswith("stddev"))
        server[r["merchant"]] = (float(r[avg_key]), float(r[sd_key]))
    assert set(server) == set(MERCHANTS), sorted(server)

    # fallback path, timed exactly as run_gigi.py would have timed it
    avg_q = f"INTEGRATE {BUNDLE} OVER merchant MEASURE avg(amount), count(*);"

    def fallback_once():
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

    fallback_result = {}

    def timed_once() -> float:
        nonlocal fallback_result
        ms, res = wall_ms(fallback_once)
        fallback_result = res["per_merchant"]
        return ms

    reps = warmup_plus_reps(timed_once)

    max_abs = {"mean": 0.0, "stddev": 0.0}
    max_rel = {"mean": 0.0, "stddev": 0.0}
    for name in MERCHANTS:
        s_mu, s_sd = server[name]
        f_mu = fallback_result[name]["mean"]
        f_sd = fallback_result[name]["stddev"]
        for key, sv, fv in (("mean", s_mu, f_mu), ("stddev", s_sd, f_sd)):
            d = abs(sv - fv)
            max_abs[key] = max(max_abs[key], d)
            if abs(sv) > 0:
                max_rel[key] = max(max_rel[key], d / abs(sv))

    match = max_rel["mean"] < 1e-9 and max_rel["stddev"] < 1e-9
    print(json.dumps({
        "fallback_reps_wall_ms": [round(x, 2) for x in reps],
        "fallback_wall_ms_median": round(median(reps), 2),
        "match_server_vs_fallback": match,
        "max_abs_diff": max_abs,
        "max_rel_diff": max_rel,
        "n_merchants": len(MERCHANTS),
        "convention": "population stddev both sides (divisor n)",
    }, indent=2))


if __name__ == "__main__":
    main()
