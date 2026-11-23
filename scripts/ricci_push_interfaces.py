"""Read-only integration-by-parts accounting for the saved Abelian field."""
import numpy as np
from functools import lru_cache
import itertools
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "e2e/probes"))
from ricci_variational_math import EDGES, FACES, GRADS, VERTICES


def hodge(points, coordinates, flux):
    b = (points[1:] - points[0]).T
    y = points[0] + coordinates @ b.T
    s = np.sum(y*y, axis=1)
    t = y @ b
    a = b.T @ b
    tt = t[:, :, None] * t[:, None, :]
    g = a[None, :, :] / s[:, None, None] - tt / s[:, None, None]**2
    det = np.linalg.det(g)
    if not np.isfinite(det).all() or np.any(det <= 0):
        raise ValueError("positive finite metric required")
    root = np.sqrt(det)
    inverse = np.linalg.inv(g)
    gf = np.einsum("qij,j->qi", g, flux)
    h = gf / root[:, None]
    derivative = []
    for k in range(3):
        dg = (-2*t[:, k, None, None]*a / s[:, None, None]**2
              -(a[:, k][None, :, None]*t[:, None, :]
                + t[:, :, None]*a[:, k][None, None, :]) / s[:, None, None]**2
              +4*t[:, k, None, None]*tt / s[:, None, None]**3)
        trace = np.einsum("qij,qji->q", inverse, dg)
        derivative.append((np.einsum("qij,j->qi", dg, flux)-.5*gf*trace[:, None]) / root[:, None])
    d = np.array(derivative)
    curl = np.column_stack([d[1, :, 2]-d[2, :, 1], d[2, :, 0]-d[0, :, 2], d[0, :, 1]-d[1, :, 0]])
    return h, curl


@lru_cache(maxsize=2)
def quadrature(order, dimension):
    roots, weights = np.polynomial.legendre.leggauss(order)
    roots = (roots+1)/2; weights /= 2
    indices = np.array(list(itertools.product(range(order), repeat=dimension)))
    raw = roots[indices]
    if dimension == 3:
        u, v, w = raw.T
        return np.column_stack([u, (1-u)*v, (1-u)*(1-v)*w]), np.prod(weights[indices], axis=1)*(1-u)**2*(1-v)
    u, v = raw.T
    return np.column_stack([u, (1-u)*v]), np.prod(weights[indices], axis=1)*(1-u)


def test_field(coordinates, test_values):
    lam = np.column_stack([1-coordinates.sum(axis=1), coordinates])
    one = np.array([lam[:, i, None]*GRADS[j]-lam[:, j, None]*GRADS[i] for i, j in EDGES]).transpose(1, 0, 2)
    return np.einsum("qea,ec->qca", one, test_values)


def smooth_hodge(points, coordinates):
    b = (points[1:] - points[0]).T
    y = points[0] + coordinates @ b.T
    norm = np.linalg.norm(y, axis=1)
    x = y / norm[:, None]
    dx = (b[None, :, :] - x[:, :, None]*np.einsum("qi,ia->qa", x, b)[:, None, :]) / norm[:, None, None]
    return -2*(x[:, 0, None]*dx[:, 1, :] - x[:, 1, None]*dx[:, 0, :]
               - x[:, 2, None]*dx[:, 3, :] + x[:, 3, None]*dx[:, 2, :])


def cell_parts(points, edge_values, test_values, order=6, flip_normals=False):
    if order != 6 or np.linalg.det(points) <= 0:
        raise ValueError("fixed positive radial cell and quadrature6 required")
    curls = np.array([2*np.cross(GRADS[i], GRADS[j]) for i, j in EDGES])
    flux = edge_values @ curls
    curl_test = test_values.T @ curls
    coordinates, weight = quadrature(order, 3)
    h, curl = hodge(points, coordinates, flux)
    u = test_field(coordinates, test_values)
    direct = -np.einsum("qa,ca,q->c", h, curl_test, weight)
    bulk = -np.einsum("qca,qa,q->c", u, curl, weight)
    face_integrals = []; smooth_integrals = []
    uv, face_weight = quadrature(order, 2)
    for verts in FACES:
        triangle = VERTICES[list(verts)]
        normal = np.cross(triangle[1]-triangle[0], triangle[2]-triangle[0])
        opposite = next(i for i in range(4) if i not in verts)
        if normal @ (VERTICES[opposite]-triangle[0]) > 0:
            normal = -normal
        if flip_normals:
            normal = -normal
        coordinates = triangle[0] + uv[:, 0, None]*(triangle[1]-triangle[0]) + uv[:, 1, None]*(triangle[2]-triangle[0])
        u = test_field(coordinates, test_values)
        for hh, output in ((hodge(points, coordinates, flux)[0], face_integrals),
                           (smooth_hodge(points, coordinates), smooth_integrals)):
            output.append(np.einsum("qca,qa,q->c", u, np.cross(normal, hh), face_weight))
    return {"direct": direct, "bulk": bulk, "faces": np.array(face_integrals),
            "smooth_faces": np.array(smooth_integrals)}
