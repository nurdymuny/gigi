#!/bin/bash
cd /Users/bdavis/.claude/jobs/c8f0f39d/tmp/var
export GIGI_RICCI_DATA_ROOT=/Users/bdavis/.claude/jobs/c8f0f39d/tmp/ricci OMP_NUM_THREADS=4
P=/Users/bdavis/Projects/tessera/venv/bin/python
D=research/variational-v1c-20261119
for lv in 0 1; do
  $P scripts/ricci_variational_v1.py $lv $D/L$lv.json --generators constants > /Users/bdavis/.claude/jobs/c8f0f39d/tmp/v1c_L$lv.log 2>&1
  echo "L$lv exit $?"; tail -3 /Users/bdavis/.claude/jobs/c8f0f39d/tmp/v1c_L$lv.log | cut -c1-300
done
