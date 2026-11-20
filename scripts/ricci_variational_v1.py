"""V1: variational force from a constrained two-ring recovery (Abelian control).
Spec: docs/VARIATIONAL_RECOVERY_V1_SPEC.md. Research script, not an engine verb.

usage: python scripts/ricci_variational_v1.py LEVEL [RECEIPT.json]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import eigh

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "e2e/probes")]
import os                                   # noqa: E402
import ricci_pg_curvature as pg            # noqa: E402
_DATA = Path(os.environ.get("GIGI_RICCI_DATA_ROOT", HERE.parent))
pg.BASE = _DATA / "output/ricci-shape-regular-20261117/berger"
pg.DIAG = _DATA / "output/ricci-consistency-20261117/diagnostic"
import ricci_pg_tworing as tw              # noqa: E402
from ricci_consistency_math import solve_rhs  # noqa: E402

ORDER = 6
PAIRS = [(0, 1, 2, 3), (0, 2, 3, 1), (0, 3, 1, 2)]   # complementary index pairs (ij, kl)


# ---------------------------------------------------------------- per-cell geometry
def frame(verts, lam):
    """S^3 points X (Q,4) and tangent vectors dX/dr (Q,4,3) in reference coordinates."""
    b = (verts[1:] - verts[0]).T                      # 4 x 3
    y = lam @ verts
    nrm = np.linalg.norm(y, axis=1)
    X = y / nrm[:, None]
    dx = (b[None] - X[:, :, None] * np.einsum("qi,ia->qa", X, b)[:, None, :]) / nrm[:, None, None]
    return X, dx


def generator_proxies(X, dx):
    """Proxies (Q,3) of smooth closed 2-forms of R^4 on S^3: (left-inv 3, right-inv 3, linear)."""
    cr = lambda i, j: np.cross(dx[:, i, :], dx[:, j, :])            # noqa: E731
    plus = [cr(i, j) + cr(k, l) for i, j, k, l in PAIRS]
    minus = [cr(i, j) - cr(k, l) for i, j, k, l in PAIRS]
    lin = []
    for k in range(4):
        for l in range(k, 4):
            for m in range(4):
                if m in (k, l) and k == l:
                    continue
                lin.append(X[:, l, None] * cr(k, m) + X[:, k, None] * cr(l, m))
    return np.stack(plus, 1), np.stack(minus, 1), np.stack(lin, 1)   # (Q,3,3), (Q,3,3), (Q,nl,3)


class Cell:
    def __init__(self, cells, faces, ci):
        c = cells[ci]
        self.ci, self.weight = ci, float(c["weight"])
        self.verts = np.array(c["vertices"], float)
        nb = tw.neighbourhood_rings(cells, faces, ci, 2)
        self.nb = nb
        self.cells_ref = cells
        to_orth, J, t0 = pg.chart(nb[ci])
        seen, own, rest = set(), [], []
        for nj in [ci] + [k for k in nb if k != ci]:
            O = to_orth(nb[nj])
            for f in pg.FACES:
                key = pg.face_key(cells[nj], f)
                if key in seen:
                    continue
                seen.add(key)
                (own if nj == ci else rest).append((pg.flux_row(O[list(f)]), pg.face_edge_terms(cells[nj], f)))
        A = np.array([r for r, _ in own]); R = np.array([r for r, _ in rest])
        X = np.vstack([A, R])
        fit = tw.fit_cell_rings(cells, np.zeros(1 + max(max(cc["edges"]) for cc in cells)), faces, ci, rings=2)
        self.M = fit["map"]                                             # 11 x nf
        assert np.abs(self.M @ X - np.eye(11)).max() < 1e-9
        terms = fit["terms"]
        self.tcols = sorted({e for t in terms for e, _ in t})
        col = {e: p for p, e in enumerate(self.tcols)}
        self.D = np.zeros((len(terms), len(self.tcols)))
        for fi, t in enumerate(terms):
            for e, cf in t:
                self.D[fi, col[e]] += cf
        self.col = col
        # parameterisation of the change: Z (own fluxes kept), N (closed linear fields kept)
        ua, sa, vta = np.linalg.svd(A)
        ra = int((sa > pg.RCOND * sa[0]).sum())
        self.Z = vta[ra:].T                                             # 11 x (11-ra)
        ux, sx, vtx = np.linalg.svd(X.T)                                # X^T: 11 x nf
        rx = int((sx > pg.RCOND * sx[0]).sum())
        self.N = vtx[rx:].T                                             # nf x (nf-11)
        self.ND = self.N.T @ self.D                                     # (nf-11) x ncols
        self.T = self.M @ self.D                                        # 11 x ncols
        # quadrature, trial basis, Gram
        q = pg.quadrature(self.verts[None], ORDER)
        self.lam, self.g, self.w = q["lam"], q["g"][0], q["measure"][0] / q["det"][0]
        xo = q["xref"] @ J.T + t0
        Porth = pg.BC[None] + np.einsum("kij,qj->qki", pg.BB, xo)
        self.trial = np.linalg.det(J) * np.einsum("ij,qkj->qki", np.linalg.inv(J), Porth)   # Q,11,3
        self.two = q["two"]                                             # Q,4,3
        self.G = np.einsum("qka,qab,qlb,q->kl", self.trial, self.g, self.trial, self.w)
        s = np.where(c["reverse"], -1., 1.)
        self.TW = pg.INCIDENCE * s[None, :]                             # 4 x 6 (own edges)
        self.wedges = list(c["edges"])
        self.Pex = pg.exact_proxy_from_maurer(q["maurer"])[0]           # control proxy, Q x 3

    def pair(self, P):
        """P: (Q,m,3) field proxies -> trial pairings (11,m) and Whitney pairings (4,m)."""
        v = np.einsum("qka,qab,qmb,q->km", self.trial, self.g, P, self.w)
        u = np.einsum("qfa,qab,qmb,q->fm", self.two, self.g, P, self.w)
        return v, u


def lift_to(cell, e, P, Q):
    """Left multiplication h carrying the copy of edge e in cell's own neighbourhood onto the
    reference endpoints (P, Q); returns h and the number of distinct copies of e seen."""
    cells = cell.cells_ref
    hs = []
    for nj, V in cell.nb.items():
        if e not in cells[nj]["edges"]:
            continue
        i, j = pg.EDGES[cells[nj]["edges"].index(e)]
        p, q = V[i], V[j]
        for A, B in ((P, Q), (Q, P)):
            h = pg.qmul(A, pg.qconj(p))
            if np.abs(pg.qmul(h, q) - B).max() < 1e-9:
                if not any(np.abs(h - h0).max() < 1e-9 for h0 in hs):
                    hs.append(h)
                break
    if not hs:
        raise RuntimeError(f"no copy of edge {e} in the neighbourhood of cell {cell.ci} maps onto the reference")
    return hs[0], len(hs)


def generators_on(cell, verts):
    X, dx = frame(verts, cell.lam)
    plus, minus, lin = generator_proxies(X, dx)
    return plus, minus, lin


GENERATORS = "all"     # V1: 3 constants + 20 linear; V1c: "constants"


def run(level, objective="change"):
    t0 = time.time()
    L = pg.load_level(level, ORDER)
    cells, a, mass, vol = L["cells"], np.asarray(L["a"]), L["mass"], L["volume"]
    faces = pg.face_index(cells)
    C = [Cell(cells, faces, ci) for ci in range(len(cells))]
    print("cells ready", round(time.time() - t0, 1), "s", flush=True)
    ne = len(a)
    norm = lambda v: float(np.sqrt(max(0., v @ (mass @ v)) / vol))   # noqa: E731

    def assemble(Ts):
        K = np.zeros((ne, ne))
        for c, T in zip(C, Ts):
            K[np.ix_(c.tcols, c.tcols)] += c.weight * T.T @ c.G @ T
        return K

    def rel(K):
        return norm(solve_rhs(mass, -(K @ a)) + 4 * a) / norm(-4 * a)

    out = {"level": level, "objective": objective, "generators": GENERATORS}
    # C-repro
    K0 = assemble([c.T for c in C])
    out["C_repro_error"] = rel(K0)
    ref = pg.build(level, ORDER, "reconstruction", data=L, force="variational")["relative_error"] if False else None
    # C-span on every cell (stored sheet): control vs held-out / enforced spans
    worst_in, best_out = 0.0, np.inf
    left_is = None
    for c in C:
        plus, minus, lin = generators_on(c, c.verts)
        ip = lambda P, Qm: np.einsum("qma,qab,qnb,q->mn", P, c.g, Qm, c.w)          # noqa: E731
        for name, S3 in (("plus", plus), ("minus", minus)):
            G3 = ip(S3, S3); b3 = ip(S3, c.Pex[:, None, :])[:, 0]
            coef = np.linalg.solve(G3, b3)
            res = c.Pex - np.einsum("m,qma->qa", coef, S3)
            r = np.sqrt(np.einsum("qa,qab,qb,q->", res, c.g, res, c.w) / np.einsum("qa,qab,qb,q->", c.Pex, c.g, c.Pex, c.w))
            if name == "plus":
                rp = r
            else:
                rm = r
        if left_is is None:
            left_is = "plus" if rp < rm else "minus"
        worst_in = max(worst_in, rp if left_is == "plus" else rm)
        enforced = np.concatenate([minus if left_is == "plus" else plus] + ([lin] if GENERATORS == "all" else []), axis=1)
        Ge = ip(enforced, enforced); be = ip(enforced, c.Pex[:, None, :])[:, 0]
        coef = np.linalg.lstsq(Ge, be, rcond=1e-12)[0]
        res = c.Pex - np.einsum("m,qma->qa", coef, enforced)
        best_out = min(best_out, float(np.sqrt(np.einsum("qa,qab,qb,q->", res, c.g, res, c.w) /
                                               np.einsum("qa,qab,qb,q->", c.Pex, c.g, c.Pex, c.w))))
    out["left_invariant_family"] = left_is
    out["C_span_control_in_heldout_max"] = worst_in
    out["C_span_control_from_enforced_min"] = best_out
    print("C-span", left_is, worst_in, best_out, flush=True)

    def enforced_gens(c, verts):
        plus, minus, lin = generators_on(c, verts)
        return np.concatenate([minus if left_is == "plus" else plus] + ([lin] if GENERATORS == "all" else []), axis=1)

    # constraints, per edge on the sheet of a reference cell
    owner = {}
    for ci, c in enumerate(cells):
        for e in c["edges"]:
            owner.setdefault(e, ci)
    users = {}
    for k, c in enumerate(C):
        for e in c.tcols:
            users.setdefault(e, []).append(k)
    rows_count_check = 0
    rhs, row_count = [], 0
    blocks = {k: [] for k in range(len(C))}
    multi = 0
    for e in range(ne):
        r = owner[e]
        i0, j0 = pg.EDGES[cells[r]["edges"].index(e)]
        Pv, Qv = C[r].verts[i0], C[r].verts[j0]
        gref = enforced_gens(C[r], C[r].verts)
        Sg = np.einsum("qma,qab,qnb,q->mn", gref, C[r].g, gref, C[r].w)
        ev, evec = np.linalg.eigh(Sg)
        keep = ev > 1e-10 * ev.max()
        Qe = evec[:, keep] / np.sqrt(ev[keep])                # orthonormal effective basis on the ref cell
        rhs_e = np.zeros(Qe.shape[1])
        contrib = []
        for k in users.get(e, []):
            h, ncopies = lift_to(C[k], e, Pv, Qv)
            multi += ncopies > 1
            Vk = np.array([pg.qmul(h, v) for v in C[k].verts])
            v, _ = C[k].pair(enforced_gens(C[k], Vk))
            vQ = v @ Qe
            ck = C[k].col[e]
            rhs_e -= C[k].weight * (C[k].T[:, ck] @ vQ)
            contrib.append((k, C[k].weight * np.einsum("pm,q->mpq", C[k].Z.T @ vQ, C[k].ND[:, ck])))
        for k, c in enumerate(cells):
            if e in c["edges"]:
                jj = c["edges"].index(e)
                a0, b0 = pg.EDGES[jj]
                p, q = C[k].verts[a0], C[k].verts[b0]
                hk = None
                for A_, B_ in ((Pv, Qv), (Qv, Pv)):
                    h = pg.qmul(A_, pg.qconj(p))
                    if np.abs(pg.qmul(h, q) - B_).max() < 1e-9:
                        hk = h; break
                if hk is None:
                    raise RuntimeError(f"cell {k} does not carry edge {e} onto the reference")
                Vk = np.array([pg.qmul(hk, v) for v in C[k].verts])
                _, u = C[k].pair(enforced_gens(C[k], Vk))
                rhs_e += C[k].weight * (C[k].TW[:, jj] @ (u @ Qe))
        m = Qe.shape[1]
        for k, coef in contrib:
            blocks[k].append((row_count, coef.reshape(m, -1)))
        rhs.append(rhs_e)
        row_count += m
    out["cells_seeing_two_copies_of_an_edge"] = int(multi)
    b = np.concatenate(rhs)
    print("constraints", row_count, "rows;", round(time.time() - t0, 1), "s", flush=True)
    # Variables. objective "change" (V1): minimise sum ||C_K||^2.  objective "stiffness" (V1b): minimise
    # trace K' = sum w ||G^1/2 (T + Z C ND)||^2; with S1 = Z^T G Z, S2 = ND ND^T and the substitution
    # C = P1 Ct P2 / sqrt(w) (P1 = S1^-1/2, P2 = S2^+1/2) it becomes min ||Ct - Ct0||^2, Ct0 the
    # unconstrained minimiser; directions with no effect on T' (null of S2) are fixed at zero.
    xform = {}
    for k, c in enumerate(C):
        nz, nq = c.Z.shape[1], c.N.shape[1]
        if objective == "stiffness":
            l1, U1 = np.linalg.eigh(c.Z.T @ c.G @ c.Z)
            l2, U2 = np.linalg.eigh(c.ND @ c.ND.T)
            k2 = l2 > 1e-12 * l2.max()
            P1 = U1 @ np.diag(l1 ** -0.5) @ U1.T
            P2 = U2[:, k2] @ np.diag(l2[k2] ** -0.5) @ U2[:, k2].T
            S1h = U1 @ np.diag(l1 ** 0.5) @ U1.T
            S2h = U2[:, k2] @ np.diag(l2[k2] ** 0.5) @ U2[:, k2].T
            S1i = U1 @ np.diag(1 / l1) @ U1.T
            S2p = U2[:, k2] @ np.diag(1 / l2[k2]) @ U2[:, k2].T
            C0 = -S1i @ c.Z.T @ c.G @ c.T @ c.ND.T @ S2p
            Ct0 = np.sqrt(c.weight) * S1h @ C0 @ S2h
            xform[k] = (P1 / np.sqrt(c.weight), P2, Ct0)
        else:
            xform[k] = (np.eye(nz), np.eye(nq), np.zeros((nz, nq)))
    # constraint blocks in the new variables, and the right-hand side shifted by the reference point
    bshift = b.copy()
    for k in range(len(C)):
        P1, P2, Ct0 = xform[k]
        nz, nq = Ct0.shape
        new = []
        for r0, co in blocks[k]:
            cm = co.reshape(-1, nz, nq)
            cm = np.einsum("ip,mpq,qj->mij", P1.T, cm, P2.T) if objective == "stiffness" else cm
            bshift[r0:r0 + cm.shape[0]] -= np.einsum("mij,ij->m", cm, Ct0)
            new.append((r0, cm.reshape(cm.shape[0], -1)))
        blocks[k] = new
    # normal equations, dense at this size
    AAt = np.zeros((row_count, row_count))
    Aloc = {}
    for k in range(len(C)):
        if not blocks[k]:
            continue
        idx = np.concatenate([np.arange(r0, r0 + co.shape[0]) for r0, co in blocks[k]])
        Ak = np.vstack([co for _, co in blocks[k]])
        Aloc[k] = (idx, Ak)
        AAt[np.ix_(idx, idx)] += Ak @ Ak.T
    ev, evec = eigh(AAt)
    keep = ev > 1e-12 * ev.max()
    y = evec[:, keep] @ ((evec[:, keep].T @ bshift) / ev[keep])
    Cs = {}
    resid = bshift.copy()
    for k, (idx, Ak) in Aloc.items():
        ck = Ak.T @ y[idx]
        resid[idx] -= Ak @ ck
        P1, P2, Ct0 = xform[k]
        Ct = Ct0 + ck.reshape(Ct0.shape)
        Cs[k] = (P1 @ Ct @ P2).ravel() if objective == "stiffness" else Ct.ravel()
    out["C_solve_relative_residual"] = float(np.linalg.norm(resid) / max(np.linalg.norm(b), 1e-300))
    out["constraint_rows"] = int(row_count); out["rank"] = int(keep.sum())
    # new maps
    Ts, dM, amp0, amp1, exact_err = [], [], [], [], 0.0
    for k, c in enumerate(C):
        nz, nq = c.Z.shape[1], c.N.shape[1]
        Ck = Cs.get(k, np.zeros(nz * nq)).reshape(nz, nq)
        Mn = c.M + c.Z @ Ck @ c.N.T
        Ts.append(Mn @ c.D)
        dM.append(float(np.linalg.norm(Ck)))
        amp0.append(float(np.linalg.norm(c.M, 2))); amp1.append(float(np.linalg.norm(Mn, 2)))
    out["change_norm_median"] = float(np.median(dM)); out["change_norm_max"] = float(np.max(dM))
    out["amplification_median_before_after"] = [float(np.median(amp0)), float(np.median(amp1))]
    out["amplification_max_before_after"] = [float(np.max(amp0)), float(np.max(amp1))]
    K1 = assemble(Ts)
    out["V1_relative_error"] = rel(K1)
    out["trace_before_after"] = [float(np.trace(K0)), float(np.trace(K1))]
    # Proposition 1 decomposition for S = R (variational): covectors over edges
    def terms(Tlist):
        rec = np.zeros(ne); tv = np.zeros(ne); cnum = cden = 0.0
        for c, T in zip(C, Tlist):
            coeff = T @ a[c.tcols]
            P = np.einsum("qka,k->qa", c.trial, coeff)
            diff = P - c.Pex
            gdiff = np.einsum("qka,qab,qb,q->k", c.trial, c.g, diff, c.w)
            rec[c.tcols] += c.weight * (T.T @ gdiff)
            vF = np.einsum("qka,qab,qb,q->k", c.trial, c.g, c.Pex, c.w)
            uF = np.einsum("qfa,qab,qb,q->f", c.two, c.g, c.Pex, c.w)
            tv[c.tcols] += c.weight * (T.T @ vF)
            tv[c.wedges] -= c.weight * (c.TW.T @ uF)
            cnum += c.weight * float(np.einsum("qa,qab,qb,q->", diff, c.g, diff, c.w))
            cden += c.weight * float(np.einsum("qa,qab,qb,q->", c.Pex, c.g, c.Pex, c.w))
        r4 = norm(-4 * a)
        return {"reconstruction_term": norm(solve_rhs(mass, rec)) / r4,
                "test_variation_term": norm(solve_rhs(mass, tv)) / r4,
                "curvature_relative_error": float(np.sqrt(cnum / cden))}
    out["terms_before"] = terms([c.T for c in C])
    out["terms_after"] = terms(Ts)
    out["asymmetry"] = float(np.abs(K1 - K1.T).max() / np.abs(K1).max())
    # S3: generalized symmetric spectrum against the mass
    Mden = mass.toarray() if hasattr(mass, "toarray") else np.asarray(mass)
    lam = eigh(0.5 * (K1 + K1.T), Mden, eigvals_only=True)
    out["S3_small_count"] = int((np.abs(lam) < 1).sum())
    out["S3_negative_below_minus1"] = int((lam < -1).sum())
    out["S3_min_eig"] = float(lam.min())
    phys = lam[np.abs(lam) >= 1]
    out["S3_min_physical"] = float(phys.min()) if len(phys) else None
    out["seconds"] = round(time.time() - t0, 1)
    print(json.dumps(out), flush=True)
    return out


if __name__ == "__main__":
    args = sys.argv[1:]
    objective = "change"
    if "--generators" in args:          # V1c: "constants" enforces only the 3 right-invariant constants
        i = args.index("--generators"); GENERATORS = args[i + 1]; del args[i:i + 2]
        assert GENERATORS in ("all", "constants")
    if "--objective" in args:
        i = args.index("--objective"); objective = args[i + 1]; del args[i:i + 2]
    level = int(args[0])
    receipt = Path(args[1]) if len(args) > 1 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    out = run(level, objective)
    if receipt:
        receipt.parent.mkdir(parents=True, exist_ok=True)
        receipt.write_text(json.dumps(out, indent=1) + "\n")
