"""Fail-closed and solver controls for the RF2E experiment."""
import copy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import numpy as np
from scipy.sparse import eye
from ricci_curvature_reconstruction import checked_solve
from ricci_reconstruction_evidence import validate_structure, verify


def valid_structure():
    return {'schema': 'rf2e_abelian_v1', 'status': 'COMPLETE_UNVERIFIED',
            'cases': {f'level{i}_q{q}': {} for i in range(4) for q in [4, 6]},
            'berger': 'NOT_RUN', 'release_gate': 'OPEN', 'full_regressions': 'DEFERRED',
            'manifest_sha256': 'a'*64, 'numerical_gate': {}, 'elapsed_seconds': 1.}


class GuardTests(unittest.TestCase):
    def test_complete_structure(self):
        validate_structure(valid_structure())

    def test_incomplete_and_forged_claims(self):
        good = valid_structure()
        bad = []
        for key, value in [('schema', 'made_up'), ('status', 'FAILED'), ('berger', 'PASS'),
                           ('release_gate', 'CLOSED'), ('full_regressions', 'PASS')]:
            changed = copy.deepcopy(good); changed[key] = value; bad.append(changed)
        changed = copy.deepcopy(good); del changed['cases']['level3_q6']; bad.append(changed)
        changed = copy.deepcopy(good); changed['cases']['extra'] = {}; bad.append(changed)
        for changed in bad:
            with self.subTest(changed=changed):
                with self.assertRaises(ValueError): validate_structure(changed)

    def test_stale_verified_and_pass_are_removed_even_without_input(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for name in ['gate.PASS.json', 'VERIFIED_DIAGNOSTIC.json']:
                (root/name).write_text('{"pass":true}')
            with self.assertRaises((ValueError, FileNotFoundError)): verify(root)
            self.assertFalse((root/'gate.PASS.json').exists())
            self.assertFalse((root/'VERIFIED_DIAGNOSTIC.json').exists())

    def test_cg_success_flag_without_true_solution_refuses(self):
        with patch('ricci_curvature_reconstruction.cg', return_value=(np.zeros(3), 0)):
            with self.assertRaises(ValueError): checked_solve(eye(3).tocsr(), np.ones(3))

    def test_cg_failure_refuses_even_accurate_iterate(self):
        with patch('ricci_curvature_reconstruction.cg', return_value=(np.ones(3), 1)):
            with self.assertRaises(ValueError): checked_solve(eye(3).tocsr(), np.ones(3))


if __name__ == '__main__': unittest.main()
