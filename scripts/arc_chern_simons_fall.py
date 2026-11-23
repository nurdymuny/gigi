"""Which winding does she fall with? Chern-Simons along the fall of the Berger field (22 November 2026).

Open question 2 of docs/research/the-basin-2026-11-22.md.

Conventions (those of scripts/ricci_su2_exact_edge.py and the basin note):
  * SU(2) = unit S^3 with left-invariant frame e_0, e_1, e_2, [e_i, e_j] = 2 eps_ijk e_k, dual coframe theta^i, so
    d theta^k = -2 theta^i ^ theta^j (cyclic). Berger metric (t,1,1): g = t (theta^0)^2 + (theta^1)^2 + (theta^2)^2,
    orthonormal coframe theta-hat^i = sqrt(g_i) theta^i, i.e. theta-hat^0 = sqrt(t) theta^0.
  * su(2) = R^3 with bracket = cross product, realised as tau_a = -(i/2) sigma_a  ([tau_a, tau_b] = eps_abc tau_c).
  * Family: A = sum_i m_i theta-hat^i, m_0 = x tau_0, m_1 = y tau_1, m_2 = y tau_2;  F = dA + (1/2)[A, A].
    A = theta = g^-1 dg is x = 2/sqrt t, y = 2 (theta_MC = 2 theta^i tau_i since e_i <-> 2 tau_i).
  * CS(A) = (1/8 pi^2) Int_{S^3} tr(A ^ dA + (2/3) A ^ A ^ A), tr = matrix trace in the fundamental (2x2) rep of the
    anti-Hermitian tau's, so tr(XY) = -(1/2) X.Y for X, Y in R^3.  S^3 oriented by theta^0 ^ theta^1 ^ theta^2 (the
    Berger volume form sqrt(t) theta^0^theta^1^theta^2 has the same orientation).  Vol(S^3, round, e_i orthonormal)
    = 2 pi^2 (unit sphere), Vol(Berger) = 2 pi^2 sqrt(t).

Parts:
  1. PROVED (sympy, exact): CS in the family = (t x^2 + 2 y^2 - sqrt(t) x y^2) / 4, derived by explicit exterior
     algebra on 2x2 matrices; CS(theta) = 1, CS(0) = 0, CS(meron t=1,x=y=1) = 1/2, CS(Berger LC field) = (t^2-2t+2)/2.
  2. COMPUTED (numerical quadrature): independent check that CS(theta) = 1 = -(1/24 pi^2) Int tr (g^-1 dg)^3 on the
     identity map, with Vol = 2 pi^2 and the orientation tied to theta^0^theta^1^theta^2 (Euler-angle chart).
  3. PROVED (sympy): critical points of the family energy E = (B^2 + 2 c^2)/2 and their CS ("the pass").
  4. COMPUTED: the family gradient flow (family_flow.py ODE) from the Berger field at t in {2, 3, 3.55, 5, 10}
     (plus extras), CS along the fall, half-integer crossings, CS at the door->wall moment, the landing.
usage: OMP_NUM_THREADS=1 python scripts/arc_chern_simons_fall.py OUTDIR
"""
import itertools
import json
import sys
from pathlib import Path

import numpy as np
import sympy as sp
from scipy.integrate import solve_ivp, nquad
from scipy.optimize import brentq

OUT = Path(sys.argv[1] if len(sys.argv) > 1 else "research/arc-deepdive-20261122")
OUT.mkdir(parents=True, exist_ok=True)
R = {}

x, y = sp.symbols("x y", real=True)
t = sp.symbols("t", positive=True)
st = sp.sqrt(t)
I = sp.I
sig = [sp.Matrix([[0, 1], [1, 0]]), sp.Matrix([[0, -I], [I, 0]]), sp.Matrix([[1, 0], [0, -1]])]
tau = [-I / 2 * s for s in sig]
eps = lambda a, b, c: sp.LeviCivita(a, b, c)  # noqa: E731

# ---------------------------------------------------------------- 1. exact CS by exterior algebra
# forms on the coframe theta^0, theta^1, theta^2: a p-form = {sorted index tuple: 2x2 matrix coefficient}
def wedge(a, b):
    out = {}
    for ia, ca in a.items():
        for ib, cb in b.items():
            idx = ia + ib
            if len(set(idx)) < len(idx):
                continue
            perm = sorted(range(len(idx)), key=lambda k: idx[k])
            sgn = sp.combinatorics.Permutation(perm).signature()
            key = tuple(sorted(idx))
            out[key] = out.get(key, sp.zeros(2, 2)) + sgn * ca * cb
    return out


def d1(a):
    """d of a left-invariant 1-form sum_k a_k theta^k (constant matrix coefficients): d theta^k = -sum eps_ijk th^i^th^j."""
    out = {}
    for (k,), ck in a.items():
        for i, j in itertools.combinations(range(3), 2):
            e = eps(i, j, k)
            if e:
                out[(i, j)] = out.get((i, j), sp.zeros(2, 2)) - 2 * e * ck
    return out


def add(a, b, s=1):
    out = dict(a)
    for k, v in b.items():
        out[k] = out.get(k, sp.zeros(2, 2)) + s * v
    return out


# Lie algebra checks
for a, b in itertools.product(range(3), repeat=2):
    lhs = tau[a] * tau[b] - tau[b] * tau[a]
    rhs = sum((eps(a, b, c) * tau[c] for c in range(3)), sp.zeros(2, 2))
    assert sp.simplify(lhs - rhs) == sp.zeros(2, 2)
assert all(sp.simplify((tau[a] * tau[b]).trace() + sp.Rational(1, 2) * int(a == b)) == 0 for a in range(3) for b in range(3))

# A in the theta basis: A = x sqrt(t) theta^0 tau_0 + y theta^1 tau_1 + y theta^2 tau_2
coef = [x * st, y, y]
A = {(k,): coef[k] * tau[k] for k in range(3)}
dA = d1(A)
# F = dA + (1/2)[A,A] = dA + A^A (matrix wedge)
AA = wedge(A, A)
F = add(dA, AA)
F12 = sp.simplify(F[(1, 2)])
# component along tau_0: tr(F12 tau_0) = -(1/2) F12^0
F12_0 = sp.simplify(-2 * (F12 * tau[0]).trace())
assert sp.simplify(F12_0 - (y**2 - 2 * st * x)) == 0, F12_0          # matches B of the basin note
F20_1 = sp.simplify(-2 * (F[(0, 2)] * tau[1]).trace())               # F_{02} along tau_1, theta basis
three = (0, 1, 2)
AdA = wedge(A, dA)[three]
AAA = wedge(AA, A)[three]
dens = sp.expand(sp.simplify((AdA + sp.Rational(2, 3) * AAA).trace()))      # coefficient of theta^0^theta^1^theta^2
VOL_ROUND = 2 * sp.pi**2
CS = sp.expand(sp.simplify(dens * VOL_ROUND / (8 * sp.pi**2)))
CS_claim = (t * x**2 + 2 * y**2 - st * x * y**2) / 4
assert sp.simplify(CS - CS_claim) == 0, CS
# same thing in the orthonormal coframe (Berger volume 2 pi^2 sqrt t): coefficient of theta-hat^0^1^2 is dens / sqrt t
dens_hat = sp.simplify(dens / st)
assert sp.simplify(dens_hat * 2 * sp.pi**2 * st / (8 * sp.pi**2) - CS_claim) == 0
CSf = sp.lambdify((x, y, t), CS, "numpy")

xb, yb = (2 - t) / st, st                                   # Berger (Levi-Civita) field in the family
special = {
    "A=0": sp.simplify(CS.subs({x: 0, y: 0})),
    "A=theta": sp.simplify(CS.subs({x: 2 / st, y: 2})),
    "meron (t=1, x=y=1)": sp.simplify(CS.subs({t: 1, x: 1, y: 1})),
    "Berger LC field (x=(2-t)/sqrt t, y=sqrt t)": sp.factor(sp.simplify(CS.subs({x: xb, y: yb}))),
}
assert special["A=0"] == 0 and special["A=theta"] == 1 and special["meron (t=1, x=y=1)"] == sp.Rational(1, 2)
R["exact"] = {
    "status": "PROVED (sympy exterior algebra on 2x2 matrices)",
    "normalisation": "CS(A) = (1/8 pi^2) Int_{S^3} tr(A^dA + (2/3)A^A^A); tr = trace of 2x2 anti-Hermitian matrices "
                     "tau_a = -(i/2) sigma_a, tr(tau_a tau_b) = -delta_ab/2; orientation theta^0^theta^1^theta^2; "
                     "Vol(unit S^3)=2 pi^2, Vol(Berger)=2 pi^2 sqrt(t)",
    "density_theta_basis": str(dens),
    "density_orthonormal_basis": str(sp.expand(dens_hat)),
    "CS_family": str(CS),
    "CS_family_note": "polynomial in x, y and sqrt(t); the sqrt(t) of the volume cancels against the coframe "
                      "normalisation theta-hat^0 = sqrt(t) theta^0",
    "F_12_along_tau0": str(F12_0),
    "special_values": {k: str(v) for k, v in special.items()},
    "flat_CS_reading": "for flat A, dA = -A^A so tr(AdA + 2/3 A^3) = -(1/3) tr A^3; CS(theta) = -(1/24 pi^2) Int tr(g^-1dg)^3 = 1",
}
print("CS(x,y,t) =", CS, "  specials:", {k: str(v) for k, v in special.items()})

# ---------------------------------------------------------------- 2. independent quadrature check of CS(theta) = 1
# g(psi, th, ph) = exp(ph tau_2*2/2 ...): use unit quaternion chart g = e^{phi k/2} e^{theta j/2} e^{psi k/2} (Euler),
# realised with E_i = 2 tau_i (E_i E_j = -delta + eps E_k, the quaternions i,j,k), ranges th in [0,pi], ph in [0,2pi),
# psi in [0,4pi). L_u = g^-1 d_u g; theta^i(d_u) = -tr(L_u E_i)/2 (since tr(E_i E_j) = -2 delta).
u1, u2, u3 = sp.symbols("u1 u2 u3", real=True)  # (theta, phi, psi)
E = [2 * tm for tm in tau]
def expE(k, ang):
    return sp.cos(ang / 2) * sp.eye(2) + sp.sin(ang / 2) * E[k]
g = expE(2, u2) * expE(1, u1) * expE(2, u3)
ginv = g.H
L = [sp.simplify(ginv * sp.diff(g, u)) for u in (u1, u2, u3)]
th = sp.Matrix(3, 3, lambda i, u: sp.simplify(-(L[u] * E[i]).trace() / 2))   # theta^i(d_u)
detth = sp.simplify(th.det())
tr3 = sp.simplify(sum(sp.combinatorics.Permutation(list(p)).signature() * (L[p[0]] * L[p[1]] * L[p[2]]).trace()
                      for p in itertools.permutations(range(3))))
fdet = sp.lambdify((u1, u2, u3), detth); ftr = sp.lambdify((u1, u2, u3), tr3)
rng = [[0, np.pi], [0, 2 * np.pi], [0, 4 * np.pi]]
vol_chart = nquad(lambda a, b, c: float(np.real(fdet(a, b, c))), rng)[0]
int_tr3 = nquad(lambda a, b, c: float(np.real(ftr(a, b, c))), rng)[0]
sgn = np.sign(vol_chart)                               # chart orientation relative to theta^0^theta^1^theta^2
R["quadrature_check"] = {
    "status": "COMPUTED (scipy nquad, Euler chart of the identity map g: S^3 -> SU(2))",
    "det_theta_in_chart": str(detth),
    "Int theta^0^theta^1^theta^2 (chart orientation)": vol_chart,
    "2 pi^2": 2 * np.pi**2,
    "Int tr(g^-1 dg)^3, theta-orientation": sgn * int_tr3,
    "-24 pi^2": -24 * np.pi**2,
    "CS(theta) = -(1/24 pi^2) Int tr(g^-1dg)^3": -sgn * int_tr3 / (24 * np.pi**2),
}
print("quadrature: |vol| =", abs(vol_chart), "vs 2pi^2 =", 2 * np.pi**2, "; CS(theta) =", -sgn * int_tr3 / (24 * np.pi**2))
assert abs(abs(vol_chart) - 2 * np.pi**2) < 1e-8 and abs(-sgn * int_tr3 / (24 * np.pi**2) - 1) < 1e-8

# ---------------------------------------------------------------- 3. critical points in the family (the pass)
B = y**2 - 2 * st * x; c = y * (x - 2 / st)
Efam = sp.Rational(1, 2) * (B**2 + 2 * c**2)
# check E against F computed above: F_12 along tau0 = B (orthonormal = theta basis for 12);
# F_{02} theta basis = sqrt t * F-hat_{02}, so F-hat_{02} along tau_1 should be +-c
assert sp.simplify(F20_1 / st - c) == 0 or sp.simplify(F20_1 / st + c) == 0
crit = sp.solve([sp.diff(Efam, x), sp.diff(Efam, y)], [x, y], dict=True)
crit_out = []
for sol in crit:
    xs, ys = sp.simplify(sol[x]), sp.simplify(sol[y])
    crit_out.append({"x": str(xs), "y": str(ys), "E": str(sp.factor(sp.simplify(Efam.subs({x: xs, y: ys})))),
                     "CS": str(sp.factor(sp.simplify(CS.subs({x: xs, y: ys}))))})
# clean closed form of the non-flat pair (the deformed meron): with u = x - 2/sqrt t, grad E = 0 and y != 0, u != 0 give
# B = -u^2 and y^2 = -sqrt(t) u, hence u^2 - 3 sqrt(t) u - 4 = 0, u = (3 sqrt t - sqrt(9t+16))/2 (the root with y^2 > 0).
u_s = (3 * st - sp.sqrt(9 * t + 16)) / 2
xs_m, y2_m = 2 / st + u_s, -st * u_s
gx = sp.diff(Efam, x).subs({x: xs_m, y: sp.sqrt(y2_m)}); gy = sp.diff(Efam, y).subs({x: xs_m, y: sp.sqrt(y2_m)})
assert sp.simplify(gx) == 0 and sp.simplify(gy) == 0
E_m = sp.simplify(sp.expand(Efam.subs({x: xs_m, y: sp.sqrt(y2_m)})))
CS_m = sp.simplify(sp.expand(CS.subs({x: xs_m, y: sp.sqrt(y2_m)})))
assert sp.simplify(CS_m.subs(t, 1) - sp.Rational(1, 2)) == 0 and sp.simplify(E_m.subs(t, 1) - sp.Rational(3, 2)) == 0
# every sympy root is one of: (0,0), (2/sqrt t, +-2), (xs_m, +-sqrt(y2_m))
for sol in crit:
    v = [(0, 0), (2 / st, 2), (2 / st, -2), (xs_m, sp.sqrt(y2_m)), (xs_m, -sp.sqrt(y2_m))]
    assert any(all(abs(complex((sol[s] - w).subs(t, tv))) < 1e-10 for s, w in zip((x, y), vv) for tv in (0.5, 2, 7))
               for vv in v), sol
lim_inf = sp.limit(CS_m, t, sp.oo)
R["critical_points"] = {
    "status": "PROVED (sympy: grad E = 0 solved in the family; closed form verified; these are all real critical points)",
    "flat": ["A=0 (0,0): E=0, CS=0", "A=theta (2/sqrt t, +-2): E=0, CS=1 (y -> -y is the constant gauge rotation by pi about tau_0)"],
    "deformed_meron": {"u": str(u_s), "x": "2/sqrt(t) + u", "y^2": "-sqrt(t) u", "E": str(E_m), "CS": str(CS_m),
                       "CS_limit_t_to_infinity": str(lim_inf), "at t=1": "x=y=1, E=3/2, CS=1/2 (the meron)"},
    "raw_sympy_solve": crit_out,
}
print("deformed meron: CS =", CS_m, "; E =", E_m, "; CS(t->oo) =", lim_inf)
Hs = sp.hessian(Efam, (x, y))
Gm = sp.diag(1, 2)


def crit_numeric(T):
    rows = []
    for sol in crit:
        xs = complex(sol[x].subs(t, T)); ys = complex(sol[y].subs(t, T))
        if abs(xs.imag) > 1e-12 or abs(ys.imag) > 1e-12:
            continue
        H = np.array(Hs.subs({x: xs.real, y: ys.real, t: T}).evalf(), float)
        ev = np.linalg.eigvals(np.linalg.solve(np.array(Gm, float), H)).real
        rows.append({"x": xs.real, "y": ys.real, "E": float(Efam.subs({x: xs.real, y: ys.real, t: T})),
                     "CS": float(CSf(xs.real, ys.real, T)), "metric_hessian_eigs": sorted(ev.tolist())})
    return rows


# ---------------------------------------------------------------- 4. the fall
fx = sp.lambdify((x, y, t), -sp.diff(Efam, x)); fy = sp.lambdify((x, y, t), -sp.diff(Efam, y) / 2)
Ef = sp.lambdify((x, y, t), Efam)
dCS = sp.lambdify((x, y, t), sp.diff(CS, x) * (-sp.diff(Efam, x)) + sp.diff(CS, y) * (-sp.diff(Efam, y) / 2))
jv = sp.symbols("j")
pf = sp.lambdify((x, y, t, jv), (x + (jv + 2) / st)**2 + 2 * st * x - y**2)
TAU_END = 200.0
falls = {}
for T in [2.0, 3.0, 3.55, 5.0, 10.0, 0.5, 1.5, 1.0]:
    x0, y0 = (2 - T) / np.sqrt(T), np.sqrt(T)
    sol = solve_ivp(lambda s, v: [fx(v[0], v[1], T), fy(v[0], v[1], T)], [0, TAU_END], [x0, y0],
                    dense_output=True, rtol=1e-11, atol=1e-13, method="LSODA")
    ss = np.concatenate([np.linspace(0, 0.2, 40001), np.linspace(0.2, 5, 9601)[1:], np.linspace(5, TAU_END, 2001)[1:]])
    XY = sol.sol(ss); cs = CSf(XY[0], XY[1], T); en = Ef(XY[0], XY[1], T)
    rate = dCS(XY[0], XY[1], T)
    xe, ye = sol.y[:, -1]
    land = "A=0" if np.hypot(xe, ye) < 1e-4 else ("A=theta" if np.hypot(xe - 2 / np.sqrt(T), ye - 2) < 1e-4 else
                                                  "meron/other")
    # half-integer crossings
    crosses = []
    for k in range(len(ss) - 1):
        a, b = cs[k], cs[k + 1]
        for h in np.arange(-0.5, 60.5, 1.0):
            if (a - h) * (b - h) < 0:
                s0 = brentq(lambda s: CSf(*sol.sol(s), T) - h, ss[k], ss[k + 1], xtol=1e-14)
                crosses.append({"CS": float(h), "tau": float(s0), "E": float(Ef(*sol.sol(s0), T)),
                                "direction": "down" if b < a else "up"})
    # door -> wall moment
    m = np.array([min(pf(XY[0][i], XY[1][i], T, jj) for jj in range(13)) for i in range(0, 40001, 4)])
    sd = ss[0:40001:4]
    if m[0] > 0:
        door = None
    else:
        kk = int(np.argmax(m > 0)); door = {"tau": float(sd[kk]), "CS": float(CSf(*sol.sol(sd[kk]), T)),
                                            "E": float(Ef(*sol.sol(sd[kk]), T))} if m[kk] > 0 else "stays negative"
    samples = []
    for s in [0, 0.005, 0.01, 0.02, 0.03, 0.05, 0.1, 0.2, 0.5, 1, 2, 5, 20, 200]:
        xx, yy = sol.sol(s)
        samples.append({"tau": s, "x": float(xx), "y": float(yy), "E": float(Ef(xx, yy, T)), "CS": float(CSf(xx, yy, T))})
    mono = bool(np.all(np.diff(cs) <= 1e-12))
    xm = 2 / np.sqrt(T) + (3 * np.sqrt(T) - np.sqrt(9 * T + 16)) / 2; ym = np.sqrt(-np.sqrt(T) * (xm - 2 / np.sqrt(T)))
    dist = np.sqrt((XY[0] - xm)**2 + 2 * (XY[1] - ym)**2)              # L2 metric x^2 + 2 y^2
    kmin = int(np.argmin(dist)); kcs = int(np.argmin(cs))
    near = {"deformed_meron": {"x": float(xm), "y": float(ym), "E": float(Ef(xm, ym, T)), "CS": float(CSf(xm, ym, T))},
            "closest_L2_distance": float(dist[kmin]), "tau_closest": float(ss[kmin]), "CS_at_closest": float(cs[kmin]),
            "tau_CS_min": float(ss[kcs]), "E_at_CS_min": float(en[kcs])}
    falls[str(T)] = {
        "t": T, "start": {"x": x0, "y": y0, "E": float(Ef(x0, y0, T)), "CS": float(CSf(x0, y0, T)),
                          "CS_exact": str(sp.nsimplify(special["Berger LC field (x=(2-t)/sqrt t, y=sqrt t)"].subs(t, sp.nsimplify(T))))},
        "landing": {"x": float(xe), "y": float(ye), "E": float(Ef(xe, ye, T)), "CS": float(CSf(xe, ye, T)), "field": land},
        "CS_min_along_fall": float(cs.min()), "CS_max_along_fall": float(cs.max()),
        "CS_monotone_nonincreasing": mono, "max_dCS_dtau": float(rate.max()), "min_dCS_dtau": float(rate.min()),
        "half_integer_crossings": crosses, "door_to_wall": door, "samples": samples,
        "critical_points_at_t": crit_numeric(T), "pass": near,
    }
    print(f"t={T}: CS {CSf(x0, y0, T):.4f} -> {CSf(xe, ye, T):.6f} ({land}); min {cs.min():.4f}; monotone {mono}; "
          f"near meron d={near['closest_L2_distance']:.4f} at tau={near['tau_closest']:.3f} (CS_m={near['deformed_meron']['CS']:.4f}); "
          f"crossings {[(c_['CS'], round(c_['tau'], 5)) for c_ in crosses]}; door {door}")
R["falls"] = {"status": "COMPUTED (scipy LSODA, rtol 1e-11, the family_flow.py ODE: L2 metric x^2+2y^2)", "runs": falls}

(OUT / "cs_fall.json").write_text(json.dumps(R, indent=1, default=str))
print("wrote", OUT / "cs_fall.json")
