"""Assemble compatible BDM1 mass and target load, with original edge mass."""
import numpy as np
from scipy.sparse import coo_matrix, eye, kron
from ricci_variational_math import cell_masses


def assemble(payload, topo, patches, order):
    nc = len(payload['cells']); nf = len(topo['faces']); ne = len(payload['edge_points'])
    hr = []; hc = []; hv = []; qr = []; qc = []; qv = []; mr = []; mc = []; mv = []
    volume = 0.; load = np.zeros((ne, 3))
    for start in range(0, nc, 128):
        cells = payload['cells'][start:start+128]
        points = np.array([cell['vertices'] for cell in cells])
        h, m, vol, weak = cell_masses(points, order); volume += float(vol.sum())
        ids = topo['moment_ids'][start:start+len(cells)]; signs = topo['moment_signs'][start:start+len(cells)]
        hr.append(np.broadcast_to(ids[:,:,None], h.shape).ravel())
        hc.append(np.broadcast_to(ids[:,None,:], h.shape).ravel())
        hv.append((h*signs[:,:,None]*signs[:,None,:]).ravel())
        edgeids = np.array([cell['edges'] for cell in cells])
        edgesigns = np.where(np.array([cell['reverse'] for cell in cells]), -1., 1.)
        mr.append(np.broadcast_to(edgeids[:,:,None], m.shape).ravel())
        mc.append(np.broadcast_to(edgeids[:,None,:], m.shape).ravel())
        mv.append((m*edgesigns[:,:,None]*edgesigns[:,None,:]).ravel())
        np.add.at(load, edgeids.ravel(), (weak*edgesigns[:,:,None]).reshape(-1, 3))
        for offset, (faceids, recovered) in enumerate(patches[start:start+len(cells)]):
            part = (h[offset]@recovered)*signs[offset,:,None]
            qr.append(np.broadcast_to(ids[offset,:,None], part.shape).ravel())
            qc.append(np.broadcast_to(faceids[None,:], part.shape).ravel()); qv.append(part.ravel())
    build = lambda values, rows, cols, shape: coo_matrix((np.concatenate(values),
        (np.concatenate(rows), np.concatenate(cols))), shape=shape).tocsr()
    h = build(hv, hr, hc, (3*nf, 3*nf)); q = build(qv, qr, qc, (3*nf, nf))
    m = build(mv, mr, mc, (ne, ne))
    j = kron(eye(nf, format='csr'), np.ones((3, 1))/3, format='csr')
    z = kron(eye(nf, format='csr'), np.array([[1., 0.], [0., 1.], [-1., -1.]]), format='csr')
    return h, q, m, j, z, volume, load
