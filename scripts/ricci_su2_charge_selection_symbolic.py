"""Exact proofs of the two algebraic steps the all-sector floor proof rests on (t symbolic, l and x symbolic).

(1) Charge selection. In the charge basis of frame (x) Lie (eigenbasis of the rotation about the long axis, charges
    c = c_f + c_l): Omega_0 is diagonal, Omega_+ = Omega_1 + i Omega_2 raises c by exactly 1, Omega_- lowers it by 1,
    and R0(t) preserves c. With rho'_+- = -2i J_+- shifting the weight m by +-1, every term of
    H + G G^* = X_0^* X_0 + (X_+^* X_+ + X_-^* X_-)/2 + R0 then preserves q = m + c, so each sector is the orthogonal
    direct sum of its charge blocks, and the spectrum of the sector is the union of the block spectra.
(3) Mirror: G(x, a) = G(2l - x, -a), so rows with x in [0, l] cover every row.
(2) The Gershgorin quantity. For the state (m, a), m = l - x, of type a = (c_f, c_l), the diagonal of its block minus
    lambda_t(2l) minus the same-m couplings R_a minus the ladder radius equals the closed form G of
    scripts/ricci_su2_floor_large_j.py, identically in l, x, t; and every nonzero entry of Omega_+- has modulus sqrt(2t).
usage: python scripts/ricci_su2_charge_selection_symbolic.py OUT.txt
"""
import sys
from pathlib import Path

import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_block_rows_symbolic as BR     # noqa: E402
import ricci_su2_floor_large_j as LJ           # noqa: E402

t = BR.t


def main():
    ch, w, C, Op, Omn, offd = BR.symbolic_pieces()
    c = [a + b for a, b in ch]
    lines = []
    z = lambda e: sp.simplify(e) == 0                                                      # noqa: E731
    ok_diag = offd == sp.zeros(9, 9)
    ok_plus = all(z(Op[b, a]) for a in range(9) for b in range(9) if c[b] != c[a] + 1)
    ok_minus = all(z(Omn[b, a]) for a in range(9) for b in range(9) if c[b] != c[a] - 1)
    # R0 alone (C = -(Omega_1^2 + Omega_2^2) + R0, and Omega_1^2 + Omega_2^2 = (Op Omn + Omn Op)/2)
    R0c = sp.simplify(C + (Op * Omn + Omn * Op) / 2)
    ok_R0 = all(z(R0c[b, a]) for a in range(9) for b in range(9) if c[b] != c[a])
    ok_C = all(z(C[b, a]) for a in range(9) for b in range(9) if c[b] != c[a])
    ok_w = all(z(w[a] - c[a] * (t - 2) / sp.sqrt(t)) for a in range(9))
    mods = {sp.simplify(sp.Abs(M[b, a])) for M in (Op, Omn) for a in range(9) for b in range(9) if not z(M[b, a])}
    lines += [f"Omega_0 diagonal in the charge basis: {ok_diag}",
              f"Omega_0 = i diag(w_a), w_a = c(t-2)/sqrt(t): {ok_w}",
              f"Omega_+ raises c by exactly 1: {ok_plus}",
              f"Omega_- lowers c by exactly 1: {ok_minus}",
              f"R0(t) preserves c: {ok_R0}   (so C = -(Omega_1^2+Omega_2^2) + R0 preserves c: {ok_C})",
              f"moduli of the nonzero entries of Omega_+-: {mods}"]
    # (2) Gershgorin closed form
    l, x = sp.symbols("l x", positive=True)
    m = l - x
    lam = ((2 * l) * (2 * l + 2) - 2 * (t - 3) * (2 * l) - 2 * (t - 2) * (t + 4)) / t
    ok_G = True
    for k, a in enumerate(ch):
        cc = c[k]
        Ca = C[k, k]
        Ra = sum(sp.Abs(C[b, k]) for b in range(9) if b != k)
        npl = sum(1 for b in range(9) if not z(Op[b, k]))
        nmi = sum(1 for b in range(9) if not z(Omn[b, k]))
        D = (w[k] - 2 * m / sp.sqrt(t)) ** 2 + 4 * (l * (l + 1) - m ** 2) + Ca
        # ladder: to m+1 via Omega_- (n_- = nmi entries, each 2 sqrt((l-m)(l+m+1)) sqrt(2t)), to m-1 via Omega_+
        radius = Ra + 2 * sp.sqrt(2 * t) * (nmi * sp.sqrt((l - m) * (l + m + 1)) + npl * sp.sqrt((l + m) * (l - m + 1)))
        Cs, Rs, np_, nm_ = LJ.TYPES[tuple(a)]
        E = cc ** 2 * (t - 2) ** 2 / t + 4 * cc * (t - 2) * x / t + sp.sympify(Cs, locals={"t": t}) \
            - sp.sympify(Rs, locals={"t": t}) + 2 * (t - 2) * (t + 4) / t
        G = 4 * x * (2 * l - x) * (1 - 1 / t) + 4 * l * (2 - cc) * (t - 2) / t + E \
            - 2 * sp.sqrt(2 * t) * (nm_ * sp.sqrt(x * (2 * l - x + 1)) + np_ * sp.sqrt((2 * l - x) * (x + 1)))
        diff = sp.simplify(sp.expand(D - lam - radius - G))
        good = diff == 0 and (np_, nm_) == (npl, nmi) and z(Ra - sp.sympify(Rs, locals={"t": t}))
        ok_G &= good
        lines.append(f"type {tuple(a)}: D - lambda - radius - G = {diff}; counts (n+, n-) = ({npl}, {nmi}) {'ok' if good else 'MISMATCH'}")
    lines.append(f"Gershgorin closed form exact for all nine types: {ok_G}")
    # (3) mirror: the row (m, a) and the row (-m, -a) have the same G (x -> 2l - x), so x in [0, l] suffices
    def Gx(xx, a):
        cc = a[0] + a[1]; Cs, Rs, np_, nm_ = LJ.TYPES[a]
        E = cc ** 2 * (t - 2) ** 2 / t + 4 * cc * (t - 2) * xx / t + sp.sympify(Cs, locals={"t": t}) \
            - sp.sympify(Rs, locals={"t": t}) + 2 * (t - 2) * (t + 4) / t
        return 4 * xx * (2 * l - xx) * (1 - 1 / t) + 4 * l * (2 - cc) * (t - 2) / t + E \
            - 2 * sp.sqrt(2 * t) * (nm_ * sp.sqrt(xx * (2 * l - xx + 1)) + np_ * sp.sqrt((2 * l - xx) * (xx + 1)))
    ok_mirror = all(z(sp.expand(Gx(x, a) - Gx(2 * l - x, (-a[0], -a[1])))) for a in LJ.TYPES)
    lines.append(f"mirror G(x, a) = G(2l - x, -a) for all nine types: {ok_mirror}")
    ok_G &= ok_mirror
    allok = ok_diag and ok_plus and ok_minus and ok_R0 and ok_w and mods == {sp.sqrt(2) * sp.sqrt(t)} and ok_G
    lines.append("ALL EXACT" if allok else "NOT ALL EXACT")
    txt = "\n".join(lines); print(txt)
    Path(sys.argv[1]).write_text(txt + "\n")


if __name__ == "__main__":
    main()
