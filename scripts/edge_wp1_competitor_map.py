"""WP1: which charge block (j, q) sets the H + GG^* edge x*(t, y) of the twist's region (21 November 2026).

Region t in [2, 12], y in [0.25, 3]; sectors j <= 12 (EE.JMAX); operator H + GG^*; blocks from
scripts/ricci_su2_exact_edge.py (Blocks.block, Blocks.phi_A with arg tracking, edge()).

For every grid point:
  * x* = EE.edge(phi_A, y, t) on the default window [-3, 1] (81-point sign scan + brentq, xtol 1e-12);
  * all per-block values f_K(x*) = lambda_min(B_K(x*)) - lambda_j(x*) for every non-twist block K = (j, q);
    the competitor is the argmin (it is ~0 at x*); the runner-up gap is recorded to flag near-ties;
  * phi_A's arg just right of the edge (x* + 1e-6), as a second identification;
  * number of sign changes of phi_A on a 401-point scan of [-3, 1] (single crossing => half-line);
  * the q <-> -q mirror: lambda_min(B_{j,q}) vs lambda_min(B_{j,-q}) at x*.
Switch boundaries: per-block edge x_K(t, y) = root of f_K near x*; along every grid line (fixed y, varying t;
fixed t, varying y) where the competitor changes between neighbours, the boundary is the root of x_A - x_B (brentq).
Everything here is double precision (COMPUTED).
usage: OMP_NUM_THREADS=1 python scripts/edge_wp1_competitor_map.py research/edge-theorem-20261121/wp1_competitor_map.json
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.optimize import brentq

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE              # noqa: E402

TS = [float(v) for v in np.arange(2.0, 12.0 + 1e-9, 1.0)]            # 11 values
YS = [float(v) for v in np.arange(0.25, 3.0 + 1e-9, 0.25)]           # 12 values


def block_values(bl, xv, yv, tv):
    d = bl.data(xv, yv, tv); out = {}
    for j in range(EE.JMAX + 1):
        l = j / 2; lam = EE.lam_tw(xv, yv, tv, j)
        for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0):
            if abs(abs(q) - (l + 2)) < 1e-9:
                continue
            _, B = bl.block(d, j, q, tv)
            if B.size:
                out[(j, float(q))] = float(np.linalg.eigvalsh(B)[0] - lam)
    return out


def f_block(bl, K, xv, yv, tv):
    j, q = K
    _, B = bl.block(bl.data(xv, yv, tv), j, q, tv)
    return float(np.linalg.eigvalsh(B)[0] - EE.lam_tw(xv, yv, tv, j))


def block_edge(bl, K, yv, tv, x0, half=0.6):
    """Largest sign change (+ to -) of f_K in [x0 - half, x0 + half], refined by brentq; None if none."""
    xs = np.linspace(x0 - half, x0 + half, 121); v = [f_block(bl, K, xv, yv, tv) for xv in xs]
    k = None
    for i in range(len(xs) - 1):
        if v[i] >= 0 > v[i + 1]:
            k = i
    if k is None:
        return None
    return brentq(lambda xv: f_block(bl, K, xv, yv, tv), xs[k], xs[k + 1], xtol=1e-13)


def canon(K):
    return (K[0], abs(K[1]))


def main():
    t0 = time.time()
    bl = EE.Blocks()
    grid = []
    for tv in TS:
        for yv in YS:
            xe = EE.edge(bl.phi_A, yv, tv)
            row = {"t": tv, "y": yv, "edge_x": xe}
            xs = np.linspace(-3, 1, 401); ph = [bl.phi_A(xv, yv, tv) for xv in xs]
            row["phi_A_sign_changes_on_401pt_scan"] = int(sum(1 for i in range(400) if (ph[i] >= 0) != (ph[i + 1] >= 0)))
            row["phi_A_at_x=-3"] = float(ph[0])
            if xe is not None:
                vals = block_values(bl, xe, yv, tv)
                order = sorted(vals, key=lambda K: vals[K])
                K0 = order[0]
                # runner-up that is not the mirror of K0
                rest = [K for K in order[1:] if canon(K) != canon(K0)]
                row["competitor"] = list(K0)
                row["competitor_value_at_edge"] = vals[K0]
                mirror = (K0[0], -K0[1])
                row["mirror_value_at_edge"] = vals.get(mirror)
                row["runner_up"] = list(rest[0]); row["runner_up_gap"] = vals[rest[0]] - vals[K0]
                row["phi_A_arg_right_of_edge"] = list(bl.phi_A(xe + 1e-6, yv, tv, True)[1])
            grid.append(row)
            print(f"t={tv:5.2f} y={yv:4.2f} x*={xe} comp={row.get('competitor')} runner={row.get('runner_up')} "
                  f"gap={row.get('runner_up_gap')} sc={row['phi_A_sign_changes_on_401pt_scan']}  [{time.time() - t0:.0f}s]",
                  flush=True)
    Path(sys.argv[1]).parent.mkdir(parents=True, exist_ok=True)
    Path(sys.argv[1]).with_name(Path(sys.argv[1]).stem + "_grid_only.json").write_text(json.dumps(grid, indent=1) + "\n")
    # switch boundaries
    G = {(r["t"], r["y"]): r for r in grid}
    bnd = []

    def try_boundary(p, q_, axis):
        ra, rb = G[p], G[q_]
        if ra["edge_x"] is None or rb["edge_x"] is None:
            return
        A, B = tuple(ra["competitor"]), tuple(rb["competitor"])
        if canon(A) == canon(B):
            return

        def diff(u):
            # per-block edges near the linear interpolation of the two grid edges (cheap: one block each)
            lo_, hi_ = (p[0], q_[0]) if axis == "t" else (p[1], q_[1])
            wgt = (u - lo_) / (hi_ - lo_); xg = (1 - wgt) * ra["edge_x"] + wgt * rb["edge_x"]
            tv, yv = (u, p[1]) if axis == "t" else (p[0], u)
            xa = block_edge(bl, A, yv, tv, xg, half=0.25); xb = block_edge(bl, B, yv, tv, xg, half=0.25)
            return xa - xb
        lo, hi = (p[0], q_[0]) if axis == "t" else (p[1], q_[1])
        rec = {"axis": axis, "from": list(p), "to": list(q_), "blocks": [list(A), list(B)]}
        try:
            dlo, dhi = diff(lo), diff(hi)
            rec["diff_ends"] = [dlo, dhi]
            if dlo * dhi < 0:
                u = brentq(diff, lo, hi, xtol=1e-10)
                tv, yv = (u, p[1]) if axis == "t" else (p[0], u)
                xe = EE.edge(bl.phi_A, yv, tv)
                rec["switch_at"] = {"t": tv, "y": yv, "edge_x": xe,
                                    "x_A": block_edge(bl, A, yv, tv, xe, half=0.25),
                                    "x_B": block_edge(bl, B, yv, tv, xe, half=0.25)}
            else:
                rec["switch_at"] = None; rec["note"] = "no sign change of x_A - x_B between the grid points"
        except Exception as e:                                  # report plainly
            rec["error"] = repr(e)
        bnd.append(rec); print("boundary", rec, flush=True)

    for i, tv in enumerate(TS):
        for k, yv in enumerate(YS):
            if i + 1 < len(TS):
                try_boundary((tv, yv), (TS[i + 1], yv), "t")
            if k + 1 < len(YS):
                try_boundary((tv, yv), (tv, YS[k + 1]), "y")
    comps = sorted({tuple(canon(tuple(r["competitor"]))) for r in grid if r.get("competitor")})
    out = {"status": "COMPUTED (double precision); sectors j <= 12; operator H + GG^*",
           "grid_t": TS, "grid_y": YS, "grid": grid, "boundaries": bnd,
           "distinct_competitors_up_to_q_sign": [list(c) for c in comps],
           "competitors_with_sign": sorted({tuple(r["competitor"]) for r in grid if r.get("competitor")}),
           "runtime_s": time.time() - t0}
    Path(sys.argv[1]).parent.mkdir(parents=True, exist_ok=True)
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1, default=str) + "\n")
    print("distinct competitors:", comps)


if __name__ == "__main__":
    main()
