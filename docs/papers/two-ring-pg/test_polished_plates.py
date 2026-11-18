"""Focused drawing checks; these do not rerun or certify the research gates."""
import copy
import json
import unittest

import numpy as np

from make_polished_plates import DATA, centre_tet, technical_camera, validate
from transparent_tetra_renderer import composite, geometry_items, render


class TransparencyTests(unittest.TestCase):
    def setUp(self):
        self.colors=np.array([[1.,0.,0.],[0.,0.,1.]],dtype=np.float32)
        self.alpha=np.full((2,1,1),.5,dtype=np.float32)

    def test_red_front_of_blue_matches_independent_over_rule(self):
        depth=np.array([[[2.]],[[1.]]])
        np.testing.assert_allclose(composite(depth,self.alpha,self.colors)[0,0], [.75,.25,.5])

    def test_swapping_physical_depth_changes_the_overlap_color(self):
        depth=np.array([[[1.]],[[2.]]])
        np.testing.assert_allclose(composite(depth,self.alpha,self.colors)[0,0], [.5,.25,.75])

    def test_submission_order_cannot_override_depth(self):
        depth=np.array([[[2.]],[[1.]]])
        expected=composite(depth,self.alpha,self.colors)
        np.testing.assert_array_equal(composite(depth[::-1],self.alpha[::-1],self.colors[::-1]),expected)

    def test_crossing_depths_are_resolved_per_pixel_not_per_object(self):
        depth=np.array([[[2.,1.]],[[1.,2.]]])
        alpha=np.full_like(depth,.5)
        expected=np.array([[[.75,.25,.5],[.5,.25,.75]]])
        np.testing.assert_allclose(composite(depth,alpha,self.colors),expected)

    def test_front_opacity_really_attenuates_rear_layer(self):
        depth=np.array([[[2.]],[[1.]]])
        alpha=np.array([[[1.]],[[.5]]])
        np.testing.assert_allclose(composite(depth,alpha,self.colors)[0,0],[1.,0.,0.])

    def test_duplicate_shared_geometry_does_not_darken_image(self):
        tet={"ring":"K","v":[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}
        cam=technical_camera(np.array(tet["v"]))
        self.assertEqual(len(geometry_items([tet,tet],cam)),10)
        limits=(-1.,1.,-1.,1.)
        np.testing.assert_array_equal(render([tet],cam,limits,width=80),render([tet,tet],cam,limits,width=80))


class MeasurementTests(unittest.TestCase):
    def setUp(self):
        self.geo=json.loads((DATA/"rings_3d.json").read_text())
        self.measure=json.loads((DATA/"cell_views.json").read_text())

    def test_saved_geometry_and_measurements_agree_at_saved_precision(self):
        validate(self.geo,self.measure)

    def test_modified_vertex_is_rejected(self):
        broken=copy.deepcopy(self.geo)
        broken["cells"][0]["tets"][0]["v"][0][0]+=.01
        with self.assertRaises(AssertionError):
            validate(broken,self.measure)

    def test_wrong_measurement_is_rejected(self):
        broken=copy.deepcopy(self.measure)
        broken["chordal_edge_lengths"]["v0v1"]+=.01
        with self.assertRaises(AssertionError):
            validate(self.geo,broken)

    def test_false_selection_count_is_rejected(self):
        broken=copy.deepcopy(self.geo)
        broken["cells"][0]["k"]-=1
        with self.assertRaises(AssertionError):
            validate(broken,self.measure)

    def test_camera_is_rigid_not_a_shape_distortion(self):
        k=centre_tet(self.geo["cells"][0])
        cam=technical_camera(k)
        np.testing.assert_allclose(cam.T@cam,np.eye(3),atol=1e-14)
        self.assertAlmostEqual(np.linalg.det(cam),1.)


if __name__=="__main__":
    unittest.main()
