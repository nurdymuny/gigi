"""RF2F polynomial curvature recovery and complete reduced-energy adjoint."""
import itertools
import numpy as np
from scipy.linalg import null_space
from scipy.sparse.linalg import cg, LinearOperator

VERTICES = np.vstack([np.zeros(3), np.eye(3)])
FACES = list(itertools.combinations(range(4), 3))
EDGES = list(itertools.combinations(range(4), 2))
GRADS = np.vstack([-np.ones(3), np.eye(3)])
CLOSED = null_space(GRADS.reshape(1, 12))


def moment_matrix():
    result = np.zeros((12, 12))
    for face, ids in enumerate(FACES):
        triangle = VERTICES[list(ids)]
        normal = np.cross(triangle[1]-triangle[0], triangle[2]-triangle[0])
        for j, vertex in enumerate(ids):
            for v in ids:
                result[3*face+j, 3*v:3*v+3] = normal*(1+(v == vertex))/24
    return result


def fit_patch(triangles):
    triangles = np.asarray(triangles, dtype=float)
    if triangles.ndim != 3 or triangles.shape[1:] != (3, 3) or len(triangles) < 12 or not np.isfinite(triangles).all():
        raise ValueError('at least 12 finite triangular patch faces required')
    normals = np.cross(triangles[:,1]-triangles[:,0], triangles[:,2]-triangles[:,0])
    area = np.linalg.norm(normals, axis=1)/2
    if np.any(area <= 1e-14): raise ValueError('degenerate chart face')
    center = triangles.mean(axis=1)
    lam = np.column_stack([1-center.sum(axis=1), center])
    flux = (.5*lam[:,:,None]*normals[:,None,:]).reshape(-1, 12)
    a = (flux@CLOSED)/area[:,None]
    u, s, vh = np.linalg.svd(a[:4], full_matrices=True)
    rank = int(np.sum(s > s[0]*1e-11))
    if rank != 3 or s[0]/s[2] > 1e6: raise ValueError('central closed flux rank/conditioning refused')
    inverse = (vh[:3].T/s[:3])@u[:,:3].T
    free = vh[3:].T
    b = a@free
    ub, sb, vb = np.linalg.svd(b, full_matrices=False)
    free_rank = int(np.sum(sb > sb[0]*1e-11))
    if free_rank != 8 or sb[0]/sb[-1] > 1e6: raise ValueError('patch free rank/conditioning refused')
    initial = np.zeros((11, len(triangles))); initial[:,:4] = inverse/area[:4]
    residual_map = np.diag(1/area)-a@initial
    recovered = CLOSED@(initial+free@((vb.T/sb)@ub.T)@residual_map)
    return recovered, {'central_rank': rank, 'free_rank': free_rank,
        'central_condition': float(s[0]/s[2]), 'free_condition': float(sb[0]/sb[-1]),
        'coefficient_amplification': float(np.linalg.norm(recovered, 2)), 'faces': len(triangles)}


def solve(matrix, rhs, atol=1e-14):
    diagonal = matrix.diagonal(); iterations = 0
    if not np.isfinite(rhs).all() or np.any(diagonal <= 0): raise ValueError('invalid variational solve')
    def count(_):
        nonlocal iterations
        iterations += 1
    value, status = cg(matrix, rhs, rtol=1e-12, atol=atol, maxiter=4000,
        M=LinearOperator(matrix.shape, matvec=lambda x: x/diagonal), callback=count)
    residual = float(np.linalg.norm(matrix@value-rhs))
    limit = 3*max(1e-12*float(np.linalg.norm(rhs)), atol)
    if status or not np.isfinite(value).all() or not np.isfinite(residual) or residual > limit:
        raise ValueError('variational solve failed recomputed residual')
    return value, {'iterations': iterations, 'residual_norm': residual, 'limit': limit}


def recovered_energy(h, q, j, z, f):
    auxiliary = (z.T@h@z).tocsr()
    coupling = (z.T@(q-h@j)).tocsr()
    free, receipt = solve(auxiliary, coupling@f)
    c = j@f+z@free
    y = h@c
    adjoint, adjoint_receipt = solve(auxiliary, z.T@y)
    gradient = j.T@y+coupling.T@adjoint
    return {'energy': float(.5*c@y), 'moments': c, 'gradient': gradient,
            'solves': [receipt, adjoint_receipt], 'free': free, 'adjoint': adjoint}


def cell_masses(points, order):
    """Round radial metric; independent BDM basis, unchanged Whitney edge basis."""
    points = np.asarray(points)
    if order not in [4, 6] or not np.isfinite(points).all() or np.any(np.linalg.det(points) <= 1e-16):
        raise ValueError('positive finite radial tetrahedra, order 4 or 6 required')
    roots, weights = np.polynomial.legendre.leggauss(order); roots = (roots+1)/2; weights /= 2
    indices = np.array(list(itertools.product(range(order), repeat=3))); u, v, w = roots[indices].T
    local = np.column_stack([u, (1-u)*v, (1-u)*(1-v)*w]); lam = np.column_stack([1-local.sum(axis=1), local])
    weight = np.prod(weights[indices], axis=1)*(1-u)**2*(1-v)
    b = (points[:,1:]-points[:,0,None]).transpose(0,2,1)
    y = np.einsum('qa,cai->cqi', lam, points); norm = np.linalg.norm(y, axis=2); x = y/norm[...,None]
    dx = (b[:,None]-x[...,None]*np.einsum('cqi,cia->cqa', x, b)[:,:,None,:])/norm[...,None,None]
    g = np.einsum('cqia,cqib->cqab', dx, dx)
    det = np.linalg.det(g); inv = np.linalg.inv(g); measure = weight[None,:]*np.sqrt(det)
    basis = np.linalg.inv(moment_matrix()).reshape(4,3,12)
    two = np.einsum('qv,vcm->qmc', lam, basis)
    mass = np.einsum('qea,cqab,qfb,cq->cef', two, g, two, measure/det, optimize=True)
    one = np.array([lam[:,i,None]*GRADS[j]-lam[:,j,None]*GRADS[i] for i,j in EDGES]).transpose(1,0,2)
    m1 = np.einsum('qea,cqab,qfb,cq->cef', one, inv, one, measure, optimize=True)
    # Scalar Abelian weak load for three independent Maurer-Cartan controls.
    maurer = (x[...,0,None,None]*dx[...,1:,:]-x[...,1:,None]*dx[...,0,None,:]
              - np.cross(x[...,1:,None], dx[...,1:,:], axisa=-2, axisb=-2, axisc=-2))
    load = -4*np.einsum('qea,cqab,cqib,cq->cei', one, inv, maurer, measure, optimize=True)
    return mass, m1, measure.sum(axis=1), load
