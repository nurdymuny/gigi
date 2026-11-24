"""Truncation diagnostic for dome functions, kept separate so the registered dome sources stay byte-identical."""
from fractions import Fraction as F

import mpmath as mp

import ym_helike_j0 as j0
from ym_helike_dome import ONE, Moments, _mpf, dome_polynomial, mono_mul


def truncation_diagnostic(N, g2, alpha, beta, K, K_ref, m=ONE, dps=80):
    """How much the Haar-series truncation changes a dome function, measured two ways.

    total:   ||Phi_Kref - Phi_K|| / ||Phi_Kref||
    outside: ||P_perp(Phi_Kref - Phi_K)|| / ||P_perp Phi_Kref||, with P_perp the projection orthogonal to the origin
             basis at cutoff N. This is the part the dome adds, so this is the ratio a truncation choice must control.
    Uses only overlaps of trial functions: no Hamiltonian, no residual, no hypothesis outcome."""
    with mp.workdps(dps):
        g = mp.sqrt(_mpf(F(g2).limit_denominator(10 ** 12)))
        al = _mpf(F(alpha).limit_denominator(10 ** 12))
        be = _mpf(F(beta).limit_denominator(10 ** 12))
        Mo = Moments()
        orig = [{mm: mp.mpf(1)} for mm in j0.basis(N)]
        n = len(orig)
        Soo = mp.matrix(n, n)
        for i in range(n):
            for j in range(i, n):
                Soo[i, j] = Soo[j, i] = Mo.inner(orig[i], al, orig[j], al)
        Sinv = mp.inverse(Soo)

        def perp2(f):
            b = mp.matrix([Mo.inner(o, al, f, be) for o in orig])
            return Mo.inner(f, be, f, be) - (b.T * Sinv * b)[0]

        ref = {mono_mul(m, k): v for k, v in dome_polynomial(K_ref, be / g).items()}
        low = {mono_mul(m, k): v for k, v in dome_polynomial(K, be / g).items()}
        tail = {k: ref[k] - low.get(k, 0) for k in ref if ref[k] != low.get(k, 0)}
        full2, pf2, pt2 = Mo.inner(ref, be, ref, be), perp2(ref), perp2(tail)
        return {"N": N, "g2": float(g2), "beta": float(beta), "K": K, "K_ref": K_ref,
                "outside_fraction_of_dome": float(pf2 / full2),
                "total": float(mp.sqrt(Mo.inner(tail, be, tail, be) / full2)),
                "outside": float(mp.sqrt(abs(pt2) / pf2))}
