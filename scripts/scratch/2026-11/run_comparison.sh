#!/bin/bash
# Cohort comparison: kmer + hybrid CPU encoders -> TESSERA detection -> GIGI eval.
# Usage: run_comparison.sh <encoder>   (kmer | hybrid)
set -euo pipefail
ENC="$1"
T=/Users/bdavis/.claude/jobs/c8f0f39d/tmp
REPO=/Users/bdavis/Projects/tessera/.claude/worktrees/gigi-eval
PY=/Users/bdavis/Projects/tessera/venv/bin/python
OUT=$T/cohort_$ENC
cd "$REPO"
export GIGI_URL=http://localhost:3199 GIGI_API_KEY=$(cat $T/gigi_key)

filter() { grep -vE 'NumPy 1|compiled using|_ARRAY|downgrade|pybind|rebuild|may crash|support both|^\s*$|torch/|Traceback|File "|device: torch|UserWarning|A module that|expect that|If you are|from \.|import torch' || true; }

echo "== $ENC: embed + detect"
$PY run_local.py --stages embed,detect --encoder "$ENC" --max-len 491520 \
    --data-dir $T/localrun_data --out "$OUT" 2>&1 | filter | grep -E '^\[' | grep -vE 'embedded [0-9]+/' || true

for P in "Salmonella enterica" "Escherichia coli"; do
  tag=$(echo "$P" | cut -d' ' -f1 | tr 'A-Z' 'a-z')
  echo "== $ENC: GIGI eval $P"
  $PY scripts/gigi_eval.py "$OUT/embeddings.npy" "$OUT/embeddings_metadata.csv" \
      --pathogen "$P" --label pds_acc --min-class 3 --temporal \
      --out "$OUT/gigi_eval_${tag}_pds.json" 2>&1 | filter | grep -E '^  [a-z]|^[A-Z][a-z]+ [a-z]+:|saved|Error' || true
done
echo "== $ENC: done"
