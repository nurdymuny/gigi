"""GIGI demos for the grid cells (docs/research/helike-bundle-framing-math-2026-11-27.md, Part II). Planted, exact answers.

(A) Layer 2, the double cover and the centre, on the lattice bundle `lattice_su2` built by gigi_lattice_gauge_demo.py:
    - adjoint (j = 1) loops from the same point-query holonomies: <chi_1>/3 = (I3/I1)^area  (Theorem 5);
    - Casimir scaling: sigma_1/sigma_1/2 from the exact formula at several beta, -> C2(1)/C2(1/2) = 8/3;
    - a centre vortex written into the bundle (one edge per plaquette in a column of plaquettes multiplied by -1):
      fundamental loops that encircle it flip sign, adjoint loops and non-encircling loops are unchanged (Theorem 6).
(B) Layer 5, topology on the Maurer-Cartan line: a GIGI bundle `mc_line` with base lambda and fibres V (unsourced and
    sourced model potential, computed from the model's V(q)), CS(lambda); point queries at the vacua, the sphaleron and
    the background (Theorems 8, 9).
usage: python scripts/gigi_grid_demos.py OUT.json [--port 3147]"""
import argparse
import json
import math
import time

import numpy as np
from scipy.special import iv

import gigi_lattice_gauge_demo as lg

# ------------------------------------------------------------------ (A) double cover and centre


def holonomy(db, cfg, x0, y0, R, T, L):
    h = np.array([1.0, 0, 0, 0])
    for i in range(R):
        h = lg.qmul(h, db.link(cfg, x0 + i, y0, 0, L))
    for j in range(T):
        h = lg.qmul(h, db.link(cfg, x0 + R, y0 + j, 1, L))
    for i in reversed(range(R)):
        h = lg.qmul(h, lg.qinv(db.link(cfg, x0 + i, y0 + T, 0, L)))
    for j in reversed(range(T)):
        h = lg.qmul(h, lg.qinv(db.link(cfg, x0, y0 + j, 1, L)))
    return h


def chi(j, h):
    """SU(2) characters from a unit quaternion: chi_1/2 = 2 a0, chi_1 = 4 a0^2 - 1."""
    a0 = h[0]
    return {0.5: 2 * a0, 1.0: 4 * a0 * a0 - 1}[j]


def demo_double_cover(db, L, beta, n_cfg):
    out = {"adjoint_area_law": {}, "casimir_scaling": {}, "vortex": {}}
    r3 = iv(3, beta) / iv(1, beta)
    for R, T in [(1, 1), (1, 2), (2, 2)]:
        vals = [chi(1.0, holonomy(db, c, x, y, R, T, L)) / 3 for c in range(n_cfg)
                for x in range(0, L - R, R) for y in range(0, L - T, T)]
        v = np.array(vals)
        out["adjoint_area_law"][f"{R}x{T}"] = {"measured": float(v.mean()), "stderr": float(v.std(ddof=1) / math.sqrt(len(v))),
                                              "exact": r3 ** (R * T), "z": float((v.mean() - r3 ** (R * T)) / (v.std(ddof=1) / math.sqrt(len(v))))}
    for b in (2.0, 4.0, 16.0, 64.0, 256.0):
        s_half, s_one = -math.log(iv(2, b) / iv(1, b)), -math.log(iv(3, b) / iv(1, b))
        out["casimir_scaling"][str(b)] = s_one / s_half
    # a centre vortex at plaquette (xv, yv): its Dirac string is a dual line running from that plaquette's centre up and
    # out through the top boundary. It crosses the horizontal edges (xv, y, 0) for y = yv+1 .. L-1, and each crossed
    # edge is multiplied by -1. Plaquette (xv, yv) then has one flipped edge (a centre flux -1); every plaquette above it
    # has two (no flux). Loops enclosing (xv, yv) cross the string once; loops above it cross twice; far loops never.
    cfg, xv, yv = 0, L // 2, L // 2 - 2
    probe = {"encloses the vortex": (xv - 1, yv - 1, 2, 2), "crosses the string twice": (xv - 1, yv + 1, 2, 2),
             "far away": (1, 1, 2, 2)}
    before = {k: holonomy(db, cfg, *p, L) for k, p in probe.items()}
    string = [(xv, y) for y in range(yv + 1, L)]
    for (x, y) in string:
        u = db.link(cfg, x, y, 0, L)
        db.post(f"/bundles/{lg.BUNDLE}/upsert", {"record": lg.record(cfg, x, y, 0, -u, L)})
    after = {k: holonomy(db, cfg, *p, L) for k, p in probe.items()}
    for k in probe:
        out["vortex"][k] = {"fund_before": chi(0.5, before[k]), "fund_after": chi(0.5, after[k]),
                            "adj_before": chi(1.0, before[k]), "adj_after": chi(1.0, after[k]),
                            "fund_flipped": bool(abs(chi(0.5, after[k]) + chi(0.5, before[k])) < 1e-12 and abs(chi(0.5, before[k])) > 1e-6),
                            "adj_unchanged": bool(abs(chi(1.0, after[k]) - chi(1.0, before[k])) < 1e-12)}
    for (x, y) in string:                                           # restore the section
        u = db.link(cfg, x, y, 0, L)
        db.post(f"/bundles/{lg.BUNDLE}/upsert", {"record": lg.record(cfg, x, y, 0, -u, L)})
    return out


# ------------------------------------------------------------------ (B) topology on the Maurer-Cartan line


def model_V(q, s, t, g, sourced):
    """The reduced model's potential V(q) = sum_k sum_c 1/2 (omega_k q_kc - g cof_kc)^2 - 1/2 a_k q_kc^2 (Rust Model)."""
    om = np.array([math.sqrt(t), 1 / math.sqrt(t), 1 / math.sqrt(t)])
    a = (1 - s) * (t - 2 * s); b = (1 - s) * ((1 - s) / t - s)
    src = np.array([a, b, b]) if sourced else np.zeros(3)
    cof = np.array([np.cross(q[1], q[2]), np.cross(q[2], q[0]), np.cross(q[0], q[1])])
    return float(sum(0.5 * np.sum((om[k] * q[k] - g * cof[k]) ** 2) - 0.5 * src[k] * np.sum(q[k] ** 2) for k in range(3)))


def demo_topology(db, s, t, g2):
    g = math.sqrt(g2)
    qmc = np.diag([1 / math.sqrt(t), 1.0, 1.0]) / g
    db.gql("COLLAPSE mc_line;")
    db.gql("BUNDLE mc_line BASE (id NUMERIC) FIBER (lam NUMERIC, V_free NUMERIC, V_sourced NUMERIC, CS NUMERIC);")
    lams = np.round(np.arange(-0.5, 1.5001, 0.01), 4)
    recs = []
    for i, lam in enumerate(lams):
        q = lam * qmc
        Dq = np.diag(1 / np.diag(qmc)) @ q
        cs = float(np.trace(Dq @ Dq.T) - 2 * np.linalg.det(Dq))      # = 3 lam^2 - 2 lam^3 on the line
        recs.append({"id": i, "lam": float(lam), "V_free": model_V(q, s, t, g, False), "V_sourced": model_V(q, s, t, g, True), "CS": cs})
    db.post("/bundles/mc_line/insert", {"records": recs})
    def at(lam):
        i = int(round((lam + 0.5) / 0.01))
        return db.gql(f"SECTION mc_line AT id={i};")["rows"][0]
    vfree = np.array([r["V_free"] for r in recs]); vsrc = np.array([r["V_sourced"] for r in recs])
    closed = (1 + 2 / t) * (lams - lams ** 2) ** 2 / (2 * g2)
    i_min = int(np.argmin(vsrc[lams > 0.6])) + int(np.sum(lams <= 0.6))
    return {"point": [s, t, g2], "max_abs_V_free_minus_closed_form": float(np.max(np.abs(vfree - closed))),
            "max_abs_CS_minus_3l2_2l3": float(np.max(np.abs(np.array([r["CS"] for r in recs]) - (3 * lams ** 2 - 2 * lams ** 3)))),
            "vacuum_0": at(0.0), "sphaleron": at(0.5), "vacuum_1": at(1.0), "background": at(round(s, 2)),
            "sourced_minimum_on_line_lambda": float(lams[i_min]), "expected_background_lambda": s,
            "sphaleron_closed_form": (1 + 2 / t) / (32 * g2)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--port", type=int, default=3147)
    ap.add_argument("--L", type=int, default=10)
    ap.add_argument("--beta", type=float, default=4.0)
    ap.add_argument("--configs", type=int, default=24)
    a = ap.parse_args()
    db = lg.Gigi(a.port)
    t0 = time.time()
    rep = {"double_cover": demo_double_cover(db, a.L, a.beta, a.configs), "topology": demo_topology(db, 1.15, 3.0, 0.004)}
    rep["gigi_point_queries"] = db.queries
    rep["seconds"] = time.time() - t0
    open(a.out, "w").write(json.dumps(rep, indent=1, default=float) + "\n")
    print(json.dumps(rep, indent=1, default=float))


if __name__ == "__main__":
    main()
