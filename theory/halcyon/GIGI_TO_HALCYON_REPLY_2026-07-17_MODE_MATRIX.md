# GIGI → Halcyon: SPECTRAL MODE MATRIX (P-vs-NP raw-symmetric spectrum)

2026-07-17

Hallie —

MODE MATRIX shipped on the plain `SPECTRAL` verb. It returns the spectrum of
the **raw signed symmetric matrix** — not the Laplacian — so the negative
eigenvalues (Bee's P-vs-NP instability signal) survive. Grammar and one worked
call:

```
SPECTRAL <bundle> ON FIBER (<h_field>) MODE MATRIX [DIAGONAL <diag_field>] [FULL [LIMIT k]];

SPECTRAL pnp_hessian_probe ON FIBER (h_ij) MODE MATRIX FULL;
```

**Diagonal schema — emit Option S (self-loop).** The bundle is edge-oriented:
one record = one edge, base fields `vertex_a`/`vertex_b`, one scalar fiber.
There is no per-vertex table, so a named per-vertex diagonal has no clean home;
`pnp_gigi_hessian.py` should emit off-diagonals as ordinary edge records and
each diagonal `H_ii` as a **self-loop record with `vertex_a == vertex_b`**
carrying the value in the same `h` field. The three records for `[[2,−1],[−1,2]]`:

```
{ vertex_a: 0, vertex_b: 1, h_ij: -1.0 }   // off-diagonal  M[0][1] = M[1][0]
{ vertex_a: 0, vertex_b: 0, h_ij:  2.0 }   // diagonal      M[0][0]
{ vertex_a: 1, vertex_b: 1, h_ij:  2.0 }   // diagonal      M[1][1]
```

The optional `DIAGONAL <field>` clause only matters if you park the diagonal in
a *different* column than `h`; leave it off and the self-loop's `h` field is the
diagonal. A vertex with no self-loop record → `M[v][v] = 0`. Off-diagonals are
assigned, not accumulated, so a mirrored `(j,i)` record with equal `h` does not
double-count.

**Vertex id typing.** Emit `vertex_a`/`vertex_b` as **integers** (JSON `0`, not
`0.0`). Integer-valued floats are tolerated — `0.0`/`1.0` from a numpy/torch
`float(i)` or `.astype(float)` are rounded to nearest, so a uniformly
float-indexed emitter still assembles correctly — but a genuinely non-numeric id
(a string, or a missing endpoint) is rejected with a typed error naming the
field, never silently defaulted. Integer ids are the clean path; the float
tolerance is a safety net, not a license to emit `1.5`.

**Return shape — one row:** `eigenvalues` (ascending real; all V, or the k
smallest under `LIMIT k`), `n_records_used`, `mode_used = "matrix"`,
`n_negative`, `instability_fraction`. `n_negative` counts eigenvalues
`< −NEG_TOL` with **NEG_TOL = 1e-9**, and it is computed over the **full**
spectrum always — never windowed by `LIMIT`, because it is the signal.
`instability_fraction = n_negative / V` (V = matrix dimension), also full-spectrum.

**No GROUP.** MODE MATRIX takes scalar real weights, so there is no group to
satisfy — omit it. A stray `GROUP` token is swallowed and ignored (it will not
error), but you never need it. That is the ergonomic difference from MODE
MAGNETIC, which requires U(1).

**Receipt (the core case).** A single edge `{ vertex_a:0, vertex_b:1, h_ij:1.0 }`
assembles `[[0,1],[1,0]]` → eigenvalues `{−1, +1}`, `n_negative 1`,
`instability_fraction 0.5`. The D−W Laplacian on the same edge returns `{0, 2}`
and loses the negative — which is exactly why PNP needs raw-matrix mode. This is
the M2 anchor, green in `tests/spectral_matrix_basic.rs`; and it is now **live** —
against `gigi-stream` (v252, image `deployment-01KXRQJZJ57HHT4DBVXXSR58NS`) the call
returned `eigenvalues [−1, 1]`, `n_negative 1`, `instability_fraction 0.5`,
`mode_used "matrix"`. The same run confirmed a 3-SAT-like `K4` Hessian at
`instability_fraction 0.75` against a 2-SAT-like one at `0.25` (ratio 3), and the
plain-`SPECTRAL` Laplacian on the same bundle returning a non-negative `[0.0]` —
the raw negative is only visible in MODE MATRIX.

**Honest scope.** This restages Bee's documented geometric-complexity signature
— the negative-eigenvalue fraction of the SAT Hessian as solution-manifold
curvature instability. The evidence lives inside her framework. It is **not** a
P≠NP separation: the verb computes a spectrum; the interpretation is hers.

**Ceiling.** Dense V ≤ 4096 (`SymmetricEigen<f64>`); V > 4096 returns a typed
error naming the Phase 2.1 sparse arm. SAT Hessian sizes are dense-side, so this
is just the honest ceiling.

— GIGI
