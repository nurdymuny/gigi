"""Section 1 of 'Yang-Mills Requirements as Geometric Objects' on the real cells of Reading the Curve.

The recovery of Reading the Curve (scripts/ricci_pg_curvature.py, scripts/ricci_pg_tworing.py) reads a closed linear
curvature P_K(x) = c + Bx, tr B = 0 (eleven coefficients), from face fluxes in a diameter-scaled chart of each cell K.
X_K maps the coefficients to the neighbourhood's face fluxes; R_K is the fit (own faces exact, neighbours by least
squares). Everything below is assembled with the paper's own functions on its frozen meshes (n = 1, 2, 3; one and two
rings); nothing in the recovery is reimplemented.

  T1a  exactness: R_K X_K = I_11 on every cell (Lemma "Exactness and accuracy").
  T1b  the ellipsoid: the image of the unit coefficient ball under X_K has the singular values of X_K as semiaxes.
       Reproduce the committed amplifications Lambda_K = ||R_K||_2 (docs/papers/two-ring-pg/data/conditioning.json)
       and the paper's statement that, for one ring, Lambda_K lies between 1.03 and 1.13 times 1/sigma_min(X_K).
       Report the weakest direction: which closed linear field the measurements barely see.
  T1c  metric fidelity (Section 11, test 1): rebuild the flux rows from the cell's triangles in physical units
       (chart coordinates times the diameter d). With the coefficient metric transported (c weighted 1, B weighted d),
       the semiaxes scale exactly by d^2. Gate: raw coefficient length (no transported metric) distorts the ellipsoid
       (its condition number changes).
  T1d  gauge fidelity (Section 11, test 2): the Poincare homology sphere has H^1 = 0, checked here from ranks: every
       edge datum with zero flux through every face is pure gauge. Such data must recover exactly zero, and adding it to
       the paper's data must leave every recovered coefficient unchanged. Gates: the edge data themselves change by
       O(1) (a method reading edges, not fluxes, would see the gauge); closure-violating fluxes are refused by the
       paper's own one-ring fit.

usage: python scripts/ym_geometry_rtc_cells.py OUT.json DATA_ROOT
       DATA_ROOT holds output/ricci-shape-regular-20261117 and output/ricci-consistency-20261117 (the frozen inputs of
       Reading the Curve; not in git, about 100 MB).
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.sparse import coo_matrix
from scipy.sparse.linalg import lsqr

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import _receipt_status  # noqa: E402
import ricci_pg_curvature as pg  # noqa: E402
import ricci_pg_tworing as tr  # noqa: E402

CONDITIONING = HERE.parent / "docs/papers/two-ring-pg/data/conditioning.json"


def star(cells, faces, ci, rings):
    """The chart triangles of K's neighbourhood, in the same order as fit_cell_rings uses them."""
    nb = tr.neighbourhood_rings(cells, faces, ci, rings)
    to_orth, _, _ = pg.chart(nb[ci])
    K = nb[ci]
    diam = max(np.linalg.norm(K[i] - K[j]) for i in range(4) for j in range(i + 1, 4))
    seen, tris, nown = set(), [], 0
    for nj in [ci] + [k for k in nb if k != ci]:
        O = to_orth(nb[nj])
        for f in pg.FACES:
            key = pg.face_key(cells[nj], f)
            if key in seen:
                continue
            seen.add(key)
            tris.append(O[list(f)])
            nown += nj == ci
    return tris, nown, diam


def edge_reader(cells, faces, ci, a):
    """A rival that reads edges, not fluxes: fit a linear connection A(z) = a0 + M z to the edge integrals of K's
    one-ring star (each chart segment's integral of a linear field is exact at its midpoint). Returns the size of its
    curl (the antisymmetric part of M) and of the fitted connection itself (a0, M). Reporting the connection as the
    physical object is the failure Section 11 names: a change of frame read as a physical change."""
    nb = tr.neighbourhood_rings(cells, faces, ci, 1)
    to_orth, _, _ = pg.chart(nb[ci])
    A_rows, b, seen = [], [], set()
    for nj in [ci] + [k for k in nb if k != ci]:
        O = to_orth(nb[nj])
        for f in pg.FACES:
            i, j, k = f
            z = {i: O[i], j: O[j], k: O[k]}
            (gij, sij), (gjk, sjk), (gik, nsik) = pg.face_edge_terms(cells[nj], f)
            for (p0, p1, g, s) in ((i, j, gij, sij), (j, k, gjk, sjk), (i, k, gik, -nsik)):
                if (g, p0 < p1) in seen:
                    continue
                seen.add((g, p0 < p1))
                dz, m = z[p1] - z[p0], 0.5 * (z[p0] + z[p1])
                A_rows.append(np.r_[dz, np.outer(dz, m).ravel()])
                b.append(s * a[g])
    sol = np.linalg.lstsq(np.array(A_rows), np.array(b), rcond=None)[0]
    M = sol[3:].reshape(3, 3)
    return float(np.max(np.abs(M - M.T))), float(np.max(np.abs(sol)))


def rows(tris, scale=1.0):
    return np.array([pg.flux_row(t * scale) for t in tris])


def level_tests(level, rings, committed):
    data = pg.load_cells(level)
    cells, a = data["cells"], data["a"]
    faces = pg.face_index(cells)
    zero = np.zeros(int(max(max(c["edges"]) for c in cells)) + 1)
    ex, amp_err, ratio, wB, metric_err, cond_raw_ratio = [], [], [], [], [], []
    for ci in range(len(cells)):
        fit = tr.fit_cell_rings(cells, zero, faces, ci, rings=rings)
        tris, nown, d = star(cells, faces, ci, rings)
        X = rows(tris)
        R = fit["map"]
        assert X.shape[0] == R.shape[1] == fit["faces"]
        ex.append(float(np.max(np.abs(R @ X - np.eye(11)))))
        U, s, Vt = np.linalg.svd(X, full_matrices=False)
        lam = float(np.linalg.norm(R, 2))
        amp_err.append(abs(lam / committed[ci] - 1))
        ratio.append(lam * s[-1])
        wB.append(float(np.sum(Vt[-1, 3:] ** 2)))            # share of the weakest direction in the B (gradient) part
        # metric fidelity: physical-unit rows; transported metric D = diag(1,1,1, d,...,d)
        Xd = rows(tris, d)
        D = np.r_[np.ones(3), np.full(8, d)]
        sd = np.linalg.svd(Xd / D[None, :], compute_uv=False)
        metric_err.append(float(np.max(np.abs(sd / (d ** 2 * s) - 1))))
        sraw = np.linalg.svd(Xd, compute_uv=False)
        cond_raw_ratio.append(float((sraw[0] / sraw[-1]) / (s[0] / s[-1])))
    out = dict(level=level, n=level + 1, rings=rings, cells=len(cells),
               exactness_max=max(ex), committed_amplification_max_rel_err=max(amp_err),
               amplification_over_inverse_sigma_min=[min(ratio), max(ratio)],
               weakest_direction_B_share=[min(wB), max(wB)],
               metric_transported_max_rel_err=max(metric_err),
               raw_condition_change=[min(cond_raw_ratio), max(cond_raw_ratio)])
    out["pass"] = bool(out["exactness_max"] < 1e-9 and out["committed_amplification_max_rel_err"] < 1e-9
                       and out["metric_transported_max_rel_err"] < 1e-9)
    if rings == 1:
        lo, hi = out["amplification_over_inverse_sigma_min"]
        out["paper_claim_1.03_to_1.13"] = {"range": [round(lo, 4), round(hi, 4)],
                                           "pass": bool(lo >= 1.03 - 5e-3 and hi <= 1.13 + 5e-3)}
    out["gate_raw_coefficient_length"] = {"condition_ratio_range": out["raw_condition_change"],
                                          "caught": bool(max(abs(np.log(cond_raw_ratio))) > 0.05)}
    return out


def gauge_tests(level, rings=2):
    data = pg.load_cells(level)
    cells, a = data["cells"], np.asarray(data["a"], float)
    faces = pg.face_index(cells)
    nE = len(a)
    # face-edge incidence D1 (one row per distinct face), from the paper's own face_edge_terms
    r, c, v, nF = [], [], [], 0
    for key, occ in faces.items():
        ci, f = occ[0]
        for g, s in pg.face_edge_terms(cells[ci], f):
            r.append(nF); c.append(g); v.append(s)
        nF += 1
    D1 = coo_matrix((v, (r, c)), shape=(nF, nE)).tocsr()
    nC = len(cells)
    # H^1 = 0: nullity(D1) = rank(D0) = V - 1, with V = E - F + C (Euler characteristic 0 of a closed 3-manifold)
    rank = np.linalg.matrix_rank(D1.toarray()) if nE <= 4000 else None
    V = nE - nF + nC
    h1_zero = rank is not None and (nE - rank) == V - 1
    # a pure-gauge edge datum: the component of a random vector in ker D1
    rng = np.random.default_rng(1123 + level)
    z = rng.standard_normal(nE)
    y = lsqr(D1.T, z, atol=1e-14, btol=1e-14, iter_lim=20000)[0]
    g = z - D1.T @ y
    g *= np.linalg.norm(a) / np.linalg.norm(g)                   # same size as the paper's data
    closure = float(np.max(np.abs(D1 @ g)) / np.max(np.abs(g)))
    rec_gauge, rec_shift, rival_conn, rival_curl = [], [], [], []
    for ci in range(nC):
        x0 = tr.fit_cell_rings(cells, a, faces, ci, rings=rings)["x"]
        xg = tr.fit_cell_rings(cells, g, faces, ci, rings=rings)["x"]
        x1 = tr.fit_cell_rings(cells, a + g, faces, ci, rings=rings)["x"]
        scale = max(1.0, float(np.max(np.abs(x0))))
        rec_gauge.append(float(np.max(np.abs(xg))) / scale)
        rec_shift.append(float(np.max(np.abs(x1 - x0))) / scale)
        if ci < 40:
            curl_g, conn_g = edge_reader(cells, faces, ci, g)
            curl_a, conn_a = edge_reader(cells, faces, ci, a)
            rival_conn.append(conn_g / conn_a)
            rival_curl.append(curl_g / max(curl_a, 1e-300))
    # gate: closure-violating fluxes must be refused by the paper's one-ring fit
    refused = 0
    for ci in range(min(nC, 20)):
        bump = {"n": 0}

        def bad_flux(tri, bump=bump):
            bump["n"] += 1
            return 1.0 if bump["n"] == 7 else 0.0                # one neighbour face's flux, nothing else
        try:
            pg.fit_cell(cells, bad_flux, faces, ci)
        except ValueError:
            refused += 1
    out = dict(level=level, n=level + 1, edges=nE, faces=nF, cells=nC, vertices=V,
               nullity_D1=None if rank is None else nE - rank, H1_zero=bool(h1_zero),
               gauge_datum_closure_residual=closure,
               recovered_from_gauge_max=max(rec_gauge), recovered_shift_max=max(rec_shift),
               gate_connection_read_as_physical={"cells": len(rival_conn),
                                                 "connection_change_from_gauge_over_data": [min(rival_conn),
                                                                                            max(rival_conn)],
                                                 "caught": bool(min(rival_conn) > 1e-2)},
               observation_rival_curl_also_gauge_blind={"curl_from_gauge_over_curl_from_data":
                                                        [min(rival_curl), max(rival_curl)]},
               gate_non_closed_refused={"tried": min(nC, 20), "refused": refused,
                                        "caught": refused == min(nC, 20)})
    out["pass"] = bool(h1_zero and closure < 1e-10 and max(rec_gauge) < 1e-9 and max(rec_shift) < 1e-9)
    return out


def main():
    out_path, root = Path(sys.argv[1]), Path(sys.argv[2])
    pg.BASE = root / "output/ricci-shape-regular-20261117/berger"
    pg.DIAG = root / "output/ricci-consistency-20261117/diagnostic"
    committed = json.loads(CONDITIONING.read_text())
    res = {"note": "docs/research/yang-mills-geometric-identities-2026-11-23.md, Section 1",
           "data_root_note": "frozen inputs of Reading the Curve (archived checkpoint)",
           "cells": [], "gauge": []}
    for level in (0, 1, 2):
        for rings in (1, 2):
            t = level_tests(level, rings, committed[f"L{level}_r{rings}"]["amplification"])
            res["cells"].append(t)
            print({k: t[k] for k in ("n", "rings", "cells", "exactness_max", "committed_amplification_max_rel_err",
                                     "amplification_over_inverse_sigma_min", "weakest_direction_B_share",
                                     "metric_transported_max_rel_err", "raw_condition_change", "pass")}, flush=True)
    for level in (0, 1):
        t = gauge_tests(level)
        res["gauge"].append(t)
        print({k: v for k, v in t.items() if not isinstance(v, dict)}, flush=True)
    res["all_pass"] = all(t["pass"] for t in res["cells"] + res["gauge"]) and all(
        t["paper_claim_1.03_to_1.13"]["pass"] for t in res["cells"] if t["rings"] == 1)
    gates = [t[k]["caught"] for t in res["cells"] + res["gauge"] for k in t if k.startswith("gate")]
    res["all_gates_caught"] = all(gates)
    res["ok"] = res["all_pass"] and res["all_gates_caught"]
    _receipt_status.publish(out_path, res)
    print("gates caught:", sum(gates), "/", len(gates), "| ok:", res["ok"])


if __name__ == "__main__":
    _receipt_status.run(sys.argv[1], main)
    _receipt_status.exit_on_failure(sys.argv[1])
