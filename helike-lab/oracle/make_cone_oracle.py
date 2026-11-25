"""Python oracle for helike-quantum parity tests (milestone M3).

Writes helike-lab/oracle/oracle_cone.json from scripts/helike_positive_cone.py (ConeRitz) and its dependencies, so
the Rust tests compare against numbers the Python code produced, never against Rust's own output.

Contents, for the two weak-coupling controls (s = 1.15, t = 3, g^2 = 0.004, 0.002):
  background   G0, the normal frame M, frequencies w, chart u0 and S, energy shift, stationarity residual;
  chart        Gram matrix, cometric and log measure of the production chart at random z;
  dense        H at Mq = 2 and 4 (g^2 = 0.004 only), T at Mq = 2, entrywise;
  spectra      the lowest 12 Ritz values at Mq = 4;
  results      helike_and_competitor() at Mq = 4 and 6 (gaps, weights, level indices, E0);
  amplitudes   reference_amplitudes at Mq = 2 for two fixed vectors, quadrature order 3.

usage (from the repository root): python helike-lab/oracle/make_cone_oracle.py
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import helike_positive_cone as pc  # noqa: E402
import helike_berger_model as bm  # noqa: E402

S_, T_ = 1.15, 3.0


def main():
    rng = np.random.default_rng(20261124)
    out = {"seed": 20261124, "s": S_, "t": T_, "controls": []}
    for g2 in (0.004, 0.002):
        g = float(np.sqrt(g2))
        t0 = time.time()
        L2 = pc.ConeRitz(S_, T_, g, Mq=2)
        ch = L2.chart
        row = {"g2": g2, "g": g,
               "background": {"G0": ch.G0.ravel().tolist(), "M": ch.M.ravel().tolist(), "w": L2.w.tolist(),
                              "u0": ch.u0.tolist(), "S": ch.S.ravel().tolist(), "energy_shift": L2.energy_shift,
                              "helike_w": float(np.sqrt(bm.mc_helike_freq2(S_, T_)))},
               "chart": []}
        for z in rng.normal(scale=0.5, size=(6, 6)):
            row["chart"].append({"z": z.tolist(), "gram": ch.gram(z).ravel().tolist(),
                                 "metric": ch.metric(z).ravel().tolist(), "log_measure": ch.log_measure(z),
                                 "v_sourced": float(bm.V_sourced(ch.factor(z), g, T_, *bm.mc_sources(S_, T_)))})
        row["dense"] = {}
        if g2 == 0.004:
            L4 = pc.ConeRitz(S_, T_, g, Mq=4)
            row["dense"] = {"H2": L2.H.ravel().tolist(), "T2": L2.T.ravel().tolist(), "n2": len(L2.basis),
                            "H4": L4.H.ravel().tolist(), "n4": len(L4.basis),
                            "basis4": [list(map(int, b)) for b in L4.basis]}
            vecs = np.linalg.qr(np.random.default_rng(21).normal(size=(len(L2.basis), 2)))[0]
            row["amplitudes"] = {"vectors": vecs.ravel().tolist(), "order": 3,
                                 "want": L2.reference_amplitudes(vecs, 3).ravel().tolist()}
        row["results"] = {}
        for Mq in (4, 6):
            L = pc.ConeRitz(S_, T_, g, Mq=Mq)
            r = L.helike_and_competitor()
            r["dimension"] = len(L.basis)
            r["spectrum12"] = np.linalg.eigvalsh(L.H)[:12].tolist()
            row["results"][str(Mq)] = r
        row["seconds"] = round(time.time() - t0, 1)
        out["controls"].append(row)
        print(f"g2={g2}: {row['seconds']} s", flush=True)
    path = Path(__file__).with_name("oracle_cone.json")
    path.write_text(json.dumps(out))
    print("wrote", path, path.stat().st_size, "bytes")


if __name__ == "__main__":
    main()
