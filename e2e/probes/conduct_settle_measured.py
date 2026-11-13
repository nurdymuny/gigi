"""Measured RC audit. No fitting; fixed protocol in docs/research.

Run: python e2e/probes/conduct_settle_measured.py --build --output <new-directory>
The downloaded scope data is not redistributed and is not calibration evidence.
"""
from __future__ import annotations
import argparse
import csv
import hashlib
import io
import json
import math
from pathlib import Path
import subprocess
import urllib.request
import numpy as np

from conduct_settle_live import Engine, ROOT, bundle, conduct, settle
from settling_applicability import assess, canonical_bytes, digest, SI

REQUIRED_EVIDENCE = ("voltage_calibration", "timebase_calibration", "component_measurements",
                     "switch_characterization", "uncertainty_budget", "held_out_protocol")

def parse_scope(text, resistance, capacitance, headers=('time (s)', 'square wave (V)', 'Vcapacitor (V)')):
    rows = list(csv.reader(io.StringIO(text.replace('\r\r\n', '\n'), newline=None)))
    if not rows:
        raise ValueError('incomplete scope export')
    header = rows[0]
    if any(k not in header for k in headers):
        raise ValueError(f'header mismatch: expected {headers}, found {header}')
    columns = [header.index(k) for k in headers]
    components = {}
    pending = None
    values = []
    for row in rows:
        first = row[0].strip() if row else ''
        if first in ('Resistance (ohms)', 'Capacitance (Farads)'):
            pending = first
        elif first and pending:
            components[pending] = float(first)
            pending = None
        if row is header:
            continue
        cells = [row[i].strip() if i < len(row) else '' for i in columns]
        if not any(cells):
            continue
        if not all(cells):
            raise ValueError('incomplete time/input/output observation')
        value = [float(x) for x in cells]
        if not all(math.isfinite(x) for x in value):
            raise ValueError('observations must be finite')
        values.append(value)
    if components != {'Resistance (ohms)': resistance, 'Capacitance (Farads)': capacitance}:
        raise ValueError(f'component declaration mismatch: {components}')
    if len(values) < 2:
        raise ValueError('incomplete trace: need at least two observations')
    t, u, v = np.asarray(values).T
    if np.any(np.diff(t) <= 0):
        raise ValueError('timestamps must be strictly increasing')
    return t, u, v

def source_phases(times, inputs):
    t, u = np.asarray(times, dtype=float), np.asarray(inputs, dtype=float)
    if len(t) != len(u) or len(t) < 2 or not np.isfinite([t, u]).all():
        raise ValueError('finite aligned times and input voltages required')
    if np.any(np.diff(t) <= 0):
        raise ValueError('timestamps must be strictly increasing')
    low, high = np.percentile(u, [5, 95])
    if high <= low:
        raise ValueError('nonzero input swing required')
    sides = u >= (low + high) / 2
    cuts = np.flatnonzero(sides[1:] != sides[:-1]) + 1
    if len(cuts) >= 64:
        raise ValueError('more than 64 plateaus; no truncation permitted')
    indices = [0, *cuts.tolist(), len(t)]
    boundaries = [float(t[0]), *[(float(t[i-1])+float(t[i]))/2 for i in cuts], float(t[-1])]
    uncertainties = [0., *[(float(t[i])-float(t[i-1]))/2 for i in cuts]]
    return [dict(start=boundaries[j], end=boundaries[j+1],
                 source=float(np.median(u[a:b])), event_uncertainty=uncertainties[j])
            for j, (a, b) in enumerate(zip(indices[:-1], indices[1:]))]

def observed_settling(times, errors, tolerance, minimum_followup=5):
    if len(times) != len(errors) or minimum_followup < 1 or tolerance <= 0:
        raise ValueError('aligned observations, positive tolerance and follow-up required')
    if not np.isfinite(times).all() or not np.isfinite(errors).all():
        raise ValueError('observations must be finite')
    good = np.abs(errors) <= tolerance
    suffix = np.logical_and.accumulate(good[::-1])[::-1]
    for i in range(len(times)-minimum_followup+1):
        if suffix[i]:
            return float(times[i])
    return None

def hardware_qualified(metadata, numerical_pass):
    """Legacy compatibility only; Booleans cannot constitute hardware evidence."""
    return False


DATASETS = [
    ('rc1', 1000, 1e-8, '1UoV0Pwj85WiBQRafID7pXmjuMdURN4HAUl1gb92KEc4'),
    ('rc2', 10000, 1e-8, '1YbKOis6QQ40RrqcI8_9EAsbye9MOVV0xxwcnO2fpFBo'),
    ('rc3', 100000, 1e-8, '1I7xIqoPz8CdR-OZh7bdxhMl3n1cZ3WTJ5YErhIdt0oU'),
    ('rc4', 1000, 1e-6, '1txaMWy5D5CtJWt_z-uxNDbZarNIzh7X3ZyduD7LoiQQ'),
    ('rc5', 10000, 1e-6, '1AJfUKWDaShTmqNfTyzUIJsalWfWIZXmyx64_a0Tfkr4'),
    ('rc6', 100000, 1e-6, '1dyXOdOuLxcAWXYQl_L_vVUSEYxcp5GlXt_VWaLaii4U'),
]


def configuration(resistance, voltage):
    return dict(edges=[], ports=[dict(node='cap', conductance_s=1/resistance, voltage_v=voltage)])


def independent_values(times, phases, initial, tau):
    states = [initial]
    for phase in phases[:-1]:
        states.append(phase['source'] + (states[-1]-phase['source']) *
                      math.exp(-(phase['end']-phase['start'])/tau))
    result = []
    for t in times:
        index = min(int(np.searchsorted([p['end'] for p in phases], t, side='right')), len(phases)-1)
        p = phases[index]
        result.append(p['source'] + (states[index]-p['source'])*math.exp(-(t-p['start'])/tau))
    return np.asarray(result)


def compare_trace(engine, out, name, resistance, capacitance, t, u, v, *, source_bytes, source_identity):
    raw_name = name+'_observations'
    engine.ok('/v1/bundles', dict(name=raw_name, schema=dict(
        fields={k:'numeric' for k in ('seq', 'time_s', 'source_v', 'capacitor_v')}, keys=['seq'])))
    observations = [dict(seq=i, time_s=float(tt), source_v=float(uu), capacitor_v=float(vv))
                    for i, (tt, uu, vv) in enumerate(zip(t, u, v))]
    for start in range(0, len(t), 1000):
        engine.ok(f'/v1/bundles/{raw_name}/insert', dict(records=observations[start:start+1000]))
    stored = engine.ok('/v1/gql', dict(query=f'SELECT * FROM {raw_name};'))
    assert 'rows' in stored, 'SELECT must return rows, not a success-only acknowledgment'
    assert sorted(stored['rows'], key=lambda r:r['seq']) == observations, 'stored observations must round-trip exactly'

    phases = source_phases(t, u)
    shifted = [dict(p, start=p['start']-float(t[0]), end=p['end']-float(t[0])) for p in phases]
    schedule = [dict(duration_s=p['end']-p['start'], configuration=configuration(resistance, p['source']))
                for p in shifted]
    bundle(engine, name, [dict(id='cap', c=capacitance, v=float(v[0]))])
    relative = t-t[0]
    # Query actual observations and switching boundaries in one continuous prediction.
    requested = sorted(set(relative.tolist()+[p['start'] for p in shifted]))
    predicted = []
    for start in range(0, len(requested), 2048):
        batch = requested[start:start+2048]
        response = conduct(engine, name, schedule, batch)
        assert response['node_order'] == ['cap']
        assert [s['time_s'] for s in response['samples']] == batch
        predicted.extend(s['voltages_v'][0] for s in response['samples'])
    expected = independent_values(requested, shifted, float(v[0]), resistance*capacitance)
    oracle_error = float(np.max(np.abs(np.asarray(predicted)-expected)))
    assert oracle_error <= 1e-9, f'engine/oracle disagreement: {oracle_error}'
    by_time = dict(zip(requested, predicted))
    prediction = np.array([by_time[x] for x in relative])
    residual = prediction-v
    swing = float(np.percentile(u, 95)-np.percentile(u, 5))
    rmse = float(np.sqrt(np.mean(residual**2)))
    max_error = float(np.max(np.abs(residual)))
    diagnostic_pass = rmse/swing <= .05 and max_error/swing <= .10
    events = []
    for p in shifted[1:]:
        initial = by_time[p['start']]
        cfg = configuration(resistance, p['source'])
        engine.ok(f'/v1/bundles/{name}/insert', dict(records=[dict(id='cap', c=capacitance, v=initial)]))
        tol = .02*swing
        result = settle(engine, name, cfg, {'cap':1.}, tol, p['end']-p['start'])
        oracle_bound = resistance*capacitance*max(0., math.log(max(abs(initial-p['source'])/tol, 1.)))
        assert math.isclose(result['settling_bound_s'], oracle_bound, rel_tol=1e-9, abs_tol=1e-12)
        assert math.isclose(result['readout_equilibrium_v'], p['source'], rel_tol=1e-12, abs_tol=1e-12)
        indices = (relative >= p['start']) & (relative <= p['end'])
        elapsed = relative[indices]-p['start']
        errors = v[indices]-p['source']
        after = elapsed >= result['settling_bound_s']
        count = int(np.count_nonzero(after))
        status = ('insufficient_followup' if count < 5 else
                  'within_tolerance' if np.all(np.abs(errors[after]) <= tol) else 'observed_departure')
        request = dict(source=source_identity, units=SI,
            circuit=dict(resistance_ohm=resistance, capacitance_f=capacitance),
            initial=dict(voltage_v=float(v[0]), origin='first_measured_output'),
            schedule=[dict(start_s=q['start']+float(t[0]), end_s=q['end']+float(t[0]), source_v=q['source']) for q in shifted],
            horizon_s=[p['start']+float(t[0]), p['end']+float(t[0])], target_v=p['source'], tolerance_v=tol,
            times_s=t.tolist(), source_v=u.tolist(), output_v=v.tolist(),
            model=dict(native_result=result, receipt=dict(id=f'{name}/settle/{len(events)}',
                revision=str(result['source']['revision']), sha256=digest(canonical_bytes(result)))), evidence={})
        assessment = assess(request, source_bytes)
        assert assessment['observed_window']['status'] == status, 'adapter must preserve frozen observation rule'
        assert assessment['observed_window']['samples_after_bound'] == count
        assert assessment['model_calculation']['native_result'] == result
        events.append(dict(source_v=p['source'], switch_time_s=p['start']+float(t[0]),
            event_uncertainty_s=p['event_uncertainty'], predicted_initial_v=initial,
            tolerance_v=tol, model_bound_s=result['settling_bound_s'], oracle_bound_s=oracle_bound,
            observed_suffix_settling_s=observed_settling(elapsed, errors, tol),
            samples_after_bound=count, observation_status=status,
            max_observed_error_after_bound_v=float(np.max(np.abs(errors[after]))) if count else None,
            meets_model_deadline=result['meets_deadline'], assessment=assessment))
    with (out/f'{name}-comparison.csv').open('w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['time_s','source_v','measured_capacitor_v','gigi_capacitor_v','residual_v'])
        writer.writerows(zip(t, u, v, prediction, residual))
    return dict(n=len(t), tau_s=resistance*capacitance, input_swing_v=swing, phases=phases,
        rmse_v=rmse, max_error_v=max_error, normalized_rmse=rmse/swing,
        normalized_max_error=max_error/swing, diagnostic_screen_pass=diagnostic_pass,
        engine_oracle_max_error_v=oracle_error, events=events,
        hardware_qualification='not_qualified', missing_evidence=list(REQUIRED_EVIDENCE))


def source_hashes():
    paths = sorted(p for p in (ROOT/'src').rglob('*') if p.is_file())
    paths += [ROOT/'Cargo.toml', ROOT/'Cargo.lock', ROOT/'openapi.json']
    return {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--build', action='store_true')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = ROOT/'target/debug/gigi-stream.exe'
    before = source_hashes()
    if args.build:
        with (out/'build.log').open('wb') as log:
            subprocess.run(['cargo','build','--features','kahler','--bin','gigi-stream','--target-dir',str(ROOT/'target')],
                           cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    assert source_hashes() == before, 'source changed during build'
    report = dict(status='RUNNING', CS08='OPEN', head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                  binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), source_manifest=before,
                  protocol_sha256=hashlib.sha256((ROOT/'docs/research/conduct-settle-measured-protocol-2026-11-13.md').read_bytes()).hexdigest(),
                  harness_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), traces=[])
    try:
        with Engine(binary, out) as engine:
            for name, resistance, capacitance, sheet in DATASETS:
                url = f'https://docs.google.com/spreadsheets/d/{sheet}/export?format=csv'
                entry = dict(name=name, resistance_ohm=resistance, capacitance_f=capacitance, url=url)
                try:
                    raw = urllib.request.urlopen(url, timeout=90).read()
                    (out/f'{name}-source.csv').write_bytes(raw)
                    entry['sha256'] = hashlib.sha256(raw).hexdigest()
                    headers = (('time','source voltage','Vcapacitor') if name in ('rc5','rc6')
                               else ('time (s)','square wave (V)','Vcapacitor (V)'))
                    entry['declared_header_mapping'] = headers
                    t, u, v = parse_scope(raw.decode('utf-8-sig'), resistance, capacitance, headers=headers)
                    entry.update(compare_trace(engine, out, name, resistance, capacitance, t, u, v,
                        source_bytes=raw, source_identity=dict(id=url, revision=entry['sha256'],
                            sha256=entry['sha256'], parser='parse_scope-v1:' + '|'.join(headers))))
                    entry['status'] = 'COMPARED'
                except Exception as error:
                    entry.update(status='REFUSED_OR_FAILED', error=f'{type(error).__name__}: {error}')
                report['traces'].append(entry)
                print(json.dumps({k:v for k,v in entry.items() if k not in ('phases','events')}, allow_nan=False), flush=True)
            report['source_unchanged'] = source_hashes() == before
            assert report['source_unchanged']
            report['status'] = 'DIAGNOSTIC_COMPLETE_HARDWARE_GATE_OPEN'
    finally:
        (out/'report.json').write_text(json.dumps(report, indent=2, allow_nan=False), encoding='utf-8')
    if any(t['status'] != 'COMPARED' for t in report['traces']):
        raise SystemExit('some traces refused or failed; see report.json; no omissions permitted')


if __name__ == '__main__':
    main()
