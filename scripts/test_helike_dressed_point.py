"""Integration tests for the dressed chart's solver plumbing and the selection fix (flux-method §5.10, A1).
Real solves at cheap points only (Mq = 6). usage: python scripts/test_helike_dressed_point.py [--mutants]"""
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import helike_domain_gate as dg
import helike_dressed_point as dp

WORK = Path(os.environ.get("DRESSED_WORK", os.path.join(tempfile.gettempdir(), "dressed_point_test")))
WORK.mkdir(parents=True, exist_ok=True)


def cone(extra, tag):
    out = WORK / f"cone_{tag}.json"
    subprocess.run([str(dp.BIN), "--out", str(out), "cone", "--s", "1.15", "--t", "3", "--g2", "0.004", "--mq", "6",
                    *extra], check=True, capture_output=True, text=True)
    return json.loads(out.read_text())["results"]["result"]


def export(sel, s, g2, tag):
    out = WORK / f"export_{tag}.json"
    r = subprocess.run([str(dp.BIN), "--out", str(WORK / f"export_{tag}.receipt.json"), "cone-export", "--s", str(s),
                        "--t", "3", "--g2", str(g2), "--mq", "6", "--states-out", str(out), "--selection", sel],
                       capture_output=True, text=True)
    return r, (json.loads(out.read_text()) if r.returncode == 0 else None)


class Plumbing(unittest.TestCase):
    def test_bare_chart_file_reproduces_the_bare_solver_exactly(self):
        X = dp.export(1.15, 3.0, 0.004, 6, WORK / "bare6.json")
        dp.write_chart(WORK / "barechart.json", X["G0"], X["M"], X["w"], X["roles"])
        a, b = cone([], "bare"), cone(["--chart", str(WORK / "barechart.json")], "chart")
        self.assertEqual(a["E0"], b["E0"])
        for n in ("helike", "competitor"):
            for k in ("gap", "weight", "dressed_weight", "level_index"):
                self.assertEqual(a[n][k], b[n][k], (n, k))

    def test_dressing_loop_settles_with_bare_roles(self):
        chart, X, h = dp.dressed_point(1.15, 3.0, 0.004, 6, WORK / "loop")
        self.assertLessEqual(len(h) - 1, 6)
        self.assertTrue(all(r["asymmetry"] <= dp.ASYM_TOL for r in h[1:]))
        self.assertEqual(h[-1]["roles"], [int(r) for r in dp.gm.load_levels(WORK / "loop/levels_bare.json")["roles"]])
        self.assertLess(h[-1]["centre_change"], 1e-6)


class Selection(unittest.TestCase):
    def test_grid_selection_accepts_what_the_grid_identified(self):
        """At s = 1.18, t = 3, g2 = 0.03, Mq = 6 the competitor has w~ = 0.916: the quasimode rule refuses, the grid
        rule exports the identified states, with Helike's partner found by charge."""
        rq, _ = export("quasimode", 1.18, 0.03, "q")
        self.assertNotEqual(rq.returncode, 0)
        self.assertIn("ambiguous state selection", rq.stderr + rq.stdout)
        rg, eg = export("grid", 1.18, 0.03, "g")
        self.assertEqual(rg.returncode, 0, rg.stderr)
        self.assertEqual(len(set(eg["level_indices"])), 4)
        self.assertTrue(all(abs(abs(c) - 2) < 0.05 for c in eg["compressed_charges"]))

    def test_both_selections_agree_where_both_apply(self):
        rq, eq = export("quasimode", 1.15, 0.004, "qc")
        rg, eg = export("grid", 1.15, 0.004, "gc")
        self.assertEqual((rq.returncode, rg.returncode), (0, 0))
        self.assertEqual(eq["level_indices"], eg["level_indices"])

    def test_domain_gate_asks_for_the_grid_selection(self):
        args = dg.export_args(1.18, 3.0, 0.03, 10, "x.json")
        self.assertEqual(args[args.index("--selection") + 1], "grid")
        args = dg.export_args(1.18, 3.0, 0.03, 10, "x.json", chart="c.json")
        self.assertEqual(args[args.index("--chart") + 1], "c.json")


MUTANTS = {}


def mutant(name):
    def deco(f):
        MUTANTS[name] = f
        return f
    return deco


@mutant("the export's 0.95 rule restored in the domain gate")
def _m1():
    orig = dg.SELECTION
    dg.SELECTION = "quasimode"
    return lambda: setattr(dg, "SELECTION", orig)


def run():
    suite = unittest.TestSuite()
    for case in (Plumbing, Selection):
        suite.addTests(unittest.defaultTestLoader.loadTestsFromTestCase(case))
    return unittest.TextTestRunner(verbosity=0).run(suite)


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
