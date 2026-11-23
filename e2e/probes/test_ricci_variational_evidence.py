import copy
from pathlib import Path
import tempfile
import unittest
from ricci_variational_evidence import validate_structure, verify


def valid():
    return {'schema': 'rf2f_abelian_v1', 'status': 'COMPLETE_UNVERIFIED',
        'release_gate': 'OPEN', 'berger': 'NOT_RUN', 'full_regressions': 'DEFERRED',
        'cases': {f'level{i}_q{q}': {} for i in range(4) for q in [4, 6]},
        'patches': {str(i): {} for i in range(4)}, 'manifest_sha256': 'a'*64,
        'elapsed_seconds': 1., 'numerical_gate': {}}


class EvidenceTests(unittest.TestCase):
    def test_complete_structure(self):
        validate_structure(valid())

    def test_altered_and_incomplete_scope_refuses(self):
        for key, value in [('status','FAILED'), ('release_gate','PASS'), ('berger','PASS'),
                           ('full_regressions','PASS'), ('schema','fake')]:
            value0 = valid(); value0[key] = value
            with self.subTest(key=key):
                with self.assertRaises(ValueError): validate_structure(value0)
        for key in ['cases', 'patches']:
            value0 = valid(); value0[key].pop(next(iter(value0[key])))
            with self.subTest(key=key):
                with self.assertRaises(ValueError): validate_structure(value0)

    def test_no_stale_pass_on_missing_input(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            for name in ['gate.PASS.json', 'VERIFIED_DIAGNOSTIC.json']:
                (directory/name).write_text('{"pass":true}')
            with self.assertRaises((ValueError, FileNotFoundError)): verify(directory)
            self.assertFalse((directory/'gate.PASS.json').exists())
            self.assertFalse((directory/'VERIFIED_DIAGNOSTIC.json').exists())


if __name__ == '__main__': unittest.main()
