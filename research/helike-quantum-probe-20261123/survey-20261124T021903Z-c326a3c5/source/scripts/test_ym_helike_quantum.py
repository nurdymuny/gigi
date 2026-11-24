"""Focused gates for the registered Helike quantum probe; no engine tests."""

import unittest
import numpy as np
import scipy.linalg as la
import ym_helike_quantum as q


class BasisTests(unittest.TestCase):
    def test_all_invariants_and_determinant_sector(self):
        b = q.build_basis(4)
        self.assertEqual(b["C"].shape[1], 29)
        self.assertEqual(dict(zip(*np.unique(b["degrees"], return_counts=True))),
                         {0: 1, 2: 6, 3: 1, 4: 21})
        np.testing.assert_allclose(np.diag(b["S"]), 1, atol=1e-14)
        self.assertGreater(la.eigvalsh(b["S"])[0], 0)

    def test_oscillator_physical_gap_and_degeneracy(self):
        b = q.build_basis(4)
        ops = q.build_operators(b)
        e, v, _ = q.solve_generalized(ops["H0"], b["S"], "cpu")
        np.testing.assert_allclose(e[:7], [4.5] + [6.5]*6, atol=1e-11)
        for channel in (ops["tensor"], [ops["scalar"]]):
            report = q.probe_report(e, v, channel)
            self.assertEqual(report["status"], "DETECTED")
            self.assertAlmostEqual(report["accessible_gap"], 2, places=10)
            self.assertGreater(report["lowest_weight"], 0)

    def test_independent_gauss_kernel_and_operator(self):
        b = q.build_basis(4)
        ops = q.build_operators(b)
        ref = q.independent_reference(4, 1)
        self.assertEqual(ref["dimension"], 29)
        self.assertLess(ref["gauss_residual"], 1e-10)
        h = ops["H0"] - 3*ops["D"] + ops["V4"]
        e, _, _ = q.solve_generalized(h, b["S"], "cpu")
        np.testing.assert_allclose(e, ref["energies"], rtol=1e-10, atol=1e-10)
        self.assertLess(q.compare_reference(b, ops, ref), 1e-10)

    def test_rotation_never_demotes_to_float32(self):
        b = q.build_basis(6)
        for axis in range(3):
            rotation = q.rotation(b, axis)
            self.assertEqual(rotation.dtype, np.dtype("float64"))
            self.assertLess(np.linalg.norm((rotation @ b["C"]).toarray()), 1e-10)

    def test_missing_cubic_and_early_projection_are_rejected(self):
        b = q.build_basis(4)
        ops = q.build_operators(b)
        ref = q.independent_reference(4, 1)
        no_cubic = dict(ops, D=np.zeros_like(ops["D"]))
        self.assertGreater(q.compare_reference(b, no_cubic, ref), 1e-3)
        wrong_scalar = sum(o @ la.solve(b["S"], o, assume_a="pos") for o in ops["tensor"])
        self.assertGreater(q.compare_reference(b, dict(ops, scalar=wrong_scalar), ref), 1e-3)


class SolverTests(unittest.TestCase):
    def test_complex_generalized_metric_cuda(self):
        s = np.array([[4., 1j], [-1j, 2.]], dtype=np.complex128)
        l = la.cholesky(s, lower=True)
        h = l @ np.diag([2., 7.]) @ l.conj().T
        e, v, _ = q.solve_generalized(h, s, "cuda")
        np.testing.assert_allclose(e, [2., 7.], atol=1e-11)
        np.testing.assert_allclose(v.conj().T@s@v, np.eye(2), atol=1e-11)

    def test_generalized_nonidentity_metric_cuda(self):
        s = np.array([[4., 1.], [1., 2.]])
        l = la.cholesky(s, lower=True)
        h = l @ np.diag([2., 7.]) @ l.T
        e, v, diag = q.solve_generalized(h, s, "cuda")
        np.testing.assert_allclose(e, [2, 7], atol=1e-11)
        np.testing.assert_allclose(v.T @ s @ v, np.eye(2), atol=1e-11)
        self.assertEqual(diag["device"], "cuda:0")
        self.assertLess(diag["residual"], 1e-10)

    def test_precision_and_bad_metric_refused(self):
        with self.assertRaisesRegex(ValueError, "float64"):
            q.solve_generalized(np.eye(2, dtype=np.float32), np.eye(2), "cuda")
        with self.assertRaisesRegex(ValueError, "positive|Cholesky"):
            q.solve_generalized(np.eye(2), np.diag([1., -1.]), "cuda")

    def test_nonhermitian_and_nonfinite_refused(self):
        with self.assertRaisesRegex(ValueError, "Hermitian"):
            q.solve_generalized(np.array([[1., 2.], [0., 1.]]), np.eye(2), "cuda")
        with self.assertRaisesRegex(ValueError, "finite"):
            q.solve_generalized(np.diag([1., np.nan]), np.eye(2), "cuda")


class ProbeTests(unittest.TestCase):
    def test_diagnostics_include_a_probe_level_above_the_first_three(self):
        low, high = np.zeros((5, 5)), np.zeros((5, 5))
        low[0, 1] = low[1, 0] = 1
        high[0, 4] = high[4, 0] = 1
        groups = q.reported_groups(np.arange(5.), np.eye(5), [[low], [high]])
        self.assertEqual([g.tolist() for g in groups], [[0], [1], [4]])
    def test_lower_invisible_level_is_not_relabelled(self):
        o = np.array([[0., 0., 1.], [0., 0., 0.], [1., 0., 0.]])
        report = q.probe_report(np.array([0., 1., 3.]), np.eye(3), [o])
        self.assertEqual(report["status"], "HIGHER_LEVEL_ONLY")
        self.assertEqual(report["absolute_gap"], 1)
        self.assertEqual(report["accessible_gap"], 3)
        self.assertEqual(report["lowest_weight"], 0)

    def test_constant_probe_does_not_normalize_vacuum(self):
        report = q.probe_report(np.array([0., 1., 3.]), np.eye(3), [np.eye(3)])
        self.assertEqual(report["status"], "ZERO_EXCITED_NORM")
        self.assertIsNone(report["accessible_gap"])

    def test_degenerate_level_uses_whole_projector(self):
        o = np.array([[0., 1., 2.], [1., 0., 0.], [2., 0., 0.]])
        e = np.array([0., 2., 2.])
        v = np.eye(3)
        v[1:, 1:] = np.array([[1., -1.], [1., 1.]]) / np.sqrt(2)
        first = q.probe_report(e, np.eye(3), [o])
        second = q.probe_report(e, v, [o])
        self.assertAlmostEqual(first["lowest_weight"], second["lowest_weight"], places=13)
        self.assertEqual(first["levels"][0]["multiplicity"], 2)
        self.assertAlmostEqual(first["lowest_weight"], 1, places=13)

    def test_exterior_action_keeps_excitation_above_cutoff(self):
        b = q.build_basis(0)
        codes = q.oscillator_codes(2)
        action = q.expanded_action(b, np.ones((1, 1)), [(1., (0, 0))], codes)
        self.assertAlmostEqual(float(action[np.searchsorted(codes, 0), 0]), 0.5)
        self.assertAlmostEqual(float(action[np.searchsorted(codes, 2), 0]), 1/np.sqrt(2))
        self.assertAlmostEqual(float(np.sum(action**2)), 0.75)

    def test_gpu_propagator_matches_planted_spectrum(self):
        h = np.diag([0., 2., 3.])
        o = np.array([[0., 1., 2.], [1., 0., 0.], [2., 0., 0.]])
        report = q.propagator_check(h, np.eye(3), np.array([0., 2., 3.]), np.eye(3), [o])
        self.assertLess(report["max_error"], 1e-10)
        self.assertEqual(report["device"], "cuda:0")


class ReceiptTests(unittest.TestCase):
    def evidence_fixture(self):
        h = np.diag([0., 2., 3.])
        probe = np.array([[0., 1., 2.], [1., 0., 0.], [2., 0., 0.]])
        arrays = {"energies": np.diag(h), "vectors": np.eye(3), "H": h, "S": np.eye(3),
                  "tensor": np.array([probe]), "scalar": probe}
        report = q.probe_report(np.diag(h), np.eye(3), [probe])
        return {"E0": 0., "E1": 2., "absolute_gap": 2., "H0": dict(report), "H2": dict(report)}, arrays

    def test_evidence_replay_rejects_changed_gap_and_weight(self):
        import copy
        from verify_ym_helike_evidence import check_case
        case, arrays = self.evidence_fixture()
        check_case(case, arrays)
        altered = copy.deepcopy(case)
        altered["absolute_gap"] = 1.5
        with self.assertRaisesRegex(ValueError, "absolute_gap"):
            check_case(altered, arrays)
        altered = copy.deepcopy(case)
        altered["H2"]["lowest_weight"] = 0.99
        with self.assertRaisesRegex(ValueError, "H2 lowest_weight"):
            check_case(altered, arrays)

    def test_evidence_replay_rejects_nonfinite_claims(self):
        import copy
        from verify_ym_helike_evidence import check_case
        case, arrays = self.evidence_fixture()
        for value in (float("nan"), float("inf"), -float("inf")):
            altered = copy.deepcopy(case)
            altered["E0"] = value
            with self.assertRaisesRegex(ValueError, "finite"):
                check_case(altered, arrays)
            altered = copy.deepcopy(case)
            altered["H0"]["lowest_weight"] = value
            with self.assertRaisesRegex(ValueError, "finite"):
                check_case(altered, arrays)

    def test_stale_and_missing_gates_cannot_pass(self):
        good = {"status": "PASS", "run_id": "new", "source_hashes": {"source": "abc"},
                "required_gates": ["basis", "metric"], "gates": {"basis": True, "metric": True}}
        self.assertTrue(q.validate_receipt(good, "new", {"source": "abc"}))
        for bad in (dict(good, status="RUNNING"), dict(good, run_id="old"),
                    dict(good, gates={"basis": True}), dict(good, source_hashes={"source": "bad"}),
                    dict(good, required_gates=["basis"], gates={"basis": True}),
                    dict(good, gates={"basis": True, "metric": False})):
            self.assertFalse(q.validate_receipt(bad, "new", {"source": "abc"}))

    def test_stable_gap_cannot_hide_drifting_energies(self):
        import copy
        case = {"E0": 5., "E1": 7., "absolute_gap": 2.,
                "H0": {"accessible_gap": 2., "lowest_weight": 0.4},
                "H2": {"accessible_gap": 3., "lowest_weight": 0.},
                "exterior": {"residual_gate": True, "tail_gate": True}}
        history = [copy.deepcopy(case) for _ in range(3)]
        self.assertTrue(q.convergence(history))
        history[0]["exterior"]["residual_gate"] = False
        self.assertTrue(q.convergence(history))
        history[-1]["E0"] += 0.1
        history[-1]["E1"] += 0.1
        self.assertFalse(q.convergence(history))

    def test_gpu_krylov_counts_work_and_recovers_planted_level(self):
        result = q.gpu_krylov(np.diag([0., 2., 3.]), np.array([[0.], [1.], [1.]]),
                              np.array([1., 0., 0.]), budgets=(1, 2))
        self.assertEqual(result["checkpoints"][-1]["matvecs"], 2)
        self.assertAlmostEqual(result["checkpoints"][-1]["gap"], 2, places=12)
        self.assertLess(result["checkpoints"][-1]["residual"], 1e-10)


if __name__ == "__main__":
    unittest.main(verbosity=2)
