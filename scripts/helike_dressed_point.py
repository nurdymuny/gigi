"""The dressed chart at one point and rung (flux-method amendment §5.10): the self-consistency loop around the solver.

  solve in the current chart (helike cone-levels [--chart])  ->  vacuum mean and covariance, kinetic form at the mean
  ->  dressing step (helike_dressed_chart.dress)  ->  roles by overlap  ->  new chart (G0', M', w', roles)  ->  repeat
until the centre and the widths settle (1e-6), at most 6 iterations, else OPEN.
"""
import json
import math
from pathlib import Path
import subprocess
import sys

import numpy as np

import helike_dressed_chart as dc
import helike_m8_geometry as gm

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / ("helike-lab/target/release/helike" + (".exe" if sys.platform == "win32" else ""))
ROLE_NAMES = ("H", "C", "X", "S")


def bits(a):
    return np.ascontiguousarray(np.asarray(a, dtype="<f8")).view("<u8").ravel().tolist()


def write_chart(path, G0, M, w, roles):
    Path(path).write_text(json.dumps({"schema": "helike-lab/dressed-chart/v1", "G0": bits(G0), "M": bits(M),
                                      "w": bits(w), "roles": [int(r) for r in roles]}) + "\n")


def roles_dict(roles):
    """[h1, h2, c, x, s1, s2] -> {role: tuple of chart modes}."""
    r = list(roles)
    return {"H": (r[0], r[1]), "C": (r[2],), "X": (r[3],), "S": (r[4], r[5])}


def roles_list(d):
    return [d["H"][0], d["H"][1], d["C"][0], d["X"][0], d["S"][0], d["S"][1]]


def seam_order(mq):
    n = max(10, mq + 1)
    return n + n % 2 + 2                      # the fine order of the amendment-4 rule


def export(s, t, g2, mq, out, chart=None):
    cmd = [str(BIN), "--out", str(Path(out).with_suffix(".receipt.json")), "cone-levels", "--s", repr(s), "--t", repr(t),
           "--g2", repr(g2), "--mq", str(mq), "--levels-out", str(out)]
    if chart is not None:
        cmd += ["--chart", str(chart)]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode:
        raise RuntimeError(f"cone-levels refused (exit {r.returncode}): {r.stderr.strip() or r.stdout.strip()}")
    X = gm.load_levels(out)
    X["path"] = str(out)
    return X


ASYM_TOL = 1e-2             # §5.10 A2: gross bound at every rung (a geometric error is refused here or by the ladder rule)
ASYM_FINAL = 1e-3           # §5.10 A2: at the final rung, after a strict decrease along the ladder


def dress_step(X, order=None):
    """From one solved point in its frame: the dressed frame (G0', M', w', roles') and the step's record.
    §5.10 A1: the dressing is done in the Gram-linear normal coordinates y = M^{-1} (x - x0), where the charge rotation
    acts linearly, with the kinetic form A(x) written in y. The new frame is fed to the solver exactly like the bare one:
    G0' = the vacuum's mean Gram matrix, M' = M T (the dressed normal frame), w' (the dressed widths)."""
    import helike_condensate_pt as cp
    import helike_positive_cone as pc
    order = order or seam_order(X["mq"])
    P = gm.Point(X, order)
    p, _ = P.values([0])
    y = P.ycoords(P.core.z)
    ybar, Cy = dc.vacuum_moments(P.core.W, y, p[:, 0])
    rd = roles_dict(X["roles"])
    ybar, Cy, asym = dc.symmetrize(ybar, Cy, [rd["H"], rd["S"]], [rd["C"], rd["X"]])
    if asym > ASYM_TOL:
        raise ArithmeticError(f"vacuum moments break the charge rotation by {asym:.2e} > {ASYM_TOL} (OPEN)")
    xbar = P.x0 + X["M"] @ ybar
    Ainv_M = P.invM
    Ay = Ainv_M @ cp.A_at(xbar) @ Ainv_M.T                    # the kinetic form in y at the vacuum mean
    Ay = 0.5 * (Ay + Ay.T)
    yb, T, w = dc.dress(ybar, Cy, Ay)
    A0 = Ainv_M @ cp.A_at(P.x0) @ Ainv_M.T                    # the current frame's kinetic form at its centre
    roles = dc.assign_roles(T, 0.5 * (A0 + A0.T), rd, w)
    G0 = cp.mat_from_vec(xbar)
    pc.positive_factor(G0)                                     # refuses a mean Gram matrix off the cone
    M = X["M"] @ T
    chart = pc.ConeChart(G0, M)
    return dict(G0=G0, M=M, w=w, roles=roles_list(roles), u0=chart.u0, S=chart.S,
                record=dict(order=order, ybar=ybar.tolist(), widths=w.tolist(), roles=roles_list(roles),
                            asymmetry=asym))


def dressed_point(s, t, g2, mq, workdir):
    """Iterate to self-consistency. Returns (chart path, final levels export, history) or raises (OPEN)."""
    d = Path(workdir); d.mkdir(parents=True, exist_ok=True)
    X = export(s, t, g2, mq, d / "levels_bare.json")
    prev = (X["u0"], np.sort(X["w"]))
    history = [dict(iteration=0, chart="bare", levels=X["levels"][:4].tolist())]
    for it in range(1, dc.MAX_ITER + 1):
        step = dress_step(X)
        chart = d / f"chart_{it}.json"
        write_chart(chart, step["G0"], step["M"], step["w"], step["roles"])
        X = export(s, t, g2, mq, d / f"levels_{it}.json", chart)
        new = (step["u0"], np.sort(step["w"]))
        history.append(dict(iteration=it, chart=str(chart), levels=X["levels"][:4].tolist(), **step["record"],
                            centre_change=float(np.max(np.abs(new[0] - prev[0]))),
                            width_change=float(np.max(np.abs(new[1] - prev[1]) / np.abs(prev[1])))))
        if dc.settled(prev, new):
            return chart, X, history
        prev = new
    raise ArithmeticError(f"dressed chart did not settle in {dc.MAX_ITER} iterations (OPEN)")
