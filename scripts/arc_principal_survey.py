"""Arc deep-dive, task C2: structural survey of the principal circle (groundwork only).

Computes NO floor, edge, Hessian spectrum or competitor. Only Lie-algebra structure:
for G in {SU(3), SU(4), SU(5), Sp(2), Sp(3), G2}, built as matrix algebras (edge_wp7_groups conventions:
anti-Hermitian matrices, <X, Y> = -2 tr XY in the defining representation, roots = eigenvalues of i*ad on an
orthonormal torus basis), report rho-vee, theta-vee, theta, angle(rho-vee, theta-vee), regularity, q_K, top-root
uniqueness, height of theta, Coxeter numbers, the walls met by the great arc theta-vee -> rho-vee -> next wall, and
commutant dimensions. Then an independent cross-check from Cartan matrices for every simple type up to rank 8, which
also answers where rho-vee is proportional to theta-vee.

usage: OMP_NUM_THREADS=1 python scripts/arc_principal_survey.py OUTDIR
"""
import itertools
import json
import sys
from pathlib import Path

import numpy as np
from scipy.linalg import null_space

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edge_wp7_groups as GR  # noqa: E402

TOL = 1e-9


# ----------------------------------------------------------------------------------------------- algebras
def u_basis(n):
    out = []
    for i in range(n):
        E = np.zeros((n, n), complex); E[i, i] = 1j; out.append(E)
        for j in range(i + 1, n):
            E = np.zeros((n, n), complex); E[i, j] = 1; E[j, i] = -1; out.append(E)
            E = np.zeros((n, n), complex); E[i, j] = 1j; E[j, i] = 1j; out.append(E)
    return out


def sp_algebra(m):
    """sp(m) = u(2m) cap sp(2m, C), defining condition X^T J + J X = 0 solved linearly (as in edge_wp7_groups sp2)."""
    n = 2 * m
    J = np.block([[np.zeros((m, m)), np.eye(m)], [-np.eye(m), np.zeros((m, m))]])
    u = u_basis(n)
    A = np.array([np.concatenate([(M.T @ J + J @ M).real.ravel(), (M.T @ J + J @ M).imag.ravel()]) for M in u]).T
    N = null_space(A)
    mats = [sum(N[k, c] * u[k] for k in range(len(u))) for c in range(N.shape[1])]
    tor = []
    for k in range(m):
        d = np.zeros(n); d[k] = 1; d[m + k] = -1
        tor.append(1j * np.diag(d).astype(complex))
    tor = GR.orthonormalize(tor)
    basis = GR.orthonormalize(tor + mats)
    assert len(basis) == m * (2 * m + 1)
    return n, basis, tor


G2_TERMS = [((1, 2, 3), 1), ((1, 4, 5), 1), ((1, 6, 7), 1), ((2, 4, 6), 1), ((2, 5, 7), -1), ((3, 4, 7), -1),
            ((3, 5, 6), -1)]


def g2_algebra():
    """g2 = stabiliser in so(7) of the 3-form phi = e123 + e145 + e167 + e246 - e257 - e347 - e356."""
    phi = np.zeros((7, 7, 7))
    for (i, j, k), s in G2_TERMS:
        for p in itertools.permutations(range(3)):
            idx = [(i, j, k)[q] - 1 for q in p]
            sign = np.linalg.det(np.eye(3)[list(p)])
            phi[tuple(idx)] = s * sign
    so7 = []
    for i in range(7):
        for j in range(i + 1, 7):
            E = np.zeros((7, 7)); E[i, j] = 1; E[j, i] = -1; so7.append(E)

    def act(X):  # derivation action of X on phi
        return (np.einsum("li,ljk->ijk", X, phi) + np.einsum("lj,ilk->ijk", X, phi)
                + np.einsum("lk,ijl->ijk", X, phi))
    A = np.array([act(X).ravel() for X in so7]).T
    N = null_space(A)
    assert N.shape[1] == 14, N.shape
    mats = [sum(N[k, c] * so7[k] for k in range(21)).astype(complex) for c in range(14)]
    mats = GR.orthonormalize(mats)
    # Cartan: centraliser of a generic element
    rng = np.random.default_rng(7)
    X0 = sum(rng.standard_normal() * M for M in mats)
    B = np.array([(X0 @ M - M @ X0).ravel() for M in mats]).T
    Nc = null_space(np.concatenate([B.real, B.imag]))
    assert Nc.shape[1] == 2, Nc.shape
    tor = GR.orthonormalize([sum(Nc[k, c] * mats[k] for k in range(14)) for c in range(2)])
    basis = GR.orthonormalize(tor + mats)
    assert len(basis) == 14
    return 7, basis, tor


def build(name):
    if name.startswith("su"):
        return GR.algebra(name)
    if name == "sp2":
        return sp_algebra(2)
    if name == "sp3":
        return sp_algebra(3)
    if name == "g2":
        return g2_algebra()
    raise ValueError(name)


# ----------------------------------------------------------------------------------------------- root data
def root_data(roots):
    """roots: (N, r) array of all roots in an orthonormal torus frame. Returns a dict of structural data."""
    rng = np.random.default_rng(11)
    gen = rng.standard_normal(roots.shape[1])
    assert min(abs(roots @ gen)) > 1e-6
    pos = np.array([a for a in roots if a @ gen > 0])
    # simple roots: positive roots that are not a sum of two positive roots
    def is_root(v):
        return any(np.allclose(v, b, atol=1e-7) for b in roots)
    simple = np.array([a for a in pos if not any(is_root(a - b) and (a - b) @ gen > 0 for b in pos
                                                      if not np.allclose(a, b))])
    r = roots.shape[1]
    assert len(simple) == r, len(simple)
    cor = lambda a: 2 * a / (a @ a)  # noqa: E731
    rhov = 0.5 * sum(cor(a) for a in pos)
    rho = 0.5 * pos.sum(axis=0)
    # height via simple-root coordinates
    S = simple.T
    coords = np.array([np.linalg.lstsq(S, a, rcond=None)[0] for a in pos])
    assert np.allclose(coords, np.round(coords), atol=1e-7) and (np.round(coords) >= 0).all()
    coords = np.round(coords).astype(int)
    heights = coords.sum(axis=1)
    assert np.allclose(simple @ rhov, 1.0)                   # <alpha_i, rho-vee> = 1: rho-vee grades by height
    assert np.allclose(pos @ rhov, heights)
    it = int(np.argmax(heights)); theta = pos[it]
    assert (heights == heights[it]).sum() == 1
    thv = cor(theta)
    Sv = np.array([cor(a) for a in simple]).T
    thv_co = np.round(np.linalg.lstsq(Sv, thv, rcond=None)[0]).astype(int)
    h = int(heights[it]) + 1
    hdual = int(round(rho @ thv)) + 1
    assert len(roots) == r * h                                # |Phi| = r h
    cart = np.array([[2 * (ai @ aj) / (aj @ aj) for aj in simple] for ai in simple])
    return dict(pos=pos, simple=simple, coords=coords, heights=heights, theta=theta, theta_coords=coords[it],
                rhov=rhov, rho=rho, thv=thv, thv_coroot_coords=thv_co, h=h, hdual=hdual,
                cartan=np.round(cart).astype(int))


def angle(u, v):
    c = u @ v / np.linalg.norm(u) / np.linalg.norm(v)
    return float(np.degrees(np.arccos(np.clip(c, -1, 1))))


def top_info(pos, coords, e):
    vals = pos @ e
    m = vals.max()
    ties = [coords[k].tolist() for k in range(len(pos)) if abs(vals[k] - m) < 1e-9]
    return float(m), ties


def arc_perp(D, u0, rv):
    """Unit direction of the great arc at theta-vee. When rho-vee is parallel to theta-vee (A1, A2) the arc is not defined
    by rho-vee; then (rank 2 only) take the perpendicular to u0 in the torus with p . alpha_1 > 0, alpha_1 = first simple root
    (by the Weyl symmetry fixing rho-vee in A2 both choices are mirror images)."""
    p = rv - (rv @ u0) * u0
    if np.linalg.norm(p) > 1e-8:
        return p / np.linalg.norm(p), False
    assert len(u0) == 2
    p = np.array([-u0[1], u0[0]])
    if p @ D["simple"][0] < 0:
        p = -p
    return p, True


def arc_survey(D):
    """Great arc e(phi) = cos phi u0 + sin phi p, u0 = unit theta-vee, p = unit part of rho-vee orthogonal to u0."""
    pos, coords = D["pos"], D["coords"]
    u0 = D["thv"] / np.linalg.norm(D["thv"])
    rv = D["rhov"] / np.linalg.norm(D["rhov"])
    p, degenerate = arc_perp(D, u0, rv)
    phi_rho = float(np.arctan2(rv @ p, rv @ u0))
    e = lambda f: np.cos(f) * u0 + np.sin(f) * p  # noqa: E731
    a_u, a_p = pos @ u0, pos @ p
    # walls: alpha . e(phi) = 0 <=> tan phi = -a_u / a_p ; take phi in (-pi/2, pi)
    walls = {}
    for k in range(len(pos)):
        f = np.arctan2(-a_u[k], a_p[k])            # one zero of a_u cos + a_p sin
        for ff in (f, f + np.pi, f - np.pi):
            if -np.pi / 2 - 1e-12 < ff < np.pi + 1e-12:
                walls.setdefault(round(float(ff), 10), []).append(k)
    def wall_record(f):
        ks = [k for k in range(len(pos)) if abs(pos[k] @ e(f)) < 1e-9]
        q, ties = top_info(pos, coords, e(f))
        return dict(angle_from_thv_deg=float(np.degrees(f)), vanishing_roots_simple_coords=[coords[k].tolist() for k in ks],
                    n_vanishing_positive_roots=len(ks), top_value=q, top_roots=ties, top_unique=len(ties) == 1)
    fs = sorted(walls)
    at0 = [k for k in range(len(pos)) if abs(pos[k] @ u0) < 1e-9]
    nxt = min(f for f in fs if f > phi_rho + 1e-9)
    prev_neg = [f for f in fs if f < -1e-9]
    # where does the top root stop being theta alone along the arc (theta . e = beta . e for some other positive beta)?
    it = int(np.argmax(D["heights"]))
    tie = None
    for k in range(len(pos)):
        if k == it:
            continue
        du, dp = a_u[it] - a_u[k], a_p[it] - a_p[k]
        f = np.arctan2(-du, dp)
        for ff in (f, f + np.pi, f - np.pi):
            if 1e-9 < ff <= nxt + 1e-9:
                if tie is None or ff < tie[0] - 1e-12:
                    tie = (ff, [k])
                elif abs(ff - tie[0]) < 1e-12:
                    tie[1].append(k)
    # the arc beyond the wall leaves the dominant chamber; the region [0, nxt] is the sweep segment
    return dict(
        direction_degenerate_rho_vee_parallel_theta_vee=degenerate,
        angle_rho_vs_theta_deg=float(np.degrees(phi_rho)),
        wall_at_theta_vee=dict(angle_from_thv_deg=0.0, vanishing_roots_simple_coords=[coords[k].tolist() for k in at0],
                               n_vanishing_positive_roots=len(at0), top_unique=top_info(pos, coords, u0)[1].__len__() == 1),
        next_wall_beyond_rho_vee=wall_record(nxt),
        previous_wall_before_theta_vee=(wall_record(max(prev_neg)) if prev_neg else None),
        first_top_tie_along_arc=(None if tie is None else dict(
            angle_from_thv_deg=float(np.degrees(tie[0])),
            tied_with_simple_coords=[coords[k].tolist() for k in tie[1]],
            at_or_before_next_wall=bool(tie[0] <= nxt + 1e-9))),
        arc_in_closed_dominant_chamber_0_to_next_wall=bool(all((D["simple"] @ e(f) >= -1e-9).all()
                                                               for f in np.linspace(0, nxt, 400))),
        rho_vee_angle_in_sweep_fraction=float(phi_rho / nxt),
    )


# ----------------------------------------------------------------------------------------------- commutants
def commutant_dim(adlist, d):
    """dim of {A in End(R^d) : [ad_T, A] = 0 for all T}; the action A -> Ad_t A Ad_t^{-1} on left-invariant fields."""
    rows = []
    I = np.eye(d)
    for M in adlist:
        rows.append(np.kron(M, I) - np.kron(I, M.T))          # vec(M A - A M) row-major
    K = np.concatenate(rows)
    s = np.linalg.svd(K, compute_uv=False)
    return int(d * d - (s > 1e-8).sum())


def survey_group(label, name):
    n, basis, tor = build(name)
    d, r = len(basis), len(tor)
    C = GR.structure(basis)
    # check closure / antisymmetry (structure constants real and totally antisymmetric for an orthonormal basis)
    assert np.allclose(C, -C.transpose(1, 0, 2), atol=1e-10)
    assert np.allclose(C, -C.transpose(0, 2, 1), atol=1e-10)
    rts = GR.roots(basis, tor)
    roots = np.array([w for w, _ in rts])
    assert len(roots) == d - r
    D = root_data(roots)
    pos, coords = D["pos"], D["coords"]
    rhov, thv, theta = D["rhov"], D["thv"], D["theta"]
    u_rho, u_th = rhov / np.linalg.norm(rhov), thv / np.linalg.norm(thv)
    q_rho, ties_rho = top_info(pos, coords, u_rho)
    q_th, ties_th = top_info(pos, coords, u_th)
    reg = lambda e: bool(min(abs(pos @ e)) > TOL)  # noqa: E731
    arc = arc_survey(D)
    # torus generators as matrices (ad in the orthonormal basis); e_K matrices
    def ad_of(coeffs):
        T = sum(c * Tm for c, Tm in zip(coeffs, tor))
        return np.array([[GR.inner(B, T @ A - A @ T) for A in basis] for B in basis])
    ad_tor = [ad_of(np.eye(r)[k]) for k in range(r)]
    f_next = np.radians(arc["next_wall_beyond_rho_vee"]["angle_from_thv_deg"])
    p, _ = arc_perp(D, u_th, u_rho)
    e_next = np.cos(f_next) * u_th + np.sin(f_next) * p
    comm = dict(
        field_space_dim_End_g=d * d,
        maximal_torus=commutant_dim(ad_tor, d),
        maximal_torus_formula_r2_plus_2Npos=r * r + 2 * len(pos),
        circle_rho_vee=commutant_dim([ad_of(u_rho)], d),
        circle_theta_vee=commutant_dim([ad_of(u_th)], d),
        circle_next_wall=commutant_dim([ad_of(e_next)], d),
        diagonal_subfamily_x_w_ytheta_y=4 if r >= 2 else 3,
        per_plane_diagonal_family_x_w_plus_one_per_plane=2 + len(pos) if r >= 2 else 1 + len(pos),
    )
    # sanity of the principal grading by circle frequencies: circle commutant from the multiplicities of |q|
    vals = np.round(pos @ u_rho / (pos @ u_rho).min(), 6)
    mult = {}
    for v in vals:
        mult[v] = mult.get(v, 0) + 1
    pred = r * r + sum(2 * m * m for m in mult.values())
    assert pred == comm["circle_rho_vee"], (pred, comm["circle_rho_vee"])
    # Killing-type renormalisation: rescale so that long roots have |alpha|^2 = 2
    long2 = max(a @ a for a in pos)
    s = np.sqrt(2.0 / long2)  # roots scale by s, unit generators' q scales by s
    out = dict(
        group=label, algebra=name, defining_rep_dim=n, dim=d, rank=r, n_positive_roots=len(pos),
        cartan_matrix=D["cartan"].tolist(),
        simple_root_norms2_minus2tr=[float(a @ a) for a in D["simple"]],
        root_lengths_squared_minus2tr=sorted(set(round(float(a @ a), 9) for a in pos)),
        theta_simple_root_coords=D["theta_coords"].tolist(),
        theta_vee_simple_coroot_coords=D["thv_coroot_coords"].tolist(),
        theta_torus=theta.tolist(), theta_norm2=float(theta @ theta),
        theta_vee_torus=thv.tolist(), rho_vee_torus=rhov.tolist(), rho_vee_norm2=float(rhov @ rhov),
        rho_torus=D["rho"].tolist(),
        height_theta=int(D["h"] - 1), coxeter_h=D["h"], dual_coxeter_hvee=D["hdual"],
        angle_rho_vee_theta_vee_deg=angle(rhov, thv), angle_rho_theta_vee_deg=angle(D["rho"], thv),
        rho_vee_proportional_theta_vee=bool(angle(rhov, thv) < 1e-6),
        theta_vee_regular=reg(u_th), rho_vee_regular=reg(u_rho),
        q_K_unit_principal=q_rho, q_K_unit_principal_closed_form_h_minus_1_over_norm=float((D["h"] - 1) / np.linalg.norm(rhov)),
        q_K_unit_theta_vee=q_th, q_K_unit_theta_vee_equals_norm_theta=float(np.linalg.norm(theta)),
        q_ratio_principal_over_theta_vee=q_rho / q_th,
        q_K_killing_normalised=dict(note="roots rescaled so long roots have |alpha|^2 = 2", unit_principal=q_rho * s,
                                    unit_theta_vee=q_th * s),
        theta_unique_top_for_principal=len(ties_rho) == 1,
        top_roots_principal=ties_rho,
        theta_unique_top_for_theta_vee=len(ties_th) == 1,
        top_roots_theta_vee=ties_th,
        principal_second_highest_over_highest=float(np.sort(pos @ u_rho)[-2] / q_rho),
        principal_grading_multiplicities_by_height={int(k): int(v) for k, v in
                                                   sorted(((int(round(h)), m) for h, m in mult.items()))},
        arc=arc, commutant=comm,
    )
    return out


# ----------------------------------------------------------------------------------------------- abstract cross-check
def cartan_matrix(typ, n):
    A = 2 * np.eye(n, dtype=int)
    for i in range(n - 1):
        A[i, i + 1] = A[i + 1, i] = -1
    if typ == "A":
        return A
    # convention used below: A_ij = 2 (alpha_i, alpha_j) / (alpha_i, alpha_i)
    if typ == "B":
        A[n - 1, n - 2] = -2; return A          # alpha_n short
    if typ == "C":
        A[n - 2, n - 1] = -2; return A          # alpha_n long
    if typ == "D":
        A = 2 * np.eye(n, dtype=int)
        for i in range(n - 2):
            A[i, i + 1] = A[i + 1, i] = -1
        A[n - 3, n - 1] = A[n - 1, n - 3] = -1
        return A
    if typ == "G":
        return np.array([[2, -1], [-3, 2]])
    if typ == "F":
        return np.array([[2, -1, 0, 0], [-1, 2, -2, 0], [0, -1, 2, -1], [0, 0, -1, 2]])
    if typ == "E":
        A = 2 * np.eye(n, dtype=int)  # Bourbaki: 1-3-4-5-...; 2 attached to 4
        edges = [(0, 2), (2, 3), (3, 4), (1, 3)] + [(k, k + 1) for k in range(4, n - 1)]
        for i, j in edges:
            A[i, j] = A[j, i] = -1
        return A
    raise ValueError(typ)


def abstract_check(typ, n):
    A = cartan_matrix(typ, n)
    # symmetrise: find d_i with d_i A_ij symmetric -> Gram matrix of simple roots
    dvec = np.ones(n)
    for _ in range(3 * n):
        for i in range(n):
            for j in range(n):
                if A[i, j] != 0 and i != j:
                    dvec[j] = dvec[i] * A[i, j] / A[j, i] if A[j, i] != 0 else dvec[j]
    G = np.array([[A[i, j] * dvec[i] for j in range(n)] for i in range(n)]) / 2.0  # (alpha_i, alpha_j) up to scale
    G = (G + G.T) / 2
    assert np.allclose(G * 2 / np.diag(G)[:, None], A)
    L = np.linalg.cholesky(G)  # simple roots as rows of L
    simple = L
    # generate positive roots by simple reflections
    def refl(v, a):
        return v - 2 * (v @ a) / (a @ a) * a
    roots = [s for s in simple]
    frontier = list(roots)
    while frontier:
        new = []
        for v in frontier:
            for a in simple:
                w = refl(v, a)
                if not any(np.allclose(w, u) for u in roots):
                    roots.append(w); new.append(w)
        frontier = new
    roots = np.array(roots)
    D = root_data(roots)
    a = angle(D["rhov"], D["thv"])
    return dict(type=f"{typ}{n}", rank=n, n_roots=len(roots), coxeter_h=D["h"], dual_coxeter_hvee=D["hdual"],
                theta_simple_root_coords=D["theta_coords"].tolist(),
                angle_rho_vee_theta_vee_deg=a, rho_vee_proportional_theta_vee=bool(a < 1e-6),
                theta_vee_regular=bool(min(abs(D["pos"] @ D["thv"])) > TOL),
                n_positive_roots_orthogonal_to_theta=int((abs(D["pos"] @ D["thv"]) < TOL).sum()))


def main():
    outdir = Path(sys.argv[1]); outdir.mkdir(parents=True, exist_ok=True)
    groups = [("SU(3)", "su3"), ("SU(4)", "su4"), ("SU(5)", "su5"), ("Sp(2)", "sp2"), ("Sp(3)", "sp3"), ("G2", "g2")]
    res = dict(
        task="arc deep-dive C2: structural survey of the principal circle (no floor, edge, spectrum or competitor)",
        conventions=dict(
            inner_product="<X,Y> = -2 tr(XY) in the defining representation (7-dim for G2); torus basis orthonormal",
            roots="eigenvalues of i*ad(T_k) on the orthonormal torus basis; identified with vectors in R^rank",
            coroot="alpha-vee = 2 alpha / |alpha|^2",
            q_K="<theta, e_K> for a unit generator e_K of the torus (convention dependent: scales with the inner product)",
            positive_system="a fixed random generic functional; theta = unique root of maximal height",
            arc="e(phi) = cos(phi) u0 + sin(phi) p, u0 = unit theta-vee, p = unit component of rho-vee orthogonal to u0",
            commutant="dim of {A in End(g): A commutes with ad(T)}, i.e. left-invariant fields A: g -> g invariant under "
                      "A -> Ad_t A Ad_t^-1, for the maximal torus and for single circles",
        ),
        groups=[survey_group(lab, nm) for lab, nm in groups],
    )
    # matrix vs abstract agreement for the six
    abstract = []
    for typ, ns in (("A", range(1, 9)), ("B", range(2, 9)), ("C", range(3, 9)), ("D", range(4, 9)),
                    ("E", (6, 7, 8)), ("F", (4,)), ("G", (2,))):
        for n in ns:
            abstract.append(abstract_check(typ, n))
    res["abstract_crosscheck_all_simple_types_rank_le_8"] = abstract
    match = {"SU(3)": "A2", "SU(4)": "A3", "SU(5)": "A4", "Sp(2)": "B2", "Sp(3)": "C3", "G2": "G2"}
    agree = {}
    for g in res["groups"]:
        a = next(x for x in abstract if x["type"] == match[g["group"]])
        agree[g["group"]] = dict(abstract_type=a["type"],
                                 angle_agrees=abs(a["angle_rho_vee_theta_vee_deg"] - g["angle_rho_vee_theta_vee_deg"]) < 1e-6,
                                 coxeter_agrees=a["coxeter_h"] == g["coxeter_h"] and a["dual_coxeter_hvee"] == g["dual_coxeter_hvee"],
                                 regular_agrees=a["theta_vee_regular"] == g["theta_vee_regular"])
    res["matrix_vs_abstract_agreement"] = agree
    res["rho_vee_proportional_theta_vee_types"] = [x["type"] for x in abstract if x["rho_vee_proportional_theta_vee"]]
    res["theta_vee_regular_types"] = [x["type"] for x in abstract if x["theta_vee_regular"]]
    (outdir / "principal_survey.json").write_text(json.dumps(res, indent=1, default=float))
    for g in res["groups"]:
        a = g["arc"]; nw = a["next_wall_beyond_rho_vee"]
        print(f"{g['group']:6s} h={g['coxeter_h']} hv={g['dual_coxeter_hvee']} ht={g['height_theta']} "
              f"ang={g['angle_rho_vee_theta_vee_deg']:.4f} thv_reg={g['theta_vee_regular']} "
              f"qP={g['q_K_unit_principal']:.6f} qT={g['q_K_unit_theta_vee']:.6f} "
              f"next_wall={nw['angle_from_thv_deg']:.4f} vanish={nw['vanishing_roots_simple_coords']} "
              f"top_unique={nw['top_unique']} tie={a['first_top_tie_along_arc']} comm={g['commutant']}")
    print("rho-vee || theta-vee:", res["rho_vee_proportional_theta_vee_types"])
    print("theta-vee regular:", res["theta_vee_regular_types"])
    print("agreement:", agree)


if __name__ == "__main__":
    main()
