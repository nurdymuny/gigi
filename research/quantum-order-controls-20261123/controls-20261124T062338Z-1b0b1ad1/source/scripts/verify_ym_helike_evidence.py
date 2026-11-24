"""Replay stored finite matrices and final exterior checks; never upgrade UNRESOLVED."""

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
import scipy.linalg as la
from threadpoolctl import threadpool_limits

import ym_helike_quantum as q


def check_case(case, arrays):
    e, v, h, s = (arrays[key] for key in ("energies", "vectors", "H", "S"))
    if not all(np.isfinite(a).all() for a in (e, v, h, s, arrays["tensor"], arrays["scalar"])):
        raise ValueError("stored arrays must be finite")
    declared = [case[key] for key in ("E0", "E1", "absolute_gap")]
    declared += [case[name][key] for name in ("H0", "H2")
                 for key in ("lowest_weight", "accessible_gap") if case[name][key] is not None]
    if not np.isfinite(declared).all():
        raise ValueError("declared energy and probe values must be finite")
    values, _, _ = q.solve_generalized(h, s, "cpu")
    np.testing.assert_allclose(e, values, rtol=1e-10, atol=1e-10)
    np.testing.assert_allclose(v.T@s@v, np.eye(len(e)), rtol=0, atol=1e-10)
    if la.norm(h@v-s@v*e) > 1e-10*max(1., la.norm(h)*la.norm(v)):
        raise ValueError("stored eigenvectors fail original residual")
    for name, value in (("E0", e[0]), ("E1", e[1]), ("absolute_gap", e[1]-e[0])):
        if abs(case[name]-value) > 1e-10*max(1., abs(value)):
            raise ValueError(f"declared {name} differs from stored spectrum")
    for channel, probes in (("H2", arrays["tensor"]), ("H0", [arrays["scalar"]])):
        report = q.probe_report(e, v, probes)
        for key in ("lowest_weight", "accessible_gap"):
            if report[key] is None or case[channel][key] is None:
                if report[key] != case[channel][key]:
                    raise ValueError(f"declared {channel} {key} differs")
            elif abs(report[key]-case[channel][key]) > 1e-10*max(1., abs(report[key])):
                raise ValueError(f"declared {channel} {key} differs")


def verify(folder, expected_run_id):
    import run_ym_helike_quantum as runner
    def refuse_constant(value):
        raise ValueError(f"nonfinite JSON constant: {value}")
    receipt = json.loads((folder/"receipt.json").read_text(), parse_constant=refuse_constant)
    if receipt["run_id"] != expected_run_id or receipt["status"] not in ("PASS", "UNRESOLVED"):
        raise ValueError("stale, incomplete, or failed run")
    if receipt["source_hashes"] != runner.hashes():
        raise ValueError("source hashes differ from this verifier's source snapshot")
    for name, digest in receipt["source_hashes"].items():
        if hashlib.sha256((folder/"source"/name).read_bytes()).hexdigest() != digest:
            raise ValueError("source snapshot integrity failure")
    if [c["g2"] for c in receipt["couplings"]] != [1., 0.5, 2.]:
        raise ValueError("registered couplings missing or reordered")
    rebuilt = {}
    for coupling in receipt["couplings"]:
        history = coupling["cases"]
        if [c["cutoff"] for c in history] not in ([4, 6, 8, 10], [4, 6, 8, 10, 12]):
            raise ValueError("cutoff history incomplete")
        for case in history:
            expected_name = f"N{case['cutoff']}-g2-{coupling['g2']:g}.npz"
            if case["arrays"] != expected_name:
                raise ValueError("unexpected array filename")
            path = folder/expected_name
            if hashlib.sha256(path.read_bytes()).hexdigest() != case["arrays_sha256"]:
                raise ValueError("array integrity failure")
            with np.load(path, allow_pickle=False) as arrays:
                check_case(case, arrays)
                cutoff = case["cutoff"]
                if cutoff not in rebuilt:
                    b = q.build_basis(cutoff)
                    rebuilt[cutoff] = b, q.build_operators(b)
                b, ops = rebuilt[cutoff]
                expected_h = ops["H0"]-3*np.sqrt(coupling["g2"])*ops["D"]+coupling["g2"]*ops["V4"]
                for actual, expected in ((arrays["H"], expected_h), (arrays["S"], b["S"]),
                                         (arrays["tensor"], np.array(ops["tensor"])), (arrays["scalar"], ops["scalar"])):
                    np.testing.assert_allclose(actual, expected, rtol=1e-10, atol=1e-10)
                if case is history[-1]:
                    ext = q.exterior_diagnostics(b, ops, arrays["energies"], arrays["vectors"], coupling["g2"])
                    np.testing.assert_allclose(ext["level_worst_residuals"], case["exterior"]["level_worst_residuals"], rtol=1e-10, atol=1e-10)
                    for name in ("H0", "H2"):
                        np.testing.assert_allclose(ext["probe_discarded_fractions"][name], case["exterior"]["probe_discarded_fractions"][name], rtol=1e-10, atol=1e-10)
                    if any(ext[key] != case["exterior"][key] for key in ("tail_gate", "residual_gate")):
                        raise ValueError("forged exterior gate")
        if bool(q.convergence(history)) != coupling["converged"]:
            raise ValueError("forged convergence verdict")
    convergence = all(c["converged"] for c in receipt["couplings"])
    if receipt["gates"]["convergence"] != convergence:
        raise ValueError("forged convergence gate")
    if receipt["status"] == "PASS" and not q.validate_receipt(receipt, expected_run_id, runner.hashes(), runner.GATES):
        raise ValueError("scientific acceptance gates incomplete")
    return {"integrity": "VERIFIED", "run_id": expected_run_id,
            "survey_status": receipt["status"], "scientific_acceptance": receipt["status"] == "PASS",
            "replayed_cases": sum(len(c["cases"]) for c in receipt["couplings"])}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folder", type=Path)
    parser.add_argument("--run-id", required=True)
    args = parser.parse_args()
    with threadpool_limits(limits=1):
        print(json.dumps(verify(args.folder, args.run_id), indent=2))
