#!/bin/bash
cd /Users/bdavis/.claude/jobs/c8f0f39d/tmp/var
export GIGI_RICCI_DATA_ROOT=/Users/bdavis/.claude/jobs/c8f0f39d/tmp/su2 OMP_NUM_THREADS=6
P=/Users/bdavis/Projects/tessera/venv/bin/python
D=research/variational-v1b-20261119; mkdir -p $D
$P scripts/ricci_variational_v1.py 0 $D/L0.json --objective stiffness 2>&1 | tail -1 | cut -c1-600
$P scripts/ricci_variational_v1.py 1 $D/L1.json --objective stiffness 2>&1 | tail -1 | cut -c1-600
