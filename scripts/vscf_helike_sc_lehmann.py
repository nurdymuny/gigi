"""Lehmann steps with block-adapted (VSCF-type) trial functions on base B3' (registration section 13).
Floating point; not a bound.

Section 12 showed the binding constraint is trial-function quality near s = 1: with products of the isotropic base
eigenfunctions, residuals reach ~0.1 against a margin of ~0.03. Here the trial functions at each s are Ritz vectors of
H_s in a product basis of BLOCK EIGENFUNCTIONS OF THE TRUE BLOCK OPERATORS

    h_u(s) = h_u^0 + s (P(u, 0) - 1/2 ws |u|^2),     h_v(s) = h_v^0 + s (P(0, v) - 1/2 wt |v|^2),

each diagonalised in the full span of its block's base eigenfunctions (so the anisotropic frequencies and each block's
own cubic and quartic terms are built in; only the mixed u-v terms are left to the product Galerkin).

Every trial function is a finite combination of products phi_i(u) psi_j(v) of base eigenfunctions, which satisfy the
Robin condition, so it lies in D(H_s). A trial vector is held as a coefficient tensor C (n_u x n_v), and
    <u, H_0 w> uses (E_u + E_v) C,    <u, D w> = sum_alpha <C, U^alpha C' Vt_alpha^T>,
    <D u, D w> = <u, D^2 w> = sum_gamma <C, U^gamma C' W_gamma^T>     (moments of the degree-8 polynomial D^2),
so A_0, A_1, A_2 are exact polynomial moments, as in section 9.2. The registered ceiling of 2,000 functions applies to
the Galerkin trial space (VSCF products); the representation span is the full tensor product of the two block spans.

The homotopy driver is replan_helike_sc_lehmann_path.run with the real Lehmann step replaced by vscf_lehmann
(the planner still uses the cached isotropic-basis Ritz curves as its proxy).

usage: python scripts/vscf_helike_sc_lehmann.py [--controls 0.004,0.002] [--cache DIR] [--out PREFIX] [--check]
"""
import argparse
import json
import math
import time

import numpy as np
from scipy.linalg import eigh, cholesky

import dryrun_helike_sc_lehmann_homotopy as dr
import estimate_helike_sc_gbase as gb
import replan_helike_sc_lehmann_path as rp
from plan_helike_sc_lehmann_path import S_GRID, build

NU_TARGET, NV_TARGET = 700, 80        # block representation sizes (cut at level boundaries)
M_TRIAL = 2000                        # Galerkin trial space (registered ceiling)


def block_cut(B, n):
    E = B.E
    if len(E) <= n:
        return list(range(len(E)))
    c = E[n]
    return [i for i in range(len(E)) if E[i] < c - 1e-12]


FOLD = None                                               # (a, b): if set, overrides the uniform 2 C3 L reduction


class VSCF:
    def __init__(self, g2, E0rel, log):
        t0 = time.time(); rng = np.random.default_rng(2)
        m = gb.setup(g2); self.rhob = E0rel + gb.RHO_B_REL
        P, _ = dr.fit_quartic(m, rng)
        C3 = max(gb.cubic_constant(m, np.random.default_rng(2)), dr.cubic_constant_poly(P, rng))
        vlo, _, beta = gb.curvature(m, math.hypot(dr.LU, dr.LV), np.random.default_rng(2), n=300)
        w2 = m["w2"]; Lt = math.hypot(dr.LU, dr.LV)
        ws, wt = w2[:3].min() - 2 * C3 * Lt, w2[3:].min() - 2 * C3 * Lt
        if FOLD is not None:                                  # per-block reductions (a, b) of the ray-aware fold
            ws, wt = w2[:3].min() - FOLD[0], w2[3:].min() - FOLD[1]
        D = dict(P)
        for k in range(6):
            e = tuple(2 if i == k else 0 for i in range(6))
            D[e] = D.get(e, 0.0) - 0.5 * (ws if k < 3 else wt)
        ecut = self.rhob + 12.0
        ub = dr.Block(ws, dr.LU, beta, vlo, ecut); vb = dr.Block(wt, dr.LV, beta, 0.0, ecut)
        ui = block_cut(ub, NU_TARGET); vi = block_cut(vb, NV_TARGET)
        self.Eu, self.Ev = ub.E[ui], vb.E[vi]
        self.U = ub.monomials(ui); self.V = vb.monomials(vi)
        z3 = (0, 0, 0)
        self.Du = {e[:3]: c for e, c in D.items() if e[3:] == z3}                     # includes the constant
        self.Dv = {e[3:]: c for e, c in D.items() if e[:3] == z3 and e[3:] != z3}
        mix = {e: c for e, c in D.items() if e[:3] != z3 and e[3:] != z3}
        self.Dgroups = self._group(D)                       # all of D, grouped by u-monomial
        self.Mgroups = self._group(mix)                     # mixed part only
        self.D2groups = self._group(dr.pmul(D, D))
        log(f"  VSCF representation: soft {len(ui)} (E_u < {ub.E[len(ui)] if len(ui) < len(ub.E) else ecut:.3f}), "
            f"stiff {len(vi)}; tensor {len(ui) * len(vi)}; mixed u-groups {len(self.Mgroups)}; "
            f"D^2 u-groups {len(self.D2groups)}; built in {time.time() - t0:.0f} s")

    def _group(self, poly):
        g = {}
        for e, c in poly.items():
            g.setdefault(e[:3], {})[e[3:]] = c
        out = []
        for a, inner in g.items():
            out.append((a, sum(c * self.V[b] for b, c in inner.items())))
        return out

    def apply(self, groups, C):
        """sum_alpha U^alpha C Vt_alpha^T for a stack C of shape (n, n_u, n_v)."""
        out = np.zeros_like(C)
        for a, Vt in groups:
            out += np.einsum("ij,njk->nik", self.U[a], C) @ Vt.T
        return out

    def trial(self, s, N):
        hu = np.diag(self.Eu) + s * sum(c * self.U[a] for a, c in self.Du.items())
        hv = np.diag(self.Ev) + s * sum(c * self.V[b] for b, c in self.Dv.items())
        eu, Xu = np.linalg.eigh(0.5 * (hu + hu.T)); ev, Xv = np.linalg.eigh(0.5 * (hv + hv.T))
        pairs = sorted(((a + b, i, j) for i, a in enumerate(eu) for j, b in enumerate(ev)), key=lambda x: x[0])
        pairs = pairs[:M_TRIAL]
        A = np.array([p[1] for p in pairs]); B = np.array([p[2] for p in pairs])
        H = np.diag([p[0] for p in pairs])
        for a, Vt in self.Mgroups:
            Ua = Xu.T @ self.U[a] @ Xu; Va = Xv.T @ Vt @ Xv
            H += s * Ua[np.ix_(A, A)] * Va[np.ix_(B, B)]
        mu, Y = eigh(0.5 * (H + H.T), subset_by_index=[0, N - 1])
        Z = np.zeros((N, len(eu), len(ev)))
        Z[:, A, B] = Y.T
        C = np.einsum("ia,nab,jb->nij", Xu, Z, Xv, optimize=True)
        return C

    def lehmann(self, s, rho, N):
        C = self.trial(s, N)
        Esum = self.Eu[:, None] + self.Ev[None, :]
        H0C = Esum[None] * C
        DC = self.apply(self.Dgroups, C)
        D2C = self.apply(self.D2groups, C)
        f = lambda X, Y: np.einsum("pij,qij->pq", X, Y)
        A0 = f(C, C)
        A1 = f(C, H0C) + s * f(C, DC)
        A2 = f(H0C, H0C) + s * (f(H0C, DC) + f(DC, H0C)) + s * s * f(C, D2C)
        A0, A1, A2 = [0.5 * (X + X.T) for X in (A0, A1, A2)]
        mu = np.linalg.eigvalsh(A1) if False else eigh(A1, A0, eigvals_only=True)
        Bm = A2 - 2 * rho * A1 + rho * rho * A0
        cholesky(Bm)
        tau = eigh(A1 - rho * A0, Bm, eigvals_only=True)
        lo = np.full(N, -np.inf)
        for i, t in enumerate(tau, start=1):
            if t < 0:
                lo[N - i] = rho + 1 / t
        nrm = np.diag(A0)
        rq = np.diag(A1) / nrm
        resid = np.sqrt(np.maximum(np.diag(A2) / nrm - rq ** 2, 0))
        return lo, mu, resid, int((tau < 0).sum())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--controls", default="0.004,0.002")
    ap.add_argument("--cache", default=None)
    ap.add_argument("--out", default=None)
    ap.add_argument("--check", action="store_true", help="component check only: residuals at s = 1, no homotopy")
    a = ap.parse_args()
    lines, res = [], []
    def log(s):
        print(s, flush=True); lines.append(s)
    log("VSCF-trial Lehmann homotopy on B3' (L_u = 5, L_v = 2); floating point, not a bound")
    for g2, E0 in gb.CONTROLS:
        if str(g2) not in a.controls.split(","):
            continue
        log(f"\n=== control g2 = {g2}")
        vs = VSCF(g2, E0, log)
        if a.check:
            C = vs.trial(1.0, 13)
            lo, mu, resid, _ = vs.lehmann(1.0, vs.rhob + 1.0, 13)
            log("  s = 1 Ritz - E0: " + " ".join(f"{x - E0:.4f}" for x in mu))
            log("  s = 1 residuals: " + " ".join(f"{x:.1e}" for x in resid))
            continue
        E, Dm, D2m, MU = build(g2, E0, a.cache, log)
        out = dict(g2=g2, runs=[])
        orig = dr.lehmann
        dr.lehmann = lambda E_, Dm_, D2m_, s, rho, N: vs.lehmann(s, rho, N)
        try:
            for loss in rp.LOSSES:
                log(f"  [planning LOSS = {loss}]")
                verdict, trail = rp.run(E, Dm, D2m, MU, vs.rhob, loss, log)
                log(f"  [planning LOSS = {loss}] {verdict}")
                out["runs"].append(dict(loss=loss, verdict=verdict, trail=trail))
        finally:
            dr.lehmann = orig
        res.append(out)
        if a.out:
            json.dump(res, open(a.out + ".json", "w"), indent=1)
            open(a.out + ".log", "w").write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
