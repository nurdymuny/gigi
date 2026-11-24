"""Frozen four-state-file diagnosis; no eigensolver and no full-grid release."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time
from types import SimpleNamespace
import numpy as np
import torch
import helike_residual_moments as rm
import helike_positive_cone as pc
import helike_quasimode as qm
from run_helike_flux_background import sha,save_receipt
from run_helike_quasimode import SOURCES as BASE_SOURCES,utc,event

ROOT=Path(__file__).resolve().parents[1]
OLD=ROOT/'research/quasimode-construction-20261124/controls-e21087a1'
EXPECTED='e922d6bfffea50907d1c28d7cf7cdae622bb8eda0da5e4f97fdc1c13807045ab'
SOURCES=tuple(BASE_SOURCES)+('scripts/helike_residual_moments.py','scripts/run_helike_residual_moments.py',
    'scripts/test_helike_residual_moments.py','scripts/check_helike_residual_mutations.py',
    'docs/research/helike-residual-integration-registration-2026-11-24.md')


def load_controls(folder):
    folder=Path(folder); path=folder/'receipt.json'
    if sha(path)!=EXPECTED or json.loads((folder/'receipt-integrity.json').read_text())['sha256']!=EXPECTED:
        raise ValueError('parent receipt does not match frozen measurement')
    receipt=json.loads(path.read_text())
    if (receipt['execution']!='COMPLETE_NUMERICAL_AUDIT' or receipt['certified'] or receipt['full_grid_released']
            or receipt['source_dirty'] or receipt['status']!='UNRESOLVED_INTEGRATION'):
        raise ValueError('invalid parent status')
    rows=receipt['controls']
    if len(rows)!=4 or {(r['g2'],r['Mq']) for r in rows}!={(.004,6),(.004,8),(.002,6),(.002,8)}:
        raise ValueError('control coverage differs from registration')
    for source,digest in receipt['sources'].items():
        if sha(folder/'sources'/source)!=digest: raise ValueError('parent source snapshot mismatch')
    for row in rows:
        if row['s']!=1.15 or row['t']!=3.: raise ValueError('model parameters changed')
        state=folder/row['state_file']
        if hashlib.sha256(state.read_bytes()).hexdigest()!=row['state_sha256']:
            raise ValueError('saved state hash mismatch')
        with np.load(state,allow_pickle=False) as saved:
            for key in saved.files: qm.finite(saved[key])
            if not np.array_equal(saved['centers'],row['centers']): raise ValueError('spectral centers changed')
            if not np.allclose(saved['coeff'].conj().T@saved['coeff'],np.eye(4),rtol=0,atol=1e-12):
                raise ValueError('saved coefficients not orthonormal')
            if not np.allclose(saved['mixing'].conj().T@saved['mixing'],np.eye(4),rtol=0,atol=1e-12):
                raise ValueError('saved mixing is not unitary')
    return receipt


def point_checks(space,groups,state,row):
    chart=state.chart; base=chart.factor(np.zeros(6))
    points=[base,chart.factor(np.array([.1,-.1,.05,.07,-.09,.04])),
            chart.factor(np.array([-.12,.08,.09,-.06,.1,-.03])),base*(.015**(1/6)),base*np.sqrt(6.)]
    errors=[]
    for q in points:
        z=chart.inverse(q@q.T)
        f=np.exp(.25*np.log(state.w/np.pi).sum()-.5*(state.w*z*z).sum()-.5*chart.log_measure(z))
        expanded=rm.evaluate(space,groups,z[None,:])[0]*f
        for j in range(4):
            psi=complex(qm.physical_z(torch.tensor(z),chart,state.basis,state.w,state.coeff[:,j]).item())
            oracle=qm.cartesian_action(q,chart,state.basis,state.w,state.coeff[:,j],row['s'],row['t'],np.sqrt(row['g2']),False)
            target=oracle-state.centers[j]*psi; error=abs(expanded[j]-target)
            if not np.isfinite(error) or error>1e-9*(1+abs(target)): raise ArithmeticError('expanded operator differs from Cartesian oracle')
            errors.append(float(error))
    return errors


def main():
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument('out_dir'); args=parser.parse_args()
    out=Path(args.out_dir)
    if out.exists() and any(out.iterdir()): raise ValueError('refusing nonempty evidence folder')
    out.mkdir(parents=True,exist_ok=True)
    report=dict(execution='RUNNING',status='UNRESOLVED_INTEGRATION',certified=False,full_grid_released=False,
        source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        source_dirty=bool(subprocess.check_output(['git','status','--porcelain','--',*SOURCES],cwd=ROOT,text=True).strip()),
        sources={s:sha(ROOT/s) for s in SOURCES},parent_receipt_sha256=EXPECTED,controls=[],started_utc=utc())
    receipt=out/'receipt.json'; save_receipt(receipt,report)
    torch.set_num_threads(1)
    try:
        if report['source_dirty']: raise ValueError('commit frozen integration correction before replay')
        parent=load_controls(OLD)
        for source in SOURCES:
            dest=out/'sources'/source; dest.parent.mkdir(parents=True,exist_ok=True); shutil.copyfile(ROOT/source,dest)
            if sha(dest)!=report['sources'][source]: raise ValueError('source changed during snapshot')
        for row in parent['controls']:
            start=time.monotonic(); deadline=start+900
            event(out,'control_start',g2=row['g2'],Mq=row['Mq'])
            with np.load(OLD/row['state_file'],allow_pickle=False) as saved:
                chart=pc.ConeChart(saved['G0'],saved['M'])
                state=SimpleNamespace(chart=chart,basis=[tuple(n) for n in saved['basis']],w=saved['w'].copy(),
                                      coeff=saved['coeff']@saved['mixing'],centers=saved['centers'].copy())
            space,groups=rm.build_residual(chart,state.basis,state.w,state.coeff,state.centers,row['s'],row['t'],np.sqrt(row['g2']))
            current=dict(g2=row['g2'],Mq=row['Mq'],state_sha256=row['state_sha256'],group_count=len(groups),
                         workspace_degree=space.degree,old_replay={},old_replay_checks={})
            report['controls'].append(current); save_receipt(receipt,report)
            current['point_errors']=point_checks(space,groups,state,row)
            event(out,'point_checks',g2=row['g2'],Mq=row['Mq'],count=len(current['point_errors']),largest=max(current['point_errors']))
            for oracle,label in ((False,'moments'),(True,'independent_moments')):
                event(out,'moments_start',g2=row['g2'],Mq=row['Mq'],method=label)
                current[label]=rm.norm_squared(space,groups,oracle,deadline)
                save_receipt(receipt,report)
                event(out,'moments_complete',g2=row['g2'],Mq=row['Mq'],method=label,result=current[label])
            current['moment_check']=rm.numerical_result(current['moments']['value'],current['independent_moments']['value'])
            if not current['moment_check']['agreement']: raise ArithmeticError('independent moments disagree')
            names=('ground','helike_plus','helike_minus','competitor')
            for order in (10,12):
                event(out,'old_rule_replay_start',g2=row['g2'],Mq=row['Mq'],order=order)
                values=rm.tensor_norm(space,groups,order,deadline)
                reference=[row['integrals'][str(order)][name]['residual_sq'] for name in names]
                current['old_replay'][str(order)]=values
                current['old_replay_checks'][str(order)]=rm.numerical_result(values,reference)
                current.setdefault('old_errors_vs_moments',{})[str(order)]=(np.array(reference)-current['moments']['value']).tolist()
                save_receipt(receipt,report)
                event(out,'old_rule_replay_complete',g2=row['g2'],Mq=row['Mq'],order=order,values=values,
                      agreement=current['old_replay_checks'][str(order)]['agreement'])
                if not current['old_replay_checks'][str(order)]['agreement']: raise ArithmeticError('old rule replay differs from saved residual')
            if time.monotonic()>deadline: raise TimeoutError('control budget exhausted')
            current['seconds']=time.monotonic()-start; save_receipt(receipt,report)
            event(out,'control_complete',g2=row['g2'],Mq=row['Mq'],seconds=current['seconds'])
        report.update(execution='COMPLETE_DIAGNOSIS',numerical_diagnosis_passed=True,finished_utc=utc())
        save_receipt(receipt,report); event(out,'complete',status=report['status'],full_grid_released=False)
        return 0
    except (Exception,KeyboardInterrupt) as exc:
        report.update(execution='FAILED',numerical_diagnosis_passed=False,error=f'{type(exc).__name__}: {exc}',finished_utc=utc())
        save_receipt(receipt,report); event(out,'refusal',error=report['error'],full_grid_released=False)
        return 2


if __name__=='__main__': sys.exit(main())
