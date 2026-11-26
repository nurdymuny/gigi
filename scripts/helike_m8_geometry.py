"""M8 geometry kernel: the weighted Dirichlet geometry of an excitation (registration §1–§3, amendments 1–3).

The solver's trial functions are a unitary pullback of the physical states onto the chart z (u = u0 + S z):
  chi(z) = sqrt(rho(z)) Psi(u(z)),   rho(z) dz = dmu,   chi = p(x) e^{-|x|^2/2} prod (w/pi)^{1/4},   x = sqrt(w) z,
with p a polynomial in orthonormal Hermite functions. Hence:
  f = Psi / Psi0 = chi / chi0 = p / p0                       (the measure and the Gaussian cancel)
  grad_z f = sqrt(w) (dp p0 - p dp0) / p0^2                   (dp = d/dx; the -x p terms cancel)
  int chi_a chi_b F(z) dz = sum_nodes W p_a p_b F            (W = Gauss–Hermite weights / pi^3)
  D~chi = d_z chi - (1/2) ell chi,  ell = S^T (3,2,1,0,0,0):  D~chi / gauss = q = sqrt(w)(dp - x p) - ell p / 2
  <Psi_a, T Psi_b> = 1/2 sum W q_a^T G q_b,  G = g^{ij}_z (the chart's kinetic form, ConeChart.metric)

The identity (★) is  <psi,(H-E0)psi> = 1/2 int g grad f.grad f psi0^2 dmu + <f^2 psi0, (H-E0) psi0>,  with the residual
term evaluated in first-order form:
  R = sum W [ 1/2 (f^2 q0 + 2 f p0 grad f)^T G q0 + (V - E0) f^2 p0^2 ].

The positive shares (A2.1) are taken in the frame z~ = G(0)^{-1/2} z (A3.3), with v = G~^{1/2} grad_{z~} f.
The core functions are generic in (nodes, metric, potential), so the closed-form tests exercise the same code.
"""
import json
import math
from pathlib import Path

import numpy as np
from scipy.special import roots_hermite

BLOCKS = ("H", "C", "S", "R")
CHUNK = 65536


# ---------------------------------------------------------------- generic numerics
def sym_sqrt(G):
    """Batched symmetric positive square root of (n, d, d) matrices."""
    e, V = np.linalg.eigh(G)
    if np.min(e) <= 0:
        raise ArithmeticError("kinetic form is not positive definite at a node")
    return np.einsum("nij,nj,nkj->nik", V, np.sqrt(e), V)


def frame_matrix(G0, blocks, tol=1e-8):
    """A3.3: A = G(0)^{-1/2}, refusing a G(0) that is not block-diagonal across the declared blocks."""
    G0 = np.asarray(G0, float)
    lab = np.empty(len(G0), dtype=object)
    for name, idx in blocks.items():
        lab[list(idx)] = name
    off = np.array([[G0[i, j] if lab[i] != lab[j] else 0.0 for j in range(len(G0))] for i in range(len(G0))])
    if np.max(np.abs(off)) > tol * np.linalg.norm(G0, 2):
        raise ValueError("G(0) is not block-diagonal across H, C, S, R: the frame is undefined (OPEN)")
    e, V = np.linalg.eigh(G0)
    return (V / np.sqrt(e)) @ V.T


def positive_contributions(grad, G, A):
    """Per node and coordinate: 1/2 |v_k|^2 with v = G~^{1/2} grad_{z~} f, G~ = A G A^T, grad_{z~} = A^{-T} grad_z."""
    Gt = np.einsum("ij,njk,lk->nil", A, G, A)
    gt = np.linalg.solve(A, grad.T).T                        # A^{-T}: grad_z~ = A^{-T} grad_z, and A is symmetric
    v = np.einsum("nij,nj->ni", sym_sqrt(Gt), gt)
    return 0.5 * v * v


def raw_contributions(grad, G, A):
    Gt = np.einsum("ij,njk,lk->nil", A, G, A)
    gt = np.linalg.solve(A, grad.T).T
    return 0.5 * np.einsum("ni,nij,nj->nij", gt, Gt, gt)


class Core:
    """Quadrature data at the nodes: weights W, Hermite variables x, chart z, kinetic form G(z) (callable on a chunk
    of z), ell, the potential V(z) (callable, already including the energy shift), frequencies w."""

    def __init__(self, W, x, w, G_fn, ell, V_fn, y_fn=None, dy_fn=None):
        self.W, self.x, self.w = np.asarray(W), np.asarray(x), np.asarray(w)
        self.z = self.x / np.sqrt(self.w)
        self.G_fn, self.ell, self.V_fn = G_fn, np.asarray(ell, float), V_fn
        self.y_fn, self.dy_fn = y_fn, dy_fn                   # linear-Gram normal coordinates and their z-gradients

    def chunks(self):
        n = len(self.W)
        for a in range(0, n, CHUNK):
            yield slice(a, min(a + CHUNK, n))

    def q(self, p, dp, sl):
        return np.sqrt(self.w) * (dp[sl] - self.x[sl] * p[sl, None]) - 0.5 * self.ell * p[sl, None]

    def analyse(self, p0, dp0, p, dp, E0, blocks, A, positivity_floor=1e-12, cuts=(1e-12, 1e-14)):
        """Energies, the (★) identity terms, positive shares and the raw block matrix for one real state p.
        Amendment 5: shares and the raw matrix are integrated only where the vacuum weight W p0^2 exceeds cut x total;
        the first cut decides, the second is the cut seam. The identity terms use the full domain."""
        names = list(blocks)
        idx = {b: list(blocks[b]) for b in names}
        acc = dict(D=0.0, R=0.0, T=0.0, Vp=0.0, norm=0.0,
                   pos={c: {b: 0.0 for b in names} for c in cuts},
                   raw={c: np.zeros((len(names), len(names))) for c in cuts}, Dcut={c: 0.0 for c in cuts})
        wtot = float(np.sum(self.W * p0 * p0))
        bad = 0.0
        for sl in self.chunks():
            W = self.W[sl]
            P0, P = p0[sl], p[sl]
            heavy = W * P0 * P0 > positivity_floor * wtot
            if np.any(heavy & (P0 <= 0)):
                bad += float(np.sum((W * P0 * P0)[heavy & (P0 <= 0)]))
            safe = np.where(np.abs(P0) > 0, P0, 1.0)
            G = self.G_fn(self.z[sl])
            V = self.V_fn(self.z[sl])
            f = P / safe
            grad = np.sqrt(self.w) * (dp[sl] * P0[:, None] - P[:, None] * dp0[sl]) / (safe * safe)[:, None]
            q0, qa = self.q(p0, dp0, sl), self.q(p, dp, sl)
            wD = W * P0 * P0
            dens = 0.5 * wD * np.einsum("ni,nij,nj->n", grad, G, grad)
            acc["D"] += float(np.sum(dens))
            lhs = f[:, None] ** 2 * q0 + 2 * (f * P0)[:, None] * grad
            acc["R"] += float(np.sum(W * (0.5 * np.einsum("ni,nij,nj->n", lhs, G, q0) + (V - E0) * f * f * P0 * P0)))
            acc["T"] += 0.5 * float(np.sum(W * np.einsum("ni,nij,nj->n", qa, G, qa)))
            acc["Vp"] += float(np.sum(W * V * P * P))
            acc["norm"] += float(np.sum(W * P * P))
            pc = positive_contributions(grad, G, A)
            rc = raw_contributions(grad, G, A)
            for cut in cuts:
                m = wD > cut * wtot
                acc["Dcut"][cut] += float(np.sum(dens[m]))
                for i, b in enumerate(names):
                    acc["pos"][cut][b] += float(np.sum(wD[m, None] * pc[m][:, idx[b]]))
                    for j, c in enumerate(names):
                        acc["raw"][cut][i, j] += float(np.sum(wD[m, None, None] * rc[m][:, idx[b]][:, :, idx[c]]))
        D = acc["D"]
        c0 = cuts[0]
        Dc = acc["Dcut"][c0]
        tot = {c: sum(acc["pos"][c].values()) for c in cuts}
        shares_by_cut = {c: {b: acc["pos"][c][b] / tot[c] for b in names} for c in cuts}
        return dict(D=D, R=acc["R"], identity=D + acc["R"], kinetic=acc["T"], potential=acc["Vp"], norm=acc["norm"],
                    rayleigh=(acc["T"] + acc["Vp"]) / acc["norm"] - E0,
                    shares=shares_by_cut[c0], shares_by_cut={repr(c): v for c, v in shares_by_cut.items()},
                    D_by_cut={repr(c): acc["Dcut"][c] for c in cuts},
                    excluded_fraction={repr(c): 1 - acc["Dcut"][c] / D for c in cuts},
                    share_sum=tot[c0] / Dc if Dc else None,
                    raw=(acc["raw"][c0] / Dc).tolist() if Dc else None,
                    raw_sum=float(np.sum(acc["raw"][c0]) / Dc) if Dc else None,
                    nonpositive_weight=bad / wtot)

    def alignment(self, p0, p, modes):
        """Q2: projection weight of f on span{centred y_m, m in modes} in the psi0^2 inner product (Gram inverse;
        equal to the registration's sum when the y_m are orthogonal), per unit ||f||."""
        n0 = float(np.sum(self.W * p0 * p0))
        ybar = np.zeros(len(modes)); B = np.zeros((len(modes),) * 2); v = np.zeros(len(modes)); nf = 0.0
        for sl in self.chunks():
            y = self.y_fn(self.z[sl])[:, modes]
            ybar += np.sum((self.W[sl] * p0[sl] * p0[sl])[:, None] * y, axis=0)
        ybar /= n0
        for sl in self.chunks():
            y = self.y_fn(self.z[sl])[:, modes] - ybar
            w0 = self.W[sl] * p0[sl] * p0[sl]
            B += np.einsum("n,ni,nj->ij", w0, y, y) / n0
            v += np.sum((self.W[sl] * p[sl] * p0[sl])[:, None] * y, axis=0) / math.sqrt(n0)
            nf += float(np.sum(self.W[sl] * p[sl] * p[sl]))
        return float(v @ np.linalg.solve(B, v)) / nf

    def shape_price(self, p0, dp0, modes, E0):
        """Q3: the lowest Rayleigh quotient of span{ybar_m psi0} by (★) with its residual term (generalised 2x2 or
        1x1 eigenproblem)."""
        n0 = float(np.sum(self.W * p0 * p0))
        ybar = np.zeros(len(modes))
        for sl in self.chunks():
            ybar += np.sum((self.W[sl] * p0[sl] * p0[sl])[:, None] * self.y_fn(self.z[sl])[:, modes], axis=0)
        ybar /= n0
        K = np.zeros((len(modes),) * 2); B = np.zeros_like(K)
        for sl in self.chunks():
            W, P0 = self.W[sl], p0[sl]
            G = self.G_fn(self.z[sl]); V = self.V_fn(self.z[sl])
            y = self.y_fn(self.z[sl])[:, modes] - ybar
            dy = self.dy_fn(self.z[sl])[:, modes, :]            # (n, m, 6): d y_m / d z
            q0 = self.q(p0, dp0, sl)
            w0 = W * P0 * P0
            K += 0.5 * np.einsum("n,nai,nij,nbj->ab", w0, dy, G, dy)
            for a in range(len(modes)):
                for b in range(len(modes)):
                    yy = y[:, a] * y[:, b]
                    dyy = dy[:, a, :] * y[:, b, None] + dy[:, b, :] * y[:, a, None]
                    lhs = yy[:, None] * q0 + P0[:, None] * dyy          # D~(yy chi0) / gauss
                    K[a, b] += float(np.sum(W * (0.5 * np.einsum("ni,nij,nj->n", lhs, G, q0) + (V - E0) * yy * P0 * P0)))
            B += np.einsum("n,na,nb->ab", w0, y, y)
        e = np.linalg.eigvals(np.linalg.solve(B, K))
        return float(np.min(e.real))


# ---------------------------------------------------------------- the solver's states on a product grid
def normalized_hermites(x, nmax):
    h = np.ones((nmax + 1, len(x)))
    if nmax:
        h[1] = np.sqrt(2.0) * x
    for n in range(1, nmax):
        h[n + 1] = np.sqrt(2.0 / (n + 1)) * x * h[n] - np.sqrt(n / (n + 1)) * h[n - 1]
    return h


def hermite_tables(x, nmax):
    h = normalized_hermites(x, nmax)
    dh = np.zeros_like(h)
    for n in range(1, nmax + 1):
        dh[n] = np.sqrt(2.0 * n) * h[n - 1]
    return h, dh


def product_grid(order):
    x, wts = roots_hermite(order)
    return x, wts / np.sqrt(np.pi)


def grid_values(basis, coeff, order, mq):
    """Polynomial parts p and their x-gradients for the given coefficient columns on the full product grid.
    Returns p (n_nodes, ncol) and dp (n_nodes, 6, ncol)."""
    x, _ = product_grid(order)
    h, dh = hermite_tables(x, mq)
    ncol = coeff.shape[1]
    T = np.zeros((mq + 1,) * 6 + (ncol,))
    for i, n in enumerate(basis):
        T[tuple(n)] = coeff[i]

    def contract(tables):
        v = T
        for axis in range(6):
            v = np.moveaxis(np.tensordot(tables[axis].T, v, axes=(1, axis)), 0, axis)
        return v.reshape(order ** 6, ncol)
    p = contract([h] * 6)
    dp = np.stack([contract([dh if a == j else h for a in range(6)]) for j in range(6)], axis=1)
    return p, dp


def grid_nodes(order, w):
    x1, w1 = product_grid(order)
    ix = np.array(np.unravel_index(np.arange(order ** 6), (order,) * 6)).T
    return np.prod(w1[ix], axis=1), x1[ix]


# ---------------------------------------------------------------- loading a cone-levels export
def f64(b, shape=None):
    a = np.array(b, dtype=np.uint64).view(np.float64)
    return a.reshape(shape) if shape else a


def load_levels(path):
    e = json.loads(Path(path).read_text())
    if e.get("schema") != "helike-lab/cone-levels-export/v1" or e.get("moments") != "exact":
        raise ValueError("not an exact-moment cone-levels export")
    n, k = e["coeff_shape"]
    return dict(raw=e, u0=f64(e["u0"]), S=f64(e["S"], (6, 6)), w=f64(e["w"]), G0=f64(e["G0"], (3, 3)),
                M=f64(e["M"], (6, 6)), basis=[tuple(b) for b in e["basis"]], coeff=f64(e["coeff"], (n, k)),
                levels=np.array(e["levels"]), resid=np.array(e["in_space_residuals"]),
                charge=np.array(e["charge_im"]).reshape(k, k), roles=e["roles"], shift=e["energy_shift"],
                mq=e["Mq"], s=e["s"], t=e["t"], g2=e["g2"])


def branches(X, pair_min=0.5):
    """Branches from the charge compression: a pair is a mutual strongest coupling with |Im<v_a,Lv_b>| > pair_min.
    Returns [(levels tuple, |charge|)] in level order."""
    C = np.abs(X["charge"]); k = len(C)
    out, used = [], set()
    for a in range(k):
        if a in used:
            continue
        b = int(np.argmax(C[a]))
        if C[a, b] > pair_min and int(np.argmax(C[b])) == a and b not in used:
            out.append(((a, b), round(float(C[a, b]))))
            used.update((a, b))
        else:
            out.append(((a,), 0))
            used.add(a)
    return sorted(out, key=lambda t: t[0][0])


def block_map(roles):
    """[h1, h2, c, x, s1, s2] -> the registration's blocks."""
    return {"H": (roles[0], roles[1]), "C": (roles[2],), "S": (roles[4], roles[5]), "R": (roles[3],)}


class Point:
    """One exported point at one quadrature order: chart, metric, potential and normal coordinates on the grid."""

    def __init__(self, X, order):
        import helike_positive_cone as pc
        import helike_condensate_pt as cp
        self.X, self.order = X, order
        self.chart = pc.ConeChart(X["G0"], X["M"])
        if not (np.allclose(self.chart.u0, X["u0"], rtol=0, atol=1e-12) and np.allclose(self.chart.S, X["S"], rtol=1e-12, atol=1e-14)):
            raise ValueError("the Python chart does not reproduce the exported chart")
        self.cp = cp
        metric, V = pc.laurent_model(X["s"], X["t"], math.sqrt(X["g2"]))
        self.Vterms = V
        self.x0 = np.array([X["G0"][i, j] for i, j in cp.GKEYS])
        self.invM = np.linalg.inv(X["M"])
        W, x = grid_nodes(order, X["w"])
        self.core = Core(W, x, X["w"], self.metric, self.chart.ell, self.potential, self.ycoords, self.dycoords)
        self.blocks = block_map(X["roles"])
        self.A = frame_matrix(self.metric(np.zeros((1, 6)))[0], self.blocks)

    def factor(self, z):
        u = self.chart.u0 + z @ self.chart.S.T
        r = np.exp(u[:, :3])
        return u, r

    def gram(self, z):
        u, r = self.factor(z)
        a, b, c = u[:, 3], u[:, 4], u[:, 5]
        return np.column_stack((r[:, 0] ** 2, a * a + r[:, 1] ** 2, b * b + c * c + r[:, 2] ** 2,
                                r[:, 0] * a, r[:, 0] * b, a * b + r[:, 1] * c)), u, r

    def jac(self, z):
        """dx/dz (n, 6, 6): x the six Gram entries."""
        _, u, r = self.gram(z)
        r0, r1, r2 = r[:, 0], r[:, 1], r[:, 2]
        a, b, c = u[:, 3], u[:, 4], u[:, 5]
        n = len(z); J = np.zeros((n, 6, 6))
        J[:, 0, 0] = 2 * r0 * r0
        J[:, 1, 1] = 2 * r1 * r1; J[:, 1, 3] = 2 * a
        J[:, 2, 2] = 2 * r2 * r2; J[:, 2, 4] = 2 * b; J[:, 2, 5] = 2 * c
        J[:, 3, 0] = r0 * a; J[:, 3, 3] = r0
        J[:, 4, 0] = r0 * b; J[:, 4, 4] = r0
        J[:, 5, 1] = r1 * c; J[:, 5, 3] = b; J[:, 5, 4] = a; J[:, 5, 5] = r1
        return J @ self.chart.S

    def metric(self, z):
        x, _, _ = self.gram(z)
        A = np.einsum("ijm,nm->nij", self.cp.C_A, x)
        Ji = np.linalg.inv(self.jac(z))
        return Ji @ A @ np.transpose(Ji, (0, 2, 1))

    def potential(self, z):
        u, r = self.factor(z)
        var = np.column_stack((r, u[:, 3:]))
        V = np.zeros(len(z))
        for key, c in self.Vterms.items():
            V += c * np.prod(var ** np.array(key), axis=1)
        return V - self.X["shift"]

    def ycoords(self, z):
        x, _, _ = self.gram(z)
        return (x - self.x0) @ self.invM.T

    def dycoords(self, z):
        return np.einsum("ij,njk->nik", self.invM, self.jac(z))

    def values(self, cols):
        return grid_values(self.X["basis"], self.X["coeff"][:, cols], self.order, self.X["mq"])


def analyse_point(X, order, carriers):
    """Shares, the (★) identity, alignment and shape price at one point and order. carriers: {name: (levels, block)}.
    States are evaluated one at a time (the ground state is kept)."""
    P = Point(X, order)
    p0, dp0 = P.values([0])
    p0, dp0 = p0[:, 0], dp0[:, :, 0]
    if np.sum(P.core.W * p0) < 0:
        p0, dp0 = -p0, -dp0                                   # the ground state's sign is a convention
    E0 = float(X["levels"][0])
    out = {"order": order, "E0": E0, "frame": "G(0)^{-1/2}, block-diagonal check passed"}
    for name, (lev, block) in carriers.items():
        parts, al = [], []
        for l in lev:
            p, dp = P.values([l])
            parts.append(P.core.analyse(p0, dp0, p[:, 0], dp[:, :, 0], E0, P.blocks, P.A))
            al.append(P.core.alignment(p0, p[:, 0], list(P.blocks[block])))
            del p, dp
        D = sum(r["D"] for r in parts)
        R = sum(r["R"] for r in parts)
        # amendment 5: the shares combine the partners' in-cut energies
        def combine(key):
            Dk = sum(r["D_by_cut"][key] for r in parts)
            return {b: sum(r["shares_by_cut"][key][b] * r["D_by_cut"][key] for r in parts) / Dk for b in BLOCKS}, Dk
        cut_keys = list(parts[0]["shares_by_cut"])
        by_cut = {k: combine(k) for k in cut_keys}
        shares = by_cut[cut_keys[0]][0]
        ritz = sum(float(X["levels"][l]) - E0 for l in lev)
        ident = D + R
        out[name] = dict(levels=list(lev), block=block, D=D, residual_term=R, identity=ident, ritz_excitation=ritz,
                         identity_rel_error=abs(ident - ritz) / ritz, residual_share=abs(R) / ritz,
                         quadrature_rayleigh=[r["rayleigh"] for r in parts], norms=[r["norm"] for r in parts],
                         shares=shares, own_share=shares[block],
                         shares_by_cut={k: v[0] for k, v in by_cut.items()},
                         excluded_fraction={k: 1 - v[1] / D for k, v in by_cut.items()},
                         cut_seam=max(abs(by_cut[cut_keys[0]][0][b] - by_cut[cut_keys[1]][0][b]) for b in BLOCKS),
                         share_sum=sum(r["share_sum"] * r["D_by_cut"][cut_keys[0]] for r in parts) / by_cut[cut_keys[0]][1],
                         raw=[r["raw"] for r in parts], alignment_per_level=al, alignment=float(np.mean(al)),
                         nonpositive_weight=max(r["nonpositive_weight"] for r in parts))
    for block in ("H", "C"):
        out[f"shape_price_{block}"] = P.core.shape_price(p0, dp0, list(P.blocks[block]), E0)
    return out
