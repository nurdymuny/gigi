"""Registered dome (second-envelope) test: docs/research/helike-dome-envelope-preregistration-2026-11-24.md.

usage: python scripts/run_ym_helike_dome.py OUT_DIR

Fail-closed: OUT_DIR must not exist or must be empty. The receipt (OUT_DIR/receipt.json) is RUNNING while it works,
then ends with status COMPLETE (all validity gates pass; the scientific status is in `decision`), INVALID (a gate
failed) or FAILED (an exception). The exit code is nonzero unless COMPLETE. Nothing in the design can be changed
from the command line except --smoke, which runs a tiny unregistered configuration for software checks only.
"""
import argparse
import hashlib
import json
import math
import os
import platform
import sys
import time
import traceback
import shutil
import subprocess
import csv
from pathlib import Path

import numpy as np
import mpmath as mp
from threadpoolctl import threadpool_limits

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ym_helike_dome as dm  # noqa: E402
import ym_helike_j0 as j0  # noqa: E402

DESIGN = {
    "cases": [[1.0, 1.7], [2.0, 2.3]],
    "cutoffs": [12, 14, 16, 18],
    "dome_sizes": [0, 3, 4],
    "primary_dome_size": 4,
    "beta_grid": [0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0],
    "beta_selection_cutoff": 10,
    "K": 30, "K_sensitivity": 36, "dps": 80, "check_dps": 120,
}
SMOKE = dict(DESIGN, cases=[[1.0, 1.7]], cutoffs=[6], dome_sizes=[0, 4], beta_grid=[0.9, 1.1, 1.3], force_beta=1.1,
             beta_selection_cutoff=6, K=16, K_sensitivity=18, dps=60, check_dps=80)
LEVELS = ("vacuum", "scalar", "scalar_next")
ROOT = HERE.parent
SOURCES = tuple("scripts/" + s for s in ("ym_helike_dome.py", "ym_helike_j0.py", "run_ym_helike_dome.py",
    "test_ym_helike_dome.py", "test_ym_helike_j0.py", "test_dome_execution.py",
    "check_ym_helike_dome_mutations.py")) + (
    "docs/research/helike-dome-envelope-preregistration-2026-11-24.md",)
HOST_LIMIT, TIME_LIMIT = 8 * 1024**3, 45 * 60


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def write(path, obj):
    tmp = path.with_suffix(".tmp")
    with tmp.open("w", encoding="utf-8") as f:
        f.write(json.dumps(obj, indent=1, allow_nan=False)); f.flush(); os.fsync(f.fileno())
    os.replace(tmp, path)


def slim(sol):
    keys = ("dimension", "n_origin", "n_dome", "beta", "n_d", "K", "dps", "gram_digits_lost", "energies",
            "residuals", "outside_origin_fraction", "min_exterior_eig")
    return {k: sol[k] for k in keys if k in sol}


def gains(base, aug, nxt):
    n_dome, dn = aug["n_dome"], nxt["dimension"] - base["dimension"]
    out = {}
    for i, lev in enumerate(LEVELS):
        e0, ed, en = base["residuals"][i], aug["residuals"][i], nxt["residuals"][i]
        out[lev] = {"eps_origin": e0, "eps_dome": ed, "eps_next_cutoff": en,
                    "r_dome": math.log(e0 / ed) / n_dome, "r_orig": math.log(e0 / en) / dn,
                    "outside_origin_fraction": aug["outside_origin_fraction"][i]}
    out["H1a"] = out["scalar"]["r_dome"] > out["scalar"]["r_orig"]
    out["H1b"] = out["scalar"]["r_dome"] > out["vacuum"]["r_dome"]
    out["n_dome"], out["delta_n_origin"] = n_dome, dn
    return out


def decide(primary, gates, refused, expected):
    if refused or not primary:
        return "REFUSED_BETA_EDGE"
    if len(primary) != expected or not all(v for k, v in gates.items() if k != "D04"):
        return "INVALID"
    if not gates["D04"]:
        return "UNRESOLVED_TRUNCATION"
    if all(c["H1a"] and c["H1b"] for c in primary):
        return "SUPPORTED"
    if not any(c["H1a"] for c in primary):
        return "NOT_SUPPORTED"
    return "MIXED"


def repeated_dome_sizes(design):
    return list(design["dome_sizes"])


def event(out, phase, **data):
    row = dict(utc=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), phase=phase, **data)
    with (out / "events.jsonl").open("a", encoding="utf-8") as f:
        f.write(json.dumps(row, allow_nan=False) + "\n"); f.flush(); os.fsync(f.fileno())
    print(json.dumps(row, allow_nan=False), flush=True)


def source_hashes():
    return {s: sha(ROOT / s) for s in SOURCES}


def initialize(out, design, smoke):
    if out.exists() and any(out.iterdir()):
        raise ValueError(f"refusing to write into non-empty {out}")
    out.mkdir(parents=True, exist_ok=True)
    rec = dict(status="READY", registration=SOURCES[-1], smoke=smoke, design=design,
        sources_sha256=source_hashes(), source_revision=subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        started_utc=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), python=sys.version,
        platform=platform.platform(), certified=False, cases=[], solves=[],
        budgets=dict(host_bytes=HOST_LIMIT, seconds=TIME_LIMIT))
    for name, digest in rec["sources_sha256"].items():
        target = out / "source" / name; target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / name, target)
        if sha(target) != digest:
            raise ValueError("source drift while snapshotting")
    write(out / "receipt.json", rec)
    return rec


def precision_check(low, high):
    if any(len(s[k]) != 6 or not np.isfinite(s[k]).all() for s in (low, high) for k in ("energies", "residuals")):
        raise ValueError("precision check requires six finite eigenpairs")
    de = max(abs(a-b) for a,b in zip(low["energies"], high["energies"]))
    dr = max(abs(a-b)/max(abs(a), 1e-300) for a,b in zip(low["residuals"], high["residuals"]))
    return dict(dps=high["dps"], max_abs_dE=de, max_rel_dR=dr, D03=de <= 1e-11 and dr <= 1e-6)


def supervise(process, out):
    import psutil
    start = time.monotonic(); peak = 0
    try:
        with (out / "resources.csv").open("w", newline="", encoding="utf-8") as f:
            writer = csv.writer(f); writer.writerow(["elapsed_seconds", "rss_bytes", "cpu_seconds"])
            while process.poll() is None:
                try:
                    p = psutil.Process(process.pid); rss = p.memory_info().rss; cpu = p.cpu_times()
                except psutil.NoSuchProcess:
                    break
                elapsed = time.monotonic()-start; peak = max(peak,rss)
                writer.writerow([elapsed,rss,cpu.user+cpu.system]); f.flush()
                if rss > HOST_LIMIT or elapsed > TIME_LIMIT:
                    process.kill(); process.wait()
                    r = json.loads((out / "receipt.json").read_text())
                    r.update(status="RESOURCE_STOP", decision="NOT_EVALUATED", peak_host_rss_bytes=peak,
                             supervised_seconds=elapsed)
                    write(out / "receipt.json", r)
                    write(out / "verification.json", dict(status="FAIL", reason="resource limit"))
                    return 2
                time.sleep(.5)
        code = process.wait()
        r = json.loads((out / "receipt.json").read_text())
        r.update(peak_host_rss_bytes=peak, supervised_seconds=time.monotonic()-start)
        if r["status"] in ("READY", "RUNNING"):
            r.update(status="FAILED", decision="NOT_EVALUATED", error=f"worker exited {code}")
        write(out / "receipt.json", r)
        return code or (0 if r["status"] == "COMPLETE" else 1)
    except BaseException:
        process.kill(); process.wait()
        r = json.loads((out / "receipt.json").read_text())
        r.update(status="FAILED", decision="NOT_EVALUATED", error="supervisor interrupted")
        write(out / "receipt.json", r)
        raise


def verify(out):
    write(out / "verification.json", dict(status="RUNNING"))
    try:
        rec = json.loads((out / "receipt.json").read_text())
        if rec["status"] not in ("COMPLETE", "INVALID") or rec["sources_sha256"] != source_hashes():
            raise ValueError("incomplete run or source drift")
        verify_contract(rec)
        for name,digest in rec["sources_sha256"].items():
            if sha(out / "source" / name) != digest:
                raise ValueError("source snapshot changed")
        for row in rec["solves"]:
            if sha(out / row["file"]) != row["sha256"]:
                raise ValueError("matrix archive changed")
            with np.load(out / row["file"], allow_pickle=False) as z, mp.workdps(row["dps"]):
                h, x = mp.matrix(z["H"].tolist()), mp.matrix(z["X"].tolist())
                np.testing.assert_allclose(np.linalg.eigvalsh(np.array(h.tolist(), dtype=float))[:6], z["E"], atol=1e-11, rtol=0)
                np.testing.assert_allclose(z["V"].T @ z["V"], np.eye(6), atol=1e-10, rtol=0)
                residuals,_ = j0.residuals_from_matrices(h,x,z["E"],z["V"],row["dps"])
                np.testing.assert_allclose(residuals,z["R"],atol=1e-14,rtol=1e-10)
                np.testing.assert_array_equal(z["E"], row["solution"]["energies"])
                np.testing.assert_array_equal(z["R"], row["solution"]["residuals"])
                if "S" in z:
                    fr = dm.outside_origin(mp.matrix(z["S"].tolist()), z["V"], row["solution"]["n_origin"], row["dps"])
                    np.testing.assert_allclose(fr, row["solution"]["outside_origin_fraction"], atol=1e-14, rtol=1e-10)
        result = dict(status="PASS", replayed_solves=len(rec["solves"]), decision=rec["decision"],
                      certified=False, replay="saved matrices, not fresh physical assembly")
        write(out / "verification.json", result)
        return result
    except BaseException as exc:
        write(out / "verification.json", dict(status="FAIL", error=str(exc)))
        raise


def verify_contract(rec):
    D = SMOKE if rec["smoke"] else DESIGN
    if rec["design"] != D or [(c["g2"],c["alpha"]) for c in rec["cases"]] != [tuple(c) for c in D["cases"]]:
        raise ValueError("incomplete or unregistered cases")
    used = set(); primary = []; refused = []
    gates = {f"D{i:02}": True for i in range(6)}

    def get(role,n,g,a,**kw):
        rows = [r for r in rec["solves"] if (r["role"],r["cutoff"],r["g2"],r["alpha"],r["parameters"]) == (role,n,g,a,kw)]
        if len(rows) != 1:
            raise ValueError("missing or duplicated solve: " + role)
        used.add(rows[0]["file"])
        return rows[0]["solution"]

    def equal(actual,expected,label):
        if actual != expected:
            raise ValueError("receipt differs from replay: " + label)

    for case in rec["cases"]:
        g,a = case["g2"],case["alpha"]
        sel = {str(b):get("beta_selection",D["beta_selection_cutoff"],g,a,n_d=0,beta=b,K=D["K"],dps=D["dps"])["energies"][1]
               for b in D["beta_grid"]}
        best = min(D["beta_grid"],key=lambda b:(sel[str(b)],b))
        edge = best in (D["beta_grid"][0],D["beta_grid"][-1])
        if "force_beta" in D:
            best,edge = D["force_beta"],False
        equal(case["beta_selection"],dict(second_ritz_value=sel,beta=best,edge=edge),"width selection")
        if edge:
            equal(case.get("refused"),"BETA_AT_GRID_EDGE","width refusal")
            equal(case.get("points",[]),[],"refused points")
            refused.append(case); continue
        equal([p["cutoff"] for p in case["points"]],D["cutoffs"],"cutoff coverage")
        for p in case["points"]:
            n = p["cutoff"]
            base = get("origin",n,g,a,dps=D["dps"])
            nxt = get("next_cutoff",n+2,g,a,dps=D["dps"])
            ref = get("reference",n,g,a,dps=60,nlev=6)
            equal(p["origin"],base,"origin"); equal(p["next_cutoff"],nxt,"comparator")
            d00 = all(max(abs(x-y) for x,y in zip(base[k],ref[k])) <= 1e-12 for k in ("energies","residuals"))
            equal(p["D00"],d00,"D00"); gates["D00"] &= d00
            equal(set(p["dome"]),set(map(str,D["dome_sizes"])),"dome coverage")
            for nd in D["dome_sizes"]:
                aug = get("dome",n,g,a,n_d=nd,beta=best,K=D["K"],dps=D["dps"])
                d01 = all(x <= y+1e-12 for x,y in zip(aug["energies"][:3],base["energies"][:3]))
                d05 = all(-1e-12 <= f <= 1+1e-12 for f in aug["outside_origin_fraction"])
                c = gains(base,aug,nxt)
                equal(p["dome"][str(nd)],dict(solution=aug,comparison=c,D01=d01,D05=d05),"dome comparison")
                gates["D01"] &= d01; gates["D05"] &= d05
                if nd == D["primary_dome_size"]:
                    primary.append(c)
            if n == D["cutoffs"][-1]:
                equal(set(p["precision_checks"]),{"origin","next_cutoff"}|set(map(str,D["dome_sizes"])),"precision coverage")
                equal(set(p["K_sensitivity"]),set(map(str,D["dome_sizes"])),"truncation coverage")
                for name,low,nn in (("origin",base,n),("next_cutoff",nxt,n+2)):
                    check = precision_check(low,get("precision_"+name,nn,g,a,dps=D["check_dps"]))
                    equal(p["precision_checks"][name],check,"precision"); gates["D03"] &= check["D03"]
                for nd in repeated_dome_sizes(D):
                    aug = get("K_sensitivity",n,g,a,n_d=nd,beta=best,K=D["K_sensitivity"],dps=D["dps"])
                    ck = gains(base,aug,nxt); c0 = p["dome"][str(nd)]["comparison"]
                    d04 = (ck["H1a"],ck["H1b"]) == (c0["H1a"],c0["H1b"])
                    equal(p["K_sensitivity"][str(nd)],dict(solution=aug,comparison=ck,D04=d04),"truncation")
                    gates["D04"] &= d04
                    hi = get("precision_dome",n,g,a,n_d=nd,beta=best,K=D["K"],dps=D["check_dps"])
                    check = precision_check(p["dome"][str(nd)]["solution"],hi)
                    equal(p["precision_checks"][str(nd)],check,"precision"); gates["D03"] &= check["D03"]
    if len(used) != len(rec["solves"]):
        raise ValueError("unexpected or duplicated stored solves")
    gates["D02"] = all(r["solution"]["gram_digits_lost"] < r["dps"]-25 for r in rec["solves"])
    equal(rec["gates"],gates,"validity gates")
    equal(rec["decision"],decide(primary,gates,refused,len(D["cases"])*len(D["cutoffs"])),"decision")
    equal(rec["status"],"COMPLETE" if all(gates.values()) and not refused else "INVALID","completion")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir")
    ap.add_argument("--smoke", action="store_true", help="tiny unregistered software check")
    ap.add_argument("--worker", action="store_true", help=argparse.SUPPRESS)
    ap.add_argument("--verify", action="store_true", help="replay saved matrices without a new solve")
    args = ap.parse_args()
    D = SMOKE if args.smoke else DESIGN
    out = Path(args.out_dir).resolve()
    if args.verify:
        with threadpool_limits(limits=1):
            print(verify(out))
        return
    if not args.worker:
        initialize(out, D, args.smoke)
        env = dict(os.environ, OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1", MKL_NUM_THREADS="1")
        command = [sys.executable, "-u", str(Path(__file__).resolve()), str(out), "--worker"]
        if args.smoke:
            command.append("--smoke")
        with (out / "worker.log").open("w", encoding="utf-8") as f:
            process = subprocess.Popen(command, cwd=ROOT, env=env, stdout=f, stderr=subprocess.STDOUT)
            raise SystemExit(supervise(process,out))
    path = out / "receipt.json"
    rec = json.loads(path.read_text())
    if rec["status"] != "READY" or rec["design"] != D or rec["sources_sha256"] != source_hashes():
        raise ValueError("worker requires fresh matching source-pinned initialization")
    rec["status"] = "RUNNING"
    write(path, rec)
    event(out,"startup",source=rec["source_revision"],smoke=args.smoke)
    gates = {"D00": True, "D01": True, "D02": True, "D03": True, "D04": True, "D05": True}

    def measure(role, N, g2, alpha, reference=False, **kw):
        event(out,"solve",role=role,cutoff=N,g2=g2,**kw)
        start = time.monotonic()
        sol = (j0.solve if reference else dm.solve)(N,g2,alpha,**kw)
        filename = f"solve-{len(rec['solves']):03}.npz"
        arrays = dict(H=np.array(sol["matrix_H"]), X=np.array(sol["matrix_X"]), V=sol["vectors"],
                      E=sol["energies"][:6],R=sol["residuals"][:6])
        if "matrix_S" in sol:
            arrays["S"] = np.array(sol["matrix_S"])
        with (out / filename).open("wb") as f:
            np.savez_compressed(f,**arrays); f.flush(); os.fsync(f.fileno())
        rec["solves"].append(dict(role=role,cutoff=N,g2=g2,alpha=alpha,parameters=kw,
            file=filename,sha256=sha(out / filename),dps=sol["dps"],solution=slim(sol)))
        gates["D02"] &= sol["gram_digits_lost"] < sol["dps"]-25
        write(path,rec)
        event(out,"solve_complete",role=role,cutoff=N,g2=g2,seconds=time.monotonic()-start,
              energies=sol["energies"][:3],residuals=sol["residuals"][:3])
        return sol
    try:
        for g2, alpha in D["cases"]:
            case = {"g2": g2, "alpha": alpha}
            rec["cases"].append(case)
            sel = {}
            case["beta_trials"] = sel
            for beta in D["beta_grid"]:
                s = measure("beta_selection", D["beta_selection_cutoff"], g2, alpha, n_d=0, beta=beta, K=D["K"], dps=D["dps"])
                sel[str(beta)] = s["energies"][1]
            best = min(D["beta_grid"], key=lambda b: (sel[str(b)], b))
            edge = best in (D["beta_grid"][0], D["beta_grid"][-1])
            if "force_beta" in D:            # smoke only: exercise the pipeline without a selection
                best, edge = D["force_beta"], False
            case["beta_selection"] = {"second_ritz_value": sel, "beta": best, "edge": edge}
            write(path, rec)
            print(f"g2={g2}: beta selected", flush=True)
            if case["beta_selection"]["edge"]:
                case["refused"] = "BETA_AT_GRID_EDGE"
                continue
            beta = best
            case["points"] = []
            for N in D["cutoffs"]:
                base = measure("origin", N, g2, alpha, dps=D["dps"])
                nxt = measure("next_cutoff", N + 2, g2, alpha, dps=D["dps"])
                ref = measure("reference", N, g2, alpha, reference=True, dps=60, nlev=6)
                d00 = (max(abs(a - b) for a, b in zip(base["energies"], ref["energies"])) <= 1e-12
                       and max(abs(a - b) for a, b in zip(base["residuals"], ref["residuals"])) <= 1e-12)
                gates["D00"] &= d00
                point = {"cutoff": N, "origin": slim(base), "next_cutoff": slim(nxt), "D00": d00, "dome": {}}
                case["points"].append(point)
                for n_d in D["dome_sizes"]:
                    aug = measure("dome", N, g2, alpha, n_d=n_d, beta=beta, K=D["K"], dps=D["dps"])
                    d01 = all(a <= b + 1e-12 for a, b in zip(aug["energies"][:3], base["energies"][:3]))
                    d05 = all(-1e-12 <= f <= 1 + 1e-12 for f in aug["outside_origin_fraction"])
                    gates["D01"] &= d01
                    gates["D05"] &= d05
                    point["dome"][str(n_d)] = {"solution": slim(aug), "comparison": gains(base, aug, nxt),
                                               "D01": d01, "D05": d05}
                    write(path,rec)
                if N == D["cutoffs"][-1]:
                    point["precision_checks"], point["K_sensitivity"] = {}, {}
                    for name, low, nn in (("origin",base,N),("next_cutoff",nxt,N+2)):
                        hi = measure("precision_"+name, nn, g2, alpha, dps=D["check_dps"])
                        check = precision_check(low,hi)
                        point["precision_checks"][name] = check; gates["D03"] &= check["D03"]
                    for nd in repeated_dome_sizes(D):
                        augK = measure("K_sensitivity", N, g2, alpha, n_d=nd, beta=beta, K=D["K_sensitivity"], dps=D["dps"])
                        cK = gains(base, augK, nxt); c0 = point["dome"][str(nd)]["comparison"]
                        d04 = (cK["H1a"],cK["H1b"]) == (c0["H1a"],c0["H1b"])
                        gates["D04"] &= d04
                        point["K_sensitivity"][str(nd)] = dict(solution=slim(augK),comparison=cK,D04=d04)
                        hi = measure("precision_dome", N, g2, alpha, n_d=nd, beta=beta, K=D["K"], dps=D["check_dps"])
                        check = precision_check(point["dome"][str(nd)]["solution"],hi)
                        point["precision_checks"][str(nd)] = check; gates["D03"] &= check["D03"]
                        write(path,rec)
                write(path, rec)
                print(f"g2={g2} N={N} done", flush=True)
        primary = [p["dome"][str(D["primary_dome_size"])]["comparison"]
                   for c in rec["cases"] for p in c.get("points", [])]
        refused = [c for c in rec["cases"] if "refused" in c]
        decision = decide(primary,gates,refused,len(D["cases"])*len(D["cutoffs"]))
        rec["gates"] = gates
        rec["decision"] = decision
        rec["status"] = "COMPLETE" if all(gates.values()) and not refused else "INVALID"
    except BaseException as exc:
        rec["status"] = "FAILED"
        rec["error"] = "".join(traceback.format_exception(type(exc), exc, exc.__traceback__))
        write(path, rec)
        raise
    rec["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    write(path, rec)
    verify(out)
    print(rec["status"], rec.get("decision"), rec["gates"])
    sys.exit(0 if rec["status"] == "COMPLETE" else 1)


if __name__ == "__main__":
    with threadpool_limits(limits=1):
        main()
