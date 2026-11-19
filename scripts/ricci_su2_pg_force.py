"""S2a: the two-ring Petrov-Galerkin force for SU(2) on the Berger field.
Spec: docs/RICCI_SU2_PG_FORCE_SPEC.md. Research script, not an engine verb.

Contract unchanged from RF2C: frozen native inputs and outputs, native tangent
mass and volume, continuum target and B-norm error via
ricci_lattice_convergence.measures. Only the gradient handed to measures changes.

usage: python scripts/ricci_su2_pg_force.py LEVELS ORDER TRIAL [RECEIPT.json]
  LEVELS  e.g. 0,1,2,3      ORDER  4 | 6
  TRIAL   native | pg | pg_noregister | pg_onering | whitney_check
"""
import json
import os
import sys
import time
from pathlib import Path

import numpy as np

DATA = Path(os.environ.get("GIGI_RICCI_DATA_ROOT", Path(__file__).resolve().parents[1]))
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "e2e/probes"))
import ricci_pg_curvature as pg     # noqa: E402
pg.BASE = DATA / "output/ricci-shape-regular-20261117/berger"
pg.DIAG = DATA / "output/ricci-consistency-20261117/diagnostic"
import ricci_pg_tworing as tw      # noqa: E402
import ricci_su2_recovery as su    # noqa: E402
from ricci_consistency_math import compose, inverse, rotations, EDGES, FACES  # noqa: E402
from ricci_lattice_convergence import measures  # noqa: E402

BERGER = DATA / "output/ricci-shape-regular-20261117/berger"
EPS = 1e-7
ONE = np.array([1.0, 0, 0, 0])
TAU = [np.array([0.0, 1, 0, 0]), np.array([0.0, 0, 1, 0]), np.array([0.0, 0, 0, 1])]
# native face definitions: (a, b, c) edges, defect = u[a] u[b] u[c]^-1; face 123 carried by u[0]
FACE_EDGES = [(0, 3, 1), (0, 4, 2), (1, 5, 2), (3, 5, 4)]


def local_links(cell, links):
    u = np.asarray(links, float)[cell["edges"]].copy()
    rev = np.asarray(cell["reverse"], bool)
    u[rev, 1:] *= -1
    return u


def defects(u):
    D = []
    for f, (a, b, c) in enumerate(FACE_EDGES):
        h = compose(compose(u[a], u[b]), inverse(u[c]))
        if f == 3:
            h = compose(compose(u[0], h), inverse(u[0]))
        D.append(h - ONE)
    return np.array(D)                                   # 4 faces x 4 components


# each face defect as a product of (local edge, inverted?) factors, exactly as in defects():
# faces 0-2 are u_a u_b u_c^-1; face 3 (face 123) is u_0 (u_3 u_5 u_4^-1) u_0^-1
DEFECT_FACTORS = ([[(a, False), (b, False), (c, True)] for (a, b, c) in FACE_EDGES[:3]]
                  + [[(0, False), (FACE_EDGES[3][0], False), (FACE_EDGES[3][1], False),
                      (FACE_EDGES[3][2], True), (0, True)]])


def defect_derivatives(u):
    """dD[e, k, face, comp] for right-trivialised variations u_e -> u_e exp(t tau_k), exactly.

    Product rule: the derivative of u_e is u_e tau_k, and of u_e^-1 its conjugate (on unit
    quaternions the inverse is the conjugate, which is linear). No finite-difference step, so no
    rounding amplification; see docs/RICCI_SU2_EXACT_DEFECT_DERIVATIVE_SPEC.md."""
    u = np.asarray(u, float)
    out = np.zeros((6, 3, 4, 4))
    for f, factors in enumerate(DEFECT_FACTORS):
        plain = [inverse(u[e]) if inv else u[e] for e, inv in factors]
        for pos, (e, inv) in enumerate(factors):
            left = ONE
            for q in plain[:pos]:
                left = compose(left, q)
            right = ONE
            for q in plain[pos + 1:]:
                right = compose(right, q)
            for k in range(3):
                d = compose(u[e], TAU[k])
                if inv:
                    d = np.r_[d[0], -d[1:]]
                out[e, k, f] += compose(compose(left, d), right)
    return out


def defect_derivatives_fd(u):
    """The former central-difference version (step EPS), kept for comparison only."""
    out = np.zeros((6, 3, 4, 4))
    for e in range(6):
        for k in range(3):
            step = np.r_[np.cos(EPS), np.sin(EPS) * TAU[k][1:]]
            back = np.r_[np.cos(EPS), -np.sin(EPS) * TAU[k][1:]]
            up, dn = u.copy(), u.copy()
            up[e] = compose(u[e], step)
            dn[e] = compose(u[e], back)
            out[e, k] = (defects(up) - defects(dn)) / (2 * EPS)
    return out


def quadrature(points, metric, order):
    """cell_integrals' quadrature, returning per-point data (same formulas)."""
    import itertools
    points = np.asarray(points, float)[None]
    roots, weights = np.polynomial.legendre.leggauss(order)
    roots = (roots + 1) / 2
    weights = weights / 2
    idx = np.array(list(itertools.product(range(order), repeat=3)))
    uu, vv, ww = roots[idx].T
    local = np.column_stack([uu, (1 - uu) * vv, (1 - uu) * (1 - vv) * ww])
    lam = np.column_stack([1 - local.sum(axis=1), local])
    weight = np.prod(weights[idx], axis=1) * (1 - uu) ** 2 * (1 - vv)
    b = (points[:, 1:] - points[:, 0, None]).transpose(0, 2, 1)
    y = np.einsum('qa,cai->cqi', lam, points)
    norm = np.linalg.norm(y, axis=2)
    x = y / norm[..., None]
    dx = (b[:, None] - x[..., None] * np.einsum('cqi,cia->cqa', x, b)[:, :, None, :]) / norm[..., None, None]
    frame = np.empty(x.shape[:-1] + (4, 3))
    frame[..., 0, :] = -x[..., 1:]
    skew = np.zeros(x.shape[:-1] + (3, 3))
    a, bb, c = np.moveaxis(x[..., 1:], -1, 0)
    skew[..., 0, 1] = -c; skew[..., 0, 2] = bb; skew[..., 1, 0] = c
    skew[..., 1, 2] = -a; skew[..., 2, 0] = -bb; skew[..., 2, 1] = a
    frame[..., 1:, :] = x[..., 0, None, None] * np.eye(3) + skew
    maurer = np.einsum('cqia,cqib->cqab', frame, dx)
    g = np.einsum('cqia,i,cqib->cqab', maurer, np.asarray(metric, float), maurer)
    det = np.linalg.det(g)
    measure = weight[None, :] * np.sqrt(det)
    grads = np.vstack([-np.ones(3), np.eye(3)])
    two = np.array([2 * (lam[:, i, None] * np.cross(grads[j], grads[k])
                         - lam[:, j, None] * np.cross(grads[i], grads[k])
                         + lam[:, k, None] * np.cross(grads[i], grads[j]))
                    for i, j, k in FACES]).transpose(1, 0, 2)          # q x 4 x 3
    m2 = np.einsum('qea,cqab,qfb,cq->cef', two, g, two, measure / det)[0]
    return {"local": local, "g": g[0], "w": (measure / det)[0], "two": two, "m2": m2}


def run(level, order, trial, metric=(3.0, 1.0, 1.0)):
    payload = json.loads((BERGER / f"level{level}_berger_q{order}.input.json").read_text())
    raw = json.loads((BERGER / f"level{level}_berger_q{order}.output.json").read_text())
    links = np.asarray(raw["links"], float)
    cells = payload["cells"]
    faces = pg.face_index(cells) if trial.startswith("pg") or trial == "whitney_check" else None
    vid = su.vertex_ids(cells) if faces is not None else None
    zeros = np.zeros(len(links))
    grad = np.zeros((len(links), 3))
    energy = 0.0
    for ci, cell in enumerate(cells):
        u = local_links(cell, links)
        D = defects(u)
        quad = quadrature(cell["vertices"], metric, order)
        M = quad["m2"]
        energy += cell["weight"] * 2 * float(np.einsum('ij,ik,jk->', M, D, D))
        h = M @ D                                        # native trial, 4 faces x 4 comps
        if trial != "native":
            if trial == "whitney_check":
                # C3: the cell's own Whitney field fed through the pairing
                P = np.einsum('jA,qja->qAa', D[:, 1:], quad["two"])        # q x su2 x 3
            else:
                rings = 1 if trial == "pg_onering" else 2
                fit = tw.fit_cell_rings(cells, zeros, faces, ci, rings=rings)
                rec = su.recover_cell(cells, faces, vid, links, ci, rings=rings, fit=fit,
                                      register=(trial != "pg_noregister"), flux="defect")
                C = rec["coefficients"]                                    # su2 x 11
                P = np.stack([pg.proxy_ref(dict(fit, x=C[A]), quad["local"]) for A in range(3)], axis=1)
            hvec = np.einsum('qAa,qab,qib,q->iA', P, quad["g"], quad["two"], quad["w"])
            h = np.column_stack([h[:, 0], hvec])
        dD = defect_derivatives(u)
        a_loc = 4 * np.einsum('ekic,ic->ek', dD, h)                         # 6 x 3
        for e in range(6):
            g_id = cell["edges"][e]
            lift = -rotations(links[g_id]) if cell["reverse"][e] else np.eye(3)
            grad[g_id] += cell["weight"] * lift.T @ a_loc[e]
    native_grad = np.asarray(raw["gradient"], float)
    result = dict(raw, gradient=grad.tolist())
    m = measures(payload, list(metric), result)
    return {"level": level, "order": order, "trial": trial,
            "energy_replay_rel": abs(energy - raw["energy"]) / abs(raw["energy"]),
            "gradient_vs_native_rel": float(np.linalg.norm(grad - native_grad) / np.linalg.norm(native_grad)),
            "relative_error": m["relative_error"], "full_mass_error": m["full_mass_error"],
            "reference_mass_norm": m["reference_mass_norm"], "solver": m["diagnostics"],
            "_grad": grad}


def main():
    levels = [int(x) for x in sys.argv[1].split(",")]
    order, trial = int(sys.argv[2]), sys.argv[3]
    receipt = Path(sys.argv[4]) if len(sys.argv) > 4 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    rows = []
    for level in levels:
        t = time.time()
        r = run(level, order, trial)
        r.pop("_grad")
        r["seconds"] = round(time.time() - t, 1)
        rows.append(r)
        print(json.dumps({k: (round(v, 6) if isinstance(v, float) else v) for k, v in r.items() if k != "solver"}), flush=True)
    if receipt:
        receipt.write_text(json.dumps({"kind": "S2a research script (not an engine run)", "rows": rows}, indent=2) + "\n")


if __name__ == "__main__":
    main()
