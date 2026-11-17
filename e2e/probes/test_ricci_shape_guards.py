import copy
import io
import unittest
from unittest.mock import patch
from ricci_shape_experiment import make_manifest,check_manifest
from ricci_shape_regular_mesh import mesh
from ricci_shape_certificate import certify


class FrozenTests(unittest.TestCase):
    def test_sources_binary_and_thresholds_cannot_drift(self):
        original=make_manifest();check_manifest(original)
        for attack in ['threshold','time','binary','source','missing_source','old_evidence','resolution']:
            with self.subTest(attack=attack):
                m=copy.deepcopy(original)
                if attack=='threshold':m['contract']['maximum_final_relative_error']=.5
                elif attack=='time':m['contract']['time_scale']=1.2
                elif attack=='binary':m['binary']['sha256']='0'*64
                elif attack=='source':m['frozen_sources'][next(iter(m['frozen_sources']))]='0'*64
                elif attack=='missing_source':m['frozen_sources'].pop(next(iter(m['frozen_sources'])))
                elif attack=='old_evidence':m['baseline'][next(iter(m['baseline']))]='0'*64
                elif attack=='resolution':m['contract']['resolutions']=[1,2,3,6]
                with self.assertRaises(ValueError):check_manifest(m)

    def test_reversed_cell_cannot_be_certified(self):
        m=copy.deepcopy(mesh(0));m['tetrahedra'][0,[0,1]]=m['tetrahedra'][0,[1,0]]
        with self.assertRaises(ValueError):certify(m,0)


class RemovalTests(unittest.TestCase):
    def test_removing_oriented_complex_check_makes_gate_red(self):
        from ricci_lattice_mesh import validate
        accepted=validate(mesh(0))
        with patch('ricci_shape_certificate.validate',return_value=accepted):
            suite=unittest.TestSuite([FrozenTests('test_reversed_cell_cannot_be_certified')])
            result=unittest.TextTestRunner(stream=io.StringIO()).run(suite)
        self.assertEqual(len(result.failures),1)
        self.assertFalse(result.wasSuccessful())

    def test_removing_source_guard_makes_gate_red(self):
        with patch(__name__+'.check_manifest',return_value=None):
            suite=unittest.TestSuite([FrozenTests('test_sources_binary_and_thresholds_cannot_drift')])
            result=unittest.TextTestRunner(stream=io.StringIO()).run(suite)
        self.assertEqual(len(result.failures),7)
        self.assertFalse(result.wasSuccessful())


if __name__=='__main__':unittest.main()
