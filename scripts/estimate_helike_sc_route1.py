"""Floating-point FEASIBILITY ESTIMATE (not a bound, not a proof) for Route 1 of
docs/research/helike-sc-proof-design-2026-11-25.md: IMS localization with a uniform cubic reduction inside a tube of
radius L around the background orbit. Prints, per control and L, the reduced third charge-0 harmonic level against
rho_C, and the sampled minimum of V - V_bg at transverse distance L/2. Also a many-start global minimisation.
usage: python scripts/estimate_helike_sc_route1.py      (a few minutes; CPU only)"""
import math
import numpy as np
from scipy.optimize import minimize
import helike_berger_model as bm


def Vq(q, g, t, a, b):
    om = np.array([math.sqrt(t), 1 / math.sqrt(t), 1 / math.sqrt(t)]); src = np.array([a, b, b])
    cof = np.array([np.cross(q[1], q[2]), np.cross(q[2], q[0]), np.cross(q[0], q[1])])
    return 0.5 * np.sum((om[:, None] * q - g * cof) ** 2) - 0.5 * np.sum(src[:, None] * q ** 2)


def main():
    rng = np.random.default_rng(0)
    for g2, E0rel in ((0.004, 5.40626), (0.002, 5.40917)):   # saved ground centre minus V_bg (controls, Mq = 8)
        s, t = 1.15, 3.0; g = math.sqrt(g2); a, b = bm.mc_sources(s, t)
        qb = np.diag([s / (g * math.sqrt(t)), s / g, s / g]); x0 = qb.ravel()
        f = lambda x: Vq(x.reshape(3, 3), g, t, a, b)
        V0, h, K = f(x0), 1e-3, np.zeros((9, 9))
        for i in range(9):
            for j in range(9):
                e = np.zeros(9); e[i] = h; d = np.zeros(9); d[j] = h
                K[i, j] = (f(x0 + e + d) - f(x0 + e - d) - f(x0 - e + d) + f(x0 - e - d)) / (4 * h * h)
        w2, U = np.linalg.eigh(K); N = U[:, 3:]; om = np.sqrt(w2[3:])
        P3 = lambda z: (f(x0 + z) - f(x0 - z)) / 2
        C3 = max(-minimize(lambda y: -abs(P3(N @ (y / np.linalg.norm(y)))), rng.normal(size=6), method="Nelder-Mead",
                           options={"maxiter": 3000, "xatol": 1e-10, "fatol": 1e-12}).fun for _ in range(300))
        R = np.linalg.norm(qb); wH, wmin = om[0], om.min(); rhoC = E0rel + 2 * wH - wmin / 4
        print(f"g2={g2}: C3={C3:.4f} |q_bg|={R:.2f} rho_C-V_bg={rhoC:.4f}")
        for L in (3, 4, 5, 6, 7, 8, 10):
            red = w2[3:] - 2 * C3 * L
            if red.min() <= 0:
                print(f"  L={L}: inside bound collapses"); continue
            omr = np.sqrt(red); loc = 0.5 * (math.pi / (L / 2)) ** 2
            third = 0.5 * omr.sum() + 2 * omr[0] - loc - 1.0 / R ** 2
            dmin = min(f(x0 + N @ (y / np.linalg.norm(y) * L / 2)) - V0 for y in rng.normal(size=(150, 6)))
            print(f"  L={L}: inside margin {third - rhoC:+.4f}; outside sampled min {dmin:.3f} vs need {rhoC + loc:.3f}")
        lows = [minimize(f, rng.normal(size=9) * R * rng.choice([0.05, 0.5, 1, 2]), method="BFGS").fun - V0 for _ in range(200)]
        print(f"  global minimisation (200 starts): lowest V - V_bg = {min(lows):.3f}")


if __name__ == "__main__":
    main()
