"""Tests and mutants for the M8 geometry kernel (registration §6, A1.4, A2.3, A3.4). Closed-form Gaussians only.
usage: python scripts/test_helike_m8_geometry.py [--mutants]"""
import math
import sys
import unittest

import numpy as np

import helike_m8_geometry as gm

W6 = np.array([1.0, 1.3, 0.7, 2.1, 1.7, 0.9])
BLOCKS = {"H": (0, 1), "C": (2,), "S": (3, 4), "R": (5,)}


def harmonic_core(order=6, w=W6, G=None, Omega=None):
    """H = 1/2 grad.G grad + 1/2 z^T K z with constant G, ground state exp(-1/2 z^T Omega z), K = Omega G Omega.
    Defaults: G = I, Omega = diag(w) (the plain oscillator)."""
    Wn, x = gm.grid_nodes(order, w)
    G = np.eye(6) if G is None else np.asarray(G)
    Om = np.diag(w) if Omega is None else np.asarray(Omega)
    K = Om @ G @ Om
    E0 = 0.5 * np.trace(G @ Om)
    core = gm.Core(Wn, x, w, lambda z: np.broadcast_to(G, (len(z), 6, 6)), np.zeros(6),
                   lambda z: 0.5 * np.einsum("ni,ij,nj->n", z, K, z), y_fn=lambda z: z,
                   dy_fn=lambda z: np.broadcast_to(np.eye(6), (len(z), 6, 6)))
    z = core.z
    # chi0 = gauss_x * p0  =>  p0 = exp(-1/2 z^T (Omega - diag w) z), normalised below
    Dm = Om - np.diag(w)
    p0 = np.exp(-0.5 * np.einsum("ni,ij,nj->n", z, Dm, z))
    dp0 = -(z @ Dm) * p0[:, None] / np.sqrt(w)               # d/dx = (1/sqrt w) d/dz
    nrm = math.sqrt(np.sum(core.W * p0 * p0))
    return core, p0 / nrm, dp0 / nrm, E0, G, Om


def linear_state(core, p0, dp0, a):
    """p for f = a.z (times psi0): p = (a.z) p0, dp/dx = (a/sqrt w) p0 + (a.z) dp0, normalised."""
    z = core.z
    f = z @ a
    p = f * p0
    dp = (a / np.sqrt(core.w))[None, :] * p0[:, None] + f[:, None] * dp0
    nrm = math.sqrt(np.sum(core.W * p * p))
    return p / nrm, dp / nrm


class Kernel(unittest.TestCase):
    def test_plain_oscillator(self):
        """f = its coordinate: share 1 on its block, alignment 1, price = gap = w_j, residual term 0."""
        core, p0, dp0, E0, G, _ = harmonic_core()
        A = gm.frame_matrix(G, BLOCKS)
        for j, block in ((0, "H"), (2, "C"), (3, "S")):
            a = np.eye(6)[j]
            p, dp = linear_state(core, p0, dp0, a)
            r = core.analyse(p0, dp0, p, dp, E0, BLOCKS, A)
            self.assertAlmostEqual(r["D"], W6[j], places=10)
            self.assertAlmostEqual(r["R"], 0.0, places=10)
            self.assertAlmostEqual(r["rayleigh"], W6[j], places=10)
            self.assertAlmostEqual(r["shares"][block], 1.0, places=12)
            self.assertAlmostEqual(r["share_sum"], 1.0, places=12)
            self.assertAlmostEqual(core.alignment(p0, p, list(BLOCKS[block])), 1.0, places=10)
        self.assertAlmostEqual(core.shape_price(p0, dp0, [0, 1], E0), W6[0], places=10)

    def test_planted_mixture(self):
        """f = cos t yH + sin t yS (unit coordinates): pi_H = cos^2 w_H / (cos^2 w_H + sin^2 w_S), cross 0, a = cos^2."""
        core, p0, dp0, E0, G, _ = harmonic_core()
        A = gm.frame_matrix(G, BLOCKS)
        th = 0.6
        a = np.zeros(6); a[0] = math.cos(th) * math.sqrt(2 * W6[0]); a[3] = math.sin(th) * math.sqrt(2 * W6[3])
        p, dp = linear_state(core, p0, dp0, a)
        r = core.analyse(p0, dp0, p, dp, E0, BLOCKS, A)
        c2, s2 = math.cos(th) ** 2, math.sin(th) ** 2
        self.assertAlmostEqual(r["shares"]["H"], c2 * W6[0] / (c2 * W6[0] + s2 * W6[3]), places=10)
        raw = np.array(r["raw"])
        self.assertAlmostEqual(raw[0, 2], 0.0, places=12)
        self.assertAlmostEqual(core.alignment(p0, p, [0, 1]), c2, places=10)

    def test_sheared_gaussian_chart(self):
        """Constant non-identity G (block-diagonal across blocks, sheared within) and a ground state that is not the
        chart's Gaussian. For f = a.z with a a generalised eigenvector: D + R = lambda = eigenvalue of G Omega,
        R = 0, and the shares match the closed form from v = G^{1/2} a in the frame G(0)^{-1/2}."""
        rng = np.random.default_rng(3)
        G = np.eye(6); G[0, 1] = G[1, 0] = 0.3; G[3, 4] = G[4, 3] = -0.2; G[0, 0] = 1.4
        Om = np.diag(W6) + 0.05 * np.diag(rng.uniform(-1, 1, 6))
        Om[0, 2] = Om[2, 0] = 0.04
        core, p0, dp0, E0, G, Om = harmonic_core(order=10, G=G, Omega=Om)
        lam, vec = np.linalg.eig(G @ Om)                     # f = a.z with a = Omega e (e eigvec of G Omega)
        i = int(np.argmin(lam.real))
        a = Om @ vec[:, i].real
        p, dp = linear_state(core, p0, dp0, a)
        A = gm.frame_matrix(G, BLOCKS)
        r = core.analyse(p0, dp0, p, dp, E0, BLOCKS, A)
        self.assertAlmostEqual(r["identity"], lam[i].real, places=6)
        self.assertAlmostEqual(r["R"], 0.0, places=6)
        g = a / math.sqrt(a @ np.linalg.inv(Om) @ a / 2)      # ||f||^2 = a^T Omega^{-1} a / 2
        v = gm.sym_sqrt((A @ G @ A)[None])[0] @ np.linalg.solve(A, g)
        want = {b: float(np.sum(v[list(ix)] ** 2)) / float(v @ v) for b, ix in BLOCKS.items()}
        for b in BLOCKS:
            self.assertAlmostEqual(r["shares"][b], want[b], places=6)

    def test_identity_holds_for_an_inexact_ground_state(self):
        """(★) is exact for any positive psi0 and any E0; with a wrong trial ground state the residual term is
        nonzero and is needed to reproduce the direct quadrature of <psi,(H-E0)psi>."""
        core, _, _, E0, G, Om = harmonic_core(order=10)
        z = core.z
        Dm = 0.15 * np.diag([1.0, -0.5, 0.8, 0.3, -0.2, 0.6])            # a deliberately wrong width
        p0 = np.exp(-0.5 * np.einsum("ni,ij,nj->n", z, Dm, z))
        dp0 = -(z @ Dm) * p0[:, None] / np.sqrt(core.w)
        nrm = math.sqrt(np.sum(core.W * p0 * p0)); p0, dp0 = p0 / nrm, dp0 / nrm
        A = gm.frame_matrix(G, BLOCKS)
        p, dp = linear_state(core, p0, dp0, np.eye(6)[0])
        r = core.analyse(p0, dp0, p, dp, E0, BLOCKS, A)
        self.assertGreater(abs(r["R"]), 1e-3)
        self.assertAlmostEqual(r["identity"], r["rayleigh"], places=8)

    def test_reviewer_counterexample(self):
        """Raw normalised contributions (5, 5, -9); positive shares (1/2, 1/2); J in [0, 1]."""
        G = np.array([[[1.0, -0.9], [-0.9, 1.0]]])
        grad = np.array([[1.0, 1.0]])
        raw = gm.raw_contributions(grad, G, np.eye(2))[0]
        D = raw.sum()
        self.assertAlmostEqual(raw[0, 0] / D, 5.0, places=10)
        self.assertAlmostEqual(2 * raw[0, 1] / D, -9.0, places=10)
        pc = gm.positive_contributions(grad, G, np.eye(2))[0]
        self.assertTrue(np.all(pc >= 0))
        self.assertAlmostEqual(pc.sum(), D, places=12)
        self.assertTrue(np.allclose(pc / D, [0.5, 0.5]))

    def test_frame_invariance_and_refusal(self):
        """Shares are unchanged under any block-diagonal linear change of z (the frame absorbs it; within a block
        the residual freedom is orthogonal), and under permutations; a non-block-diagonal G(0) is refused."""
        rng = np.random.default_rng(7)
        n = 400
        G0 = np.eye(6); G0[0, 1] = G0[1, 0] = 0.4; G0[3, 3] = 2.0
        Gs = np.array([G0 + 0.3 * (lambda m: m @ m.T)(rng.normal(size=(6, 6))) / 6 for _ in range(n)])
        grad = rng.normal(size=(n, 6))

        def shares(G0, Gs, grad):
            A = gm.frame_matrix(G0, BLOCKS)
            pc = gm.positive_contributions(grad, Gs, A).sum(axis=0)
            return {b: pc[list(ix)].sum() / pc.sum() for b, ix in BLOCKS.items()}
        base = shares(G0, Gs, grad)
        B = np.zeros((6, 6))
        for ix in BLOCKS.values():
            m = len(ix)
            B[np.ix_(ix, ix)] = rng.normal(size=(m, m)) + 2 * np.eye(m)
        # z' = B z: contravariant G' = B G B^T, covariant grad' = B^{-T} grad
        G0p = B @ G0 @ B.T
        Gsp = np.einsum("ij,njk,lk->nil", B, Gs, B)
        gradp = np.linalg.solve(B.T, grad.T).T
        other = shares(G0p, Gsp, gradp)
        for b in BLOCKS:
            self.assertAlmostEqual(base[b], other[b], places=10)
        P = np.eye(6)[[1, 0, 2, 4, 3, 5]]                    # swaps inside H and inside S
        perm = shares(P @ G0 @ P.T, np.einsum("ij,njk,lk->nil", P, Gs, P), grad @ P.T)
        for b in BLOCKS:
            self.assertAlmostEqual(base[b], perm[b], places=10)
        bad = G0.copy(); bad[0, 2] = bad[2, 0] = 0.1
        with self.assertRaises(ValueError):
            gm.frame_matrix(bad, BLOCKS)

    def test_complex_and_real_forms_agree(self):
        """For f = f1 + i f2: 1/2 g grad conj(f).grad f = the sum over the two real partners, to 1e-12."""
        rng = np.random.default_rng(11)
        G = (lambda m: m @ m.T + np.eye(6))(rng.normal(size=(6, 6)))
        g1, g2 = rng.normal(size=6), rng.normal(size=6)
        gc = g1 + 1j * g2
        complex_form = 0.5 * np.real(np.conj(gc) @ G @ gc)
        real_form = 0.5 * (g1 @ G @ g1 + g2 @ G @ g2)
        self.assertAlmostEqual(complex_form, real_form, places=12)

    def test_positivity_is_checked(self):
        core, p0, dp0, E0, G, _ = harmonic_core()
        A = gm.frame_matrix(G, BLOCKS)
        p, dp = linear_state(core, p0, dp0, np.eye(6)[0])
        neg = p0.copy(); neg[np.argmax(core.W * p0 * p0)] *= -1
        r = core.analyse(neg, dp0, p, dp, E0, BLOCKS, A)
        self.assertGreater(r["nonpositive_weight"], 0.0)


MUTANTS = {}


def mutant(name):
    def deco(f):
        MUTANTS[name] = f
        return f
    return deco


@mutant("raw D_XX/D used as the fingerprint")
def _m1():
    orig = gm.positive_contributions
    gm.positive_contributions = lambda grad, G, A: np.einsum("nii->ni", gm.raw_contributions(grad, G, A))
    return lambda: setattr(gm, "positive_contributions", orig)


@mutant("a Cholesky factor in place of the symmetric square root")
def _m2():
    orig = gm.sym_sqrt
    gm.sym_sqrt = lambda G: np.transpose(np.linalg.cholesky(G), (0, 2, 1))
    return lambda: setattr(gm, "sym_sqrt", orig)


@mutant("the frame normalisation G(0)^{-1/2} skipped")
def _m3():
    orig = gm.frame_matrix
    gm.frame_matrix = lambda G0, blocks, tol=1e-8: (orig(G0, blocks, tol), np.eye(len(G0)))[1]
    return lambda: setattr(gm, "frame_matrix", orig)


@mutant("the weight psi0^2 dropped from the measure")
def _m4():
    orig = gm.Core.analyse
    def analyse(self, p0, dp0, p, dp, E0, blocks, A, positivity_floor=1e-12):
        return orig(self, np.ones_like(p0), np.zeros_like(dp0), p, dp, E0, blocks, A, positivity_floor)
    gm.Core.analyse = analyse
    return lambda: setattr(gm.Core, "analyse", orig)


@mutant("the residual term dropped from Q0a")
def _m5():
    orig = gm.Core.analyse
    def analyse(self, *a, **k):
        r = orig(self, *a, **k)
        r["identity"] = r["D"]; r["R"] = 0.0
        return r
    gm.Core.analyse = analyse
    return lambda: setattr(gm.Core, "analyse", orig)


def run():
    return unittest.TextTestRunner(verbosity=0).run(unittest.defaultTestLoader.loadTestsFromTestCase(Kernel))


if __name__ == "__main__":
    if "--mutants" in sys.argv:
        caught = 0
        for name, apply in MUTANTS.items():
            undo = apply()
            try:
                ok = not run().wasSuccessful()
            finally:
                undo()
            caught += ok
            print(("CAUGHT  " if ok else "MISSED  ") + name, flush=True)
        print(f"{caught}/{len(MUTANTS)} mutants caught")
        sys.exit(0 if caught == len(MUTANTS) else 1)
    sys.exit(0 if run().wasSuccessful() else 1)
