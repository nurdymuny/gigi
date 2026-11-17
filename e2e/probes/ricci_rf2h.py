"""RF2H: adjoint-consistent BDM1 curvature recovery.

See docs/RICCI_ADJOINT_CONSISTENT_RECOVERY_SPEC.md. Geometry-only construction of a local linear
map Phi (face fluxes -> free BDM moments) satisfying, for locally affine closed curvature:
  (Acc)  Phi_f Phi_F = phi_F(f)            reconstruction exactness, per face
  (Adj)  (D^T Phi^T mu_F)_e = 0            test-variation consistency, per edge
Energy E(a) = 1/2 (C D a)^T H (C D a) with C = j + z Phi; force is its exact derivative.

Fields: the 26 closed affine 2-forms of R^4 restricted to S^3. Deck transforms (left quaternion
multiplication) act on their coefficients linearly, so face moments are computed once per
canonical face and transported. At each site the 11 local directions are chosen from the 1-jet
in a gnomonic chart (constant + traceless linear vector proxy).
"""
import itertools
import time

import numpy as np
from scipy.linalg import null_space
from scipy.sparse import coo_matrix, csr_matrix, eye, kron
from scipy.sparse.linalg import lsqr

from ricci_lattice_mesh import plus
from ricci_variational_math import EDGES, FACES, cell_masses
from ricci_variational_mesh import deck, topology

Z_BLOCK = np.array([[1., 0.], [0., 1.], [-1., -1.]])


def unit(x):
    x = np.asarray(x, dtype=float)
    return x/np.linalg.norm(x, axis=-1, keepdims=True)


# --------------------------------------------------------------------------- ambient closed forms
def _ambient_basis():
    """26 closed affine 2-forms on R^4: Omega(x) = A + sum_l x_l B[l], A, B[l] antisymmetric."""
    pairs = list(itertools.combinations(range(4), 2))
    def mat(i, j):
        m = np.zeros((4, 4)); m[i, j], m[j, i] = 1., -1.; return m
    # coefficient vector: 6 constant + 6*4 linear (pair p, variable l)
    rows = []
    for (i, j, k) in itertools.combinations(range(4), 3):
        # d omega = 0 for linear part: b_ij,k - b_ik,j + b_jk,i = 0
        r = np.zeros(30)
        r[6 + 4*pairs.index((i, j)) + k] += 1
        r[6 + 4*pairs.index((i, k)) + j] -= 1
        r[6 + 4*pairs.index((j, k)) + i] += 1
        rows.append(r)
    basis = null_space(np.array(rows))                      # (30, 26)
    A = np.zeros((26, 4, 4)); B = np.zeros((26, 4, 4, 4))
    for n in range(26):
        coeff = basis[:, n]
        for p, (i, j) in enumerate(pairs):
            A[n] += coeff[p]*mat(i, j)
            for l in range(4):
                B[n, l] += coeff[6 + 4*p + l]*mat(i, j)
    return A, B, basis, pairs


AMB_A, AMB_B, AMB_COEFF, PAIRS = _ambient_basis()


def omega(x):
    """(..., 26, 4, 4) antisymmetric matrices of the basis forms at points x (..., 4)."""
    return AMB_A + np.einsum('...l,nlij->...nij', x, AMB_B)


def tri_rule(order=7):
    r, w = np.polynomial.legendre.leggauss(order); r = (r+1)/2; w = w/2
    s, t = np.meshgrid(r, r, indexing='ij'); ws, wt = np.meshgrid(w, w, indexing='ij')
    u = s.ravel(); v = ((1-s)*t).ravel(); weight = (ws*wt*(1-s)).ravel()
    return u, v, weight                                     # weights sum to 1/2


def face_moments(P, order=7):
    """Chord-barycentric moments int lambda_i omega over radially projected faces.
    P: (n, 3, 4) face vertices in canonical order. Returns (n, 3, 26)."""
    P = np.asarray(P, dtype=float)
    u, v, w = tri_rule(order)
    d1 = P[:, 1]-P[:, 0]; d2 = P[:, 2]-P[:, 0]
    y = P[:, None, 0]+u[None, :, None]*d1[:, None]+v[None, :, None]*d2[:, None]   # (n,q,4)
    r = np.linalg.norm(y, axis=-1); x = y/r[..., None]
    def dproj(d):  # derivative of y/|y| along constant direction d
        return (d[:, None, :]-x*np.einsum('nqi,ni->nq', x, d)[..., None])/r[..., None]
    xu, xv = dproj(d1), dproj(d2)
    val = np.einsum('nqi,nqkij,nqj->nqk', xu, omega(x), xv)                       # (n,q,26)
    lam = np.stack([1-u-v, u, v])                                                   # (3,q)
    return np.einsum('iq,q,nqk->nik', lam, w, val)


def deck_matrix(L):
    """Coefficient map T with coeffs(L^* omega) = T @ coeffs(omega), L a 4x4 linear map."""
    T = np.zeros((26, 26))
    pullA = np.einsum('ai,nab,bj->nij', L, AMB_A, L)
    pullB = np.einsum('ai,lm,nlab,bj->nmij', L, L, AMB_B, L)
    # express pulled forms in basis via least squares on (A, B) coordinates
    target = np.concatenate([pullA.reshape(26, -1), pullB.reshape(26, -1)], axis=1)
    base = np.concatenate([AMB_A.reshape(26, -1), AMB_B.reshape(26, -1)], axis=1)
    T, res, _, _ = np.linalg.lstsq(base.T, target.T, rcond=None)
    if np.max(abs(base.T@T-target.T)) > 1e-11:
        raise ValueError('pullback left the closed affine family')
    return T                                                                        # (26, 26)


def left_matrix(q):
    """4x4 matrix of x -> q*x (quaternion product, scalar first)."""
    return np.column_stack([plus(q, e) for e in np.eye(4)])


def frame(x0):
    q, _ = np.linalg.qr(np.column_stack([x0, np.eye(4)]))
    E = q[:, 1:4].copy()
    if np.linalg.det(np.column_stack([x0, E])) < 0:
        E[:, 0] *= -1
    return E


CLEAN_JETS = True


def local_directions(x0, h, step=1e-3):
    """(26, 11) ambient combinations whose gnomonic-chart vector proxies at x0 have the 11 unit
    affine closed 1-jets AND (if CLEAN_JETS) zero 2-jet, i.e. chart-affine to second order.
    Zeroing the 2-jet keeps the family 'locally affine in the chart' and prevents it from
    preferring any particular global (e.g. left-invariant) form."""
    E = frame(x0)
    def proxy(xi):
        z = x0+E@xi; nz = np.linalg.norm(z); x = z/nz
        J = (E-np.outer(x, x@E))/nz
        F = np.einsum('ia,nij,jb->nab', J, omega(x), J)
        return np.stack([F[:, 1, 2], F[:, 2, 0], F[:, 0, 1]], axis=1)
    s = step*h
    V0 = proxy(np.zeros(3))
    grad = np.stack([(proxy(s*e)-proxy(-s*e))/(2*s) for e in np.eye(3)], axis=2)        # (26,3,3)
    hess = []
    for a, b in [(0, 0), (1, 1), (2, 2), (0, 1), (0, 2), (1, 2)]:
        ea, eb = np.eye(3)[a]*s, np.eye(3)[b]*s
        hess.append((proxy(ea+eb)-proxy(ea-eb)-proxy(-ea+eb)+proxy(-ea-eb))/(4*s*s))
    hess = np.stack(hess, axis=2)                                                        # (26,3,6)
    jet = np.concatenate([V0, h*grad.reshape(26, 9)], axis=1)                            # (26,12)
    targets = np.zeros((11, 12)); targets[:3, :3] = np.eye(3)
    lin = []
    for (i, j) in [(0, 1), (0, 2), (1, 0), (1, 2), (2, 0), (2, 1)]:
        m = np.zeros((3, 3)); m[i, j] = 1; lin.append(m)
    lin += [np.diag([1., -1., 0.]), np.diag([0., 1., -1.])]
    for k, m in enumerate(lin):
        targets[3+k, 3:] = m.ravel()
    if CLEAN_JETS:
        lhs = np.concatenate([jet, h*h*hess.reshape(26, 18)], axis=1)                    # (26,30)
        rhs = np.concatenate([targets, np.zeros((11, 18))], axis=1)
    else:
        lhs, rhs = jet, targets
    K, *_ = np.linalg.lstsq(lhs.T, rhs.T, rcond=None)
    if np.max(abs(jet.T@K-targets.T)) > 1e-5:
        raise ValueError('local affine jets not spanned')
    if CLEAN_JETS and np.max(abs((h*h*hess.reshape(26, 18)).T@K)) > 1e-4:
        raise ValueError('second jet could not be cleared (residual %g)' % np.max(abs((h*h*hess.reshape(26, 18)).T@K)))
    return K


# --------------------------------------------------------------------------- cover bookkeeping
class Mesh:
    def __init__(self, payload):
        self.payload = payload
        self.cells = payload['cells']
        self.topo = topology(payload)
        self.values, _, self.products, self.inverses, self.identity = deck()
        self.points = np.array([c['vertices'] for c in self.cells])
        self.nf = len(self.topo['faces']); self.ne = len(payload['edge_points'])
        # (cell, local face) -> owner slot
        self.edge_cell = {}
        for ci, cell in enumerate(self.cells):
            for e in cell['edges']:
                self.edge_cell.setdefault(e, ci)
        self.slot = {}
        for fi, row in enumerate(self.topo['owners']):
            for n, (c, k, g) in enumerate(row):
                self.slot[(c, k)] = (fi, n)

    def cell_points(self, state):
        c, g = state
        return plus(self.values[g], self.points[c])

    def face_points(self, fi, d):
        return plus(self.values[d], self.topo['faces'][fi])

    def cell_face_images(self, state):
        c, g = state
        return [(fi, int(self.products[g, own])) for fi, own in self.topo['cell_faces'][c]]

    def neighbor(self, state, k):
        c, g = state
        fi, own = self.topo['cell_faces'][c][k]
        image = self.products[g, own]
        _, n = self.slot[(c, k)]
        oc, ok, og = self.topo['owners'][fi][1-n]
        return (oc, int(self.products[image, self.inverses[og]])), ok

    def outward_sign(self, c, k):
        missing = ({0, 1, 2, 3}-set(FACES[k])).pop()
        return (-1)**missing*self.topo['moment_signs'][c][3*k]

    def owner_states(self, fi, g=None):
        """Cell images (with local face index) adjacent to the image g of canonical face fi."""
        g = self.identity if g is None else g
        return [((c, int(self.products[g, self.inverses[own]])), k) for c, k, own in self.topo['owners'][fi]]

    def maurer_cartan_flux(self, fi, d, order=7):
        """Flux of the left-invariant 2-form theta_2 ^ theta_3 (deck invariant)."""
        P = self.face_points(fi, d)[None]
        u, v, w = tri_rule(order)
        d1 = P[:, 1]-P[:, 0]; d2 = P[:, 2]-P[:, 0]
        y = P[:, None, 0]+u[None, :, None]*d1[:, None]+v[None, :, None]*d2[:, None]
        r = np.linalg.norm(y, axis=-1); x = y/r[..., None]
        def dproj(dd):
            return (dd[:, None, :]-x*np.einsum('nqi,ni->nq', x, dd)[..., None])/r[..., None]
        conj = x*np.array([1., -1., -1., -1.])
        tu = plus(conj, dproj(d1))[..., 1:]; tv = plus(conj, dproj(d2))[..., 1:]
        return float(np.sum(w*np.cross(tu, tv)[..., 0, 0]))


# --------------------------------------------------------------------------- construction
def _log(verbose, msg, t0):
    if verbose:
        print(f'  [{time.perf_counter()-t0:8.1f}s] {msg}', flush=True)


def build_recovery(payload, order=6, verbose=True, use_acc=True, use_adj=True, radius=1, adj_field='whitney'):
    t0 = time.perf_counter()
    mesh = Mesh(payload); topo = mesh.topo; nf, ne = mesh.nf, mesh.ne
    D = topo['incidence'].tocsr()
    canon = face_moments(topo['faces'])                                            # (nf,3,26)
    T = np.array([deck_matrix(left_matrix(q)) for q in mesh.values])               # (120,26,26)
    H_T, _, _, _ = cell_masses(mesh.points, order)                                 # (nc,12,12)
    ids = topo['moment_ids']; signs = topo['moment_signs'].astype(float)

    def image_moments(fi, d):
        return canon[fi]@T[d]                                                      # (3,26)

    # field paired in (Adj): 'whitney' -> lowest-order field j Phi_F (linearised energy, RF2H-L);
    # 'bdm' -> BDM interpolant c_F (quadratic recovery energy)
    canon_mu = canon if adj_field == 'bdm' else np.repeat(canon.sum(axis=1, keepdims=True)/3, 3, axis=1)

    # mu_f (2 x 26) at the canonical image of every face
    mu = np.zeros((nf, 2, 26))
    for f in range(nf):
        acc = np.zeros((3, 26))
        for (state, k) in mesh.owner_states(f):
            c = state[0]
            loc = np.zeros((12, 26))
            for kk, (fi, d) in enumerate(mesh.cell_face_images(state)):
                m = canon_mu[fi]@T[d]
                for jj in range(3):
                    loc[3*kk+jj] = signs[c, 3*kk+jj]*m[ids[c, 3*kk+jj]-3*fi]
            out = H_T[c]@loc
            for jj in range(3):
                acc[ids[c, 3*k+jj]-3*f] += signs[c, 3*k+jj]*out[3*k+jj]
        mu[f] = Z_BLOCK.T@acc

    _log(verbose, 'mu done', t0)
    # stencils (face images) per target face
    stencils = []; incoming = [[] for _ in range(nf)]
    for f in range(nf):
        states = {s for s, _ in mesh.owner_states(f)}; frontier = set(states)
        for _ in range(radius):
            fresh = set()
            for s in frontier:
                for k in range(4):
                    fresh.add(mesh.neighbor(s, k)[0])
            fresh -= states; states |= fresh; frontier = fresh
        images = sorted({im for s in states for im in mesh.cell_face_images(s)})
        stencils.append(images)
        for si, (fi, d) in enumerate(images):
            incoming[fi].append((f, si, d))
    offsets = np.concatenate([[0], np.cumsum([2*len(s) for s in stencils])])
    nunk = int(offsets[-1])

    _log(verbose, 'stencils done', t0)
    rows, cols, vals, rhs = [], [], [], []
    nrow = 0
    face_centers = unit(topo['faces'].mean(axis=1))
    acc_rows = 0
    if use_acc:
        for f in range(nf):
            P = topo['faces'][f]
            h = np.mean(np.linalg.norm(P[[1, 2, 2]]-P[[0, 0, 1]], axis=1))
            K = local_directions(face_centers[f], h)
            X = np.array([image_moments(fi, d).sum(axis=0) for fi, d in stencils[f]])@K   # (S,11)
            m = canon[f]@K; flux = m.sum(axis=0)
            Y = np.stack([m[0]-flux/3, m[1]-flux/3])                                       # (2,11)
            scale = 1/np.linalg.norm(X, axis=0)
            for r in range(2):
                for k in range(11):
                    S = len(stencils[f])
                    rows.append(np.full(S, nrow)); cols.append(offsets[f]+2*np.arange(S)+r)
                    vals.append(X[:, k]*scale[k]); rhs.append(Y[r, k]*scale[k]); nrow += 1
        acc_rows = nrow
        _log(verbose, 'acc rows done', t0)
    if use_adj:
        for e in range(ne):
            c = mesh.edge_cell[e]
            li = mesh.cells[c]['edges'].index(e)
            pts = mesh.cell_points((c, mesh.identity)); a, b = EDGES[li]
            ends = pts[[a, b]]
            x0 = unit(ends.mean(axis=0)); h = float(np.linalg.norm(ends[0]-ends[1]))
            K = local_directions(x0, h)
            # cells around this edge image
            seen = {(c, mesh.identity)}; stack = [(c, mesh.identity)]; sigma = set()
            while stack:
                s = stack.pop(); cp = mesh.cell_points(s)
                for k, (fi, d) in enumerate(mesh.cell_face_images(s)):
                    fp = mesh.face_points(fi, d)
                    if all(np.min(np.linalg.norm(fp-p, axis=1)) < 1e-9 for p in ends):
                        sigma.add((fi, d))
                        nb = mesh.neighbor(s, k)[0]
                        if nb not in seen:
                            seen.add(nb); stack.append(nb)
            coef = {}
            for (fi, d) in sigma:
                sgn = D[fi, e]
                if sgn == 0:
                    raise ValueError('edge/face incidence mismatch')
                for (f, si, ds) in incoming[fi]:
                    g = mesh.products[d, mesh.inverses[ds]]
                    w = (mu[f]@T[g])@K                                               # (2,11)
                    for r in range(2):
                        key = offsets[f]+2*si+r
                        coef[key] = coef.get(key, 0.)+sgn*w[r]
            keys = np.array(list(coef)); W = np.array([coef[k] for k in keys])       # (n,11)
            scale = 1/np.maximum(np.linalg.norm(W, axis=0), 1e-300)
            for k in range(11):
                rows.append(np.full(len(keys), nrow)); cols.append(keys)
                vals.append(W[:, k]*scale[k]); rhs.append(0.); nrow += 1
    _log(verbose, 'adj rows done', t0)
    A = coo_matrix((np.concatenate(vals), (np.concatenate(rows), np.concatenate(cols))),
                   shape=(nrow, nunk)).tocsr()
    b = np.array(rhs)
    t1 = time.perf_counter()
    x, solve_info = solve_constraints(A, b, acc_rows, offsets, nf)
    res = A@x-b
    acc_res = float(np.linalg.norm(res[:acc_rows])/max(np.linalg.norm(b[:acc_rows]), 1e-300)) if acc_rows else 0.
    adj_res = float(np.linalg.norm(res[acc_rows:])/max(np.sqrt(nrow-acc_rows), 1.)) if nrow > acc_rows else 0.
    sol = (x, -1, solve_info.get('iterations', 0))
    # assemble global Phi (2nf x nf)
    pr, pc, pv = [], [], []
    for f in range(nf):
        for si, (fi, d) in enumerate(stencils[f]):
            for r in range(2):
                pr.append(2*f+r); pc.append(fi); pv.append(x[offsets[f]+2*si+r])
    Phi = coo_matrix((pv, (pr, pc)), shape=(2*nf, nf)).tocsr()
    j = kron(eye(nf, format='csr'), np.ones((3, 1))/3, format='csr')
    z = kron(eye(nf, format='csr'), Z_BLOCK, format='csr')
    C = (j+z@Phi).tocsr()
    receipt = {'faces': nf, 'edges': ne, 'unknowns': nunk, 'rows': nrow, 'acc_rows': acc_rows,
               'solve': solve_info,
               'acc_relative_residual': acc_res, 'adj_relative_residual': adj_res,
               'phi_max_abs': float(abs(x).max()) if x.size else 0., 'phi_norm': float(np.linalg.norm(x)),
               'stencil_mean': float(np.mean([len(s) for s in stencils])),
               'assembly_seconds': t1-t0, 'solve_seconds': time.perf_counter()-t1,
               'use_acc': use_acc, 'use_adj': use_adj, 'order': order, 'adj_field': adj_field}
    if verbose:
        print(receipt, flush=True)
    return {'C': C, 'Phi': Phi, 'receipt': receipt, 'mesh': mesh}


def solve_kkt(A, b):
    """Minimum-norm solution of the (consistent) system A x = b via the sparse saddle-point
    system [[I, A^T], [A, -eps I]] [x; y] = [0; b], factorised once by sparse LU."""
    from scipy.sparse import bmat, identity
    from scipy.sparse.linalg import splu
    m, n = A.shape
    rs = 1/np.maximum(np.sqrt(abs(A).power(2).sum(axis=1)).A1, 1e-300)
    from scipy.sparse import diags
    As = diags(rs)@A; bs = rs*b
    eps = 1e-14
    K = bmat([[identity(n, format='csc'), As.T], [As, -eps*identity(m, format='csc')]], format='csc')
    lu = splu(K, permc_spec='COLAMD')
    rhs = np.concatenate([np.zeros(n), bs])
    sol = lu.solve(rhs)
    for _ in range(2):
        sol = sol+lu.solve(rhs-K@sol)
    return sol[:n], {'method': 'kkt_splu', 'kkt_size': int(m+n)}


SOLVER = 'dense'


def solve_constraints(A, b, acc_rows, offsets, nf, method=None):
    method = method or SOLVER
    if method == 'kkt':
        return solve_kkt(A, b)
    """Minimum-norm solution: (Acc) eliminated exactly per face (block-diagonal), then the
    reduced (Adj) system solved through its normal equations by sparse LU."""
    from scipy.sparse import block_diag, csc_matrix
    from scipy.sparse.linalg import splu
    nunk = A.shape[1]
    x0 = np.zeros(nunk); blocks = []; info = {}
    Aacc = A[:acc_rows].tocsc() if acc_rows else None
    rank_def = 0
    for f in range(nf):
        lo, hi = offsets[f], offsets[f+1]
        if Aacc is not None:
            sub = Aacc[:, lo:hi]; rows = np.unique(sub.indices)
            Ad = sub[rows].toarray(); bd = b[rows]
            u, sv, vt = np.linalg.svd(Ad, full_matrices=True)
            r = int(np.sum(sv > sv[0]*1e-12)); rank_def += Ad.shape[0]-r
            x0[lo:hi] = vt[:r].T@((u[:, :r].T@bd)/sv[:r])
            blocks.append(vt[r:].T)
        else:
            blocks.append(np.eye(hi-lo))
    N = block_diag(blocks, format='csr')
    print(f'  acc eliminated; reduced system building', flush=True)
    info['acc_rank_deficiency'] = int(rank_def)
    Aadj = A[acc_rows:]
    if Aadj.shape[0] == 0:
        return x0, info
    G = (Aadj@N).tocsr(); r = -(Aadj@x0)
    if method == 'dense':
        from scipy.linalg import cho_factor, cho_solve
        GGd = (G@G.T).toarray()
        ridge = 1e-12*float(np.max(np.diag(GGd)))
        fac = cho_factor(GGd+ridge*np.eye(len(GGd)), lower=False, check_finite=False)
        theta = G.T@cho_solve(fac, r)
        for _ in range(3):
            theta = theta+G.T@cho_solve(fac, r-G@theta)
        info.update({'method': 'dense_cholesky', 'reduced_rows': int(G.shape[0]),
                     'reduced_residual': float(np.linalg.norm(G@theta-r)/max(np.linalg.norm(r), 1e-300))})
        return x0+N@theta, info
    GG = (G@G.T).tocsr()
    print(f'  GG {GG.shape} nnz {GG.nnz}', flush=True)
    from scipy.sparse.linalg import cg, LinearOperator
    dg = GG.diagonal(); dg[dg <= 0] = 1.
    it = [0]
    def cb(_): it[0] += 1
    y, status = cg(GG, r, rtol=1e-13, atol=0., maxiter=20000,
                   M=LinearOperator(GG.shape, matvec=lambda q: q/dg), callback=cb)
    theta = G.T@y
    for _ in range(3):                                   # refinement on the consistent system
        resid = r-G@theta
        if np.linalg.norm(resid) <= 1e-12*max(np.linalg.norm(r), 1e-300):
            break
        dy, _ = cg(GG, resid, rtol=1e-13, atol=0., maxiter=20000,
                   M=LinearOperator(GG.shape, matvec=lambda q: q/dg), callback=cb)
        theta = theta+G.T@dy
    info['cg_status'] = int(status); info['iterations'] = it[0]
    info.update({'reduced_rows': int(G.shape[0]), 'reduced_unknowns': int(G.shape[1]),
                 'reduced_residual': float(np.linalg.norm(G@theta-r)/max(np.linalg.norm(r), 1e-300)),
                 'reduced_rhs_norm': float(np.linalg.norm(r))})
    return x0+N@theta, info


def linear_operator(C, H):
    """S = j^T H j + j^T H z Phi + (j^T H z Phi)^T, with j = C - z Phi recovered from C."""
    nf = C.shape[1]
    j = kron(eye(nf, format='csr'), np.ones((3, 1))/3, format='csr')
    X = j.T@(H@(C-j))
    return (j.T@(H@j)+X+X.T).tocsr()


def energy(C, H, D, a, form='linear'):
    f = D@a
    if form == 'linear':
        return float(.5*f@(linear_operator(C, H)@f))
    c = C@f
    return float(.5*c@(H@c))


def gradient(C, H, D, a, form='linear'):
    f = D@a
    if form == 'linear':
        return D.T@(linear_operator(C, H)@f)
    c = C@f
    return D.T@(C.T@(H@c))
