"""Bounded Amendment 2 runner. Survey remains NOT RUN pending review.

Physics, alpha-selection rule and HP08 thresholds are unchanged. COMPLETE
means execution finished, not scientific acceptance or an HP09 utility result.
"""
import argparse
import hashlib
import json
import math
import os
import shutil
import subprocess
import sys
import time
import uuid
from pathlib import Path

import numpy as np
import scipy.linalg as la

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import ym_helike_quantum as q  # noqa: E402

COUPLINGS = (1.0, 0.5, 2.0)
ALPHAS = (0.8, 0.9, 1.0, 1.1, 1.2, 1.35, 1.5, 1.7, 2.0)
N_SELECT = 10
CUTOFFS = (8, 10, 12, 14, 16)
GPU_BUDGET_BYTES = 4 * 1024**3
HOST_BUDGET_BYTES = 8 * 1024**3
TIME_BUDGET_SECONDS = 2700
REQUIRED_GATES = ('HP11_width', 'HP12_spin', 'HP13_enclosure', 'runner_tests', 'mutations')
FILES = (
    'scripts/ym_helike_quantum.py', 'scripts/run_ym_helike_convergence.py',
    'scripts/test_ym_helike_convergence.py', 'scripts/check_ym_helike_convergence_mutations.py',
    'scripts/verify_ym_helike_evidence.py',
    'docs/research/helike-quantum-probe-preregistration-2026-11-23.md',
    'docs/research/helike-quantum-gpu-amendment-2026-11-23.md',
    'docs/research/helike-quantum-convergence-amendment-2026-11-23.md',
)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def atomic(path, obj):
    data = json.dumps(obj, indent=2, allow_nan=False) + '\n'
    tmp = path.with_name(path.name + f'.tmp{os.getpid()}')
    tmp.write_text(data, encoding='utf-8')
    os.replace(tmp, path)


def read_json(path):
    def reject(value):
        raise ValueError(f'nonfinite JSON constant: {value}')
    return json.loads(path.read_text(encoding='utf-8'), parse_constant=reject)


def source_hashes():
    return {name: digest(ROOT/name) for name in FILES}


def resource_estimate(cutoff):
    if cutoff not in (4, N_SELECT) + CUTOFFS + (18, 20):
        raise ValueError('unsupported cutoff')
    dimension = sum(math.comb((n-3*(n % 2))//2+5, 5)
                    for n in range(cutoff+1) if n != 1)
    rows = math.comb(cutoff+4+9, 9)
    matrix_bytes = dimension**2*8
    return {'dimension': dimension, 'exterior_rows': rows,
            'estimated_gpu_bytes': 20*matrix_bytes,
            'estimated_host_bytes': 24*matrix_bytes + rows*(9*16 + 32*8 + 40)}


def enforce_budget(cutoff, device):
    estimate = resource_estimate(cutoff)
    if estimate['estimated_host_bytes'] > HOST_BUDGET_BYTES:
        raise MemoryError(f'N={cutoff} host estimate exceeds budget before construction')
    if device == 'cuda' and estimate['estimated_gpu_bytes'] > GPU_BUDGET_BYTES:
        raise MemoryError(f'N={cutoff} GPU estimate exceeds budget before construction')
    return estimate


def prepare_basis(cutoff, device):
    enforce_budget(cutoff, device)
    b = q.build_basis(cutoff) if cutoff <= 12 else q.build_basis_extended(cutoff)
    return b, q.build_operators(b)


def case(b, ops, g2, alpha, device):
    enforce_budget(b['cutoff'], device)
    h = q.hamiltonian(ops, g2, alpha)
    e, v, diag = q.solve_generalized(h, b['S'], device)
    agreement = {'mode': 'cpu_only', 'eigen_error': 0., 'projector_error': 0.}
    if device == 'cuda':
        ec, vc, cpu = q.solve_generalized(h, b['S'], 'cpu')
        error = float(np.max(np.abs(e-ec))/max(1., np.max(np.abs(ec))))
        l = la.cholesky(b['S'], lower=True)
        projectors = []
        for group in q.reported_groups(e, v, [ops['tensor'], [ops['scalar']]]):
            ug, uc = l.T@v[:, group], l.T@vc[:, group]
            projectors.append(float(la.norm(ug@ug.conj().T-uc@uc.conj().T)))
        agreement = {'mode': 'cpu_cuda', 'eigen_error': error, 'projector_error': max(projectors), 'cpu_solver': cpu}
        if error > 1e-10 or max(projectors) > 1e-10:
            raise ArithmeticError('CPU/CUDA spectrum or projector disagreement')
    probes = q.scaled_probes(ops, alpha)
    spins = q.level_spins(e, v, ops['J2'])
    out = {'cutoff': b['cutoff'], 'g2': g2, 'alpha': alpha, 'dimension': len(e),
           'E0': float(e[0]), 'E1': float(e[1]), 'absolute_gap': float(e[1]-e[0]),
           'solver': diag, 'solver_agreement': agreement, 'solver_agreement_gate': True,
           'spins': spins, 'spin_gate': True, 'evidence_gate': False,
           'H2': q.probe_report_with_spin(e, v, probes['tensor'], ops['J2'], 'H2'),
           'H0': q.probe_report_with_spin(e, v, [probes['scalar']], ops['J2'], 'H0'),
           'probe_coordinates': 'physical q=x/sqrt(alpha); H2(x)/alpha; H0(x)/alpha^2'}
    out['exterior'] = q.exterior_diagnostics_scaled(b, ops, e, v, g2, alpha)
    arrays = {'H': h, 'S': b['S'], 'J2': ops['J2'], 'energies': e, 'vectors': v,
              'tensor': np.array(probes['tensor']), 'scalar': probes['scalar'],
              'alpha': np.array(alpha), 'g2': np.array(g2)}
    return out, arrays


def save_case(folder, result, arrays):
    name = f"N{result['cutoff']}-g2-{result['g2']:g}-alpha-{result['alpha']:g}.npz"
    path = folder/name
    temporary = path.with_suffix('.pending')
    with temporary.open('wb') as stream:
        np.savez_compressed(stream, **arrays)
    temporary.replace(path)
    result.update(arrays=name, arrays_sha256=digest(path))


def compare_report(actual, expected, path='report'):
    if isinstance(expected, dict):
        if not isinstance(actual, dict) or actual.keys() != expected.keys():
            raise ValueError(f'{path} fields differ')
        for key in expected:
            compare_report(actual[key], expected[key], f'{path}.{key}')
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise ValueError(f'{path} length differs')
        for i, item in enumerate(expected):
            compare_report(actual[i], item, f'{path}[{i}]')
    elif type(expected) is float:
        if type(actual) not in (int, float) or not np.isfinite(actual) or not np.isclose(actual, expected, atol=1e-10, rtol=1e-10):
            raise ValueError(f'{path} numeric value differs')
    elif actual != expected or type(actual) is not type(expected):
        raise ValueError(f'{path} value differs')


def check_saved_case(folder, result, replay_exterior=False):
    from verify_ym_helike_evidence import check_case
    name = f"N{result['cutoff']}-g2-{result['g2']:g}-alpha-{result['alpha']:g}.npz"
    if result.get('arrays') != name or digest(folder/name) != result.get('arrays_sha256'):
        raise ValueError('array integrity failure')
    with np.load(folder/name, allow_pickle=False) as arrays:
        check_case(result, arrays)
        for key in ('alpha', 'g2'):
            if arrays[key].shape != () or float(arrays[key]) != result[key]:
                raise ValueError(f'{key} differs from stored coordinates')
        e, v, j2 = (arrays[k] for k in ('energies', 'vectors', 'J2'))
        compare_report(result['spins'], q.level_spins(e, v, j2), 'spins')
        for channel, probes in (('H2', arrays['tensor']), ('H0', [arrays['scalar']])):
            compare_report(result[channel], q.probe_report_with_spin(e, v, probes, j2, channel), channel)
        if replay_exterior:
            b, ops = prepare_basis(result['cutoff'], 'cpu')
            physical = q.scaled_probes(ops, result['alpha'])
            for key, expected in (('H', q.hamiltonian(ops, result['g2'], result['alpha'])),
                                  ('S', b['S']), ('J2', ops['J2']),
                                  ('tensor', np.array(physical['tensor'])), ('scalar', physical['scalar'])):
                np.testing.assert_allclose(arrays[key], expected, atol=1e-10, rtol=1e-10)
            compare_report(result['exterior'], q.exterior_diagnostics_scaled(
                b, ops, e, v, result['g2'], result['alpha']), 'exterior')
    return True


def gated_convergence(history, gates):
    if set(gates) != set(REQUIRED_GATES) or not all(gates[k] is True for k in REQUIRED_GATES):
        return False
    if not history or not all(c.get(k) is True for c in history
                              for k in ('spin_gate', 'evidence_gate', 'solver_agreement_gate')):
        return False
    if not all(c[channel].get('spin_status') == 'VALIDATED' for c in history for channel in ('H0', 'H2')):
        return False
    return bool(q.convergence(history))


def mark_failed(receipt, status, reason):
    receipt.update(status=status, error=reason[:1000], scientific_status={})
    for track in receipt.get('results', {}).values():
        track['converged'] = False


def run_prerequisites(folder, receipt):
    import unittest
    import test_ym_helike_convergence as tests
    import check_ym_helike_convergence_mutations as mutations
    for label, cls in (('HP11_width', tests.WidthTests), ('HP12_spin', tests.SpinTests),
                       ('HP13_enclosure', tests.EnclosureTests), ('runner_tests', tests.RunnerTests)):
        with (folder/f'{label}.txt').open('w', encoding='utf-8') as stream:
            result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(cls))
        receipt['gates'][label] = result.wasSuccessful() and result.testsRun > 0
        atomic(folder/'receipt.json', receipt)
        if not receipt['gates'][label]:
            raise RuntimeError(f'{label} failed; no survey allowed')
    checks = mutations.run()
    atomic(folder/'mutation-checks.json', checks)
    receipt['gates']['mutations'] = bool(checks) and all(r['rejected'] for r in checks)
    atomic(folder/'receipt.json', receipt)
    if not receipt['gates']['mutations']:
        raise RuntimeError('mechanism-removal gate failed; no survey allowed')


def verify_run(folder, receipt, replay=True):
    if receipt.get('status') != 'COMPLETE' or folder.name != 'convergence-'+receipt.get('run_id', ''):
        raise ValueError('incomplete, failed or mismatched run')
    if receipt.get('certified_spectral_enclosure') is not False or receipt.get('HP09_utility') != 'NOT_RUN':
        raise ValueError('unsupported certification or utility claim')
    if receipt.get('source_hashes') != source_hashes():
        raise ValueError('source hashes differ; replay from the saved source snapshot')
    if any(digest(folder/'source'/name) != value for name, value in receipt['source_hashes'].items()):
        raise ValueError('source snapshot integrity failure')
    gates = receipt.get('gates', {})
    if set(gates) != set(REQUIRED_GATES) or not all(gates[k] is True for k in REQUIRED_GATES):
        raise ValueError('missing or failed prerequisite gate')
    cutoffs = receipt['cutoffs']
    if cutoffs not in [list(c for c in CUTOFFS+(18, 20) if c <= n) for n in (12, 14, 16, 18, 20)]:
        raise ValueError('unregistered cutoff sequence')
    if receipt.get('alphas') != list(ALPHAS) or receipt.get('n_select') != N_SELECT:
        raise ValueError('alpha selection contract changed')
    expected_tracks = set()
    for g2 in COUPLINGS:
        choice = receipt['alpha_selection'][str(g2)]
        table = choice['E0_by_alpha']
        if list(table) != [str(a) for a in ALPHAS] or not np.isfinite(list(table.values())).all():
            raise ValueError('incomplete alpha table')
        if choice['alpha_star'] != float(min(table, key=table.get)):
            raise ValueError('alpha does not follow frozen ground-energy rule')
        for alpha in sorted({1., choice['alpha_star']}):
            key = f'g2={g2:g},alpha={alpha:g}'
            expected_tracks.add(key)
            track = receipt['results'][key]
            hist = track['history']
            if [c['cutoff'] for c in hist] != cutoffs or any(c['g2'] != g2 or c['alpha'] != alpha for c in hist):
                raise ValueError('partial or mixed-coordinate history')
            for c in hist:
                if replay:
                    check_saved_case(folder, c, replay_exterior=True)
                elif digest(folder/c['arrays']) != c['arrays_sha256']:
                    raise ValueError('array integrity failure')
                if not all(c.get(k) is True for k in ('spin_gate', 'evidence_gate', 'solver_agreement_gate')):
                    raise ValueError('failed case validation')
            converged = gated_convergence(hist, gates)
            status = 'numerically_converged_model_estimate' if converged else 'finite_cutoff_observations_only'
            if track['converged'] is not converged or receipt['scientific_status'].get(key) != status:
                raise ValueError('forged convergence verdict')
    if set(receipt['results']) != expected_tracks or set(receipt['scientific_status']) != expected_tracks:
        raise ValueError('missing or extra convergence track')
    return {'integrity': 'VERIFIED', 'run_id': receipt['run_id'], 'tracks': len(expected_tracks),
            'certified_spectral_enclosure': False, 'HP09_utility': 'NOT_RUN'}


def worker(folder):
    receipt = read_json(folder/'receipt.json')
    device = receipt['device']
    started = time.monotonic()
    try:
        if source_hashes() != receipt['source_hashes']:
            raise RuntimeError('sources changed before worker startup')
        if device == 'cuda':
            import torch
            if not torch.cuda.is_available():
                raise RuntimeError('CUDA unavailable; no CPU fallback')
            torch.set_num_threads(1)
            total = torch.cuda.get_device_properties(0).total_memory
            torch.cuda.set_per_process_memory_fraction(min(1., GPU_BUDGET_BYTES/total))
            torch.cuda.reset_peak_memory_stats()
            receipt['gpu'] = {'name': torch.cuda.get_device_name(0), 'torch': torch.__version__, 'cuda': torch.version.cuda}
        run_prerequisites(folder, receipt)
        for cutoff in receipt['cutoffs']:
            enforce_budget(cutoff, device)
        b, ops = prepare_basis(N_SELECT, device)
        choice = {}
        for g2 in COUPLINGS:
            table = {str(a): float(q.solve_generalized(q.hamiltonian(ops, g2, a), b['S'], device)[0][0]) for a in ALPHAS}
            best = min(table, key=table.get)
            choice[str(g2)] = {'alpha_star': float(best), 'E0_by_alpha': table,
                               'interior': best not in (str(ALPHAS[0]), str(ALPHAS[-1]))}
            receipt['alpha_selection'] = choice
            atomic(folder/'receipt.json', receipt)
        del b, ops
        # One basis at a time; release it before constructing the next cutoff.
        for cutoff in receipt['cutoffs']:
            t0 = time.monotonic()
            b, ops = prepare_basis(cutoff, device)
            build_seconds = time.monotonic()-t0
            for g2 in COUPLINGS:
                for alpha in sorted({1., choice[str(g2)]['alpha_star']}):
                    key = f'g2={g2:g},alpha={alpha:g}'
                    track = receipt['results'].setdefault(key, {'history': [], 'converged': False})
                    t0 = time.monotonic()
                    result, arrays = case(b, ops, g2, alpha, device)
                    result.update(seconds=time.monotonic()-t0, build_seconds=build_seconds)
                    save_case(folder, result, arrays)
                    del arrays
                    result['evidence_gate'] = check_saved_case(folder, result)
                    track['history'].append(result)
                    atomic(folder/'receipt.json', receipt)
                    print(f"{key} N={cutoff}: gap={result['absolute_gap']:.10f}", flush=True)
            del b, ops
        for track in receipt['results'].values():
            track['converged'] = gated_convergence(track['history'], receipt['gates'])
        receipt['status'] = 'COMPLETE'
        receipt['scientific_status'] = {k: ('numerically_converged_model_estimate' if v['converged']
                                           else 'finite_cutoff_observations_only') for k, v in receipt['results'].items()}
        receipt['elapsed_seconds'] = time.monotonic()-started
        if device == 'cuda':
            receipt['peak_gpu_allocated_bytes'] = torch.cuda.max_memory_allocated()
            receipt['peak_gpu_reserved_bytes'] = torch.cuda.max_memory_reserved()
            if receipt['peak_gpu_reserved_bytes'] > GPU_BUDGET_BYTES:
                raise MemoryError('measured CUDA reservation exceeds budget')
        verify_run(folder, receipt, replay=False)
        atomic(folder/'receipt.json', receipt)
        return 0
    except BaseException as exc:
        mark_failed(receipt, 'FAILED', f'{type(exc).__name__}: {exc}')
        atomic(folder/'receipt.json', receipt)
        raise


def supervise(process, folder):
    import psutil
    started, peak = time.monotonic(), 0
    try:
        while process.poll() is None:
            try:
                rss = psutil.Process(process.pid).memory_info().rss
            except psutil.NoSuchProcess:
                break
            peak = max(peak, rss)
            if time.monotonic()-started > TIME_BUDGET_SECONDS or rss > HOST_BUDGET_BYTES:
                process.kill(); process.wait()
                receipt = read_json(folder/'receipt.json')
                mark_failed(receipt, 'RESOURCE_STOP', 'time or host-memory ceiling reached')
                receipt['peak_host_rss_bytes'] = peak
                atomic(folder/'receipt.json', receipt)
                return 2
            time.sleep(.5)
        code = process.wait()
    except BaseException:
        process.kill(); process.wait()
        receipt = read_json(folder/'receipt.json')
        mark_failed(receipt, 'FAILED', 'supervisor interrupted')
        atomic(folder/'receipt.json', receipt)
        raise
    receipt = read_json(folder/'receipt.json')
    receipt['peak_host_rss_bytes'] = peak
    if code != 0 or receipt['status'] != 'COMPLETE':
        mark_failed(receipt, 'FAILED', f'worker exited {code} without verified completion')
    atomic(folder/'receipt.json', receipt)
    return code if code else (0 if receipt['status'] == 'COMPLETE' else 2)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('out', nargs='?', type=Path)
    ap.add_argument('--device', default='cuda', choices=('cuda', 'cpu'))
    ap.add_argument('--max-cutoff', type=int, default=16, choices=(12, 14, 16, 18, 20))
    ap.add_argument('--worker', type=Path, help=argparse.SUPPRESS)
    ap.add_argument('--verify', type=Path, help='Replay from this source snapshot; may be expensive')
    args = ap.parse_args()
    from threadpoolctl import threadpool_limits
    if args.worker:
        with threadpool_limits(limits=1):
            return worker(args.worker)
    if args.verify:
        with threadpool_limits(limits=1):
            print(json.dumps(verify_run(args.verify, read_json(args.verify/'receipt.json')), indent=2))
        return 0
    if args.out is None:
        ap.error('output directory required')
    run_id = time.strftime('%Y%m%dT%H%M%SZ', time.gmtime()) + '-' + uuid.uuid4().hex[:8]
    folder = args.out.resolve()/f'convergence-{run_id}'
    folder.mkdir(parents=True, exist_ok=False)
    receipt = {'status': 'RUNNING', 'run_id': run_id, 'device': args.device,
               'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
               'source_hashes': source_hashes(), 'alphas': list(ALPHAS), 'n_select': N_SELECT,
               'cutoffs': [c for c in CUTOFFS+(18, 20) if c <= args.max_cutoff],
               'gates': {name: False for name in REQUIRED_GATES}, 'results': {}, 'scientific_status': {},
               'HP09_utility': 'NOT_RUN', 'certified_spectral_enclosure': False,
               'budgets': {'host_bytes': HOST_BUDGET_BYTES, 'gpu_bytes': GPU_BUDGET_BYTES, 'seconds': TIME_BUDGET_SECONDS},
               'environment': {'python': sys.version, 'numpy': np.__version__}}
    atomic(folder/'receipt.json', receipt)
    try:
        for name, value in receipt['source_hashes'].items():
            target = folder/'source'/name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT/name, target)
            if digest(target) != value:
                raise ValueError('source changed during snapshot capture')
        env = dict(os.environ, OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1')
        with (folder/'worker.log').open('w', encoding='utf-8') as log:
            process = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), '--worker', str(folder)],
                                       cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
            return supervise(process, folder)
    except BaseException as exc:
        receipt = read_json(folder/'receipt.json')
        mark_failed(receipt, 'FAILED', f'{type(exc).__name__}: {exc}')
        atomic(folder/'receipt.json', receipt)
        raise


if __name__ == '__main__':
    raise SystemExit(main())
