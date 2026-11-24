# Quasimode construction gates

The dated construction registration precedes these tests. Logs are retained in run order.
Numerical quadrature is not an interval certificate. The full grid remains held.

`red.log` records nine failing behavioral fixtures before implementation; `red-operator.log` records the
missing full-operator behavior. `green-full.log` and `mutations.json` retain the focused construction gates.
The audit has no interval-certification success path. Its output always preserves UNRESOLVED_INTEGRATION.

```powershell
python -m unittest discover -s scripts -p test_helike_quasimode.py -v
python scripts/check_helike_quasimode_mutations.py "$env:TEMP/helike-quasimode-mutants.json"
python -u scripts/run_helike_quasimode.py NEW_EMPTY_CONTROL_FOLDER
```

The last command runs only s=1.15, t=3, g^2=0.004,0.002 and Mq=6,8. It refuses dirty source files,
saves the state coefficients and chart, checkpoints each quadrature stage, and never calls the full grid.
The frozen 10^6 and 12^6 Gaussian nodes are numerical diagnostics; sampled zero leakage is not a tail bound.
