# Reading the Curve, Not Counting the Points (paper)

`paper.pdf` is the compiled manuscript (14 pages). Every number, table and figure is regenerated from the frozen, hash-checked inputs of the GIGI Ricci checkpoint. The archives under `research/ricci-checkpoint-20261117/` must be extracted under `output/`.

```bash
python docs/papers/two-ring-pg/make_data.py              # errors.json, spectra.npz, conditioning.json  (~15 min)
python docs/papers/two-ring-pg/numerical_range.py        # numerical_range.json (n = 1, 2)
python docs/papers/two-ring-pg/make_tables.py            # tables/*.tex (orders from absolute error, as in BW05)
python docs/papers/two-ring-pg/make_figures.py           # figures/*.pdf
cd docs/papers/two-ring-pg && pdflatex paper.tex && pdflatex paper.tex
```

`make_tables.py` also reads the gate receipt `research/ricci-pg-tworing-20261117/diagnostic.json`, produced by `scripts/ricci_pg_tworing_gate.py`. The paper needs only standard LaTeX packages; `enumitem` and TikZ are not used.
