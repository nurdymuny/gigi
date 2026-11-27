"""Diagnostic only (decides nothing): how compact are the converged Mq 10 states in a Hermite basis built in dressed
sqrt(G) coordinates, against the solver's own (dressed log-Cholesky) chart basis?
Tail weight eps_k = 1 - sum over basis functions of total degree <= k of c_n^2 (states normalised; Parseval)."""
import sys, math, itertools, json
import numpy as np
import helike_m8_geometry as gm

def sqrtG(x):
    G = np.zeros((len(x), 3, 3))
    G[:, 0, 0], G[:, 1, 1], G[:, 2, 2] = x[:, 0], x[:, 1], x[:, 2]
    G[:, 0, 1] = G[:, 1, 0] = x[:, 3]; G[:, 0, 2] = G[:, 2, 0] = x[:, 4]; G[:, 1, 2] = G[:, 2, 1] = x[:, 5]
    e, V = np.linalg.eigh(G)
    R = np.einsum("nij,nj,nkj->nik", V, np.sqrt(np.clip(e, 0, None)), V)
    return np.column_stack((R[:, 0, 0], R[:, 1, 1], R[:, 2, 2], R[:, 0, 1], R[:, 0, 2], R[:, 1, 2]))

path, order, kmax = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
X = gm.load_levels(path)
P = gm.Point(X, order)
ident = X["raw"]["identification"]
lev = {"ground": 0, "helike": ident["helike"]["level_index"], "competitor": ident["competitor"]["level_index"]}
p, _ = P.values(list(lev.values()))
W, x, z, w = P.core.W, P.core.x, P.core.z, X["w"]
p0 = p[:, 0]
keep = W * p0 * p0 > 1e-16 * np.sum(W * p0 * p0)
W, x, z, p = W[keep], x[keep], z[keep], p[keep]
p0 = p[:, 0]

# chart-basis tails, exact from the Ritz coefficients
deg = np.array([sum(b) for b in X["basis"]])
chart = {}
for i, (name, l) in enumerate(lev.items()):
    c2 = X["coeff"][:, l] ** 2
    chart[name] = [float(1 - c2[deg <= k].sum() / c2.sum()) for k in range(kmax + 1)]

# sqrt(G) frame dressed to the vacuum: zeta = (2C)^{-1/2} (P - mu), so a Gaussian vacuum has cov(zeta) = I/2
Pc = sqrtG(P.gram(z)[0])
m = W * p0 * p0; m = m / m.sum()
mu = m @ Pc; d = Pc - mu; C = (d * m[:, None]).T @ d
e, V = np.linalg.eigh(2 * C); Ai = (V / np.sqrt(e)) @ V.T
zeta = d @ Ai.T
# Jacobian |d zeta / d z| by central differences of sqrt(G(x(z)))
h = 1e-5
J = np.zeros((len(z), 6, 6))
for j in range(6):
    dz = np.zeros(6); dz[j] = h
    J[:, :, j] = (sqrtG(P.gram(z + dz)[0]) - sqrtG(P.gram(z - dz)[0])) @ Ai.T / (2 * h)
absdet = np.abs(np.linalg.det(J))
Nw = np.prod((w / math.pi) ** 0.25)
base = W * np.exp(0.5 * np.sum(x * x, axis=1)) / Nw * np.sqrt(absdet) * np.exp(-0.5 * np.sum(zeta * zeta, axis=1)) * math.pi ** -1.5
H = [gm.normalized_hermites(zeta[:, j], kmax) for j in range(6)]
states = [s for s in itertools.product(range(kmax + 1), repeat=6) if sum(s) <= kmax]
sq = {name: np.zeros(kmax + 1) for name in lev}
for s in states:
    phi = base * H[0][s[0]] * H[1][s[1]] * H[2][s[2]] * H[3][s[3]] * H[4][s[4]] * H[5][s[5]]
    c = phi @ p                                             # one coefficient per state
    for i, name in enumerate(lev):
        sq[name][sum(s)] += c[i] ** 2
unf = {name: [float(1 - sq[name][:k + 1].sum()) for k in range(kmax + 1)] for name in lev}
out = {"export": path, "order": order, "kmax": kmax, "levels": lev, "chart_tail": chart, "sqrtG_tail": unf}
print(json.dumps(out, indent=1))
