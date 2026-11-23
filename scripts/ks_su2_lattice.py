"""Kogut-Susskind SU(2) lattice gauge theory on small graphs, built two independent ways.

Lattice units (a = 1), coupling g:
    H = (g^2/2) sum_links E_l^2  +  (2/g^2) sum_plaquettes (2 - tr U_p).

Link Hilbert space: L^2(SU(2)) truncated to spins j <= jmax, orthonormal basis f_{j m m'}(U) = sqrt(2j+1) D^j_{m m'}(U),
m, m' = j, j-1, ..., -j. Everything is defined from that basis and checked against sampled D-functions:

* `link_U(jmax)`: multiplication by the fundamental matrix element U_{ab}. On coefficients, c -> M c with
  M[(J,mu,mu'),(j,m,m')] = sqrt((2j+1)/(2J+1)) <j m; 1/2 a|J mu> <j m'; 1/2 b|J mu'>   (product rule for D-functions).
* Gauge generators, from U -> g U h^{-1} on the coefficient matrix c_{m m'}: the tail end (first index) carries
  J^T, the head end (second index) carries -J. Gauss's law at a vertex: their sum over the vertex's ends annihilates
  physical states.
* Inverse link: U^{-1}_{ab} = conj(U_{ba}) = (-1)^{b-a} U_{-b,-a}, checked numerically.

Two Hamiltonians:
* `BruteForce`: the full product space with Gauss's law as an energy penalty mu * sum_v |G_v|^2 (tiny graphs only).
* `SpinNetwork`: link spins with the unique intertwiner at each vertex of valence <= 3; the electric term is diagonal
  and tr U_p is assembled from Clebsch-Gordan contractions at the plaquette's vertices.
"""
import itertools
from functools import lru_cache

import numpy as np
import scipy.sparse as sp
from scipy.linalg import expm, null_space
from sympy import Rational
from sympy.physics.quantum.cg import CG

HALF = (0.5, -0.5)                     # fundamental index values a = +1/2, -1/2 (row 0, row 1)


def spins(jmax):
    return [k / 2 for k in range(int(round(2 * jmax)) + 1)]


@lru_cache(maxsize=None)
def spin_mats(j):
    d = int(round(2 * j)) + 1
    ms = j - np.arange(d)
    Jp = np.zeros((d, d))
    for k in range(1, d):
        m = ms[k]
        Jp[k - 1, k] = np.sqrt(j * (j + 1) - m * (m + 1))
    return ((Jp + Jp.T) / 2, (Jp - Jp.T) / 2j, np.diag(ms).astype(complex))


def D(j, theta):
    Jx, Jy, Jz = spin_mats(j)
    return expm(1j * (theta[0] * Jx + theta[1] * Jy + theta[2] * Jz))


def midx(j, m):
    return int(round(j - m))


@lru_cache(maxsize=None)
def cg(j1, m1, j2, m2, J, M):
    if abs(m1 + m2 - M) > 1e-9 or J < abs(j1 - j2) - 1e-9 or J > j1 + j2 + 1e-9 or abs(M) > J + 1e-9:
        return 0.0
    r = lambda x: Rational(int(round(2 * x)), 2)  # noqa: E731
    return float(CG(r(j1), r(m1), r(j2), r(m2), r(J), r(M)).doit())


# ── one link ─────────────────────────────────────────────────────────────────────────────────────────────────────────

class LinkSpace:
    def __init__(self, jmax):
        self.jmax = jmax
        self.basis = [(j, m, mp) for j in spins(jmax) for m in (j - np.arange(int(round(2 * j)) + 1))
                      for mp in (j - np.arange(int(round(2 * j)) + 1))]
        self.index = {b: i for i, b in enumerate(self.basis)}
        self.d = len(self.basis)
        self.casimir = np.array([j * (j + 1) for j, _, _ in self.basis])
        self.U = {}
        for ia, a in enumerate(HALF):
            for ib, b in enumerate(HALF):
                M = np.zeros((self.d, self.d))
                for k, (j, m, mp) in enumerate(self.basis):
                    for J in (j - 0.5, j + 0.5):
                        if J < 0 or J > jmax + 1e-9:
                            continue
                        f = np.sqrt((2 * j + 1) / (2 * J + 1))
                        for mu in (m + a,):
                            for mup in (mp + b,):
                                if abs(mu) > J + 1e-9 or abs(mup) > J + 1e-9:
                                    continue
                                M[self.index[(J, mu, mup)], k] += f * cg(j, m, 0.5, a, J, mu) * cg(j, mp, 0.5, b, J, mup)
                self.U[(ia, ib)] = M
        # gauge generators on the tail end (first index) and the head end (second index)
        self.gen_tail, self.gen_head = [], []
        for a in range(3):
            T = np.zeros((self.d, self.d), complex)
            Hd = np.zeros((self.d, self.d), complex)
            for j in spins(jmax):
                Ja = spin_mats(j)[a]
                dj = int(round(2 * j)) + 1
                o = self.index[(j, j, j)]
                T[o:o + dj * dj, o:o + dj * dj] = np.kron(Ja.T, np.eye(dj))
                Hd[o:o + dj * dj, o:o + dj * dj] = -np.kron(np.eye(dj), Ja)
            self.gen_tail.append(T)
            self.gen_head.append(Hd)

    def f(self, theta):
        """Values of the basis functions at the group element exp(i theta . sigma/2)."""
        out = np.zeros(self.d, complex)
        for j in spins(self.jmax):
            Dj = D(j, theta)
            for (jj, m, mp), k in self.index.items():
                if jj == j:
                    out[k] = np.sqrt(2 * j + 1) * Dj[midx(j, m), midx(j, mp)]
        return out

    def Uinv(self, ia, ib):
        """Multiplication by (U^{-1})_{ab} = conj(U_{ba}): the adjoint of multiplication by U_{ba}."""
        return self.U[(ib, ia)].conj().T


# ── graphs ───────────────────────────────────────────────────────────────────────────────────────────────────────────

def ladder(p, periodic=False):
    """p plaquettes in a row. Vertices: bottom b_i, top t_i. Links: bottom b_i->b_{i+1}, top t_i->t_{i+1},
    rungs b_i->t_i. Plaquette i, counterclockwise: bottom_i forward, rung_{i+1} forward, top_i backward, rung_i backward."""
    nv = p if periodic else p + 1
    b = lambda i: i % nv               # noqa: E731
    t = lambda i: nv + i % nv          # noqa: E731
    links = []
    bottom = [len(links) + i for i in range(p)]; links += [(b(i), b(i + 1)) for i in range(p)]
    top = [len(links) + i for i in range(p)]; links += [(t(i), t(i + 1)) for i in range(p)]
    rung = [len(links) + i for i in range(nv)]; links += [(b(i), t(i)) for i in range(nv)]
    plaqs = []
    for i in range(p):
        steps = [(bottom[i], True), (rung[(i + 1) % nv], True), (top[i], False), (rung[i % nv], False)]
        verts = [b(i), b(i + 1), t(i + 1), t(i)]
        plaqs.append((steps, verts))
    return {"nv": 2 * nv, "links": links, "plaqs": plaqs, "name": f"{'ring' if periodic else 'ladder'}{p}"}


def ends_at(graph):
    ends = {v: [] for v in range(graph["nv"])}
    for e, (tl, hd) in enumerate(graph["links"]):
        ends[tl].append((e, "tail"))
        ends[hd].append((e, "head"))
    return ends


# ── brute force: product space with a Gauss penalty ──────────────────────────────────────────────────────────────────

class BruteForce:
    def __init__(self, graph, jmax):
        self.g, self.L = graph, LinkSpace(jmax)
        self.nl = len(graph["links"])
        self.dim = self.L.d ** self.nl

    def _kron(self, ops):
        out = sp.csr_matrix(np.ones((1, 1)))
        for e in range(self.nl):
            out = sp.kron(out, sp.csr_matrix(ops.get(e, np.eye(self.L.d))), format="csr")
        return out

    def electric(self):
        return sum(self._kron({e: np.diag(self.L.casimir)}) for e in range(self.nl))

    def trU(self, pl):
        steps, _ = self.g["plaqs"][pl]
        n = len(steps)
        out = None
        for al in itertools.product((0, 1), repeat=n):
            ops = {}
            for k, (e, fwd) in enumerate(steps):
                a, b = al[k], al[(k + 1) % n]
                ops[e] = self.L.U[(a, b)] if fwd else self.L.Uinv(a, b)
            term = self._kron(ops)
            out = term if out is None else out + term
        return out

    def gauss(self):
        pen = None
        for v, ends in ends_at(self.g).items():
            for a in range(3):
                G = sum(self._kron({e: (self.L.gen_tail if kind == "tail" else self.L.gen_head)[a]}) for e, kind in ends)
                term = (G.conj().T @ G).real
                pen = term if pen is None else pen + term
        return pen

    def hamiltonian(self, g2, mu=200.0):
        H = 0.5 * g2 * self.electric()
        for pl in range(len(self.g["plaqs"])):
            H = H + (2 / g2) * (2 * sp.identity(self.dim) - self.trU(pl).real)
        return H, mu * self.gauss()

    def wavefunction(self, c, thetas):
        """psi(U_1..U_n) = sum_k c_k prod_e f_{k_e}(U_e), with U_e = exp(i theta_e . sigma/2)."""
        vals = c.reshape([self.L.d] * self.nl)
        for th in thetas:
            vals = np.tensordot(self.L.f(th), vals, axes=([0], [0]))
        return complex(vals)


# ── spin networks ────────────────────────────────────────────────────────────────────────────────────────────────────

@lru_cache(maxsize=None)
def intertwiner(endspec):
    """Unit invariant tensor for ends ((j, kind), ...); kind 'tail' carries J^T, 'head' carries -J. None if absent."""
    dims = [int(round(2 * j)) + 1 for j, _ in endspec]
    n = int(np.prod(dims))
    rows = []
    for a in range(3):
        G = np.zeros((n, n), complex)
        for i, (j, kind) in enumerate(endspec):
            Ja = spin_mats(j)[a]
            op = Ja.T if kind == "tail" else -Ja
            mats = [np.eye(d) for d in dims]
            mats[i] = op
            K = mats[0]
            for M in mats[1:]:
                K = np.kron(K, M)
            G += K
        rows.append(G)
    N = null_space(np.vstack(rows))
    if N.shape[1] == 0:
        return None
    if N.shape[1] > 1:
        raise ValueError(f"intertwiner not unique for {endspec}")
    v = N[:, 0]
    k = int(np.argmax(np.abs(v) > 1e-10))                      # deterministic phase: first nonzero entry positive
    v = v * (abs(v[k]) / v[k])
    return v.reshape(dims)


_VF_CACHE = {}


def _plus(a):
    return a


def _minus(a):
    return -a


def _ph_none(a):
    return 1.0


def _ph_out(a):
    return np.exp(-1j * np.pi * a)


def _ph_in(a):
    return np.exp(1j * np.pi * a)


def _ph_both(a):
    return 1.0 + 0j       # exp(-i pi a) * exp(i pi a)


class SpinNetwork:
    def __init__(self, graph, jmax):
        self.g, self.jmax = graph, jmax
        self.ends = ends_at(graph)
        self.nl = len(graph["links"])
        self.states = self._enumerate()
        self.index = {s: i for i, s in enumerate(self.states)}

    def _vertex_ok(self, v, js):
        spec = tuple((js[e], kind) for e, kind in self.ends[v])
        return intertwiner(spec) is not None

    def _enumerate(self):
        order = list(range(self.nl))
        complete_after = {}
        for v, ends in self.ends.items():
            last = max(order.index(e) for e, _ in ends)
            complete_after.setdefault(last, []).append(v)
        out, js = [], [None] * self.nl

        def rec(k):
            if k == self.nl:
                out.append(tuple(js)); return
            for j in spins(self.jmax):
                js[k] = j
                if all(self._vertex_ok(v, js) for v in complete_after.get(k, [])):
                    rec(k + 1)
            js[k] = None
        rec(0)
        return out

    def electric(self):
        return np.array([sum(j * (j + 1) for j in s) for s in self.states])

    def _vertex_factor(self, v, s, s2, xmap, phase):
        roles = []
        for (e, kind) in self.ends[v]:
            roles.append(xmap[(e, kind)].__name__ if (e, kind) in xmap else "-")
        key = (tuple((s[e], kind) for e, kind in self.ends[v]), tuple(s2[e] for e, _ in self.ends[v]),
               tuple(roles), phase.__name__)
        hit = _VF_CACHE.get(key)
        if hit is None:
            hit = self._vertex_factor_raw(v, s, s2, xmap, phase)
            _VF_CACHE[key] = hit
        return hit

    def _vertex_factor_raw(self, v, s, s2, xmap, phase):
        """Contract the intertwiners at v (ket s, bra s2) through the plaquette's two ends at v. xmap: end ->
        function alpha -> x (the 1/2-index the link end couples). phase: alpha -> complex."""
        spec = tuple((s[e], kind) for e, kind in self.ends[v])
        spec2 = tuple((s2[e], kind) for e, kind in self.ends[v])
        iota, iota2 = intertwiner(spec), intertwiner(spec2)
        total = 0j
        for alpha in HALF:
            mats = []
            for (e, kind) in self.ends[v]:
                j, J = s[e], s2[e]
                if (e, kind) in xmap:
                    x = xmap[(e, kind)](alpha)
                    C = np.zeros((int(round(2 * J)) + 1, int(round(2 * j)) + 1))
                    for im, m in enumerate(j - np.arange(int(round(2 * j)) + 1)):
                        mu = m + x
                        if abs(mu) <= J + 1e-9:
                            C[midx(J, mu), im] = cg(j, m, 0.5, x, J, mu)
                    mats.append(C)
                else:
                    if j != J:
                        return 0j
                    mats.append(np.eye(int(round(2 * j)) + 1))
            K = mats[0]
            for M in mats[1:]:
                K = np.kron(K, M)
            total += phase(alpha) * (iota2.ravel().conj() @ (K @ iota.ravel()))
        return total

    def trU(self, pl):
        steps, verts = self.g["plaqs"][pl]
        n = len(steps)
        links = [e for e, _ in steps]
        rows, cols, vals = [], [], []
        for s in self.states:
            for shifts in itertools.product((-0.5, 0.5), repeat=n):
                s2 = list(s)
                for e, dj in zip(links, shifts):
                    s2[e] = s[e] + dj
                if any(j < 0 or j > self.jmax + 1e-9 for j in (s2[e] for e in links)):
                    continue
                s2 = tuple(s2)
                if s2 not in self.index:
                    continue
                amp = np.prod([np.sqrt((2 * s[e] + 1) / (2 * s2[e] + 1)) for e in links])
                for k, v in enumerate(verts):
                    # step k leaves v (index alpha_k), step k-1 arrives at v (also alpha_k)
                    xmap = {}
                    e_out, fwd_out = steps[k]
                    e_in, fwd_in = steps[k - 1]
                    if fwd_out:
                        xmap[(e_out, "tail")] = _plus
                    else:
                        xmap[(e_out, "head")] = _minus
                    if fwd_in:
                        xmap[(e_in, "head")] = _plus
                    else:
                        xmap[(e_in, "tail")] = _minus
                    phase = {(True, True): _ph_none, (False, True): _ph_out, (True, False): _ph_in,
                             (False, False): _ph_both}[(fwd_out, fwd_in)]
                    amp = amp * self._vertex_factor(v, s, s2, xmap, phase)
                    if abs(amp) < 1e-15:
                        break
                if abs(amp) > 1e-14:
                    rows.append(self.index[s2]); cols.append(self.index[s]); vals.append(amp)
        n_ = len(self.states)
        return sp.csr_matrix((vals, (rows, cols)), shape=(n_, n_))

    def hamiltonian(self, g2):
        n = len(self.states)
        H = sp.diags(0.5 * g2 * self.electric())
        self.plaquette_ops = [self.trU(pl) for pl in range(len(self.g["plaqs"]))]
        for T in self.plaquette_ops:
            H = H + (2 / g2) * (2 * sp.identity(n) - T)
        return H
