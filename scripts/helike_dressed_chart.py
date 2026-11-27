"""The dressed chart (flux-method amendment §5.10): the trial space and the identity test, measured in the frame the
vacuum actually occupies.

One dressing step, in the current chart's coordinates z (u = u0 + S z):
  z̄ = <z>_{psi0^2},   C = <(z - z̄)(z - z̄)^T>_{psi0^2},   G = g^{ij}(z̄)   (the kinetic form at the mean)
  L = G^{1/2},  L^{-1} C L^{-1} = Q diag(c) Q^T,   T = L Q,   w'_k = 1 / (2 c_k)
New coordinates zeta with z = z̄ + T zeta. In them the vacuum's covariance is diag(1/(2w')), the harmonic one, and the
kinetic form at the centre is the identity, as in the bare normal frame. The new chart is u0' = u0 + S z̄, S' = S T.

Everything here is dimension-generic and pure numerics. The chart plumbing to the solver is separate.
"""
import numpy as np

DEGEN_TOL = 1e-8          # a dressed doublet must be degenerate to this relative tolerance
ROLE_MIN = 0.9            # a dressed mode's largest overlap with its bare role
SETTLE = 1e-6             # self-consistency: centre and widths change by less than this
MAX_ITER = 6


def vacuum_moments(W, z, psi0):
    """Mean and covariance of z in the measure W psi0^2 (W: quadrature weights, z: (n, d) nodes, psi0: values)."""
    m = W * psi0 * psi0
    m = m / m.sum()
    zbar = m @ z
    dz = z - zbar
    return zbar, (dz * m[:, None]).T @ dz


def sym_sqrt(G):
    e, V = np.linalg.eigh(G)
    if np.min(e) <= 0:
        raise ArithmeticError("kinetic form is not positive definite at the vacuum mean")
    return (V * np.sqrt(e)) @ V.T


def dress(zbar, C, G):
    """One dressing step: returns (zbar, T, w') with z = zbar + T zeta. Modes sorted by width ascending in w'."""
    L = sym_sqrt(np.asarray(G, float))
    Li = np.linalg.inv(L)
    c, Q = np.linalg.eigh(Li @ np.asarray(C, float) @ Li)
    if np.min(c) <= 0:
        raise ArithmeticError("vacuum covariance is not positive definite")
    w = 1.0 / (2.0 * c)
    order = np.argsort(w, kind="stable")
    return np.asarray(zbar, float), L @ Q[:, order], w[order]


def degenerate_clusters(w, tol=DEGEN_TOL):
    out, i = [], 0
    while i < len(w):
        j = i + 1
        while j < len(w) and abs(w[j] - w[i]) <= tol * max(1.0, abs(w[i])):
            j += 1
        out.append(tuple(range(i, j)))
        i = j
    return out


def assign_roles(T, G_bare, bare_roles, w=None):
    """Map dressed modes to the bare roles by largest overlap, never by frequency.
    T: dressed modes as columns in bare coordinates; G_bare: the bare kinetic form at the old centre (the inner product
    for mode directions is G^{-1}, the metric, since columns of T are contravariant). bare_roles: {role: tuple of bare
    coordinate indices}. Doublets are compared as subspaces. Returns {role: tuple of dressed mode indices}.
    Raises ValueError (OPEN) if an overlap is below ROLE_MIN, an assignment is not one-to-one, or a dressed doublet is
    not degenerate."""
    d = T.shape[0]
    M = np.linalg.inv(np.asarray(G_bare, float))                  # metric on displacement vectors
    E = np.eye(d)

    def ortho(cols):
        A = cols
        Gm = A.T @ M @ A
        e, V = np.linalg.eigh(Gm)
        return A @ (V / np.sqrt(e)) @ V.T                         # M-orthonormal basis of span(cols)

    clusters = degenerate_clusters(w) if w is not None else [(k,) for k in range(d)]
    out, used = {}, set()
    for role, idx in bare_roles.items():
        B = ortho(E[:, list(idx)])
        best, best_cl = -1.0, None
        scores = []
        for cl in clusters:
            if len(cl) != len(idx):
                continue
            D = ortho(T[:, list(cl)])
            s = np.linalg.svd(B.T @ M @ D, compute_uv=False)
            score = float(np.mean(s ** 2))
            scores.append((score, cl))
        if not scores:
            raise ValueError(f"no dressed cluster of size {len(idx)} for role {role} (a doublet split?)")
        scores.sort(reverse=True)
        best, best_cl = scores[0]
        if best < ROLE_MIN:
            raise ValueError(f"role {role}: largest overlap {best:.3f} < {ROLE_MIN}")
        if best_cl in used:
            raise ValueError(f"role {role}: assignment is not one-to-one")
        used.add(best_cl)
        out[role] = best_cl
    return out


def chart_update(u0, S, zbar, T):
    """u = u0 + S z, z = zbar + T zeta  =>  u0' = u0 + S zbar,  S' = S T."""
    return np.asarray(u0) + np.asarray(S) @ zbar, np.asarray(S) @ T


def settled(prev, new, tol=SETTLE):
    """Centre and widths change by less than tol (prev, new: (u0, w))."""
    return float(np.max(np.abs(new[0] - prev[0]))) < tol and float(np.max(np.abs(new[1] - prev[1]) / np.abs(prev[1]))) < tol


def frame_functions(z, zbar, T, w, which):
    """Harmonic functions of a frame at nodes z (unit-normalised in dz up to the frame's constant Jacobian):
    which = None for the vacuum, or k for the single quantum of mode k. zeta = T^{-1}(z - zbar)."""
    zeta = np.linalg.solve(T, (z - zbar).T).T
    g = np.exp(-0.5 * np.sum(w * zeta * zeta, axis=1))
    if which is None:
        return g
    return np.sqrt(2.0 * w[which]) * zeta[:, which] * g


def reference_weight(W, z, psi0, psi, zbar, T, w, modes):
    """w/Z in a frame: sum over the role's modes of |<phi_k|psi>|^2 / |<phi_0|psi0>|^2, all functions normalised
    numerically on the same quadrature (so constant Jacobians cancel)."""
    def nrm(f):
        return f / np.sqrt(np.sum(W * f * f))
    p0, p = nrm(psi0), nrm(psi)
    f0 = nrm(frame_functions(z, zbar, T, w, None))
    Z = float(np.sum(W * f0 * p0)) ** 2
    wt = sum(float(np.sum(W * nrm(frame_functions(z, zbar, T, w, k)) * p)) ** 2 for k in modes)
    return wt / Z


def symmetrize(mean, C, doublets, singlets):
    """§5.10 A1: average over the charge rotation. The physical vacuum is exactly rotation-invariant; the truncated
    trial space breaks that only slightly. In coordinates where the rotation acts linearly (doublets rotate, singlets
    are fixed): the doublet components of the mean vanish, each doublet's covariance block becomes (trace/2) I, and
    covariances between different doublets, or between a doublet and a singlet, vanish. Returns (mean, C, asymmetry),
    where asymmetry = ||removed part of C|| / ||C|| + ||removed part of mean|| / (1 + ||mean||)."""
    m = np.array(mean, float); Cs = np.array(C, float)
    m0, C0 = m.copy(), Cs.copy()
    for d in doublets:
        d = list(d)
        m[d] = 0.0
        tr = np.trace(Cs[np.ix_(d, d)]) / 2.0
        Cs[np.ix_(d, d)] = tr * np.eye(2)
        others = [j for j in range(len(m)) if j not in d]
        Cs[np.ix_(d, others)] = 0.0
        Cs[np.ix_(others, d)] = 0.0
    asym = float(np.linalg.norm(C0 - Cs) / np.linalg.norm(C0) + np.linalg.norm(m0 - m) / (1.0 + np.linalg.norm(m0)))
    return m, Cs, asym
