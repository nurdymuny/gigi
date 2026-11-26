# helike-view: an M8 scene, "The carrier" (spec)

26 November 2026. **A specification, not yet built.** It adds a fourth scene to `helike-view`, beside the ladder, the
sphere and the edge. The scene draws M8, the weighted geometry of the carrier
(`helike-M8-weighted-geometry-registration-2026-11-26.md`), from its receipts. It is for seeing what the tests say. It
decides nothing.

## 1. Principles (the viewer's existing rules, applied to M8)

- **Every number drawn comes from a receipt or a frame**, and every panel shows its frame's status chip. The viewer
  never recomputes a gate, a share, a crossing or a verdict.
- **OPEN is drawn as OPEN.** It gets its own neutral hatch, never a pass colour and never a fail colour. A gate shows
  its value against its tolerance, not just a tick.
- **Signed quantities are never drawn as a distribution.** The raw block matrix D_XY can have negative cross terms
  (A2.1). It is drawn as a signed grid, never as a stacked bar.
- **Anything the viewer computes itself is DisplayGrade.** Only the slice in §3.4 is computed in the viewer, from the
  exported coefficients, and its chip says DisplayGrade.
- The palette is the existing one:
  - HELIKE (cool blue) for Helike's channel and the H block;
  - RIVAL (amber) for the competitor and the C block;
  - MUTED for S and R;
  - PROVED (green) only for a gate that passed.

## 2. Data sources (read-only)

| source | produced by | used for |
|---|---|---|
| `research/m8-*/controls*/report.json` | `run_helike_m8.py --controls` | §3.1–§3.3 |
| `levels_*.json` (cone-levels exports) | `helike cone-levels` | §3.4 slice; §3.5 level lines |
| case reports (grid stage) | the M8 grid runner (after the registered grid) | §3.5 |
| `research/m8-*/receipts/*.txt` | the test and mutant runs | §3.6 |

A new `Request::LoadM8(path)` in `solver.rs` reads a directory and emits frames on new scenes. The scene names are
`m8-controls`, `m8-slice` and `m8-case`. The status is `Computed` for receipt values and `DisplayGrade` for the slice.
No new solver work is started from the viewer.

## 3. Panels

### 3.1 The gate board (controls)

- **Layout.** A matrix with one row per control point (g², Mq) and one column per gate:
  - Q0a, the identity;
  - Q0b, the quadrature seam;
  - Q2, the cross-check;
  - positivity;
  - the residual flag.
- **Each cell** shows the value, its tolerance, and the ratio as a short bar on a log scale. A cell is green if the gate
  passed and hatched if it was OPEN. The residual flag is a marker, never a gate colour.
- **Each row** carries:
  - its seam order pair, for example "12/14";
  - an "amendment 4" tag where the pair was set by degree;
  - a link chip to the superseded pre-amendment row, if one exists. That row is drawn struck through and greyed: kept,
    not hidden.
- **Hover** shows the exact numbers from the report.

### 3.2 Where the energy lives (shares)

- **Main view.** For each point and channel, a horizontal stacked bar of the positive shares π_H, π_C, π_S and π_R.
  These sum to 1 by construction, so stacking is honest. A vertical rule marks the decided threshold of ½ on the
  channel's own block, and the per-point label (ALIGNED, DRESSED or TURNED) sits at the right.
- **Inset.** The raw block matrix D_XY/D as a 4×4 signed grid. The colour diverges around zero, with negative cells
  hatched. It is titled "raw, signed: not a distribution".
- **Toggle.** The coarse and fine orders side by side, so the seam can be seen as two nearly identical bars.

### 3.3 Shape against level (the "why")

For each point, a dumbbell per channel joins the bare-shape price P_X (an open circle) to the level Δ_X (a filled
circle). The segment between them is the dressing discount, labelled with its value.
- Two arrows compare the orderings: P_C against P_H, and Δ_C against Δ_H.
- A caption says "decided by shape" if the arrows agree, and "decided by dressing" if they differ.
- Q4 appears only as a greyed chip saying "curvature: not computed". When Q4 exists, it is drawn as a floor line
  labelled "constrains how cheap, not which". It is never drawn as an explanation of an ordering (A1.2).

### 3.4 The slow function (the geometric picture)

This is the picture behind the numbers: f = ψ/ψ₀ on a two-dimensional slice of field space.
- **Plane.** A selector picks the plane: (h₁, h₂), Helike's plane; (h₁, c); (c, x); or (s₁, s₂). The other four normal
  coordinates are held at 0.
- **Field.** A heat map of f, with contours of the vacuum weight ψ₀² drawn over it, since that weight is where the
  geometry lives. Arrows show the frame gradient v = G̃^{1/2} ∇f, coloured by the block of their largest component.
- **For Helike's pair**, a toggle shows f₊ = (f₁ + i f₂)/√2 as phase and modulus. Her charge-2 winding should appear as
  two turns of phase around the origin of the (h₁, h₂) plane.
- **Computation.** The viewer evaluates the Hermite basis from the export's coefficients on a 241 × 241 slice. That is
  cheap, about 8,000 basis functions per point. The chip says **DisplayGrade**. A caption gives the plane, the held
  coordinates, and "display only; shares are in §3.2".
- This panel needs the Hermite evaluation and the chart, which live in `helike-quantum`. Either `helike-view` takes a
  dependency on it, or the two functions move into `helike-core`. The second keeps the viewer light.

### 3.5 The carrier along s (grid cases)

For one (t, g²) case at a time, with s on the shared horizontal axis:
- **Levels strip.** The sector's levels against s: the charge-2 branches for Helike, and the charge-0 singles for the
  competitor. Each has its uncertainty band, which for a pair includes half its splitting. The followed carrier is a
  thick line. The other tracked branches are thin.
- **Markers on the levels strip:**
  - a verified crossing, as a filled diamond at its bracket;
  - a possible crossing left unresolved, as a hatched diamond;
  - the registered inter-sector switch, as a dashed vertical line labelled "the gap hands over", touching neither
    channel's line.
- **Share strip.** The carrier's own-block share against s, with the ½ threshold line. Between grid points, the
  accepted refinement nodes appear as small dots, so a resolved narrow transition shows its width.
- **State band.** A thin band under the share strip, coloured HELD, HANDED, DISSOLVED or OPEN, step by step. DISSOLVED is
  final and is drawn as a hard stop.
- **Ticks:**
  - selection_changed, as a small tick under the band, with the tooltip "selector and carrier disagree";
  - refinement depth, as tick height per step, up to the 1/32 limit;
  - rejected coarse matches, as faint crosses.
- **Verdict chip** for the case: PERSISTS, HANDS OVER, DISSOLVES or OPEN, with the reason on hover.

### 3.6 The test receipts

A sidebar lists the fixture tests and mutants from the receipts.
- **Tests:** the name, a tick, and the first line of each docstring.
- **Mutants:** "caught by an assertion" or "missed", and the totals (14/14, 5/5, and so on).
- If a receipt is missing, or its code commit differs from the report's, the sidebar says so in amber. That is how the
  viewer enforces the registration's rule that no M8 value is released without receipts.

## 4. Controls

- the scene picker: "The carrier";
- the point selector (g², Mq) for §3.2–§3.4, and the case selector (t, g²) for §3.5;
- the channel toggle: Helike or competitor;
- the slice plane for §3.4, and the f₊ phase view for Helike;
- the order toggle: coarse or fine;
- `--shot m8` in `parse_shot`, for a headless screenshot of the scene, as for the other scenes.

## 5. Acceptance (before the scene is used to show anything)

1. **Numbers match receipts.** A unit test loads the controls report into the scene's data model and checks that every
   value the scene would draw equals the report value, bit for bit, for shares, gates, prices and discounts.
2. **OPEN is never green.** A test feeds a report with an OPEN gate and checks the cell's colour is the hatch.
3. **No raw distribution.** A test feeds the reviewers' (5, 5, −9) raw contributions into the inset and checks that the
   inset is signed and that no stacked bar is built from raw values.
4. **Superseded rows are kept.** A pre-amendment row is present, struck through, and linked.
5. **The slice is labelled DisplayGrade.** Its chip and caption are checked in a screenshot test.
6. **Receipt guard.** A missing receipt turns the sidebar amber.

## 6. Scope

The scene is a reading aid for M8 inside the homogeneous reduction. It shows the registered verdicts and the numbers
behind them. It adds no claim, and a picture it draws is never evidence of anything a receipt does not already say.
