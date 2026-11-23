#!/bin/bash
cd /Users/bdavis/.claude/jobs/c8f0f39d/tmp/paper
export GIGI_RICCI_DATA_ROOT=/Users/bdavis/.claude/jobs/c8f0f39d/tmp/su2 OMP_NUM_THREADS=8
P=/Users/bdavis/Projects/tessera/venv/bin/python
D=research/su2-spectrum-n3-20261119
mkdir -p $D
for lv in 0 1; do for t in native pg; do
  $P scripts/ricci_su2_pg_spectrum.py $lv $t 12 $D/L${lv}_${t}.json 2>&1 | tail -1 | cut -c1-200
done; done
for t in native pg pg_onering; do
  $P scripts/ricci_su2_pg_spectrum.py 2 $t 12 $D/L2_${t}.json 2>&1 | tail -1 | cut -c1-300
done
