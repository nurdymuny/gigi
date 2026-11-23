# Independent assessment: the fixed-exterior obstruction

2026-11-17. Answers the "suggested first deliverable" in
`docs/RICCI_HANDOFF_2026_11_17.md`. **Assessment only.** No construction was changed, no
receipt was overwritten, no threshold was touched, and no global flow or test suite was run.

## How this was checked

Branch `codex/feat-ricci-correspondence-checkpoint`. I unpacked the archived evidence and confirmed
its integrity:
- every extracted `output/` entry matches `MANIFEST.json`;
- `attributions.npz` matches its pinned `arrays_sha256`;
- the boundary `diagnostic.json` matches the handoff's SHA-256 (`d8297e81…`).

The branch's own verifiers refuse on this Mac, because they hash absolute Windows paths. That is the
provenance refusal the handoff predicts, not a numerical failure. For this read-only analysis only
that path-hash comparison was neutralised. All numerics use the branch's own functions unchanged:
`geometry`, `construct`, `edge_audit`, `old_flux`, `hodge`, `unit_component`, `boundary_system`.
The scripts are `exterior_jumps.py` and `scale_check.py`.

**Reproduced exactly:**
- the edge-613 witness: cell 457 gives 0.2436842946, cell 461 gives 0.0759265385, gap 0.1677577561;
- the 3.7380948786 boundary mismatch.

## Finding 1: the obstruction belongs to the exterior field, everywhere, not to this patch

The outside targets are the original per-cell lowest-order field (`old_flux`: one constant
curvature two-form per cell, made metric-dual with that cell's own metric). A field like that is
built to share **normal** flux across faces. Nothing makes its **tangential** part continuous. Measured
directly on the original level-1 mesh, with the patch playing no role:

| Check | Result |
|---|---|
| Faces of the closed mesh with a tangential jump above 1e-9 | **1920 of 1920** |
| Tangential jump, all faces: median / 90% / max | 0.077 / 0.176 / 0.220 |
| Same, only faces touching no patch cell: median / 90% / max | 0.077 / 0.176 / 0.220 (identical) |
| Do cells 457 and 461 share a face? | No; they share only edge 613 |
| The six cells around edge 613, same point and direction | 0.244, 0.134 (patch cell 458), 0.076, 0.334, 0.333, 0.167 |
| Share of all mesh faces with a jump at least the edge-613 gap | 11% |

So the original field has **no single tangential value on any edge**. Around edge 613, six cells
give five clearly different values. The witness pair is two of those six, and an ordinary one at
that.

This makes the handoff's statement stronger. It isn't that these particular data happen to conflict
on 18 edges. Any patch boundary placed in this mesh meets the same conflict. That is a property of
the lowest-order reconstruction, independent of the measurements, the patch, the polynomial degree
and the patch radius.

**Consequence:** "add a collar" or "reconstruct a ring of neighbours" with an unchanged field beyond
it cannot succeed under a strong (pointwise, 1e-9) trace contract. The conflict just moves to the
new outer boundary, where it's equally dense. Strong matching works only if the reconstruction is
tangentially conforming all the way out, i.e. globally. The alternative is to change the matching
to something weaker. That's a precise statement of the handoff's "coupled reconstruction or change
the contract".

It also explains a historical pattern. RF2F and RF2G were global but used BDM1, which is H(div):
normal-conforming only. **No attempt so far has been tangentially conforming globally.** That's the
untested hypothesis the shared-curvature patch actually points at.

## Finding 2: a separate problem, the quadratic misbehaves at its own rim

The 3.738 mismatch is **not** explained by Finding 1. In the same whitened physical norm, the old
field's own inside-to-outside mismatch is 0.110. That's the intrinsic jump level, and it's 34 times
smaller.

| On the 432 boundary quadrature points | Original field outside | Shared quadratic |
|---|---|---|
| Tangential physical norm, median / max | 1.90 / 2.01 | 1.57 / **3.86** |
| Pointwise normal flux, max \|value\| | 1.10 (constant per face) | **2.79** |
| Integrated flux on every face | = data | = data (exact) |

The minimum-energy fit gets every integral exactly right but swings pointwise near the outer faces.
Its normal flux varies across a face, reaching 2.5 times the original, while the face average stays
exact. That's typical of a polynomial constrained only through integrals and judged at the patch
rim.

**Consequence:** even with Finding 1 solved, splicing this patch field at its rim would carry
O(field-size) interface errors into any weak coupling too. Any globalisation should use each local
reconstruction near its **centre** (here edge 103, whose full edge star is exactly this patch), not
at its rim.

## What I think the contract allows, and the bounded options

The measurements, mapping, metric, clock, Whitney test mass and BW05 are untouched by everything
below. What has to be decided is the **continuity requirement on the derived field**, which
Bee owns:

1. **Global tangential conformity (keeps the strong contract).** Build one field that is
   tangentially continuous everywhere, closed, and exact on every edge integral and face flux. The
   way to do this without a monolithic solve is **patchwise equilibration**:
   - on each vertex (or edge) star, solve a small closed, data-exact problem for a hat-function-weighted
     share of the data, with homogeneous tangential conditions on the star boundary;
   - then sum the pieces.

   Because each piece vanishes at its own rim, neither Finding 1 nor Finding 2 enters, and nothing
   is matched to a fixed exterior. This is the mechanism of Braess–Pillwein–Schöberl (2009) and
   Ern–Vohralík, including the Chaumont-Frelet–Ern–Vohralík extension operators already cited.
   Caveat: those results are Euclidean and polynomial. Keeping closure and data exact under the
   weighting, on a curved metric-dual field, is research work. The plain partition-of-unity sum ψF
   would break closure (d(ψF) = dψ ∧ F) and the data, which is exactly why the equilibration form is
   needed.
2. **Weak interface matching (changes the continuity question).** Require agreement only in the
   original Whitney test moments, which is what the force actually sees when tested against Whitney
   variations. The internal-face gate already checks these moments. This sits closer to the
   strong-adjoint-consistency question than pointwise traces do, but it is a different contract and
   needs Bee's explicit choice.

## Suggested cheap step before either option (not run)

This patch is the **complete edge star of edge 103**, so the patch energy determines the force on
edge 103 entirely locally. `attributions.npz` already holds per-cell `continuum` / `reference` /
`force` arrays. One bounded check: does the shared-curvature reconstruction's force at edge 103 sit
closer to the continuum reference than the original force does?
- **If not:** a conforming reconstruction doesn't fix the local force, and option 1 is unlikely to
  rescue BW05. That's worth knowing before building anything global.
- **If yes:** that's the first local evidence that tangential conformity attacks the actual error,
  which justifies a spec for option 1.

It's six cells, one edge and seconds of compute. It needs a short spec first (what "closer" means,
which reference, both signs of the derivative path) per the RED/GREEN discipline.

## Not claimed

No new construction, no gate change, no global or BW05 progress. The obstruction isn't presented as
a no-go for the research programme: weaker trace spaces and globally conforming reconstructions
remain open, as the handoff says.
