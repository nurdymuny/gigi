"""Q1 input (docs/HALCYON_QUANTUM_GAP_SPEC.md): regenerate Halcyon's M1 ensemble with Halcyon's own heat bath
(davis-wilson-lattice/inertia_damping/su2_4d_heatbath.run_heatbath, CPU path), at the parameters its receipt
ym4_glueball_in_gigi_receipt.json was made with: L = 16, beta = 2.3, n_thermalize = 200, n_measure = 100,
measure_every = 2, seed = 20260628. Saves the per-configuration time-slice plaquette densities p_bar (n_cfg x L)
and Halcyon's own M1 extraction of them, for the engine's parity check.

The original run may have used the GPU path, so the series need not be bit-identical; the parity gate is
"within errors" (spec Q1-a).

usage: python scripts/halcyon_m1_regenerate.py HALCYON_ROOT OUT_DIR [L] [BETA] [SEED]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

root, out = Path(sys.argv[1]), Path(sys.argv[2])
L = int(sys.argv[3]) if len(sys.argv) > 3 else 16
beta = float(sys.argv[4]) if len(sys.argv) > 4 else 2.3
seed = int(sys.argv[5]) if len(sys.argv) > 5 else 20260628
sys.path.insert(0, str(root))
from inertia_damping.su2_4d_heatbath import run_heatbath                     # noqa: E402
from inertia_damping.su2_4d_glueball import (extract_correlator_and_mass,    # noqa: E402
                                              shuffled_null_control)

out.mkdir(parents=True, exist_ok=True)
stem = out / f"m1_su2_L{L}_beta{str(beta).replace('.', 'p')}_seed{seed}"
if stem.with_suffix(".npy").exists():
    raise SystemExit(f"refusing to overwrite {stem}.npy")
t0 = time.time()
rep = run_heatbath(L=L, beta=beta, n_thermalize=200, n_measure=100, measure_every=2, seed=seed,
                   keep_t_slices=True, wilson_max=0, verbose=True)
p_bar = np.array(rep.plaquette_per_config_t_slices, dtype=np.float64)
np.save(stem.with_suffix(".npy"), p_bar)
r = extract_correlator_and_mass(p_bar, L=L, beta=beta)
rn = extract_correlator_and_mass(shuffled_null_control(p_bar, seed=seed + 1), L=L, beta=beta)
tolist = lambda a: [None if not np.isfinite(x) else float(x) for x in np.asarray(a, float)]   # noqa: E731
receipt = {"kind": "Q1 input: Halcyon M1 ensemble regenerated with Halcyon's CPU heat bath",
           "L": L, "beta": beta, "n_thermalize": 200, "n_measure": 100, "measure_every": 2, "seed": seed,
           "p_bar_file": stem.with_suffix(".npy").name, "p_bar_shape": list(p_bar.shape),
           "halcyon_extraction": {"C_t": tolist(r.C_PP_connected_t), "C_err_t": tolist(r.C_PP_error_t),
                                  "m_eff_t": tolist(r.m_eff_t), "m_eff_err_t": tolist(r.m_eff_error_t),
                                  "window": list(r.plateau_fit_window) if r.plateau_fit_window else None,
                                  "plateau_mass": r.plateau_fit_mass, "plateau_error": r.plateau_fit_error},
           "halcyon_null_extraction": {"plateau_mass": rn.plateau_fit_mass, "plateau_error": rn.plateau_fit_error,
                                       "window": list(rn.plateau_fit_window) if rn.plateau_fit_window else None},
           "original_receipt_plateau": [1.9490807280083904, 0.014906651141309539],
           "seconds": round(time.time() - t0, 1)}
stem.with_suffix(".json").write_text(json.dumps(receipt, indent=1) + "\n")
print(json.dumps({k: receipt[k] for k in ("halcyon_extraction", "halcyon_null_extraction", "seconds")})[:1500])
