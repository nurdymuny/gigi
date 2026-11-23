#!/bin/bash
cd /Users/bdavis/.claude/jobs/c8f0f39d/tmp/var
export GIGI_RICCI_DATA_ROOT=/Users/bdavis/.claude/jobs/c8f0f39d/tmp/ricci OMP_NUM_THREADS=4
P=/Users/bdavis/Projects/tessera/venv/bin/python
for lv in 0 1 2; do
  $P scripts/ricci_magnet_m0.py $lv research/magnet-m0-20261119 > /Users/bdavis/.claude/jobs/c8f0f39d/tmp/m0_run_L$lv.log 2>&1
  echo "L$lv exit $?"; grep -E "^(kappa|controls|unseated|frozen|seated|seam)" /Users/bdavis/.claude/jobs/c8f0f39d/tmp/m0_run_L$lv.log | cut -c1-260
done
