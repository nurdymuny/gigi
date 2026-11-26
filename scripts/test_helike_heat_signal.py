"""Tests and mutants for the Helike heat signal (registration and amendment 1). Synthetic records only.
usage: python scripts/test_helike_heat_signal.py [--mutants]"""
import copy
import math
import sys
import unittest

import numpy as np

import helike_heat_signal as hs

NM = len(hs.MONO)


def record(levels, overlaps, bare=None, g2=0.004, coarse_shift=0.0):
    """Synthetic cone-heat record. overlaps: dict monomial -> list of <y^a G|psi_c> over levels (G-orthogonal,
    orthonormal monomial sources, so the Gram is the identity and p = 0)."""
    k = len(levels)
    d = np.zeros((NM, k))
    for m, row in overlaps.items():
        d[hs.IDX[m], :] = row
    amp = np.zeros((7, k)) if bare is None else bare

    def order(shift):
        return {"order": 12, "reference_amplitudes": amp.ravel().tolist(), "d": (d + shift).ravel().tolist(),
                "n": np.eye(NM).ravel().tolist(), "p": [0.0] * NM, "s": [1.0] + [0.0] * (k - 1)}
    return {"g2": g2, "levels": list(levels), "roles": [0, 1, 2, 5, 3, 4],
            "identification": {"helike": {"level_index": 1}, "competitor": {"level_index": 3}},
            "fine": order(0.0), "coarse": order(coarse_shift)}


LEV = [0.0, 1.0, 1.0, 1.1, 2.0, 2.05, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6]


def harmonic():
    z = [0.0] * 12
    ov = {m: list(z) for m in hs.MONO}
    ov["h1"] = [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    ov["h2"] = [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    ov["c"] = [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0]
    ov["h1c"] = [0, 0.6, 0, 0, 0.8, 0, 0, 0, 0, 0, 0, 0]
    ov["h2c"] = [0, 0, 0.6, 0, 0, 0.8, 0, 0, 0, 0, 0, 0]
    ov["s1s1"] = [0, 0.1, 0, 0, 0, 0, 0.99, 0, 0, 0, 0, 0]      # charge-2 part (s1^2 - s2^2 pairs with h1)
    ov["s2s2"] = [0, -0.1, 0, 0, 0, 0, -0.99, 0, 0, 0, 0, 0]    # s1^2 + s2^2 (charge 0) has no weight on Helike
    ov["s1s2"] = [0, 0, 0.05, 0, 0, 0, 0, 0.99, 0, 0, 0, 0]
    ov["h1h1"] = [0, 0, 0, 0.3, 0, 0, 0, 0, 0.95, 0, 0, 0]
    ov["h2h2"] = [0, 0, 0, 0.3, 0, 0, 0, 0, 0, 0.95, 0, 0]
    amp = np.zeros((7, 12)); amp[0, 1] = 1.0; amp[1, 2] = 1.0; amp[2, 3] = 1.0
    return record(LEV, ov, bare=amp)


class Signal(unittest.TestCase):
    def test_harmonic_signal_plateau_and_tone(self):
        r = harmonic()
        w, miss = hs.level_weights(r, "H")
        self.assertAlmostEqual(w[1] + w[2], 1.0, places=12)
        self.assertAlmostEqual(miss, 0.0, places=12)
        n, d = hs.plateau(r["levels"], w)
        self.assertEqual((n, d), (1, 1.0))
        tau = np.array([0.0, 0.5, 2.0])
        self.assertTrue(np.allclose(hs.signal(r["levels"], w, tau), np.exp(-tau)))
        tn = hs.tone(r["levels"], hs.level_weights(r, "H''")[0])
        self.assertEqual(tn["levels"][0], [1, 2])                  # the degenerate pair counts once
        self.assertAlmostEqual(tn["ratios"][1], 2.1, places=12)

    def test_corridor_and_selection_rule(self):
        r = harmonic()
        a = hs.t1(r)
        self.assertEqual(a["helike"]["verdict"], "CORRIDOR")
        self.assertEqual(a["helike"]["corridor_level"], 1)
        self.assertEqual(a["competitor"]["corridor_level"], 3)
        # plant a charge-0 source with zero weight on the corridor level: a selection rule, not a split
        r2 = copy.deepcopy(r)
        F = r2["fine"]; d = np.array(F["d"]).reshape(NM, 12)
        d[hs.IDX["h1h1"], 3] = 0.0; d[hs.IDX["h2h2"], 3] = 0.0
        F["d"] = d.ravel().tolist()
        b = hs.t1(r2)
        self.assertIn("C''", b["competitor"]["zero_at_corridor"])    # amendment 2: listed, and the verdict is SPLIT
        self.assertEqual(b["competitor"]["verdict"], "SPLIT")

    def test_planted_split(self):
        r = harmonic()
        F = r["fine"]; d = np.array(F["d"]).reshape(NM, 12)
        d[hs.IDX["c"], 3] = 0.0; d[hs.IDX["c"], 4] = 1.0          # C now first weighted on level 4
        F["d"] = d.ravel().tolist()
        a = hs.t1(r)
        self.assertEqual(a["competitor"]["corridor_level"], 4)
        self.assertEqual(a["competitor"]["verdict"], "SPLIT")      # C'' lives on level 3 and misses level 4

    def test_t1b_against_identification(self):
        r = harmonic()
        self.assertEqual(hs.t1b(r, hs.t1(r))["helike"], "AGREES")
        r["identification"]["helike"]["level_index"] = 2
        self.assertEqual(hs.t1b(r, hs.t1(r))["helike"], "DISAGREES")

    def test_t2_tolerances(self):
        a = harmonic(); b = harmonic()
        self.assertEqual(hs.t2(a, b)["helike"]["verdict"], "TONE-STABLE")
        b2 = copy.deepcopy(b); b2["levels"][4] += 0.03 * 1.0              # moves a ratio by 0.03 in the H'' tone? use H
        b3 = copy.deepcopy(b); b3["levels"] = [x * 1.0 for x in b["levels"]]
        # scale the whole spectrum: the octave moves, the tone does not
        b3["levels"] = [x * 1.7 for x in b["levels"]]
        self.assertEqual(hs.t2(a, b3)["helike"]["verdict"], "TONE-STABLE")
        # change the second weighted level of the registered competitor source by > tolerance
        a4 = harmonic(); F = a4["fine"]; d = np.array(F["d"]).reshape(NM, 12)
        d[hs.IDX["c"], 3] = 0.9; d[hs.IDX["c"], 6] = math.sqrt(1 - 0.81); F["d"] = d.ravel().tolist()
        b4 = copy.deepcopy(a4); b4["levels"][6] = a4["levels"][6] + 0.05
        self.assertEqual(hs.t2(a4, b4)["competitor"]["verdict"], "TONE-SHIFT")

    def test_t4_seams(self):
        r = harmonic(); lo = copy.deepcopy(r)
        self.assertEqual(hs.t4(r, lo)["verdict"], "SIGNAL-INVARIANT")
        lo2 = copy.deepcopy(r); lo2["levels"] = [0.0, 1.004, 1.004] + r["levels"][3:]
        self.assertEqual(hs.t4(r, lo2)["verdict"], "SIGNAL-SENSITIVE")
        q = record(LEV, {m: [0.0] * 12 for m in hs.MONO}, coarse_shift=1e-3)
        q2 = harmonic(); q2["coarse"]["d"] = (np.array(q2["fine"]["d"]) + 1e-3).tolist()
        self.assertEqual(hs.t4(q2, None)["verdict"], "SIGNAL-SENSITIVE")

    def test_missing_weight_is_reported(self):
        r = harmonic()
        F = r["fine"]; d = np.array(F["d"]).reshape(NM, 12)
        d[hs.IDX["c"], 3] = 0.6                                    # 64% of the source lies outside the 12 levels
        F["d"] = d.ravel().tolist()
        _, miss = hs.level_weights(r, "C")
        self.assertAlmostEqual(miss, 0.64, places=12)
        self.assertAlmostEqual(hs.t1(r)["competitor"]["sources"]["C"]["missing_weight"], 0.64, places=12)

    def test_tone_is_normalised_by_the_plateau_not_the_room(self):
        a = harmonic(); F = a["fine"]; d = np.array(F["d"]).reshape(NM, 12)
        d[hs.IDX["c"], 3] = 0.8; d[hs.IDX["c"], 6] = 0.6; F["d"] = d.ravel().tolist()
        b = copy.deepcopy(a)
        b["levels"][1] = b["levels"][2] = 0.7                       # only Helike's levels move; the competitor's do not
        self.assertEqual(hs.t2(a, b)["competitor"]["verdict"], "TONE-STABLE")

    def test_k_truncation_band_holds_when_truncation_is_harmless(self):
        r = harmonic()
        self.assertTrue(hs.t4(r, None)["seams"]["helike"]["levels_k8_vs_k12"])


MUTANTS = {}


def mutant(name):
    def deco(f):
        MUTANTS[name] = f
        return f
    return deco


@mutant("plateau from the first level, not the first weighted level")
def _m1():
    orig = hs.plateau
    hs.plateau = lambda levels, w: (1, levels[1] - levels[0])
    return lambda: setattr(hs, "plateau", orig)


@mutant("missing weight dropped from the k-truncation band")
def _m2():
    orig = hs.level_weights
    def lw(rec, name, order="fine", k=None):
        w, miss = orig(rec, name, order, k)
        return w, 0.0
    hs.level_weights = lw
    return lambda: setattr(hs, "level_weights", orig)


@mutant("curve normalised by the room (first level) instead of the plateau")
def _m3():
    orig = hs.normalised_curve
    hs.normalised_curve = lambda levels, w: hs.signal(levels, w, hs.X_GRID / (levels[1] - levels[0]))
    return lambda: setattr(hs, "normalised_curve", orig)


@mutant("T2 tolerance loosened")
def _m4():
    orig = hs.TONE_TOL
    hs.TONE_TOL = 0.2
    return lambda: setattr(hs, "TONE_TOL", orig)


@mutant("corridor taken from the lowest level any source reaches (the unamended rule)")
def _m5():
    orig = hs.REGISTERED
    hs.REGISTERED = {"helike": "R2", "competitor": "R0"}
    return lambda: setattr(hs, "REGISTERED", orig)


def run():
    return unittest.TextTestRunner(verbosity=0).run(unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__]))


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
