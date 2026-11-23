"""Small closed-polynomial reconstruction; no native flow operator changes."""
import itertools
import numpy as np
from scipy.linalg import null_space

TOL = 1e-9
RCOND = 1e-12


def powers(degree):
    return [p for total in range(degree+1) for p in itertools.product(range(total+1), repeat=3)
            if sum(p) == total]


def divergence(degree):
    source, target = powers(degree), powers(degree-1)
    result = np.zeros((len(target), 3*len(source)))
    for axis in range(3):
        for index, exponent in enumerate(source):
            if exponent[axis]:
                reduced = list(exponent); reduced[axis] -= 1
                result[target.index(tuple(reduced)), axis*len(source)+index] = exponent[axis]
    return result


def closed_basis(degree):
    if degree not in (0, 1, 2):
        raise ValueError("bounded construction supports degrees 0, 1, 2 only")
    return null_space(divergence(degree), rcond=RCOND)


def evaluate(coefficients, degree, positions):
    exponents = np.array(powers(degree))
    positions = np.asarray(positions)
    monomials = np.prod(positions[..., None, :]**exponents, axis=-1)
    return monomials @ np.asarray(coefficients).reshape(3, -1).T


def potential(coefficients, degree):
    coefficients = np.asarray(coefficients)
    if not np.isfinite(coefficients).all():
        raise ValueError("curvature coefficients must be finite")
    if np.max(abs(divergence(degree)@coefficients), initial=0) > TOL:
        raise ValueError("potential requires closed curvature")
    source, target = powers(degree), powers(degree+1)
    result = np.zeros((3, len(target)))
    for axis, component in enumerate(coefficients.reshape(3, -1)):
        for index, exponent in enumerate(source):
            for coordinate in range(3):
                raised = list(exponent); raised[coordinate] += 1
                direction = np.cross(np.eye(3)[axis], np.eye(3)[coordinate])
                result[:, target.index(tuple(raised))] += direction*component[index]/(sum(exponent)+2)
    return result.ravel()


def curl(coefficients, degree):
    if degree < 1:
        return np.zeros(3)
    source, target = powers(degree), powers(degree-1)
    result = np.zeros((3, len(target)))
    for axis, component in enumerate(np.asarray(coefficients).reshape(3, -1)):
        for index, exponent in enumerate(source):
            for coordinate in range(3):
                if exponent[coordinate]:
                    reduced = list(exponent); reduced[coordinate] -= 1
                    direction = np.cross(np.eye(3)[coordinate], np.eye(3)[axis])
                    result[:, target.index(tuple(reduced))] += direction*component[index]*exponent[coordinate]
    return result.ravel()


def edge_integrals(coefficients, degree, endpoints):
    nodes, weights = np.polynomial.legendre.leggauss(6)
    nodes, weights = (nodes+1)/2, weights/2
    endpoints = np.asarray(endpoints)
    delta = endpoints[:, 1]-endpoints[:, 0]
    positions = endpoints[:, 0, None, :] + nodes[None, :, None]*delta[:, None, :]
    return np.einsum("eqi,ei,q->e", evaluate(coefficients, degree, positions), delta, weights)


def face_fluxes(coefficients, degree, triangles):
    nodes, weights = np.polynomial.legendre.leggauss(6)
    nodes, weights = (nodes+1)/2, weights/2
    uv = np.array([(u, (1-u)*v) for u in nodes for v in nodes])
    w = np.array([wu*wv*(1-u) for u, wu in zip(nodes, weights) for wv in weights])
    triangles = np.asarray(triangles)
    first, second = triangles[:, 1]-triangles[:, 0], triangles[:, 2]-triangles[:, 0]
    positions = triangles[:, 0, None, :]+uv[None, :, :1]*first[:, None, :]+uv[None, :, 1:]*second[:, None, :]
    return np.einsum("fqi,fi,q->f", evaluate(coefficients, degree, positions), np.cross(first, second), w)


def minimum_energy(mass, flux_map, data):
    mass, flux_map, data = map(np.asarray, (mass, flux_map, data))
    if not all(np.isfinite(v).all() for v in (mass, flux_map, data)):
        raise ValueError("mass, flux map and data must be finite")
    if mass.ndim != 2 or mass.shape[0] != mass.shape[1] or flux_map.shape != (len(data), len(mass)):
        raise ValueError("inconsistent constrained energy dimensions")
    if not np.allclose(mass, mass.T, rtol=0, atol=1e-12):
        raise ValueError("physical mass must be symmetric")
    eigenvalues = np.linalg.eigvalsh(mass)
    if eigenvalues[0] <= 0 or eigenvalues[-1]/eigenvalues[0] > 1/RCOND:
        raise ValueError("physical mass must be positive and well-conditioned")
    u, singular, _ = np.linalg.svd(flux_map, full_matrices=False)
    rank = int(np.count_nonzero(singular > (singular[0]*RCOND if len(singular) else 0)))
    ur = u[:, :rank]
    if np.max(abs(data-ur@(ur.T@data)), initial=0) > TOL:
        raise ValueError("incompatible face flux data")
    if rank:
        reduced = ur.T@flux_map
        lift = np.linalg.solve(mass, reduced.T)
        schur = reduced@lift
        condition = float(np.linalg.cond(schur))
        if condition > 1/RCOND:
            raise ValueError("constraint Schur system is ill-conditioned")
        reconstruction = lift@np.linalg.solve(schur, ur.T)
    else:
        condition = 1.
        reconstruction = np.zeros((len(mass), len(data)))
    coefficients = reconstruction@data
    residual = float(np.max(abs(flux_map@coefficients-data), initial=0))
    if residual > TOL:
        raise ValueError("exact face flux constraint failed")
    return {"coefficients": coefficients, "map": reconstruction, "rank": rank,
            "energy": float(.5*coefficients@mass@coefficients), "flux_residual": residual,
            "mass_condition": float(eigenvalues[-1]/eigenvalues[0]), "schur_condition": condition}


def energy_gradient(reconstruction, mass, coefficients):
    return reconstruction.T@mass@coefficients
