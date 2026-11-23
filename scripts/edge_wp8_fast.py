"""Fast block evaluation for pre-registration G (research/edge-theorem-20261122-g/), the same mathematics as Test F's
`edge_wp7_principal_circle.GroupK`, rebuilt so that the bigger groups G2 and Sp(3) are affordable:

  - the pieces of ricci_su3_test2_family.hessian_and_gauge (per frame pair D_ij, the curvature blocks, G) are kept
    apart, never forming the N x N Hessian;
  - the charge Q_K = s0 (i ad K) x 1 x 1 + s1 1 x (i ad K) x 1 + s2 1 x 1 x (i rho(K)) is diagonalised factor by factor
    (small eigh calls), so its eigenbasis U is a Kronecker product; each D_ij is carried into it factor by factor and
    only the charge blocks U^* H U (and the columns of her top block, for the exactness residual) are accumulated;
  - the sign fit and the commutation gate use random probe vectors with a matrix-free H;
  - per (sector, shape) the quadratic pieces H0, H1, H2 and the linear pieces of G are stored block by block.

Gate G-K0 (in the G driver) checks this code against Test F's dense code at F points, to 1e-9 relative.
Also: algebra builders for g2 and sp3 (from scripts/arc_principal_survey.py) and highest-weight sector construction
from the defining representation's weights.
"""
import sys
from pathlib import Path

import numpy as np
import scipy.sparse as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T2        # noqa: E402
import edge_wp7_groups as GR               # noqa: E402
import arc_principal_survey as SURV        # noqa: E402
import edge_wp7_principal_circle as F      # noqa: E402

_orig_algebra = GR.algebra


def _algebra(name):
    if name in ("g2", "sp3"):
        return SURV.build(name)
    return _orig_algebra(name)


GR.algebra = _algebra                      # F.GroupK calls GR.algebra; g2 / sp3 now resolve to the survey builders


def operator_pieces(C, g, f, M, rho):
    """The pieces of ricci_su3_test2_family.hessian_and_gauge, kept apart (nothing N x N is formed):
    B[m] = deriv[m] + ad[m] (rd x rd, sparse); for each frame pair (i, j) the column blocks of D_ij
    {j: B[i], i: -B[j], k: -c_ijk I}; the curvature blocks P_ij (x) I_d; c (the scaled frame brackets).
    H = sum_pairs D_ij^* D_ij + sum_curv (E_ij (x) P + E_ji (x) P^T);  G = vstack(B)."""
    n, r, d = len(g), f.shape[0], rho[0].shape[0]
    s = np.sqrt(np.asarray(g, float))
    c = C * s[None, None, :] / (s[:, None, None] * s[None, :, None])
    Ir, Id = sp.identity(r, format="csr"), sp.identity(d, format="csr")
    B = [(sp.kron(Ir, sp.csr_matrix(rho[i] / s[i])) + sp.kron(sp.csr_matrix(T2.adm(M[i], f)), Id)).tocsr() for i in range(n)]
    pairs = [(i, j) for i in range(n) for j in range(i + 1, n)]
    curv = []
    for (i, j) in pairs:
        Fij = T2.adm(M[i], f) @ M[j] - sum(c[i, j, k] * M[k] for k in range(n))
        if np.abs(Fij).max() == 0:
            continue
        P = np.einsum("a,bca->bc", Fij, f)
        curv.append((i, j, sp.kron(sp.csr_matrix(P), Id).tocsr()))
    return {"B": B, "c": c, "pairs": pairs, "curv": curv, "n": n, "rd": r * d}


def D_sparse(ops):
    n, rd = ops["n"], ops["rd"]; rows = []
    Ird = sp.identity(rd, format="csr", dtype=complex)
    for (i, j) in ops["pairs"]:
        acc = {j: ops["B"][i], i: -ops["B"][j]}
        for k in range(n):
            if ops["c"][i, j, k] != 0:
                acc[k] = acc.get(k, sp.csr_matrix((rd, rd), dtype=complex)) - ops["c"][i, j, k] * Ird
        rows.append(sp.hstack([acc.get(k, sp.csr_matrix((rd, rd), dtype=complex)) for k in range(n)], format="csr"))
    return sp.vstack(rows, format="csr")


def curv_sparse(ops):
    n, rd = ops["n"], ops["rd"]; N = n * rd
    H = sp.lil_matrix((N, N), dtype=complex)
    for (i, j, Pk) in ops["curv"]:
        H[i * rd:(i + 1) * rd, j * rd:(j + 1) * rd] = H[i * rd:(i + 1) * rd, j * rd:(j + 1) * rd] + Pk
        H[j * rd:(j + 1) * rd, i * rd:(i + 1) * rd] = H[j * rd:(j + 1) * rd, i * rd:(i + 1) * rd] + Pk.T
    return H.tocsr()


class MatFreeH:
    """H v without forming H (for the probe gates)."""

    def __init__(self, ops=None, D=None, Cv=None, G=None):
        if ops is not None:
            D, Cv, G = D_sparse(ops), curv_sparse(ops), sp.vstack(ops["B"], format="csr")
        self.D, self.Cv, self.G = D.tocsr(), Cv.tocsr(), G.tocsr()
        self.DH = self.D.conj().T.tocsr(); self.GH = self.G.conj().T.tocsr()

    def __call__(self, v):
        return self.DH @ (self.D @ v) + self.Cv @ v

    def GG(self, v):
        return self.G @ (self.GH @ v)


class FastGroupK(F.GroupK):
    """Test F's GroupK, evaluated block by block in the Kronecker eigenbasis of Q_K (no N x N matrix is formed)."""

    def ops(self, x, shape, s):
        g = np.ones(self.d); g[0] = F.T
        return operator_pieces(self.Cy, g, self.Cy, self.field(x, shape), self.reps[s])

    def factors(self, s):
        Af = 1j * self.adK; AV = 1j * self.reps[s][0]
        Af = 0.5 * (Af + Af.conj().T); AV = 0.5 * (AV + AV.conj().T)
        return Af, AV

    def Qv(self, v, s, sg):
        Af, AV = self.factors(s); d = self.d; dV = AV.shape[0]
        T = v.reshape(d, d, dV)
        out = sg[0] * np.einsum("ab,bcd->acd", Af, T) + sg[1] * np.einsum("cb,abd->acd", Af, T) + sg[2] * np.einsum("db,acb->acd", AV, T)
        return out.reshape(-1)

    def sign_fit(self, shape, s, x=1.0, nprobe=3):
        Hm = MatFreeH(self.ops(x, shape, s)); _, AV = self.factors(s)
        N = self.d * self.d * AV.shape[0]; rng = np.random.default_rng(5)
        V = [rng.standard_normal(N) + 1j * rng.standard_normal(N) for _ in range(nprobe)]
        ok, seen = [], set()
        for sg in [(1, 1, 1), (1, 1, -1), (1, -1, 1), (1, -1, -1)]:
            eff = (sg[0], sg[1], sg[2] if np.abs(AV).max() > 0 else 0)
            if eff in seen:
                continue
            e = 0.0
            for v in V:
                Hv = Hm(v); a = self.Qv(Hv, s, sg) - Hm(self.Qv(v, s, sg))
                Gv = Hm.GG(v); b = self.Qv(Gv, s, sg) - Hm.GG(self.Qv(v, s, sg))
                scale = max(np.abs(self.factors(s)[0]).max(), np.abs(AV).max(), 1.0)
                e = max(e, np.linalg.norm(a) / (scale * max(np.linalg.norm(Hv), 1e-300)), np.linalg.norm(b) / (scale * max(np.linalg.norm(Gv), 1e-300)))
            if e <= 1e-10:
                ok.append(sg); seen.add(eff)
        return ok

    def sym_probe(self, x, shape, s, sg, nprobe=3):
        """|[Q, H(x)] v| / (|Q| |H v|) over random probes: the G-sym gate."""
        Hm = MatFreeH(self.ops(x, shape, s)); Af, AV = self.factors(s)
        N = self.d * self.d * AV.shape[0]; rng = np.random.default_rng(9); e = 0.0
        scale = max(np.abs(Af).max(), np.abs(AV).max(), 1.0)
        for _ in range(nprobe):
            v = rng.standard_normal(N) + 1j * rng.standard_normal(N)
            Hv = Hm(v); a = self.Qv(Hv, s, sg) - Hm(self.Qv(v, s, sg))
            e = max(e, np.linalg.norm(a) / (scale * np.linalg.norm(Hv)))
        return e

    def basis_info(self, s, sg):
        Af, AV = self.factors(s); d = self.d; dV = AV.shape[0]
        qf, Uf = np.linalg.eigh(Af); qV, UV = np.linalg.eigh(AV)
        q = (sg[0] * qf[:, None, None] + sg[1] * qf[None, :, None] + sg[2] * qV[None, None, :]).ravel()
        flip = abs(q.min() + q.max()) > 1e-8 and q.max() < -q.min()
        if flip:
            q = -q
        W = np.kron(Uf, UV)                                              # (alg, V) factor of U, rd x rd
        qr = np.round(q, 7)
        ia, ib, ic = np.unravel_index(np.arange(q.size), (d, d, dV))
        cols = ib * dV + ic
        blocks = {cv: np.where(qr == cv)[0] for cv in np.unique(qr)}
        return {"q": q, "Uf": Uf, "UV": UV, "W": W, "ia": ia, "cols": cols, "blocks": blocks, "flip": flip}

    def U_columns(self, bi, idx):
        d = self.d; dV = bi["UV"].shape[0]
        ia, ib, ic = np.unravel_index(idx, (d, d, dV))
        Uf, UV = bi["Uf"], bi["UV"]
        return (Uf[:, ia][:, None, None, :] * Uf[:, ib][None, :, None, :] * UV[:, ic][None, None, :, :]).reshape(-1, idx.size)

    def build_blocks(self, x, shape, s, bi, top):
        """U^* H(x) U on every charge block, U^* H(x) U[:, top] (all rows), and G(x)^* U, from the pieces."""
        ops = self.ops(x, shape, s); Uf, W, ia, cols = bi["Uf"], bi["W"], bi["ia"], bi["cols"]
        d, rd = self.d, ops["rd"]; N = d * rd
        BW = [Bm @ W for Bm in ops["B"]]
        Hb = {cv: np.zeros((idx.size, idx.size), complex) for cv, idx in bi["blocks"].items()}
        Ht = np.zeros((N, top.size), complex)
        for (i, j) in ops["pairs"]:
            ck = ops["c"][i, j]; cU = ck @ Uf                                 # sum_k c_ijk Uf[k, :]
            Dfull = BW[i][:, None, :] * Uf[j][None, :, None] - BW[j][:, None, :] * Uf[i][None, :, None]
            if np.any(ck != 0):
                Dfull = Dfull - cU[None, :, None] * W[:, None, :]
            Dflat = Dfull.reshape(rd, N)
            for cv, idx in bi["blocks"].items():
                Db = Dflat[:, idx]; Hb[cv] += Db.conj().T @ Db
            Ht += Dflat.conj().T @ Dflat[:, top]
        for (i, j, Pk) in ops["curv"]:
            PW = W.conj().T @ (Pk @ W); PWt = W.conj().T @ (Pk.T @ W)
            for cv, idx in bi["blocks"].items():
                a_, c_ = ia[idx], cols[idx]
                Hb[cv] += np.outer(Uf[i, a_].conj(), Uf[j, a_]) * PW[np.ix_(c_, c_)] + np.outer(Uf[j, a_].conj(), Uf[i, a_]) * PWt[np.ix_(c_, c_)]
            at, ct = ia[top], cols[top]
            Ht += (Uf[i].conj()[:, None, None] * PW[None, :, ct] * Uf[j, at][None, None, :]).reshape(N, top.size)
            Ht += (Uf[j].conj()[:, None, None] * PWt[None, :, ct] * Uf[i, at][None, None, :]).reshape(N, top.size)
        BHW = np.array([(Bm.conj().T @ W) for Bm in ops["B"]])                # (d, rd, rd)
        Gs = np.einsum("ma,mrs->ras", Uf, BHW).reshape(rd, N)
        for cv in Hb:
            Hb[cv] = 0.5 * (Hb[cv] + Hb[cv].conj().T)
        return Hb, Ht, Gs

    def prepare(self, shape, s):
        key = (tuple(shape), s)
        if key in self.cache:
            return self.cache[key]
        ok = self.sign_fit(shape, s)
        if len(ok) != 1:
            raise SystemExit(f"stop: G-sym sign fit {self.gname} {self.angle} {s}: {ok}")
        sg = ok[0]; bi = self.basis_info(s, sg); q = bi["q"]
        top = np.where(np.abs(q - q.max()) < 1e-7)[0]; bot = np.where(np.abs(q - q.min()) < 1e-7)[0]
        Hz, Tz, Gz = self.build_blocks(0.0, shape, s, bi, top)
        Hp, Tp, Gp = self.build_blocks(1.0, shape, s, bi, top)
        Hm, Tm, Gm = self.build_blocks(-1.0, shape, s, bi, top)
        blocks = {}
        for cv, idx in bi["blocks"].items():
            blocks[cv] = {"idx": idx, "q": float(q[idx[0]]), "H0": Hz[cv], "H1": (Hp[cv] - Hm[cv]) / 2, "H2": (Hp[cv] + Hm[cv]) / 2 - Hz[cv],
                          "Gs0": Gz[:, idx], "Gs1": ((Gp - Gm) / 2)[:, idx]}
        ctop = max(blocks, key=lambda cv: blocks[cv]["q"]); cbot = min(blocks, key=lambda cv: blocks[cv]["q"])
        others = np.setdiff1d(np.arange(q.size), top)
        T0, T1, T2_ = Tz[others], (Tp[others] - Tm[others]) / 2, (Tp[others] + Tm[others]) / 2 - Tz[others]
        out = {"q": q, "blocks": blocks, "ctop": ctop, "cbot": cbot, "top": top, "bot": bot, "signs": sg, "N": q.size,
               "offtop": (T0, T1, T2_), "bi": bi}
        assert np.array_equal(blocks[ctop]["idx"], top) and np.array_equal(blocks[cbot]["idx"], bot)
        self.cache[key] = out
        return out

    def evaluate(self, x, shape, s, want_vec=False):
        pr = self.prepare(shape, s); B = pr["blocks"]
        bt, bb = B[pr["ctop"]], B[pr["cbot"]]
        Ht = bt["H0"] + x * bt["H1"] + x * x * bt["H2"]; wt, vt = np.linalg.eigh(Ht)
        Hb = bb["H0"] + x * bb["H1"] + x * x * bb["H2"]; wb = np.linalg.eigvalsh(Hb)
        lam = float(wt[0]); top_spread = float(np.ptp(wt))
        mirror_tied = bool(abs(wb[0] - lam) <= 1e-9 * max(1.0, abs(lam)))
        tol_ = 1e-9 * max(1.0, abs(lam))
        n_top_tied = int((np.abs(wt - lam) <= tol_).sum()); n_bot_tied = int((np.abs(wb - wb[0]) <= tol_).sum())
        T0, T1, T2_ = pr["offtop"]
        resid = float(np.linalg.norm(T0 + x * T1 + x * x * T2_))          # = |H[others, top]| in the eigenbasis
        Gst = bt["Gs0"] + x * bt["Gs1"]
        gstar = float(np.linalg.norm(Gst @ vt[:, 0]))
        Gsx = {cv: b["Gs0"] + x * b["Gs1"] for cv, b in B.items()}
        gmax = max((np.linalg.svd(Gc, compute_uv=False).max() for Gc in Gsx.values() if Gc.size), default=0.0)
        mu = []
        for cv, b in B.items():
            Hc = b["H0"] + x * b["H1"] + x * x * b["H2"]; Gc = Gsx[cv]
            if Gc.shape[0] and gmax > 0:
                _, sv, Vh = np.linalg.svd(Gc, full_matrices=True)
                V = Vh.conj().T[:, int((sv > 1e-10 * gmax).sum()):]
            else:
                V = np.eye(Hc.shape[0], dtype=complex)
            if V.shape[1]:
                A = V.conj().T @ Hc @ V; A = 0.5 * (A + A.conj().T)
                mu.extend(np.linalg.eigvalsh(A))
        mu = np.sort(np.array(mu))
        ncopies = n_top_tied + (n_bot_tied if mirror_tied else 0)
        out = {"lam": lam, "lam_top_spread": 0.0, "top_block_spread": top_spread, "top_dim": int(Ht.shape[0]),
               "mirror_tied": mirror_tied, "resid": resid, "gstar": gstar, "mu": mu, "ncopies": ncopies}
        if want_vec:
            out["her_vec"] = self.U_columns(pr["bi"], pr["top"]) @ vt[:, 0]
        return out

    def HG(self, x, shape, s):
        """Dense H and G (used only by Test F's bounding() on the trivial sector)."""
        g = np.ones(self.d); g[0] = F.T
        return T2.hessian_and_gauge(self.Cy, g, self.Cy, self.field(x, shape), self.reps[s])


# ------------------------------------------------------------------------------------------------- sectors
def defining_weights(g):
    """Weights (eigenvalues of i*T on the defining representation) and weight vectors, highest first under the same
    positive functional GroupK uses (key = 1, 2, ..., rank)."""
    rng = np.random.default_rng(11); cc = rng.standard_normal(len(g.tor))
    Mg = sum(ci * 1j * T for ci, T in zip(cc, g.tor)); Mg = 0.5 * (Mg + Mg.conj().T)
    _, V = np.linalg.eigh(Mg)
    wts = np.array([[float((V[:, k].conj() @ (1j * T) @ V[:, k]).real) for T in g.tor] for k in range(V.shape[1])])
    key = np.arange(1, len(g.tor) + 1, dtype=float)
    o = np.argsort(-(wts @ key))
    return wts[o], V[:, o], (wts @ key)[o]


def antisym(vs):
    import itertools
    k = len(vs); out = 0
    for p in itertools.permutations(range(k)):
        sgn = np.linalg.det(np.eye(k)[list(p)])
        out = out + sgn * F.tens(*[vs[i] for i in p])
    return out


def sector_specs(g, gname):
    """Main sectors and truncation-check sectors, as (p, q, hw_vector, expected_dim); 'adj' is built from ad."""
    wts, V, fv = defining_weights(g)
    assert all(abs(fv[i] - fv[i + 1]) > 1e-9 for i in range(3)), "highest defining weights not separated"
    v = [V[:, i] for i in range(V.shape[1])]
    if gname == "g2":
        a2 = antisym(v[:2])
        main = {"1": (0, 0, None, 1), "7": (1, 0, v[0], 7), "14": "adj", "27": (2, 0, F.tens(v[0], v[0]), 27)}
        extra = {"77s": (3, 0, F.tens(v[0], v[0], v[0]), 77), "64": (3, 0, F.tens(v[0], a2), 64),
                 "77p": (4, 0, np.kron(a2, a2), 77)}
    elif gname == "sp3":
        main = {"1": (0, 0, None, 1), "6": (1, 0, v[0], 6), "14": (2, 0, antisym(v[:2]), 14), "21": "adj",
                "14p": (3, 0, antisym(v[:3]), 14)}
        a2 = antisym(v[:2])
        extra = {"56s": (3, 0, F.tens(v[0], v[0], v[0]), 56), "64": (3, 0, F.tens(v[0], a2), 64),
                 "90": (4, 0, np.kron(a2, a2), 90), "126s": (4, 0, F.tens(v[0], v[0], v[0], v[0]), 126)}
    else:
        raise ValueError(gname)
    return main, extra


def add_sectors(g, specs):
    ops = [E for _, E in g.rts]
    for key_, spec in specs.items():
        if spec == "adj":
            Cb = GR.structure(g.basis)
            RX = [Cb[a].T.astype(complex) for a in range(len(g.basis))]
        elif spec[0] + spec[1] == 0:
            RX = [np.zeros((1, 1), complex) for _ in g.basis]
        else:
            p, q, vec, dim = spec
            RX = GR.irrep_from(g.basis, ops, np.asarray(vec, complex), GR.rep_tensor(None, p, q), dim_expected=dim)
        g.reps[key_] = [sum(g.R[k, a] * RX[a] for a in range(len(g.basis))) for k in range(g.d)]


def rep_check(g, key_):
    """rho([Y_a, Y_b]) = [rho Y_a, rho Y_b] in the rotated basis, max deviation."""
    RY = g.reps[key_]; worst = 0.0
    for a in range(g.d):
        for b in range(a + 1, g.d):
            lhs = sum(g.Cy[a, b, c] * RY[c] for c in range(g.d)); rhs = RY[a] @ RY[b] - RY[b] @ RY[a]
            worst = max(worst, float(np.abs(lhs - rhs).max()))
    return worst


# ------------------------------------------------------------------------------------- matrix-free check sectors
import scipy.sparse.linalg as spla   # noqa: E402


def check_pieces(g, shape, s):
    """D, G linear in x and the curvature quadratic in x (gated by G-S5c): pieces built once per (shape, sector)."""
    key = ("chk", tuple(shape), s)
    if key in g.cache:
        return g.cache[key]
    P = {}
    for xv in (0.0, 1.0, -1.0):
        ops = g.ops(xv, shape, s); P[xv] = (D_sparse(ops), curv_sparse(ops), sp.vstack(ops["B"], format="csr"))
    D0, C0, G0 = P[0.0]
    out = {"D": (D0, (P[1.0][0] - P[-1.0][0]) * 0.5), "G": (G0, (P[1.0][2] - P[-1.0][2]) * 0.5),
           "C": (C0, (P[1.0][1] - P[-1.0][1]) * 0.5, (P[1.0][1] + P[-1.0][1]) * 0.5 - C0)}
    ok = g.sign_fit(shape, s)
    if len(ok) != 1:
        raise SystemExit(f"stop: G-sym sign fit (check sector) {g.gname} {g.angle} {s}: {ok}")
    out["sg"] = ok[0]; out["bi"] = g.basis_info(s, ok[0])
    g.cache[key] = out
    return out


def check_margin(g, x, shape, s, k=3, tol=1e-11, beta_pad=100.0):
    """Her margin in sector s by a matrix-free Lanczos solve on the exact gauge-orthogonal slice (no N x N matrix, no
    charge blocks): A = Pc H Pc + beta (1 - Pc), Pc = projector onto ker G^* minus her copies. Returns the same dict
    shape F.phi reads: lam (her top-state price in this sector), mu (lowest eigenvalues of A), ncopies = 0."""
    cp = check_pieces(g, shape, s)
    Hm = MatFreeH(D=cp["D"][0] + x * cp["D"][1], Cv=cp["C"][0] + x * cp["C"][1] + x * x * cp["C"][2], G=cp["G"][0] + x * cp["G"][1])
    Gm = Hm.G
    GtG = (Gm.conj().T @ Gm).toarray(); GtG = 0.5 * (GtG + GtG.conj().T)
    w, Z = np.linalg.eigh(GtG); keep = w > 1e-12 * w.max(); Zk = Z[:, keep] / w[keep]
    N = Gm.shape[0]

    ZkH = Z[:, keep].conj().T

    def Pslice(v):
        return v - Gm @ (Zk @ (ZkH @ (Hm.GH @ v)))
    bi = cp["bi"]; q = bi["q"]
    top = np.where(np.abs(q - q.max()) < 1e-7)[0]; bot = np.where(np.abs(q - q.min()) < 1e-7)[0]
    Ut = g.U_columns(bi, top); Ub = g.U_columns(bi, bot)
    HUt = np.column_stack([Hm(Ut[:, i]) for i in range(Ut.shape[1])]); Ht = Ut.conj().T @ HUt; Ht = 0.5 * (Ht + Ht.conj().T)
    HUb = np.column_stack([Hm(Ub[:, i]) for i in range(Ub.shape[1])]); Hb = Ub.conj().T @ HUb; Hb = 0.5 * (Hb + Hb.conj().T)
    wt, vt = np.linalg.eigh(Ht); wb, vb = np.linalg.eigh(Hb); lam = float(wt[0]); tol_ = 1e-9 * max(1.0, abs(lam))
    resid = float(np.linalg.norm(HUt - Ut @ Ht))
    cop = [Ut @ vt[:, i] for i in range(len(wt)) if abs(wt[i] - lam) <= tol_]
    mirror_tied = bool(abs(wb[0] - lam) <= tol_)
    if mirror_tied:
        cop += [Ub @ vb[:, i] for i in range(len(wb)) if abs(wb[i] - wb[0]) <= tol_]
    C = np.column_stack(cop); C, _ = np.linalg.qr(C)
    gstar = float(np.linalg.norm(Hm.GH @ C))
    beta = lam + beta_pad

    def Pc(v):
        u = Pslice(v); return u - C @ (C.conj().T @ u)

    def mv(v):
        v = np.asarray(v).reshape(-1); u = Pc(v)
        return Pc(Hm(u)) + beta * (v - u)
    A = spla.LinearOperator((N, N), matvec=mv, dtype=complex)
    vals = spla.eigsh(A, k=k, which="SA", tol=tol, return_eigenvectors=False, maxiter=20000)
    return {"lam": lam, "mu": np.sort(vals.real), "ncopies": 0, "resid": resid, "gstar": gstar, "mirror_tied": mirror_tied,
            "ncopies_removed": int(C.shape[1]), "top_dim": int(len(top)), "beta": beta}


class CheckGroupK(FastGroupK):
    """FastGroupK whose truncation-check sectors (self.check) are evaluated matrix-free."""
    check = ()

    def evaluate(self, x, shape, s, want_vec=False):
        if s in self.check:
            return check_margin(self, x, shape, s)
        return super().evaluate(x, shape, s, want_vec=want_vec)
