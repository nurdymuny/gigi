"""Require assertion failures, not import/runtime errors, for removed mechanisms."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import types
import unittest

import test_settling_applicability as tests


def run(module, names=None):
    tests.adapter = module
    names = names or unittest.defaultTestLoader.getTestCaseNames(tests.ApplicabilityTests)
    log = io.StringIO()
    result = unittest.TextTestRunner(stream=log, verbosity=2).run(
        unittest.TestSuite(tests.ApplicabilityTests(n) for n in names))
    return result, log.getvalue()


def load(source, path):
    module = types.ModuleType('applicability_audit_variant')
    module.__file__ = str(path)
    exec(compile(source, str(path), 'exec'), module.__dict__)
    return module


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    original = tests.adapter
    path = Path(original.__file__)
    source = path.read_text(encoding='utf-8')
    report = dict(adapter_sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    try:
        green, log = run(original)
        assert green.wasSuccessful(), log
        report.update(tests=green.testsRun, green=log)
        legacy_path = Path(__file__).parent/'fixtures/settling_legacy_bridge.py'
        legacy = load(legacy_path.read_text(), legacy_path)
        new_tests = {'test_sa09_modified_native_result_refuses', 'test_sa00_wrong_target_refuses',
            'test_sa00_schedule_must_support_declared_fixed_source', 'test_sa00_schedule_must_cover_entire_horizon',
            'test_sa05_boundary_sample_with_uncertain_clock_is_not_contradiction',
            'test_sa07_replay_uses_declared_first_state_not_measured_resets',
            'test_sa08_enclosure_refuses_unknown_initial_state', 'test_sa08_enclosure_refuses_wrong_units',
            'test_sa08_enclosure_discloses_identifiability', 'test_sa09_native_tolerance_cannot_be_relabelled',
            'test_sa09_components_must_match_native_model', 'test_sa09_multinode_is_not_scalar_applicability',
            'test_sa09_initial_origin_must_match_measurement'}
        baseline_names = [n for n in unittest.defaultTestLoader.getTestCaseNames(tests.ApplicabilityTests) if n not in new_tests]
        red, log = run(legacy, baseline_names)
        assert len(red.failures) == len(baseline_names) and not red.errors, log
        report['legacy_contract_red'] = dict(assertion_failures=len(red.failures), errors=len(red.errors), log=log,
            scope='Schema bridge: None denotes absent old output, not a historical positive claim.',
            bridge_sha256=hashlib.sha256(legacy_path.read_bytes()).hexdigest())
        changes = [
            ('suffix_check', 'all(e <= tolerance for e in errors)', 'errors[0] <= tolerance', 'test_sa03_crossing_then_departure'),
            ('followup_minimum', "observation_status = ('insufficient_followup' if count < 5 else", "observation_status = ('insufficient_followup' if count < 4 else", 'test_sa06_four_samples_insufficient'),
            ('source_digest', "digest(source_bytes) != request['source']['sha256']", 'False', 'test_sa09_source_bytes_must_match_digest'),
            ('evidence_digest', "digest(raw) != ref['sha256']", 'False', 'test_sa09_evidence_bytes_must_match_digest'),
            ('native_digest', "digest(canonical_bytes(native)) != request['model']['receipt']['sha256']", 'False', 'test_sa09_modified_native_result_refuses'),
            ('target_alignment', 'if not math.isclose(request[\'target_v\'], equilibrium, rel_tol=1e-12, abs_tol=1e-12):', 'if False:', 'test_sa00_wrong_target_refuses'),
            ('clock_order', 'if any(b <= a for a, b in zip(arrays[0], arrays[0][1:])):', 'if False:', 'test_sa09_duplicate_times_refuse'),
            ('units', "if request['units'] != SI:", 'if False:', 'test_sa09_units_required'),
            ('schedule_conflict', 'if schedule_contradicts:', 'if False:', 'test_sa00_schedule_must_support_declared_fixed_source'),
            ('schedule_horizon', 'elif not schedule_covers:', 'elif False:', 'test_sa00_schedule_must_cover_entire_horizon'),
            ('calibration_scope', "or a > request['horizon_s'][0] or b < request['horizon_s'][1]", 'or False', 'test_sa05_expired_calibration_is_unresolved'),
            ('calibration_source', "doc.get('source_id') != request['source']['id']", 'False', 'test_sa05_calibration_for_another_source_is_unresolved'),
            ('zero_forcing_contract', "forcing['bound_v'] == 0.", "forcing['bound_v'] <= tolerance", 'test_sa04_nonzero_bound_is_not_fixed_source_evidence'),
            ('small_disturbance', 'abs(u - target) > vu', 'abs(u - target) > max(vu, tolerance)', 'test_sa02_small_forcing_invalidates_old_deadline'),
            ('unknown_uncertainty', "vu = voltage['absolute_uncertainty'] if voltage else None", "vu = voltage['absolute_uncertainty'] if voltage else 0.", 'test_sa05_missing_calibration_does_not_mean_zero_uncertainty'),
            ('clock_uncertainty', 'if t - tu >= start and t + tu <= end', 'if start <= t <= end', 'test_sa05_boundary_sample_with_uncertain_clock_is_not_contradiction'),
            ('identity_binding', 'assessment_id=digest(canonical_bytes(identity))', "assessment_id='0' * 64", 'test_sa09_source_revision_changes_identity'),
            ('qualification_barrier', "hardware_qualification='not_qualified'", "hardware_qualification='qualified'", 'test_sa01_constant_control_is_scoped_not_certified'),
            ('scope_label', "model['scope'] = SCOPE", "model['scope'] = ''", 'test_sa11_published_contract_names_three_scopes'),
            ('observation_scope', "comparison_policy='recorded_values_at_nominal_timestamps'", "comparison_policy=''", 'test_sa11_published_contract_names_three_scopes'),
            ('no_output_reset', "initial = request['initial']['voltage_v']", "initial = request['output_v'][0]", 'test_sa07_replay_uses_declared_first_state_not_measured_resets'),
            ('native_tolerance', "if request['tolerance_v'] != inputs.get('tolerance_v'):", 'if False:', 'test_sa09_native_tolerance_cannot_be_relabelled'),
            ('native_component_binding', '    scalar_model_binding(request, native)', '    pass', 'test_sa09_components_must_match_native_model'),
            ('initial_origin_binding', "if request['initial']['origin'] == 'first_measured_output' and request['initial']['voltage_v'] != arrays[2][0]:", 'if False:', 'test_sa09_initial_origin_must_match_measurement'),
        ]
        receipts = []
        for name, before, after, test in changes:
            assert source.count(before) == 1, name
            mutant = load(source.replace(before, after, 1), path)
            killed, log = run(mutant, [test])
            assert len(killed.failures) == 1 and not killed.errors, f'{name}: {log}'
            receipts.append(dict(mechanism=name, test=test, assertion_failures=1, errors=0, log=log))
        report['mechanism_removals'] = receipts
        report['status'] = 'PASS'
    finally:
        tests.adapter = original
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(f'{green.testsRun} green; {len(red.failures)} legacy contract assertion failures; {len(receipts)} removals caught')


if __name__ == '__main__':
    main()
