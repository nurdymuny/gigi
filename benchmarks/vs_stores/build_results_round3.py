"""Assemble results_round3.json from the round-3 runner outputs.

Inputs (all produced by the round-3 serial run):
  embedded_round3.json          stdout of target/release/examples/vs_stores_embedded
  results_gigi.json             run_gigi.py output (round-3 numbers)
  results_gigi_expert.json      run_gigi_expert.py output (round-3 numbers)
  aggcheck_round3.json          run_round3_aggcheck.py output
  gate_totals (inline below)    from bench_logs/gate*.log
Round-2 references are inlined from results_round2.json (committed).
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

from eval_common import env_info

HERE = Path(__file__).resolve().parent

ROUND2 = {
    "embedded_ingest_rows_per_sec": 116526.3,
    "zero_config_scan_pr_auc": 0.0848,
    "zero_config_scan_wall_ms": 317.4,
    "expert_play1_scan_weighted_pr_auc": 0.7003,
    "expert_play1_wall_ms": 287.86,
    "expert_play2_gql_cohort_z_pr_auc": 0.7189,
    "expert_play2_wall_ms": 53.49,
    "aggregate_fallback_wall_ms_round1": 2167.34,
    "sql_expert_pr_auc": 0.7189,
}


def delta(new, old):
    return round(new - old, 4)


def main() -> None:
    emb = json.loads((HERE / "embedded_round3.json").read_text(encoding="utf-8"))
    gigi = json.loads((HERE / "results_round3_gigi.json").read_text(encoding="utf-8"))
    expert = json.loads((HERE / "results_round3_expert.json").read_text(encoding="utf-8"))
    aggchk = json.loads((HERE / "aggcheck_round3.json").read_text(encoding="utf-8"))
    extra = json.loads(sys.argv[1]) if len(sys.argv) > 1 else {}

    emb_ingest = emb["cells"]["ingest"]
    agg = gigi["cells"]["aggregate"]
    anom = gigi["cells"]["anomaly"]
    p1 = expert["cells"]["anomaly_expert_scan_weighted"]
    p2 = expert["cells"]["anomaly_expert_gql_cohort_z"]

    out = {
        "benchmark": "gigi vs stores — ROUND 3 (fix-lane integration re-run)",
        "date_utc": "2026-07-31",
        "integration": extra.get("integration", {}),
        "environment": env_info(),
        "protocol_note": (
            "same datasets (data.csv sha256 verified identical to round-2 manifest "
            "data_100k), same seeds (20260731 dataset, +1 point ids, +2 embedded ids), "
            "same warmup(1)+reps(3)/median rule, strictly serial, committed harness "
            "unmodified; server: release build of the INTEGRATED branch, PORT=3143, "
            "fresh scratch dir, GIGI_SKIP_BOOT_SNAPSHOT=1"
        ),
        "cells_of_record": {
            "embedded_ingest_100k": {
                "unit": "rows_per_sec",
                "reps": emb_ingest["reps"],
                "median": emb_ingest["median"],
                "round2": ROUND2["embedded_ingest_rows_per_sec"],
                "delta": delta(emb_ingest["median"], ROUND2["embedded_ingest_rows_per_sec"]),
                "delta_pct": round(100.0 * (emb_ingest["median"] / ROUND2["embedded_ingest_rows_per_sec"] - 1.0), 2),
            },
            "zero_config_scan_anomaly": {
                "unit": "ms",
                "reps_wall_ms": anom["reps_wall_ms"],
                "wall_ms_median": anom["wall_ms_median"],
                "pr_auc": anom["pr_auc"],
                "round2_pr_auc": ROUND2["zero_config_scan_pr_auc"],
                "delta_pr_auc": delta(anom["pr_auc"], ROUND2["zero_config_scan_pr_auc"]),
                "round2_wall_ms": ROUND2["zero_config_scan_wall_ms"],
                "delta_wall_ms": delta(anom["wall_ms_median"], ROUND2["zero_config_scan_wall_ms"]),
            },
            "expert_play1_scan_weighted": {
                "unit": "ms",
                "reps_wall_ms": p1["reps_wall_ms"],
                "wall_ms_median": p1["wall_ms_median"],
                "pr_auc": p1["pr_auc"],
                "round2_pr_auc": ROUND2["expert_play1_scan_weighted_pr_auc"],
                "delta_pr_auc": delta(p1["pr_auc"], ROUND2["expert_play1_scan_weighted_pr_auc"]),
                "round2_wall_ms": ROUND2["expert_play1_wall_ms"],
                "delta_wall_ms": delta(p1["wall_ms_median"], ROUND2["expert_play1_wall_ms"]),
            },
            "expert_play2_gql_cohort_z": {
                "unit": "ms",
                "reps_wall_ms": p2["reps_wall_ms"],
                "wall_ms_median": p2["wall_ms_median"],
                "pr_auc": p2["pr_auc"],
                "round2_pr_auc": ROUND2["expert_play2_gql_cohort_z_pr_auc"],
                "delta_pr_auc": delta(p2["pr_auc"], ROUND2["expert_play2_gql_cohort_z_pr_auc"]),
                "round2_wall_ms": ROUND2["expert_play2_wall_ms"],
                "delta_wall_ms": delta(p2["wall_ms_median"], ROUND2["expert_play2_wall_ms"]),
            },
            "aggregate_server_side_stddev": {
                "unit": "ms",
                "method": agg["method"],
                "reps_wall_ms": agg["reps_wall_ms"],
                "wall_ms_median": agg["wall_ms_median"],
                "round1_fallback_wall_ms": ROUND2["aggregate_fallback_wall_ms_round1"],
                "delta_wall_ms": delta(agg["wall_ms_median"], ROUND2["aggregate_fallback_wall_ms_round1"]),
                "fallback_recheck": aggchk,
            },
        },
        "honest_regressions": [
            {
                "cell": "expert_play1_scan_weighted",
                "metric": "pr_auc",
                "round2": ROUND2["expert_play1_scan_weighted_pr_auc"],
                "round3": p1["pr_auc"],
                "delta": delta(p1["pr_auc"], ROUND2["expert_play1_scan_weighted_pr_auc"]),
                "cause": (
                    "the interaction context lens skips context:cohort (129 distinct "
                    "values exceeds the 64-value interaction side limit); cohort was "
                    "the field play 1 put all fusion weight on, so the play collapses "
                    "toward zero-config behavior"
                ),
            },
            {
                "cell": "zero_config_scan_anomaly",
                "metric": "wall_ms",
                "round2": ROUND2["zero_config_scan_wall_ms"],
                "round3": anom["wall_ms_median"],
                "delta": delta(anom["wall_ms_median"], ROUND2["zero_config_scan_wall_ms"]),
                "cause": (
                    "the interaction lens does more geometry per scan; the same "
                    "cell's PR-AUC gain is the trade"
                ),
            },
        ],
        "context_cells_not_of_record": {
            "http_ingest": gigi["cells"]["ingest"],
            "http_point_query": gigi["cells"]["point_query"],
            "note": "run_gigi.py measures all four round-1 cells; ingest/point are recorded as context, not round-3 cells of record",
        },
        "regression_guard": extra.get("regression_guard", {}),
        "gates": extra.get("gates", {}),
        "reference": ROUND2,
    }
    (HERE / "results_round3.json").write_text(json.dumps(out, indent=2), encoding="utf-8")
    print("wrote", HERE / "results_round3.json")


if __name__ == "__main__":
    main()
