"""Which zonal wavenumber does Saturn's polar jet actually favour?

Barotropic linear stability of a zonal jet on a beta-plane, with a
deformation radius. For each zonal wavenumber m the meridional structure
equation is

    (U - c) [ psi'' - (k^2 + L_D^-2) psi ] + (beta - U'') psi = 0

with k = m / r(phi0) the zonal wavenumber per metre at the jet latitude,
psi = 0 at the domain edges. Discretised on a latitude grid this is a
generalised eigenproblem A psi = c L psi; growth rate is k * Im(c). The
fastest-growing m is the prediction. Rayleigh-Kuo (beta - U'' changes sign)
is reported as the necessary condition.

This is the outside review's question 12, and the first analysis in this
directory that could in principle predict "ten south, six north" without
inserting ten disturbances by hand. It is also only a barotropic beta-plane
model of a stratified, spherical, vertically sheared atmosphere, so the
number it returns is a first-order statement about jet shape and latitude,
not a full theory.

INPUT PROFILE. Two sources, and the output says which was used:
  measured    u(phi) from wind_profile.py, if its validation gate passed.
  parametric  Gaussian jet built from the discovery paper's published 2025
              numbers: 116 m/s peak at 60.5 S, FWHM 2.8 deg. The north uses
              a literature-order hexagon jet, 120 m/s near 77.5 N, with the
              SAME width as a controlled comparison - the northern width is
              an assumption and is swept.

Sensitivity: peak +-10%, FWHM +-20%, latitude +-0.5 deg, 200 draws; the
distribution of the fastest m is reported per L_D.
"""
import json
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
A_EQ, B_POL = 60268.0, 54364.0
E2 = 1.0 - (B_POL / A_EQ) ** 2
OMEGA = 2 * np.pi / (10.656 * 3600.0)          # rad/s, System III
LD_KM = [1e9, 4000, 3000, 2000, 1500, 1000, 700, 500]
MS = np.arange(2, 21)
RNG = np.random.default_rng(20260903)


def axis_radius_m(phi):
    p = np.radians(abs(phi))
    return 1000 * A_EQ * np.cos(p) / np.sqrt(1 - E2 * np.sin(p) ** 2)


def merid_radius_m(phi):
    p = np.radians(abs(phi))
    return 1000 * A_EQ * (1 - E2) / (1 - E2 * np.sin(p) ** 2) ** 1.5


def gaussian_jet(phi, phi0, u0, fwhm):
    sig = fwhm / (2 * np.sqrt(np.log(2)))
    return u0 * np.exp(-((phi - phi0) / sig) ** 2)


def stability(phi, U, phi0, ld_km):
    """Return dict m -> max growth rate (1/day), plus Rayleigh-Kuo flag."""
    M = merid_radius_m(phi0)
    r = axis_radius_m(phi0)
    y = np.radians(phi) * M                     # metres, increasing north
    dy = y[1] - y[0]
    n = len(y)
    beta = 2 * OMEGA * np.cos(np.radians(abs(phi0))) / M
    Upp = np.gradient(np.gradient(U, dy), dy)
    q = beta - Upp                              # PV gradient
    rk = bool(np.any(q > 0) and np.any(q < 0))
    # second-difference operator with Dirichlet ends
    D2 = (np.diag(np.ones(n - 1), 1) - 2 * np.eye(n) + np.diag(np.ones(n - 1), -1)) / dy ** 2
    out = {}
    for m in MS:
        k = m / r
        K2 = k * k + (1.0 / (ld_km * 1000.0)) ** 2
        L = D2 - K2 * np.eye(n)
        # (U - c)(psi'' - K^2 psi) + (beta - U'') psi = 0
        #   =>  [diag(U) L + diag(q)] psi = c L psi
        # First version had a MINUS here (and in the docstring). That flips the
        # PV-gradient term and makes every jet nearly neutral: growth rates of
        # ~0.002/day, e-folding in years, for a 116 m/s jet 3000 km wide.
        Amat = np.diag(U) @ L + np.diag(q)
        try:
            c = np.linalg.eigvals(np.linalg.solve(L, Amat))
        except np.linalg.LinAlgError:
            out[int(m)] = 0.0
            continue
        sigma = k * np.max(c.imag)              # 1/s
        out[int(m)] = float(max(sigma, 0.0) * 86400.0)   # 1/day
    return out, rk, beta


def fastest(growth):
    m, g = max(growth.items(), key=lambda kv: kv[1])
    return (m, g) if g > 0 else (None, 0.0)


def run_pole(label, phi0, u0, fwhm, measured=None):
    print(f"\n{'=' * 78}\n{label}\n{'=' * 78}")
    if measured is not None:
        phi = np.array([p["lat"] for p in measured])
        U = np.array([p["u"] for p in measured])
        src = "MEASURED (wind_profile.py)"
    else:
        phi = np.arange(phi0 - 9.0, phi0 + 9.0 + 1e-9, 0.1)
        U = gaussian_jet(phi, phi0, u0, fwhm)
        src = f"PARAMETRIC Gaussian: {u0:.0f} m/s at {phi0:+.1f}, FWHM {fwhm:.1f} deg"
    print(f"  profile source: {src}")
    print(f"  r(phi0) = {axis_radius_m(phi0)/1e3:,.0f} km   M(phi0) = {merid_radius_m(phi0)/1e3:,.0f} km")

    print(f"\n  growth rate (1/day) by m, per deformation radius; * = fastest")
    hdr = f"  {'L_D km':>7} {'RK':>3} " + " ".join(f"m{m:<4}" for m in MS) + "   fastest"
    print(hdr)
    table = {}
    for ld in LD_KM:
        g, rk, beta = stability(phi, U, phi0, ld)
        fm, fg = fastest(g)
        table[ld] = (g, fm, fg)
        cells = " ".join((f"{g[m]:5.2f}" if g[m] > 0 else "  .  ") for m in MS)
        star = f"  m={fm} ({fg:.2f}/d, e-fold {1/fg:.1f} d)" if fm else "  stable"
        print(f"  {('inf' if ld >= 1e8 else f'{ld:.0f}'):>7} {'Y' if rk else 'n':>3} {cells}{star}")
    print(f"  beta at phi0 = {beta:.3e} /m/s")

    # sensitivity
    if measured is None:
        # 200 draws x 8 L_D x 19 m x 2 poles with multithreaded BLAS thrashing on
        # 180x180 matrices ran at ~6.5 cores for 40 min and was ~15% done. Run
        # single-threaded (OMP/OPENBLAS/MKL_NUM_THREADS=1) with 60 draws.
        NDRAW = 60
        print(f"\n  sensitivity: peak +-10%, FWHM +-20%, lat +-0.5 ({NDRAW} draws) -> distribution of fastest m")
        print(f"  {'L_D km':>7}  " + " ".join(f"m{m:<3}" for m in range(4, 15)) + "   mode")
        for ld in LD_KM:
            counts = {m: 0 for m in MS}
            for _ in range(NDRAW):
                u0s = u0 * RNG.uniform(0.9, 1.1)
                fw = fwhm * RNG.uniform(0.8, 1.2)
                p0 = phi0 + RNG.uniform(-0.5, 0.5)
                ph = np.arange(p0 - 9.0, p0 + 9.0 + 1e-9, 0.1)
                g, _, _ = stability(ph, gaussian_jet(ph, p0, u0s, fw), p0, ld)
                fm, _ = fastest(g)
                if fm:
                    counts[fm] += 1
            tot = max(1, sum(counts.values()))
            row = " ".join(f"{100*counts[m]/tot:3.0f}%" if counts[m] else "  . " for m in range(4, 15))
            mode = max(counts.items(), key=lambda kv: kv[1])[0]
            print(f"  {('inf' if ld >= 1e8 else f'{ld:.0f}'):>7}  {row}   m={mode}")
    return table


if __name__ == "__main__":
    measured_s = measured_n = None
    wr = os.path.join(HERE, "wind_results.json")
    use_measured = "--measured" in sys.argv
    if use_measured and os.path.exists(wr):
        res = json.load(open(wr))
        r25 = next((r for r in res if r["year"] == "2025" and r["filter"] == "f631n"), None)
        if r25 and r25.get("south_jet") and abs(r25["south_jet"]["peak_u"] - 116) < 30:
            measured_s = r25["south"]
        else:
            print("measured southern profile did not pass validation; using parametric")

    south = run_pole("SOUTH - the decagon jet", -60.5, 116.0, 2.8, measured_s)
    north = run_pole("NORTH - the hexagon jet (width ASSUMED equal to south)", 77.5, 120.0, 2.8, measured_n)

    print(f"\n{'=' * 78}\nNORTH width sweep (the one number we do not have)\n{'=' * 78}")
    print(f"  {'FWHM':>5}  " + "  ".join(f"L_D={('inf' if ld >= 1e8 else int(ld))}" for ld in LD_KM[:6]))
    for fw in (2.0, 2.4, 2.8, 3.2, 3.6, 4.0):
        ph = np.arange(77.5 - 9.0, 77.5 + 9.0 + 1e-9, 0.1)
        cells = []
        for ld in LD_KM[:6]:
            g, _, _ = stability(ph, gaussian_jet(ph, 77.5, 120.0, fw), 77.5, ld)
            fm, fg = fastest(g)
            cells.append(f"m={fm if fm else '-':<2} ({fg:4.2f})")
        print(f"  {fw:5.1f}  " + "  ".join(cells))

    print("\nREAD")
    print("  The question is whether one physical parameterisation (same L_D, same jet")
    print("  shape) gives m=10 at 60.5 S AND m=6 at 77.5 N. Circumference alone would")
    print(f"  scale m by r_south/r_north = {axis_radius_m(-60.5)/axis_radius_m(77.5):.2f}; beta at 77.5 N is")
    print(f"  {np.cos(np.radians(77.5))/np.cos(np.radians(60.5)):.2f}x the southern value, which weakens the stabilisation.")
