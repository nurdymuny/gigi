# The SU(2) force, batched across cells (spec)

2026-11-19. Written before any batched code ran. Research script
`scripts/ricci_su2_batched.py`, **not an engine run**. It changes how the S3/S3b force is
computed, not what is computed.

## In plain English

Every SU(2) script so far computes the force one cell at a time in a Python loop. At n = 3
that costs about 146 s per two-ring force evaluation and 41 s per native one (S3b, six
runs sharing the machine). The arithmetic per cell is small; the loop is the cost. This
rewrite does the same arithmetic for all cells at once as array operations. It must give the
same numbers as the loop, to rounding, before anything is run with it. A GPU version comes
second, and is measured against the batched CPU version in the same way.

## What is batched

Inside `Lattice` of `scripts/ricci_su2_pg_flow.py`, for the native and two-ring trials:

- **Local links and face defects.** Gather each cell's six links with the reversal sign
  applied, then the four face defects, as (cells × 6 × 4) and (cells × 4 × 4) arrays.
- **Defect derivatives.** The exact product-rule derivative of
  `ricci_su2_pg_force.defect_derivatives`, with the prefix and suffix products formed for
  all cells at once: (cells × 6 × 3 × 4 × 4).
- **Two-ring pairing.** The matrix T (faces × listed faces) depends only on the metric. It is
  formed for all cells in one contraction per metric, with each cell's face list padded to
  the longest, padding columns zero.
- **Transport tree.** Each cell's breadth-first tree is flattened into depth layers. Layer d
  is one batched quaternion product for every (cell, vertex) at depth d.
- **Face holonomies.** Each listed face's holonomy, then its Ad by the tree element at its
  first vertex, gathered for every (cell, listed face) at once.
- **Scatter.** The lift (−R(U) for a reversed edge, the identity otherwise) and the
  accumulation into the global gradient, with `np.add.at`.
- **Mass.** All (cells × 6 × 6) blocks 4 w m1[e, f] (R_e L_e)ᵀ(R_f L_f) at once. The sparse
  pattern is built once.
- **Unchanged:** `metric_pieces` (already batched), the CG mass solve, `exact_links`, the
  Newton matrix K₀ (cached for n ≤ 3; not rebuilt here).

## Parity gates (fixed now)

The loop code is the reference. Relative errors are ‖batched − loop‖/‖loop‖ in the
Euclidean or Frobenius norm. Each is checked for native and two-ring at n = 1, 2, 3, at two
states (the stored initial links, and the saved S3b final state `L{level}_pg_N8.final.npy`),
and at two metrics (g(0) and g(0.02)).

- **P-grad.** Gradient: ≤ 10⁻¹².
- **P-mass.** Mass matrix: ≤ 10⁻¹³.
- **P-vel.** Velocity (gradient, mass and the CG solve together): ≤ 10⁻¹⁰. The CG
  tolerance is 10⁻¹², so this is the solve's own precision with a margin.
- **P-angles.** Face angles: ≤ 10⁻¹³.
- **P-run.** Rerun the S3b Cayley step with the batched force for native and two-ring at
  n = 1 and n = 2 with N = 8. E(0.02) must match the committed receipts to 10⁻⁹ relative,
  with the same Newton iteration count at every step.

The rounding order of some products changes (for example (R L)ᵀ(R L) in place of
Lᵀ(RᵀR)L), so bit-identical output is not required. The thresholds are four or more orders
above double precision and four or more below anything the S3 gates compare.

## Timing (measured, not gated)

Seconds per force evaluation, one process, nothing else running, for the loop and the batched
version at n = 1, 2, 3. At n = 3 the loop is timed on one evaluation only.

## GPU (second step, same gates)

The batched arrays are moved to PyTorch in float64 on the RTX 5070 (consumer FP64 runs at
about 1/64 of FP32). Parity: P-grad and P-mass against the batched CPU version at the same
thresholds. It is kept only if it is faster than the batched CPU version at n = 3. That is
reported either way.

## Not done here

Rebuilding K₀ with the batched force; n ≥ 4; the engine path (BW08).

## Amendment 1 (2026-11-19): the metric pieces

**Written after the CPU parity and timing results, before any change to the metric pieces.**
No existing gate changes; one gate is added.

**What was measured.** At n = 3, one batched force evaluation spends 2.4 s in
`metric_pieces` and 0.07 s (native) or 0.5 s (two-ring) in the gradient, 0.07 s in the mass
and 0.06 s in the CG solve. The metric pieces were listed above as unchanged.

**Change.** Two things, both in the batched class only:
- **Cache by metric.** The metric pieces depend on g alone, not on the links. Every Newton
  iteration of a Cayley step uses the same midpoint metric, so they are kept for the last g
  and recomputed only when g changes. Exact: the same numbers, computed once.
- **Contraction order.** The two four-operand contractions (m1, m2) and G are evaluated with
  an optimised contraction order. This changes rounding only.

**New gate.** **P-metric:** m1, m2, G, w and the volume against the loop's `metric_pieces`,
≤ 10⁻¹³ relative, at n = 1, 2, 3, at g(0) and g(0.02). P-grad, P-mass, P-vel, P-angles and
P-run are rerun with the change, at the same thresholds.

**GPU.** The GPU step is still run as specified, against this version.
