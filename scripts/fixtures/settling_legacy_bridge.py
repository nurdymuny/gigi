"""Test-only projection of the old Boolean adapter into the proposed schema.

None denotes a claim the old adapter did not supply, not a historical verdict.
This bridge does not pretend the old code reported positive applicability.
"""
import copy

REQUIRED_EVIDENCE = ('voltage_calibration', 'timebase_calibration', 'component_measurements',
                     'switch_characterization', 'uncertainty_budget', 'held_out_protocol')


def hardware_qualified(metadata, numerical_pass):
    # Exact old shortcut, retained only as a negative-control fixture.
    return bool(numerical_pass and all(metadata.get(k) is True for k in REQUIRED_EVIDENCE))


def assess(request, source_bytes, evidence_documents=None):
    qualified = hardware_qualified(request.get('legacy_metadata', {}), True)
    return dict(assessment_id='', model_calculation=copy.deepcopy(request['model']),
        observed_window=dict(status=None, samples_after_bound=None,
                             observed_duration_s=None, comparison_policy=None),
        forcing_applicability=dict(status=None, horizon_s=None, max_recorded_departure_v=None),
        hardware_qualification='qualified' if qualified else 'not_qualified',
        next_evidence=[], uncertainty=dict(voltage_v=None, time_s=None))
