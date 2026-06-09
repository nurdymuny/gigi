# gigi-notebook — Jupyter kernel for GIGI

A Jupyter kernel that talks to a running `gigi-stream` server. Write GQL
queries (or HTTP verbs the engine exposes) in notebook cells; get
structured results back with custom rich renderings.

This is the MVP — minimum viable kernel + one cell magic + one custom
renderer. See `MVP_SCOPE.md` for what's included vs. what's next.

## What you can do today

- **GQL cells**: write a GQL query (default cell language), get the
  parsed-result rows back as a Python `dict`.
- **`%%commutator`**: shorthand for the new `POST /v1/causal_states/commutator`
  endpoint. Body is JSON; the response is rendered as a structured
  regime-tagged table.

State (open HTTP session, current bundle, last result) persists between
cells the way Python objects persist in a normal notebook.

## Install

```bash
cd sdk/notebook
pip install -e .
python -m gigi_notebook --install
```

This registers a kernel called `gigi` with Jupyter. Pick it from the
kernel selector in JupyterLab, VS Code, Colab, etc.

## Configure

Set environment variables before launching Jupyter:

```bash
export GIGI_URL="https://gigi-stream.fly.dev"   # or http://localhost:3142
export GIGI_API_KEY="..."                        # your X-API-Key value
```

These can also be set per-cell with `%env GIGI_URL=...`.

## Quick example

```
# Cell 1 — default GQL
SHOW BUNDLES

# Cell 2 — magic for the causal-states endpoint
%%commutator
{
  "a": {"kind": "hmm", "alpha": 0.2, "beta": 0.3, "symbol": 0},
  "b": {"kind": "hmm", "alpha": 0.2, "beta": 0.3, "symbol": 1},
  "base_belief": [0.5, 0.5]
}
```

Cell 2 renders the commutator result as a tagged table:

```
┌──────────┬───────────────┐
│  regime  │     smooth    │
├──────────┼───────────────┤
│   TV     │ 0.10619       │
│  Hel     │ 0.07520       │
│   KL     │ 0.03266 bits  │
└──────────┴───────────────┘
forward : [0.4469, 0.5531]
backward: [0.5531, 0.4469]
```

## What's next

See `MVP_SCOPE.md` for the planned cell magics (`%%gql`, `%%transport`,
`%%marcella`) and custom renderers (commutator heatmap, bundle SVG,
transport vector overlay on the belief simplex).
