"""Basis convergence of the positive-cone controls, Mq 6 -> 8, at the unchanged 0.003 gap requirement.

usage: python scripts/run_helike_cone_convergence.py OUT_DIR [--verify]

Same controls as the positive-cone results (s = 1.15, t = 3, g^2 in {0.004, 0.002}); only the basis grows.
Mq = 8 is 3,003 trial functions with a 5,005-function derivative shell: dense moment matrices of about
200 MB each. Run it on the large machine, not a laptop.

Decision (frozen before the run): CONVERGED if both identified gaps (Helike and competitor) change by at
most 0.003 from Mq = 6 to Mq = 8 at both couplings, and every identification weight is at least 0.95;
NOT_CONVERGED if a change exceeds 0.003; UNRESOLVED if identification or numerics refuse. The Mq 4 -> 6
failure (0.0034559546) stays on record whatever this returns. This is a statement about the interior trial
family; it is not a boundary or spectral certificate (see the quasimode certification spec).
"""
import argparse, hashlib, json, math, platform, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import helike_positive_cone as pc  # noqa: E402

S, T, G2S, LADDER = 1.15, 3.0, (0.004, 0.002), (6, 8)
GAP_TOL, WEIGHT_MIN = 0.003, 0.95
SOURCES = ["scripts/helike_positive_cone.py", "scripts/run_helike_cone_convergence.py"]


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def decide(rows):
    """rows: {g2: {Mq: result}} -> (decision, per-coupling changes)."""
    changes, decision = {}, "CONVERGED"
    for g2, by in rows.items():
        a, b = by.get(LADDER[0]), by.get(LADDER[1])
        if not a or not b or "error" in a or "error" in b:
            return "UNRESOLVED", changes
        if min(r[ch]["weight"] for r in (a, b) for ch in ("helike", "competitor")) < WEIGHT_MIN:
            return "UNRESOLVED", changes
        d = {ch: abs(b[ch]["gap"] - a[ch]["gap"]) for ch in ("helike", "competitor")}
        changes[g2] = d
        if max(d.values()) > GAP_TOL:
            decision = "NOT_CONVERGED"
    return decision, changes


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir")
    ap.add_argument("--verify", action="store_true")
    a = ap.parse_args()
    out = Path(a.out_dir)
    path = out / "receipt.json"
    if a.verify:
        rec = json.loads(path.read_text())
        dec, ch = decide({float(k): {int(m): r for m, r in v.items()} for k, v in rec["rows"].items()})
        ok = dec == rec["decision"]
        print("VERIFY", "OK" if ok else "MISMATCH", dec)
        return 0 if ok else 1
    if out.exists() and any(out.iterdir()):
        sys.exit(f"refusing to write into non-empty {out}")
    out.mkdir(parents=True, exist_ok=True)
    rec = {"status": "RUNNING", "s": S, "t": T, "g2": G2S, "ladder": LADDER, "gap_tol": GAP_TOL,
           "weight_min": WEIGHT_MIN, "sources_sha256": {s: sha(ROOT / s) for s in SOURCES},
           "python": sys.version, "platform": platform.platform(),
           "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "rows": {}}
    try:
        rec["revision"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        rec["source_dirty"] = bool(subprocess.check_output(["git", "status", "--porcelain", "--", *SOURCES],
                                                           cwd=ROOT, text=True).strip())
    except Exception as exc:  # receipts still record that provenance is missing
        rec["revision_error"] = repr(exc)
    rows = {}
    for g2 in G2S:
        rows[g2] = {}
        for Mq in LADDER:
            t0 = time.time()
            try:
                R = pc.ConeRitz(S, T, math.sqrt(g2), Mq=Mq, max_Mq=max(LADDER))
                r = R.helike_and_competitor()
                r.update(dimension=len(R.basis), asymmetry=R.asym)
            except (ArithmeticError, ValueError, MemoryError) as exc:
                r = {"error": repr(exc)}
            r["seconds"] = round(time.time() - t0, 1)
            rows[g2][Mq] = r
            rec["rows"] = {str(k): {str(m): v for m, v in by.items()} for k, by in rows.items()}
            path.write_text(json.dumps(rec, indent=1))
            print(f"g2={g2} Mq={Mq}: " + (r.get("error") or
                  f"helike {r['helike']['gap']:.10f} competitor {r['competitor']['gap']:.10f}") + f" ({r['seconds']}s)",
                  flush=True)
    rec["decision"], ch = decide(rows)
    rec["changes"] = {str(k): v for k, v in ch.items()}
    rec["status"] = "COMPLETE"
    rec["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    path.write_text(json.dumps(rec, indent=1))
    print(rec["decision"], json.dumps(rec["changes"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
