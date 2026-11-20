"""Adversarial tests of scripts/ricci_su2_outward.py (Astra's review, finding 1), checked at a precision four times
the working one: adjacent endpoints, narrow intervals around large entries, singletons, negative and straddling
intervals, cancellation in shifted diagonals, and every scalar entry of the actual j = 4, q = 0, t = 3 charge block at
the block certificate's 64-bit precision (where the old conversion missed 12 of 90 endpoints).
usage: python scripts/ricci_su2_outward_test.py OUT.txt
"""
import sys
from pathlib import Path

import mpmath

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_outward as O     # noqa: E402

iv = mpmath.iv


def covers(v, m, r, prec):
    """At 4x precision: both endpoints of v within r of m."""
    save = mpmath.mp.prec
    a, b = mpmath.mpf(v.a), mpmath.mpf(v.b)
    mpmath.mp.prec = 4 * prec
    ok = abs(b - m) <= r and abs(m - a) <= r
    mpmath.mp.prec = save
    return ok


def old_rad(v, prec):
    a, b = mpmath.mpf(v.a), mpmath.mpf(v.b)
    return (a + b) / 2, ((b - a) / 2) * (1 + mpmath.mpf(2) ** (8 - prec))


def main():
    lines = []
    for prec in (64, 96, 200):
        iv.prec = prec; mpmath.mp.prec = prec
        e = mpmath.mpf(2) ** (1 - prec)
        cases = {
            "adjacent endpoints at 1": iv.mpf([1, 1 + e]),
            "adjacent endpoints at 1e30": iv.mpf([mpmath.mpf(10) ** 30, mpmath.mpf(10) ** 30 * (1 + e)]),
            "narrow around -7.3e12": iv.mpf([-mpmath.mpf("7.3e12") * (1 + 3 * e), -mpmath.mpf("7.3e12")]),
            "singleton 1/3": iv.mpf(mpmath.mpf(1) / 3),
            "straddling zero": iv.mpf([-e, 3 * e]),
            "cancellation: [x] - x for x = 2/3 + 1e20": (iv.mpf(2) / 3 + iv.mpf(10) ** 20) - (iv.mpf(2) / 3 + iv.mpf(10) ** 20),
            "shifted diagonal: [10/3] - 10/3": iv.mpf(10) / 3 - iv.mpf(mpmath.mpf(10) / 3),
        }
        for name, v in cases.items():
            m, r = O.mid_rad(v)
            mo, ro = old_rad(v, prec)
            lines.append(f"prec {prec:3d} {name:42s}: new covers {covers(v, m, r, prec)}; old covers {covers(v, mo, ro, prec)}")
    # the actual block (Astra's reproduction)
    import ricci_su2_block_interval_cert as BC
    iv.prec = BC.PREC; mpmath.mp.prec = BC.PREC
    Tv = iv.mpf(3)
    fd = BC.frame_iv(Tv)
    _, B = BC.block_iv(4, 0, Tv, fd)
    n = len(B); new_bad = old_bad = tot = 0
    for x in range(n):
        for y in range(x + 1):
            for comp in B[x][y]:
                v = comp if isinstance(comp, type(iv.mpf(0))) else iv.mpf(comp)
                tot += 1
                m, r = O.mid_rad(v); new_bad += not covers(v, m, r, BC.PREC)
                mo, ro = old_rad(v, BC.PREC); old_bad += not covers(v, mo, ro, BC.PREC)
    lines.append(f"j = 4, q = 0, t = 3 block at {BC.PREC} bits: {tot} scalar intervals; old conversion misses {old_bad}, "
                 f"new misses {new_bad}")
    allok = all("new covers True" in s for s in lines if "covers" in s) and new_bad == 0
    lines.append("ALL NEW ENCLOSURES HOLD" if allok else "FAILURE")
    txt = "\n".join(lines); print(txt)
    Path(sys.argv[1]).write_text(txt + "\n")
    sys.exit(0 if allok else 1)


if __name__ == "__main__":
    main()
