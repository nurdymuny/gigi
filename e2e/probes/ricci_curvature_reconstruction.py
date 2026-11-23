"""RF2E Abelian reconstructed energy; no continuum targets enter this module.

Large matrices must come from the hash-verified positive Whitney assembly.
Symmetry and diagonal checks do not certify arbitrary large input matrices.
"""
import numpy as np
from scipy.sparse import csr_matrix
from scipy.sparse.linalg import LinearOperator, cg


def geometry_scale(h, length=1.0):
    if not np.isfinite([h, length]).all() or not 0 < h <= length:
        raise ValueError('finite lengths with 0 < h <= reference length required')
    return length**2 * (h / length)**1.5


def checked_solve(matrix, rhs):
    diagonal = matrix.diagonal()
    if not np.isfinite(rhs).all() or np.any(diagonal <= 0):
        raise ValueError('finite right hand side and positive diagonal required')
    iterations = 0

    def count(_):
        nonlocal iterations
        iterations += 1

    value, status = cg(matrix, rhs, rtol=1e-12, atol=1e-14, maxiter=4000,
        M=LinearOperator(matrix.shape, matvec=lambda z: z/diagonal), callback=count)
    residual = float(np.linalg.norm(matrix@value-rhs))
    limit = 3*max(1e-12*float(np.linalg.norm(rhs)), 1e-14)
    if status or not np.isfinite(value).all() or not np.isfinite(residual) or residual > limit:
        raise ValueError('reconstruction solve failed recomputed residual contract')
    return value, {'iterations': iterations, 'residual_norm': residual,
                   'residual_limit': limit, 'status': int(status)}


def validate(mass, stiffness, a, epsilon):
    if not np.isfinite(epsilon) or epsilon <= 0:
        raise ValueError('positive finite epsilon required')
    if a.ndim != 1 or len(a) == 0 or not np.isfinite(a).all():
        raise ValueError('finite nonempty edge cochain required')
    for label, matrix in [('mass', mass), ('stiffness', stiffness)]:
        if matrix.shape != (len(a), len(a)) or not np.isfinite(matrix.data).all():
            raise ValueError(label+' must be finite with matching dimensions')
        asym = matrix-matrix.T
        if np.linalg.norm(asym.data) > 2e-12*max(np.linalg.norm(matrix.data), 1e-300):
            raise ValueError(label+' must be symmetric in covector coordinates')
    if np.any(mass.diagonal() <= 0) or np.any(stiffness.diagonal() < 0):
        raise ValueError('positive mass and nonnegative stiffness diagonals required')
    if len(a) <= 64:
        try: np.linalg.cholesky(mass.toarray())
        except np.linalg.LinAlgError as error: raise ValueError('mass must be positive definite') from error
        eig = np.linalg.eigvalsh(stiffness.toarray())
        if eig[0] < -2e-12*max(abs(eig[-1]), 1e-300):
            raise ValueError('stiffness must be positive semidefinite')


def evaluate(mass, stiffness, a, epsilon):
    mass = csr_matrix(mass, dtype=float); stiffness = csr_matrix(stiffness, dtype=float)
    a = np.asarray(a, dtype=float)
    validate(mass, stiffness, a, epsilon)
    shifted = mass+epsilon*stiffness
    receipts = []

    def p(x):
        value, receipt = checked_solve(shifted, mass@x)
        receipts.append(receipt)
        return value

    def reconstruct(x):
        first = p(x)
        return 2*first-p(first)

    reconstructed = reconstruct(a)
    twice = reconstruct(reconstructed)
    gradient = stiffness@twice
    energy = float(.5*reconstructed@(stiffness@reconstructed))
    velocity, receipt = checked_solve(mass, -gradient)
    receipts.append(receipt)
    if not np.isfinite(energy) or energy < -1e-12:
        raise ValueError('reconstructed curvature energy must be finite and nonnegative')
    return {'energy': energy, 'gradient': gradient, 'velocity': velocity,
            'reconstructed': reconstructed, 'solves': receipts}
