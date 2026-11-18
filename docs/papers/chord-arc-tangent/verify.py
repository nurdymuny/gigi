"""Symbolic and numerical checks for every identity in paper.tex.
usage: python docs/papers/chord-arc-tangent/verify.py      (prints PASS/FAIL per item)
"""
import sympy as sp
import numpy as np

th, t, x = sp.symbols("theta t x", real=True)
I = sp.I
ok = []


def check(name, cond):
    ok.append(bool(cond))
    print(("PASS " if cond else "FAIL ") + name)


C = 2 * sp.sin(th / 2)          # chord
A = th                          # arc
T = 2 * sp.tan(th / 2)          # outside (two tangent segments)

# 1. chord as a complex number
U = sp.exp(I * th)
check("|U-1| = 2 sin(theta/2)", sp.simplify(sp.Abs(U - 1).rewrite(sp.cos) - C).equals(0) or
      sp.simplify(sp.expand_complex((U - 1) * sp.conjugate(U - 1)) - C**2) == 0)
check("U-1 = 2i sin(theta/2) e^{i theta/2}  (chord direction turns by theta/2)",
      sp.simplify(sp.expand_complex(U - 1 - 2 * I * sp.sin(th / 2) * sp.exp(I * th / 2))) == 0)

# 2. series of the three lengths
sC = sp.series(C, th, 0, 7).removeO()
sT = sp.series(T, th, 0, 7).removeO()
print("chord   =", sC)
print("outside =", sT)
check("arc - chord = theta^3/24 + O(theta^5)", sp.series(A - C, th, 0, 5).removeO() == th**3 / 24)
check("outside - arc = theta^3/12 + O(theta^5)", sp.series(T - A, th, 0, 5).removeO() == th**3 / 12)
sh = sp.series((2 * C + T) / 3 - A, th, 0, 7).removeO()
print("(2 chord + outside)/3 - arc =", sh)
check("Snell-Huygens: (2C+T)/3 = theta + theta^5/320 + O(theta^7)", sp.simplify(sh - th**5 / 320) == 0)

# 3. sandwich on (0, pi)
grid = np.linspace(1e-4, np.pi - 1e-4, 200001)
c, a, tt = 2 * np.sin(grid / 2), grid, 2 * np.tan(grid / 2)
check("chord < arc < outside on (0, pi) (grid)", bool(np.all(c < a) and np.all(a < tt)))
check("Snell-Huygens estimate >= arc on (0, pi) (grid)", bool(np.all((2 * c + tt) / 3 >= a - 1e-15)))

# 4. energies
E_C = sp.series(C**2, th, 0, 9).removeO()
E_T = sp.series(T**2, th, 0, 9).removeO()
print("chord^2   =", E_C)
print("outside^2 =", E_T)
check("chord^2 = 2(1 - cos theta) (Wilson)", sp.simplify(C**2 - 2 * (1 - sp.cos(th))) == 0)
check("chord^2 = theta^2 - theta^4/12 + ...", sp.series(C**2, th, 0, 6).removeO() == th**2 - th**4 / 12)
check("outside^2 = theta^2 + theta^4/6 + ...", sp.series(T**2, th, 0, 6).removeO() == th**2 + th**4 / 6)
mix = sp.series((2 * C**2 + T**2) / 3 - th**2, th, 0, 9).removeO()
print("(2 chord^2 + outside^2)/3 - theta^2 =", mix)
check("energy mix: (2C^2+T^2)/3 = theta^2 + 7 theta^6/720 + O(theta^8)  (theta^4 cancels)",
      mix.coeff(th, 4) == 0 and mix.coeff(th, 6) == sp.Rational(7, 720))

# 5. sameness and the double cover
S = sp.cos(th / 2)**2
d = sp.sin(th / 2)
check("S + d^2 = 1 with d = half chord", sp.simplify(S + d**2 - 1) == 0)
check("S = (1 + Re U)/2", sp.simplify(S - (1 + sp.cos(th)) / 2) == 0)
check("1 - S = chord^2 / 4", sp.simplify(1 - S - C**2 / 4) == 0)

# 6. Cayley: tangent leg builds the rotation
tau = sp.tan(th / 2)
g = np.linspace(-np.pi + 1e-3, np.pi - 1e-3, 100001)
tg = np.tan(g / 2)
check("U = (1 + i tan(theta/2)) / (1 - i tan(theta/2))  (dense grid, |err| < 1e-12)",
      float(np.abs((1 + 1j * tg) / (1 - 1j * tg) - np.exp(1j * g)).max()) < 1e-12)
check("tan(theta/2) = sin(theta) / (1 + cos(theta))  (|theta| < 3.1, pointwise rel err < 1e-12)",
      float((np.abs(tg - np.sin(g) / (1 + np.cos(g))) / np.maximum(np.abs(tg), 1e-300))[np.abs(g) < 3.1].max()) < 1e-12)
check("algebraic: ((1+i t)/(1-i t)) has modulus 1 and argument 2 arctan t",
      sp.simplify(sp.expand_complex(((1 + I * t) / (1 - I * t)) * sp.conjugate((1 + I * t) / (1 - I * t)))) == 1)

# 7. quaternions (SU(2)): U = cos a + n sin a, |n| = 1, pure
rng = np.random.default_rng(7)


def qmul(p, q):
    a1, b1 = p[0], p[1:]
    a2, b2 = q[0], q[1:]
    return np.r_[a1 * a2 - b1 @ b2, a1 * b2 + a2 * b1 + np.cross(b1, b2)]


def qinv(q):
    return np.r_[q[0], -q[1:]] / (q @ q)


worst = {"chord": 0.0, "cayley": 0.0, "log": 0.0}
for _ in range(2000):
    a = rng.uniform(0.01, np.pi - 0.01)
    n = rng.normal(size=3); n /= np.linalg.norm(n)
    Uq = np.r_[np.cos(a), np.sin(a) * n]
    worst["chord"] = max(worst["chord"], abs(np.linalg.norm(Uq - np.r_[1, 0, 0, 0]) - 2 * np.sin(a / 2)))
    v = Uq[1:] / (1 + Uq[0])                                 # Cayley vector, |v| = tan(a/2)
    V = np.r_[0, v]
    cay = qmul(np.r_[1, 0, 0, 0] + V, qinv(np.r_[1, 0, 0, 0] - V))
    worst["cayley"] = max(worst["cayley"], float(np.abs(cay - Uq).max()),
                          abs(np.linalg.norm(v) - np.tan(a / 2)))
    lg = np.arctan2(np.linalg.norm(Uq[1:]), Uq[0])           # geodesic angle = arc
    worst["log"] = max(worst["log"], abs(lg - a))
print("quaternion worst errors:", worst)
check("SU(2): |U - 1| = 2 sin(a/2) and arc = a (to 1e-14)", max(worst["chord"], worst["log"]) < 1e-14)
check("SU(2): Cayley (1+v)(1-v)^-1 = U with |v| = tan(a/2) (to 1e-9; conditioning ~1/(1+cos a) near the half-turn)",
      worst["cayley"] < 1e-9)

# 8. Crank-Nicolson is the Cayley map: |r(z)| <= 1 iff Re z >= 0
z = sp.symbols("z")
r = (1 - z / 2) / (1 + z / 2)
pts = rng.normal(size=(20000, 2)) * 5
vals = np.abs((1 - (pts[:, 0] + 1j * pts[:, 1]) / 2) / (1 + (pts[:, 0] + 1j * pts[:, 1]) / 2))
check("Crank-Nicolson |r(z)| <= 1 exactly on Re z >= 0 (sample)",
      bool(np.all((vals <= 1 + 1e-12) == (pts[:, 0] >= -1e-12))))
check("r(i s) = Cayley rotation with tan(theta/2) = -s/2: |r(i s)| = 1",
      sp.simplify(sp.Abs(r.subs(z, I * x)).rewrite(sp.sqrt) - 1) == 0 or
      abs(complex(r.subs(z, 0.7j)).__abs__() - 1) < 1e-15)

# 8b. one step of a pure oscillation y' = i y with step s: where each rule puts the point
sv = np.linspace(1e-3, 3.0, 3001)
ee, ie, cn = 1 + 1j * sv, 1 / (1 - 1j * sv), (1 + 1j * sv / 2) / (1 - 1j * sv / 2)
check("explicit Euler leaves the circle outward along the tangent: |1+is| = sqrt(1+s^2) > 1",
      bool(np.allclose(np.abs(ee), np.sqrt(1 + sv**2)) and np.all(np.abs(ee) > 1)))
check("implicit Euler falls inside: |1/(1-is)| = 1/sqrt(1+s^2) < 1",
      bool(np.allclose(np.abs(ie), 1 / np.sqrt(1 + sv**2)) and np.all(np.abs(ie) < 1)))
check("Crank-Nicolson stays on the circle, turning by phi = 2 arctan(s/2) (step s plays the outside path)",
      bool(np.allclose(np.abs(cn), 1) and np.allclose(np.angle(cn), 2 * np.arctan(sv / 2))))
lag = sp.series(x - 2 * sp.atan(x / 2), x, 0, 5).removeO()
check("Crank-Nicolson phase lag s - 2 arctan(s/2) = s^3/12 + O(s^5) (= outside minus arc)", sp.simplify(lag - x**3 / 12) == 0)

# 9. half-turn: tangents parallel, outside path infinite
check("outside path -> infinity as theta -> pi", sp.limit(T, th, sp.pi, "-") == sp.oo)

print(f"\n{sum(ok)}/{len(ok)} PASS")
