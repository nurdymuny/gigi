"""The Weitzenbock identity and the charge-block rows of the circle-symmetric family, proved symbolically (22 Nov 2026).

Closes the gap "the closed-form charge-block rows are the blocks of H + G G^*" (edge paper review, findings S5-1 and
S6-1). Everything below is exact sympy arithmetic with x, y real symbols, t a positive symbol, and l a symbol.

CONVENTIONS (stated explicitly; they are the ones the code has always used)
  * Frame. S^3 = SU(2) with the left-invariant frame e_0, e_1, e_2, [e_a, e_b] = 2 eps_abc e_c. Metric (t, 1, 1),
    orthonormal frame E_i = e_i / s_i, s = (sqrt t, 1, 1); [E_i, E_k] = c_ik^l E_l with c_ik^l = 2 eps_ikl s_l/(s_i s_k).
    Levi-Civita: Gamma_iab = <nabla_{E_i} E_a, E_b> = (c_ia^b - c_ab^i + c_bi^a) / 2 (Koszul, constant coefficients).
  * Gauge algebra. su(2) with basis tau_0, tau_1, tau_2 and
        [tau_a, tau_b] = eps_abc tau_c        (THE CROSS PRODUCT: [u, v] = u x v on R^3 = su(2)),
    so ad(M) acts on Lie components as the matrix cross(m). (tau_a corresponds to e_a / 2; NOT the frame's factor 2.)
  * Field. A = sum_i M_i theta^i (orthonormal coframe), family m_0 = x e_0, m_1 = y e_1, m_2 = y e_2.
    Curvature F_ik = m_i x m_k - sum_l c_ik^l m_l (constant).
  * Operator. b = sum_i b_i theta^i, b_i : SU(2) -> su(2). D_ik b = nabla_i b_k - nabla_k b_i - sum_l c_ik^l b_l with
    nabla_i = E_i + m_i x. Q(b) = sum_{i<k} |D_ik b|^2 + 2 <F_ik, [b_i, b_k]>; H is its operator against the L2 mass
    sum_i |b_i|^2, i.e. H = D^* D + K with K the symmetric matrix whose (i,k) block (i<k) is -cross(F_ik) and whose
    (k,i) block is its transpose (because x^T (-cross F) y = <F, x x y>). G xi = (nabla_i xi)_i.
    This is exactly the general builder scripts/ricci_su3_test2_family.py hessian_and_gauge specialised to SU(2).
  * Peter-Weyl. E_i acts as rho_i / s_i with rho_i abstract generators of U(su(2)), [rho_a, rho_b] = 2 eps_abc rho_c
    (cyclic), rho_a^* = -rho_a (unitarity of the right-regular representation; E_i^* = -E_i since SU(2) is unimodular).

PART 1 (PROVED, exact, all x, y real, t > 0). In 9 x 9 matrices over U(su(2)) (rows/cols (frame i, Lie a)):
        H + G G^* = sum_i X_i^* X_i + R0,   X_i = rho_i / s_i + Omega_i,   Omega_i = I (x) cross(m_i) - Gamma_i (x) I,
        R0 = Ric (x) I - 2 [F, .],   ([F, .] b)_i = sum_k [F_ik, b_k],   Ric = diag(2t, 4 - 2t, 4 - 2t).
  Method: every product is put in PBW normal order (rho_a rho_b with a <= b, using rho_b rho_a = rho_a rho_b
  - 2 eps_abc rho_c ... i.e. rho_a rho_b -> rho_b rho_a + 2 eps_abc rho_c for a > b); the matrix coefficient of each PBW
  monomial (1, rho_a, rho_a rho_b with a <= b) is compared exactly. The PBW theorem makes these monomials linearly
  independent in U(su(2)), so the identity holds in U(su(2)) and hence in every representation (every sector j).
  Also proved: Ric = diag(2t, 4-2t, 4-2t) is the Ricci tensor of (t, 1, 1), computed from Gamma; Gamma_i is
  antisymmetric; the Berger Levi-Civita field is the family member x = (2-t)/sqrt t, y = sqrt t; F matches eq. (B).
  Bonus (same method): the identity with R0 = Ric (x) I - 2[F, .] for an arbitrary constant M (9 free symbols).

PART 2 (PROVED, exact). The rows used by scripts/ricci_su2_exact_edge.py symbolic_rows follow from Part 1.
  Charge basis: U = U3 (x) U3, U3 columns = charge -1, 0, +1 (the same U3 as symbolic_rows); states a = (c_f, c_l),
  c_f, c_l in {-1, 0, 1}, frame index first; Q_int = diag(c_f + c_l). Omc_i = U^H Omega_i U.
  Proved: U unitary; Omega_i real antisymmetric and Omc_i antihermitian; Omc_0 diagonal, = i diag(w_a) with w_a real;
  Omega_+- = Omc_1 +- i Omc_2 satisfy [Q_int, Omega_+] = Omega_+, [Q_int, Omega_-] = -Omega_- (raise/lower the combined
  charge by 1), Omega_- = -Omega_+^H; C = -(Omc_1^2 + Omc_2^2) + U^H R0 U is Hermitian with [Q_int, C] = 0. And, as an
  identity in U(su(2)) (x) Mat_9 (PBW, exact), with J_z = (i/2) rho_0, J_+- = (i/2)(rho_1 +- i rho_2):
        U^H (H + G G^*) U = (W - 2 J_z / sqrt t)^2 + 2 (J_+ J_- + J_- J_+) + C + 2i (J_+ Omega_- + J_- Omega_+),
  W = diag(w_a). With J_z|l,m> = m|l,m>, J_+-|l,m> = sqrt((l -+ m)(l +- m + 1))|l,m+-1> this is entry for entry the
  block matrix of symbolic_rows / Blocks.block: diagonal (w_a - 2m/sqrt t)^2 + 4(l(l+1) - m^2) + C_aa, coupling
  2i sqrt((l-m)(l+m+1)) (Omega_-)_{a'a} to m+1 and 2i sqrt((l+m)(l-m+1)) (Omega_+)_{a'a} to m-1; the total charge
  q = m + c_f + c_l is conserved. The twist row: D(l, (1,1)) - lambda = 0 identically in l, x, y, t, with
  lambda = (x + (2l+2)/sqrt t)^2 + 2 sqrt(t) x - y^2; the charge-(l+2) block is 1 x 1; likewise the mirror
  D(-l, (-1,-1)) = lambda in the charge -(l+2) block.

ASSUMED (standard, not re-proved here): (a) the Peter-Weyl decomposition: on sector j the frame fields act through the
spin-j/2 representation, in which J = ((i/2)rho_1, (i/2)rho_2, (i/2)rho_0) is a standard angular-momentum triple with
an orthonormal basis |l,m> (l = j/2) and the standard matrix elements of J_z, J_+- above; (b) that Q above (with the
L2 inner product given by the Killing-normalised dot product on su(2) and the orthonormal frame) is the second
variation of E = 1/2 sum_{i<k} |F_ik|^2, which is the two-line expansion F(A + b) = F + D b + [b_i, b_k].

PART 3 (COMPUTED, double precision): the exact rows here, lambdified, agree with the rows lambdified in
scripts/ricci_su2_exact_edge.py symbolic_rows at sample points (including the certified domain x in [-3, -1.5],
t <= 10), i.e. the code the certificates use is this construction (no transcription drift).

usage: python scripts/ricci_su2_weitzenbock_family_symbolic.py [OUT.json]
       default OUT = research/edge-theorem-20261122-proofs/weitzenbock_family.json (repo-relative)
"""
import json
import sys
import time
from itertools import product
from pathlib import Path

import sympy as sp

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUT = ROOT / "research/edge-theorem-20261122-proofs/weitzenbock_family.json"

x, y = sp.symbols("x y", real=True)
t = sp.symbols("t", positive=True)
lsym = sp.symbols("l", real=True)
I3, I9 = sp.eye(3), sp.eye(9)
PAIRS = [(0, 1), (1, 2), (0, 2)]
EPS = {(0, 1, 2): 1, (1, 2, 0): 1, (2, 0, 1): 1, (1, 0, 2): -1, (2, 1, 0): -1, (0, 2, 1): -1}
FRAME_BRACKET = 2          # [e_a, e_b] = 2 eps_abc e_c
RHO_BRACKET = 2            # [rho_a, rho_b] = 2 eps_abc rho_c  (rho_a represents e_a)
# gauge algebra: [tau_a, tau_b] = eps_abc tau_c  (cross product), encoded by cross() below


def cross(v):
    """ad(v) on R^3 = su(2) under [tau_a, tau_b] = eps_abc tau_c: cross(v) w = v x w."""
    return sp.Matrix([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]])


def zero(M):
    return all(sp.simplify(e) == 0 for e in M)


# ---------------------------------------------------------------- the metric (t, 1, 1)
def geometry():
    s = [sp.sqrt(t), sp.Integer(1), sp.Integer(1)]
    c = {(i, k, l): FRAME_BRACKET * EPS.get((i, k, l), 0) * s[l] / (s[i] * s[k]) for i, k, l in product(range(3), repeat=3)}
    Gam = {(i, a, b): (c[(i, a, b)] - c[(a, b, i)] + c[(b, i, a)]) / 2 for i, a, b in product(range(3), repeat=3)}
    gmat = [sp.Matrix(3, 3, lambda a, b: Gam[(i, a, b)]) for i in range(3)]       # gmat[i][a, b] = <nabla_i E_a, E_b>
    return s, c, Gam, gmat


def ricci(c, Gam):
    """Ric(E_b, E_c) = sum_a <R(E_a, E_b) E_c, E_a>, R(X,Y) = [nabla_X, nabla_Y] - nabla_[X,Y], constant Gamma."""
    def R(a, b, cc, e):        # <R(E_a, E_b) E_c, E_e>
        v = sum(Gam[(b, cc, d)] * Gam[(a, d, e)] - Gam[(a, cc, d)] * Gam[(b, d, e)] for d in range(3))
        return v - sum(c[(a, b, k)] * Gam[(k, cc, e)] for k in range(3))
    return sp.Matrix(3, 3, lambda b, cc: sp.simplify(sum(R(a, b, cc, a) for a in range(3))))


def curvature(m, c):
    return {(i, k): sp.simplify(cross(m[i]) * m[k] - sum((c[(i, k, l)] * m[l] for l in range(3)), sp.zeros(3, 1)))
            for i in range(3) for k in range(3)}


# ---------------------------------------------------------------- matrices over U(su(2))
class Op:
    """rows x cols matrix over U(su(2)) (x) C: {word: sympy Matrix}; a word is a tuple of generator indices."""

    def __init__(self, rows, cols, terms=None):
        self.rows, self.cols, self.t = rows, cols, {}
        for w, mm in (terms or {}).items():
            self.add(w, mm)

    def add(self, word, mat):
        self.t[word] = self.t.get(word, sp.zeros(self.rows, self.cols)) + sp.Matrix(mat)

    def __add__(self, o):
        r = Op(self.rows, self.cols, self.t)
        for w, mm in o.t.items():
            r.add(w, mm)
        return r

    def __neg__(self):
        return Op(self.rows, self.cols, {w: -mm for w, mm in self.t.items()})

    def __sub__(self, o):
        return self + (-o)

    def scale(self, k):
        return Op(self.rows, self.cols, {w: k * mm for w, mm in self.t.items()})

    def __mul__(self, o):
        # generators act on the Peter-Weyl factor, coefficient matrices on frame (x) Lie: they commute
        r = Op(self.rows, o.cols)
        for w, a in self.t.items():
            for u, b in o.t.items():
                r.add(w + u, a * b)
        return r

    def dag(self):
        """(C rho_w)^* = C^H rho_w^*, rho_w^* = (-1)^len(w) reversed(w)."""
        return Op(self.cols, self.rows, {tuple(reversed(w)): (-1) ** len(w) * mm.H for w, mm in self.t.items()})

    def normal(self):
        """PBW normal order for words of length <= 2: rho_a rho_b (a > b) -> rho_b rho_a + 2 eps_abc rho_c."""
        r = Op(self.rows, self.cols)
        for w, mm in self.t.items():
            assert len(w) <= 2, "degree > 2 cannot occur here"
            if len(w) == 2 and w[0] > w[1]:
                a, b = w
                r.add((b, a), mm)
                for cc in range(3):
                    e = EPS.get((a, b, cc), 0)
                    if e:
                        r.add((cc,), RHO_BRACKET * e * mm)
            else:
                r.add(w, mm)
        return r

    def residual_words(self):
        n = self.normal()
        return {str(w): str(sp.simplify(mm)) for w, mm in n.t.items() if not zero(mm)}


def blockmat(rows, cols, placements):
    M = sp.zeros(3 * rows, 3 * cols)
    for (bi, bj), blk in placements:
        M[3 * bi:3 * bi + 3, 3 * bj:3 * bj + 3] += sp.Matrix(blk)
    return M


def build(m, s, c, gmat, F):
    """H + GG^* (as D^*D + K + GG^*) and the X_i, all as matrices over U(su(2))."""
    D = Op(9, 9)
    for p, (i, k) in enumerate(PAIRS):
        D.add((i,), blockmat(3, 3, [((p, k), I3 / s[i])]))
        D.add((), blockmat(3, 3, [((p, k), cross(m[i]))]))
        D.add((k,), blockmat(3, 3, [((p, i), -I3 / s[k])]))
        D.add((), blockmat(3, 3, [((p, i), -cross(m[k]))]))
        for l in range(3):
            D.add((), blockmat(3, 3, [((p, l), -c[(i, k, l)] * I3)]))
    K = Op(9, 9)
    for (i, k) in PAIRS:
        blk = -cross(F[(i, k)])                     # x^T (-cross F) y = <F, [x, y]>
        K.add((), blockmat(3, 3, [((i, k), blk), ((k, i), blk.T)]))
    G = Op(9, 3)
    for i in range(3):
        G.add((i,), blockmat(3, 1, [((i, 0), I3 / s[i])]))
        G.add((), blockmat(3, 1, [((i, 0), cross(m[i]))]))
    Om = [sp.kronecker_product(I3, cross(m[i])) - sp.kronecker_product(gmat[i], I3) for i in range(3)]
    X = [Op(9, 9, {(i,): I9 / s[i], (): Om[i]}) for i in range(3)]
    lhs = D.dag() * D + K + G * G.dag()
    sumXX = Op(9, 9)
    for Xi in X:
        sumXX = sumXX + Xi.dag() * Xi
    return lhs, sumXX, Om


def r0_formula(F, Ric):
    T = sp.zeros(9, 9)
    for i in range(3):
        for k in range(3):
            T[3 * i:3 * i + 3, 3 * k:3 * k + 3] = cross(F[(i, k)])
    return sp.kronecker_product(Ric, I3) - 2 * T


def part1_identity(m, s, c, gmat, F, Ric):
    lhs, sumXX, Om = build(m, s, c, gmat, F)
    diff = (lhs - sumXX).normal()
    nonconst = {str(w): str(sp.simplify(mm)) for w, mm in diff.t.items() if len(w) > 0 and not zero(mm)}
    R0 = diff.t.get((), sp.zeros(9, 9)).applyfunc(sp.simplify)
    R0f = r0_formula(F, Ric)
    return {"nonconstant_residual_words": nonconst, "R0_minus_formula_zero": zero(R0 - R0f),
            "R0_symmetric": zero(R0 - R0.T)}, lhs, sumXX, Om, R0


# ---------------------------------------------------------------- part 2: the charge basis rows
def part2_rows(Om, R0, lhs):
    r2 = sp.sqrt(2)
    U3 = sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-sp.I / r2, 0, sp.I / r2]])     # as symbolic_rows
    U = sp.kronecker_product(U3, U3)
    ch = [(cf, cl) for cf in (-1, 0, 1) for cl in (-1, 0, 1)]                         # as symbolic_rows
    Qint = sp.diag(*[cf + cl for cf, cl in ch])
    tr = lambda M: (U.H * M * U).applyfunc(sp.simplify)                               # noqa: E731
    res = {"U_unitary": zero(U.H * U - I9)}
    res["U3_columns_are_charge_-1_0_+1_under_cross_e0"] = zero(
        U3.H * cross([1, 0, 0]) * U3 - sp.diag(sp.I, 0, -sp.I)) and zero(U3.H * U3 - I3)
    res["Omega_i_real_antisymmetric"] = all(zero(O + O.T) and all(sp.im(sp.simplify(e)) == 0 for e in O) for O in Om)
    Omc = [tr(O) for O in Om]
    res["Omc_i_antihermitian"] = all(zero(O + O.H) for O in Omc)
    res["Omc_0_diagonal"] = zero(Omc[0] - sp.diag(*[Omc[0][a, a] for a in range(9)]))
    w = [sp.simplify(Omc[0][a, a] / sp.I) for a in range(9)]
    res["w_real"] = all(sp.simplify(sp.im(e)) == 0 for e in w)
    Opl = (Omc[1] + sp.I * Omc[2]).applyfunc(sp.simplify)
    Omi = (Omc[1] - sp.I * Omc[2]).applyfunc(sp.simplify)
    R0c = tr(R0)
    C = (-(Omc[1] * Omc[1] + Omc[2] * Omc[2]) + R0c).applyfunc(sp.simplify)
    res["Omega_plus_raises_charge_by_1"] = zero(Qint * Opl - Opl * Qint - Opl)
    res["Omega_minus_lowers_charge_by_1"] = zero(Qint * Omi - Omi * Qint + Omi)
    res["Omega_minus_eq_minus_Omega_plus_H"] = zero(Omi + Opl.H)
    res["C_preserves_charge"] = zero(Qint * C - C * Qint)
    res["C_hermitian"] = zero(C - C.H)
    # the block-operator identity in U(su(2)) (x) Mat_9
    i_ = sp.I
    Jz = Op(9, 9, {(0,): i_ / 2 * I9})
    Jp = Op(9, 9, {(1,): i_ / 2 * I9, (2,): -I9 / 2})            # (i/2)(rho_1 + i rho_2)
    Jm = Op(9, 9, {(1,): i_ / 2 * I9, (2,): I9 / 2})             # (i/2)(rho_1 - i rho_2)
    W = Op(9, 9, {(): sp.diag(*w)})
    A0 = W - Jz.scale(2 / sp.sqrt(t))
    block = A0 * A0 + (Jp * Jm + Jm * Jp).scale(2) + Op(9, 9, {(): C}) \
        + (Jp * Op(9, 9, {(): Omi}) + Jm * Op(9, 9, {(): Opl})).scale(2 * i_)
    lhs_c = Op(9, 9, {w_: tr(mm) for w_, mm in lhs.t.items()})
    res["block_operator_residual_words"] = (lhs_c - block).residual_words()
    res["block_operator_identity_holds"] = not res["block_operator_residual_words"]
    # the twist row and its mirror
    lam = (x + (2 * lsym + 2) / sp.sqrt(t)) ** 2 + 2 * sp.sqrt(t) * x - y ** 2
    Dtw = (w[8] - 2 * lsym / sp.sqrt(t)) ** 2 + 4 * (lsym * (lsym + 1) - lsym ** 2) + C[8, 8]
    Dmi = (w[0] + 2 * lsym / sp.sqrt(t)) ** 2 + 4 * (lsym * (lsym + 1) - lsym ** 2) + C[0, 0]
    res["twist_index_8_is_charge_(1,1)"] = ch[8] == (1, 1) and ch[0] == (-1, -1)
    res["twist_row_D_minus_lambda"] = str(sp.simplify(sp.expand(Dtw - lam)))
    res["mirror_row_D_minus_lambda"] = str(sp.simplify(sp.expand(Dmi - lam)))
    res["twist_row_identically_lambda"] = sp.simplify(sp.expand(Dtw - lam)) == 0
    res["mirror_row_identically_lambda"] = sp.simplify(sp.expand(Dmi - lam)) == 0
    # charge l+2 block is 1x1: m = l+2 - c_f - c_l <= l forces (c_f, c_l) = (1,1), m = l; its couplings:
    # to m+1 vanishes (sqrt((l-l)(...)) = 0); to m-1 needs (Omega_+)_{a',8} != 0 with charge 3: column 8 of Omega_+ = 0
    res["Omega_plus_column_(1,1)_zero"] = zero(Opl[:, 8])
    res["Omega_minus_column_(-1,-1)_zero"] = zero(Omi[:, 0])
    res["twist_block_1x1"] = bool(res["Omega_plus_column_(1,1)_zero"] and res["Omega_minus_column_(-1,-1)_zero"])
    rows = {"charge_states_(c_f,c_l)": ch, "w": [str(e) for e in w],
            "C": [[str(C[a, b]) for b in range(9)] for a in range(9)],
            "Omega_plus": [[str(Opl[a, b]) for b in range(9)] for a in range(9)],
            "Omega_minus": [[str(Omi[a, b]) for b in range(9)] for a in range(9)],
            "C_diagonal": [str(sp.factor(C[a, a])) for a in range(9)]}
    return res, rows, (w, C, Opl, Omi)


# ---------------------------------------------------------------- part 3: tie to the code the certificates use
def part3_tie(rows_exact):
    import numpy as np
    sys.path.insert(0, str(ROOT / "scripts"))
    _argv, sys.argv = sys.argv, [sys.argv[0]]
    import ricci_su2_exact_edge as EE          # noqa: E402
    sys.argv = _argv
    _, f_code = EE.symbolic_rows()
    f_here = sp.lambdify((x, y, t), list(rows_exact), "numpy")
    pts = [(-2.0, 1.5, 3.0), (-3.0, 0.5, 10.0), (-1.5, 3.1, 7.3), (0.7, 0.2, 0.5), (-2.4, 2.0, 1.1), (1.9, 2.9, 8.0)]
    worst = 0.0
    for p in pts:
        a = f_code(*p); b = f_here(*p)
        for A_, B_ in zip(a, b):
            worst = max(worst, float(np.abs(np.array(A_, complex) - np.array(B_, complex)).max()))
    return {"status": "COMPUTED", "points_(x,y,t)": pts, "max_abs_diff_rows_vs_exact_edge_symbolic_rows": worst}


def main():
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OUT
    t0 = time.time()
    s, c, Gam, gmat = geometry()
    Ric = ricci(c, Gam)
    geo = {"Ric_computed": [str(Ric[a, b]) for a in range(3) for b in range(3)],
           "Ric_eq_diag(2t,4-2t,4-2t)": zero(Ric - sp.diag(2 * t, 4 - 2 * t, 4 - 2 * t)),
           "Gamma_i_antisymmetric": all(zero(gm + gm.T) for gm in gmat)}
    # Levi-Civita field as an su(2) field: m_i = vector of the so(3) matrix -Gamma_i acting on frame components
    mLC = [sp.simplify(sp.Matrix([gmat[i][1, 2], gmat[i][2, 0], gmat[i][0, 1]])) for i in range(3)]
    geo["Levi_Civita_field_m"] = [[str(e) for e in v] for v in mLC]
    geo["Levi_Civita_field_is_family_member_x=(2-t)/sqrt(t),y=sqrt(t)"] = zero(
        sp.Matrix.hstack(*mLC) - sp.diag((2 - t) / sp.sqrt(t), sp.sqrt(t), sp.sqrt(t)))
    Ric = sp.diag(2 * t, 4 - 2 * t, 4 - 2 * t)
    m = [sp.Matrix([x, 0, 0]), sp.Matrix([0, y, 0]), sp.Matrix([0, 0, y])]
    F = curvature(m, c)
    B = y ** 2 - 2 * sp.sqrt(t) * x; Pc = x * y - 2 * y / sp.sqrt(t)
    geo["F_matches_eq_B"] = (zero(F[(1, 2)] - sp.Matrix([B, 0, 0])) and zero(F[(0, 1)] - sp.Matrix([0, 0, Pc]))
                             and zero(F[(0, 2)] - sp.Matrix([0, -Pc, 0])))
    print("geometry:", geo, flush=True)
    p1, lhs, sumXX, Om, R0 = part1_identity(m, s, c, gmat, F, Ric)
    p1["identity_holds_in_U_su2_all_x_y_t"] = not p1["nonconstant_residual_words"] and p1["R0_minus_formula_zero"]
    print("part 1 (family):", p1, flush=True)
    # bonus: arbitrary constant M
    gen = sp.symbols("M0:9", real=True)
    mg = [sp.Matrix(gen[3 * i:3 * i + 3]) for i in range(3)]
    Fg = curvature(mg, c)
    p1g, *_ = part1_identity(mg, s, c, gmat, Fg, Ric)
    p1g["identity_holds_in_U_su2_all_M_t"] = not p1g["nonconstant_residual_words"] and p1g["R0_minus_formula_zero"]
    print("part 1 (general constant M):", p1g, flush=True)
    p2, rows, rows_exact = part2_rows(Om, R0, lhs)
    print("part 2:", {k: v for k, v in p2.items() if k != "block_operator_residual_words"}, flush=True)
    p3 = part3_tie(rows_exact)
    print("part 3:", p3, flush=True)
    proved = (geo["Ric_eq_diag(2t,4-2t,4-2t)"] and geo["Gamma_i_antisymmetric"] and geo["F_matches_eq_B"]
              and p1["identity_holds_in_U_su2_all_x_y_t"] and p1["R0_symmetric"]
              and all(p2[k] for k in ("U_unitary", "Omega_i_real_antisymmetric", "Omc_i_antihermitian", "Omc_0_diagonal",
                                      "w_real", "Omega_plus_raises_charge_by_1", "Omega_minus_lowers_charge_by_1",
                                      "Omega_minus_eq_minus_Omega_plus_H", "C_preserves_charge", "C_hermitian",
                                      "block_operator_identity_holds", "twist_row_identically_lambda",
                                      "mirror_row_identically_lambda", "twist_block_1x1")))
    res = {
        "kind": "Weitzenbock identity and charge-block rows of the circle-symmetric SU(2) family, exact (sympy)",
        "script": "scripts/ricci_su2_weitzenbock_family_symbolic.py",
        "conventions": {
            "frame": "[e_a, e_b] = 2 eps_abc e_c; metric (t,1,1); E_i = e_i/s_i, s = (sqrt t, 1, 1)",
            "gauge_algebra": "[tau_a, tau_b] = eps_abc tau_c (the cross product); ad(M) acts as cross(m); tau_a ~ e_a/2",
            "family": "m_0 = x e_0, m_1 = y e_1, m_2 = y e_2; x, y real symbols, t positive symbol",
            "generators": "rho_a represents e_a: [rho_a, rho_b] = 2 eps_abc rho_c, rho_a^* = -rho_a",
            "H": "operator of Q(b) = sum_{i<k} |D_ik b|^2 + 2<F_ik,[b_i,b_k]> against sum_i |b_i|^2; G xi = (E_i xi + m_i x xi)_i",
            "Omega_i": "I (x) cross(m_i) - Gamma_i (x) I, Gamma_i[a,b] = <nabla_{E_i} E_a, E_b>",
            "charge_basis": "U = U3 (x) U3 as in ricci_su2_exact_edge.symbolic_rows; states (c_f, c_l), frame first",
            "J": "J_z = (i/2) rho_0, J_+- = (i/2)(rho_1 +- i rho_2)"},
        "status": "PROVED" if proved else "NOT PROVED",
        "geometry": geo, "part1_family": p1, "part1_general_constant_M": p1g, "part2_rows": p2,
        "exact_rows": rows, "part3_tie_to_code": p3,
        "assumed": [
            "Peter-Weyl: on sector j the frame fields act through the spin-j/2 representation, where "
            "((i/2)rho_1, (i/2)rho_2, (i/2)rho_0) is a standard angular-momentum triple with orthonormal basis |l,m>, "
            "J_z|l,m> = m|l,m>, J_+-|l,m> = sqrt((l -+ m)(l +- m + 1))|l,m+-1>, l = j/2 (standard).",
            "Q above is the second variation of E = 1/2 sum_{i<k}|F_ik|^2 (from F(A+b) = F + D b + [b_i,b_k]); "
            "this is the definition of H used by all the scripts (hessian_and_gauge) and the paper.",
            "rho_a^* = -rho_a in L2 (unitary right-regular representation of a unimodular group)."],
        "seconds": round(time.time() - t0, 1)}
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(res, indent=1, default=str) + "\n")
    print(res["status"], res["seconds"], "s ->", out, flush=True)


if __name__ == "__main__":
    import _receipt_status                     # exit 1 if the receipt is missing, stale or records a failed check
    _receipt_status.begin()
    main()
    _receipt_status.exit_on_failure(sys.argv[1])
