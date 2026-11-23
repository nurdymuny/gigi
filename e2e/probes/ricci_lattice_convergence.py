"""Measured ALL-edge RF2B operator errors. Never promotes a partial run to PASS."""
import argparse
import json
from pathlib import Path
import subprocess
import numpy as np
from scipy.linalg import expm_frechet,logm
from scipy.sparse.linalg import cg,LinearOperator
from ricci_lattice_operator import orbit_fixture
from ricci_lattice_mesh import mesh,validate
from ricci_lattice_demo import matrix,SIGMA
from ricci_wilson_oracle import phi,wilson,energy
from test_ricci_operator_native import sparse_matrix
from monodromy_evidence import sha

ROOT=Path(__file__).resolve().parents[2]
CASES={'round':[1.,1.,1.],'berger':[3.,1.,1.],'triaxial':[.5,1.,2.],'negative':[5.,1.,1.],
    'held_out':[1.3,.8,2.1]}


def reference_velocity(edge_points,g):
    x=phi(g);w=wilson(np.array(g),x);out=[]
    for p,q in edge_points:
        p=np.array(p);q=np.array(q)
        base=matrix(np.r_[p[0],-p[1:]]).conj().T@matrix(np.r_[q[0],-q[1:]])
        logarithm=logm(base)
        xi=np.array([(0.5j*np.trace(s@logarithm)).real for s in SIGMA])
        a=-.5j*np.einsum('i,ijk->jk',x*xi,SIGMA)
        da=-.5j*np.einsum('i,ijk->jk',w*xi,SIGMA)
        u,du=expm_frechet(a,da)
        out.append([(-.5j*np.trace(s@u.conj().T@du)).real for s in SIGMA])
    return np.ravel(out)


def solve(result):
    b=sparse_matrix(result);rhs=-np.ravel(result['gradient'])
    if not np.isfinite(b.data).all() or not np.isfinite(rhs).all():raise ValueError('nonfinite operator')
    asymmetry=b-b.T
    if asymmetry.nnz and np.max(np.abs(asymmetry.data))>1e-12:raise ValueError('nonsymmetric mass')
    diagonal=b.diagonal()
    if np.any(diagonal<=0):raise ValueError('nonpositive mass diagonal')
    iterations=[0]
    def tick(_):iterations[0]+=1
    velocity,status=cg(b,rhs,rtol=1e-12,atol=1e-14,maxiter=4000,
        M=LinearOperator(b.shape,matvec=lambda x:x/diagonal),callback=tick)
    absolute=np.linalg.norm(b@velocity-rhs);rhs_norm=np.linalg.norm(rhs)
    relative=absolute/max(rhs_norm,1e-14)
    if status!=0 or absolute>3*max(1e-12*rhs_norm,1e-14):raise ValueError(f'incomplete mass solve {status}: {absolute}')
    return b,velocity,{'iterations':iterations[0],'relative_linear_residual':float(relative),
        'absolute_linear_residual':float(absolute),'rhs_norm':float(rhs_norm),'solver':'scipy_cg_external_diagnostic'}


def measures(fixture,g,result):
    b,w,diagnostics=solve(result);reference=reference_velocity(fixture['edge_points'],g)
    r=w-reference;volume=result['volume']
    if volume<=0:raise ValueError('positive volume required')
    error=np.sqrt(r@(b@r)/volume);norm=np.sqrt(reference@(b@reference)/volume)
    reference_energy=energy(np.array(g),np.diag(phi(g)))
    return {'full_mass_error':float(error),'reference_mass_norm':float(norm),
        'relative_error':float(error/norm) if norm>1e-12 else None,
        'energy_density':result['energy']/volume,'continuum_energy_density':float(reference_energy),
        'energy_relative_error':float(abs(result['energy']/volume-reference_energy)/reference_energy) if reference_energy else None,
        'quotient_degrees_of_freedom':len(reference),'cover_degrees_of_freedom':120*len(reference),
        'volume':volume,'diagnostics':diagnostics}


def run(binary,output,max_level,orders):
    output.mkdir(parents=True,exist_ok=True);(output/'gate.PASS.json').unlink(missing_ok=True)
    receipt={'schema':'rf2b_operator_diagnostic_v1','status':'IN_PROGRESS','gate':'OPEN',
        'binary':{'path':str(binary),'sha256':sha(binary)},
        'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'continuum_convergence_proven':False,'cases':{},'time_scale':1,'fitted_constants':[]}
    files=list((ROOT/'src/gauge').glob('ricci_*.rs'))+list((ROOT/'e2e/probes').glob('*ricci*lattice*.py'))
    files+=[Path(__file__),ROOT/'examples/ricci_operator_probe.rs',ROOT/'e2e/probes/test_ricci_operator_native.py',ROOT/'docs/RICCI_LATTICE_CORRESPONDENCE_SPEC.md']
    receipt['sources']={str(p.resolve()):sha(p) for p in files}
    try:
        for level in range(max_level+1):
            fixture=orbit_fixture(level); geometry=validate(mesh(level))
            for name,g in CASES.items():
                for order in orders:
                    stem=f'level{level}_{name}_q{order}';a=output/(stem+'.input.json');b=output/(stem+'.output.json')
                    payload={key:fixture[key] for key in ['edge_points','cells']};payload.update(metric=g,order=order)
                    a.write_text(json.dumps(payload,allow_nan=False))
                    subprocess.run([str(binary),str(a),str(b)],check=True,timeout=1200)
                    result=json.loads(b.read_text());numbers=measures(fixture,g,result)
                    receipt['cases'][stem]={'metric':g,'level':level,'quadrature_order':order,'mesh':geometry,
                        'measures':numbers,'input_sha256':sha(a),'output_sha256':sha(b)}
                    print(stem,numbers,flush=True)
                    (output/'diagnostic.json').write_text(json.dumps(receipt,indent=2,allow_nan=False))
        receipt['status']='DIAGNOSTIC_COMPLETE_FULL_GATE_OPEN'
    except Exception as e:
        receipt['status']='FAILED';receipt['error']=str(e);raise
    finally:
        (output/'diagnostic.json').write_text(json.dumps(receipt,indent=2,allow_nan=False))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--max-level',type=int,choices=range(4),default=1);p.add_argument('--orders',nargs='+',type=int,choices=range(2,7),default=[4,6])
    a=p.parse_args();run(a.binary.resolve(),a.output.resolve(),a.max_level,a.orders)
