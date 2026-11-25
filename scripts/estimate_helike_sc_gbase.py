"""Floating-point FEASIBILITY ESTIMATE for gate G-base of
docs/research/helike-sc-lehmann-goerisch-registration-2026-11-25.md (candidate B3: Neumann bracketing in a tube
around the background colour orbit). Not a bound, not a proof.

Per control coupling it estimates:
1. the transverse frequencies, the harmonic singlet levels (to fix N), and C3 = max |P3(z)| / |z|^3;
2. that the quartic remainder P4 is >= 0 on samples (it is (g^2/2)|cof Nz|^2 exactly);
3. OUTSIDE: m(r) = min over the distance-r shell of V - V_bg, and the smallest tube radius L at which
   min_{r >= L} m(r) clears (a) the registered rho_C + 0.05 and (b) the Lehmann base level rho_b;
4. the orbit-curvature terms of the singlet reduction on the tube: the range of the Jacobian potential
   V_J = Lap(sqrt J) / (2 sqrt J) and the boundary Robin coefficient beta = max |d_n log sqrt J|;
5. INSIDE: the explicit radial base W(r) = 1/2 w_min r^2 - C3 r^3 + min V_J on the Neumann/Robin 6-ball of radius L
   (a rigorous minorant: 1/2 z.Kz >= 1/2 w_min |z|^2, P3 >= -C3|z|^3, P4 >= 0, singlet kinetic >= flat), solved by
   radial finite elements per angular momentum; reports its lambda_11 and the number of base levels below rho_b,
   which is the size of the homotopy Lehmann would have to climb. For reference, the same count for the separable
   uniformly reduced oscillator (Route 1's minorant, full space; not a valid Neumann base).
usage: python scripts/estimate_helike_sc_gbase.py      (CPU, a few minutes)"""
import itertools
import math

import numpy as np
from scipy.linalg import eigh
from scipy.optimize import minimize
from scipy.spatial.transform import Rotation

import helike_berger_model as bm
from estimate_helike_sc_route1 import Vq

S, T = 1.15, 3.0
CONTROLS = ((0.004, 5.40626), (0.002, 5.40917))     # g2, saved ground centre minus V_bg (controls, Mq = 8)
RHO_B_REL = 2.32                                    # rho_b - E0: between harmonic lambda_10 (2.213) and lambda_11 (2.430)
GEN = [np.array(m, float) for m in (((0, 0, 0), (0, 0, -1), (0, 1, 0)), ((0, 0, 1), (0, 0, 0), (-1, 0, 0)),
                                    ((0, -1, 0), (1, 0, 0), (0, 0, 0)))]


def setup(g2):
    g = math.sqrt(g2); a, b = bm.mc_sources(S, T)
    qb = np.diag([S / (g * math.sqrt(T)), S / g, S / g]); x0 = qb.ravel()
    f = lambda x: Vq(x.reshape(3, 3), g, T, a, b)
    V0, h, K = f(x0), 1e-3, np.zeros((9, 9))
    for i in range(9):
        for j in range(9):
            e = np.zeros(9); e[i] = h; d = np.zeros(9); d[j] = h
            K[i, j] = (f(x0 + e + d) - f(x0 + e - d) - f(x0 - e + d) + f(x0 - e - d)) / (4 * h * h)
    w2, U = np.linalg.eigh(K)
    return dict(g=g, f=f, x0=x0, qb=qb, V0=V0, w2=w2[3:], N=U[:, 3:], Rq=np.linalg.norm(qb))


def levels(om, n=16):
    return sorted(sum(k * w for k, w in zip(ks, om)) for ks in itertools.product(range(5), repeat=len(om)))[:n]


def cubic_constant(m, rng, starts=60):
    f, x0, N = m["f"], m["x0"], m["N"]
    P3 = lambda z: (f(x0 + z) - f(x0 - z)) / 2
    best = 0.0
    for _ in range(starts):
        r = minimize(lambda y: -abs(P3(N @ (y / np.linalg.norm(y)))), rng.normal(size=6), method="Nelder-Mead",
                     options={"maxiter": 3000, "xatol": 1e-10, "fatol": 1e-12})
        best = max(best, -r.fun)
    return best


def quartic_min(m, rng, L):
    f, x0, N, V0, w2 = m["f"], m["x0"], m["N"], m["V0"], m["w2"]
    lo = math.inf
    for y in rng.normal(size=(500, 6)):
        y = y / np.linalg.norm(y) * rng.uniform(0, L)
        z = N @ y
        lo = min(lo, (f(x0 + z) + f(x0 - z)) / 2 - V0 - 0.5 * np.sum(w2 * y * y))
    return lo


def shell_min(m, r, rng, starts=12):
    f, x0, N, V0 = m["f"], m["x0"], m["N"], m["V0"]
    best, arg = math.inf, None
    for _ in range(starts):
        res = minimize(lambda y: f(x0 + N @ (y / np.linalg.norm(y) * r)), rng.normal(size=6), method="Nelder-Mead",
                       options={"maxiter": 2500, "xatol": 1e-7, "fatol": 1e-9})
        if res.fun < best:
            best, arg = res.fun, res.x / np.linalg.norm(res.x) * r
    return best - V0, arg


def orbit_distance(m, q):
    qb = m["qb"]
    res = min((minimize(lambda v: np.sum((q - qb @ Rotation.from_rotvec(v).as_matrix()) ** 2), v0, method="BFGS")
               for v0 in (np.zeros(3), *np.random.default_rng(1).normal(size=(5, 3)))), key=lambda r: r.fun)
    return math.sqrt(res.fun)


def sqrtJ(m, y):
    q = (m["x0"] + m["N"] @ y).reshape(3, 3)
    cols = np.column_stack([m["N"]] + [(q @ G).ravel() for G in GEN])
    return math.sqrt(abs(np.linalg.det(cols)))


def curvature(m, L, rng, n=300):
    h, vj = 1e-2, []
    for y in rng.normal(size=(n, 6)):
        y = y / np.linalg.norm(y) * rng.uniform(0, L)
        s0 = sqrtJ(m, y)
        lap = sum(sqrtJ(m, y + h * e) + sqrtJ(m, y - h * e) - 2 * s0 for e in np.eye(6)) / h ** 2
        vj.append(lap / (2 * s0))
    beta = 0.0
    for y in rng.normal(size=(n, 6)):
        u = y / np.linalg.norm(y)
        beta = max(beta, abs(math.log(sqrtJ(m, (L + h) * u)) - math.log(sqrtJ(m, (L - h) * u))) / (2 * h))
    return min(vj), max(vj), beta


def radial_base(w_min, C3, shift, L, beta, target, n_el=600):
    """Eigenvalues below target of -1/2 Lap + 1/2 w_min r^2 - C3 r^3 + shift on the 6-ball of radius L with the
    natural (Neumann) condition plus a Robin term -beta/2 on the sphere; linear FEM in R(r) with weight r^5."""
    x = np.linspace(0, L, n_el + 1); gp, gw = np.polynomial.legendre.leggauss(4)
    out = []
    for l in range(0, 60):
        n = n_el + 1
        A = np.zeros((n, n)); B = np.zeros((n, n))
        for k in range(n_el):
            a, c = x[k], x[k + 1]; hk = c - a
            for p, w in zip(gp, gw):
                r = a + (p + 1) * hk / 2; wt = w * hk / 2 * r ** 5
                phi = np.array([(c - r) / hk, (r - a) / hk]); dphi = np.array([-1 / hk, 1 / hk])
                W = 0.5 * w_min * r * r - C3 * r ** 3 + shift + (l * (l + 4) / (2 * r * r) if r > 0 else 0)
                idx = [k, k + 1]
                A[np.ix_(idx, idx)] += wt * (0.5 * np.outer(dphi, dphi) + W * np.outer(phi, phi))
                B[np.ix_(idx, idx)] += wt * np.outer(phi, phi)
        A[-1, -1] -= 0.5 * beta * L ** 5
        if l > 0:                                   # R(0) = 0 for l > 0
            A, B = A[1:, 1:], B[1:, 1:]
        ev = eigh(A, B, eigvals_only=True, subset_by_index=[0, min(40, A.shape[0] - 1)])
        below = ev[ev < target]
        if len(below) == 0:
            break
        deg = (l + 1) * (l + 2) ** 2 * (l + 3) // 12
        out += [e for e in below for _ in range(deg)]
    return sorted(out)


def separable_count(w2, C3, L, target):
    red = w2 - 2 * C3 * L
    if red.min() <= 0:
        return None
    om = np.sqrt(red); z = 0.5 * om.sum()
    nmax = [int((target - z) / o) + 1 for o in om]
    return sum(1 for ks in itertools.product(*[range(k + 1) for k in nmax]) if z + np.dot(ks, om) < target)


def main():
    rng = np.random.default_rng(0)
    for g2, E0rel in CONTROLS:
        m = setup(g2); w2 = m["w2"]; om = np.sqrt(w2)
        lev = levels(om); wH = wmin = om.min()
        rhoC = E0rel + 2 * wH - wmin / 4; rhob = E0rel + RHO_B_REL
        print(f"\n=== g2={g2}: |q_bg|={m['Rq']:.2f}  omega={np.round(om, 4)}")
        print(f"harmonic singlet levels - E0: {np.round(lev[:13], 4)}")
        print(f"rho_C - V_bg = {rhoC:.4f};  rho_b - V_bg = {rhob:.4f} (E0 + {RHO_B_REL})")
        C3 = cubic_constant(m, rng); print(f"C3 = {C3:.4f}")
        print(f"P4 sampled min on |z|<=8: {quartic_min(m, rng, 8.0):.3e}  (>= 0 expected; exact (g^2/2)|cof Nz|^2)")
        print("outside: shell minima m(r) = min_{|z|=r} V - V_bg, nearest-orbit distance of the minimiser")
        radii = [3, 4, 5, 6, 7, 8, 10, 13, 17]
        mins = {}
        for r in radii:
            mr, y = shell_min(m, r, rng); mins[r] = mr
            d = orbit_distance(m, (m["x0"] + m["N"] @ y).reshape(3, 3))
            print(f"  r={r:5.1f}: m(r)={mr:9.4f}  dist={d:7.4f}")
        for name, tgt in (("rho_C+0.05", rhoC + 0.05), ("rho_b", rhob)):
            Ls = [r for r in radii if all(mins[s] >= tgt for s in radii if s >= r)]
            print(f"  smallest sampled L with min_(r>=L) m(r) >= {name} ({tgt:.3f}): {Ls[0] if Ls else 'none'}")
        for L in (4.0, 5.0, 6.0):
            vlo, vhi, beta = curvature(m, L, rng)
            ev = radial_base(w2.min(), C3, vlo, L, beta, rhob)
            l11 = ev[10] if len(ev) > 10 else math.inf
            sep = separable_count(w2, C3, L, rhob)
            print(f"  L={L}: V_J in [{vlo:.2e},{vhi:.2e}], beta={beta:.3f};  radial base: lambda_1={ev[0] if ev else math.inf:.3f}"
                  f" lambda_11={l11:.3f} (need > {rhob:.3f}), levels below rho_b = {len(ev)};  separable uniform: {sep}")


# ---- B3': product tube (soft-block ball x stiff-block ball) with a block-isotropic separable Neumann base ----
def block_shell_min(m, block, r, rng, starts=12):
    """min of V - V_bg over {|z_block| = r}, the other block free (unbounded)."""
    f, x0, N, V0 = m["f"], m["x0"], m["N"], m["V0"]
    sb, ob = (slice(0, 3), slice(3, 6)) if block == "soft" else (slice(3, 6), slice(0, 3))
    def V(y):
        z = np.zeros(6); u = y[:3]; z[sb] = u / np.linalg.norm(u) * r; z[ob] = y[3:]
        return f(x0 + N @ z)
    best = min(minimize(V, np.concatenate([rng.normal(size=3), 0.3 * rng.normal(size=3)]), method="Nelder-Mead",
                        options={"maxiter": 3000, "xatol": 1e-7, "fatol": 1e-9}).fun for _ in range(starts))
    return best - V0


def radial3(w2r, L, beta, target, shift=0.0, n_el=500):
    """Eigenvalues below target of -1/2 Lap + 1/2 w2r r^2 + shift on the 3-ball of radius L, natural (Neumann) BC
    with a Robin term -beta/2 on the sphere; with multiplicity (2l+1)."""
    x = np.linspace(0, L, n_el + 1); gp, gw = np.polynomial.legendre.leggauss(4); out = []
    for l in range(0, 80):
        n = n_el + 1; A = np.zeros((n, n)); B = np.zeros((n, n))
        for k in range(n_el):
            a, c = x[k], x[k + 1]; hk = c - a
            for p, w in zip(gp, gw):
                r = a + (p + 1) * hk / 2; wt = w * hk / 2 * r * r
                phi = np.array([(c - r) / hk, (r - a) / hk]); dphi = np.array([-1 / hk, 1 / hk])
                W = 0.5 * w2r * r * r + shift + l * (l + 1) / (2 * r * r)
                idx = [k, k + 1]
                A[np.ix_(idx, idx)] += wt * (0.5 * np.outer(dphi, dphi) + W * np.outer(phi, phi))
                B[np.ix_(idx, idx)] += wt * np.outer(phi, phi)
        A[-1, -1] -= 0.5 * beta * L * L
        if l > 0:
            A, B = A[1:, 1:], B[1:, 1:]
        ev = eigh(A, B, eigvals_only=True, subset_by_index=[0, min(60, A.shape[0] - 1)])
        below = ev[ev < target]
        if len(below) == 0:
            break
        out += [e for e in below for _ in range(2 * l + 1)]
    return sorted(out)


def product_base(m, C3, Lu, Lv, beta, vJ, target):
    w2 = m["w2"]; L = math.hypot(Lu, Lv)
    ws, wt = w2[:3].min() - 2 * C3 * L, w2[3:].min() - 2 * C3 * L
    if min(ws, wt) <= 0:
        return None
    es = radial3(ws, Lu, beta, target, vJ); et = radial3(wt, Lv, beta, target)
    tot = sorted(a + b for a in es for b in et if a + b < target)
    return tot, ws, wt


def main_product():
    rng = np.random.default_rng(2)
    for g2, E0rel in CONTROLS:
        m = setup(g2); rhob = E0rel + RHO_B_REL
        C3 = cubic_constant(m, rng)
        print(f"\n=== B3' g2={g2}: rho_b - V_bg = {rhob:.4f}, C3 = {C3:.4f}, soft w2 = {np.round(m['w2'][:3], 4)}")
        for blk, radii in (("soft", (3.5, 4, 4.5, 5, 6, 8, 12)), ("stiff", (1.5, 1.75, 2, 2.5, 3, 4, 6))):
            print(f"  {blk}: " + "  ".join(f"r={r}: {block_shell_min(m, blk, r, rng):.3f}" for r in radii))
        for Lu, Lv in ((4.25, 2.0), (4.5, 2.0), (5.0, 2.0), (4.5, 2.5)):
            vlo, _, beta = curvature(m, math.hypot(Lu, Lv), rng, n=150)
            r = product_base(m, C3, Lu, Lv, beta, vlo, rhob)
            if r is None:
                print(f"  Lu={Lu} Lv={Lv}: reduced frequency collapses"); continue
            tot, ws, wt = r
            l11 = tot[10] if len(tot) > 10 else math.inf
            print(f"  Lu={Lu} Lv={Lv}: reduced w2 soft {ws:.3f} stiff {wt:.3f}; base lambda_1={tot[0]:.3f} "
                  f"lambda_11={l11:.3f} (need > {rhob:.3f}); levels below rho_b = {len(tot)}")


if __name__ == "__main__":
    import sys
    main_product() if "--product" in sys.argv else main()
