"""Read-only, horizon-scoped interpretation of scalar RC measurement evidence.

Not a new physics solver, calibration authority, or hardware certification API.
Documents are content-checked caller attestations, not independently verified facts.
"""
import copy
import hashlib
import json
import math

VERSION = 'settling-applicability-v1'
SI = dict(time='s', voltage='V', resistance='ohm', capacitance='F')
REQUEST_FIELDS = {'source', 'units', 'circuit', 'initial', 'schedule', 'horizon_s',
                  'target_v', 'tolerance_v', 'times_s', 'source_v', 'output_v', 'model', 'evidence'}
ROLES = {'voltage_calibration', 'timebase_calibration', 'forcing'}
SCOPE = ('Conditional fixed-source passive RC calculation using declared nominal components '
         'and initial state; not a measured-hardware readiness certificate.')


def canonical_bytes(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode('utf-8')


def digest(value):
    return hashlib.sha256(value).hexdigest()


def finite(value, label, positive=False):
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise ValueError(f'{label}: finite number required')
    if positive and value <= 0:
        raise ValueError(f'{label}: positive number required')
    return value


def window(value, label):
    if not isinstance(value, list) or len(value) != 2:
        raise ValueError(f'{label}: [start_s, end_s] required')
    a, b = [finite(v, label) for v in value]
    if b <= a:
        raise ValueError(f'{label}: end must be after start')
    return a, b


def reference(ref, label):
    if not isinstance(ref, dict):
        raise ValueError(f'{label}: content-addressed reference required')
    for key in ('id', 'revision', 'sha256'):
        if not isinstance(ref.get(key), str) or not ref[key]:
            raise ValueError(f'{label}.{key}: nonempty string required')
    if len(ref['sha256']) != 64 or any(c not in '0123456789abcdef' for c in ref['sha256']):
        raise ValueError(f'{label}: lowercase SHA256 digest required')


def scalar_model_binding(request, native):
    nodes = native.get('source', {}).get('nodes', [])
    order = native.get('node_order', [])
    inputs = native.get('inputs', {})
    config = inputs.get('configuration', {})
    ports = config.get('ports', [])
    if (len(order) != 1 or len(nodes) != 1 or len(ports) != 1 or config.get('edges') != []
            or order != [nodes[0].get('id')] or ports[0].get('node') != order[0]
            or inputs.get('readout') != {order[0]:1.} or native.get('units') != 'SI'):
        raise ValueError('applicability requires a one-node, one-port passive SI RC native model with unit readout')
    if request['tolerance_v'] != inputs.get('tolerance_v'):
        raise ValueError('tolerance_v must equal the native request tolerance, without reinterpretation')
    c = finite(nodes[0].get('capacitance_f'), 'native capacitance_f', positive=True)
    g = finite(ports[0].get('conductance_s'), 'native conductance_s', positive=True)
    if (not math.isclose(c, request['circuit']['capacitance_f'], rel_tol=1e-12, abs_tol=0.)
            or not math.isclose(g, 1/request['circuit']['resistance_ohm'], rel_tol=1e-12, abs_tol=0.)):
        raise ValueError('circuit components do not match the native model')


def validate(request, source_bytes):
    if not isinstance(request, dict):
        raise ValueError('request must be an object')
    unknown = set(request) - REQUEST_FIELDS
    missing = REQUEST_FIELDS - set(request)
    if unknown or missing:
        raise ValueError(f'unknown fields {sorted(unknown)}; missing fields {sorted(missing)}')
    if request['units'] != SI:
        raise ValueError(f'units must be explicit SI: {SI}')
    reference(request['source'], 'source')
    if not isinstance(source_bytes, bytes) or digest(source_bytes) != request['source']['sha256']:
        raise ValueError('source digest does not match supplied raw bytes')
    if not isinstance(request['source'].get('parser'), str) or not request['source']['parser']:
        raise ValueError('source.parser must name the ingesting application recipe')
    start, end = window(request['horizon_s'], 'horizon_s')
    finite(request['target_v'], 'target_v')
    finite(request['tolerance_v'], 'tolerance_v', positive=True)
    for key in ('resistance_ohm', 'capacitance_f'):
        finite(request['circuit'].get(key), 'circuit.' + key, positive=True)
    finite(request['initial'].get('voltage_v'), 'initial.voltage_v')
    if request['initial'].get('origin') not in ('declared_fixture', 'declared_initial_state', 'first_measured_output', 'predicted_state'):
        raise ValueError('initial.origin required; do not infer or reset the initial state')
    arrays = [request[k] for k in ('times_s', 'source_v', 'output_v')]
    if not all(isinstance(a, list) for a in arrays) or not arrays[0] or len({len(a) for a in arrays}) != 1:
        raise ValueError('aligned nonempty observation arrays required')
    for key, values in zip(('times_s', 'source_v', 'output_v'), arrays):
        for value in values:
            finite(value, key)
    if any(b <= a for a, b in zip(arrays[0], arrays[0][1:])):
        raise ValueError('times_s must be strictly increasing; no sorting or deduplication')
    if request['initial']['origin'] == 'first_measured_output' and request['initial']['voltage_v'] != arrays[2][0]:
        raise ValueError('initial voltage must match the first measured output for this declared origin')
    schedule = request['schedule']
    if not isinstance(schedule, list) or not schedule:
        raise ValueError('schedule must preserve the declared source phases')
    previous_end = None
    for phase in schedule:
        a, b = window([phase.get('start_s'), phase.get('end_s')], 'schedule')
        finite(phase.get('source_v'), 'schedule.source_v')
        if previous_end is not None and a != previous_end:
            raise ValueError('schedule phases must be contiguous and ordered')
        previous_end = b
    reference(request['model'].get('receipt'), 'model.receipt')
    native = request['model'].get('native_result', {})
    if digest(canonical_bytes(native)) != request['model']['receipt']['sha256']:
        raise ValueError('model result digest mismatch')
    scalar_model_binding(request, native)
    equilibrium = finite(native.get('readout_equilibrium_v'), 'model.readout_equilibrium_v')
    if not math.isclose(request['target_v'], equilibrium, rel_tol=1e-12, abs_tol=1e-12):
        raise ValueError('target_v does not match native model equilibrium')
    bound = finite(native.get('settling_bound_s'), 'model.settling_bound_s')
    if bound < 0:
        raise ValueError('model.settling_bound_s must be nonnegative')
    if not isinstance(request['evidence'], dict) or set(request['evidence']) - ROLES:
        raise ValueError('evidence has unknown roles; hardware approval is not accepted here')
    return start, end, bound


def evidence_document(role, request, documents, reasons):
    ref = request['evidence'].get(role)
    if ref is None:
        reasons.append(f'{role}: supply a source-specific, horizon-scoped evidence document.')
        return None
    reference(ref, 'evidence.' + role)
    raw = documents.get(ref['sha256'])
    if raw is None:
        reasons.append(f'{role}: referenced evidence is unavailable; supply its bytes.')
        return None
    if not isinstance(raw, bytes) or digest(raw) != ref['sha256']:
        raise ValueError(f'evidence {role}: digest mismatch')
    try:
        doc = json.loads(raw)
    except (ValueError, UnicodeDecodeError) as error:
        raise ValueError(f'evidence {role}: invalid JSON document') from error
    if not isinstance(doc, dict) or any(doc.get(k) != ref[k] for k in ('id', 'revision')):
        raise ValueError(f'evidence {role}: identity/revision mismatch')
    try:
        a, b = window(doc.get('horizon_s'), 'evidence.horizon_s')
        uncertainty_key = 'bound_v' if role == 'forcing' else 'absolute_uncertainty'
        value = finite(doc.get(uncertainty_key), role + '.' + uncertainty_key)
        if value < 0:
            raise ValueError('negative uncertainty')
    except ValueError as error:
        reasons.append(f'{role}: invalid evidence scope or uncertainty ({error}).')
        return None
    expected_unit = 's' if role == 'timebase_calibration' else 'V'
    if (doc.get('role') != role or doc.get('source_id') != request['source']['id']
            or doc.get('unit') != expected_unit
            or a > request['horizon_s'][0] or b < request['horizon_s'][1]):
        reasons.append(f'{role}: source, SI unit, or coverage does not apply to the entire requested horizon.')
        return None
    return doc


def assess(request, source_bytes, evidence_documents=None):
    """Return separate model, recorded-window, and conditional applicability claims."""
    start, end, bound = validate(request, source_bytes)
    documents = evidence_documents or {}
    reasons = []
    voltage = evidence_document('voltage_calibration', request, documents, reasons)
    clock = evidence_document('timebase_calibration', request, documents, reasons)
    forcing = evidence_document('forcing', request, documents, reasons)
    target, tolerance = request['target_v'], request['tolerance_v']
    rows = list(zip(request['times_s'], request['source_v'], request['output_v']))
    during = [(t, u, v) for t, u, v in rows if start <= t <= end]
    after = [(t, v) for t, _, v in during if t - start >= bound]
    errors = [abs(v - target) for _, v in after]
    count = len(after)
    observation_status = ('insufficient_followup' if count < 5 else
                          'within_tolerance' if all(e <= tolerance for e in errors) else
                          'observed_departure')
    departures = [abs(u - target) for _, u, _ in during]
    applicability = 'not_established'
    schedule_covers = request['schedule'][0]['start_s'] <= start and request['schedule'][-1]['end_s'] >= end
    schedule_contradicts = any(p['source_v'] != target for p in request['schedule']
                               if p['start_s'] < end and p['end_s'] > start)
    vu = voltage['absolute_uncertainty'] if voltage else None
    tu = clock['absolute_uncertainty'] if clock else None
    # Only timestamp uncertainty intervals fully inside the horizon can contradict it.
    contradictions = ([t for t, u, _ in during if t - tu >= start and t + tu <= end
                       and abs(u - target) > vu] if vu is not None and tu is not None else [])
    if schedule_contradicts:
        applicability = 'contradicted'
        reasons.append('schedule: declared source changes or differs from target inside the fixed-source horizon.')
    elif contradictions:
        applicability = 'contradicted'
        reasons.append('source_v: calibrated sample interval excludes the fixed target; retain measured input for diagnostic replay.')
    elif not schedule_covers:
        reasons.append('schedule: extend the declared source schedule to cover the entire requested horizon.')
    elif forcing and voltage and clock:
        if (forcing.get('basis') == 'independent_characterization'
                and forcing.get('target_v') == target and forcing['bound_v'] == 0.):
            applicability = 'supported_over_declared_horizon'
        else:
            reasons.append('forcing: fixed-source calculation requires independent zero-departure evidence at this target; a nonzero bound consumes error budget.')
    if count < 5:
        reasons.append('observed_window: acquire at least five samples after the model bound within the declared interval.')
    if applicability == 'not_established' and departures:
        reasons.append('source_v: finite recorded spread alone proves neither a constant source nor a future disturbance bound.')
    reasons.append('hardware_qualification: requires a separately approved held-out hardware protocol, component and switch characterization, and a complete uncertainty budget.')
    model = copy.deepcopy(request['model'])
    model['scope'] = SCOPE
    # No cache: evidence availability itself is included as well as referenced content.
    identity = dict(version=VERSION, request=request,
                    available_evidence=sorted(ref['sha256'] for ref in request['evidence'].values()
                                              if ref['sha256'] in documents))
    return dict(assessment_id=digest(canonical_bytes(identity)), adapter_version=VERSION,
        source=copy.deepcopy(request['source']), model_calculation=model,
        observed_window=dict(status=observation_status, samples_after_bound=count,
            first_sample_s=after[0][0] if count else None,
            last_sample_s=after[-1][0] if count else None,
            observed_duration_s=after[-1][0] - after[0][0] if count else None,
            max_recorded_error_v=max(errors) if count else None,
            comparison_policy='recorded_values_at_nominal_timestamps',
            scope='Finite recorded samples only; no guarantee between samples or after acquisition.'),
        forcing_applicability=dict(status=applicability, assumption='source equals target throughout declared horizon',
            horizon_s=list(request['horizon_s']), max_recorded_departure_v=max(departures) if departures else None,
            contradiction_samples=len(contradictions), evidence=copy.deepcopy(request['evidence']),
            scope='Conditional on supplied evidence; integrity/scope checked, certificate truth not independently established.'),
        uncertainty=dict(voltage_v=vu, time_s=tu),
        hardware_qualification='not_qualified', next_evidence=reasons)


def diagnostic_replay(request, source_bytes, mode, tau_bounds_s=None):
    """Explicit optional reconstruction; never edits observations or native results."""
    validate(request, source_bytes)
    from settling_diagnosis import propagate, tolerance_enclosure
    times, source = request['times_s'], request['source_v']
    initial = request['initial']['voltage_v']
    tau = request['circuit']['resistance_ohm'] * request['circuit']['capacitance_f']
    result = dict(mode=mode, implementation='application diagnostic, not native CONDUCT interpolation',
                  predicted_v=propagate(times, source, initial, tau, mode).tolist())
    if tau_bounds_s is not None:
        if len(tau_bounds_s) != 2:
            raise ValueError('tau_bounds_s requires two explicit endpoints')
        low, high = tolerance_enclosure(times, source, initial, *tau_bounds_s)
        result['enclosure'] = dict(low_v=low.tolist(), high_v=high.tolist(), tau_bounds_s=list(tau_bounds_s),
            assumptions='Known initial state; intersample source between adjacent readings; SI; no channel uncertainty included.',
            scope='Floating-point diagnostic enclosure, not an outward-rounded interval certificate; R and C are not separately identifiable.')
    return result
