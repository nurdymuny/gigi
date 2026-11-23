"""Frozen six-trace replay and live checks; never deploys or touches production."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import numpy as np

from conduct_settle_live import Engine, ROOT, math_gates, wire_gates
from conduct_settle_measured import compare_trace, parse_scope, source_hashes
from settling_diagnosis import propagate, matrix_oracle, native_left


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--input', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    prior = json.loads((args.input/'report.json').read_text())
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    manifest = source_hashes()
    helper_paths = [Path(__file__).with_name(name + '.py') for name in
                    ('settling_applicability', 'settling_applicability_live', 'settling_diagnosis',
                     'conduct_settle_measured', 'conduct_settle_live')]
    helper_paths += [ROOT/'docs/SETTLING_APPLICABILITY_SPEC.md']
    helpers = {str(p.relative_to(ROOT)):sha(p) for p in helper_paths}
    binary = ROOT/'target/debug/gigi-stream.exe'
    report = dict(status='RUNNING', CS08='OPEN', traces=[],
        head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        source_manifest=manifest, original_engine_manifest_matches=manifest == prior['source_manifest'],
        harness_sha256=sha(Path(__file__)), adapter_sha256=sha(Path(__file__).with_name('settling_applicability.py')),
        prior_report_sha256=sha(args.input/'report.json'), adapter_source_manifest=helpers)
    before_raw = {t['name']:sha(args.input/f"{t['name']}-source.csv") for t in prior['traces']}
    try:
        build = ['cargo', 'build', '--features', 'kahler', '--bin', 'gigi-stream', '--target-dir', str(ROOT/'target')]
        with (out/'build.log').open('wb') as log:
            subprocess.run(build, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        assert source_hashes() == manifest, 'engine source changed during build'
        report.update(build_command=build, binary_sha256=sha(binary), binary_mtime_ns=binary.stat().st_mtime_ns,
                      same_binary_as_frozen=sha(binary) == prior['binary_sha256'])
        with Engine(binary, out) as engine:
            report['live_math'] = math_gates(engine)
            report['live_wire'] = wire_gates(engine)
            for old in prior['traces']:
                name = old['name']
                raw = (args.input/f'{name}-source.csv').read_bytes()
                assert hashlib.sha256(raw).hexdigest() == old['sha256'], name
                t, u, v = parse_scope(raw.decode('utf-8-sig'), old['resistance_ohm'], old['capacitance_f'],
                                      headers=old['declared_header_mapping'])
                identity = dict(id=old['url'], revision=old['sha256'], sha256=old['sha256'],
                                parser='parse_scope-v1:' + '|'.join(old['declared_header_mapping']))
                new = compare_trace(engine, out, name, old['resistance_ohm'], old['capacitance_f'], t, u, v,
                                    source_bytes=raw, source_identity=identity)
                assert new['n'] == old['n']
                assert new['diagnostic_screen_pass'] == old['diagnostic_screen_pass']
                assert len(new['events']) == len(old['events'])
                for current, frozen in zip(new['events'], old['events']):
                    for key in ('observation_status', 'samples_after_bound', 'meets_model_deadline', 'tolerance_v'):
                        assert current[key] == frozen[key], (name, key)
                    assert abs(current['model_bound_s'] - frozen['model_bound_s']) <= 1e-12
                    a = current['assessment']
                    assert a['forcing_applicability']['status'] == 'not_established'
                    assert a['hardware_qualification'] == 'not_qualified'
                    assert a['uncertainty'] == dict(voltage_v=None, time_s=None)
                # All 2,499 intervals per trace cross many 64-phase native batch boundaries.
                native = native_left(engine, 'applicability_' + name, t, u, v[0], old['resistance_ohm'], old['capacitance_f'])
                hold_error = float(np.max(np.abs(native - propagate(t, u, v[0], old['tau_s'], 'left'))))
                linear = propagate(t, u, v[0], old['tau_s'], 'linear')
                linear_error = float(np.max(np.abs(matrix_oracle(t, u, v[0], old['tau_s']) - linear)))
                assert hold_error <= 1e-9 and linear_error <= 1e-9
                new.update(name=name, source_sha256=old['sha256'], native_hold_oracle_error_v=hold_error,
                           linear_matrix_oracle_error_v=linear_error,
                           diagnostic_linear_input_rmse_v=float(np.sqrt(np.mean((v-linear)**2))))
                report['traces'].append(new)
                print(json.dumps(dict(name=name, n=new['n'], events=[e['observation_status'] for e in new['events']],
                                     forcing=[e['assessment']['forcing_applicability']['status'] for e in new['events']],
                                     native_hold_oracle_error_v=hold_error)), flush=True)
        assert source_hashes() == manifest, 'engine changed during replay'
        assert before_raw == {t['name']:sha(args.input/f"{t['name']}-source.csv") for t in prior['traces']}
        assert report['harness_sha256'] == sha(Path(__file__))
        assert report['adapter_sha256'] == sha(Path(__file__).with_name('settling_applicability.py'))
        assert helpers == {str(p.relative_to(ROOT)):sha(p) for p in helper_paths}
        report['status'] = 'PASS_ADAPTER_AND_LIVE_REGRESSION_NOT_HARDWARE_QUALIFIED'
    except BaseException as error:
        report.update(status='FAIL', error=repr(error))
        raise
    finally:
        (out/'report.json').write_text(json.dumps(report, indent=2, allow_nan=False), encoding='utf-8')


if __name__ == '__main__':
    main()
