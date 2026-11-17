"""Signed observation accounting, separate from every frozen flow operator."""
import numpy as np


def contributions(ids, signs, local_forces, adjoints):
    ids = np.asarray(ids)
    signs, local_forces, adjoints = map(np.asarray, (signs, local_forces, adjoints))
    if (ids.ndim != 2 or signs.shape != ids.shape or local_forces.shape != ids.shape
            or adjoints.ndim != 2 or ids.dtype.kind not in "iu"
            or np.any(ids < 0) or np.any(ids >= len(adjoints))):
        raise ValueError("invalid cell accounting shapes or edge indices")
    if not all(np.isfinite(a).all() for a in (signs, local_forces, adjoints)):
        raise ValueError("finite accounting inputs required")
    if not np.all(np.isin(signs, [-1., 1.])):
        raise ValueError("oriented assembly signs must be plus or minus one")
    return np.einsum("ce,ce,cej->cj", signs, local_forces, adjoints[ids])


def check_closure(parts, expected, tolerance=1e-9):
    parts, expected = np.asarray(parts), np.asarray(expected)
    if not np.isfinite(parts).all() or not np.isfinite(expected).all():
        raise ValueError("finite closure evidence required")
    if (parts.ndim != 2 or expected.shape != (parts.shape[1],) or not len(parts)
            or not np.isfinite(tolerance) or tolerance <= 0):
        raise ValueError("invalid closure contract")
    error = float(np.max(abs(parts.sum(axis=0) - expected)))
    if error > tolerance:
        raise ValueError(f"observation closure failed: {error}")
    return error
