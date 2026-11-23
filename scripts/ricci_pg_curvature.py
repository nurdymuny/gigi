"""Petrov-Galerkin curvature test, Abelian control. Spec: docs/RICCI_PG_CURVATURE_SPEC.md

Reconstructed curvature trial, ORIGINAL Whitney test functions. Static only.
The Whitney trial runs through the same quadrature and assembly path as the
reconstruction, with a different trial basis, so it is a real check of this
module against the native stiffness rather than a shortcut to it.

Commands:
  python scripts/ricci_pg_curvature.py s12 LEVEL ORDER [--trial whitney] [--no-transport]
  python scripts/ricci_pg_curvature.py s3 LEVEL
"""
from collections import defaultdict
from pathlib import Path
import argparse
import hashlib
import json
import sys
import time

import numpy as np
from scipy.sparse import coo_matrix

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "e2e/probes"))
from ricci_consistency_math import EDGES, FACES, INCIDENCE   # noqa: E402

BASE = ROOT / "output/ricci-shape-regular-20261117/berger"
DIAG = ROOT / "output/ricci-consistency-20261117/diagnostic"
OUT = ROOT / "output/ricci-pg-curvature-20261117"
RCOND, COND_CAP, COINCIDE = 1e-12, 1e12, 1e-12
GRADS = np.vstack([-np.ones(3), np.eye(3)])
VREF = np.vstack([np.zeros(3), np.eye(3)])


# ── data plumbing ────────────────────────────────────────────────────────────

def load_cells(level):
    payload = json.loads((BASE / f"level{level}_berger_q6.input.json").read_text())
    with np.load(DIAG / f"level{level}_q6.arrays.npz", allow_pickle=False) as z:
        a = z["abelian_edge_integrals"].copy()
    return {"cells": payload["cells"], "a": a, "payload": payload}


def load_level(level, order):
    """Cells, data, and the native mass/stiffness from the frozen assembler."""
    from ricci_consistency_diagnostic import assemble
    base = load_cells(level)
    key = f"level{level}_berger_q{order}"
    payload = json.loads((BASE / f"{key}.input.json").read_text())
    raw = json.loads((BASE / f"{key}.output.json").read_text())
    _, mass, stiffness, volume, _ = assemble(payload, raw, order)
    return dict(base, mass=mass, stiffness=stiffness, volume=volume, order=order)


def face_key(cell, face):
    i, j, k = face
    return frozenset(cell["edges"][EDGES.index(e)] for e in ((i, j), (i, k), (j, k)))


def face_index(cells):
    index = defaultdict(list)
    for ci, c in enumerate(cells):
        for face in FACES:
            index[face_key(c, face)].append((ci, face))
    bad = [k for k, v in index.items() if len(v) != 2]
    if bad:
        raise ValueError(f"closed mesh requires every face in exactly two cells; {len(bad)} are not")
    return index


def signed(cell, a):
    """Integral from local vertex i to j along local edge (i, j), i < j."""
    return np.asarray(a)[cell["edges"]] * np.where(cell["reverse"], -1., 1.)


def face_edge_terms(cell, face):
    """Global edges and coefficients whose sum is the flux, boundary i->j->k."""
    i, j, k = face
    s = np.where(cell["reverse"], -1., 1.)
    e = lambda p, q: EDGES.index((p, q))   # noqa: E731
    return [(cell["edges"][e(i, j)], s[e(i, j)]), (cell["edges"][e(j, k)], s[e(j, k)]),
            (cell["edges"][e(i, k)], -s[e(i, k)])]


def data_flux(cell, a, face):
    return float(sum(c * np.asarray(a)[g] for g, c in face_edge_terms(cell, face)))


# ── sheets and neighbourhoods ────────────────────────────────────────────────

def qmul(p, q):
    return np.r_[p[0] * q[0] - p[1:] @ q[1:], p[0] * q[1:] + q[0] * p[1:] + np.cross(p[1:], q[1:])]


def qconj(p):
    return np.r_[p[0], -p[1:]]


def corners(cell, face):
    """Corner of a face -> local vertex, identified by its two global face edges."""
    out = {}
    for v in face:
        others = [w for w in face if w != v]
        out[frozenset(cell["edges"][EDGES.index(tuple(sorted((v, w))))] for w in others)] = v
    return out


def neighbourhood(cells, faces, ci, transport=True, include_neighbours=True):
    """K and its face-neighbours, all on K's sheet. Refuses if shared vertices
    do not coincide after transport (identity when transport is False)."""
    K = np.array(cells[ci]["vertices"], float)
    out = {ci: K}
    if not include_neighbours:
        return out
    for face in FACES:
        for nj, fn in faces[face_key(cells[ci], face)]:
            if nj == ci:
                continue
            ck, cn = corners(cells[ci], face), corners(cells[nj], fn)
            V = np.array(cells[nj]["vertices"], float)
            if transport:
                k0 = next(iter(ck))
                h = qmul(K[ck[k0]], qconj(V[cn[k0]]))
                V = np.array([qmul(h, v) for v in V])
            gap = max(float(np.max(abs(K[ck[k]] - V[cn[k]]))) for k in ck)
            if gap > COINCIDE:
                raise ValueError(f"shared vertices of cell {ci} and neighbour {nj} "
                                 f"do not coincide (gap {gap:.3e})")
            if nj in out and np.max(abs(out[nj] - V)) > COINCIDE:
                raise ValueError(f"neighbour {nj} reached on two different sheets")
            out[nj] = V
    return out


# ── chart and closed linear fit ──────────────────────────────────────────────

def chart(K):
    p0, b = K[0], (K[1:] - K[0]).T
    n = np.linalg.svd(b.T)[2][-1]
    c = float(n @ p0)
    if c < 0:
        n, c = -n, -c
    Q, _ = np.linalg.qr(b)
    cen = K.mean(axis=0)
    cen = cen * (c / (cen @ n))
    diam = max(np.linalg.norm(K[i] - K[j]) for i in range(4) for j in range(i + 1, 4))
    J = (Q.T @ b) / diam
    t0 = ((p0 - cen) @ Q) / diam

    def to_orth(V):
        den = V @ n
        if np.any(den <= 0):
            raise ValueError("neighbourhood vertex outside the projective chart")
        return ((V * (c / den)[:, None] - cen) @ Q) / diam
    return to_orth, J, t0


def _basis():
    basis = [(np.eye(3)[i], np.zeros((3, 3))) for i in range(3)]
    for i in range(3):
        for j in range(3):
            if i != j:
                B = np.zeros((3, 3)); B[i, j] = 1; basis.append((np.zeros(3), B))
    for d in ([1., -1, 0], [0, 1., -1]):
        basis.append((np.zeros(3), np.diag(d)))
    return basis


BASIS = _basis()
BC = np.array([c for c, _ in BASIS])          # 11 x 3
BB = np.array([B for _, B in BASIS])          # 11 x 3 x 3


def flux_row(tri):
    z1, z2, z3 = tri
    area = np.cross(z2 - z1, z3 - z1) / 2
    cen = (z1 + z2 + z3) / 3
    return (BC + BB @ cen) @ area


def fit_cell(cells, a, faces, ci, transport=True, include_neighbours=True):
    """Closed linear curvature for cell ci. `a` is edge data, or a callable
    giving the flux through an oriented chart triangle (planted tests).
    Returns coefficients and the linear map from face fluxes to them."""
    nb = neighbourhood(cells, faces, ci, transport, include_neighbours)
    to_orth, J, t0 = chart(nb[ci])
    seen, own, rest = set(), [], []
    for nj in [ci] + [k for k in nb if k != ci]:
        O = to_orth(nb[nj])
        for f in FACES:
            key = face_key(cells[nj], f)
            if key in seen:
                continue
            seen.add(key)
            tri = O[list(f)]
            d = a(tri) if callable(a) else data_flux(cells[nj], a, f)
            (own if nj == ci else rest).append((flux_row(tri), d, face_edge_terms(cells[nj], f)))
    rows = np.array([r for r, _, _ in own + rest])
    s = np.linalg.svd(rows, compute_uv=False)
    rank = int((s > RCOND * s[0]).sum())
    if rank < 11:
        raise ValueError(f"closed linear flux map has rank {rank} < 11 for cell {ci}")
    cond = float(s[0] / s[-1])
    if cond > COND_CAP:
        raise ValueError(f"condition {cond:.3e} exceeds cap for cell {ci}")
    # Exactly determined: 16 star faces, 5 closure relations, 11 unknowns.
    # The closed linear field interpolating every star flux, or a refusal.
    fmap = np.linalg.pinv(rows, rcond=RCOND)                      # 11 x nfaces
    d = np.array([dd for _, dd, _ in own + rest])
    x = fmap @ d
    residual = float(np.max(abs(rows @ x - d)))
    if residual > 1e-12 * max(1., float(np.max(abs(d)))):
        raise ValueError(f"star fluxes of cell {ci} cannot be interpolated by a closed "
                         f"linear field (residual {residual:.3e})")
    return {"x": x, "c": x @ BC, "B": _B(x), "J": J, "t0": t0, "map": fmap,
            "terms": [t for _, _, t in own + rest], "rank": rank, "cond": cond,
            "residual": residual}


def _B(x):
    return np.einsum("k,kij->ij", x, BB)


def proxy_ref(fit, xref):
    xref = np.atleast_2d(xref)
    P = fit["x"] @ BC + (xref @ fit["J"].T + fit["t0"]) @ _B(fit["x"]).T
    return np.linalg.det(fit["J"]) * np.linalg.solve(fit["J"], P.T).T


def whitney_two(lam):
    """Whitney face functions at barycentric points lam (Q,4) -> (Q,4,3)."""
    return np.array([2 * (lam[:, i, None] * np.cross(GRADS[j], GRADS[k])
                          - lam[:, j, None] * np.cross(GRADS[i], GRADS[k])
                          + lam[:, k, None] * np.cross(GRADS[i], GRADS[j]))
                     for i, j, k in FACES]).transpose(1, 0, 2)


def barycentric(xref):
    xref = np.atleast_2d(xref)
    return np.column_stack([1 - xref.sum(axis=1), xref])


def reference_flux(proxy, face):
    i, j, k = face
    area = np.cross(VREF[j] - VREF[i], VREF[k] - VREF[i]) / 2
    cen = (VREF[i] + VREF[j] + VREF[k]) / 3
    if isinstance(proxy, tuple) and proxy[0] == "whitney":
        P = whitney_two(barycentric(cen))[0, proxy[1]]
    else:
        P = proxy_ref(proxy, cen)[0]
    return float(P @ area)


# ── geometry on the sphere, matching cell_integrals exactly ──────────────────

def frame_quantities(points, lam):
    """maurer (C,Q,3,3), metric g, det for round metric at barycentric points."""
    points = np.asarray(points, float)
    b = (points[:, 1:] - points[:, 0, None]).transpose(0, 2, 1)
    y = np.einsum("qa,cai->cqi", lam, points)
    norm = np.linalg.norm(y, axis=2)
    x = y / norm[..., None]
    dx = (b[:, None] - x[..., None] * np.einsum("cqi,cia->cqa", x, b)[:, :, None, :]) / norm[..., None, None]
    frame = np.empty(x.shape[:-1] + (4, 3))
    frame[..., 0, :] = -x[..., 1:]
    skew = np.zeros(x.shape[:-1] + (3, 3))
    a_, bb, c_ = np.moveaxis(x[..., 1:], -1, 0)
    skew[..., 0, 1] = -c_; skew[..., 0, 2] = bb; skew[..., 1, 0] = c_
    skew[..., 1, 2] = -a_; skew[..., 2, 0] = -bb; skew[..., 2, 1] = a_
    frame[..., 1:, :] = x[..., 0, None, None] * np.eye(3) + skew
    maurer = np.einsum("cqia,cqib->cqab", frame, dx)
    g = np.einsum("cqia,cqib->cqab", maurer, maurer)
    return maurer, g, np.linalg.det(g)


def quadrature(points, order):
    roots, weights = np.polynomial.legendre.leggauss(order)
    roots = (roots + 1) / 2; weights = weights / 2
    idx = np.array(np.meshgrid(*[np.arange(order)] * 3, indexing="ij")).reshape(3, -1).T
    u, v, w = roots[idx].T
    local = np.column_stack([u, (1 - u) * v, (1 - u) * (1 - v) * w])
    lam = barycentric(local)
    weight = np.prod(weights[idx], axis=1) * (1 - u) ** 2 * (1 - v)
    maurer, g, det = frame_quantities(points, lam)
    if np.any(det <= 0):
        raise ValueError("positive metric required")
    return {"xref": local, "lam": lam, "weight": weight, "maurer": maurer, "g": g,
            "det": det, "measure": weight[None] * np.sqrt(det), "two": whitney_two(lam)}


def exact_proxy_from_maurer(maurer):
    return -2 * np.cross(maurer[..., 1, :], maurer[..., 2, :])


def exact_proxy(points, xref):
    maurer, _, _ = frame_quantities(np.asarray(points, float)[None], barycentric(xref))
    return exact_proxy_from_maurer(maurer)[0]


def exact_face_flux(points, face, n=12):
    i, j, k = face
    r, w = np.polynomial.legendre.leggauss(n)
    r = (r + 1) / 2; w = w / 2
    U, Vv = np.meshgrid(r, r, indexing="ij"); W = np.outer(w, w)
    s, t = U.ravel(), ((1 - U) * Vv).ravel()
    jac = (W * (1 - U)).ravel()
    e1, e2 = VREF[j] - VREF[i], VREF[k] - VREF[i]
    xr = VREF[i] + s[:, None] * e1 + t[:, None] * e2
    return float(jac @ (exact_proxy(points, xr) @ np.cross(e1, e2)))


# ── assembly ─────────────────────────────────────────────────────────────────

def energy(data, a, order, chunk=400):
    """E = one half, sum over K, of weight times the integral of P_K-transpose
    g P_K over root det g. Computed through fit_cell, proxy_ref and quadrature
    directly, never through the assembled operator, so T11 compares two paths."""
    cells = data["cells"]
    faces = face_index(cells)
    total = 0.
    for start in range(0, len(cells), chunk):
        idx = range(start, min(start + chunk, len(cells)))
        q = quadrature(np.array([cells[ci]["vertices"] for ci in idx], float), order)
        wq = q["measure"] / q["det"]
        for li, ci in enumerate(idx):
            P = proxy_ref(fit_cell(cells, a, faces, ci), q["xref"])
            total += 0.5 * float(cells[ci]["weight"]) * float(
                np.einsum("qa,qab,qb,q->", P, q["g"][li], P, wq[li]))
    return total


def build(level, order, trial="reconstruction", transport=True, data=None, chunk=400,
          force="pg"):
    """Assemble the operator explicitly. force='pg' tests the trial against the
    original Whitney face functions; force='variational' is the gradient of the
    energy of the trial curvature. Returns force on the data, the BW05 error as
    in scalar_control, and the S1 curvature error."""
    from ricci_consistency_math import solve_rhs
    if force not in ("pg", "variational"):
        raise ValueError(f"unknown force {force!r}")
    L = data or load_level(level, order)
    cells, a, mass, volume = L["cells"], L["a"], L["mass"], L["volume"]
    faces = face_index(cells)
    n_edges = len(a)
    rows, cols, vals = [], [], []
    num = den = 0.
    conds = []
    for start in range(0, len(cells), chunk):
        idx = range(start, min(start + chunk, len(cells)))
        pts = np.array([cells[ci]["vertices"] for ci in idx], float)
        q = quadrature(pts, order)
        wq = q["measure"] / q["det"]                         # (C,Q)
        Pex = exact_proxy_from_maurer(q["maurer"])           # (C,Q,3)
        for local_i, ci in enumerate(idx):
            cell = cells[ci]
            s = np.where(cell["reverse"], -1., 1.)
            weight = float(cell["weight"])
            g, w = q["g"][local_i], wq[local_i]
            if trial == "whitney":
                trial_basis = q["two"]                       # (Q,4,3)
                tmap = INCIDENCE * s[None, :]                # face fluxes from K's own 6 edges
                tcols = list(cell["edges"])
                coeff = tmap @ np.asarray(a)[tcols]
            else:
                fit = fit_cell(cells, a, faces, ci, transport=transport)
                conds.append(fit["cond"])
                J, t0 = fit["J"], fit["t0"]
                xo = q["xref"] @ J.T + t0                     # (Q,3)
                Porth = BC[None] + np.einsum("kij,qj->qki", BB, xo)   # (Q,11,3)
                trial_basis = np.linalg.det(J) * np.einsum("ij,qkj->qki", np.linalg.inv(J), Porth)
                tcols = sorted({e for terms in fit["terms"] for e, _ in terms})
                col_of = {e: p for p, e in enumerate(tcols)}
                D = np.zeros((len(fit["terms"]), len(tcols)))
                for fi, terms in enumerate(fit["terms"]):
                    for e, cf in terms:
                        D[fi, col_of[e]] += cf
                tmap = fit["map"] @ D                        # 11 x ncols
                coeff = fit["x"]
            if force == "variational":
                # Gram of the trial basis: gradient of the trial-curvature energy.
                G = np.einsum("qka,qab,qlb,q->kl", trial_basis, g, trial_basis, w)
                emap = weight * (tmap.T @ G @ tmap)                           # ncols x ncols
                rows.append(np.repeat(tcols, len(tcols)))
                cols.append(np.tile(tcols, len(tcols)))
            else:
                W = np.einsum("qfa,qab,qkb,q->fk", q["two"], g, trial_basis, w)   # 4 x ntrial
                emap = weight * (s[:, None] * (INCIDENCE.T @ W @ tmap))          # 6 x ncols
                rows.append(np.repeat(cell["edges"], len(tcols)))
                cols.append(np.tile(tcols, 6))
            vals.append(emap.ravel())
            P = np.einsum("qkb,k->qb", trial_basis, coeff)
            diff = P - Pex[local_i]
            num += weight * float(np.einsum("qa,qab,qb,q->", diff, g, diff, w))
            den += weight * float(np.einsum("qa,qab,qb,q->", Pex[local_i], g, Pex[local_i], w))
    op = coo_matrix((np.concatenate(vals), (np.concatenate(rows), np.concatenate(cols))),
                    shape=(n_edges, n_edges)).tocsr()
    force = op @ a
    velocity = solve_rhs(mass, -force)
    err = velocity + 4 * a
    norm = lambda v: float(np.sqrt(max(0., v @ (mass @ v)) / volume))   # noqa: E731
    return {"operator": op, "force": force, "velocity": velocity,
            "full_mass_error": norm(err), "reference_mass_norm": norm(-4 * a),
            "relative_error": norm(err) / norm(-4 * a),
            "curvature_relative_error": float(np.sqrt(num / den)),
            "max_condition": max(conds) if conds else None, "edges": n_edges}


# ── commands ─────────────────────────────────────────────────────────────────

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def record(name, payload):
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / name
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    payload["source_sha256"] = {"scripts/ricci_pg_curvature.py": sha(__file__)}
    path.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"wrote {path.relative_to(ROOT)}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["s12", "s3"])
    ap.add_argument("level", type=int)
    ap.add_argument("order", type=int, nargs="?", default=6)
    ap.add_argument("--trial", default="reconstruction", choices=["reconstruction", "whitney"])
    ap.add_argument("--no-transport", action="store_true")
    ap.add_argument("--force", default="pg", choices=["pg", "variational"])
    args = ap.parse_args()
    t = time.time()
    fsuffix = "" if args.force == "pg" else "_variational"
    if args.cmd == "s12":
        r = build(args.level, args.order, args.trial, not args.no_transport, force=args.force)
        out = {k: r[k] for k in ("full_mass_error", "reference_mass_norm", "relative_error",
                                 "curvature_relative_error", "max_condition", "edges")}
        out.update(level=args.level, order=args.order, trial=args.trial, force=args.force,
                   transport=not args.no_transport, seconds=round(time.time() - t, 2))
        print(json.dumps(out, indent=2))
        tag = (f"s12_L{args.level}_q{args.order}_{args.trial}" + fsuffix
               + ("_notransport" if args.no_transport else ""))
        record(tag + ".json", out)
    else:
        from scipy.linalg import eig, eigh
        L = load_level(args.level, 6)
        pg = build(args.level, 6, "reconstruction", data=L, force=args.force)["operator"].toarray()
        M = L["mass"].toarray(); Kw = L["stiffness"].toarray()
        lw = eigh(Kw, M, eigvals_only=True)          # Whitney is symmetric
        gauge = int((abs(lw) < 1).sum())
        lp = eig(pg, M, right=False)
        small = abs(lp) < 1
        phys = lp[~small]
        out = {"level": args.level, "edges": len(M), "gauge_dimension_whitney": gauge,
               "small_eigenvalues_pg": int(small.sum()),
               "largest_small_eigenvalue_pg": float(abs(lp[small]).max()) if small.any() else None,
               "min_real_part_physical": float(phys.real.min()),
               "max_abs_imag_physical": float(abs(phys.imag).max()),
               "min_real_part_whitney_physical": float(lw[abs(lw) >= 1].min()),
               "stable": bool(small.sum() == gauge and phys.real.min() > 0),
               "seconds": round(time.time() - t, 2)}
        out["force"] = args.force
        print(json.dumps(out, indent=2))
        record(f"s3_L{args.level}{fsuffix}.json", out)


if __name__ == "__main__":
    main()
