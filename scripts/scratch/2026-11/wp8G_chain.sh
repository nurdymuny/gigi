#!/bin/bash
cd $SCRATCH/rtc
while pgrep -f "wp8G_g2.json" >/dev/null; do sleep 30; done
echo "g2 finished $(date); starting sp3" >> ../wp8G_sp3.log
OMP_NUM_THREADS=16 /Users/bdavis/Projects/tessera/venv/bin/python scripts/edge_wp8_two_new_groups.py ../wp8G_sp3.json sp3 >> ../wp8G_sp3.log 2>&1
