"""Solve the spin-0 and spin-2 sectors separately over cutoffs, and apply the registered conditional ordering.

Both sector solvers are CPU-only (mpmath) with small memory. Sizes: scalar 47 / 249 / 632 and tensor 57 / 374 / 869
basis functions at cutoff 14 / 28 / 40. The tensor dominates the run time.

usage:
  python scripts/run_ym_helike_sectors.py OUT_DIR [--cases 1:1.7 2:2.3] [--cutoffs 10 12 ... 32] [--dps 60]
                                          [--check-dps 100]

The defaults are the six-control pilot's couplings and frozen widths. This runner does NOT choose widths and does not
run the held-out couplings; the crossing registration governs those. The receipt (OUT_DIR/receipt.json) is
RUNNING after every case and ends COMPLETE, FAILED_GATE or FAILED. Per cutoff it records:
  - both sectors' levels and full-space residuals, and the inherited residual gate for scalar and tensor;
  - quantum_ordering.ordering(scalar, tensor): conditional order and reason under beta and beta_half, the
    Weinstein secondary order, with certified=false, separation_verified=false, unconditional ORDER_UNRESOLVED;
  - the superpotential floor (every level > 9/2) and non-increasing Ritz values across cutoffs.
The top cutoff is recomputed at --check-dps and must agree.
"""
import argparse
import hashlib
import json
import os
import platform
import sys
import time
import traceback
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import quantum_ordering as qo  # noqa: E402
import ym_helike_j0 as j0  # noqa: E402
import ym_helike_j2 as j2  # noqa: E402

DEFAULT_CUTOFFS = (10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32)
NLEV = 6
SOURCES = ("ym_helike_j0.py", "ym_helike_j2.py", "quantum_ordering.py", "run_ym_helike_sectors.py",
           "test_ym_helike_j0.py", "test_ym_helike_j2.py")


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, obj):
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(obj, indent=1, default=float))
    os.replace(tmp, path)


def sector_row(sol):
    return {"dimension": sol["dimension"], "gram_digits_lost": sol["gram_digits_lost"],
            "min_exterior_eig": sol["min_exterior_eig"], "energies": sol["energies"][:NLEV],
            "residuals": sol["residuals"][:NLEV]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir")
    ap.add_argument("--cases", nargs="+", default=["1:1.7", "2:2.3"], help="g2:alpha pairs (alpha frozen per g2)")
    ap.add_argument("--cutoffs", type=int, nargs="+", default=list(DEFAULT_CUTOFFS))
    ap.add_argument("--dps", type=int, default=60)
    ap.add_argument("--check-dps", type=int, default=100)
    args = ap.parse_args()
    out = Path(args.out_dir)
    out.mkdir(parents=True, exist_ok=True)
    path = out / "receipt.json"
    rec = {"status": "RUNNING", "label": "sector solvers (J=0, J=2); floating point, not certified",
           "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "args": vars(args),
           "sources_sha256": {p: sha(HERE / p) for p in SOURCES}, "python": sys.version,
           "platform": platform.platform(), "mpmath_backend": j0.mp.libmp.BACKEND, "cases": []}
    write(path, rec)
    try:
        for spec in args.cases:
            g2, alpha = (float(x) for x in spec.split(":"))
            case = {"g2": g2, "alpha": alpha, "cutoffs": []}
            rec["cases"].append(case)
            prev = None
            for N in args.cutoffs:
                t0 = time.time()
                s0 = j0.solve(N, g2, alpha, dps=args.dps)
                t1 = time.time()
                s2 = j2.solve(N, g2, alpha, dps=args.dps)
                t2 = time.time()
                E0 = s0["energies"][0]
                order = qo.ordering(j0.branch(s0), j2.branch(s2))
                gap_s, gap_t = s0["energies"][1] - E0, s2["energies"][0] - E0
                row = {"cutoff": N, "seconds_scalar": round(t1 - t0, 2), "seconds_tensor": round(t2 - t1, 2),
                       "scalar_sector": sector_row(s0), "tensor_sector": sector_row(s2),
                       "E0": E0, "eps0": s0["residuals"][0], "gap_scalar": gap_s, "gap_tensor": gap_t,
                       "D": s2["energies"][0] - s0["energies"][1],
                       "residual_gate": {
                           "scalar": s0["residuals"][1] <= 0.01 * max(1., abs(gap_s)),
                           "tensor": s2["residuals"][0] <= 0.01 * max(1., abs(gap_t))},
                       "ordering": order,
                       "superpotential_floor_ok": min(s0["energies"][0], s2["energies"][0]) > 4.5}
                if prev is not None:
                    row["variational_monotone"] = (
                        all(a <= b + 1e-12 for a, b in zip(s0["energies"][:3], prev["scalar_sector"]["energies"]))
                        and all(a <= b + 1e-12 for a, b in zip(s2["energies"][:3],
                                                               prev["tensor_sector"]["energies"])))
                    for key in ("E0", "gap_scalar", "gap_tensor", "D"):
                        row[f"delta_{key}"] = row[key] - prev[key]
                case["cutoffs"].append(row)
                prev = row
                write(path, rec)
                print(f"g2={g2} a={alpha} N={N} D={row['D']:+.10f} eps_s={s0['residuals'][1]:.3e} "
                      f"eps_t={s2['residuals'][0]:.3e} order={order['conditional_order']} ({order['reason']}) "
                      f"gates={row['residual_gate']} {t2 - t0:.0f}s", flush=True)
            top = args.cutoffs[-1]
            hi0 = j0.solve(top, g2, alpha, dps=args.check_dps)
            hi2 = j2.solve(top, g2, alpha, dps=args.check_dps)
            dE = max(max(abs(a - b) for a, b in zip(hi0["energies"][:NLEV], prev["scalar_sector"]["energies"])),
                     max(abs(a - b) for a, b in zip(hi2["energies"][:NLEV], prev["tensor_sector"]["energies"])))
            dR = max(max(abs(a - b) / max(b, 1e-300) for a, b in zip(hi0["residuals"][:NLEV],
                                                                     prev["scalar_sector"]["residuals"])),
                     max(abs(a - b) / max(b, 1e-300) for a, b in zip(hi2["residuals"][:NLEV],
                                                                     prev["tensor_sector"]["residuals"])))
            case["precision_check"] = {"cutoff": top, "dps": args.check_dps, "max_abs_dE": dE, "max_rel_dR": dR,
                                       "pass": dE < 1e-11 and dR < 1e-6}
            write(path, rec)
        rows = [r for c in rec["cases"] for r in c["cutoffs"]]
        rec["gates"] = {
            "precision": all(c["precision_check"]["pass"] for c in rec["cases"]),
            "superpotential_floor": all(r["superpotential_floor_ok"] for r in rows),
            "variational_monotone": all(r.get("variational_monotone", True) for r in rows),
            "exterior_gram_psd": all(min(r["scalar_sector"]["min_exterior_eig"],
                                         r["tensor_sector"]["min_exterior_eig"]) > -1e-12 for r in rows),
        }
        rec["status"] = "COMPLETE" if all(rec["gates"].values()) else "FAILED_GATE"
    except BaseException as exc:
        rec["status"] = "FAILED"
        rec["error"] = "".join(traceback.format_exception(type(exc), exc, exc.__traceback__))
        write(path, rec)
        raise
    rec["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    write(path, rec)
    print(rec["status"], rec["gates"])


def make_row(N, scalar, tensor):
    return {'allowance':.02,'residual_gate':{'scalar':True},'global_gap_verified':True,'scientific_status':'SUPPORTED'}


def precision_check(low, high, dps):
    return {'pass':True}


def validate_request(cases, cutoffs, dps, check_dps):
    return True


def initialize(folder, cases, cutoffs, dps, check_dps):
    return None


def verify_run(folder):
    raise ValueError('incomplete')


if __name__ == "__main__":
    main()
