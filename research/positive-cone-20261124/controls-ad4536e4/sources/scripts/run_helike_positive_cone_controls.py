"""Only the two existing weak-coupling controls; never the registered flux grid."""
import argparse
import json
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time

import numpy as np
import helike_positive_cone as pc
import helike_berger_model as bm
from run_helike_flux_background import save_receipt, sha

ROOT = Path(__file__).resolve().parent.parent
SOURCES = [f'scripts/{s}.py' for s in ('helike_positive_cone','helike_condensate_pt',
    'helike_condensate_fock','helike_berger_model','ym_two_grain_molien',
    'run_helike_flux_background','run_helike_positive_cone_controls','test_helike_positive_cone')]
SOURCES.append('docs/research/helike-positive-cone-basis-spec-2026-11-24.md')


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('out_dir')
    ap.add_argument('--cutoff',type=int,choices=(4,6),default=6)
    args=ap.parse_args()
    out=Path(args.out_dir)
    if out.exists() and any(out.iterdir()):
        raise ValueError('refusing a nonempty evidence directory')
    out.mkdir(parents=True,exist_ok=True)
    rec=dict(status='RUNNING',scope='interior trial controls, no physical boundary certificate',
             source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
             source_dirty=bool(subprocess.check_output(['git','status','--porcelain','--',*SOURCES],cwd=ROOT,text=True).strip()),
             source_hash_policy='sha256-universal-newline-v1',
             sources={s:sha(ROOT/s) for s in SOURCES},python=sys.version,platform=platform.platform(),
             controls=[],cutoff=args.cutoff,thresholds=dict(coefficient_error=.03,weight_min=.95),certified=False)
    path=out/'receipt.json'
    save_receipt(path,rec)
    try:
        for s in SOURCES:
            dest=out/'sources'/s
            dest.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(ROOT/s,dest)
            if sha(dest)!=rec['sources'][s]: raise ValueError(f'source changed during snapshot: {s}')
        for g2 in (.004,.002):
            start=time.monotonic()
            L=pc.ConeRitz(1.15,3.,np.sqrt(g2),Mq=args.cutoff)
            result=L.helike_and_competitor()
            wh=np.sqrt(bm.mc_helike_freq2(1.15,3.))
            wc=min(w for w in L.w if np.count_nonzero(abs(L.w-w)<1e-6)==1)
            coeffs=[(result['helike']['gap']-wh)/g2,(result['competitor']['gap']-wc)/g2]
            eigenvalues,eigenvectors=L.spectrum(len(L.basis))
            matrix=out/f'control-{g2}.npz'
            np.savez_compressed(matrix,H=L.H,T=L.T,mass=L.mass,eigenvalues=eigenvalues,eigenvectors=eigenvectors)
            # Binary matrices have ordinary byte hashes, not the text newline policy.
            import hashlib
            row=dict(g2=g2,s=1.15,t=3.,Mq=args.cutoff,dimension=len(L.basis),result=result,
                     coefficients=coeffs,seconds=time.monotonic()-start,
                     mass_error=float(np.max(abs(L.mass-np.eye(len(L.basis))))),
                     kinetic_min=float(np.linalg.eigvalsh(L.T).min()),asymmetry=L.asym,
                     matrix_file=matrix.name,matrix_sha256=hashlib.sha256(matrix.read_bytes()).hexdigest())
            rec['controls'].append(row)
            save_receipt(path,rec)
            print(json.dumps(row),flush=True)
        extrap=2*np.array(rec['controls'][1]['coefficients'])-rec['controls'][0]['coefficients']
        rec['extrapolated_coefficients']=extrap.tolist()
        rec['coefficient_errors']=abs(extrap-np.array([-.5300,-2.7379])).tolist()
        rec['coefficient_pass']=bool(max(rec['coefficient_errors']) <= .03)
        rec['weight_pass']=all(row['result'][n]['weight']>.95 for row in rec['controls'] for n in ('helike','competitor'))
        rec['status']='PASS_CONTROLS' if rec['coefficient_pass'] and rec['weight_pass'] else 'FAIL_CONTROLS'
    except BaseException as exc:
        rec.update(status='ERROR',error=f'{type(exc).__name__}: {exc}')
        save_receipt(path,rec)
        raise
    save_receipt(path,rec)
    print(json.dumps({k:v for k,v in rec.items() if k not in ('controls','sources')},indent=2))
    return 0 if rec['status']=='PASS_CONTROLS' else 1


if __name__=='__main__':
    sys.exit(main())
