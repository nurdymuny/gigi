"""Registered tests and mutants for the unfolded chart's Rust solver (flux-method §5.11), helike cone-sqrtg.
Cheap points only (Mq 4, order 10). usage: python scripts/test_helike_sqrtg.py [--mutants]"""
import json
import math
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / ("helike-lab/target/release/helike" + (".exe" if sys.platform == "win32" else ""))
VS = ROOT / "research/dressed-chart-20261126/vacuum-shape"
FRAME = VS / "frame_smoke.json"
WORK = Path(os.environ.get("SQRTG_WORK", os.path.join(tempfile.gettempdir(), "sqrtg_test")))
WORK.mkdir(parents=True, exist_ok=True)
KEYS = [(0, 0), (1, 1), (2, 2), (0, 1), (0, 2), (1, 2)]


def run(tag, s, t, g2, ladder, max_iter, frame=None):
    out = WORK / f"{tag}.json"
    cmd = [str(BIN), "--out", str(out), "cone-sqrtg", "--s", repr(s), "--t", repr(t), "--g2", repr(g2), "--ladder", ladder,
           "--order-offset", "6", "--max-iter", str(max_iter)]
    if frame:
        cmd += ["--frame-in", str(frame)]
    subprocess.run(cmd, check=True, capture_output=True, text=True, env=os.environ.copy())
    return json.loads(out.read_text())["results"]["rungs"]


def rot_map(theta):
    """The 6x6 map on P-vectors of P -> R P R^T, R a rotation about axis 0 (the model's symmetry: indices 1, 2)."""
    c, s = math.cos(theta), math.sin(theta)
    R = np.array([[1, 0, 0], [0, c, -s], [0, s, c]])
    M = np.zeros((6, 6))
    for k, (i, j) in enumerate(KEYS):
        E = np.zeros((3, 3)); E[i, j] = 1; E[j, i] = 1
        if i == j:
            E[i, j] = 1
        F = R @ E @ R.T
        M[:, k] = [F[a, b] for a, b in KEYS]
    return M


class Sqrtg(unittest.TestCase):
    def test_parity_with_the_python_prototype(self):
        """Same frame, Mq 4, order 10: the Rust levels equal the prototype's (computed independently in numpy)."""
        sys.path[:0] = [str(VS), str(ROOT / "scripts")]
        import sqrtg_ritz as sr
        fr = json.loads(FRAME.read_text())
        e, V = np.linalg.eigh(2 * np.array(fr["C"]))
        A = (V * np.sqrt(e)) @ V.T
        E, _, _ = sr.solve(1.18, 3.0, 0.03, 4, 10, np.array(fr["mu"]), A)
        r = run("parity", 1.18, 3.0, 0.03, "4", 0, FRAME)[0]["iterations"][0]["levels"]
        self.assertLess(float(np.max(np.abs(np.array(r) - E))), 1e-10)

    def test_rotation_covariance(self):
        """Rotating the frame about the model's symmetry axis (P -> R P R^T) leaves every level unchanged."""
        fr = json.loads(FRAME.read_text())
        M = rot_map(0.7)
        rot = {"mu": (M @ np.array(fr["mu"])).tolist(), "C": (M @ np.array(fr["C"]) @ M.T).tolist()}
        (WORK / "frame_rot.json").write_text(json.dumps(rot))
        a = run("rot_a", 1.18, 3.0, 0.03, "4", 0, FRAME)[0]["iterations"][0]["levels"]
        b = run("rot_b", 1.18, 3.0, 0.03, "4", 0, WORK / "frame_rot.json")[0]["iterations"][0]["levels"]
        self.assertLess(float(np.max(np.abs(np.array(a) - np.array(b)))), 1e-9)

    def test_face_is_detected(self):
        """A frame squeezed toward P = 0 puts vacuum weight off the sheet; it must be reported."""
        fr = json.loads(FRAME.read_text())
        bad = {"mu": (0.08 * np.array(fr["mu"])).tolist(), "C": fr["C"]}
        (WORK / "frame_face.json").write_text(json.dumps(bad))
        it = run("face", 1.18, 3.0, 0.03, "2", 0, WORK / "frame_face.json")[0]["iterations"][0]
        self.assertGreater(it["off_sheet_weight"], 1e-6)
        ok = run("face_ok", 1.18, 3.0, 0.03, "2", 0, FRAME)[0]["iterations"][0]
        self.assertLess(ok["off_sheet_weight"], 1e-12)

    def test_self_consistency_settles(self):
        """From the bare harmonic start (the classical vacuum transported to P), the loop settles within the budget."""
        r = run("settle", 1.15, 3.0, 0.004, "2", 6, None)[0]
        self.assertTrue(r["settled"])
        self.assertLess(r["iterations"][-1]["frame_change"], 1e-6)


MUTANTS = ["no_rho", "j_not_inverted", "face_unchecked", "frame_unconverged"]


def suite():
    return unittest.defaultTestLoader.loadTestsFromTestCase(Sqrtg)


if __name__ == "__main__":
    if "--mutants" in sys.argv:
        caught = 0
        for m in MUTANTS:
            os.environ["HELIKE_SQRTG_MUTANT"] = m
            try:
                ok = not unittest.TextTestRunner(verbosity=0).run(suite()).wasSuccessful()
            finally:
                os.environ.pop("HELIKE_SQRTG_MUTANT", None)
            caught += ok
            print(("CAUGHT  " if ok else "MISSED  ") + m, flush=True)
        print(f"{caught}/{len(MUTANTS)} mutants caught")
        sys.exit(0 if caught == len(MUTANTS) else 1)
    sys.exit(0 if unittest.TextTestRunner(verbosity=2).run(suite()).wasSuccessful() else 1)
