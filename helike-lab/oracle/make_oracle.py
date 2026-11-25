"""Python oracle for helike-lab parity tests (milestone M0).

Writes helike-lab/oracle/oracle_m0.json from the reference scripts, so the Rust tests compare against numbers the
Python code produced, never against Rust's own output.

Contents:
  su2_points   H and G entrywise at random members of the circle-symmetric family (hessian_and_gauge, general builder),
               plus curl and the flux matrix Phi from helike_curl_spectrum.operators;
  su3_points   SU(3) family 2b (t on lambda_8), A = theta/2: H and G entrywise in the sectors (1,0) and (0,1), and
               spectra (full and gauge slice) in (1,1);
  phi_points   phi_A (charge blocks, ricci_su2_exact_edge.Blocks) and phi_S (gauge slice) with their arguments;
  edges        the Table 1 edges x*_A / x*_S recomputed by ricci_su2_exact_edge.edge at three (t, y) cells.

usage (from the repository root): python helike-lab/oracle/make_oracle.py
"""
import json
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
_argv, sys.argv = sys.argv, sys.argv[:1]
import ricci_su3_test2_family as T2  # noqa: E402
import helike_curl_spectrum as hc  # noqa: E402
import ricci_su2_exact_edge as EE  # noqa: E402
sys.argv = _argv


def cm(a):
    a = np.asarray(a, complex)
    return {"rows": a.shape[0], "cols": a.shape[1], "re": a.real.ravel().tolist(), "im": a.imag.ravel().tolist()}


def main():
    rng = np.random.default_rng(20261124)
    S, C2, eps = T2.su2_data()
    out = {"seed": 20261124, "su2_points": [], "su3_points": [], "phi_points": [], "edges": []}
    for _ in range(8):
        x, y, t, j = float(rng.uniform(-3, 1)), float(rng.uniform(0.1, 3)), float(rng.uniform(0.5, 10)), int(rng.integers(0, 5))
        M = [x * np.eye(3)[0], y * np.eye(3)[1], y * np.eye(3)[2]]
        H, G = T2.hessian_and_gauge(C2, [t, 1, 1], eps, M, S.spin_rep(j))
        curl, H2, Phi, _, _ = hc.operators(x, y, t, j)
        out["su2_points"].append(dict(x=x, y=y, t=t, j=j, H=cm(H), G=cm(G), curl=cm(curl), Phi=cm(Phi),
                                      H_curl_minus_builder=float(np.abs(H2 - H).max()),
                                      spectrum=np.linalg.eigvalsh(H).tolist(),
                                      slice=T2.slice_spectrum(H, G).tolist()))
    X, C3 = T2.su3_data()
    for t in (0.7, 3.0):
        g = T2.su3_metric("2b", t)
        M = T2.su3_background(g)
        pt = dict(t=t, family="2b", s=0.5, sectors={})
        for (p, q) in [(1, 0), (0, 1), (1, 1)]:
            rho = T2.su3_irrep(p, q, X)
            H, G = T2.hessian_and_gauge(C3, g, C3, M, rho)
            rec = dict(spectrum=np.linalg.eigvalsh(H).tolist(), slice=T2.slice_spectrum(H, G).tolist())
            if (p, q) != (1, 1):
                rec.update(H=cm(H), G=cm(G))
            pt["sectors"][f"{p},{q}"] = rec
        out["su3_points"].append(pt)
    B = EE.Blocks()
    for (x, y, t) in [(-0.5, 1.5, 3.0), (-0.2, 0.5, 4.0), (0.1, 2.0, 6.0), (-1.0, np.sqrt(3), 3.0), (-2.5, 0.8, 8.0)]:
        va, (ja, qa) = B.phi_A(x, y, t, with_arg=True)
        vs, js = EE.phi_S(x, y, t, with_arg=True)
        out["phi_points"].append(dict(x=x, y=float(y), t=t, jmax=EE.JMAX, phi_A=float(va), arg_A=[int(ja), float(qa)],
                                      phi_S=float(vs), arg_S=int(js)))
    for (t, y) in [(3.0, 1.5), (4.0, 0.5), (3.0, float(np.sqrt(3)))]:
        xa = EE.edge(B.phi_A, y, t)
        xs = EE.edge(EE.phi_S, y, t)
        out["edges"].append(dict(t=t, y=y, x_star_A=float(xa), x_star_S=float(xs), lo=-3.0, hi=1.0, n=81, jmax=EE.JMAX))
    p = Path(__file__).resolve().parent / "oracle_m0.json"
    p.write_text(json.dumps(out))
    print("wrote", p, p.stat().st_size, "bytes")


if __name__ == "__main__":
    main()
