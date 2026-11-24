"""Run the spin-0 invariant-sector solver over cutoffs and write a replayable receipt.

CPU only (mpmath). Memory is small: the basis has 23 functions at cutoff 10, 249 at 28, 632 at 40. Run time grows
roughly as n^3 at the chosen precision; on a laptop, cutoff 28 took about 140 s.

usage:
  python scripts/run_ym_helike_j0.py OUT_DIR [--cases 1:1.7 2:2.3] [--cutoffs 10 12 ... 40] [--dps 60] [--check-dps 100]

Writes OUT_DIR/receipt.json with status RUNNING first, updated after every case, and COMPLETE or FAILED at the end.
Each case records all reported levels, full-space residuals, the superpotential floor E0 > 9/2, variational
monotonicity across cutoffs, conditioning, the registered Temple quantities for the scalar (certified: false,
separation_verified: false) and the inherited residual gate. The top cutoff of every case is recomputed at
--check-dps and must agree.
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
import ym_helike_j0 as j  # noqa: E402

DEFAULT_CUTOFFS = (10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32, 36, 40)
NLEV = 6


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, obj):
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(obj, indent=1))
    os.replace(tmp, path)


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
    sources = {p: sha(HERE / p) for p in ("ym_helike_j0.py", "run_ym_helike_j0.py", "test_ym_helike_j0.py")}
    rec = {"status": "RUNNING", "label": "spin-0 invariant-sector solver; floating point, not certified",
           "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "args": vars(args),
           "sources_sha256": sources, "python": sys.version, "platform": platform.platform(),
           "mpmath_backend": j.mp.libmp.BACKEND, "cases": []}
    write(path, rec)
    try:
        for spec in args.cases:
            g2, alpha = (float(x) for x in spec.split(":"))
            case = {"g2": g2, "alpha": alpha, "cutoffs": []}
            rec["cases"].append(case)
            prev = None
            for N in args.cutoffs:
                t0 = time.time()
                s = j.solve(N, g2, alpha, dps=args.dps)
                o = j.ordering_inputs(s)
                row = {"cutoff": N, "dimension": s["dimension"], "seconds": round(time.time() - t0, 2),
                       "gram_digits_lost": s["gram_digits_lost"], "min_exterior_eig": s["min_exterior_eig"],
                       "energies": s["energies"][:NLEV], "residuals": s["residuals"][:NLEV], **o,
                       "superpotential_floor_ok": o["E0"] > 4.5}
                if prev is not None:
                    row["variational_monotone"] = all(a <= b + 1e-12 for a, b in zip(row["energies"][:3],
                                                                                         prev["energies"][:3]))
                    for key in ("E0", "E_s", "E_s_next", "gap_s"):
                        row[f"delta_{key}"] = row[key] - prev[key]
                case["cutoffs"].append(row)
                prev = row
                write(path, rec)
                print(f"g2={g2} alpha={alpha} N={N} n={s['dimension']} E0={o['E0']:.12f} E_s={o['E_s']:.12f} "
                      f"eps_s={o['eps_s']:.3e} eps_s'={o['eps_s_next']:.3e} tau={o['tau_beta']} "
                      f"gate={o['residual_gate_scalar']} {row['seconds']}s", flush=True)
            top = args.cutoffs[-1]
            hi = j.solve(top, g2, alpha, dps=args.check_dps)
            dE = max(abs(a - b) for a, b in zip(hi["energies"][:NLEV], prev["energies"]))
            dR = max(abs(a - b) / max(b, 1e-300) for a, b in zip(hi["residuals"][:NLEV], prev["residuals"]))
            case["precision_check"] = {"cutoff": top, "dps": args.check_dps, "max_abs_dE": dE, "max_rel_dR": dR,
                                       "pass": dE < 1e-11 and dR < 1e-6}
            write(path, rec)
        rec["gates"] = {
            "precision": all(c["precision_check"]["pass"] for c in rec["cases"]),
            "superpotential_floor": all(r["superpotential_floor_ok"] for c in rec["cases"] for r in c["cutoffs"]),
            "variational_monotone": all(r.get("variational_monotone", True) for c in rec["cases"]
                                        for r in c["cutoffs"]),
            "exterior_gram_psd": all(r["min_exterior_eig"] > -1e-12 for c in rec["cases"] for r in c["cutoffs"]),
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


if __name__ == "__main__":
    main()
