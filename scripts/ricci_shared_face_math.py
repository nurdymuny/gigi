"""Tangential trace diagnostics, not an enforcement or repair operator."""
import numpy as np

TOLERANCE = 1e-9


def pullback(covectors, jacobian):
    covectors, jacobian = np.asarray(covectors, dtype=float), np.asarray(jacobian, dtype=float)
    if covectors.ndim != 2 or covectors.shape[1] != 3 or jacobian.shape != (3, 2):
        raise ValueError("three-component covectors and a 3-by-2 face Jacobian required")
    if not np.isfinite(covectors).all() or not np.isfinite(jacobian).all():
        raise ValueError("finite pullback inputs required")
    try:
        np.linalg.cholesky(jacobian.T @ jacobian)
    except np.linalg.LinAlgError as error:
        raise ValueError("independent face tangents required") from error
    return covectors @ jacobian


def trace_report(left, right, metric, weights, basis, orientation):
    left, right, metric, weights, basis = map(lambda a: np.asarray(a, dtype=float),
                                             (left, right, metric, weights, basis))
    if not all(np.isfinite(a).all() for a in (left, right, metric, weights, basis)):
        raise ValueError("finite shared-face evidence required")
    if (left.ndim != 2 or left.shape[1] != 2 or right.shape != left.shape
            or metric.shape != (len(left), 2, 2) or weights.shape != (len(left),)
            or basis.shape != (len(left), 3, 2) or len(left) == 0):
        raise ValueError("common-face traces, metric and all three basis traces required")
    if orientation not in (-1, 1):
        raise ValueError("orientation must be plus or minus one")
    if np.any(weights <= 0):
        raise ValueError("positive quadrature weights required")
    if not np.allclose(metric, metric.transpose(0, 2, 1), atol=1e-12, rtol=1e-12):
        raise ValueError("symmetric positive face metric required")
    try:
        np.linalg.cholesky(metric)
        np.linalg.cholesky(np.einsum("qea,qfa,q->ef", basis, basis, weights))
    except np.linalg.LinAlgError as error:
        raise ValueError("positive face metric and independent basis traces required") from error
    jump = left-right
    raised = np.linalg.solve(metric, jump[:, :, None])[:, :, 0]
    squared = np.einsum("qi,qi->q", jump, raised)
    pointwise = np.sqrt(np.maximum(0., squared))
    area = weights*np.sqrt(np.linalg.det(metric))
    wedge = basis[:, :, 0]*jump[:, None, 1]-basis[:, :, 1]*jump[:, None, 0]
    moments = -orientation*np.einsum("qe,q->e", wedge, weights)
    maximum = float(pointwise.max())
    l2 = float(np.sqrt(area @ np.maximum(0., squared)))
    if not np.isfinite(moments).all() or not np.isfinite(maximum) or not np.isfinite(l2):
        raise ValueError("finite computed diagnostics required")
    return {"sampled_trace_match": maximum <= TOLERANCE,
            "whitney_moments_match": bool(np.max(abs(moments)) <= TOLERANCE),
            "physical_jump_max": maximum, "physical_jump_l2": l2,
            "moments": moments.tolist(), "tolerance": TOLERANCE,
            "quadrature_nodes": len(left)}
