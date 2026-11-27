import sympy as sp, time
t0 = time.time()
p00, p11, p22, p01, p02, p12 = sp.symbols("p00 p11 p22 p01 p02 p12", real=True)
P = sp.Matrix([[p00, p01, p02], [p01, p11, p12], [p02, p12, p22]])
pv = [p00, p11, p22, p01, p02, p12]
KEYS = [(0, 0), (1, 1), (2, 2), (0, 1), (0, 2), (1, 2)]
G = P * P
x = sp.Matrix([G[i, j] for i, j in KEYS])
D = x.jacobian(pv)
import helike_condensate_pt as cp
A = sp.Matrix(6, 6, lambda a, b: sum(int(cp.C_A[a, b, m]) * x[m] for m in range(6)))
adj = D.adjugate()
e1 = P.trace(); e2 = sp.Rational(1, 2) * (P.trace() ** 2 - (P * P).trace()); e3 = P.det()
w = sp.expand(e1 * e2 - e3)
detD = 8 * e3 * w
for (a, b) in [(0, 0), (1, 3), (3, 4), (2, 5), (0, 1)]:
    Mab = sp.expand((adj[a, :] * A * adj[b, :].T)[0, 0])
    gab = sp.cancel(Mab / sp.expand(detD ** 2))       # the cometric g^P_ab
    rg = sp.cancel(w * gab)                          # orbit volume (/8) times cometric
    num, den = sp.fraction(rg)
    print(f"g[{a},{b}] denominator degree {sp.Poly(sp.fraction(gab)[1], *pv).total_degree()}; "
          f"rho*g[{a},{b}] denominator = {sp.factor(den)}; numerator degree {sp.Poly(num, *pv).total_degree()}  ({time.time()-t0:.0f}s)", flush=True)
