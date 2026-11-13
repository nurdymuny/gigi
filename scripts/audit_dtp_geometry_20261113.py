"""Independent read-only DTP audit. No DTP imports, servers, builds or timings.

Runs mathematical oracles and rechecks saved receipts, not the Rust implementation.
Writes only its own evidence directory. Requires NumPy and SciPy.
"""
import hashlib
import json
import math
import platform
import statistics
import subprocess
from decimal import Decimal, localcontext
from pathlib import Path

import numpy as np
import scipy
from scipy.integrate import solve_ivp

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "output/virtual-dtp"
OUT = ROOT / "docs/research/evidence/dtp-development-review-20261113"
HASHES = {}


def track(path):
    path = path.resolve()
    data = path.read_bytes()
    HASHES[str(path.relative_to(ROOT)).replace("\\", "/")] = hashlib.sha256(data).hexdigest()
    return data


def read(path):
    return json.loads(track(path).decode("utf-8-sig"))


def close(a, b, tol=2e-12):
    np.testing.assert_allclose(a, b, atol=tol, rtol=tol)


def matrix(register):
    if register["kind"] == "matrix":
        return np.asarray(register["rows"], dtype=float)
    assert register["kind"] == "weighted_permutation"
    p, w = register["permutation"], register["weights"]
    a = np.zeros((len(p), len(p)))
    a[np.arange(len(p)), p] = w
    return a


def product(edges):
    h = np.eye(len(edges[0]))
    for a in edges:
        h = a @ h
    return h


def lc(p, q, v):
    """Shortest non-antipodal unit-sphere arc, tangent input only."""
    close(np.dot(p, p), 1)
    close(np.dot(q, q), 1)
    close(np.dot(p, v), 0)
    assert 1 + np.dot(p, q) > 1e-10
    return v - np.dot(v, q) / (1 + np.dot(p, q)) * (p + q)


def sphere_checks():
    x, y, z = np.eye(3)
    rz = np.array([[0, -1, 0], [1, 0, 0], [0, 0, 1]])
    rx = np.array([[1, 0, 0], [0, 0, -1], [0, 1, 0]])
    ry = np.array([[0, 0, 1], [0, 1, 0], [-1, 0, 0]])
    h = product([rz, rx, ry])
    np.testing.assert_array_equal(h, rx)
    close(h @ y, z)
    close(h @ z, -y)
    close(product([ry.T, rx.T, rz.T]), h.T)
    close(product([rz, rz.T]), np.eye(3))
    for v0 in (y, z):
        v = v0.copy()
        for p, q in ((x, y), (y, z), (z, x)):
            v = lc(p, q, v)
        close(v, h @ v0)
    # This particular tangent misses a wrong middle edge; the whole map must not.
    wrong = product([rz, rx.T, ry])
    close(wrong @ y, z)
    rejected = False
    try:
        close(wrong, h)
    except AssertionError:
        rejected = True
    assert rejected

    # Random non-axis arcs: integrate D_t V = 0 in ambient coordinates.
    rng = np.random.default_rng(20261113)
    worst = 0.0
    for _ in range(24):
        p = rng.normal(size=3); p /= np.linalg.norm(p)
        n = rng.normal(size=3); n -= p * np.dot(p, n); n /= np.linalg.norm(n)
        tangent = np.cross(n, p)
        angle = rng.uniform(0.05, 2.7)
        v = rng.normal(size=3); v -= p * np.dot(p, v); v /= np.linalg.norm(v)

        def rhs(t, value):
            point = math.cos(t) * p + math.sin(t) * tangent
            velocity = -math.sin(t) * p + math.cos(t) * tangent
            return -point * np.dot(velocity, value)

        q = math.cos(angle) * p + math.sin(angle) * tangent
        ode = solve_ivp(rhs, (0, angle), v, method="DOP853", rtol=2e-12, atol=2e-13)
        assert ode.success
        expected = lc(p, q, v)
        error = float(np.max(np.abs(ode.y[:, -1] - expected)))
        worst = max(worst, error)
        close(ode.y[:, -1], expected, 2e-11)
        close(np.dot(q, expected), 0)
        close(np.linalg.norm(expected), np.linalg.norm(v))
        close(lc(q, p, expected), v)

    frames = [np.linalg.qr(rng.normal(size=(3, 3)))[0] for _ in range(3)]
    edges = [rz, rx, ry]
    framed = [frames[(i+1) % 3].T @ a @ frames[i] for i, a in enumerate(edges)]
    close(product(framed), frames[0].T @ h @ frames[0])
    close(np.linalg.norm(product(framed)-np.eye(3))**2, 4)
    s = np.diag([1., 2., 1.]); inv = np.linalg.inv(s)
    transformed = inv @ h @ s - np.eye(3)
    g = s.T @ s
    naive = float(np.linalg.norm(transformed)**2)
    metric = float(np.trace(np.linalg.inv(g) @ transformed.T @ g @ transformed))
    close(naive, 6.25); close(metric, 4)
    return {"holonomy": h.tolist(), "forward_tangent": (h@y).tolist(),
            "angle_radians": math.atan2((h@y)[2], (h@y)[1]),
            "state_error": 2, "operator_error": 4, "random_arc_ode_cases": 24,
            "max_ode_error": worst, "whole_map_wrong_direction_control_caught": rejected,
            "single_tangent_wrong_middle_edge_is_invisible": True,
            "changed_units_naive_error": naive, "metric_corrected_error": metric}


def structured_checks():
    rng = np.random.default_rng(991)
    count = 0
    for n in (2, 3, 8, 32):
        for length in (1, 2, 8, 32, 64):
            for signed in (True, False):
                p, w, dense = np.arange(n), np.ones(n), np.eye(n)
                for _ in range(length):
                    ep = rng.permutation(n)
                    ew = rng.choice([-1., 1.] if signed else [-2., -.5, .5, 2.], n)
                    a = np.zeros((n, n)); a[np.arange(n), ep] = ew
                    dense = a @ dense
                    p, w = p[ep], ew * w[ep]
                compact = np.zeros((n, n)); compact[np.arange(n), p] = w
                np.testing.assert_array_equal(compact, dense)
                defect = float(np.sum(np.where(p == np.arange(n), (w-1)**2, w*w+1)))
                close(defect, np.sum((dense-np.eye(n))**2))
                close(np.sort(np.abs(w)), np.sort(np.linalg.svd(dense, compute_uv=False)))
                count += 1
    eps = math.ulp(0.0)
    excess = math.log(2)**2 / 4
    alpha = math.sqrt(eps) / math.sqrt(eps+excess)
    with localcontext() as ctx:
        ctx.prec = 100
        e, a = Decimal.from_float(eps), Decimal.from_float(excess)
        oracle = float((e/(e+a)).sqrt())
    assert alpha > 0 and abs(alpha/oracle-1) < 1e-14
    return {"composition_spectrum_defect_cases": count, "min_subnormal_alpha": alpha,
            "decimal_alpha": oracle, "relative_error": abs(alpha/oracle-1)}


def sphere_receipts():
    report = read(RAW / "sphere-live/report.json")
    rows = []
    expected_vectors = {"forward": [0, 0, 1], "repeat21": [0, 0, 1],
                        "reverse": [0, 0, -1], "backtrack": [0, 1, 0]}
    for case, modes in report["cases"].items():
        for mode, saved in modes.items():
            machine = saved["input"]
            registers = machine["registers"]
            trace = saved["trace"]
            assert len(saved["steps"]) == len(machine["program"])
            assert len(trace) == len(machine["program"]) + 1
            assert trace[0] == machine
            for pc, (instruction, response) in enumerate(zip(machine["program"], saved["steps"]), 1):
                state = response["state"]
                assert state["pc"] == pc and state == trace[pc]
                out = state["registers"][instruction["out"]]
                v = np.asarray(registers[instruction["vector"]]["data"])
                if instruction["op"] == "transport":
                    expected = matrix(registers[instruction["matrix"]]) @ v
                    close(out["data"], expected, 0)
                else:
                    assert instruction["op"] == "loop"
                    h = product([matrix(registers[e["matrix"]]) for e in instruction["edges"]])
                    close(out["transported"], h @ v, 0)
                    close(out["state_error"], np.sum((h@v-v)**2), 0)
                    close(out["operator_error"], np.sum((h-np.eye(3))**2), 0)
                    low, high = out["numerical"]["operator_error_range"]
                    assert low <= out["operator_error"] <= high
                registers = state["registers"]
            close(registers["v_loop"]["transported"], expected_vectors[case], 0)
            close(registers["p_loop"]["transported"], [1, 0, 0], 0)
            rows.append({"case": case, "mode": mode, "steps_checked": len(saved["steps"]),
                         "loop_edges": len(machine["program"][-1]["edges"]),
                         "tangent": registers["v_loop"]["transported"],
                         "operator_error": registers["v_loop"]["operator_error"]})
    return {"cases": rows, "receipt_binary_sha256": report["binary_sha256"],
            "restart_exact_reported_by_producer": report["restart_exact"],
            "restart_independently_rerun": False}


def benchmark_receipts():
    baseline = read(RAW / "b02/baseline-allocations.json")
    candidate = read(RAW / "b02/candidate-allocations.json")
    key = lambda r: (r["dimension"], r["edges"], r["structured"])
    by_key = {key(r): r for r in candidate}
    allocations = []
    assert len(baseline) == len(by_key) == len(candidate)
    for before in baseline:
        after = by_key[key(before)]
        for field in ("output", "work"):
            assert json.dumps(before[field], sort_keys=True) == json.dumps(after[field], sort_keys=True)
        saved = before["allocation_calls"] - after["allocation_calls"]
        assert saved == (3+3*before["edges"] if before["structured"] else 0)
        allocations.append({"dimension": before["dimension"], "edges": before["edges"],
                            "structured": before["structured"], "requests_saved": saved})
    b01, b02, b03 = [], [], []
    for run in (1, 2):
        for row in read(RAW / f"benchmark-round-1/run-{run}.json")["cases"]:
            medians = {mode: statistics.median(row[mode]["samples_ns"]) for mode in ("general", "structured")}
            for mode in medians:
                close(medians[mode], row[mode]["median_ns"])
            ratio = medians["general"] / medians["structured"]
            close(ratio, row["general_over_structured"])
            b01.append({"run": run, "dimension": row["dimension"], "task": row["task"], "ratio": ratio})
        for row in read(RAW / f"b02/paired-run-{run}.json")["rows"]:
            ratios = [b/a for b, a in zip(row["before_ns"], row["after_ns"])]
            close(ratios, row["paired_ratios"])
            median = statistics.median(ratios)
            close(median, row["median_paired_ratio"])
            b02.append({"run": run, "dimension": row["dimension"], "edges": row["edges"],
                        "structured": row["structured"], "median_paired_ratio": median})
        data = read(RAW / f"b03/run-{run}.json")
        for row in data["rows"]:
            medians = np.array([statistics.median(samples) for samples in row["raw_phase_ns"]])
            close(medians, row["phase_median_ns"])
            shares = medians / sum(medians)
            close(shares, row["shares_of_measured_phases"])
            b03.append({"run": run, "dimension": row["dimension"], "task": row["task"],
                        "structural": row["structural"], "representation_share": float(sum(shares[[0, 1, 5, 6]])),
                        "geometry_share": float(shares[3]), "two_size_checks_share": float(shares[0]+shares[6])})
    return {"allocations": allocations, "b01": b01, "b02": b02, "b03": b03,
            "new_timings_taken": False}


def main():
    sources = ["crates/gigi-virtual-dtp/AGENTS.md", "crates/gigi-virtual-dtp/CONTRACT.md",
               "crates/gigi-virtual-dtp/src/processor.rs", "crates/gigi-virtual-dtp/src/numerics.rs",
               "crates/gigi-virtual-dtp/src/store.rs", "crates/gigi-virtual-dtp/Cargo.lock",
               "crates/gigi-virtual-dtp/examples/benchmark_workloads.rs",
               "crates/gigi-virtual-dtp/verify_sphere_live.py", "src/engine.rs", "src/wal.rs",
               "src/bundle.rs", "src/dials.rs", "src/curvature.rs", "src/holonomy_cycle.rs",
               "src/gauge/group_element.rs", "src/bin/gigi_stream.rs", "src/parser.rs",
               "scripts/audit_dtp_geometry_20261113.py"]
    for source in sources:
        track(ROOT / source)
    report = {"scope": "Independent math and saved-receipt audit; no new Rust/live/performance run",
              "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
              "python": platform.python_version(), "numpy": np.__version__, "scipy": scipy.__version__,
              "sphere_math": sphere_checks(), "structure_math": structured_checks(),
              "saved_sphere_receipts": sphere_receipts(), "saved_benchmark_receipts": benchmark_receipts()}
    for path, digest in HASHES.items():
        assert hashlib.sha256((ROOT/path).read_bytes()).hexdigest() == digest, f"Input changed: {path}"
    report.update(passed=True, input_sha256=HASHES, source_stable_during_audit=True)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "independent-checks.json").write_text(json.dumps(report, indent=2, allow_nan=False)+"\n", encoding="utf-8")
    print(json.dumps({"passed": True, "sphere_math": report["sphere_math"],
                      "structure_math": report["structure_math"], "sphere_receipt_cases": len(report["saved_sphere_receipts"]["cases"]),
                      "allocation_rows": len(report["saved_benchmark_receipts"]["allocations"]),
                      "output": str(OUT / "independent-checks.json")}))


if __name__ == "__main__":
    main()
