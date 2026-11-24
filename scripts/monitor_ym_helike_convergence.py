"""Read-only live observer; never changes the solver, thresholds or receipt.

Timestamps describe when this observer saw an event, not an invented completion
time. Existing cases are marked explicitly when attaching partway through a run.
"""
import argparse
import csv
import json
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

import psutil

COLUMNS = ['observed_at_utc', 'already_saved_when_monitor_started', 'run_id', 'source_revision',
           'g2', 'alpha', 'cutoff', 'E0', 'E1', 'absolute_gap', 'max_residual', 'allowed_residual',
           'residual_gate', 'H0_tail', 'H2_tail', 'tail_gate', 'H0_lowest_weight', 'H2_lowest_weight',
           'H0_accessible_gap', 'H2_accessible_gap', 'case_seconds', 'basis_build_seconds',
           'gpu_solve_seconds', 'cpu_solve_seconds', 'cpu_gpu_eigen_error', 'cpu_gpu_projector_error',
           'arrays', 'arrays_sha256']


def emit(stream, event):
    stream.write(json.dumps(event, allow_nan=False)+'\n')
    stream.flush()


def record_cases(receipt, seen, journal, writer, now, initial):
    count = 0
    for track in receipt.get('results', {}).values():
        for case in track['history']:
            key = (case['g2'], case['alpha'], case['cutoff'])
            if key in seen:
                continue
            ext = case['exterior']
            agreement = case.get('solver_agreement', {})
            row = dict(observed_at_utc=now, already_saved_when_monitor_started=initial,
                       run_id=receipt['run_id'], source_revision=receipt['source_revision'])
            row.update({k:case[k] for k in ('g2','alpha','cutoff','E0','E1','absolute_gap')})
            row.update(max_residual=max(ext['level_worst_residuals']), allowed_residual=ext['allowed_residual'],
                       residual_gate=ext['residual_gate'], tail_gate=ext['tail_gate'],
                       case_seconds=case.get('seconds'), basis_build_seconds=case.get('build_seconds'),
                       gpu_solve_seconds=case.get('solver', {}).get('elapsed_seconds'),
                       cpu_solve_seconds=agreement.get('cpu_solver', {}).get('elapsed_seconds'),
                       cpu_gpu_eigen_error=agreement.get('eigen_error'),
                       cpu_gpu_projector_error=agreement.get('projector_error'),
                       arrays=case.get('arrays'), arrays_sha256=case.get('arrays_sha256'))
            for channel in ('H0','H2'):
                row[channel+'_tail'] = ext['probe_discarded_fractions'][channel]
                row[channel+'_lowest_weight'] = case[channel]['lowest_weight']
                row[channel+'_accessible_gap'] = case[channel]['accessible_gap']
            emit(journal, {'event':'case_saved_observed', 'observed_at_utc':now,
                           'already_saved_when_monitor_started':initial, 'case':case})
            writer.writerow(row)
            seen.add(key)
            count += 1
    return count


def gpu_snapshot():
    try:
        result = subprocess.run(['nvidia-smi', '--query-gpu=index,utilization.gpu,memory.used,memory.total,temperature.gpu',
                                 '--format=csv,noheader,nounits'], capture_output=True, text=True, timeout=3)
        return {'scope':'device-wide, not this process alone', 'exit_code':result.returncode,
                'columns':['index','utilization_percent','memory_used_MiB','memory_total_MiB','temperature_C'],
                'csv':result.stdout.strip(), 'error':result.stderr.strip()}
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {'unavailable':str(exc)}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('folder', type=Path)
    ap.add_argument('--pid', required=True, type=int)
    ap.add_argument('--interval', type=float, default=5.)
    args = ap.parse_args()
    if args.interval < 1:
        ap.error('interval must be at least one second')
    process = psutil.Process(args.pid)
    folder = args.folder.resolve()
    seen, initial = set(), True
    # Exclusive creation prevents a restarted observer from overwriting earlier logs.
    with (folder/'live-progress.jsonl').open('x', encoding='utf-8') as journal, \
         (folder/'live-results.csv').open('x', encoding='utf-8', newline='') as table:
        writer = csv.DictWriter(table, fieldnames=COLUMNS); writer.writeheader()
        emit(journal, {'event':'monitor_started', 'observed_at_utc':datetime.now(timezone.utc).isoformat(),
                       'pid':args.pid, 'interval_seconds':args.interval,
                       'disclosure':'Attached during the run. Earlier resource samples cannot be recovered.'})
        print(f'Logging observations to {folder}', flush=True)
        while True:
            now = datetime.now(timezone.utc).isoformat()
            receipt = json.loads((folder/'receipt.json').read_text(encoding='utf-8'))
            count = record_cases(receipt, seen, journal, writer, now, initial)
            table.flush()
            initial = False
            resource = {'event':'resource_sample', 'observed_at_utc':now, 'receipt_status':receipt['status'],
                        'saved_cases_seen':len(seen), 'host_available_bytes':psutil.virtual_memory().available}
            try:
                cpu = process.cpu_times()
                resource.update(worker_alive=process.is_running(), worker_rss_bytes=process.memory_info().rss,
                                worker_cpu_user_seconds=cpu.user, worker_cpu_system_seconds=cpu.system,
                                worker_threads=process.num_threads())
            except psutil.NoSuchProcess:
                resource['worker_alive'] = False
            resource['gpu'] = gpu_snapshot()
            resource['pending_arrays'] = [{'name':p.name,'bytes':p.stat().st_size} for p in folder.glob('*.pending') if p.is_file()]
            emit(journal, resource)
            if count:
                print(f'{now}: recorded {count} cases; total {len(seen)}', flush=True)
            if receipt['status'] != 'RUNNING':
                emit(journal, {'event':'terminal_receipt_observed','observed_at_utc':now,'receipt':receipt})
                print(f"Terminal status: {receipt['status']}", flush=True)
                return 0
            if not resource['worker_alive']:
                # Allow the supervisor one interval to publish its terminal receipt.
                time.sleep(args.interval)
                receipt = json.loads((folder/'receipt.json').read_text(encoding='utf-8'))
                emit(journal, {'event':'worker_exit_observed','observed_at_utc':datetime.now(timezone.utc).isoformat(),
                               'receipt':receipt})
                return 0 if receipt['status'] != 'RUNNING' else 1
            time.sleep(args.interval)


if __name__ == '__main__':
    raise SystemExit(main())
