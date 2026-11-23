import json, time, sys, numpy as np
sys.path.insert(0, '$SCRATCH/ricci/e2e/probes')
import ricci_rf2h as g
from scipy.sparse import load_npz
from scipy.sparse.linalg import splu
R='$SCRATCH/ricci/output/'
VAR=R+'ricci-variational-20261117/abelian/'
for L in [int(x) for x in sys.argv[1].split(',')]:
    p = json.load(open(R+f'ricci-shape-regular-20261117/berger/level{L}_berger_q6.input.json'))
    b = g.build_recovery(p, order=6, verbose=True); r=b['receipt']
    D=load_npz(VAR+f'level{L}_q6.d.npz'); H=load_npz(VAR+f'level{L}_q6.h.npz'); M=load_npz(VAR+f'level{L}_q6.m.npz').tocsc()
    a=np.load(VAR+f'level{L}_q6.arrays.npz')['a'][:,0]; vol=json.load(open(VAR+'diagnostic.json'))['cases'][f'level{L}_q6']['volume']
    lu=splu(M); nrm=lambda x: float(np.sqrt(max(0,x@(M@x))/vol))
    v=lu.solve(-g.gradient(b['C'],H,D,a))
    print('RESULT', L, 'rel', round(nrm(v+4*a)/nrm(4*a),5), {k:r[k] for k in ('acc_relative_residual','adj_relative_residual','phi_max_abs')}, r['solve'], flush=True)
