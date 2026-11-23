"""WP4 design checks for the edge-theorem pre-registrations (21 November 2026).

These checks are structural. None of them computes a floor, an edge, or any quantity that a WP4 test scores.

  D1 (for prereg B)  Is the (x, y) ansatz general, up to constant gauge, among left-invariant SU(2) fields on (t, 1, 1)
                     that are invariant under the right circle about e_0 combined with a constant gauge circle of any
                     weight k?  Symbolic commutant for k = -2..2, then the gauge reduction, then a numerical code
                     control: full H + GG^* sector spectra (j <= 4) agree between a field and its gauge-reduced form.
  D2 (for prereg A)  Polynomial structure of the closed-form rows in x: w_a - c_l x is x-free and every entry of C is of
                     degree <= 1 in x; the j = 0 state theta^0 (x) tau_0 is gauge-orthogonal for every (x, y, t).
                     (These are the two facts the lower-edge argument of prereg A uses.)
  D3 (for prereg C)  SU(3), metric 2b (t on lambda_8): the left-invariant fields commuting with the joint (frame + gauge)
                     action of u(2) = span(X1, X2, X3, X8) form a 4-dimensional space (u, x, y, z); z is removed by a
                     constant gauge rotation exp(alpha X8).

usage: OMP_NUM_THREADS=1 python scripts/edge_wp4_design_checks.py research/edge-theorem-20261121/wp4_design_checks.json
"""
import json
import sys
from pathlib import Path

import numpy as np
import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_gap_certificates_t as GC      # noqa: E402
sys.argv = _argv
import ricci_su3_test2_family as T2            # noqa: E402
import ricci_su2_exact_edge as EE              # noqa: E402

out = {}

# ------------------------------------------------------------------ D1: commutant, symbolic
L = sp.Matrix([[0, 0, 0], [0, 0, -1], [0, 1, 0]])            # so(2) generator about axis 0 (indices 1, 2)
Msym = sp.Matrix(3, 3, sp.symbols("M0:9", real=True))        # M[i, a]: frame i, Lie a
d1 = {}
for k in (-2, -1, 0, 1, 2):
    sols = sp.solve(list(L * Msym - k * Msym * L), list(Msym), dict=True)
    Mk = Msym.subs(sols[0]) if sols else Msym
    free = sorted(Mk.free_symbols, key=str)
    d1[str(k)] = {"dim": len(free), "general_M": str(Mk.tolist())}
out["D1_commutant_by_weight_k"] = d1
print("D1 commutant:", {k: v["dim"] for k, v in d1.items()}, flush=True)


def rot(axis, a):
    c, s = np.cos(a), np.sin(a)
    R = np.eye(3)
    i, j = [(1, 2), (2, 0), (0, 1)][axis]
    R[i, i] = R[j, j] = c; R[i, j] = -s; R[j, i] = s
    return R


def field(x, y, z=0.0, k=1):
    """Rows m_i (Lie vectors). k = +1: m1 = y e1 + z e2, m2 = -z e1 + y e2; k = -1: m1 = y e1 + z e2, m2 = z e1 - y e2."""
    if k == 1:
        return [np.array([x, 0, 0.]), np.array([0, y, z]), np.array([0, -z, y])]
    return [np.array([x, 0, 0.]), np.array([0, y, z]), np.array([0, z, -y])]


# invariance of these forms (numerical, exact up to rounding): L M - k M L = 0 with M[i, a] = field row i
Lnum = np.array(L, float)
for k in (1, -1):
    M = np.array(field(0.3, 0.7, 0.4, k))
    assert np.abs(Lnum @ M - k * M @ Lnum).max() < 1e-15, k

S, C2, eps = T2.su2_data()
rng = np.random.default_rng(20261121)
worst = {"z_removed_by_gauge_rotation_about_e0": 0.0, "k_minus1_equals_ansatz_with_minus_x": 0.0,
         "random_constant_gauge_frame": 0.0}
reps = {j: S.spin_rep(j) for j in range(5)}
for _ in range(6):
    xv, yv, zv = rng.uniform(-3, 3), rng.uniform(0, 3), rng.uniform(-3, 3); tv = rng.uniform(1, 10)
    g = [tv, 1, 1]
    for j in range(5):
        def spec(M):
            H, G = T2.hessian_and_gauge(C2, g, eps, M, reps[j])
            return np.linalg.eigvalsh(H + G @ G.conj().T)
        sc = lambda a, b: float(np.abs(a - b).max() / max(1.0, np.abs(a).max()))      # noqa: E731
        rho = np.hypot(yv, zv)
        # D1a: (x, y, z) vs (x, rho, 0)
        worst["z_removed_by_gauge_rotation_about_e0"] = max(worst["z_removed_by_gauge_rotation_about_e0"],
                                                            sc(spec(field(xv, yv, zv)), spec(field(xv, rho, 0.0))))
        # D1b: k = -1 family (x, y) vs ansatz (-x, y), via a pi rotation about Lie axis 1
        worst["k_minus1_equals_ansatz_with_minus_x"] = max(worst["k_minus1_equals_ansatz_with_minus_x"],
                                                           sc(spec(field(xv, yv, 0.0, -1)), spec(field(-xv, yv))))
        # D1c: a random constant gauge frame R (Lie index) leaves the spectrum unchanged
        Q, _ = np.linalg.qr(rng.normal(size=(3, 3))); Q = Q * np.sign(np.linalg.det(Q))
        worst["random_constant_gauge_frame"] = max(worst["random_constant_gauge_frame"],
                                                   sc(spec(field(xv, yv, zv)), spec([Q @ m for m in field(xv, yv, zv)])))
# the explicit gauge maps, checked on the field itself
xv, yv, zv = 0.37, 1.1, -0.6
a = np.arctan2(zv, yv)
Mz = np.array(field(xv, yv, zv)); Mr = np.array(field(xv, np.hypot(yv, zv), 0.0))
gz = float(np.abs(Mz @ rot(0, a) - Mr).max())                      # rows m_i -> R^T m_i, i.e. M R
Mm = np.array(field(xv, yv, 0.0, -1)); Mp = np.array(field(-xv, yv))
gm = float(np.abs(Mm @ rot(1, np.pi).T - Mp).max())
out["D1_gauge_maps_on_fields_max_abs"] = {"z_removal": gz, "k_minus1_to_minus_x": gm}
out["D1_spectral_code_control_max_rel_diff_j_le_4"] = worst
print("D1 gauge maps:", out["D1_gauge_maps_on_fields_max_abs"], flush=True)
print("D1 spectra:", worst, flush=True)

# ------------------------------------------------------------------ D2: structure of the rows in x
x, y, t = EE.x, EE.y, EE.t
s_, c, _, _, gmat = GC.background(sp.sqrt, sp.Integer(1), t)
m = [sp.Matrix([x, 0, 0]), sp.Matrix([0, y, 0]), sp.Matrix([0, 0, y])]
I3 = sp.eye(3)
Om = [sp.kronecker_product(I3, EE.cross(m[i])) + sp.kronecker_product(-sp.Matrix(gmat[i]), I3) for i in range(3)]
Ric = sp.diag(2 * t, 4 - 2 * t, 4 - 2 * t)
Tm = sp.zeros(9, 9)
for i in range(3):
    for k in range(3):
        Fik = EE.cross(m[i]) * m[k]
        for l in range(3):
            Fik = Fik - c[(i, k, l)] * m[l]
        Tm[3 * i:3 * i + 3, 3 * k:3 * k + 3] = EE.cross(Fik)
R0 = sp.kronecker_product(Ric, I3) - 2 * Tm
r2 = sp.sqrt(2)
U3 = sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-sp.I / r2, 0, sp.I / r2]])
U = sp.kronecker_product(U3, U3)
tr = lambda Mx: (U.H * Mx * U).applyfunc(sp.simplify)          # noqa: E731
Omc = [tr(O) for O in Om]
w = [sp.simplify(Omc[0][a_, a_] / sp.I) for a_ in range(9)]
ch = [(cf, cl) for cf in (-1, 0, 1) for cl in (-1, 0, 1)]
Cm = (-(Omc[1] * Omc[1] + Omc[2] * Omc[2]) + tr(R0)).applyfunc(sp.expand)
Op, Omn = (Omc[1] + sp.I * Omc[2]), (Omc[1] - sp.I * Omc[2])
w_minus = {}
for a_, (cf, cl) in enumerate(ch):
    sgn = [sv for sv in (1, -1) if sp.diff(sp.simplify(w[a_] - sv * cl * x), x) == 0]
    w_minus[str((cf, cl))] = {"w": str(w[a_]), "w = sigma*c_l*x + f(t) with sigma in": sgn}
degC = max(sp.Poly(e, x).degree() if e != 0 else 0 for e in Cm)
ladder_x_free = all(sp.diff(e, x) == 0 for e in list(Op) + list(Omn))
out["D2"] = {"w_entries": w_minus, "max_degree_in_x_of_C": degC, "ladder_couplings_free_of_x": bool(ladder_x_free)}
# gauge-orthogonality of theta^0 (x) tau_0 in sector j = 0 for all (x, y, t): G^* v = 0 symbolically
# in sector j = 0 (trivial rep) G xi = (ad(m_i) xi)_i, so G^* v = -sum_i ad(m_i) v_i ; v = (e0, 0, 0)
Gs_v = -(EE.cross(m[0]) * sp.Matrix([1, 0, 0]))
out["D2"]["Gstar_theta0_tau0_sector0"] = str(list(Gs_v))
print("D2:", {k: v for k, v in out["D2"].items() if k != "w_entries"}, flush=True)
print("D2 w:", {k: v["dim"] if "dim" in v else v["w = sigma*c_l*x + f(t) with sigma in"] for k, v in w_minus.items()},
      flush=True)

# ------------------------------------------------------------------ D3: SU(3) U(2)-commutant (family C design)
X, C3 = T2.su3_data()
u2 = [0, 1, 2, 7]
eqs = []
for gi in u2:
    A = C3[gi].T                                    # [X_g, X_a] = sum_b C[g, a, b] X_b -> matrix A[b, a]
    # frame index i rotates by A, Lie index a by A: invariance A M + M A^T = 0 for M[i, a]  (M -> A M + M A^T)
    E = np.zeros((64, 64))
    for i in range(8):
        for a_ in range(8):
            Z = np.zeros((8, 8)); Z[i, a_] = 1
            E[:, i * 8 + a_] = (A @ Z + Z @ A.T).ravel()
    eqs.append(E)
E = np.vstack(eqs)
_, sv, Vt = np.linalg.svd(E)
null = Vt[(sv > 1e-10).sum():]
out["D3_u2_commutant_dim"] = int(null.shape[0])
# the four expected generators: u (X1, X2, X3 diagonal), x (X8), y (X4..X7 diagonal), z (complex structure on X4..X7)
basis = {}
Mu = np.zeros((8, 8)); Mu[[0, 1, 2], [0, 1, 2]] = 1
Mx = np.zeros((8, 8)); Mx[7, 7] = 1
My = np.zeros((8, 8)); My[[3, 4, 5, 6], [3, 4, 5, 6]] = 1
J = C3[7].T[3:7, 3:7] / np.abs(C3[7].T[3:7, 3:7]).max()   # ad(X8) on X4..X7, normalised: a complex structure
Mz = np.zeros((8, 8)); Mz[3:7, 3:7] = J
for name, Mb in (("u", Mu), ("x", Mx), ("y", My), ("z", Mz)):
    res = float(np.abs(E @ Mb.ravel()).max())
    basis[name] = res
out["D3_generators_residual"] = basis
# z removed by constant gauge rotation exp(alpha X8): rows M_i -> Ad(exp(alpha X8)) M_i
from scipy.linalg import expm
yv, zv = 0.8, 0.5
Mfield = 0.3 * Mu + 0.6 * Mx + yv * My + zv * Mz
best = np.inf
for al in np.linspace(-np.pi, np.pi, 20001):
    Ad = expm(al * C3[7].T)
    Mg = Mfield @ Ad.T
    best = min(best, float(np.abs(Mg - (0.3 * Mu + 0.6 * Mx + np.hypot(yv, zv) * My)).max()))
out["D3_z_removal_best_residual_on_1e-4_grid"] = best
print("D3:", out["D3_u2_commutant_dim"], basis, best, flush=True)

Path(sys.argv[1]).parent.mkdir(parents=True, exist_ok=True)
Path(sys.argv[1]).write_text(json.dumps(out, indent=1, default=str) + "\n")
