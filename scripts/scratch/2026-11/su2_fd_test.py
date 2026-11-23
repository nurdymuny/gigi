"""Finite-difference step sensitivity of the S2b spectrum (skeptical review, not a repo script)."""
import json, sys
from pathlib import Path
import numpy as np
from scipy.linalg import eig
REPO = Path(sys.argv[1]); sys.path[:0] = [str(REPO/'scripts'), str(REPO/'e2e/probes')]
import ricci_su2_pg_stability as S2
from test_ricci_operator_native import sparse_matrix
level = int(sys.argv[2]); trials = sys.argv[3].split(','); eps_list = [float(x) for x in sys.argv[4].split(',')]
for trial in trials:
    S = S2.Setup(level, trial); B = sparse_matrix(S.raw).toarray(); N = 3*S.n
    for eps in eps_list:
        S2.EPS_COL = eps
        K = np.zeros((N, N))
        for g in range(S.n):
            for k in range(3):
                K[:, 3*g+k] = S.column(g, k)
        lam = eig(K, B, right=False); phys = lam[abs(lam) >= 1]
        neg = np.sort(phys.real[phys.real < 0])[:5]
        print(json.dumps({'level': level, 'trial': trial, 'eps': eps,
            'small_lt_1': int((abs(lam) < 1).sum()), 'small_lt_0.1': int((abs(lam) < .1).sum()),
            'band_0.5_to_2': int(((abs(lam) > .5) & (abs(lam) < 2)).sum()),
            'rough': int((lam.real < -10).sum()), 'neg_physical': int((phys.real < 0).sum()),
            'five_most_negative': [round(float(x), 4) for x in neg],
            'max_abs_imag': round(float(abs(lam.imag).max()), 2), 'max_real': round(float(lam.real.max()), 1)}), flush=True)
