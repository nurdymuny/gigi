"""End-to-end gauge equivariance of the SU(2) forces (skeptical review, not a repo script).

grad(U) is the right-trivialised gradient assembled exactly as scripts/ricci_su2_pg_force.run.
Under U'_e = g_tail U_e g_head^-1 a covariant force satisfies grad'_e = R(g_head) grad_e,
in particular |grad'_e| = |grad_e| edge by edge. Controls: native (energy gradient, must pass);
no-register removal (expected to fail)."""
import json, sys
from pathlib import Path
import numpy as np
REPO = Path(sys.argv[1]); sys.path[:0] = [str(REPO/'scripts'), str(REPO/'e2e/probes')]
import ricci_su2_pg_force as F
from ricci_su2_pg_force import pg, tw, su, compose, inverse, rotations

def gradient(cells, faces, vid, links, trial, order=6, metric=(3., 1., 1.)):
    zeros = np.zeros(len(links)); grad = np.zeros((len(links), 3))
    for ci, cell in enumerate(cells):
        u = F.local_links(cell, links); D = F.defects(u)
        quad = F.quadrature(cell['vertices'], metric, order); h = quad['m2'] @ D
        if trial != 'native':
            fit = tw.fit_cell_rings(cells, zeros, faces, ci, rings=2)
            rec = su.recover_cell(cells, faces, vid, links, ci, rings=2, fit=fit,
                                  register=(trial != 'pg_noregister'), flux='defect')
            C = rec['coefficients']
            P = np.stack([pg.proxy_ref(dict(fit, x=C[A]), quad['local']) for A in range(3)], axis=1)
            hvec = np.einsum('qAa,qab,qib,q->iA', P, quad['g'], quad['two'], quad['w'])
            h = np.column_stack([h[:, 0], hvec])
        a_loc = 4*np.einsum('ekic,ic->ek', F.defect_derivatives(u), h)
        for e in range(6):
            g_id = cell['edges'][e]
            lift = -rotations(links[g_id]) if cell['reverse'][e] else np.eye(3)
            grad[g_id] += cell['weight']*lift.T @ a_loc[e]
    return grad

level = int(sys.argv[2]); seed = int(sys.argv[3]) if len(sys.argv) > 3 else 0
payload = json.loads((F.BERGER/f'level{level}_berger_q6.input.json').read_text())
raw = json.loads((F.BERGER/f'level{level}_berger_q6.output.json').read_text())
cells = payload['cells']; links = np.asarray(raw['links'], float)
faces = pg.face_index(cells); vid = su.vertex_ids(cells)
nv = max(vid.values())+1
rng = np.random.default_rng(seed)
q = rng.normal(size=(nv, 4)); gauge = q/np.linalg.norm(q, axis=1, keepdims=True)   # large, random rotations
links2 = su.gauge_transform(cells, vid, links, gauge)
# head vertex of every stored edge
head = {}
for ci, c in enumerate(cells):
    for e, (i, j) in enumerate(pg.EDGES):
        head.setdefault(c['edges'][e], vid[(ci, j if not c['reverse'][e] else i)])
for trial in ['native', 'pg', 'pg_noregister']:
    g1 = gradient(cells, faces, vid, links, trial); g2 = gradient(cells, faces, vid, links2, trial)
    scale = np.linalg.norm(g1)
    norm_gap = float(np.max(abs(np.linalg.norm(g2, axis=1)-np.linalg.norm(g1, axis=1)))/scale*np.sqrt(len(g1)))
    best = {}
    for name, R in [('Ad(g_head)', lambda e: rotations(gauge[head[e]])), ('Ad(g_head)^T', lambda e: rotations(gauge[head[e]]).T)]:
        best[name] = float(np.linalg.norm(np.array([R(e)@g1[e] for e in range(len(g1))])-g2)/scale)
    print(json.dumps({'level': level, 'seed': seed, 'trial': trial, 'edgewise_norm_gap_rel': norm_gap, **best}), flush=True)
