"""S_C rigorous build, R1.4 at the ENCLOSED constants (R1 registration, R1.7 status item 1).

Per control, the B3' product base on B_u(5) x B_v(2):
    soft:  -1/2 Lap + 1/2 w2s r^2 + vJ,  Robin beta_u;   stiff: -1/2 Lap + 1/2 w2t r^2,  Robin beta_v,
with every constant rounded so the base stays BELOW the true inside operator:
    w2s = lower(w2_soft_min - 2 C3 sqrt(Lu^2 + Lv^2)), w2t likewise (C3 the enclosed upper bound),
    beta the enclosed upper bound (a larger Robin coefficient lowers the form), vJ the enclosed lower bound.
Enclosed radial levels (helike_sc_r1_spectrum.RadialBlock), product levels below rho_b with multiplicity, and the
checks of R1.4: count against the floating Galerkin base at the same constants; widths.
usage: python scripts/helike_sc_r1_base_enclosed.py --g2=0.004 --c3=0.075753 --vj=-0.001616 --beta-u=0.071876 --beta-v=0.005457
"""
import sys
import time

from flint import arb

import helike_sc_r1 as r1
from helike_sc_r1_spectrum import RadialBlock

LU, LV = 5, 2
E0REL = {0.004: "5.40626", 0.002: "5.40917"}      # registered ground centres minus V_bg (gbase CONTROLS)
RHO_B_REL = "2.32"


def arg(name):
    return [a.split("=", 1)[1] for a in sys.argv if a.startswith(f"--{name}=")][0]


def main():
    g2 = float(arg("g2")); t0 = time.time()
    M = r1.Model(g2)
    C3, vJ, bu, bv = arb(arg("c3")), arb(arg("vj")), arb(arg("beta-u")), arb(arg("beta-v"))
    Lr = (arb(LU) ** 2 + arb(LV) ** 2).sqrt()
    w2s_min = min(M.w2[:3], key=lambda x: float(x.mid())); w2t_min = min(M.w2[3:], key=lambda x: float(x.mid()))
    w2s = arb((w2s_min - 2 * C3 * Lr).lower()); w2t = arb((w2t_min - 2 * C3 * Lr).lower())
    rho_b = arb(E0REL[g2]) + arb(RHO_B_REL)
    print(f"=== g2 = {g2}: w2s = {float(w2s.mid()):.6f}  w2t = {float(w2t.mid()):.6f}  vJ = {arg('vj')}  "
          f"beta_u = {arg('beta-u')}  beta_v = {arg('beta-v')}  rho_b = {float(rho_b.mid()):.5f}", flush=True)
    soft = RadialBlock(w2s, arb(LU), bu, vJ); stiff = RadialBlock(w2t, arb(LV), bv, arb(0))
    # the stiff block's lowest level bounds what the soft block may carry, and vice versa
    st_spec, st_l = stiff.spectrum(float(rho_b.upper()) - float(vJ.lower()) + 0.5)
    st_min = min(r for _, r in st_spec)
    so_spec, so_l = soft.spectrum(float((rho_b - st_min).upper()) + 1e-6)
    so_min = min(r for _, r in so_spec)
    st_spec = [(l, r) for l, r in st_spec if float((r + so_min).lower()) < float(rho_b.upper())]
    prod = []
    for ls, rs in so_spec:
        for lt, rt in st_spec:
            e = rs + rt
            if float(e.lower()) < float(rho_b.upper()):
                prod.append((e, (2 * ls + 1) * (2 * lt + 1)))
    ambiguous = [e for e, _ in prod if e.contains(rho_b) or (float(e.upper()) >= float(rho_b.lower()))]
    count = sum(mult for e, mult in prod if float(e.upper()) < float(rho_b.lower()))
    levels = sorted((e for e, mult in prod for _ in range(mult)), key=lambda x: float(x.mid()))
    width = max(float(e.rad()) for e, _ in prod)
    print(f"  soft radial levels {len(so_spec)} (l < {so_l}), stiff {len(st_spec)} (l < {st_l}); "
          f"product levels below rho_b: {count} (ambiguous at rho_b: {len(ambiguous)}); "
          f"base lambda_1 = {float(levels[0].mid()):.6f}, lambda_11 = {float(levels[10].mid()):.6f}; "
          f"max radius {width:.1e}; {time.time() - t0:.0f} s", flush=True)
    try:
        import dryrun_helike_sc_lehmann_homotopy as dr
        fs, _, _ = dr.radial_levels(float(w2s.mid()), LU, float(bu.mid()), float(vJ.mid()), float(rho_b.mid()))
        ft, _, _ = dr.radial_levels(float(w2t.mid()), LV, float(bv.mid()), 0.0, float(rho_b.mid()))
        fl = sorted(a + b for la, a, _ in fs for _ in range(2 * la + 1) for lb, b, _ in ft for _ in range(2 * lb + 1)
                    if a + b < float(rho_b.mid()))
        print(f"  floating Galerkin count at the same constants: {len(fl)}; "
              f"max |Galerkin - enclosure| over the first 30: "
              f"{max(abs(a - float(b.mid())) for a, b in zip(fl[:30], levels[:30])):.1e}", flush=True)
    except ImportError as exc:
        print(f"  floating comparison skipped ({exc})")


if __name__ == "__main__":
    main()
