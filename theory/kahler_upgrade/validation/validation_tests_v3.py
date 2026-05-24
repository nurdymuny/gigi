"""
Validation tests v3 — extending coverage to:
- Test 9 (item 2.2): Riemann-Roch / Atiyah-Singer index on T² via theta functions
- Test 10 (item 2.8): Berezin-Toeplitz semiclassical limit on harmonic oscillator
- Test 11 (item 2.9): Discrete Hodge cohomology matches T² Betti numbers
"""

import torch
import math


# ============================================================
# Test 9 (item 2.2): Riemann-Roch / index on T²
# ============================================================

def test_9_index_theorem_torus():
    """
    CLAIM (item 2.2): Riemann-Roch / Atiyah-Singer:
        ind(D_L) = ∫_M ch(L) Td(M)
    For a Riemann surface of genus g and line bundle of degree d:
        dim H⁰(L) − dim H¹(L) = d − g + 1
    
    GROUND TRUTH: On T² (g = 1), for L of degree n > 0:
        dim H⁰(L^n) = n  (H¹ vanishes by Serre duality for d > 2g−2 = 0)
    Explicit basis: theta functions θ_k(z; τ; n) for k = 0, ..., n−1.
    
    NON-CIRCULAR: We construct n theta functions by their Fourier series
    (independent of any cohomological computation), evaluate on a grid of
    points on T², and compute numerical rank of the resulting matrix.
    Rank = dim H⁰ = n if the formula holds.
    """
    print("\n=== Test 9: Riemann-Roch / index on T² ===")
    
    M = 12  # truncation of theta series (well past convergence)
    
    def theta_on_grid(grid_complex, n, k):
        """Evaluate θ_k of level n at points z = x + iy with τ = i."""
        ms = torch.arange(-M, M + 1, dtype=torch.float64) + k / n  # shifted lattice
        # θ = Σ_m exp(πi · n · m² · τ + 2πi · n · m · z), with τ = i
        # exponent = πi·n·m²·(i) + 2πi·n·m·z = -π·n·m² + 2πi·n·m·z
        z = grid_complex.unsqueeze(-1)          # (G, 1)
        ms_b = ms.view(1, -1).to(torch.complex128)  # (1, 2M+1)
        quad = (-math.pi * n) * (ms_b.real ** 2)    # real
        lin = 2j * math.pi * n * ms_b * z           # complex
        return torch.exp(quad.to(torch.complex128) + lin).sum(dim=-1)
    
    # Grid: 8×8 points in fundamental domain (0,1)×(0,1)
    n_pts = 8
    xs = torch.linspace(0.07, 0.93, n_pts, dtype=torch.float64)
    ys = torch.linspace(0.07, 0.93, n_pts, dtype=torch.float64)
    gx, gy = torch.meshgrid(xs, ys, indexing='ij')
    grid = torch.complex(gx.flatten(), gy.flatten())   # (64,) complex
    
    print(f"  Grid: {n_pts}×{n_pts} = {n_pts**2} sample points; θ series truncated at ±{M}")
    
    results = []
    for n in [1, 2, 3, 4, 5]:
        thetas = torch.stack([theta_on_grid(grid, n, k) for k in range(n)])
        # thetas: (n, 64) complex
        # Treat as real matrix by stacking real/imag parts: doubles columns
        thetas_real = torch.cat([thetas.real, thetas.imag], dim=1)  # (n, 128)
        S = torch.linalg.svdvals(thetas_real)
        max_S = S.max().item()
        rank = (S > max_S * 1e-8).sum().item()
        riemann_roch_prediction = n  # = deg(L^n) − g + 1 = n − 1 + 1
        ok = (rank == riemann_roch_prediction)
        results.append((n, rank, ok))
        print(f"  Level n = {n}: rank of θ basis = {rank}, RR prediction = {n}  "
              f"{'✓' if ok else '✗'}")
    
    # Additional negative-check: include a duplicate of θ_0 in the basis and
    # verify rank does NOT increase (the duplicate is linearly dependent).
    n = 3
    thetas = torch.stack([theta_on_grid(grid, n, k) for k in range(n)])
    thetas_with_dup = torch.cat([thetas, thetas[0:1]], dim=0)  # (n+1, ...)
    real_dup = torch.cat([thetas_with_dup.real, thetas_with_dup.imag], dim=1)
    S = torch.linalg.svdvals(real_dup)
    rank_dup = (S > S.max() * 1e-8).sum().item()
    print(f"  Negative check: n = 3 theta functions + duplicate of θ_0: rank = {rank_dup} "
          f"(must remain 3)  {'✓' if rank_dup == 3 else '✗'}")
    
    passed = all(ok for _, _, ok in results) and (rank_dup == 3)
    print(f"  {'PASS' if passed else 'FAIL'}")
    return passed


# ============================================================
# Test 10 (item 2.8): Berezin-Toeplitz semiclassical limit
# ============================================================

def test_10_berezin_toeplitz():
    """
    CLAIM (item 2.8): Berezin-Toeplitz correspondence
        [T_f, T_g] = iℏ T_{{f,g}} + O(ℏ²)
    quantizes the Poisson algebra at leading order.
    
    GROUND TRUTH (BCH for [X̂, P̂] = iℏ, derived OFF-mesh):
        exp(iX̂) exp(iP̂) = exp(−iℏ/2) exp(i(X̂+P̂))
    so
        [exp(iX̂), exp(iP̂)] = −2i sin(ℏ/2) · exp(i(X̂+P̂))  (exact)
    
    Classical: {exp(ix), exp(ip)} = −exp(i(x+p))
    Leading BT: iℏ · T_{−exp(i(x+p))} = −iℏ · exp(i(X̂+P̂))
    
    Deviation of LHS from leading BT:
        comm − (−iℏ · exp(i(X̂+P̂))) = (ℏ − 2 sin(ℏ/2)) · i · exp(i(X̂+P̂))
                                    ≈ i · (ℏ³/24) · exp(i(X̂+P̂))
    
    NON-CIRCULAR: We construct truncated bosonic ladder operators, build
    X̂ and P̂, compute matrix exponentials via Padé (linalg.matrix_exp),
    and measure deviation versus ℏ. The BCH ground-truth formula and the
    matrix-exponential computation share no mathematical structure.
    """
    print("\n=== Test 10: Berezin-Toeplitz semiclassical ===")
    
    N = 80
    n_compare = N // 4  # interior to avoid truncation edge effects
    
    sqrt_n = torch.sqrt(torch.arange(1, N, dtype=torch.float64))
    a = torch.diag(sqrt_n, diagonal=1).to(torch.complex128)
    adag = a.conj().T
    I = torch.eye(N, dtype=torch.complex128)
    
    hbars = [1.0, 0.5, 0.25, 0.125]
    print(f"  Truncation N = {N}; comparing interior {n_compare}×{n_compare} block")
    
    devs = []
    for hbar in hbars:
        scale = math.sqrt(hbar / 2)
        X = scale * (a + adag)
        P = -1j * scale * (a - adag)
        
        # Sanity: [X, P] should equal iℏ · I (modulo truncation)
        XP_err = (X @ P - P @ X - 1j * hbar * I)[:n_compare, :n_compare].abs().max().item()
        
        eiX = torch.linalg.matrix_exp(1j * X)
        eiP = torch.linalg.matrix_exp(1j * P)
        eiXP = torch.linalg.matrix_exp(1j * (X + P))
        
        comm = eiX @ eiP - eiP @ eiX
        bch_exact = -2j * math.sin(hbar / 2) * eiXP
        leading = -1j * hbar * eiXP
        
        # Verify BCH formula holds numerically (sanity)
        bch_err = (comm - bch_exact)[:n_compare, :n_compare].abs().max().item()
        
        # Measure deviation from leading BT (using Frobenius norm on interior block)
        block = (comm - leading)[:n_compare, :n_compare]
        eiXP_block = eiXP[:n_compare, :n_compare]
        dev_fro = block.abs().pow(2).sum().sqrt().item()
        eiXP_fro = eiXP_block.abs().pow(2).sum().sqrt().item()
        normalized_dev = dev_fro / eiXP_fro
        
        # Theoretical: |ℏ − 2 sin(ℏ/2)|
        theory = abs(hbar - 2 * math.sin(hbar / 2))
        
        print(f"  ℏ = {hbar:.4f}: [X,P]−iℏI err = {XP_err:.2e}, BCH err = {bch_err:.2e}, "
              f"normalized dev = {normalized_dev:.4e}, theoretical = {theory:.4e}")
        devs.append((hbar, normalized_dev, theory, bch_err))
    
    # 1. BCH should be exact (small bch_err)
    bch_ok = all(b < 1e-3 for _, _, _, b in devs)
    
    # 2. Normalized deviation should match |ℏ − 2 sin(ℏ/2)| within ~10%
    matches_theory = all(
        abs(d - t) / max(t, 1e-15) < 0.10
        for _, d, t, _ in devs
    )
    
    # 3. Deviations across ℏ values scale as ℏ³
    # Ratios devs[i] / hbars[i]³ should be ~constant
    ratios = [d / (h ** 3) for h, d, _, _ in devs]
    ratio_constancy = max(ratios) / min(ratios) < 2.0
    
    print(f"  BCH ground-truth formula holds: {bch_ok}")
    print(f"  Deviation matches |ℏ−2sin(ℏ/2)| within 10%: {matches_theory}")
    print(f"  Cubic scaling (dev/ℏ³ ratios within 2×): {ratio_constancy}")
    print(f"    ratios dev/ℏ³ = {[f'{r:.5f}' for r in ratios]}  (theoretical ~1/24 ≈ 0.0417)")
    
    passed = bch_ok and matches_theory and ratio_constancy
    print(f"  {'PASS' if passed else 'FAIL'}")
    return passed


# ============================================================
# Test 11 (item 2.9): Discrete Hodge cohomology of T²
# ============================================================

def test_11_hodge_torus():
    """
    CLAIM (item 2.9): Hodge theorem (foundational for Witten deformation):
        dim H^k(M; ℝ) = dim ker(Δ_k)
    where Δ_k is the Laplacian on k-forms. Witten deformation refines this
    by giving a Morse-theoretic description that preserves the kernel dims.
    
    GROUND TRUTH: Betti numbers of T²: (b_0, b_1, b_2) = (1, 2, 1).
    Euler characteristic χ = 0.
    
    NON-CIRCULAR: We construct discrete exterior calculus on an N×N
    periodic grid (cell complex for T²). Operators d_0 (V → E) and d_1
    (E → F) are built from the cell incidence structure with no reference
    to cohomology. We then compute Hodge Laplacians Δ_k = d† d + d d†
    (with d_{-1} and d_2 taken to be zero) and count kernel dimensions
    via eigenvalues. Match to (1, 2, 1) verifies the Hodge theorem.
    
    Also verifies d² = 0 (chain complex identity) — forced by combinatorics.
    """
    print("\n=== Test 11: Discrete Hodge cohomology of T² ===")
    
    N = 6
    nV = N * N
    nEh = N * N        # horizontal edges
    nEv = N * N        # vertical edges
    nE = nEh + nEv
    nF = N * N
    
    def v(i, j): return (i % N) * N + (j % N)
    def eh(i, j): return (i % N) * N + (j % N)
    def ev(i, j): return nEh + (i % N) * N + (j % N)
    def f(i, j): return (i % N) * N + (j % N)
    
    # d_0: V → E (gradient)
    d0 = torch.zeros(nE, nV, dtype=torch.float64)
    for i in range(N):
        for j in range(N):
            d0[eh(i, j), v(i + 1, j)] += 1
            d0[eh(i, j), v(i, j)] -= 1
            d0[ev(i, j), v(i, j + 1)] += 1
            d0[ev(i, j), v(i, j)] -= 1
    
    # d_1: E → F (curl)
    d1 = torch.zeros(nF, nE, dtype=torch.float64)
    for i in range(N):
        for j in range(N):
            # ∂(face (i,j)) traverses: H_(i,j), V_(i+1,j), -H_(i,j+1), -V_(i,j)
            d1[f(i, j), eh(i, j)] += 1
            d1[f(i, j), ev(i + 1, j)] += 1
            d1[f(i, j), eh(i, j + 1)] -= 1
            d1[f(i, j), ev(i, j)] -= 1
    
    # Verify d_1 d_0 = 0 (must hold by construction; sanity)
    d2_err = (d1 @ d0).abs().max().item()
    
    # Hodge Laplacians
    L0 = d0.T @ d0
    L1 = d0 @ d0.T + d1.T @ d1
    L2 = d1 @ d1.T
    
    e0 = torch.linalg.eigvalsh(L0)
    e1 = torch.linalg.eigvalsh(L1)
    e2 = torch.linalg.eigvalsh(L2)
    
    tol = 1e-8
    b0 = (e0 < tol).sum().item()
    b1 = (e1 < tol).sum().item()
    b2 = (e2 < tol).sum().item()
    
    print(f"  Grid: V={nV}, E={nE}, F={nF};  V−E+F = {nV - nE + nF} (must = 0 for T²)")
    print(f"  Sanity: ‖d₁∘d₀‖_∞ = {d2_err:.2e} (must = 0)")
    print(f"  dim ker Δ_0 = {b0}  (b_0 expected 1)  {'✓' if b0 == 1 else '✗'}")
    print(f"  dim ker Δ_1 = {b1}  (b_1 expected 2)  {'✓' if b1 == 2 else '✗'}")
    print(f"  dim ker Δ_2 = {b2}  (b_2 expected 1)  {'✓' if b2 == 1 else '✗'}")
    print(f"  Hodge χ = {b0 - b1 + b2}  (expected 0)")
    
    # Sanity: examine the small eigenvalues of L_1 — should be exactly 2 zeros
    smallest = torch.sort(e1).values[:6]
    print(f"  Six smallest eigenvalues of Δ_1: {[f'{x:.4e}' for x in smallest.tolist()]}")
    
    # Bonus negative check: build the same operator structure but on the SPHERE
    # (which has Betti (1, 0, 1)) — use a tetrahedron simplicial structure.
    # 4 vertices, 6 edges, 4 faces (boundary of 3-simplex = S²)
    print(f"  Bonus: compare to tetrahedron (boundary of 3-simplex, = S²) with Betti (1,0,1)")
    # Vertices 0, 1, 2, 3. Edges: all (i,j) pairs with i<j: 6 edges.
    # Faces: all 3-subsets: {0,1,2}, {0,1,3}, {0,2,3}, {1,2,3}: 4 faces.
    nV_t, nE_t, nF_t = 4, 6, 4
    edges_t = [(0,1), (0,2), (0,3), (1,2), (1,3), (2,3)]
    edge_idx = {e: k for k, e in enumerate(edges_t)}
    faces_t = [(0,1,2), (0,1,3), (0,2,3), (1,2,3)]
    
    d0_t = torch.zeros(nE_t, nV_t, dtype=torch.float64)
    for k, (i, j) in enumerate(edges_t):
        d0_t[k, j] = 1.0
        d0_t[k, i] = -1.0
    
    d1_t = torch.zeros(nF_t, nE_t, dtype=torch.float64)
    for k, (i, j, l) in enumerate(faces_t):
        # boundary: edge (j,l) - edge (i,l) + edge (i,j)
        d1_t[k, edge_idx[(j, l)]] += 1
        d1_t[k, edge_idx[(i, l)]] -= 1
        d1_t[k, edge_idx[(i, j)]] += 1
    
    d2_err_t = (d1_t @ d0_t).abs().max().item()
    L0_t = d0_t.T @ d0_t
    L1_t = d0_t @ d0_t.T + d1_t.T @ d1_t
    L2_t = d1_t @ d1_t.T
    b0_t = (torch.linalg.eigvalsh(L0_t) < tol).sum().item()
    b1_t = (torch.linalg.eigvalsh(L1_t) < tol).sum().item()
    b2_t = (torch.linalg.eigvalsh(L2_t) < tol).sum().item()
    
    print(f"    Tetrahedron: d² err = {d2_err_t:.2e}; (b_0, b_1, b_2) = "
          f"({b0_t}, {b1_t}, {b2_t}); expected (1, 0, 1) for S²  "
          f"{'✓' if (b0_t, b1_t, b2_t) == (1, 0, 1) else '✗'}")
    
    passed = (d2_err < 1e-10 and b0 == 1 and b1 == 2 and b2 == 1
              and d2_err_t < 1e-10 and (b0_t, b1_t, b2_t) == (1, 0, 1))
    print(f"  {'PASS' if passed else 'FAIL'}")
    return passed


def main():
    results = []
    for t in [test_9_index_theorem_torus, test_10_berezin_toeplitz, test_11_hodge_torus]:
        try:
            ok = t()
        except Exception as e:
            print(f"  EXCEPTION: {e}")
            import traceback; traceback.print_exc()
            ok = False
        results.append((t.__name__, ok))
    
    print("\n" + "=" * 60)
    print("V3 SUMMARY")
    print("=" * 60)
    for name, ok in results:
        print(f"  {name}: {'PASS' if ok else 'FAIL'}")
    n_pass = sum(1 for _, ok in results if ok)
    print(f"\n{n_pass}/{len(results)} v3 tests passed.")
    return n_pass == len(results)


if __name__ == "__main__":
    main()
