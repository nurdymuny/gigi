#!/bin/bash
cd /Users/bdavis/.claude/jobs/c8f0f39d/tmp/s3d
export GIGI_RICCI_DATA_ROOT=/Users/bdavis/.claude/jobs/c8f0f39d/tmp/su2 OMP_NUM_THREADS=2
P=/Users/bdavis/Projects/tessera/venv/bin/python
D=research/su2-startup-sdirk-20261119; mkdir -p $D
L=/Users/bdavis/.claude/jobs/c8f0f39d/tmp/s3b/logs
for lv in 0 1; do for t in native pg; do for N in 4 8; do
  $P scripts/ricci_su2_pg_flow_cayley.py run $lv $t $N $D/L${lv}_${t}_N${N}_sdirk2.json --startup sdirk2 > $L/S3e_L${lv}_${t}_N${N}.log 2>&1 &
done; done; done
wait
for f in $L/S3e_*.log; do echo "== $(basename $f)"; tail -1 $f | cut -c1-160; done
