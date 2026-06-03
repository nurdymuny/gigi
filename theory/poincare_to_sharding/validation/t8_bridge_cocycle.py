"""
T8: Bridge transition cocycle bound across two atlases on S^2.

================================================================================
CLAIM (CROSS_ATLAS_JOINS.md §3):
    For two atlases A_1, A_2 of the same shared semantic structure
    S_shared, the bridge transitions B_ij^{12}: V_i^{A_1} -> V_j^{A_2}
    satisfy a CROSS-ATLAS COCYCLE CONDITION analogous to the intra-
    atlas cocycle bound (T2 §3.2):

      Forward triple (i, j in A_1; k in A_2):
        || B_jk^{12}(T_ij^{A_1}(v_i)) - B_ik^{12}(v_i) || <= delta_bridge

      Forward triple (i in A_1; j, k in A_2):
        || T_jk^{A_2}(B_ij^{12}(v_i)) - B_ik^{12}(v_i) || <= delta_bridge

      Round-trip:
        || B_ji^{21}(B_ij^{12}(v_i)) - v_i || <= delta_asymm

    For analytic atlases (where bridges are derived from the shared
    geometry), delta_bridge = delta_asymm = 0 to machine precision.
    For learned bridges with perturbation epsilon, both grow first-
    order in epsilon.

REFERENCES:
    - Davis, *The Geometry of Sameness* §4.2 (cocycle bound Def 21,
      extended here to cross-atlas).
    - Davis, *The Davis Manifold* §A5 (non-vacuity, applied per-atlas).
    - CROSS_ATLAS_JOINS.md §3 (cross-atlas cocycle conditions).

GROUND TRUTH (independent):
    The underlying sphere S^2. Both atlases are stereographic
    projections from independent points on S^2. Bridges are derived
    from the closed-form composition sigma_to o sigma_from^{-1}, NEVER
    by curve-fitting to make the cocycle hold.

TEST DESIGN:
    Two atlases of S^2:
      Atlas A_1 (north-south stereographic atlas):
        Chart N: stereographic projection from north pole (0, 0, 1)
        Chart S: stereographic projection from south pole (0, 0, -1)
      Atlas A_2 (east-west stereographic atlas):
        Chart E: stereographic projection from east point (1, 0, 0)
        Chart W: stereographic projection from west point (-1, 0, 0)

    Sample points in the QUADRUPLE overlap (S^2 minus four pole
    neighborhoods of radius safety_r). For each point p:

      Forward (A_1 internal):
        v_N = sigma_N(p);  v_S = T_{N->S}^{A_1}(v_N) [via known formula]
      Bridge into A_2:
        v_E_via_N = B_{N->E}^{12}(v_N) = sigma_E(sigma_N^{-1}(v_N))
        v_E_via_S = B_{S->E}^{12}(v_S) = sigma_E(sigma_S^{-1}(v_S))
      Cross-atlas cocycle check (forward MMP triple):
        |v_E_via_N - v_E_via_S| should be ~0.

      A_2 internal:
        v_W_via_E = T_{E->W}^{A_2}(v_E_via_N)
      Cross-atlas cocycle check (forward MPP triple):
        |v_W_via_E - B_{N->W}^{12}(v_N)| should be ~0.

      Round-trip:
        v_N_back = B_{E->N}^{21}(v_E_via_N) = sigma_N(sigma_E^{-1}(v_E_via_N))
        |v_N_back - v_N| should be ~0.

    Part (a) analytic: all three discrepancies < 1e-12.
    Part (b) perturbed: independent Gaussian noise of magnitude
        epsilon on each transition + bridge; verify discrepancy
        slope ~1.0 in log-log of (eps, max_discrepancy).

PASS CRITERION:
    (a) max discrepancy across 200 samples < 1e-12 for all three
        cocycle conditions.
    (b) log-log regression slope of max_disc vs epsilon in
        [0.9, 1.1] for all three conditions over eps in
        {1e-4, 1e-3, 1e-2, 1e-1}.

CIRCULAR-LOGIC GUARDS:
    1. All four stereographic projections (sigma_N, sigma_S,
       sigma_E, sigma_W) are derived INDEPENDENTLY from closed-form
       formulas, never from each other. The cross-atlas cocycle is
       VERIFIED, not engineered.
    2. Perturbations on each transition/bridge are sampled
       INDEPENDENTLY from N(0, eps^2). They are not correlated to
       make the cocycle hold artificially.
    3. Sample points are drawn uniformly on S^2 (rejection-sampled
       to avoid all four poles by safety radius). Drawing strategy
       does not encode the cocycle.
    4. The "bridge" is constructed as composition of two independent
       stereographic maps: sigma_to o sigma_from^{-1}. The bridge's
       correctness is INHERITED from the correctness of each sigma,
       not separately engineered.
================================================================================
"""

from __future__ import annotations
import math
import sys
import numpy as np


# ============================================================================
# Stereographic projections from four poles (each independent closed form)
# ============================================================================


def sigma_N(p: np.ndarray) -> np.ndarray:
    """Stereographic from north pole (0, 0, 1)."""
    x, y, z = p
    if 1.0 - z < 1e-12:
        raise ValueError("at north pole")
    return np.array([x / (1.0 - z), y / (1.0 - z)])


def sigma_N_inv(uv: np.ndarray) -> np.ndarray:
    u, v = uv
    s = u * u + v * v + 1.0
    return np.array([2.0 * u / s, 2.0 * v / s, (u * u + v * v - 1.0) / s])


def sigma_S(p: np.ndarray) -> np.ndarray:
    """Stereographic from south pole (0, 0, -1)."""
    x, y, z = p
    if 1.0 + z < 1e-12:
        raise ValueError("at south pole")
    return np.array([x / (1.0 + z), y / (1.0 + z)])


def sigma_S_inv(uv: np.ndarray) -> np.ndarray:
    u, v = uv
    s = u * u + v * v + 1.0
    return np.array([2.0 * u / s, 2.0 * v / s, (1.0 - u * u - v * v) / s])


def sigma_E(p: np.ndarray) -> np.ndarray:
    """Stereographic from east point (1, 0, 0)."""
    x, y, z = p
    if 1.0 - x < 1e-12:
        raise ValueError("at east point")
    return np.array([y / (1.0 - x), z / (1.0 - x)])


def sigma_E_inv(uv: np.ndarray) -> np.ndarray:
    u, v = uv
    s = u * u + v * v + 1.0
    return np.array([(u * u + v * v - 1.0) / s, 2.0 * u / s, 2.0 * v / s])


def sigma_W(p: np.ndarray) -> np.ndarray:
    """Stereographic from west point (-1, 0, 0)."""
    x, y, z = p
    if 1.0 + x < 1e-12:
        raise ValueError("at west point")
    return np.array([y / (1.0 + x), z / (1.0 + x)])


def sigma_W_inv(uv: np.ndarray) -> np.ndarray:
    u, v = uv
    s = u * u + v * v + 1.0
    return np.array([(1.0 - u * u - v * v) / s, 2.0 * u / s, 2.0 * v / s])


# ============================================================================
# Intra-atlas transitions (within A_1 or within A_2)
# ============================================================================


def T_NS_A1(uv: np.ndarray) -> np.ndarray:
    """T: chart N -> chart S, within atlas A_1."""
    return sigma_S(sigma_N_inv(uv))


def T_EW_A2(uv: np.ndarray) -> np.ndarray:
    """T: chart E -> chart W, within atlas A_2."""
    return sigma_W(sigma_E_inv(uv))


# ============================================================================
# Bridge transitions (A_1 <-> A_2)
# ============================================================================


def B_N_to_E(uv: np.ndarray) -> np.ndarray:
    """Bridge from chart N of A_1 to chart E of A_2."""
    return sigma_E(sigma_N_inv(uv))


def B_N_to_W(uv: np.ndarray) -> np.ndarray:
    """Bridge from chart N of A_1 to chart W of A_2."""
    return sigma_W(sigma_N_inv(uv))


def B_S_to_E(uv: np.ndarray) -> np.ndarray:
    """Bridge from chart S of A_1 to chart E of A_2."""
    return sigma_E(sigma_S_inv(uv))


def B_E_to_N(uv: np.ndarray) -> np.ndarray:
    """Bridge from chart E of A_2 to chart N of A_1 (for round-trip)."""
    return sigma_N(sigma_E_inv(uv))


# ============================================================================
# Sample points in the quadruple overlap
# ============================================================================


def sample_quad_overlap(n: int, safety_radius: float = 0.3, seed: int = 42) -> list[np.ndarray]:
    """Sample n points on S^2 outside the four pole neighborhoods."""
    rng = np.random.RandomState(seed)
    poles = [
        np.array([0.0, 0.0, 1.0]),   # N
        np.array([0.0, 0.0, -1.0]),  # S
        np.array([1.0, 0.0, 0.0]),   # E
        np.array([-1.0, 0.0, 0.0]),  # W
    ]
    out = []
    attempts = 0
    while len(out) < n and attempts < 100 * n:
        v = rng.normal(size=3)
        v = v / np.linalg.norm(v)
        if all(np.linalg.norm(v - pole) > safety_radius for pole in poles):
            out.append(v)
        attempts += 1
    if len(out) < n:
        raise RuntimeError(f"could not sample {n} quad-overlap points")
    return out


# ============================================================================
# Cocycle discrepancies (with optional perturbations)
# ============================================================================


def cocycle_MMP_discrepancy(p: np.ndarray, perturb=None) -> float:
    """
    Forward MMP triple: i, j in A_1 (N, S); k in A_2 (E).
    Verify: B_{S->E}(T_{N->S}(v_N)) ~ B_{N->E}(v_N).
    """
    v_N = sigma_N(p)

    Tns = T_NS_A1(v_N)
    if perturb is not None:
        Tns = Tns + perturb['T_NS']
    Bse = B_S_to_E(Tns)
    if perturb is not None:
        Bse = Bse + perturb['B_S_E']

    Bne = B_N_to_E(v_N)
    if perturb is not None:
        Bne = Bne + perturb['B_N_E']

    return float(np.linalg.norm(Bse - Bne))


def cocycle_MPP_discrepancy(p: np.ndarray, perturb=None) -> float:
    """
    Forward MPP triple: i in A_1 (N); j, k in A_2 (E, W).
    Verify: T_{E->W}(B_{N->E}(v_N)) ~ B_{N->W}(v_N).
    """
    v_N = sigma_N(p)

    Bne = B_N_to_E(v_N)
    if perturb is not None:
        Bne = Bne + perturb['B_N_E']
    Tew = T_EW_A2(Bne)
    if perturb is not None:
        Tew = Tew + perturb['T_EW']

    Bnw = B_N_to_W(v_N)
    if perturb is not None:
        Bnw = Bnw + perturb['B_N_W']

    return float(np.linalg.norm(Tew - Bnw))


def round_trip_discrepancy(p: np.ndarray, perturb=None) -> float:
    """
    Round-trip: || B_{E->N}(B_{N->E}(v_N)) - v_N ||.
    """
    v_N = sigma_N(p)

    Bne = B_N_to_E(v_N)
    if perturb is not None:
        Bne = Bne + perturb['B_N_E']
    Ben = B_E_to_N(Bne)
    if perturb is not None:
        Ben = Ben + perturb['B_E_N']

    return float(np.linalg.norm(Ben - v_N))


# ============================================================================
# Test runner
# ============================================================================


def run_part_a():
    """Part (a): analytic atlases -> all cocycle discrepancies ~ 0."""
    print("\n-- Part (a): ANALYTIC atlases (delta_bridge ~ 0) " + "-" * 18)
    samples = sample_quad_overlap(200, safety_radius=0.3, seed=42)
    mmp = [cocycle_MMP_discrepancy(p) for p in samples]
    mpp = [cocycle_MPP_discrepancy(p) for p in samples]
    rt = [round_trip_discrepancy(p) for p in samples]
    print(f"  samples: {len(samples)}")
    print(f"  max MMP-triple discrepancy : {max(mmp):.3e}")
    print(f"  max MPP-triple discrepancy : {max(mpp):.3e}")
    print(f"  max round-trip discrepancy : {max(rt):.3e}")
    threshold = 1e-12
    ok = max(max(mmp), max(mpp), max(rt)) < threshold
    print(f"  threshold                  : {threshold:.0e}")
    print(f"  PASS: {ok}")
    return ok


def run_part_b():
    """Part (b): perturbed -> first-order discrepancy in epsilon."""
    print("\n-- Part (b): PERTURBED atlases (first-order in eps) " + "-" * 15)
    samples = sample_quad_overlap(200, safety_radius=0.3, seed=42)
    epsilons = [1e-4, 1e-3, 1e-2, 1e-1]

    keys = ['T_NS', 'T_EW', 'B_N_E', 'B_N_W', 'B_S_E', 'B_E_N']
    n_perturbations = len(keys) * len(samples)
    # Extreme value scaling: max-of-N 2D Gaussians ~ sqrt(2 ln N) per component
    ev_factor = math.sqrt(2.0 * math.log(n_perturbations)) * math.sqrt(2.0) * 1.1

    max_mmp = []
    max_mpp = []
    max_rt = []
    for eps in epsilons:
        rng = np.random.RandomState(int(eps * 1e7) + 7)
        mmp_local = []
        mpp_local = []
        rt_local = []
        for p in samples:
            perturb = {k: rng.normal(scale=eps, size=2) for k in keys}
            mmp_local.append(cocycle_MMP_discrepancy(p, perturb=perturb))
            mpp_local.append(cocycle_MPP_discrepancy(p, perturb=perturb))
            rt_local.append(round_trip_discrepancy(p, perturb=perturb))
        max_mmp.append(max(mmp_local))
        max_mpp.append(max(mpp_local))
        max_rt.append(max(rt_local))
        # Reasonable structural caps (conservative)
        cap_mmp = 5.0 * ev_factor * eps
        cap_mpp = 5.0 * ev_factor * eps
        cap_rt = 5.0 * ev_factor * eps
        print(f"  eps={eps:.0e}: MMP={max_mmp[-1]:.3e} (cap {cap_mmp:.3e}: {max_mmp[-1] <= cap_mmp})  "
              f"MPP={max_mpp[-1]:.3e} (cap {cap_mpp:.3e}: {max_mpp[-1] <= cap_mpp})  "
              f"RT={max_rt[-1]:.3e} (cap {cap_rt:.3e}: {max_rt[-1] <= cap_rt})")

    # Log-log slopes
    log_eps = np.log(np.array(epsilons))
    slope_mmp = float(np.polyfit(log_eps, np.log(np.array(max_mmp)), 1)[0])
    slope_mpp = float(np.polyfit(log_eps, np.log(np.array(max_mpp)), 1)[0])
    slope_rt = float(np.polyfit(log_eps, np.log(np.array(max_rt)), 1)[0])
    print(f"  slopes: MMP={slope_mmp:.3f}, MPP={slope_mpp:.3f}, RT={slope_rt:.3f}  (first-order ~ 1.0)")

    slopes_ok = all(0.9 <= s <= 1.1 for s in [slope_mmp, slope_mpp, slope_rt])
    print(f"  PASS: all slopes in [0.9, 1.1]: {slopes_ok}")
    return slopes_ok


def main():
    print("=" * 72)
    print("T8: Cross-atlas bridge cocycle bound on two S^2 atlases")
    print("=" * 72)
    print("  A_1: stereographic atlas from N, S poles")
    print("  A_2: stereographic atlas from E, W poles")
    print("  Bridges B^{12} from A_1 charts to A_2 charts via shared S^2.")
    print("  Validates three cocycle conditions per CROSS_ATLAS_JOINS.md §3.")

    ok_a = run_part_a()
    ok_b = run_part_b()

    print("\n" + "=" * 72)
    print("SUMMARY")
    print("=" * 72)
    print(f"  [{('PASS' if ok_a else 'FAIL')}] Part (a) analytic: all cocycle conditions exact")
    print(f"  [{('PASS' if ok_b else 'FAIL')}] Part (b) perturbed: all slopes first-order")

    if ok_a and ok_b:
        print("\n  T8 GREEN -- cross-atlas bridge cocycle validated:")
        print("    Analytic atlases satisfy MMP, MPP, round-trip cocycles EXACTLY.")
        print("    Perturbed atlases degrade FIRST-ORDER in bridge slack.")
        print("  Cross-atlas join math is unblocked (alongside T9 and T10).")
        return 0
    else:
        print("\n  T8 RED -- cross-atlas cocycle claims fail.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
