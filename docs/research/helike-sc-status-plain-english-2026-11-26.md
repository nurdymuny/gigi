# Where S_C stands, in plain English (26 November 2026)

**What S_C says.** In the charge-0 sector, at most two energy levels sit below a threshold ρ_C. That is the one
hypothesis behind "Helike's level is below the competitor's" at the weak-coupling controls. Everything else in that
result is already proved.

**What I proved overnight** (in rigorous ball arithmetic, at both controls):

1. **The cubic constant C₃.** The old bound never finished, because the potential has a rotation symmetry, so its
   worst direction is a whole circle rather than a point. Fixing the rotation turns circles into points, and the bound
   finishes in 15 seconds.
2. **The folded base.** To bound levels from below, I start from a simpler valley that sits everywhere under the true
   one, then deform it back. The old version flattened every direction as if it ran the full width of the tube
   against the worst tilt. But the tube is narrow in the stiff directions, the tilt is weak in the soft ones, and the
   quartic pushes back up at the walls. Folding each direction only by what it really pays leaves 23 levels to work
   through instead of 92 (or 38).
3. **The outside.** Away from the valley the potential is above ρ_C everywhere. I checked it ray by ray from the
   origin, near the valley in its own coordinates and far away in symmetric matrices. Far out, the determinant's pull
   is always caught by the quartic (AM–GM on singular values).
4. **The starting spectrum** of the folded base, enclosed to 10⁻¹³.

**What is computed but not yet proved.** Running the deformation (the Lehmann homotopy) in ordinary floating point,
the fifth level ends up 0.21 to 0.24 above ρ_C at both controls. That is the margin S_C needs, and it is large.

**What is left.** The same deformation, redone in interval arithmetic (stage R2), and the frozen ρ_C from the
ordering receipt in place of the recomputed one. Until then S_C is a stated hypothesis, and the paper says so.
