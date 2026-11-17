"""RF2F signed quotient faces and cover-local polynomial patches."""
import itertools
from functools import lru_cache
import numpy as np
from scipy.spatial import cKDTree
from scipy.sparse import coo_matrix
from monodromy_exact_oracle import exact_closure, S, T
from ricci_lattice_mesh import plus, qfloat
from ricci_variational_math import FACES, EDGES, fit_patch, moment_matrix


def inverse(q): return np.asarray(q)*np.array([1., -1., -1., -1.])


@lru_cache(maxsize=1)
def deck():
    values = np.array([qfloat(q) for q in exact_closure([S, T])]); tree = cKDTree(values)
    distance, products = tree.query(plus(values[:,None,:], values[None,:,:]).reshape(-1, 4))
    if distance.max() > 2e-12: raise ValueError('deck multiplication closure failed')
    _, inverses = tree.query(inverse(values))
    return values, tree, products.reshape(120, 120), inverses, int(tree.query([1., 0., 0., 0.])[1])


def topology(payload):
    values, tree, _, _, _ = deck()
    faces = []; lookup = {}; owners = []; cell_faces = []; moment_ids = []; moment_signs = []
    edge_count = len(payload['edge_points']); dr = []; dc = []; dv = []
    for ci, cell in enumerate(payload['cells']):
        if abs(cell['weight']-1.) > 1e-14:
            raise ValueError('RF2F requires the frozen single-order unit-weight quotient cells')
        points = np.array(cell['vertices']); edges = {}
        for index, (a, b) in enumerate(EDGES):
            signed = (cell['edges'][index]+1)*(-1 if cell['reverse'][index] else 1)
            edges[a,b] = signed; edges[b,a] = -signed
        cf = []; mi = []; ms = []
        for local_face, verts in enumerate(FACES):
            choices = [(tuple(edges[a,b] for a,b in [(p[0],p[1]), (p[0],p[2]), (p[1],p[2])]), p)
                       for p in itertools.permutations(verts)]
            key, ordered = min(choices)
            parity = (-1)**sum(ordered[i] > ordered[j] for i in range(3) for j in range(i+1,3))
            canonical_points = points[list(ordered)]
            if key not in lookup:
                fi = len(faces); lookup[key] = fi; faces.append(canonical_points); owners.append([])
                for signed, incidence in zip(key, [1, -1, 1]):
                    dr.append(fi); dc.append(abs(signed)-1); dv.append(incidence*np.sign(signed))
            fi = lookup[key]
            transformation = plus(canonical_points[0], inverse(faces[fi][0]))
            distance, gi = tree.query(transformation)
            if distance > 3e-12 or np.max(abs(plus(values[gi], faces[fi])-canonical_points)) > 3e-12:
                raise ValueError('edge-signature face collision or non-deck identification')
            owners[fi].append((ci, local_face, int(gi)))
            cf.append((fi, int(gi)))
            for vertex in verts:
                mi.append(3*fi+ordered.index(vertex)); ms.append(parity)
        cell_faces.append(cf); moment_ids.append(mi); moment_signs.append(ms)
    if any(len(row) != 2 for row in owners): raise ValueError('each quotient face must have two owners')
    incidence = coo_matrix((dv, (dr, dc)), shape=(len(faces), edge_count)).tocsr()
    return {'faces': np.array(faces), 'owners': owners, 'cell_faces': cell_faces,
            'moment_ids': np.array(moment_ids), 'moment_signs': np.array(moment_signs), 'incidence': incidence}


def patch_for_cell(payload, topo, ci):
    values, _, products, inverses, identity = deck()
    inv_chart = np.linalg.inv(np.array(payload['cells'][ci]['vertices']).T)
    central = [(fi, gi) for fi, gi in topo['cell_faces'][ci]]
    visited = {(ci, identity)}; frontier = set(visited); last_error = None
    for radius in [1, 2, 3]:
        fresh = set()
        for current, g in sorted(frontier):
            for fi, own in topo['cell_faces'][current]:
                image = products[g, own]
                for neighbor, _, other in topo['owners'][fi]:
                    state = (neighbor, int(products[image, inverses[other]]))
                    if state not in visited: fresh.add(state)
        visited.update(fresh); frontier = fresh
        face_images = set()
        for current, g in visited:
            face_images.update((fi, int(products[g, own])) for fi, own in topo['cell_faces'][current])
        ordered = central+sorted(face_images-set(central))
        coordinates = plus(values[np.array([g for _,g in ordered])][:,None,:],
                           topo['faces'][np.array([fi for fi,_ in ordered])])
        homogeneous = coordinates@inv_chart.T
        denominator = homogeneous.sum(axis=2)
        if np.min(denominator) <= 1e-8: raise ValueError('patch extends outside declared chart')
        triangles = homogeneous[:,:,1:]/denominator[:,:,None]
        try:
            coefficient_map, diagnostics = fit_patch(triangles)
            diagnostics['radius'] = radius
            return np.array([fi for fi,_ in ordered]), moment_matrix()@coefficient_map, diagnostics
        except ValueError as error:
            last_error = error
    raise ValueError('no admissible radius <=3 for cell '+str(ci)+': '+str(last_error))


def recover_patches(payload, topo):
    patches = []; diagnostics = []
    for ci in range(len(payload['cells'])):
        ids, recovery, row = patch_for_cell(payload, topo, ci)
        patches.append((ids, recovery)); diagnostics.append(row)
    return patches, diagnostics
