"""Floating-point DRY RUN of the Lehmann homotopy for the inside base problem of S_C.

Registered in docs/research/helike-sc-lehmann-goerisch-registration-2026-11-25.md, sections 8.5 and 9.
Not a bound and not a proof: every number here is floating point.

Inside model at a control, on the product tube B_u(L_u) x B_v(L_v) in the six transverse normal coordinates
z = (u, v) (u = the three soft modes, v = the three stiff modes), with natural (Neumann) boundary conditions and the
Robin term -beta/2 on each block's sphere, exactly as in estimate_helike_sc_gbase.product_base:

    H_s = -1/2 Lap + W0(z) + vJ + s D(z),    D = P - W0,    0 <= s <= 1,

where P(z) = V(x0 + N z) - V_bg is the exact quartic potential on the normal slice and
W0 = 1/2 ws |u|^2 + 1/2 wt |v|^2 is the reduced block-isotropic oscillator of B3'. H_0 is the separable explicit base;
H_1 is the inside model. D >= 0 on the tube, so every eigenvalue is non-decreasing in s.

Basis: products of the exact block eigenfunctions of H_0 (radial Galerkin in r^l P_k(2r/L - 1) times real spherical
harmonics), the lowest at most 2,000 of them, cut at a level boundary. They satisfy the Robin condition, so they lie
in D(H_s) for every s. Because H_0 phi_I = E_I phi_I and D is a polynomial,
    <H_s phi_I, H_s phi_J> = E_I E_J delta_IJ + s (E_I + E_J) D_IJ + s^2 (D^2)_IJ
with (D^2)_IJ the moment of the degree-8 polynomial D^2, not the product of truncated matrices.

Lehmann step at (s, rho, N), rho <= lambda_{N+1}(H_s): trial functions = the N lowest Ritz vectors of H_s;
pencil (A1 - rho A0) x = tau (A2 - 2 rho A1 + rho^2 A0) x; lambda_{N+1-i} >= rho + 1/tau_i for tau_i < 0.

Homotopy (registered in section 9): start from the exact base spectrum at s = 0; after each step, the next
(N', rho') = (j - 1, l_j) is the candidate j >= 11 that lets the next step reach the largest s, where a step to s' is
admissible when the Ritz value mu_{j-1}(H_s') <= l_j - DELTA. PASS as soon as a step gives l_11 >= rho_b (then
lambda_11(H_1) >= lambda_11(H_s) >= rho_b by monotonicity). FAIL if 40 steps are used, if no candidate advances, or if
s = 1 is reached without l_11 >= rho_b.

usage: python scripts/dryrun_helike_sc_lehmann_homotopy.py [--delta 0.02] [--controls 0.004,0.002]
"""
import argparse
import itertools
import json
import math
import time

import numpy as np
from numpy.polynomial import legendre as npl
from scipy.linalg import eigh, cholesky
from scipy.optimize import minimize
from scipy.special import sph_harm_y

import estimate_helike_sc_gbase as gb

LU, LV = 5.0, 2.0                 # amended product tube (section 8.4)
NBASIS = 2000                     # registered ceiling on basis functions
MAX_STEPS = 40                    # registered ceiling on homotopy steps
TARGET_INDEX = 11                 # N + 1 = 11 (section 8.1)
KMAX = 200                        # Ritz values tracked on the s-grid
S_GRID = np.linspace(0.0, 1.0, 401)
K_RAD = 44                        # radial Galerkin size per angular momentum
NQ_R = 220                        # common radial Gauss nodes per block


# ---------------------------------------------------------------- polynomials
def exponents(n, dmax):
    out = []
    for d in range(dmax + 1):
        for e in itertools.product(range(d + 1), repeat=n):
            if sum(e) == d:
                out.append(e)
    return out


E6_4 = exponents(6, 4)


def fit_quartic(m, rng):
    """Exact quartic P(z) = V(x0 + N z) - V0 by least squares on 6000 points (P is a polynomial of degree 4)."""
    def sample(k):
        z = rng.normal(size=(k, 6))
        return z / np.linalg.norm(z, axis=1)[:, None] * rng.uniform(0, 6, (k, 1))
    Z = sample(6000)
    y = np.array([m["f"](m["x0"] + m["N"] @ z) for z in Z]) - m["V0"]
    X = np.prod(Z[:, None, :] ** np.array(E6_4)[None], axis=2)
    c, *_ = np.linalg.lstsq(X, y, rcond=None)
    Zt = sample(2000)
    yt = np.array([m["f"](m["x0"] + m["N"] @ z) for z in Zt]) - m["V0"]
    err = np.max(np.abs(np.prod(Zt[:, None, :] ** np.array(E6_4)[None], axis=2) @ c - yt))
    return {e: v for e, v in zip(E6_4, c)}, err


def peval(p, Z):
    E = np.array(list(p.keys())); c = np.array(list(p.values()))
    return np.prod(Z[:, None, :] ** E[None], axis=2) @ c


def pmul(p, q):
    out = {}
    for e1, c1 in p.items():
        for e2, c2 in q.items():
            e = tuple(a + b for a, b in zip(e1, e2))
            out[e] = out.get(e, 0.0) + c1 * c2
    return out


def cubic_constant_poly(p, rng, starts=400):
    """C3 = max over the unit sphere of |P3|, from the fitted cubic part."""
    p3 = {e: c for e, c in p.items() if sum(e) == 3}
    E = np.array(list(p3.keys())); c = np.array(list(p3.values()))
    f = lambda y: -abs(np.prod((y / np.linalg.norm(y)) ** E, axis=1) @ c)
    return max(-minimize(f, rng.normal(size=6), method="BFGS").fun for _ in range(starts))


def d_min_on_tube(D, rng, k=400_000):
    u = rng.normal(size=(k, 3)); u *= (LU * rng.uniform(0, 1, (k, 1)) ** (1 / 3)) / np.linalg.norm(u, axis=1)[:, None]
    v = rng.normal(size=(k, 3)); v *= (LV * rng.uniform(0, 1, (k, 1)) ** (1 / 3)) / np.linalg.norm(v, axis=1)[:, None]
    Z = np.hstack([u, v]); val = peval(D, Z)
    lo = val.min()
    for z0 in Z[np.argsort(val)[:20]]:
        def proj(z):
            z = z.copy()
            for sl, L in ((slice(0, 3), LU), (slice(3, 6), LV)):
                n = np.linalg.norm(z[sl])
                if n > L:
                    z[sl] *= L / n
            return z
        r = minimize(lambda z: peval(D, proj(z)[None])[0], z0, method="Nelder-Mead",
                     options={"maxiter": 4000, "xatol": 1e-10, "fatol": 1e-14})
        lo = min(lo, r.fun)
    return lo


# ---------------------------------------------------------------- one block: exact base eigenfunctions
def radial_levels(w2r, L, beta, shift, ecut, K=K_RAD):
    """Eigenpairs below ecut of -1/2 Lap + 1/2 w2r r^2 + shift on the 3-ball, Robin -beta/2 on the sphere.
    Returns list of (l, E, values on the common radial grid) and the grid."""
    xr, wr0 = npl.leggauss(NQ_R); rq = (xr + 1) * L / 2; wq = wr0 * L / 2
    out, l = [], 0
    eye = np.eye(K)
    dcoef = [npl.legder(eye[k]) for k in range(K)]
    while True:
        nq = l + K + 12
        x, w = npl.leggauss(nq); r = (x + 1) * L / 2; wr = w * L / 2
        P = npl.legvander(x, K - 1)
        dP = np.column_stack([npl.legval(x, dcoef[k]) for k in range(K)]) * (2 / L)
        rl = (r / L) ** l
        b = rl[:, None] * P
        db = rl[:, None] * dP + ((l / L) * (r / L) ** (l - 1))[:, None] * P if l > 0 else dP
        M = (b * (wr * r * r)[:, None]).T @ b
        A = 0.5 * (db * (wr * r * r)[:, None]).T @ db \
            + (b * (wr * (r * r * (0.5 * w2r * r * r + shift) + 0.5 * l * (l + 1)))[:, None]).T @ b
        bL = np.ones(K)
        A -= 0.5 * beta * L * L * np.outer(bL, bL)
        sM, UM = np.linalg.eigh(M)
        keep = sM > 1e-13 * sM.max()
        X = UM[:, keep] / np.sqrt(sM[keep])
        e, y = np.linalg.eigh(X.T @ A @ X)
        C = X @ y
        sel = e < ecut
        if not sel.any():
            break
        bq = ((rq / L) ** l)[:, None] * npl.legvander(xr, K - 1)
        for E, c in zip(e[sel], C[:, sel].T):
            out.append((l, float(E), bq @ c))
        l += 1
    return out, rq, wq


def real_sph(lmax, th, ph):
    rows = []
    for l in range(lmax + 1):
        for mm in range(-l, l + 1):
            if mm == 0:
                rows.append(sph_harm_y(l, 0, th, ph).real)
            elif mm > 0:
                rows.append(math.sqrt(2) * (-1) ** mm * sph_harm_y(l, mm, th, ph).real)
            else:
                rows.append(math.sqrt(2) * (-1) ** mm * sph_harm_y(l, -mm, th, ph).imag)
    return np.array(rows)


class Block:
    """Exact base eigenfunctions of one block and the matrices of the monomials x^alpha, |alpha| <= 8."""

    def __init__(self, w2r, L, beta, shift, ecut, K=K_RAD):
        self.rad, rq, wq = radial_levels(w2r, L, beta, shift, ecut, K)
        self.funcs = []   # (radial index, l, m, E)
        for i, (l, E, _) in enumerate(self.rad):
            for mm in range(-l, l + 1):
                self.funcs.append((i, l, mm, E))
        order = np.argsort([f[3] for f in self.funcs], kind="stable")
        self.funcs = [self.funcs[i] for i in order]
        self.E = np.array([f[3] for f in self.funcs])
        self.rq, self.wq = rq, wq
        self.lmax = max(l for l, _, _ in self.rad)

    def monomials(self, idx, dmax=8):
        """Matrices <phi_i, x^alpha phi_j> for the functions idx (list of positions in self.funcs)."""
        lmax = max(self.funcs[i][1] for i in idx)
        nth, nph = lmax + dmax + 6, 2 * lmax + dmax + 8
        ct, wct = npl.leggauss(nth); th = np.arccos(ct); ph = 2 * np.pi * np.arange(nph) / nph
        TH, PH = np.meshgrid(th, ph, indexing="ij"); TH, PH = TH.ravel(), PH.ravel()
        wa = (wct[:, None] * np.full(nph, 2 * np.pi / nph)[None]).ravel()
        Y = real_sph(lmax, TH, PH)
        xh = np.stack([np.sin(TH) * np.cos(PH), np.sin(TH) * np.sin(PH), np.cos(TH)])
        ri = np.array([self.funcs[i][0] for i in idx])
        ai = np.array([self.funcs[i][1] ** 2 + self.funcs[i][1] + self.funcs[i][2] for i in idx])
        urad = np.unique(ri); pos = {r: k for k, r in enumerate(urad)}; rk = np.array([pos[r] for r in ri])
        R = np.array([self.rad[r][2] for r in urad])
        rad = [(R * (self.wq * self.rq ** (2 + k))[None]) @ R.T for k in range(dmax + 1)]
        mats = {}
        for a in exponents(3, dmax):
            ang = (Y * (wa * np.prod(xh ** np.array(a)[:, None], axis=0))[None]) @ Y.T
            mats[a] = rad[sum(a)][np.ix_(rk, rk)] * ang[np.ix_(ai, ai)]
        return mats


# ---------------------------------------------------------------- product-basis operators
def product_operator(poly, ub, vb, Ui, Vi, Umats, Vmats):
    """Matrix of the multiplication by poly(u, v) in the product basis (Ui[k], Vi[k])."""
    groups = {}
    for e, c in poly.items():
        groups.setdefault(e[:3], {})[e[3:]] = c
    n = len(Ui); M = np.zeros((n, n))
    for a, inner in groups.items():
        Vt = sum(c * Vmats[b] for b, c in inner.items())
        M += Umats[a][np.ix_(Ui, Ui)] * Vt[np.ix_(Vi, Vi)]
    return M


def lehmann(H0d, Dm, D2m, s, rho, N):
    """Lower bounds l_1..l_N (index k-1 holds l_k; -inf where none) and diagnostics."""
    H = np.diag(H0d) + s * Dm
    mu, C = eigh(H, subset_by_index=[0, N - 1])
    A2 = C.T @ ((H0d ** 2)[:, None] * C + s * (H0d[:, None] * (Dm @ C) + Dm @ (H0d[:, None] * C)) + s * s * (D2m @ C))
    A2 = 0.5 * (A2 + A2.T)
    A1 = np.diag(mu)
    B = A2 - 2 * rho * A1 + rho * rho * np.eye(N)
    cholesky(B)   # raises unless positive definite
    tau = eigh(A1 - rho * np.eye(N), B, eigvals_only=True)
    lo = np.full(N, -np.inf)
    for i, t in enumerate(tau, start=1):
        if t < 0:
            lo[N - i] = rho + 1 / t
    resid = np.sqrt(np.maximum(np.diag(A2) - mu ** 2, 0))
    return lo, mu, resid, int((tau < 0).sum())


# ---------------------------------------------------------------- one control
def run_control(g2, E0rel, delta, log):
    t0 = time.time(); rng = np.random.default_rng(2)
    m = gb.setup(g2); rhob = E0rel + gb.RHO_B_REL
    P, fit_err = fit_quartic(m, rng)
    C3_ref = gb.cubic_constant(m, np.random.default_rng(2))
    C3_poly = cubic_constant_poly(P, rng)
    C3 = max(C3_ref, C3_poly)
    vlo, vhi, beta = gb.curvature(m, math.hypot(LU, LV), np.random.default_rng(2), n=300)
    w2 = m["w2"]; Lt = math.hypot(LU, LV)
    ws, wt = w2[:3].min() - 2 * C3 * Lt, w2[3:].min() - 2 * C3 * Lt
    log(f"\n=== control g2 = {g2}: E0 - V_bg = {E0rel}, rho_b - V_bg = {rhob:.4f}")
    log(f"  quartic fit: max error on fresh points {fit_err:.2e}; linear terms max "
        f"{max(abs(c) for e, c in P.items() if sum(e) == 1):.2e}")
    log(f"  w2 = {np.round(w2, 5)}; C3 (script) = {C3_ref:.5f}, C3 (fitted cubic) = {C3_poly:.5f}, used {C3:.5f}")
    log(f"  V_J in [{vlo:.2e}, {vhi:.2e}], beta = {beta:.4f}; reduced w2 soft {ws:.4f}, stiff {wt:.4f}")
    D = dict(P)
    for k in range(6):
        e = tuple(2 if i == k else 0 for i in range(6))
        D[e] = D.get(e, 0.0) - 0.5 * (ws if k < 3 else wt)
    dmin = d_min_on_tube(D, rng)
    log(f"  min of D = P - W0 on the product tube (sampled + local search): {dmin:.3e}")

    # block bases: generous cut, then the product basis cut at a level boundary
    ecut = rhob + 5.0
    ub = Block(ws, LU, beta, vlo, ecut); vb = Block(wt, LV, beta, 0.0, ecut)
    pairs = sorted(((a + b, i, j) for i, a in enumerate(ub.E) for j, b in enumerate(vb.E)), key=lambda x: x[0])
    cut = pairs[NBASIS][0]
    pairs = [p for p in pairs if p[0] < cut - 1e-12]
    assert cut < ecut + min(ub.E.min(), vb.E.min())
    E = np.array([p[0] for p in pairs]); I = [p[1] for p in pairs]; J = [p[2] for p in pairs]
    uidx = sorted(set(I)); vidx = sorted(set(J))
    upos = {k: n for n, k in enumerate(uidx)}; vpos = {k: n for n, k in enumerate(vidx)}
    Ui = np.array([upos[i] for i in I]); Vi = np.array([vpos[j] for j in J])
    log(f"  basis: {len(E)} product functions (soft {len(uidx)} with l <= {max(ub.funcs[i][1] for i in uidx)}, "
        f"stiff {len(vidx)}), cut at E = {cut:.4f} (rho_b + {cut - rhob:.3f}); base lambda_1 = {E[0]:.4f}, "
        f"lambda_11 = {E[10]:.4f}, levels below rho_b = {(E < rhob).sum()}")
    ub2 = Block(ws, LU, beta, vlo, ecut, K=K_RAD + 12)
    log(f"  radial convergence (K = {K_RAD} vs {K_RAD + 12}): soft max |dE| over levels < cut = "
        f"{np.max(np.abs(ub.E[ub.E < cut][:len(ub2.E)] - ub2.E[:len(ub.E[ub.E < cut])])):.1e}")
    Umats = ub.monomials(uidx); Vmats = vb.monomials(vidx)
    log(f"  block orthonormality: soft {np.max(np.abs(Umats[(0, 0, 0)] - np.eye(len(uidx)))):.1e}, "
        f"stiff {np.max(np.abs(Vmats[(0, 0, 0)] - np.eye(len(vidx)))):.1e}")
    Dm = product_operator(D, ub, vb, Ui, Vi, Umats, Vmats)
    D2m = product_operator(pmul(D, D), ub, vb, Ui, Vi, Umats, Vmats)
    Dm, D2m = 0.5 * (Dm + Dm.T), 0.5 * (D2m + D2m.T)
    log(f"  operators built ({time.time() - t0:.0f} s); min eig of the D matrix {np.linalg.eigvalsh(Dm)[0]:.2e}")

    # Ritz values on the s-grid (upper bounds for the lookahead)
    MU = np.array([eigh(np.diag(E) + s * Dm, eigvals_only=True, subset_by_index=[0, KMAX - 1]) for s in S_GRID])
    log(f"  Ritz at s = 1 minus E0: " + " ".join(f"{x - E0rel:.4f}" for x in MU[-1, :13]))
    log(f"  Ritz mu_11(1) - rho_b = {MU[-1, 10] - rhob:+.4f} (the margin the homotopy must keep)")

    def reach(j, rho_new, s_cur):
        ok = (S_GRID > s_cur + 1e-12) & (MU[:, j - 2] <= rho_new - delta)
        if not ok.any():
            return s_cur
        # first grid point after s_cur where the condition fails bounds the reach (mu is non-decreasing)
        after = np.where(S_GRID > s_cur + 1e-12)[0]
        last = s_cur
        for k in after:
            if MU[k, j - 2] <= rho_new - delta:
                last = S_GRID[k]
            else:
                break
        return last

    s, steps, trail = 0.0, 0, []
    lo_known = E[:KMAX].copy()          # exact at s = 0
    N = None; rho = None; verdict = None
    while True:
        cands = []
        top = KMAX if N is None else N + 1
        for j in range(TARGET_INDEX, top + 1):
            rnew = lo_known[j - 1] if j - 1 < len(lo_known) else (rho if j == N + 1 else -np.inf)
            if N is not None and j == N + 1:
                rnew = rho
            if not np.isfinite(rnew):
                continue
            cands.append((reach(j, rnew, s), j, rnew))
        if not cands:
            verdict = "FAIL (no admissible candidate)"; break
        best = max(cands, key=lambda c: (c[0], c[2]))
        s_next, j, rnew = best
        if s_next <= s + 1e-12:
            verdict = f"FAIL (stuck at s = {s:.4f})"; break
        if steps >= MAX_STEPS:
            verdict = "FAIL (40 steps used)"; break
        N, rho = j - 1, rnew
        lo, mu, resid, nneg = lehmann(E, Dm, D2m, s_next, rho, N)
        steps += 1; s = s_next; lo_known = lo
        l11 = lo[TARGET_INDEX - 1] if N >= TARGET_INDEX else -np.inf
        trail.append(dict(step=steps, s=float(s), N=N, rho_rel=float(rho - E0rel), l11_rel=float(l11 - E0rel),
                          mu11_rel=float(mu[TARGET_INDEX - 1] - E0rel) if N >= TARGET_INDEX else None,
                          max_resid=float(resid.max()), n_neg_tau=nneg))
        log(f"  step {steps:2d}: s = {s:.4f}  N = {N:3d}  rho - E0 = {rho - E0rel:.4f}  "
            f"l_11 - E0 = {l11 - E0rel:+.4f}  (mu_11 - E0 = {mu[TARGET_INDEX - 1] - E0rel:.4f})  "
            f"max resid {resid.max():.1e}  neg tau {nneg}")
        if l11 >= rhob:
            verdict = f"PASS (lambda_11 >= rho_b at s = {s:.4f}, step {steps})"; break
        if s >= 1.0 - 1e-12 and N <= TARGET_INDEX:
            verdict = "FAIL (s = 1 reached, l_11 < rho_b)"; break
    log(f"  VERDICT g2 = {g2}: {verdict}  [{steps} steps, {len(E)} basis functions, {time.time() - t0:.0f} s]")
    return dict(g2=g2, E0rel=E0rel, rho_b_rel=float(rhob - E0rel), C3=C3, C3_script=C3_ref, C3_poly=C3_poly,
                beta=beta, vJ=[vlo, vhi], ws=ws, wt=wt, d_min=dmin, fit_err=fit_err, n_basis=len(E),
                cut_rel=float(cut - E0rel), ritz_s1_rel=[float(x - E0rel) for x in MU[-1, :13]],
                steps=trail, verdict=verdict, delta=delta)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--delta", type=float, default=0.02)
    ap.add_argument("--controls", default="0.004,0.002")
    ap.add_argument("--out", default=None)
    a = ap.parse_args()
    lines = []
    def log(s):
        print(s, flush=True); lines.append(s)
    log(f"Lehmann homotopy dry run on B3' (L_u = {LU}, L_v = {LV}), DELTA = {a.delta}; floating point, not a bound")
    res = [run_control(g2, E0, a.delta, log) for g2, E0 in gb.CONTROLS if str(g2) in a.controls.split(",")]
    if a.out:
        json.dump(res, open(a.out + ".json", "w"), indent=1)
        open(a.out + ".log", "w").write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
