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

# 10. twins on the other side: Snellius' lower estimate
L = 6 * sp.sin(th / 2) / (2 + sp.cos(th / 2))            # 2 * 3 sin x/(2 + cos x), x = theta/2
low = sp.series(th - L, th, 0, 7).removeO()
check("Snellius lower twin: theta - L = theta^5/2880 + O(theta^7)", sp.simplify(low - th**5 / 2880) == 0)
gL = grid[grid > 0.05]   # below this the true gap theta^5/2880 is under float resolution
check("L < theta on [0.05, pi) (grid; the series covers small theta)", bool(np.all(6 * np.sin(gL / 2) / (2 + np.cos(gL / 2)) < gL)))
blend = sp.series((9 * L + (2 * C + T) / 3) / 10 - th, th, 0, 7).removeO()
check("blending the twins 9:1 cancels theta^5", sp.simplify(blend) == 0)

# 11. the far-side twin: same chord, tangent leg flips sign
th0 = 1.0
check("chord(theta) = chord(2 pi - theta)", abs(2 * np.sin(th0 / 2) - 2 * np.sin((2 * np.pi - th0) / 2)) < 1e-15)
check("tangent leg(2 pi - theta) = -tangent leg(theta)", abs(np.tan((2 * np.pi - th0) / 2) + np.tan(th0 / 2)) < 1e-14)

# 12. pole and polar: chord line at distance cos(theta/2), corner at sec(theta/2)
check("chord-line distance x corner distance = 1", sp.simplify(sp.cos(th / 2) * (1 / sp.cos(th / 2)) - 1) == 0)

# 13. continuation through the rim: theta = i phi
ph = sp.symbols("phi", real=True)
check("chord(i phi) = 2 i sinh(phi/2)", sp.simplify((2 * sp.sin(I * ph / 2)).rewrite(sp.exp) - (2 * I * sp.sinh(ph / 2)).rewrite(sp.exp)) == 0)
check("tangent leg(i phi) = i tanh(phi/2)", sp.simplify((sp.tan(I * ph / 2)).rewrite(sp.exp) - (I * sp.tanh(ph / 2)).rewrite(sp.exp)) == 0)
check("S + d^2 = 1 survives: cosh^2(phi/2) + (i sinh(phi/2))^2 = 1", sp.simplify(sp.cosh(ph / 2)**2 + (I * sp.sinh(ph / 2))**2 - 1) == 0)

# 14. Bertrand: which quantity is uniform decides the answer
u = sp.symbols("u", positive=True)
check("uniform arc theta on [0, pi]: P(theta > 2 pi/3) = 1/3", sp.Rational(1, 3) == (sp.pi - 2 * sp.pi / 3) / sp.pi)
check("chord > sqrt3  <=>  theta > 2pi/3  <=>  cos(theta/2) < 1/2  <=>  S < 1/4",
      abs(2 * np.sin(np.pi / 3) - np.sqrt(3)) < 1e-15 and abs(np.cos(np.pi / 3) - 0.5) < 1e-15)
rng2 = np.random.default_rng(1)
Nb = 2_000_000
c1 = 2 * np.abs(np.sin(rng2.uniform(0, 2 * np.pi, Nb) / 2))
c2 = 2 * np.sqrt(1 - rng2.uniform(0, 1, Nb) ** 2)
c3 = 2 * np.sqrt(1 - rng2.uniform(0, 1, Nb))
pr = [(c > np.sqrt(3)).mean() for c in (c1, c2, c3)]
print("Bertrand (endpoints, radius, midpoint):", [round(float(p), 4) for p in pr])
check("Bertrand: 1/3, 1/2, 1/4 within 0.002 (2 million chords each)",
      abs(pr[0] - 1 / 3) < 2e-3 and abs(pr[1] - 1 / 2) < 2e-3 and abs(pr[2] - 1 / 4) < 2e-3)
check("random midpoint makes S uniform: S = 1 - chord^2/4 = 1 - (1 - U) = U", True)

# 15. the ticket game
deg = np.pi / 180
check("one ticket of arc at exactly one radian (57.2958 deg)", abs(1 / deg - 57.29577951) < 1e-7)
check("one ticket of chord at exactly 60 deg: 2 sin(30 deg) = 1", abs(2 * np.sin(30 * deg) - 1) < 1e-15)
check("one ticket of energy (1 - cos theta) at exactly 90 deg", abs(1 - np.cos(90 * deg) - 1) < 1e-15)
check("energy of the crossing = energy of two radius walks (1/2 + 1/2) iff theta = 90 deg  (Pythagoras)",
      sp.solve(sp.Eq(1 - sp.cos(th), sp.Rational(1, 2) + sp.Rational(1, 2)), th)[0] == sp.pi / 2)
check("crossing energy < two-trip energy below 90 deg, > above (grid)",
      bool(np.all(1 - np.cos(grid[grid < np.pi / 2]) < 1) and np.all(1 - np.cos(grid[grid > np.pi / 2 + 1e-9]) > 1)))
check("hexagon: arc premium at 60 deg = pi/3 - 1 = circumference/hexagon perimeter - 1",
      abs((np.pi / 3 - 1) - (2 * np.pi / 6 - 1)) < 1e-15 and abs(np.pi / 3 - 1 - 0.04719755) < 1e-8)
# the only two turning points one ticket from both players: the centre and A + B
worst_corner = 0.0
for tdeg in np.linspace(1, 179, 357):
    tt = tdeg * deg
    Ap, Bp = np.array([1.0, 0.0]), np.array([np.cos(tt), np.sin(tt)])
    Mp = (Ap + Bp) / 2
    up = np.array([-(Bp - Ap)[1], (Bp - Ap)[0]]); up /= np.linalg.norm(up)
    hp = np.sqrt(1 - np.linalg.norm(Bp - Ap) ** 2 / 4)
    c1, c2 = Mp + hp * up, Mp - hp * up
    got = sorted([tuple(np.round(c1, 12)), tuple(np.round(c2, 12))])
    want = sorted([tuple(np.round(np.zeros(2), 12)), tuple(np.round(Ap + Bp, 12))])
    worst_corner = max(worst_corner, float(np.abs(np.array(got) - np.array(want)).max()))
check("two whole tickets: the only turning points are the centre O and its twin A + B (1..179 deg)", worst_corner < 1e-11)
check("|A + B| = 2 cos(theta/2): twin corner on the circle exactly at 120 deg", abs(2 * np.cos(60 * deg) - 1) < 1e-15)
check("at 120 deg the crossing is Bertrand's sqrt 3", abs(2 * np.sin(60 * deg) - np.sqrt(3)) < 1e-15)
check("tangent corner sec(theta/2) = twin corner 2 cos(theta/2) iff theta = 90 deg",
      [r for r in sp.solve(sp.Eq(1 / sp.cos(th / 2), 2 * sp.cos(th / 2)), th) if 0 < r < sp.pi] == [sp.pi / 2])
check("at 90 deg the outside path is exactly two whole tickets: 2 tan(45 deg) = 2", abs(2 * np.tan(45 * deg) - 2) < 1e-15)
check("at 180 deg the twins merge: A + B = O", float(np.linalg.norm(np.array([1.0, 0]) + np.array([-1.0, 0]))) == 0.0)

# 16. the forced squares: the full one-ticket grid, the Thales twin, the energy rule, Haar
check("detour one ticket of length at 2 arctan(1/2) = 53.130 deg, the 3-4-5 angle (cos 3/5, sin 4/5)",
      abs(np.degrees(2 * np.arctan(0.5)) - 53.13010235) < 1e-7
      and abs(np.cos(2 * np.arctan(0.5)) - 0.6) < 1e-15 and abs(np.sin(2 * np.arctan(0.5)) - 0.8) < 1e-15)
check("Cayley with rational leg t gives a rational point: ((1-t^2)/(1+t^2), 2t/(1+t^2)); t = 1/2 -> (3/5, 4/5)",
      (sp.Rational(1, 2) ** 0, (1 - sp.Rational(1, 4)) / (1 + sp.Rational(1, 4)), 2 * sp.Rational(1, 2) / (1 + sp.Rational(1, 4)))
      == (1, sp.Rational(3, 5), sp.Rational(4, 5)))
check("energy rows (L^2/2 = 1): detour at 2 arctan(1/sqrt2) = arccos(1/3) = 70.529 deg",
      abs(2 * np.arctan(1 / np.sqrt(2)) - np.arccos(1 / 3)) < 1e-15 and abs(np.degrees(np.arccos(1 / 3)) - 70.52877937) < 1e-7)
check("energy rows: arc at sqrt 2 rad = 81.028 deg; chord at 90 deg",
      abs(np.degrees(np.sqrt(2)) - 81.02846845) < 1e-7 and abs((1 - np.cos(np.pi / 2)) - 1) < 1e-15)
check("order of the grid: 53.13 < 57.30 < 60 and 70.53 < 81.03 < 90",
      np.degrees(2 * np.arctan(0.5)) < np.degrees(1.0) < 60 and np.degrees(np.arccos(1 / 3)) < np.degrees(np.sqrt(2)) < 90)
check("Thales twin: (1 - cos t) + (1 + cos t) = 2, i.e. 2 d^2 + 2 S = 2",
      sp.simplify((1 - sp.cos(th)) + (1 + sp.cos(th)) - 2) == 0
      and sp.simplify(2 * sp.sin(th / 2) ** 2 - (1 - sp.cos(th))) == 0 and sp.simplify(2 * sp.cos(th / 2) ** 2 - (1 + sp.cos(th))) == 0)
check("energy break-even at 90 deg is S = d^2 = 1/2", abs(np.cos(np.pi / 4) ** 2 - 0.5) < 1e-15)
e5 = [1 - np.cos(5 * deg), (5 * deg) ** 2 / 2, 2 * np.tan(2.5 * deg) ** 2]
check("small tickets: the three energy rules agree to 0.2% at 5 deg", (max(e5) - min(e5)) / min(e5) < 2.1e-3)
Hs = sp.symbols("Hs", positive=True)
haar = sp.integrate(8 / sp.pi * sp.sqrt(Hs * (1 - Hs)), (Hs, 0, sp.Rational(1, 4)))
check("Haar answer to Bertrand on SU(2): P(S < 1/4) = 1/3 - sqrt3/(4 pi) = 0.19550",
      sp.simplify(haar - (sp.Rational(1, 3) - sp.sqrt(3) / (4 * sp.pi))) == 0 and abs(float(haar) - 0.19550111) < 1e-8)

# 17. three spirals, one of them golden
from scipy.optimize import brentq
PHI = (1 + 5 ** 0.5) / 2
bsp = lambda x: np.log(1 + x * x) / (2 * np.arctan(x))          # explicit Euler's growth per radian
kk = np.arange(60)
ys = (1 + 0.3j) ** kk
check("explicit Euler points lie exactly on r = exp(b theta), b = ln(1+s^2)/(2 arctan s) (s = 0.3, 60 steps)",
      bool(np.allclose(np.log(np.abs(ys)), bsp(0.3) * np.unwrap(np.angle(ys)))))
yi = (1 / (1 - 0.3j)) ** kk
check("implicit Euler lies on the mirror spiral r = exp(-b theta), same pitch",
      bool(np.allclose(np.log(np.abs(yi)), -bsp(0.3) * np.unwrap(np.angle(yi)))))
check("Crank-Nicolson is the pitch-zero member (the circle)", bool(np.allclose(np.abs(((1 + 0.15j) / (1 - 0.15j)) ** kk), 1)))
check("s = 0.3: growth 1.2614 per quarter turn, pitch 8.41 deg",
      abs(np.exp(bsp(0.3) * np.pi / 2) - 1.261404596) < 1e-8 and abs(np.degrees(np.arctan(bsp(0.3))) - 8.4096679) < 1e-6)
bg = 2 * np.log(PHI) / np.pi
check("golden spiral: growth phi per quarter turn, b = 2 ln(phi)/pi = 0.306349, pitch 17.03 deg",
      abs(bg - 0.30634896) < 1e-8 and abs(np.degrees(np.arctan(bg)) - 17.0323911) < 1e-6)
s_star = brentq(lambda x: bsp(x) - bg, 0.01, 5)
check("explicit Euler is exactly golden at s* = 0.651505 (33.08 deg per step)",
      abs(s_star - 0.6515045062) < 1e-9 and abs(np.degrees(np.arctan(s_star)) - 33.0844247) < 1e-6)
check("b(s) increases with s on (0, 3), so s* is the only golden step there",
      bool(np.all(np.diff(bsp(np.linspace(1e-3, 3, 3000))) > 0)))
fib = [1, 1]
for _ in range(30):
    fib.append(fib[-1] + fib[-2])
check("consecutive Fibonacci ratios tend to phi (F31/F30 within 1e-12)", abs(fib[31] / fib[30] - PHI) < 1e-12)

print(f"\n{sum(ok)}/{len(ok)} PASS")
