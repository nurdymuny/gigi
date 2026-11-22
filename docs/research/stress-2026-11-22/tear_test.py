"""Pre-registration S7: the tear (docs/research/tear-test-preregistration-2026-11-22.md, commit a5be2123, pushed before
this script was written). Quotients S^3/Gamma for Gamma = Z2, Z3, Z5, Q8, 2D3, 2T, 2O, 2I acting on the left.
Writes tear_receipt.json and prints the verdicts.
usage (repository root): python docs/research/stress-2026-11-22/tear_test.py
"""
import itertools
import json
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "docs" / "research" / "basin-2026-11-22"))
_a, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_exact_edge as EE  # noqa: E402
import poincare_fall as PF          # noqa: E402  (icosians, characters, holonomy at theta)
sys.argv = _a

J = 40
TS = (3.0, 5.0, 10.0)
L = {3.0: {1, 2, 3, 4, 5}, 5.0: {2, 3}, 10.0: {2}}   # families she loses at A = 0 (S1b, S6)
TOL = 1e-8
BLK = EE.Blocks()


def qmul(a, b):
    a0, a1, a2, a3 = a; b0, b1, b2, b3 = b
    return np.array([a0*b0 - a1*b1 - a2*b2 - a3*b3, a0*b1 + a1*b0 + a2*b3 - a3*b2,
                     a0*b2 - a1*b3 + a2*b0 + a3*b1, a0*b3 + a1*b2 - a2*b1 + a3*b0])


def uniq(qs):
    out = []
    for q in qs:
        q = np.asarray(q, float)
        if not any(np.allclose(q, p, atol=1e-9) for p in out):
            out.append(q)
    return out


def cyclic(n):
    return uniq([(np.cos(2*np.pi*k/n), np.sin(2*np.pi*k/n), 0, 0) for k in range(n)])


def bdihedral(m):
    a = [(np.cos(np.pi*k/m), np.sin(np.pi*k/m), 0, 0) for k in range(2*m)]
    return uniq(a + [qmul(np.array(x), np.array([0, 0, 1, 0])) for x in a])


def q8():
    return uniq([s * e for e in np.eye(4) for s in (1, -1)])


def btetra():
    return uniq(q8() + [0.5*np.array(s) for s in itertools.product((1, -1), repeat=4)])


def bocta():
    extra = []
    for i, k in itertools.combinations(range(4), 2):
        for si, sk in itertools.product((1, -1), repeat=2):
            q = np.zeros(4); q[i] = si / np.sqrt(2); q[k] = sk / np.sqrt(2); extra.append(q)
    return uniq(btetra() + extra)


GROUPS = {"Z2": cyclic(2), "Z3": cyclic(3), "Z5": cyclic(5), "Q8": q8(), "2D3": bdihedral(3),
          "2T": btetra(), "2O": bocta(), "2I": uniq(PF.icosians())}
ORDERS = {"Z2": 2, "Z3": 3, "Z5": 5, "Q8": 8, "2D3": 12, "2T": 24, "2O": 48, "2I": 120}


def is_group(G):
    return all(any(np.allclose(qmul(a, b), c, atol=1e-9) for c in G) for a in G for b in G)


def main():
    R = {"groups": {}}
    for name, G in GROUPS.items():
        assert len(G) == ORDERS[name], (name, len(G))
        assert is_group(G), name
    # spectra are per family and do not depend on Gamma: compute once per t
    spec = {}
    for t in TS:
        xb, yb = (2 - t) / np.sqrt(t), np.sqrt(t)
        for j in range(J + 1):
            spec[(t, j)] = dict(
                berger=float(BLK.sector_spectrum(xb, yb, t, j).min()), her_berger=float(EE.lam_tw(xb, yb, t, j)),
                theta=float(BLK.sector_spectrum(2 / np.sqrt(t), 2.0, t, j).min()),
                zero=float(BLK.sector_spectrum(0.0, 0.0, t, j).min()), her_zero=float(EE.lam_tw(0.0, 0.0, t, j)))
    ok = {k: True for k in ("T1", "T2", "T3", "T4", "T5")}
    for name, G in GROUPS.items():
        surv = [j for j in range(J + 1) if round(PF.dim_invariants(G, j)) > 0]
        hol = 0.0; abelian = all(np.allclose(qmul(a, b), qmul(b, a), atol=1e-9) for a in G for b in G)
        for q in G:
            U = PF.quat_to_su2(q)
            if np.allclose(U, np.eye(2)) or np.allclose(U, -np.eye(2)):
                continue
            hol = max(hol, float(np.abs(PF.holonomy_theta(U, 3.0) - PF.adjoint(U.conj().T)).max()))
        g = dict(order=len(G), abelian=abelian, surviving=surv, family2_survives=2 in surv, holonomy_err=hol, t={})
        ok["T2"] &= hol < 1e-10
        for t in TS:
            s = {j: spec[(t, j)] for j in surv}
            t1_floor = all(v["berger"] >= v["her_berger"] - TOL * max(1, abs(v["her_berger"])) for v in s.values())
            cheapest = min(surv, key=lambda j: s[j]["her_berger"])
            t1 = t1_floor and s[cheapest]["her_berger"] < 0
            zero_modes = sum(1 for v in s.values() if v["theta"] < 1e-9 or v["zero"] < 1e-9)
            t3_rows, t3 = [], True
            for j in surv:
                if j < 2:
                    continue
                shadow = j * j / t
                lost = (j - 2) in L[t]
                eq = abs(s[j]["theta"] - shadow) <= TOL * max(1, shadow)
                good = (not lost and eq) or (lost and s[j]["theta"] < shadow - TOL)
                t3 &= good
                t3_rows.append((j, s[j]["theta"], shadow, lost, good))
            low_theta_j = min(surv, key=lambda j: s[j]["theta"]); low_theta = s[low_theta_j]["theta"]
            if 2 in surv:
                t4 = abs(low_theta - 4 / t) <= TOL
            else:
                t4 = low_theta > 4 / t + TOL
            low_zero_j = min(surv, key=lambda j: s[j]["zero"]); low_zero = s[low_zero_j]["zero"]
            t5 = abs(low_zero - 4 / t) <= TOL and abs(s[0]["her_zero"] - 4 / t) <= TOL and 0 in surv
            ok["T1"] &= t1; ok["T2"] &= zero_modes == 0; ok["T3"] &= t3; ok["T4"] &= t4; ok["T5"] &= t5
            g["t"][str(t)] = dict(T1=t1, her_cheapest_family=cheapest, her_cheapest_price=s[cheapest]["her_berger"],
                                  zero_modes=zero_modes, T3=t3, T3_rows=t3_rows[:12], T4=t4,
                                  theta_lowest=(low_theta_j, low_theta), four_over_t=4 / t, T5=t5,
                                  zero_lowest=(low_zero_j, low_zero))
            print(f"{name:4s} t={t:4}: surv={surv[:8]}{'...' if len(surv) > 8 else ''}  T1={t1} T2zm={zero_modes} "
                  f"T3={t3} T4={t4} (theta lowest {low_theta:.4f} in family {low_theta_j}; 4/t={4/t:.4f}) T5={t5}")
        g["holonomy_ok"] = hol < 1e-10
        R["groups"][name] = g
        print(f"     {name}: order {len(G)}, abelian={abelian}, family 2 survives={2 in surv}, holonomy err {hol:.1e}")
    R["verdicts"] = ok
    print("VERDICTS", ok)
    (HERE / "tear_receipt.json").write_text(json.dumps(R, indent=1, default=float))


if __name__ == "__main__":
    main()
