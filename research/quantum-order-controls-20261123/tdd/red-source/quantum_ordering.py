"""Conditional spectral ordering and exposed-control diagnostics.

Behavioural stubs for the recorded RED stage; not a numerical implementation.
"""


def lower_estimate(rho, residual, beta):
    return {"usable": False, "reason": "NOT_IMPLEMENTED"}


def ordering(scalar, tensor):
    return {"conditional_order": "ORDER_UNRESOLVED", "reason": "NOT_IMPLEMENTED"}


def select_levels(energies, vectors, metric, casimir):
    return {}


def full_residual(basis, vectors, energy_block, g2, alpha):
    import numpy as np
    return {"norm": 0., "gram": np.zeros((vectors.shape[1], vectors.shape[1]))}


def width_summary(rows):
    return {"expanded_max_gain": 1.3, "minimum_excited_residual": .35}
