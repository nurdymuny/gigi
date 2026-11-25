"""Same four saved controls; no model solve, no full-grid release."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time
import numpy as np
import flint
from flint import arb,ctx
import helike_cutoff_bounds as cb
import helike_positive_cone as pc
from run_helike_residual_moments import OLD,ROOT,EXPECTED,load_controls
from run_helike_flux_background import sha,save_receipt
from run_helike_quasimode import SOURCES as BASE_SOURCES,utc,event

REG='docs/research/helike-cutoff-tail-bounds-registration-2026-11-24.md'
SOURCES=tuple(BASE_SOURCES)+(REG,'scripts/helike_cutoff_bounds.py','scripts/run_helike_cutoff_bounds.py',
    'scripts/test_helike_cutoff_bounds.py','scripts/check_helike_cutoff_mutations.py','scripts/run_helike_residual_moments.py')
NAMES=('ground','helike_plus','helike_minus','competitor')
MODEL=('scripts/helike_berger_model.py','scripts/helike_positive_cone.py',
       'scripts/helike_condensate_pt.py','scripts/helike_condensate_fock.py','scripts/helike_quasimode.py')


def decode(endpoint):
    if set(endpoint)!={'mantissa','exponent'} or type(endpoint['exponent']) is not int:
        raise ValueError('invalid dyadic endpoint')
    value=arb((int(endpoint['mantissa']),endpoint['exponent']))
    if not value.is_exact() or not value.is_finite(): raise ValueError('inexact or nonfinite serialized endpoint')
    return value


def finalize(report):
    controls=report['controls']
    if len(controls)!=4 or {(r['g2'],r['Mq']) for r in controls}!={(g,m) for g in (.004,.002) for m in (6,8)}:
        raise ValueError('incomplete or duplicated control coverage')
    flags=[]
    for control in controls:
        if set(control['states'])!=set(NAMES): raise ValueError('incomplete state coverage')
        for state in control['states'].values():
            if (state['status']!='UNRESOLVED_INTEGRATION' or state['certified'] or state['full_grid_released']
                    or state['method']!='analytic_gaussian_majorant_arb128'
                    or state['rigorous_scope']!='saved_trial_cutoff_tail_and_normalization_only'):
                raise ValueError('invalid bound scope')
            lo=decode(state['norm_sq']['lower']); hi=decode(state['norm_sq']['upper'])
            if not 0<lo<=hi: raise ValueError('invalid normalization interval')
            for key in ('tail_sq','leakage_sq'):
                if decode(state[key]['lower'])!=0: raise ValueError('majorant enclosure must begin at zero')
            passed=cb.classify(decode(state['tail_sq']['upper']),decode(state['leakage_sq']['upper']),lo.union(hi))['cutoff_tail_gate_passed']
            if state['cutoff_tail_gate_passed'] is not passed: raise ValueError('acceptance flag disagrees with bounds')
            flags.append(passed)
    report.update(execution='COMPLETE_CUTOFF_TAIL_BOUNDS',cutoff_tail_gate_passed=all(flags),
                  status='UNRESOLVED_INTEGRATION',certified=False,full_grid_released=False)


def failure(report,error):
    report.update(execution='FAILED',cutoff_tail_gate_passed=False,status='UNRESOLVED_INTEGRATION',
                  certified=False,full_grid_released=False,error=f'{type(error).__name__}: {error}')


def main():
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument('out_dir'); args=parser.parse_args()
    out=Path(args.out_dir)
    if out.exists() and any(out.iterdir()): raise ValueError('refusing nonempty evidence directory')
    out.mkdir(parents=True,exist_ok=True); ctx.prec=128
    report=dict(execution='RUNNING',cutoff_tail_gate_passed=False,status='UNRESOLVED_INTEGRATION',
        certified=False,full_grid_released=False,source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        source_dirty=bool(subprocess.check_output(['git','status','--porcelain','--',*SOURCES],cwd=ROOT,text=True).strip()),
        sources={s:sha(ROOT/s) for s in SOURCES},parent_receipt_sha256=EXPECTED,controls=[],started_utc=utc(),
        python=sys.version,platform=platform.platform(),numpy=np.__version__,python_flint=flint.__version__,precision_bits=ctx.prec,
        design=dict(s=1.15,t=3.,g2=[.004,.002],Mq=[6,8],delta=.01,R_factor=4.,epsilon='1/16',
                    radius_candidates='1/4 through 64/4',leakage_norm_limit='1e-10',squared_width_limit='1e-12',norm_width_limit='1e-10'),
        missing_gates=['rigorous_residual_and_charge_norms','projected_quasimode_bound','separation_and_target_identification'])
    path=out/'receipt.json'; save_receipt(path,report)
    try:
        if report['source_dirty']: raise ValueError('commit frozen bounds before control measurement')
        if flint.__version__!='0.9.0': raise ValueError('unregistered ball arithmetic version')
        parent=load_controls(OLD)
        for source in MODEL:
            if sha(ROOT/source)!=parent['sources'][source]: raise ValueError('original model or cutoff source changed: '+source)
        for source in SOURCES:
            dest=out/'sources'/source; dest.parent.mkdir(parents=True,exist_ok=True); shutil.copyfile(ROOT/source,dest)
            if sha(dest)!=report['sources'][source]: raise ValueError('source changed during snapshot')
        for row in parent['controls']:
            start=time.monotonic(); event(out,'control_start',g2=row['g2'],Mq=row['Mq'])
            with np.load(OLD/row['state_file'],allow_pickle=False) as saved:
                chart=pc.ConeChart(saved['G0'],saved['M'])
                if not np.array_equal(chart.S,saved['S']): raise ValueError('chart reconstruction changed saved S')
                u0=chart.u0; S=saved['S'].copy(); w=saved['w'].copy(); G0=saved['G0'].copy()
                C=saved['coeff']@saved['mixing']; basis=[tuple(n) for n in saved['basis']]
                filename=f"trial-g2-{row['g2']}-Mq-{row['Mq']}.npz"
                np.savez_compressed(out/filename,u0=u0,S=S,w=w,G0=G0,coeff=C,basis=saved['basis'],centers=saved['centers'])
            if sum(a.nbytes for a in (C,S,w,G0,u0))+np.asarray(basis).nbytes>12*1024**3:
                raise MemoryError('registered array budget exhausted')
            current=dict(g2=row['g2'],Mq=row['Mq'],state_sha256=row['state_sha256'],trial_file=filename,
                trial_sha256=hashlib.sha256((out/filename).read_bytes()).hexdigest(),states={})
            report['controls'].append(current); save_receipt(path,report)
            p=cb.plateau(u0,S,w,G0)
            event(out,'plateau_proved',g2=row['g2'],Mq=row['Mq'],radius=str(p['rho']),
                  determinant_lower=str(p['det_lower']),norm_upper=str(p['norm_upper']))
            metric_count=sum(len(entry) for line in pc._symbolic_metric()[2] for entry in line)
            if metric_count*len(NAMES)>100000: raise ValueError('per-control metric contribution budget exceeded')
            current['metric_contributions']=metric_count*len(NAMES)
            for j,name in enumerate(NAMES):
                result=cb.bound_state(u0,S,w,G0,basis,C[:,j],p); current['states'][name]=result
                if time.monotonic()-start>900: raise TimeoutError('registered per-control time budget exhausted')
                save_receipt(path,report)
                event(out,'state_bounded',g2=row['g2'],Mq=row['Mq'],state=name,
                      tail_norm_upper=result['tail_norm']['upper_display'],leakage_norm_upper=result['leakage_norm']['upper_display'],
                      cutoff_tail_gate_passed=result['cutoff_tail_gate_passed'],full_grid_released=False)
            current['seconds']=time.monotonic()-start; save_receipt(path,report)
            event(out,'control_complete',g2=row['g2'],Mq=row['Mq'],seconds=current['seconds'])
        for source in SOURCES:
            if sha(ROOT/source)!=report['sources'][source]: raise ValueError('source changed during bound run')
        finalize(report); report['finished_utc']=utc(); save_receipt(path,report)
        event(out,'complete',cutoff_tail_gate_passed=report['cutoff_tail_gate_passed'],status=report['status'],full_grid_released=False)
        return 0 if report['cutoff_tail_gate_passed'] else 3
    except (Exception,KeyboardInterrupt) as exc:
        failure(report,exc); report['finished_utc']=utc(); save_receipt(path,report)
        event(out,'refusal',error=report['error'],full_grid_released=False)
        return 2


if __name__=='__main__': sys.exit(main())
