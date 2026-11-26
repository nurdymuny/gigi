"""Tests and mutants for M8 carrier tracking (registration amendments 2 and 3). Synthetic families only.
usage: python scripts/test_helike_m8_tracking.py [--mutants]"""
import inspect
import math
import sys
import textwrap
import unittest

import numpy as np

import helike_m8_tracking as tr


def rot(theta):
    c, s = math.cos(theta), math.sin(theta)
    return np.array([[c, -s], [s, c]])


def const_spectrum(s):
    """The reviewer's counterexample: levels fixed at 1 and 2, eigenvectors rotated by 90 degrees over one grid step."""
    R = rot(math.pi * s / 2)
    return R @ np.diag([1.0, 2.0]) @ R.T


def crossing(s):
    return np.diag([2 + s, 2 - s])


def avoided(eps):
    return lambda s: np.array([[2 + s, eps], [eps, 2 - s]])


def by_energy_share(table):
    """Plant an own-block share by branch energy rank at each s (synthetic, not a Dirichlet decomposition)."""
    return lambda S, c: table(S.s, S.energy(c))


class Planted:
    """A family with planted overlaps, for the one-to-one and unresolved-matching fixtures."""

    def __init__(self, O, energies=(1.0, 2.0)):
        self.O, self.E = np.asarray(O, float), np.asarray(energies, float)

    def sample(self, s):
        return tr.Sample(s, self.E.copy(), np.zeros(len(self.E)), [(i,) for i in range(len(self.E))],
                         list(range(len(self.E))))

    def overlap(self, A, B):
        return self.O


class Tracking(unittest.TestCase):
    def test_constant_spectrum_rejection(self):
        """A3.4: the endpoint overlap favours the wrong branch with weight 1; the midpoint check must run, refinement
        must recover the energy-1 branch, no crossing is verified, the selector index changes, and the planted share
        cos^2(pi s/2) gives DISSOLVES."""
        fam = tr.MatrixFamily(const_spectrum)
        ref = np.array([1.0, 0.0])
        selector = lambda S: int(np.argmax([(ref @ S.payload[0][:, c[0]]) ** 2 for c in S.clusters]))
        share = lambda S, c: math.cos(math.pi * S.s / 2) ** 2 if abs(S.energy(c) - 1) < 1e-9 else 1.0
        rep = tr.run_case(fam, [0.0, 1.0], share, selector_fn=selector)
        self.assertEqual(rep["verdict"], "DISSOLVES")
        st = rep["steps"][0]
        self.assertEqual(st["status"], "RESOLVED")
        self.assertEqual(st["crossing"]["kind"], "NONE")
        self.assertTrue(st["rejected"], "the coarse endpoint match must be rejected")
        self.assertGreaterEqual(st["depth"], 1)
        end = rep["points"][-1]
        self.assertAlmostEqual(fam.sample(1.0).energy(end["carrier"]), 1.0)
        self.assertEqual(rep["selection_changed"], [1.0])

    def test_unresolved_matching_is_open(self):
        fam = Planted([[0.5, 0.5], [0.5, 0.5]])
        rep = tr.run_case(fam, [0.0, 1.0], lambda S, c: 0.9)
        self.assertEqual(rep["verdict"], "OPEN")
        self.assertEqual(rep["steps"][0]["depth"], tr.MAX_DEPTH)

    def test_one_to_one(self):
        fam = Planted([[0.9, 0.1], [0.6, 0.4]])          # both earlier branches prefer the same later state
        rep = tr.run_case(fam, [0.0, 1.0], lambda S, c: 0.9)
        self.assertEqual(rep["verdict"], "OPEN")

    def test_three_point_handover(self):
        fam = tr.MatrixFamily(crossing, labels=["a", "b"])
        shares = {-1.0: 0.8, 1.0: 0.4, 2.0: 0.3}
        rep = tr.run_case(fam, [-1.0, 1.0, 2.0], lambda S, c: shares[S.s])
        self.assertEqual(rep["verdict"], "HANDS OVER")
        self.assertEqual(rep["steps"][0]["crossing"]["kind"], "VERIFIED")
        self.assertEqual([p["state"] for p in rep["points"]], ["HELD", "HANDED", "HANDED"])

    def test_same_shares_without_crossing_dissolve(self):
        fam = tr.MatrixFamily(lambda s: np.diag([1.0, 2.0]))
        shares = {-1.0: 0.8, 1.0: 0.4, 2.0: 0.3}
        rep = tr.run_case(fam, [-1.0, 1.0, 2.0], lambda S, c: shares[S.s])
        self.assertEqual(rep["verdict"], "DISSOLVES")
        self.assertEqual(rep["points"][1]["state"], "DISSOLVED")

    def test_true_crossing_with_symmetry_labels_and_priority(self):
        fam = tr.MatrixFamily(crossing, labels=["a", "b"])
        rep = tr.run_case(fam, [-1.0, 1.0], lambda S, c: 0.9)          # the former carrier's share stays above 0.5
        self.assertEqual(rep["steps"][0]["crossing"]["kind"], "VERIFIED")
        self.assertEqual(rep["verdict"], "HANDS OVER")
        self.assertEqual(rep["steps"][0]["crossing"]["bracket"], (-1.0, 1.0))

    def test_resolved_avoided_crossing(self):
        fam = tr.MatrixFamily(avoided(0.005))
        rep = tr.run_case(fam, [-0.1, 0.1], lambda S, c: 0.9)
        st = rep["steps"][0]
        self.assertEqual(st["status"], "RESOLVED")
        self.assertEqual(st["crossing"]["kind"], "NONE")
        self.assertLessEqual(st["depth"], tr.MAX_DEPTH)
        self.assertEqual(rep["verdict"], "PERSISTS")
        self.assertAlmostEqual(fam.sample(0.1).energy(rep["points"][-1]["carrier"]), 2 - math.hypot(0.1, 0.005))

    def test_avoided_crossing_inside_inspection_tolerance(self):
        """Gap 8e-5 < 1e-4, but resolved within the budget: closeness is a reason to inspect, not a crossing."""
        fam = tr.MatrixFamily(avoided(4e-5))
        rep = tr.run_case(fam, [-0.001, 0.001], lambda S, c: 0.9)
        self.assertEqual(rep["steps"][0]["status"], "RESOLVED")
        self.assertEqual(rep["verdict"], "PERSISTS")

    def test_sub_tolerance_avoided_crossing_is_open(self):
        fam = tr.MatrixFamily(avoided(5e-6))
        rep = tr.run_case(fam, [-1.0, 1.0], lambda S, c: 0.9)
        self.assertEqual(rep["verdict"], "OPEN")

    def test_unresolved_energy_ordering_is_open(self):
        fam = tr.MatrixFamily(crossing, labels=["a", "b"])
        rep = tr.run_case(fam, [-1.0, 4e-5], lambda S, c: 0.9)         # end separation 8e-5 < 1e-4
        self.assertEqual(rep["verdict"], "OPEN")

        class Noisy(tr.MatrixFamily):
            def sample(self, s):
                S = super().sample(s)
                S.uncert = np.full(len(S.energies), 0.1 if s > 0 else 0.0)
                return S
        rep = tr.run_case(Noisy(crossing, labels=["a", "b"]), [-1.0, 0.05], lambda S, c: 0.9)
        self.assertEqual(rep["verdict"], "OPEN")

        rep = tr.run_case(tr.MatrixFamily(crossing, labels=["a", "b"], uncert=None), [-1.0, 1.0], lambda S, c: 0.9)
        self.assertEqual(rep["verdict"], "OPEN")                        # a missing uncertainty is not zero

    def test_crossing_hands_to_the_branch_that_becomes_lowest(self):
        fam = tr.MatrixFamily(lambda s: np.diag([1 + 3 * s, 2.0, 3 - 2.5 * s]), labels=["a", "b", "c"])
        rep = tr.run_case(fam, [0.0, 1.0], lambda S, c: 0.9)
        self.assertEqual(rep["steps"][0]["crossing"]["kind"], "VERIFIED")
        self.assertAlmostEqual(fam.sample(1.0).energy(rep["points"][-1]["carrier"]), 0.5)

    def test_inter_sector_switch_is_reported_not_a_handover(self):
        fam = tr.MatrixFamily(lambda s: np.diag([1.0, 2.0]))
        rep = tr.run_case(fam, [0.0, 1.0], lambda S, c: 0.8 if S.s == 0 else 0.3, inter_sector=(0,))
        self.assertEqual(rep["gap_handover"], [(0.0, 1.0)])
        self.assertEqual(rep["verdict"], "DISSOLVES")

    def test_smooth_drift_persists_and_continuity_decides_nothing(self):
        fam = tr.MatrixFamily(lambda s: np.diag([1.0, 2.0]))
        sh = lambda S, c: 0.95 - 0.35 * S.s
        pi = lambda S, c: (sh(S, c), 1 - sh(S, c))
        rep = tr.run_case(fam, [0.0, 1.0], sh, pi_fn=pi)
        self.assertEqual(rep["verdict"], "PERSISTS")
        self.assertEqual(rep["steps"][0]["continuity"]["status"], "resolved")
        self.assertLessEqual(rep["steps"][0]["continuity"]["max_jump"], tr.JUMP)

    def test_starting_identity_must_resolve(self):
        fam = tr.MatrixFamily(lambda s: np.diag([1.0, 1.0 + 1e-6]), uncert=1e-5)
        self.assertEqual(tr.run_case(fam, [0.0, 1.0], lambda S, c: 0.9)["verdict"], "OPEN")


# ---------------------------------------------------------------- mutants
def source_mutant(func_name, old, new):
    def apply():
        f = getattr(tr, func_name)
        src = textwrap.dedent(inspect.getsource(f))
        if src.count(old) != 1:
            raise AssertionError(f"mutant anchor not unique in {func_name}: {old!r}")
        exec(src.replace(old, new), tr.__dict__)
        return lambda: setattr(tr, func_name, f)
    return apply


def const_mutant(name, value):
    def apply():
        old = getattr(tr, name)
        setattr(tr, name, value)
        return lambda: setattr(tr, name, old)
    return apply


MUTANTS = {
    "selection_changed counted as a switch": source_mutant(
        "run_case", 'c = end[newb] if kind == "VERIFIED" else end["carrier"]',
        'c = end[newb] if kind == "VERIFIED" else end["carrier"]\n'
        '        if selector_fn is not None and selector_fn(B) != c:\n'
        '            c, kind = selector_fn(B), "VERIFIED"'),
    "the mandatory midpoint skipped when the endpoint overlap is high": source_mutant(
        "track_step", "    direct, ok_d, _ = match(family, A, B, branches)",
        "    direct, ok_d, _t = match(family, A, B, branches)\n"
        "    if ok_d and min(v[0][0] for v in _t.values()) >= 0.99:\n"
        "        return Step('RESOLVED', [(A, dict(branches)), (B, direct)], coarse=dict(direct))"),
    "a rejected coarse match retained for the energy-order reversal": source_mutant(
        "run_case", "kind, newb, crec = verify_crossing(step)",
        "step = Step('RESOLVED', [(A, branches), (B, step.coarse)], coarse=step.coarse)\n"
        "        kind, newb, crec = verify_crossing(step)"),
    "two earlier branches assigned to the same later state": source_mutant(
        "match", "    if len(set(out.values())) != len(out):", "    if False:"),
    "closeness within 1e-4 used as sufficient crossing evidence": source_mutant(
        "verify_crossing", "    if st0 is True and st1 is True:",
        "    for _b in others:\n"
        "        if any(abs(signed_sep(S, a[carrier], a[_b])[0]) <= INSPECT for S, a in path):\n"
        "            return 'VERIFIED', _b, record\n"
        "    if st0 is True and st1 is True:"),
    "an unresolved possible crossing converted into a verified one": source_mutant(
        "verify_crossing", '    return "POSSIBLE", None, record',
        '    return ("VERIFIED", others[0], record) if others else ("POSSIBLE", None, record)'),
    "the crossing verified by the energy condition alone": source_mutant(
        "verify_crossing", "if bracket and st0 is True and st1 is False and b_low is True:", "if bracket:"),
    "the crossing verified by the subspace condition alone": source_mutant(
        "verify_crossing", "if bracket and st0 is True and st1 is False and b_low is True:",
        "if st0 is True and st1 is not True:"),
    "the handover forgotten (the old since-the-previous-point rule)": source_mutant(
        "run_case", '        if state == "HELD":\n            state = apply_rules(state, kind, sh)',
        '        state = apply_rules("HELD", kind, sh)'),
    "the own-share rule applied before a resolved crossing": source_mutant(
        "apply_rules", '    if crossing == "VERIFIED":\n        return "HANDED"\n    if share >= OWN_SHARE_MIN:\n        return "HELD"',
        '    if share >= OWN_SHARE_MIN:\n        return "HELD"\n    if crossing == "VERIFIED":\n        return "HANDED"'),
    "the inter-sector switch granting HANDS OVER": source_mutant(
        "run_case", '            rep["gap_handover"].append((A.s, B.s))',
        '            rep["gap_handover"].append((A.s, B.s)); state = "HANDED" if state == "HELD" else state'),
    "the refinement depth cut to one level": const_mutant("MAX_DEPTH", 1),
    "an unresolved step read as DISSOLVES": source_mutant(
        "run_case", '            rep["verdict"] = "OPEN"; rep["reason"] = step.reason',
        '            rep["verdict"] = "DISSOLVES"; rep["reason"] = step.reason'),
    "continuity used as the failure criterion": source_mutant(
        "run_case", '        srec["continuity"] = continuity(family, step, pi_fn)',
        '        srec["continuity"] = continuity(family, step, pi_fn)\n'
        '        if pi_fn is not None and tv(pi_fn(A, branches["carrier"]), pi_fn(B, step.path[-1][1]["carrier"])) > JUMP:\n'
        '            state = "DISSOLVED"'),
}


def run():
    return unittest.TextTestRunner(verbosity=0).run(unittest.defaultTestLoader.loadTestsFromTestCase(Tracking))


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
