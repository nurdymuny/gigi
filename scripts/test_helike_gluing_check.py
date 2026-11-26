"""Tests and mutants for the §5.5 gluing-invariance qualifier. Synthetic data only (no solver, no gate).
usage: python scripts/test_helike_gluing_check.py [--mutants]"""
import copy
import sys
import unittest

import numpy as np

import run_helike_gluing_check as gc
import test_helike_heat_signal as th

NM = len(th.hs.MONO)


def toy():
    """The heat-signal harmonic toy, with a vacuum amplitude so that Z = 1 at both orders."""
    r = th.harmonic()
    for o in ("fine", "coarse"):
        amp = np.array(r[o]["reference_amplitudes"]).reshape(7, 12)
        amp[6, 0] = 1.0
        r[o]["reference_amplitudes"] = amp.ravel().tolist()
    return r


class Seams(unittest.TestCase):
    def test_s1_all_references_agree_on_the_toy(self):
        r = toy()
        res = gc.s1(r)
        self.assertTrue(res["holds"])
        self.assertEqual(res["selected"]["dressed"], (1, 3))

    def test_s1_catches_a_dressed_reference_that_picks_another_level(self):
        r = toy()
        F = r["fine"]; d = np.array(F["d"]).reshape(NM, 12)
        d[th.hs.IDX["c"], 3] = 0.3; d[th.hs.IDX["c"], 5] = 0.95          # dressed C now peaks on level 5
        F["d"] = d.ravel().tolist()
        self.assertFalse(gc.s1(r)["holds"])

    def test_s1_degenerate_partners_count_together(self):
        r = toy()
        F = r["fine"]; d = np.array(F["d"]).reshape(NM, 12)
        d[th.hs.IDX["h2"], 2] = 1.0001                                  # dressed argmax moves to the partner, level 2
        F["d"] = d.ravel().tolist()
        res = gc.s1(r)
        self.assertEqual(res["selected"]["dressed"][0], 2)
        self.assertTrue(res["holds"])

    def test_s2_quadrature(self):
        r = toy()
        self.assertTrue(gc.s2(r)["holds"])
        r2 = copy.deepcopy(r); amp = np.array(r2["coarse"]["reference_amplitudes"]).reshape(7, 12)
        amp[0, 1] *= 0.999; r2["coarse"]["reference_amplitudes"] = amp.ravel().tolist()
        self.assertFalse(gc.s2(r2)["holds"])

    def test_s3(self):
        lad = [{"Mq": m, "helike": {"gap": 1.0 + e}, "competitor": {"gap": 1.1}} for m, e in ((6, 0.01), (8, 0.002), (10, 0.0))]
        self.assertTrue(gc.s3(lad)["holds"])
        lad[1]["helike"]["gap"] = 1.004
        self.assertFalse(gc.s3(lad)["holds"])

    def test_s4_linear_data_all_interpolants_agree(self):
        s = [1.12, 1.15, 1.165, 1.18, 1.195, 1.22, 1.24]
        d = [x - 1.181 for x in s]
        res = gc.s4(s, d)
        self.assertTrue(res["holds"])
        self.assertAlmostEqual(res["switch"]["linear"], 1.181, places=12)

    def test_s4_curved_data_spreads(self):
        s = [1.12, 1.15, 1.165, 1.18, 1.195, 1.22, 1.24]
        d = [-0.0001, 0.0817, 0.7109, 0.8135, 1.1679, 1.6153, 20.8746]    # crossing at the bracket edge, strong curvature
        res = gc.s4(s, d)
        self.assertIsNotNone(res["switch"]["linear"])
        self.assertFalse(res["holds"])

    def test_qualifier_never_changes_the_registered_verdict(self):
        for v in ("SUPPORTED", "NOT_SUPPORTED", "UNRESOLVED", "MIXED"):
            for q in ("GLUING-INVARIANT", "GLUING-SENSITIVE"):
                self.assertEqual(gc.qualify(v, q)["registered_verdict"], v)

    def test_point_verdict(self):
        seams = {"S1": {"holds": True}, "S2": {"holds": True}, "S3": {"holds": True}, "S5": {"holds": None}}
        self.assertEqual(gc.point_verdict(seams)[0], "GLUING-INVARIANT")
        seams["S2"]["holds"] = False
        self.assertEqual(gc.point_verdict(seams), ("GLUING-SENSITIVE", ["S2"]))

    def test_s5_refuses_the_same_radius_twice(self):
        src = open(gc.__file__).read()
        self.assertIn('raise AssertionError("S5 must use two different radii")', src)
        self.assertIn("q = round(reg_rho * 4) - 1", src)


MUTANTS = {}


def mutant(name):
    def deco(f):
        MUTANTS[name] = f
        return f
    return deco


@mutant("S1 compares only the selected level (dressed reference ignored)")
def _m1():
    orig = gc.selections
    def sel(rec, order="fine"):
        out = orig(rec, order)
        out["dressed"] = out["w_over_Z"]
        return out
    gc.selections = sel
    return lambda: setattr(gc, "selections", orig)


@mutant("S4 tolerance loosened")
def _m2():
    orig = gc.SWITCH_TOL
    gc.SWITCH_TOL = 1.0
    return lambda: setattr(gc, "SWITCH_TOL", orig)


@mutant("qualifier allowed to flip a verdict")
def _m3():
    orig = gc.qualify
    gc.qualify = lambda v, q: {"registered_verdict": "UNRESOLVED" if q == "GLUING-SENSITIVE" else v, "gluing_qualifier": q}
    return lambda: setattr(gc, "qualify", orig)


@mutant("S5 reuses the registered radius")
def _m4():
    path = gc.__file__
    src = open(path).read()
    mutated = src.replace("q = round(reg_rho * 4) - 1", "q = round(reg_rho * 4)")
    open(path, "w").write(mutated)
    return lambda: open(path, "w").write(src)


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
