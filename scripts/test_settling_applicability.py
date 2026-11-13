"""Planted-answer tests: adapter scope, not a hardware acceptance protocol."""
import copy
import hashlib
import json
import math
from pathlib import Path
import sys
import unittest
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'e2e/probes'))
import settling_applicability as adapter


RAW = b'planted scalar RC observations, SI, no physical calibration claim'


def fixture():
    native = dict(settling_bound_s=math.log(10), meets_deadline=True, readout_equilibrium_v=0.,
        node_order=['cap'], units='SI', source=dict(revision=1, nodes=[dict(id='cap', capacitance_f=1., voltage_v=1.)]),
        inputs=dict(tolerance_v=.1, configuration=dict(edges=[], ports=[dict(node='cap', conductance_s=1., voltage_v=0.)]),
                    readout={'cap':1.}))
    return dict(source=dict(id='planted', revision='1', sha256=hashlib.sha256(RAW).hexdigest(),
                            parser='planted-array-fixture-v1'),
                units=dict(time='s', voltage='V', resistance='ohm', capacitance='F'),
                circuit=dict(resistance_ohm=1., capacitance_f=1.),
                initial=dict(voltage_v=1., origin='declared_fixture'),
                schedule=[dict(start_s=0., end_s=5., source_v=0.)],
                horizon_s=[0., 5.], target_v=0., tolerance_v=.1,
                times_s=[0., 1., 2., 3., 3.5, 4., 4.5, 5.],
                source_v=[0.] * 8,
                output_v=[math.exp(-t) for t in [0., 1., 2., 3., 3.5, 4., 4.5, 5.]],
                model=dict(native_result=native,
                           receipt=dict(id='native-fixture', revision='1', sha256=hashlib.sha256(
                               json.dumps(native, sort_keys=True, separators=(',', ':')).encode()).hexdigest())),
                evidence={})


def evidence(req, bound=0., voltage_uncertainty=0.):
    blobs = {}
    for role, extra in [
        ('voltage_calibration', dict(unit='V', absolute_uncertainty=voltage_uncertainty)),
        ('timebase_calibration', dict(unit='s', absolute_uncertainty=0.)),
        ('forcing', dict(unit='V', bound_v=bound, target_v=req['target_v'],
                         basis='independent_characterization')),
    ]:
        doc = dict(id=role, revision='1', source_id=req['source']['id'],
                   role=role, horizon_s=list(req['horizon_s']), **extra)
        raw = json.dumps(doc, sort_keys=True).encode()
        digest = hashlib.sha256(raw).hexdigest()
        blobs[digest] = raw
        req['evidence'][role] = dict(id=role, revision='1', sha256=digest)
    return blobs


def alter_document(req, blobs, role, **changes):
    ref = req['evidence'][role]
    doc = json.loads(blobs[ref['sha256']])
    doc.update(changes)
    raw = json.dumps(doc, sort_keys=True).encode()
    ref['sha256'] = hashlib.sha256(raw).hexdigest()
    ref['revision'] = doc['revision']
    blobs[ref['sha256']] = raw


def rehash_model(req):
    raw = json.dumps(req['model']['native_result'], sort_keys=True, separators=(',', ':'), allow_nan=False).encode()
    req['model']['receipt']['sha256'] = hashlib.sha256(raw).hexdigest()


class ApplicabilityTests(unittest.TestCase):
    def test_sa00_missing_evidence_preserves_native_result(self):
        req = fixture()
        result = adapter.assess(req, RAW)
        self.assertEqual(result['model_calculation']['native_result'], req['model']['native_result'])
        self.assertEqual(result['forcing_applicability']['status'], 'not_established')
        self.assertEqual(result['hardware_qualification'], 'not_qualified')
        self.assertIn('voltage_calibration', ' '.join(result['next_evidence']))

    def test_sa01_constant_control_is_scoped_not_certified(self):
        req = fixture()
        result = adapter.assess(req, RAW, evidence(req))
        self.assertEqual(result['forcing_applicability']['status'], 'supported_over_declared_horizon')
        self.assertEqual(result['forcing_applicability']['horizon_s'], [0., 5.])
        self.assertEqual(result['observed_window']['status'], 'within_tolerance')
        self.assertEqual(result['observed_window']['samples_after_bound'], 5)
        self.assertEqual(result['observed_window']['observed_duration_s'], 2.)
        self.assertEqual(result['hardware_qualification'], 'not_qualified')

    def test_sa02_small_forcing_invalidates_old_deadline(self):
        req = fixture()
        t0 = math.log(10)
        req['times_s'] = [t0 + i * .02 for i in range(5)]
        req['source_v'] = [.05] * 5
        req['output_v'] = [.05 + .95 * math.exp(-t) for t in req['times_s']]
        self.assertAlmostEqual(req['output_v'][0], .145, places=14)
        self.assertLess(.05, req['tolerance_v'])
        result = adapter.assess(req, RAW, evidence(req, bound=.05))
        self.assertEqual(result['forcing_applicability']['status'], 'contradicted')
        self.assertEqual(result['observed_window']['status'], 'observed_departure')
        self.assertEqual(result['model_calculation']['native_result']['settling_bound_s'], t0)

    def test_sa03_crossing_then_departure(self):
        req = fixture()
        req['output_v'][-1] = .2
        result = adapter.assess(req, RAW)
        self.assertEqual(result['observed_window']['status'], 'observed_departure')
        self.assertEqual(result['observed_window']['max_recorded_error_v'], .2)

    def test_sa04_observed_prefix_cannot_prove_future(self):
        req = fixture()
        req['horizon_s'] = [0., 100.]
        result = adapter.assess(req, RAW)
        self.assertEqual(result['forcing_applicability']['status'], 'not_established')
        self.assertEqual(result['observed_window']['last_sample_s'], 5.)

    def test_sa04_short_certificate_cannot_cover_future(self):
        req = fixture()
        blobs = evidence(req)
        req['horizon_s'] = [0., 10.]
        req['schedule'][0]['end_s'] = 10.
        self.assertEqual(adapter.assess(req, RAW, blobs)['forcing_applicability']['status'], 'not_established')

    def test_sa04_nonzero_bound_is_not_fixed_source_evidence(self):
        req = fixture()
        result = adapter.assess(req, RAW, evidence(req, bound=.001))
        self.assertEqual(result['forcing_applicability']['status'], 'not_established')

    def test_sa05_missing_calibration_does_not_mean_zero_uncertainty(self):
        req = fixture()
        req['source_v'] = [10.] * 8
        result = adapter.assess(req, RAW)
        self.assertEqual(result['forcing_applicability']['status'], 'not_established')
        self.assertIsNone(result['uncertainty'].get('voltage_v'))
        self.assertEqual(result['forcing_applicability']['max_recorded_departure_v'], 10.)

    def test_sa05_expired_calibration_is_unresolved(self):
        req = fixture()
        blobs = evidence(req)
        alter_document(req, blobs, 'voltage_calibration', horizon_s=[-2., -1.])
        result = adapter.assess(req, RAW, blobs)
        self.assertEqual(result['forcing_applicability']['status'], 'not_established')
        self.assertIsNone(result['uncertainty'].get('voltage_v'))

    def test_sa05_calibration_for_another_source_is_unresolved(self):
        req = fixture()
        blobs = evidence(req)
        alter_document(req, blobs, 'timebase_calibration', source_id='other_sensor')
        result = adapter.assess(req, RAW, blobs)
        self.assertEqual(result['forcing_applicability']['status'], 'not_established')
        self.assertIsNone(result['uncertainty'].get('time_s'))

    def test_sa05_uncertainty_interval_can_include_target(self):
        req = fixture()
        req['source_v'] = [.02] * 8
        result = adapter.assess(req, RAW, evidence(req, voltage_uncertainty=.03))
        self.assertEqual(result['forcing_applicability']['status'], 'supported_over_declared_horizon')
        self.assertEqual(result['uncertainty']['voltage_v'], .03)

    def test_sa06_four_samples_insufficient(self):
        req = fixture()
        for key in ('times_s', 'source_v', 'output_v'):
            req[key] = req[key][:-1]
        self.assertEqual(adapter.assess(req, RAW)['observed_window']['status'], 'insufficient_followup')

    def test_sa06_no_samples_after_bound(self):
        req = fixture()
        req['model']['native_result']['settling_bound_s'] = 100.
        rehash_model(req)
        result = adapter.assess(req, RAW)
        self.assertEqual(result['observed_window']['status'], 'insufficient_followup')
        self.assertEqual(result['observed_window']['samples_after_bound'], 0)
        self.assertIsNone(result['observed_window']['observed_duration_s'])

    def test_sa09_duplicate_times_refuse(self):
        req = fixture()
        req['times_s'][1] = 0.
        with self.assertRaisesRegex(ValueError, 'strictly increasing'):
            adapter.assess(req, RAW)

    def test_sa09_reversed_times_refuse(self):
        req = fixture()
        req['times_s'].reverse()
        with self.assertRaisesRegex(ValueError, 'strictly increasing'):
            adapter.assess(req, RAW)

    def test_sa09_nonfinite_refuses(self):
        req = fixture()
        req['source_v'][1] = float('nan')
        with self.assertRaisesRegex(ValueError, 'finite'):
            adapter.assess(req, RAW)

    def test_sa09_source_bytes_must_match_digest(self):
        with self.assertRaisesRegex(ValueError, 'source.*digest'):
            adapter.assess(fixture(), RAW + b'changed')

    def test_sa09_evidence_bytes_must_match_digest(self):
        req = fixture()
        blobs = evidence(req)
        blobs[req['evidence']['forcing']['sha256']] += b' '
        with self.assertRaisesRegex(ValueError, 'evidence.*digest'):
            adapter.assess(req, RAW, blobs)

    def test_sa09_evidence_revision_changes_identity(self):
        req = fixture()
        blobs = evidence(req)
        first = adapter.assess(req, RAW, blobs)['assessment_id']
        alter_document(req, blobs, 'forcing', revision='2')
        self.assertNotEqual(first, adapter.assess(req, RAW, blobs)['assessment_id'])

    def test_sa09_schedule_changes_identity(self):
        req = fixture()
        first = adapter.assess(req, RAW)['assessment_id']
        req['schedule'][0]['source_v'] = .01
        self.assertNotEqual(first, adapter.assess(req, RAW)['assessment_id'])

    def test_sa09_source_revision_changes_identity(self):
        req = fixture()
        first = adapter.assess(req, RAW)['assessment_id']
        req['source']['revision'] = '2'
        self.assertNotEqual(first, adapter.assess(req, RAW)['assessment_id'])

    def test_sa09_units_required(self):
        req = fixture()
        req['units']['time'] = 'ms'
        with self.assertRaisesRegex(ValueError, 'units'):
            adapter.assess(req, RAW)

    def test_sa09_hardware_flags_are_not_accepted(self):
        req = fixture()
        req['legacy_metadata'] = {k: True for k in ('voltage_calibration', 'timebase_calibration',
            'component_measurements', 'switch_characterization', 'uncertainty_budget', 'held_out_protocol')}
        with self.assertRaisesRegex(ValueError, 'unknown.*legacy_metadata'):
            adapter.assess(req, RAW)

    def test_sa09_read_only_and_deterministic(self):
        req = fixture()
        blobs = evidence(req)
        before = copy.deepcopy((req, blobs))
        first = adapter.assess(req, RAW, blobs)
        self.assertEqual(first, adapter.assess(req, RAW, blobs))
        self.assertEqual((req, blobs), before)
        self.assertEqual(len(first['assessment_id']), 64)

    def test_sa11_published_contract_names_three_scopes(self):
        result = adapter.assess(fixture(), RAW)
        self.assertIn('scope', result['model_calculation'])
        self.assertIn('Conditional fixed-source', result['model_calculation'].get('scope', ''))
        self.assertEqual(result['observed_window']['comparison_policy'], 'recorded_values_at_nominal_timestamps')
        self.assertEqual(result['hardware_qualification'], 'not_qualified')

    def test_sa09_modified_native_result_refuses(self):
        req = fixture()
        req['model']['native_result']['settling_bound_s'] = 0.
        with self.assertRaisesRegex(ValueError, 'model.*digest'):
            adapter.assess(req, RAW)

    def test_sa00_wrong_target_refuses(self):
        req = fixture()
        req['target_v'] = 1.
        with self.assertRaisesRegex(ValueError, 'target.*equilibrium'):
            adapter.assess(req, RAW)

    def test_sa00_schedule_must_support_declared_fixed_source(self):
        req = fixture()
        req['schedule'][0]['source_v'] = 1.
        result = adapter.assess(req, RAW, evidence(req))
        self.assertEqual(result['forcing_applicability']['status'], 'contradicted')

    def test_sa00_schedule_must_cover_entire_horizon(self):
        req = fixture()
        req['schedule'][0]['end_s'] = 3.
        result = adapter.assess(req, RAW, evidence(req))
        self.assertEqual(result['forcing_applicability']['status'], 'not_established')

    def test_sa05_boundary_sample_with_uncertain_clock_is_not_contradiction(self):
        req = fixture()
        blobs = evidence(req)
        req['source_v'][0] = 1.
        alter_document(req, blobs, 'timebase_calibration', absolute_uncertainty=.1)
        self.assertEqual(adapter.assess(req, RAW, blobs)['forcing_applicability']['status'], 'supported_over_declared_horizon')

    def test_sa07_replay_uses_declared_first_state_not_measured_resets(self):
        req = fixture()
        req['output_v'] = [999.] * len(req['times_s'])
        result = adapter.diagnostic_replay(req, RAW, 'linear')
        np.testing.assert_allclose(result['predicted_v'], np.exp(-np.array(req['times_s'])), atol=1e-13)
        self.assertIn('not native', result['implementation'])

    def test_sa08_enclosure_refuses_unknown_initial_state(self):
        req = fixture()
        req['initial']['origin'] = 'unknown'
        with self.assertRaisesRegex(ValueError, 'initial.origin'):
            adapter.diagnostic_replay(req, RAW, 'left', [.855, 1.155])

    def test_sa08_enclosure_refuses_wrong_units(self):
        req = fixture()
        req['units']['voltage'] = 'mV'
        with self.assertRaisesRegex(ValueError, 'units'):
            adapter.diagnostic_replay(req, RAW, 'left', [.855, 1.155])

    def test_sa08_enclosure_discloses_identifiability(self):
        req = fixture()
        result = adapter.diagnostic_replay(req, RAW, 'linear', [.855, 1.155])
        bounds = result['enclosure']
        self.assertIn('not separately identifiable', bounds['scope'])
        np.testing.assert_allclose(bounds['low_v'], np.exp(-np.array(req['times_s'])/.855), atol=1e-13)
        np.testing.assert_allclose(bounds['high_v'], np.exp(-np.array(req['times_s'])/1.155), atol=1e-13)

    def test_sa09_native_tolerance_cannot_be_relabelled(self):
        req = fixture()
        req['tolerance_v'] = 1.
        with self.assertRaisesRegex(ValueError, 'tolerance_v.*native'):
            adapter.assess(req, RAW)

    def test_sa09_components_must_match_native_model(self):
        req = fixture()
        req['circuit']['capacitance_f'] = .5
        with self.assertRaisesRegex(ValueError, 'circuit.*native'):
            adapter.assess(req, RAW)

    def test_sa09_multinode_is_not_scalar_applicability(self):
        req = fixture()
        req['model']['native_result']['node_order'].append('other')
        rehash_model(req)
        with self.assertRaisesRegex(ValueError, 'one-node'):
            adapter.assess(req, RAW)

    def test_sa09_initial_origin_must_match_measurement(self):
        req = fixture()
        req['initial'] = dict(voltage_v=999., origin='first_measured_output')
        with self.assertRaisesRegex(ValueError, 'initial.*first'):
            adapter.assess(req, RAW)


if __name__ == '__main__':
    unittest.main()
