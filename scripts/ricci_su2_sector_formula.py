"""Derivation of the lowest gauge-orthogonal eigenvalue in each spin sector at the Berger metric (3, 1, 1):
(j(j+2) - 14)/3, with exact eigenvector b = f_top (theta^1 + i theta^2) (x) (tau_1 + i tau_2). Symbolic (sympy) in j.
See docs/research/su2-exact-sector-gauge-check-2026-11-19.md (addendum). Research script."""
import sympy as sp

j = sp.symbols('j', positive=True)
m = j / 2
g = [sp.Integer(3), sp.Integer(1), sp.Integer(1)]
s = [sp.sqrt(x) for x in g]
BR = {}
for a, b, c in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
    BR[(a, b, c)] = 2; BR[(b, a, c)] = -2
c = lambda i, jj, k: sp.Integer(BR.get((i, jj, k), 0)) * s[k] / (s[i] * s[jj])            # noqa: E731
Gam = lambda i, jj, k: sp.Rational(1, 2) * (c(i, jj, k) - c(jj, k, i) + c(k, i, jj))      # noqa: E731
M = [sp.Matrix(3, 3, lambda k, jj: Gam(i, jj, k)) for i in range(3)]
mv = [sp.Matrix([M[i][2, 1], M[i][0, 2], M[i][1, 0]]) for i in range(3)]
PAIRS = [(0, 1), (1, 2), (0, 2)]
cross = lambda v: sp.Matrix([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]])         # noqa: E731
fF = {}
for (i, jj) in PAIRS:
    fF[(i, jj)] = mv[i].cross(mv[jj]) - sum((c(i, jj, k) * mv[k] for k in range(3)), sp.zeros(3, 1))

# unknowns: b_i = f beta_i, beta_i in C^3 (Lie), f top weight of rho_0 (rho_0 f = -2 i m f), J+ f = 0
# rho_1 f = sqrt(j) * gvec, rho_2 f = i sqrt(j) * gvec, gvec unit, orthogonal to f.
# D_ij b has components along f and along gvec.
def deriv_coeffs(i):
    """e^_i f = (coef on f, coef on gvec)"""
    if i == 0:
        return (-2 * sp.I * m / s[0], 0)
    if i == 1:
        return (0, sp.sqrt(j) / s[1])
    return (0, sp.I * sp.sqrt(j) / s[2])

N = 9                                   # index 3*i + a
rows_f, rows_g = [], []
for (i, jj) in PAIRS:
    Df = sp.zeros(3, N); Dg = sp.zeros(3, N)
    di, dj = deriv_coeffs(i), deriv_coeffs(jj)
    Df[:, 3 * jj:3 * jj + 3] += di[0] * sp.eye(3) + cross(mv[i]); Dg[:, 3 * jj:3 * jj + 3] += di[1] * sp.eye(3)
    Df[:, 3 * i:3 * i + 3] -= dj[0] * sp.eye(3) + cross(mv[jj]); Dg[:, 3 * i:3 * i + 3] -= dj[1] * sp.eye(3)
    for k in range(3):
        Df[:, 3 * k:3 * k + 3] -= c(i, jj, k) * sp.eye(3)
    rows_f.append(Df); rows_g.append(Dg)
Dall = sp.Matrix.vstack(*(rows_f + rows_g))
H = (Dall.H * Dall)
for (i, jj) in PAIRS:
    P = -cross(fF[(i, jj)])
    H[3 * i:3 * i + 3, 3 * jj:3 * jj + 3] += P
    H[3 * jj:3 * jj + 3, 3 * i:3 * i + 3] += P.T
H = sp.simplify(H)
# restrict to the squashed plane: frame {1,2} x Lie {1,2}
idx = [3 * 1 + 1, 3 * 1 + 2, 3 * 2 + 1, 3 * 2 + 2]
Hs = sp.simplify(H.extract(idx, idx))
print("coupling of the plane block to the rest (should vanish):",
      sp.simplify(H.extract(idx, [k for k in range(9) if k not in idx])))
print("4x4 block:"); sp.pprint(Hs)
ev = Hs.eigenvals()
for e, mult in ev.items():
    print("eigenvalue:", sp.factor(sp.simplify(e)), " x", mult)

lam = (j**2 + 2 * j - 14) / 3
ns = (Hs - lam * sp.eye(4)).nullspace()
v = sp.simplify(ns[0])
print("eigenvector for (j^2+2j-14)/3:", list(v))
rest = [k for k in range(9) if k not in idx]
coup = H.extract(rest, idx) * v
print("H(rest, plane) v =", sp.simplify(coup).T, " -> exact eigenvector of the 9x9 top-weight block:", sp.simplify(coup) == sp.zeros(len(rest), 1))
# gauge orthogonality at top weight: gauge directions b_i = e^_i xi + [M_i, xi], xi = f*x (x in C^3); f-component of b_i:
G = sp.zeros(9, 3)
for i in range(3):
    G[3 * i:3 * i + 3, :] = deriv_coeffs(i)[0] * sp.eye(3) + cross(mv[i])
vfull = sp.zeros(9, 1)
for p, k in enumerate(idx):
    vfull[k] = v[p]
print("overlap with gauge directions at the same weight (G^H v):", sp.simplify(G.H * vfull).T)

# leak into the next weight: inputs g (x) e_k, g = rho_1 f / sqrt(j) (so rho_2 f = i sqrt(j) g).
# rho_1 g = -sqrt(j) f + (lower), rho_2 g = i sqrt(j) f + (lower), rho_0 g = -2 i (m-1) g; lower parts are
# orthogonal to D v (whose outputs lie in span{f, g}), so they drop out of <D(g e_k), D v>.
def deriv_g(i):
    if i == 0:
        return (0, -2 * sp.I * (m - 1) / s[0])          # (coef on f, coef on g)
    if i == 1:
        return (-sp.sqrt(j) / s[1], 0)
    return (sp.I * sp.sqrt(j) / s[2], 0)
rows_f, rows_g = [], []
for (i, jj) in PAIRS:
    Df = sp.zeros(3, N); Dg = sp.zeros(3, N)
    di, dj = deriv_g(i), deriv_g(jj)
    Df[:, 3 * jj:3 * jj + 3] += di[0] * sp.eye(3); Dg[:, 3 * jj:3 * jj + 3] += di[1] * sp.eye(3) + cross(mv[i])
    Df[:, 3 * i:3 * i + 3] -= dj[0] * sp.eye(3); Dg[:, 3 * i:3 * i + 3] -= dj[1] * sp.eye(3) + cross(mv[jj])
    for k in range(3):
        Dg[:, 3 * k:3 * k + 3] -= c(i, jj, k) * sp.eye(3)
    rows_f.append(Df); rows_g.append(Dg)
DG = sp.Matrix.vstack(*(rows_f + rows_g))           # same output ordering as Dall (f-blocks, then g-blocks)
leak = sp.simplify(DG.H * (Dall * vfull))
print("leak into the next weight, <D(g e_k), D v>:", leak.T, " zero:", leak == sp.zeros(9, 1))
print("Rayleigh check:", sp.simplify((vfull.H * H * vfull)[0] / (vfull.H * vfull)[0]))
