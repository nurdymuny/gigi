"""Two fixed-control local interface checks, not a new spectral calculation."""
import argparse
import json
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time

import numpy as np
import scipy
import sympy
import helike_berger_model as bm
import helike_qspace_boundary as qb
from test_helike_qspace_boundary import probes
from run_helike_flux_background import save_receipt, sha

ROOT=Path(__file__).resolve().parent.parent
SOURCES=[f'scripts/{name}.py' for name in (
    'helike_qspace_boundary','helike_berger_model','ym_two_grain_molien',
    'helike_condensate_pt','helike_condensate_fock','helike_positive_cone',
    'test_helike_qspace_boundary','check_qspace_boundary_mutations',
    'run_helike_qspace_boundary','run_helike_flux_background')]
SOURCES += ['docs/research/helike-qspace-boundary-spec-2026-11-24.md',
            'docs/research/helike-quantum-probe-preregistration-2026-11-23.md']


def measure_control(g2):
    s,t,g=1.15,3.,np.sqrt(g2)
    _,_,background=bm.mc_background(s,t,g)
    interface=qb.coordinates(background); interface[2]=0.
    checks=[]; max_lap_error=0.; max_H_error=0.
    regular_probes=probes(1/np.diag(background)**2)
    for w in (-.2,0.,.2):
        u=interface.copy(); u[2]=w
        L=qb.factor(u); a,b=bm.mc_sources(s,t)
        for k,(cart,jets) in enumerate(regular_probes):
            value,lap=cart(*L.ravel()); fu,gradient,hessian=jets(*qb.chart_values(u))
            reduced=qb.laplacian(u,gradient,hessian)
            direct_H=-.5*lap+bm.V_sourced(L,g,t,a,b)*value
            reduced_H=-.5*reduced+qb.potential(u,s,t,g)*fu
            lap_error=float(abs(lap-reduced)); H_error=float(abs(direct_H-reduced_H))
            max_lap_error=max(max_lap_error,lap_error); max_H_error=max(max_H_error,H_error)
            if lap_error>1e-9*(1+abs(lap)) or H_error>1e-9*(1+abs(direct_H)):
                raise AssertionError('Cartesian/reduced operator disagreement')
            row=dict(w=w,probe=k,value=float(value),cartesian_laplacian=float(lap),
                     reduced_laplacian=float(reduced.real),laplacian_error=lap_error,
                     cartesian_H=float(direct_H),reduced_H=float(reduced_H.real),H_error=H_error)
            if w==0.:
                gradient=np.asarray(gradient); minus=gradient.copy(); minus[2]*=-1
                row['matching']=qb.matching_report(u,fu,fu,gradient,minus)
                if not row['matching']['matches']: raise AssertionError('smooth Cartesian probe failed matching')
            checks.append(row)
    up=interface.copy(); up[2]=.2
    um=up.copy(); um[2]=-.2
    r0,r1,w,*_=qb.chart_values(up)
    difference=qb.potential(up,s,t,g)-qb.potential(um,s,t,g)
    expected=-2*g*bm.omega(t).sum()*r0*r1*w
    if abs(difference-expected)>1e-10: raise AssertionError('signed cubic changed')
    # Normalized determinant coefficient keeps this *probe* well scaled at the fixed background.
    # This is not a change to g, the potential, or any acceptance threshold.
    value=np.exp(-1.); gradient=np.zeros(6,complex); gradient[2]=1j*value
    minus=gradient.copy(); minus[2]*=-1
    current=np.imag(np.conjugate(value)*(qb.metric(interface)[2]@gradient))
    if not np.isclose(current,np.exp(-2.),atol=1e-12,rtol=0):
        raise AssertionError('planted smooth complex probe current not reproduced')
    return dict(s=s,t=t,g2=g2,interface=interface.tolist(),
                metric_eigenvalues=np.linalg.eigvalsh(qb.metric(interface)).tolist(),
                density=qb.density(interface),max_laplacian_error=max_lap_error,max_H_error=max_H_error,
                signed_potential_difference=difference,expected_signed_difference=expected,
                complex_probe_normal_current_per_density=float(current),
                complex_probe_matching=qb.matching_report(interface,value,value,gradient,minus),checks=checks)


def main():
    ap=argparse.ArgumentParser(description=__doc__); ap.add_argument('out_dir'); args=ap.parse_args()
    out=Path(args.out_dir)
    if out.exists() and any(out.iterdir()): raise ValueError('refusing a nonempty evidence directory')
    out.mkdir(parents=True,exist_ok=True)
    receipt=dict(status='RUNNING',scope='local rank-two Cartesian matching at fixed controls',
                 physical_spectrum_certified=False,registered_grid_released=False,
                 source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                 source_dirty=bool(subprocess.check_output(['git','status','--porcelain','--',*SOURCES],cwd=ROOT,text=True).strip()),
                 source_hash_policy='sha256-universal-newline-v1',sources={s:sha(ROOT/s) for s in SOURCES},
                 python=sys.version,platform=platform.platform(),
                 libraries=dict(numpy=np.__version__,scipy=scipy.__version__,sympy=sympy.__version__),
                 matching_tolerance=qb.MATCH_TOLERANCE,controls=[])
    path=out/'receipt.json'; save_receipt(path,receipt)
    try:
        for source in SOURCES:
            dest=out/'sources'/source; dest.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(ROOT/source,dest)
            if sha(dest)!=receipt['sources'][source]: raise ValueError(f'source changed during snapshot: {source}')
        for g2 in (.004,.002):
            start=time.monotonic(); row=measure_control(g2); row['seconds']=time.monotonic()-start
            receipt['controls'].append(row); save_receipt(path,receipt)
            print(json.dumps({k:v for k,v in row.items() if k!='checks'}),flush=True)
        receipt['status']='PASS_LOCAL_MATCHING'
    except BaseException as exc:
        receipt.update(status='ERROR',error=f'{type(exc).__name__}: {exc}')
        save_receipt(path,receipt)
        raise
    save_receipt(path,receipt)
    print('PASS_LOCAL_MATCHING; no boundary-complete spectrum or grid release',flush=True)


if __name__=='__main__': main()
