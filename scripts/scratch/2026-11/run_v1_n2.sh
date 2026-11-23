#!/bin/bash
while pgrep -f ricci_su2_pg_spectrum >/dev/null; do sleep 300; done
cd /Users/bdavis/.claude/jobs/c8f0f39d/tmp/var
GIGI_RICCI_DATA_ROOT=/Users/bdavis/.claude/jobs/c8f0f39d/tmp/su2 OMP_NUM_THREADS=8 /Users/bdavis/Projects/tessera/venv/bin/python scripts/ricci_variational_v1.py 1 research/variational-v1-20261119/L1.json 2>&1 | tail -3
