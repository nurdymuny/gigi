"""Bounded registered survey; supervisor enforces time and host-memory limits."""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILES = ["scripts/ym_helike_quantum.py", "scripts/test_ym_helike_quantum.py",
         "scripts/check_ym_helike_mutations.py",
         "scripts/verify_ym_helike_evidence.py",
         "scripts/run_ym_helike_quantum.py", "scripts/ym_helike_probe_contract.py",
         "docs/research/helike-quantum-probe-preregistration-2026-11-23.md",
         "docs/research/helike-quantum-gpu-amendment-2026-11-23.md",
         "docs/research/helike-quantum-implementation-2026-11-23.md"]
GATES = ["focused_tests", "mutations", "algebra", "independent", "oscillator", "cpu_gpu_agreement",
         "propagator", "convergence", "utility_completed"]


def hashes():
    return {name: hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in FILES}


def publish(folder, receipt):
    temporary = folder/f"receipt.{os.getpid()}.pending"
    temporary.write_text(json.dumps(receipt, indent=2, allow_nan=False)+"\n", encoding="utf-8")
    temporary.replace(folder/"receipt.json")


def utility(b, ops, e, v, h):
    import numpy as np
    import scipy.linalg as la
    import ym_helike_quantum as q
    l = la.cholesky(b["S"], lower=True)
    whiten = lambda x: la.solve_triangular(l, la.solve_triangular(l, x, lower=True).T, lower=True).T
    hw = whiten(h)-e[0]*np.eye(len(h))
    omega = l.T@v[:, 0]
    jw = whiten(ops["J2"])
    angular, rotations = la.eigh(jw, driver="evd")
    out = {}
    for name, probes, casimir in (("H2", ops["tensor"], 6.), ("H0", [ops["scalar"]], 0.)):
        seed = np.column_stack([la.solve_triangular(l, o@v[:, 0], lower=True) for o in probes])
        physical = rotations[:, np.abs(angular-casimir) < 1e-8]
        reports = {"helike": q.gpu_krylov(hw, seed, omega), "random": []}
        # Match the seed's occupied oscillator degrees, not its energy eigenvectors.
        active = np.sum(np.abs(seed), axis=1) > 1e-12
        active_degrees = set(b["degrees"][active].tolist())
        allowed = np.isin(b["degrees"], list(active_degrees))
        for rng_seed in (11, 23, 47, 89, 173):
            random = np.random.default_rng(rng_seed).normal(size=(len(h), len(probes)))
            random[~allowed] = 0
            random = physical@(physical.T@random)
            reports["random"].append(dict(q.gpu_krylov(hw, random, omega), seed=rng_seed))
        if name == "H0":
            plain = la.solve_triangular(l, ops["trace"]@v[:, 0], lower=True)[:, None]
            reports["trace_control"] = q.gpu_krylov(hw, plain, omega)
        target = q.probe_report(e, v, probes)["accessible_gap"]
        def cost(report):
            for check in report["checkpoints"]:
                if target is not None and abs(check["gap"]-target) <= 1e-3*max(1., abs(target)) and check["residual"] <= 1e-10:
                    return check["matvecs"]
            return None
        helike_cost = cost(reports["helike"])
        control_costs = [cost(r) for r in reports["random"]]
        reports.update(target_gap=target, helike_cost=helike_cost, control_costs=control_costs,
                       twofold_advantage=(helike_cost is not None and all(c is not None for c in control_costs)
                                          and 2*helike_cost <= float(np.median(control_costs))))
        out[name] = reports
    return out


def worker(folder):
    import numpy as np
    import scipy.linalg as la
    import torch
    import unittest
    from threadpoolctl import threadpool_limits
    import ym_helike_quantum as q
    import ym_helike_probe_contract as contract
    import test_ym_helike_quantum as tests
    import check_ym_helike_mutations as mutations
    receipt = json.loads((folder/"receipt.json").read_text())
    started = time.perf_counter()
    try:
        if hashes() != receipt["source_hashes"]:
            raise RuntimeError("sources changed before worker startup")
        torch.set_num_threads(1)
        torch.cuda.set_per_process_memory_fraction(4*1024**3/torch.cuda.get_device_properties(0).total_memory)
        torch.cuda.reset_peak_memory_stats()
        receipt["environment"] = {"torch": torch.__version__, "cuda_runtime": torch.version.cuda,
                                  "gpu": torch.cuda.get_device_name(0), "numpy": np.__version__}
        with (folder/"focused-tests.txt").open("w", encoding="utf-8") as stream:
            test_result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromModule(tests))
        receipt["test_count"] = test_result.testsRun
        receipt["gates"]["focused_tests"] = test_result.wasSuccessful()
        receipt["algebra_checks"] = contract.check_contract(None)
        receipt["gates"]["algebra"] = all(receipt["algebra_checks"].values())
        receipt["mutation_checks"] = mutations.run()
        receipt["gates"]["mutations"] = all(r["rejected"] for r in receipt["mutation_checks"])
        if not all(receipt["gates"][k] for k in ("focused_tests", "algebra", "mutations")):
            raise RuntimeError("construction tests failed; no survey allowed")
        b = q.build_basis(4)
        ops = q.build_operators(b)
        reference = q.independent_reference(4, 1)
        receipt["independent_error"] = q.compare_reference(b, ops, reference)
        receipt["independent_gauss_residual"] = reference["gauss_residual"]
        receipt["gates"]["independent"] = receipt["independent_error"] <= 1e-10 and reference["dimension"] == 29
        osc_e, osc_v, osc_diag = q.solve_generalized(ops["H0"], b["S"], "cuda")
        receipt["oscillator"] = {"energies": osc_e[:8].tolist(), "solver": osc_diag,
                                  "H2": q.probe_report(osc_e, osc_v, ops["tensor"]),
                                  "H0": q.probe_report(osc_e, osc_v, [ops["scalar"]])}
        receipt["gates"]["oscillator"] = bool(np.max(np.abs(osc_e[:7]-np.array([4.5]+[6.5]*6))) <= 1e-10)
        if not receipt["gates"]["independent"] or not receipt["gates"]["oscillator"]:
            raise RuntimeError("independent or oscillator gate failed")
        publish(folder, receipt)
        print("Construction gates passed; beginning registered interacting survey", flush=True)
        cache = {4: (b, ops)}
        agreement, propagators = [], []
        for g2 in (1., 0.5, 2.):
            history = []
            receipt["couplings"].append({"g2": g2, "cases": history, "converged": False})
            for cutoff in (4, 6, 8, 10, 12):
                if cutoff == 12 and q.convergence(history):
                    break
                print(f"g2={g2:g}, N={cutoff}: assembling", flush=True)
                tick = time.perf_counter()
                reused = cutoff in cache
                if not reused:
                    b = q.build_basis(cutoff)
                    ops = q.build_operators(b)
                    cache[cutoff] = (b, ops)
                b, ops = cache[cutoff]
                assembly_seconds = time.perf_counter()-tick
                h = ops["H0"]-3*np.sqrt(g2)*ops["D"]+g2*ops["V4"]
                e, v, solve = q.solve_generalized(h, b["S"], "cuda")
                ec, vc, cpu = q.solve_generalized(h, b["S"], "cpu")
                eigen_error = float(np.max(np.abs(e-ec))/max(1., np.max(np.abs(ec))))
                l = la.cholesky(b["S"], lower=True)
                groups = q.energy_groups(e)
                projector_errors = []
                spins = []
                for group in groups[:3]:
                    ug, uc = l.T@v[:, group], l.T@vc[:, group]
                    projector_errors.append(float(la.norm(ug@ug.T-uc@uc.T)))
                    spins.append(la.eigvalsh(v[:, group].T@ops["J2"]@v[:, group]).tolist())
                agrees = eigen_error <= 1e-10 and max(projector_errors) <= 1e-10
                agreement.append(bool(agrees))
                if not agrees:
                    raise ArithmeticError(f"CPU/CUDA disagreement at {g2=}, {cutoff=}")
                case = {"cutoff": cutoff, "dimension": len(e), "E0": float(e[0]), "E1": float(e[1]),
                        "absolute_gap": float(e[1]-e[0]), "gpu_solver": solve, "cpu_solver": cpu,
                        "assembly_seconds": assembly_seconds, "assembly_reused": reused,
                        "cpu_gpu_eigen_error": eigen_error, "projector_errors": projector_errors,
                        "low_level_spin_casimirs": spins, "gauss_norms": ops["gauss_norms"],
                        "H2": q.probe_report(e, v, ops["tensor"]), "H0": q.probe_report(e, v, [ops["scalar"]])}
                print(f"g2={g2:g}, N={cutoff}: gap={case['absolute_gap']:.12g}; checking exterior", flush=True)
                case["exterior"] = q.exterior_diagnostics(b, ops, e, v, g2)
                spectrum_file = folder/f"N{cutoff}-g2-{g2:g}.npz"
                np.savez_compressed(spectrum_file, energies=e, vectors=v, H=h, S=b["S"], J2=ops["J2"],
                                    tensor=np.array(ops["tensor"]), scalar=ops["scalar"])
                case["arrays"] = spectrum_file.name
                case["arrays_sha256"] = hashlib.sha256(spectrum_file.read_bytes()).hexdigest()
                case["elapsed_seconds"] = time.perf_counter()-tick
                history.append(case)
                publish(folder, receipt)
            # Independent time evolution is checked at the largest computed cutoff.
            propagation = q.propagator_check(h, b["S"], e, v, ops["tensor"]+[ops["scalar"]])
            receipt["couplings"][-1]["propagator"] = propagation
            propagators.append(propagation["max_error"] <= 1e-10)
            converged = bool(q.convergence(history))
            receipt["couplings"][-1]["converged"] = converged
            if converged and propagators[-1]:
                receipt["couplings"][-1]["utility"] = utility(b, ops, e, v, h)
            else:
                receipt["couplings"][-1]["utility"] = {"status": "NOT_RUN", "reason": "convergence or propagation prerequisite not met"}
            publish(folder, receipt)
        receipt["gates"]["cpu_gpu_agreement"] = bool(agreement) and all(agreement)
        receipt["gates"]["propagator"] = bool(propagators) and all(propagators)
        receipt["gates"]["convergence"] = all(c["converged"] for c in receipt["couplings"])
        receipt["gates"]["utility_completed"] = all("H2" in c["utility"] for c in receipt["couplings"])
        receipt["status"] = "PASS" if all(receipt["gates"].values()) else "UNRESOLVED"
        receipt["scientific_status"] = ("numerically_converged_model_estimate" if receipt["gates"]["convergence"] else "finite_cutoff_observations_only")
        receipt["elapsed_seconds"] = time.perf_counter()-started
        receipt["peak_gpu_allocated_bytes"] = torch.cuda.max_memory_allocated()
        receipt["peak_gpu_reserved_bytes"] = torch.cuda.max_memory_reserved()
        if hashes() != receipt["source_hashes"]:
            raise RuntimeError("sources changed during survey; evidence cannot be accepted")
        if receipt["status"] == "PASS" and not q.validate_receipt(receipt, receipt["run_id"], hashes(), GATES):
            raise RuntimeError("receipt verification failed")
        publish(folder, receipt)
        print(f"Survey terminal status: {receipt['status']}; {folder}", flush=True)
        return 0 if receipt["status"] == "PASS" else 2
    except BaseException as exc:
        receipt.update(status="FAILED", error=f"{type(exc).__name__}: {exc}", elapsed_seconds=time.perf_counter()-started)
        publish(folder, receipt)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", type=Path)
    args = parser.parse_args()
    if args.worker:
        from threadpoolctl import threadpool_limits
        with threadpool_limits(limits=1):
            return worker(args.worker)
    import psutil
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")+"-"+uuid.uuid4().hex[:8]
    folder = ROOT/"research/helike-quantum-probe-20261123"/("survey-"+run_id)
    folder.mkdir(parents=True, exist_ok=False)
    for name in FILES:
        target = folder/"source"/name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT/name, target)
    receipt = {"status": "RUNNING", "run_id": run_id, "source_hashes": hashes(),
               "required_gates": GATES, "gates": {key: False for key in GATES}, "couplings": [],
               "source_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
               "source_state": "uncommitted implementation identified by per-file hashes",
               "started_utc": datetime.now(timezone.utc).isoformat()}
    publish(folder, receipt)
    if any(hashlib.sha256((folder/"source"/name).read_bytes()).hexdigest() != digest
           for name, digest in receipt["source_hashes"].items()):
        raise RuntimeError("source snapshot changed during capture")
    environment = dict(os.environ, OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1", MKL_NUM_THREADS="1")
    process = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "--worker", str(folder)],
                               cwd=ROOT, env=environment)
    started, peak = time.monotonic(), 0
    try:
        while process.poll() is None:
            try:
                rss = psutil.Process(process.pid).memory_info().rss
            except psutil.NoSuchProcess:
                break
            peak = max(peak, rss)
            if time.monotonic()-started > 2700 or rss > 8*1024**3:
                process.kill(); process.wait()
                receipt = json.loads((folder/"receipt.json").read_text())
                receipt.update(status="RESOURCE_STOP", peak_host_rss_bytes=peak)
                publish(folder, receipt)
                return 2
            time.sleep(0.5)
        code = process.wait()
    except BaseException:
        process.kill(); process.wait()
        receipt = json.loads((folder/"receipt.json").read_text())
        receipt.update(status="FAILED", error="supervisor interrupted")
        publish(folder, receipt)
        raise
    receipt = json.loads((folder/"receipt.json").read_text())
    receipt["peak_host_rss_bytes"] = peak
    if receipt["status"] == "RUNNING":
        receipt.update(status="FAILED", error=f"worker exited {code} without terminal receipt")
    publish(folder, receipt)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
