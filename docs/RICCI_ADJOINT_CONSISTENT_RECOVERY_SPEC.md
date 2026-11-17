# RF2H: adjoint-consistent curvature recovery (spec before implementation)

Claude, 2026-11-17, on Bee's instruction to "work under your best intuition and see if you can bring
it home", following `CLAUDE_RICCI_ASSESSMENT_2026_11_17.md`. Branch
`claude/rf2h-adjoint-consistent-recovery`, off `codex/feat-ricci-correspondence-checkpoint`.

## Frozen contract (unchanged)

These are unchanged:
- the exact signed links `a = I_h A` (the stored `arrays.npz` column 0);
- the original Whitney tangent mass `B_h = M` (`m.npz`) and incidence `D` (`d.npz`);
- the original BDM1 physical mass `H` (`h.npz`, q4 and q6) and the clock 1;
- the target velocity `-4a` and the full-mass norm.

BW05 is unchanged: the same n = 1, 2, 3, 5 levels, both finest observed orders ≥ 0.5, finest
full relative error < 5%, q4/q6 and solve differences < error/10. Abelian only; Berger is not run.

There's no target force, fitted clock, filter, regularisation parameter, input projection or removed
mode. The candidate uses geometry only to build a linear recovery, then applies it to the data.

## The consistency argument (what must be controlled)

**Recovery.** Write the curvature as a BDM1 field on the original mesh:

```text
F_R(a) = j D a + z Φ D a
```

- `j D a` is the lowest-order Whitney part, carrying the exact face fluxes.
- `z` spans the zero-flux ("free") BDM moments, two per face.
- `Φ` is a local, geometry-only linear map from face fluxes to free moments.

The field is closed and normal-conforming, and its face fluxes are exactly `D a` for every `Φ`.
The energy is `E_h(a) = ½ F_R(a)ᵀ H F_R(a)`, so the force is its exact derivative. It's a variational
recovery in the same family as RF2F and RF2G.

**Error.** Let `F = dA` be the smooth curvature and `c_F` its BDM1 interpolant (exact moments), so
that `c_F = j Φ_F + z φ_F`, with `Φ_F = D a` the fluxes and `φ_F` its true free moments. For every
variation η, using `L(η) = ⟨F, W_{Dη}⟩`, which is the stored load with its sign:

```text
dE_h(a)[η] − L(η) =  δ_accᵀ zᵀH(j+zΦ)Dη          (A) reconstruction error, δ_acc = ΦΦ_F − φ_F
                   + μ_Fᵀ Φ Dη                      (B) test-variation mismatch, μ_F = zᵀ H c_F
                   − ⟨F − c_F, W_{Dη}⟩               (C) BDM interpolation, O(h²)·O(h⁻¹)
```

Both (A) and (B) are tested against variations that the inverse inequality amplifies by h⁻¹. So
each has to vanish **exactly for locally affine closed curvature**, which leaves O(h) for smooth
F. This gives two linear conditions on `Φ`:

- **(Acc)** for every face f and every closed curvature affine in a chart at f (11 dimensions):
  `Φ_f Φ_F = φ_F(f)`.
- **(Adj)** for every edge e and every closed curvature affine in a chart at e (11 dimensions):
  `(Dᵀ Φᵀ μ_F)_e = 0`.

How the earlier attempts sit against these:

| Attempt | (A) | (B) |
|---|---|---|
| Original Whitney (Φ = 0) | violated | 0 |
| RF2G (minimum energy) | violated: free moments are not the interpolant's | 0 exactly: the correction is H-orthogonal to the free directions |
| RF2F (affine fit) | satisfied | violated |

The measured plateaus (26%, 20.6%, 80.9%) fit this. **No prior attempt imposed both.**

## Construction

- **Stencil of face f:** all faces of the two owner cells and of their face-neighbours (8 cells),
  as cover images built by deck-group breadth-first search. The quotient's injectivity radius
  (about π/10) is smaller than the stencil at coarse levels, so images are tracked explicitly, never
  by global index alone.
- **Basis fields:** charts are gnomonic at the face centroid (Acc) or edge midpoint (Adj). The
  basis is `V = v + Bξ/h` with `tr B = 0`. Fluxes and chord-barycentric moments are computed by
  quadrature on the projective image of the chord triangle, which is what `H`'s DOFs mean.
- **μ_F** is built cell by cell from `cell_masses` (the same function that assembled `H`), the
  stored moment ids and signs, on the cell images adjacent to that face image.
- **Solve:** the minimum-norm `Φ` subject to (Acc) and (Adj), via sparse LSQR. Both must hold to
  1e-9 relative afterwards, or the run refuses.

## Gates

1. **Focused tests (RED before GREEN):**
   - basis closedness (zero net flux per cell image);
   - moments summing to the flux;
   - Stokes (flux equals the circulation of the homotopy potential);
   - deck-image consistency (flux invariant under the deck group);
   - (Acc) and (Adj) residuals after the solve;
   - the energy derivative against finite differences;
   - zero data giving zero curvature.
2. **Mechanism removal:**
   - dropping (Adj) must not pass the gate (the RF2F regime);
   - dropping (Acc) must not pass the gate (the RF2G/Whitney regime).
3. **The BW05 run on all eight cases**, reported with the action and mass split.

The pass/fail decision uses the existing `numerical_gate` unchanged.

## Budget

Per-level construction plus evaluation under about 10 minutes on a laptop CPU. No native code
change, no full suite, no Berger, no merge.

## Not claimed in advance

The argument is an O(h) bound **for affine-exact constraints**. It is not a proof of mesh-uniform
stability of `Φ` (the minimum norm is reported as amplification), nor of the finite-level numbers.
The run decides.
