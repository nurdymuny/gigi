"""
Rigorous validation tests for the GIGI Kähler upgrade catalog (v0).

Design discipline:
- Each test compares a numerical computation to an INDEPENDENT ground truth
- Ground truths come from closed-form analytical results derived in a
  different formalism than the numerical test computation
- Toy data is constructed using primitives independent of the property
  being tested (e.g., group multiplication tables rather than "graphs known
  to satisfy commutativity")
- Each test includes a NEGATIVE case (the property must fail for the wrong
  data) to rule out trivially-passing tests where everything just returns 0
"""

import torch
import math
from itertools import permutations


# ============================================================
# Utility: integrate Jacobi equation J'' + K J = 0
# This is the geodesic-deviation ODE; nothing about Hadamard,
# trajectory balls, or conjugate points is assumed in writing it.
# ============================================================

def jacobi_field(K_curvature, T, n_steps=10000):
    """RK4 integration of J'' + K·J = 0 with J(0)=0, J'(0)=1."""
    dt = T / n_steps
    state = torch.tensor([0.0, 1.0], dtype=torch.float64)
    times = [0.0]
    Js = [0.0]
    K = float(K_curvature)
    
    def rhs(s):
        return torch.stack([s[1], -K * s[0]])
    
    for i in range(n_steps):
        k1 = rhs(state)
        k2 = rhs(state + 0.5 * dt * k1)
        k3 = rhs(state + 0.5 * dt * k2)
        k4 = rhs(state + dt * k3)
        state = state + (dt / 6.0) * (k1 + 2*k2 + 2*k3 + k4)
        times.append((i + 1) * dt)
        Js.append(state[0].item())
    
    return torch.tensor(times, dtype=torch.float64), torch.tensor(Js, dtype=torch.float64)


# ============================================================
# Test 1: Kähler graph dual-adjacency commutativity (item 1.1)
# ============================================================

def test_1_kahler_commutativity():
    """
    CLAIM: On vertex-transitive Kähler graphs, principal and auxiliary
    adjacency operators commute.
    
    GROUND TRUTH (independent): For Cayley graph Cay(G, S), the adjacency
    operator is right-convolution by S in the group algebra C[G]. Two
    adjacencies commute iff their generating sets commute as group-algebra
    elements. For abelian G, ANY two generators commute (C[G] is commutative).
    For non-abelian G, generic pairs don't.
    
    NON-CIRCULAR: we build adjacencies from group multiplication tables,
    never reference commutativity in the construction.
    """
    print("\n=== Test 1: Kähler graph commutativity ===")
    
    # Positive: Z/4 × Z/4 (abelian)
    n = 4
    N = n * n
    vertices = [(i, j) for i in range(n) for j in range(n)]
    idx = {v: k for k, v in enumerate(vertices)}
    
    A_p = torch.zeros(N, N, dtype=torch.float64)
    A_a = torch.zeros(N, N, dtype=torch.float64)
    
    for v in vertices:
        for shift in [(1, 0), (-1, 0)]:
            w = ((v[0] + shift[0]) % n, (v[1] + shift[1]) % n)
            A_p[idx[v], idx[w]] = 1
        for shift in [(0, 1), (0, -1)]:
            w = ((v[0] + shift[0]) % n, (v[1] + shift[1]) % n)
            A_a[idx[v], idx[w]] = 1
    
    comm_abelian = A_p @ A_a - A_a @ A_p
    err_abelian = comm_abelian.abs().max().item()
    
    # Negative: S_3 Cayley graph with two single (non-central) transpositions.
    # CAUTION: using {g_cyc, g_cyc^{-1}} as a generator set is a TRAP — that's
    # the full conjugacy class of 3-cycles in S_3, and class sums are central
    # in the group algebra, so they commute with everything (NOT because of
    # Kähler structure). We use two individual transpositions instead.
    S3 = list(permutations(range(3)))
    idx_s = {p: k for k, p in enumerate(S3)}
    
    def compose(p, q):  # p ∘ q, i.e. apply q first
        return tuple(p[q[i]] for i in range(3))
    
    g_trans_1 = (1, 0, 2)    # transposition (0 1)
    g_trans_2 = (0, 2, 1)    # transposition (1 2) — different non-central involution
    
    # Sanity: they don't commute in the group
    assert compose(g_trans_1, g_trans_2) != compose(g_trans_2, g_trans_1)
    # Sanity: each is its own inverse (so single-element generating set is symmetric)
    assert compose(g_trans_1, g_trans_1) == (0, 1, 2)
    assert compose(g_trans_2, g_trans_2) == (0, 1, 2)
    # Sanity: neither single transposition is the full transposition class (which would be central)
    transpositions = [p for p in S3 if p != (0,1,2) and compose(p, p) == (0,1,2)]
    assert len(transpositions) == 3
    
    B_p = torch.zeros(6, 6, dtype=torch.float64)
    B_a = torch.zeros(6, 6, dtype=torch.float64)
    for x in S3:
        y1 = compose(x, g_trans_1)
        B_p[idx_s[x], idx_s[y1]] = 1
        y2 = compose(x, g_trans_2)
        B_a[idx_s[x], idx_s[y2]] = 1
    
    # Both adjacency matrices should be symmetric (sanity)
    assert (B_p - B_p.T).abs().max() < 1e-15
    assert (B_a - B_a.T).abs().max() < 1e-15
    
    comm_nonab = B_p @ B_a - B_a @ B_p
    err_nonab = comm_nonab.abs().max().item()
    
    print(f"  Abelian Z/4 × Z/4: |[A_p, A_a]|_∞ = {err_abelian:.2e}")
    print(f"  Non-abelian S_3:   |[B_p, B_a]|_∞ = {err_nonab:.2e}")
    print(f"  Ratio non-ab/ab = {err_nonab / max(err_abelian, 1e-30):.2e}")
    
    passed = (err_abelian < 1e-12) and (err_nonab > 0.5)
    print(f"  {'PASS' if passed else 'FAIL'}")
    return passed


# ============================================================
# Test 2: Magnetic trajectory on flat R² (item 1.2)
# ============================================================

def test_2_magnetic_trajectory():
    """
    CLAIM: A closed 2-form B perturbs geodesics into well-defined trajectories
    via the magnetic Lagrangian L = ½|v|² + ⟨A, v⟩ with dA = B.
    
    GROUND TRUTH (independent): on flat R² with constant B = b·dx∧dy, classical
    cyclotron motion gives circles of radius r = |v|/b, period T = 2π/b,
    energy ½|v|² conserved.
    
    NON-CIRCULAR: we derive the Euler-Lagrange equations from the magnetic
    Lagrangian (Landau gauge A = -by dx), integrate with RK4, and compare the
    resulting trajectory to the cyclotron circle. The integration uses only
    Newton-style force from the magnetic potential, not the cyclotron formula.
    """
    print("\n=== Test 2: Magnetic trajectory on flat R² ===")
    
    b = 1.5
    v0 = torch.tensor([1.0, 0.0], dtype=torch.float64)
    x0 = torch.tensor([0.0, 0.0], dtype=torch.float64)
    
    # E-L from L = ½(ẋ²+ẏ²) - by·ẋ gives ẍ = bẏ, ÿ = -bẋ
    # (computed by hand; we verify the resulting trajectory matches cyclotron)
    
    def rhs(state):
        vx, vy = state[2], state[3]
        return torch.stack([vx, vy, b * vy, -b * vx])
    
    state = torch.cat([x0, v0])
    dt = 1e-4
    T = 2 * math.pi / b
    n_steps = int(round(T / dt))
    
    positions = [state[:2].clone()]
    energies = [0.5 * (state[2]**2 + state[3]**2).item()]
    
    for _ in range(n_steps):
        k1 = rhs(state)
        k2 = rhs(state + 0.5 * dt * k1)
        k3 = rhs(state + 0.5 * dt * k2)
        k4 = rhs(state + dt * k3)
        state = state + (dt / 6.0) * (k1 + 2*k2 + 2*k3 + k4)
        positions.append(state[:2].clone())
        energies.append(0.5 * (state[2]**2 + state[3]**2).item())
    
    positions = torch.stack(positions)
    
    # Expected: circle radius |v|/b, center perpendicular to initial velocity
    # Initial v=(1,0), initial acc=(0,-b), so center at (0, -1/b)
    expected_radius = 1.0 / b
    expected_center = torch.tensor([0.0, -1.0 / b], dtype=torch.float64)
    
    distances = torch.norm(positions - expected_center, dim=1)
    radius_error = (distances - expected_radius).abs().max().item()
    energy_drift = max(energies) - min(energies)
    closure_error = torch.norm(positions[-1] - positions[0]).item()
    
    # Negative case: with b=0 (no magnetic field), trajectory should be a STRAIGHT LINE,
    # NOT a circle. Verify that the same integrator doesn't spuriously curve.
    b_neg = 0.0
    state_n = torch.cat([x0, v0])
    positions_n = [state_n[:2].clone()]
    for _ in range(n_steps):
        def rhs_n(s):
            return torch.stack([s[2], s[3], b_neg * s[3], -b_neg * s[2]])
        k1 = rhs_n(state_n)
        k2 = rhs_n(state_n + 0.5 * dt * k1)
        k3 = rhs_n(state_n + 0.5 * dt * k2)
        k4 = rhs_n(state_n + dt * k3)
        state_n = state_n + (dt / 6.0) * (k1 + 2*k2 + 2*k3 + k4)
        positions_n.append(state_n[:2].clone())
    positions_n = torch.stack(positions_n)
    # Trajectory should be along x-axis: y stays at 0, x grows linearly
    max_y_drift = positions_n[:, 1].abs().max().item()
    
    print(f"  Magnetic case (b={b}):")
    print(f"    Expected cyclotron radius: {expected_radius:.6f}")
    print(f"    Max deviation from circle: {radius_error:.2e}")
    print(f"    Energy drift over period:  {energy_drift:.2e}")
    print(f"    Period closure error:      {closure_error:.2e}")
    print(f"  Control (b=0): max |y| should be ~0: {max_y_drift:.2e}")
    
    passed = (radius_error < 1e-5) and (energy_drift < 1e-5) \
             and (closure_error < 1e-3) and (max_y_drift < 1e-10)
    print(f"  {'PASS' if passed else 'FAIL'}")
    return passed


# ============================================================
# Test 3: Hadamard-Cartan via Jacobi fields (items 1.4, 1.5)
# ============================================================

def test_3_hadamard_cartan():
    """
    CLAIM: On Hadamard manifolds (K ≤ 0), exp_p is a diffeomorphism (no
    conjugate points). On positively curved manifolds, conjugate points
    appear.
    
    GROUND TRUTH: Closed-form Jacobi fields:
      K = 0  → J(t) = t       (never zero for t > 0)
      K = -1 → J(t) = sinh(t) (never zero for t > 0; Hadamard)
      K = +1 → J(t) = sin(t)  (first zero at t = π; conjugate point)
    
    NON-CIRCULAR: we solve the Jacobi ODE numerically with no reference to
    the closed forms, and compare. The conjugate-point detection is via
    sign change in the numerical solution, not by knowing where it should be.
    """
    print("\n=== Test 3: Hadamard-Cartan via Jacobi fields ===")
    
    T = 4.0
    ts_e, Js_e = jacobi_field(0.0, T)
    ts_h, Js_h = jacobi_field(-1.0, T)
    ts_s, Js_s = jacobi_field(1.0, T)
    
    err_e = (Js_e - ts_e).abs().max().item()
    rel_err_h = ((Js_h - torch.sinh(ts_h)).abs().max() / torch.sinh(ts_h).max()).item()
    err_s = (Js_s - torch.sin(ts_s)).abs().max().item()
    
    # First conjugate point on S² (linear interpolation at sign change)
    sign_changes = (Js_s[1:] * Js_s[:-1]) < 0
    sc_idx = sign_changes.nonzero(as_tuple=True)[0]
    if len(sc_idx) > 0:
        i = sc_idx[0].item()
        t1, t2 = ts_s[i].item(), ts_s[i+1].item()
        J1, J2 = Js_s[i].item(), Js_s[i+1].item()
        first_zero = t1 - J1 * (t2 - t1) / (J2 - J1)
        conj_error = abs(first_zero - math.pi)
    else:
        first_zero = None
        conj_error = float('inf')
    
    # H² Jacobi field must be positive and monotone increasing
    h_positive = (Js_h[1:] > 0).all().item()
    h_monotone = (Js_h[1:] > Js_h[:-1] - 1e-10).all().item()
    
    print(f"  Euclidean (K=0):  err vs t      = {err_e:.2e}")
    print(f"  Hyperbolic (K=-1): rel err vs sinh = {rel_err_h:.2e}")
    print(f"  Spherical (K=+1):  err vs sin   = {err_s:.2e}")
    print(f"  S² first conjugate point: t = {first_zero:.6f} (expected π = {math.pi:.6f})")
    print(f"  H² Jacobi field positive everywhere: {h_positive}")
    print(f"  H² Jacobi field monotone: {h_monotone}")
    
    passed = (err_e < 1e-6 and rel_err_h < 1e-6 and err_s < 1e-6
              and conj_error < 1e-3 and h_positive and h_monotone)
    print(f"  {'PASS' if passed else 'FAIL'}")
    return passed


# ============================================================
# Test 4: Trajectory-ball volume (item 1.3)
# ============================================================

def test_4_trajectory_ball_volume():
    """
    CLAIM: Volumes of geodesic balls obey curvature-comparison theorems
    (Bishop-Gromov-Günther).
    
    GROUND TRUTH (independent closed form):
      H² (K=-1): V(R) = 2π(cosh R - 1)  → exponential growth
      R² (K=0):  V(R) = πR²              → polynomial growth
      S² (K=+1): V(R) = 2π(1 - cos R)    → saturating growth
    
    NON-CIRCULAR: we compute V(R) by integrating the area element 2π·J(r) dr
    where J(r) comes from the independently-integrated Jacobi ODE. We then
    compare to the closed-form V(R). The qualitative growth comparison
    (exponential vs poly vs sub-poly) is itself a falsifiable check.
    """
    print("\n=== Test 4: Trajectory-ball volume ===")
    
    R = 2.0
    
    ts_h, Js_h = jacobi_field(-1.0, R, n_steps=20000)
    V_h_num = 2 * math.pi * torch.trapz(Js_h, ts_h).item()
    V_h_exact = 2 * math.pi * (math.cosh(R) - 1)
    err_h = abs(V_h_num - V_h_exact) / V_h_exact
    
    ts_e, Js_e = jacobi_field(0.0, R, n_steps=20000)
    V_e_num = 2 * math.pi * torch.trapz(Js_e, ts_e).item()
    V_e_exact = math.pi * R**2
    err_e = abs(V_e_num - V_e_exact) / V_e_exact
    
    R_s = min(R, math.pi - 0.01)  # stay before conjugate point
    ts_s, Js_s = jacobi_field(1.0, R_s, n_steps=20000)
    V_s_num = 2 * math.pi * torch.trapz(Js_s, ts_s).item()
    V_s_exact = 2 * math.pi * (1 - math.cos(R_s))
    err_s = abs(V_s_num - V_s_exact) / V_s_exact
    
    growth_h_e = V_h_num / V_e_num
    growth_s_e = V_s_num / V_e_num
    
    print(f"  R = {R}")
    print(f"  H² (K=-1): V_num={V_h_num:.6f}, V_exact={V_h_exact:.6f}, rel err={err_h:.2e}")
    print(f"  R² (K=0):  V_num={V_e_num:.6f}, V_exact={V_e_exact:.6f}, rel err={err_e:.2e}")
    print(f"  S² (K=+1): V_num={V_s_num:.6f}, V_exact={V_s_exact:.6f}, rel err={err_s:.2e}")
    print(f"  V_H/V_E = {growth_h_e:.4f} (must be > 1: negative curvature grows faster)")
    print(f"  V_S/V_E = {growth_s_e:.4f} (must be < 1: positive curvature grows slower)")
    
    passed = (err_h < 1e-3 and err_e < 1e-3 and err_s < 1e-3
              and growth_h_e > 1.0 and growth_s_e < 1.0)
    print(f"  {'PASS' if passed else 'FAIL'}")
    return passed


# ============================================================
# Test 5: Moment map / Noether conservation (item 2.3)
# ============================================================

def test_5_moment_map():
    """
    CLAIM: For a Hamiltonian system on T*M with symmetry group G, the moment
    map μ: T*M → 𝔤* is conserved by the flow iff H is G-invariant.
    
    GROUND TRUTH (independent): classical Noether theorem. For SO(2) action
    on T*R² by rotations, μ = x·p_y - y·p_x (angular momentum). Conserved
    under radially-symmetric H, NOT conserved under anisotropic H.
    
    NON-CIRCULAR: we integrate Hamilton's equations directly via ∂H/∂x and
    ∂H/∂p without reference to angular momentum. We then measure μ along
    the trajectory. The conservation/violation result is independent of any
    knowledge of Noether's theorem.
    """
    print("\n=== Test 5: Moment map / Noether conservation ===")
    
    def integrate(grad_H, state, T=10.0, dt=1e-3):
        n_steps = int(round(T / dt))
        states = [state.clone()]
        for _ in range(n_steps):
            def rhs(s):
                gH = grad_H(s)
                return torch.stack([gH[2], gH[3], -gH[0], -gH[1]])
            k1 = rhs(state)
            k2 = rhs(state + 0.5 * dt * k1)
            k3 = rhs(state + 0.5 * dt * k2)
            k4 = rhs(state + dt * k3)
            state = state + (dt / 6.0) * (k1 + 2*k2 + 2*k3 + k4)
            states.append(state.clone())
        return torch.stack(states)
    
    # Symmetric: H = (p² + r²)/2 → ∇H = (x, y, p_x, p_y)
    def grad_sym(s):
        return torch.stack([s[0], s[1], s[2], s[3]])
    
    # Asymmetric: H = (p²)/2 + x² → ∇H = (2x, 0, p_x, p_y)
    def grad_asym(s):
        return torch.stack([2*s[0], torch.zeros_like(s[1]), s[2], s[3]])
    
    init = torch.tensor([1.0, 0.5, 0.3, 0.7], dtype=torch.float64)
    
    states_sym = integrate(grad_sym, init.clone())
    states_asym = integrate(grad_asym, init.clone())
    
    mu_sym = states_sym[:, 0] * states_sym[:, 3] - states_sym[:, 1] * states_sym[:, 2]
    mu_asym = states_asym[:, 0] * states_asym[:, 3] - states_asym[:, 1] * states_asym[:, 2]
    
    drift_sym = (mu_sym.max() - mu_sym.min()).item()
    drift_asym = (mu_asym.max() - mu_asym.min()).item()
    
    # Also check that energy IS conserved in both cases (sanity for the integrator)
    def H_sym(s): return 0.5*(s[2]**2+s[3]**2) + 0.5*(s[0]**2+s[1]**2)
    def H_asym(s): return 0.5*(s[2]**2+s[3]**2) + s[0]**2
    E_sym = torch.stack([H_sym(s) for s in states_sym])
    E_asym = torch.stack([H_asym(s) for s in states_asym])
    energy_drift_sym = (E_sym.max() - E_sym.min()).item()
    energy_drift_asym = (E_asym.max() - E_asym.min()).item()
    
    print(f"  Initial μ = {(init[0]*init[3] - init[1]*init[2]).item():.6f}")
    print(f"  Symmetric H (SO(2)-invariant): μ drift = {drift_sym:.2e}")
    print(f"  Asymmetric H (broken):         μ drift = {drift_asym:.4f}")
    print(f"  Sanity (energy conserved in both): "
          f"sym E drift = {energy_drift_sym:.2e}, asym E drift = {energy_drift_asym:.2e}")
    
    passed = (drift_sym < 1e-5 and drift_asym > 0.5
              and energy_drift_sym < 1e-5 and energy_drift_asym < 1e-5)
    print(f"  {'PASS' if passed else 'FAIL'}")
    return passed


# ============================================================
# Test 6: Spectral gap vs mixing time (item 2.5)
# ============================================================

def test_6_spectral_gap():
    """
    CLAIM: Spectral gap λ_2 of the normalized Laplacian controls the mixing
    time of the lazy random walk: τ_mix ~ (1/λ_2) · log(1/ε).
    
    GROUND TRUTH (independent topology): Path P_n has gap O(1/n²), mixes
    slowly. Cycle C_n similar but slightly faster (no boundary). Complete
    graph K_n has gap = n/(n-1) ≈ 1, mixes in O(log n) steps.
    
    NON-CIRCULAR: spectral gap from eigendecomposition of the symmetric
    normalized Laplacian. Mixing time from direct simulation of the lazy
    random walk's distribution, measured by TV distance to stationary.
    These two computations share NO algorithmic structure beyond using
    the same adjacency matrix as input.
    """
    print("\n=== Test 6: Spectral gap vs mixing time ===")
    
    def normalized_laplacian_gap(A):
        n = A.shape[0]
        d = A.sum(dim=1)
        D_inv_sqrt = torch.diag(1.0 / torch.sqrt(d))
        L = torch.eye(n, dtype=torch.float64) - D_inv_sqrt @ A @ D_inv_sqrt
        eigvals = torch.sort(torch.linalg.eigvalsh(L)).values
        return eigvals[1].item()  # smallest nonzero
    
    def lazy_mixing_time(A, eps=0.05, max_steps=200000):
        n = A.shape[0]
        d = A.sum(dim=1)
        P = A / d[:, None]
        P_lazy = 0.5 * (torch.eye(n, dtype=torch.float64) + P)
        pi = d / d.sum()
        p = torch.zeros(n, dtype=torch.float64)
        p[0] = 1.0
        for t in range(1, max_steps + 1):
            p = P_lazy.T @ p
            tv = 0.5 * (p - pi).abs().sum().item()
            if tv < eps:
                return t
        return max_steps
    
    n = 20
    
    A_cycle = torch.zeros(n, n, dtype=torch.float64)
    for i in range(n):
        A_cycle[i, (i+1) % n] = 1
        A_cycle[i, (i-1) % n] = 1
    
    A_complete = torch.ones(n, n, dtype=torch.float64) - torch.eye(n, dtype=torch.float64)
    
    A_path = torch.zeros(n, n, dtype=torch.float64)
    for i in range(n - 1):
        A_path[i, i+1] = 1
        A_path[i+1, i] = 1
    
    results = {}
    for name, A in [("Cycle C_n", A_cycle), ("Complete K_n", A_complete), ("Path P_n", A_path)]:
        gap = normalized_laplacian_gap(A)
        mix = lazy_mixing_time(A)
        results[name] = (gap, mix)
        print(f"  {name:14s}: gap = {gap:.4f}, mix τ = {mix:6d}, gap·τ = {gap*mix:.2f}")
    
    # Ordering checks (independent of the gap·τ product)
    gap_C = results["Complete K_n"][0]
    gap_cy = results["Cycle C_n"][0]
    gap_P = results["Path P_n"][0]
    mix_C = results["Complete K_n"][1]
    mix_cy = results["Cycle C_n"][1]
    mix_P = results["Path P_n"][1]
    
    ordering_ok = (gap_C > gap_cy > gap_P) and (mix_C < mix_cy < mix_P)
    
    # Coupling check: gap × mix should be in O(log n) ≈ 3-30 range
    products = [results[k][0] * results[k][1] for k in results]
    coupling_ok = (max(products) / min(products)) < 50
    
    print(f"  Ordering (gap_C > gap_cy > gap_P and mix opposite): {ordering_ok}")
    print(f"  Coupling (all gap·τ within 50× of each other):     {coupling_ok}")
    
    passed = ordering_ok and coupling_ok
    print(f"  {'PASS' if passed else 'FAIL'}")
    return passed


# ============================================================
# Main
# ============================================================

def main():
    torch.set_printoptions(precision=8)
    tests = [
        test_1_kahler_commutativity,
        test_2_magnetic_trajectory,
        test_3_hadamard_cartan,
        test_4_trajectory_ball_volume,
        test_5_moment_map,
        test_6_spectral_gap,
    ]
    
    results = []
    for t in tests:
        try:
            ok = t()
        except Exception as e:
            print(f"  EXCEPTION in {t.__name__}: {e}")
            import traceback; traceback.print_exc()
            ok = False
        results.append((t.__name__, ok))
    
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    for name, ok in results:
        print(f"  {name}: {'PASS' if ok else 'FAIL'}")
    n_pass = sum(1 for _, ok in results if ok)
    print(f"\n{n_pass}/{len(results)} tests passed.")
    return n_pass == len(results)


if __name__ == "__main__":
    main()
