"""The exact edge of the twist's region in the circle-symmetric family (21 November 2026).

Family: SU(2) on (t, 1, 1), m_0 = x e_0, m_1 = y e_1, m_2 = y e_2. The charge q = m + c_f + c_l is conserved for every
member, so each sector of H + G G^* splits into blocks of <= 9 states with closed-form rows (as in
scripts/ricci_su2_block_rows_symbolic.py, now symbolic in x, y, t):
  D(m, a) = (w_a - 2m/sqrt t)^2 + 4(l(l+1) - m^2) + C_aa,  C = -(Omega_1^2 + Omega_2^2) + R0,  R0 = Ric (x) I - 2[F, .],
  ladder couplings 2i sqrt((l-m)(l+m+1)) (Omega_-)_{a'a} to m+1 and 2i sqrt((l+m)(l-m+1)) (Omega_+)_{a'a} to m-1.
The twist is the 1 x 1 block of charge l + 2, entry lambda = (x + (j+2)/sqrt t)^2 + 2 sqrt(t) x - y^2.
Edge (operator H + GG^*): the x where lambda_j = lowest eigenvalue of the first non-twist block to reach it, i.e. a root
  of det(B_{j,q}(x, y, t) - lambda_j) = 0, found by a sign-change scan and brentq on
  phi_A(x) = min_{j <= J, q} [lambda_min(B_{j,q}) - lambda_j].
Edge (gauge-orthogonal, the floor tests' definition): the same with phi_S(x) = min_j [mu_min,j - lambda_j], mu the
  gauge-orthogonal spectrum (general builder).
Checks: blocks reproduce the full sector spectra; the Berger path crosses the A-edge at t_A = 2.7311222782 (root of the
degree-16 polynomial) and the S-edge at t* = 2.6980269; S-edges reproduce Test 6's bisection edges.
usage: python scripts/ricci_su2_exact_edge.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np
import sympy as sp
from scipy.optimize import brentq

sys.path.insert(0, str(Path(__file__).resolve().parent))
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_gap_certificates_t as GC      # noqa: E402
sys.argv = _argv
import ricci_su3_test2_family as T2            # noqa: E402

x, y = sp.symbols("x y", real=True)
t = sp.symbols("t", positive=True)
JMAX = 12
T_A, T_STAR = 2.7311222782381136231, 2.6980269405200263


def cross(v):
    return sp.Matrix([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]])


def symbolic_rows():
    s, c, _, _, gmat = GC.background(sp.sqrt, sp.Integer(1), t)
    m = [sp.Matrix([x, 0, 0]), sp.Matrix([0, y, 0]), sp.Matrix([0, 0, y])]
    I3 = sp.eye(3)
    Om = [sp.kronecker_product(I3, cross(m[i])) + sp.kronecker_product(-sp.Matrix(gmat[i]), I3) for i in range(3)]
    Ric = sp.diag(2 * t, 4 - 2 * t, 4 - 2 * t)
    T = sp.zeros(9, 9)
    for i in range(3):
        for k in range(3):
            Fik = cross(m[i]) * m[k]
            for l in range(3):
                Fik = Fik - c[(i, k, l)] * m[l]
            T[3 * i:3 * i + 3, 3 * k:3 * k + 3] = cross(Fik)
    R0 = sp.kronecker_product(Ric, I3) - 2 * T
    r2 = sp.sqrt(2)
    U3 = sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-sp.I / r2, 0, sp.I / r2]])
    U = sp.kronecker_product(U3, U3)
    tr = lambda M: (U.H * M * U).applyfunc(sp.simplify)          # noqa: E731
    Omc = [tr(O) for O in Om]
    offd = (Omc[0] - sp.diag(*[Omc[0][a, a] for a in range(9)])).applyfunc(sp.simplify)
    assert offd == sp.zeros(9, 9), "Omega_0 not diagonal in the charge basis"
    w = [sp.simplify(Omc[0][a, a] / sp.I) for a in range(9)]
    Op, Omn = (Omc[1] + sp.I * Omc[2]).applyfunc(sp.simplify), (Omc[1] - sp.I * Omc[2]).applyfunc(sp.simplify)
    C = (-(Omc[1] * Omc[1] + Omc[2] * Omc[2]) + tr(R0)).applyfunc(sp.simplify)
    ch = [(cf, cl) for cf in (-1, 0, 1) for cl in (-1, 0, 1)]
    f = sp.lambdify((x, y, t), [w, C, Op, Omn], "numpy")
    return ch, f


class Blocks:
    def __init__(self):
        self.ch, self.f = symbolic_rows()

    def data(self, xv, yv, tv):
        w, C, Op, Omn = self.f(xv, yv, tv)
        return (np.array(w, complex).real, np.array(C, complex), np.array(Op, complex), np.array(Omn, complex))

    def block(self, d, j, q, tv):
        w, C, Op, Omn = d
        l = j / 2
        states = [(q - a - b, k) for k, (a, b) in enumerate(self.ch) if abs(q - a - b) <= l + 1e-9]
        idx = {s_: n for n, s_ in enumerate(states)}; n = len(states)
        B = np.zeros((n, n), complex)
        for (mm, a), k in idx.items():
            B[k, k] += (w[a] - 2 * mm / np.sqrt(tv)) ** 2 + 4 * (l * (l + 1) - mm * mm)
            for (m2, a2), k2 in idx.items():
                if m2 == mm:
                    B[k2, k] += C[a2, a]
                if m2 == mm + 1:
                    B[k2, k] += 2j * np.sqrt(max((l - mm) * (l + mm + 1), 0)) * Omn[a2, a]
                if m2 == mm - 1:
                    B[k2, k] += 2j * np.sqrt(max((l + mm) * (l - mm + 1), 0)) * Op[a2, a]
        return states, 0.5 * (B + B.conj().T)

    def sector_spectrum(self, xv, yv, tv, j):
        d = self.data(xv, yv, tv); l = j / 2; ev = []
        for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0):
            _, B = self.block(d, j, q, tv)
            if B.size:
                ev.extend(np.linalg.eigvalsh(B))
        return np.sort(ev)

    def phi_A(self, xv, yv, tv, with_arg=False):
        d = self.data(xv, yv, tv); best = (np.inf, None)
        for j in range(JMAX + 1):
            l = j / 2; lam = lam_tw(xv, yv, tv, j)
            for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0):
                if abs(abs(q) - (l + 2)) < 1e-9:
                    continue
                _, B = self.block(d, j, q, tv)
                if B.size:
                    v = np.linalg.eigvalsh(B)[0] - lam
                    if v < best[0]:
                        best = (v, (j, float(q)))
        return best if with_arg else best[0]


def lam_tw(xv, yv, tv, j):
    return (xv + (j + 2) / np.sqrt(tv)) ** 2 + 2 * np.sqrt(tv) * xv - yv ** 2


_G = {}


def phi_S(xv, yv, tv, with_arg=False):
    if not _G:
        S, C2, eps = T2.su2_data(); _G.update(C2=C2, eps=eps, rho={j: S.spin_rep(j) for j in range(JMAX + 1)})
    g = [tv, 1.0, 1.0]; M = [xv * np.eye(3)[0], yv * np.eye(3)[1], yv * np.eye(3)[2]]
    best = (np.inf, None)
    for j in range(JMAX + 1):
        H, G = T2.hessian_and_gauge(_G["C2"], g, _G["eps"], M, _G["rho"][j])
        mu = T2.slice_spectrum(H, G)
        lam = lam_tw(xv, yv, tv, j)
        # the twist itself is in the slice: skip it (and its conjugate) by removing two copies of lam
        mu = list(mu)
        for _ in range(2):
            k = int(np.argmin(np.abs(np.array(mu) - lam)))
            if abs(mu[k] - lam) < 1e-8 * max(1, abs(lam)):
                mu.pop(k)
        v = min(mu) - lam
        if v < best[0]:
            best = (v, j)
    return best if with_arg else best[0]


def edge(fun, yv, tv, lo=-3.0, hi=1.0, n=81):
    """The largest x in [lo, hi] at which fun changes sign from >= 0 (left) to < 0: the edge of the half-line."""
    xs = np.linspace(lo, hi, n); vals = [fun(xv, yv, tv) for xv in xs]
    k = None
    for i in range(len(xs) - 1):
        if vals[i] >= 0 > vals[i + 1]:
            k = i
    if k is None:
        return None
    return brentq(lambda xv: fun(xv, yv, tv), xs[k], xs[k + 1], xtol=1e-12)


def main():
    out = {}
    bl = Blocks()
    # check: blocks reproduce the full H + GG^* sector spectra
    rng = np.random.default_rng(3); worst = 0.0
    S, C2, eps = T2.su2_data()
    for _ in range(12):
        xv, yv = rng.uniform(-2, 2), rng.uniform(0, 3); tv = rng.uniform(0.5, 8); jv = int(rng.integers(0, 9))
        H, G = T2.hessian_and_gauge(C2, [tv, 1, 1], eps, [xv * np.eye(3)[0], yv * np.eye(3)[1], yv * np.eye(3)[2]], S.spin_rep(jv))
        ref = np.linalg.eigvalsh(H + G @ G.conj().T)
        got = bl.sector_spectrum(xv, yv, tv, jv)
        worst = max(worst, float(np.abs(np.sort(ref) - got).max() / max(1, np.abs(ref).max())))
    out["blocks_vs_full_spectra_max_rel_diff"] = worst
    print("blocks reproduce the full spectra:", worst, flush=True)
    # Berger crossings
    bx = lambda tv: (2 - tv) / np.sqrt(tv)         # noqa: E731
    fA = lambda tv: bl.phi_A(bx(tv), np.sqrt(tv), tv)      # noqa: E731
    fS = lambda tv: phi_S(bx(tv), np.sqrt(tv), tv)         # noqa: E731
    tA = brentq(fA, 2.6, 2.9, xtol=1e-13); tS = brentq(fS, 2.6, 2.75, xtol=1e-13)
    out["berger_crossing_A"] = {"t": tA, "t_A_exact": T_A, "diff": tA - T_A,
                                "competitor_(j,q)": bl.phi_A(bx(tA - 1e-6), np.sqrt(tA), tA - 1e-6, True)[1]}
    out["berger_crossing_S"] = {"t": tS, "t_star": T_STAR, "diff": tS - T_STAR,
                                "competitor_sector": phi_S(bx(tS - 1e-6), np.sqrt(tS), tS - 1e-6, True)[1]}
    print("Berger crossings:", out["berger_crossing_A"], out["berger_crossing_S"], flush=True)
    # edges at Test 6's points, and a finer table
    ref6 = json.loads((Path(__file__).resolve().parent.parent / "research/su2-floor-family-20261119/edge_test_results.json").read_text())
    pred6 = json.loads((Path(__file__).resolve().parent.parent / "research/su2-floor-family-20261119/edge_test_predictions.json").read_text())
    measured = {}
    for h, d in pred6["training_edges"].items():
        for ts_, r in d.items():
            measured[(float(ts_), h)] = r["edge"]
    for r in ref6["E1"]:
        measured[(r["t"], r["height"])] = r["measured"]
    rows = []
    for tv in (2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0):
        for h in ("0.5", "1.5", "sqrt_t"):
            yv = np.sqrt(tv) if h == "sqrt_t" else float(h)
            eA = edge(bl.phi_A, yv, tv); eS = edge(phi_S, yv, tv)
            compA = bl.phi_A(eA - 1e-3, yv, tv, True)[1] if eA is not None else None
            compA_after = bl.phi_A(eA + 1e-3, yv, tv, True)[1] if eA is not None else None
            compS = phi_S(eS + 1e-3, yv, tv, True)[1] if eS is not None else None
            row = {"t": tv, "height": h, "y": yv, "edge_A": eA, "edge_S": eS, "competitor_block_A": compA_after,
                   "competitor_sector_S": compS, "test6_measured": measured.get((tv, h))}
            if row["test6_measured"] is not None and eS is not None:
                row["S_minus_test6"] = eS - row["test6_measured"]
            rows.append(row)
            print(row, flush=True)
    out["edges"] = rows
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1, default=str) + "\n")


if __name__ == "__main__":
    main()
