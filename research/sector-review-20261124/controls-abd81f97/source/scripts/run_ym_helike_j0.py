"""Scalar-only entry point sharing the guarded evidence and resource workflow.

The registered first pass ends at N=24. Its allowance is scalar-sector-only;
it cannot assert a global excitation gap. Prefer the joint runner for spin-two comparison.
"""
from run_ym_helike_sectors import main

if __name__=='__main__':
    raise SystemExit(main(scalar_only=True))
