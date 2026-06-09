# MVP scope — what shipped and what's next

## Shipped (v0.1.0)

**Kernel**
- Subclass of `ipykernel.kernelbase.Kernel`
- Default cell language: GQL (POSTed to `/v1/gql`)
- `%env KEY=value` to update session config inline
- `%%config` to dump current session config

**Cell magics**
- `%%gql` — explicit GQL form (equivalent to default)
- `%%commutator` — JSON body → `POST /v1/causal_states/commutator`

**Renderers**
- Generic JSON pretty-printer (for `%%gql` results)
- Custom `Commutator` renderer — box-drawn table with regime tag, TV, Hellinger, KL (bits or "divergent"), and both arms

**Install**
- `pip install -e .` and `python -m gigi_notebook --install` register a kernelspec called `gigi` with display name "GIGI (GQL)"
- `python -m gigi_notebook --uninstall` removes it

**Tests** (`sdk/notebook/tests/`)
- Unit tests for renderers + dispatch (7 tests, no network)
- Live smoke tests against `gigi-stream.fly.dev` (2 tests, skipped without env)

**Demo**
- `demo/hello_gigi.ipynb` — walk through config, default GQL, both commutator variants

## Next (v0.2 and onwards)

**More cell magics**
- `%%transport` — POST to `/v1/bundles/{name}/transport` (TRANSPORT verb), render the result as a vector overlay on the belief simplex
- `%%marcella` — inline Python AI layer integration
- `%%md` — markdown (Jupyter already does this, just want a clean explicit form)
- `%%scan` — parameter sweep helper for commutator runs over an (α, β) grid

**More renderers**
- `BundleSummary` → SVG fiber visualization with state count and stationary belief
- `Commutator` (scan output) → 2-D heatmap of TV across (α, β)
- `TransportResult` → vector overlay on the belief simplex (T11/T12 demos)
- `MarcellaDiscourseManifold` → 2-D projection of the SwDA discourse manifold

**Embedded substrate via PyO3**
- Current state model: HTTP to gigi-stream (works in JupyterLab, Colab, VS Code)
- Next: optional embedded GIGI via PyO3 bindings so a notebook can run against a local in-process GIGI engine without spinning up a server
- Same kernel API, transport layer becomes pluggable

**Quality of life**
- Tab-completion in GQL cells (LSP-driven from the engine's parser)
- Multi-cell display: chain commutator results into a session-level summary at any time
- Cell tags as bundle scopes (so a cell tagged `bundle:my_data` operates on `my_data` by default)

## Out of scope (for now)

- Kernel-less notebook viewers (nbviewer renders the JSON; the kernel doesn't ship a special viewer)
- Multi-language kernel (this is GIGI + magics; if you want Python, use the regular ipykernel and import gigi-notebook as a library)
- Real-time WebSocket cells (interesting, but the request/response cell model is the Jupyter contract)
