# helike-view: a "Controls" scene for the registered control runs (spec)

26 November 2026. **A specification, not yet built.** It adds a fourth scene to `helike-view`, beside the ladder, the
sphere and the edge. The scene shows the **registered control runs as they happen and after they finish**. These are
the checks every experiment must pass at the weak-coupling controls before it may touch the grid. The scene reads
receipts and progress files. It decides nothing.

*Revised the same day: the first version centred on M8's geometry. Bee asked for a view of the controls we are
running, so the control runs now come first, and the M8 geometry views are panels inside the M8 suite (§4).*

## 1. Principles (the viewer's existing rules, applied to controls)

- **Every number drawn comes from a receipt or a progress event**, and every panel shows its frame's status chip. The
  viewer never recomputes a gate, a seam or a verdict.
- **Three outcomes, three looks.**
  - PASS (or RESOLVED, CORRIDOR, GLUING-INVARIANT) uses PROVED green.
  - FAIL uses a red outline.
  - OPEN uses its own neutral hatch, never green and never red.

  A check shows its **value against its tolerance**, not just a tick.
- **The registered predictions are drawn beside the outcomes.** Each control suite registered predictions before it
  ran, and a prediction board (§3.3) shows each one with what happened.
- **Superseded runs are kept, not hidden.** An example is the M8 Mq = 10 run before amendment 4. Its tile is drawn
  struck through, linked to the amendment that superseded it.
- The palette is the existing one: HELIKE blue, RIVAL amber, MUTED slate, and PROVED green only for a passed check.

## 2. The suites, and where their data comes from

Each suite has a small adapter that maps its report into one common model. Every suite is a set of control points, and
every point is a set of checks, each with a value, a tolerance and an outcome.

| suite | registration | points | checks per point | source |
|---|---|---|---|---|
| Domain gate (§5.7) | flux-method amendment §5.7 | g² ∈ {0.002, 0.004} × Mq ∈ {8, 10} | per state: e_cut ≤ 3·10⁻⁴, tail, normalisation; ratio ℓ/r shown as *reported only* | `research/flux-grid-20261126/gate57-controls/report.json` |
| Gluing (§5.5, §5.8) | §5.5, corrected in §5.8 | same four | S1 identification, S2 quadrature, S3 truncation, S5 cutoff radius (both radii) | `…/gluing-controls-gate57/report.json` |
| Heat signal | heat-signal registration, amendments 1–2 | g² ∈ {0.002, 0.004} | T1 corridor per sector, T1b, T4 seams; T2 tone across the pair; T3 reported only | `research/heat-signal-20261126/controls/report.json` |
| M8 | M8 registration, amendments 1–4 | same four | Q0a identity, Q0b seam (with its order pair), Q2 cross-check, positivity, residual flag; own-block share and label | `research/m8-20261126/controls*/report.json` |
| S_C, R2 | R2 registration | g² = 0.002, then 0.004 | stage progress, then interval ℓ₅ against ρ_C | `research/sc-r2-20261126/r2_g*.txt` / `.json` |

A new `Request::LoadControls(root)` in `solver.rs` scans these paths, emits one frame per suite on scene `controls`,
and rescans when a file changes. Receipt values carry status `Computed`. R2's interval result carries
`IntervalEnclosure`.

## 3. The scene

### 3.1 The control board (the main view)

- **Layout.** One row per suite, one column per control point, so every suite's four points line up under each other.
- **Each tile** shows:
  - **the state:** queued, running, PASS, OPEN or FAIL, as in §1;
  - **a mini-gauge per check:** a log-scale bar of value ÷ tolerance, where left of the line is inside, plus the
    numeric value on hover;
  - **the commit** the run used, and for M8 its seam order pair, for example "12/14" with an "amendment 4" tag;
  - **elapsed time** for a running tile, from the progress events (§5).
- **A suite verdict** at the row's end: for example "4/4 PASS", "controls_invariant", "controls_pass", "3/4 RESOLVED ·
  1 running".

### 3.2 The tile detail

Clicking a tile opens its checks in full.
- **Domain gate:** a table of the three gated states, with e_cut, tail², normalisation width and the reported ratio
  ℓ/r. The ratio sits in a greyed column labelled "reported, not decided". At g² = 0.004, Mq = 10 it shows 1.39·10⁻³,
  above the old 10⁻³ line, next to e_cut = 5.6·10⁻⁶ PASS. That is the picture of why §5.7 exists.
- **Gluing:** the four seams, with S5's two radii and both verdict tuples side by side.
- **Heat signal:** for each sector, the sources as rows. Each row has a dot on the corridor level, the missing weight as
  a hollow tail, and the corridor's energy.
- **M8:** the gates as gauges, plus the M8 geometry panels of §4.
- **R2:** the stage log as a timeline (VSCF built, restricted span, exact blocks, …). When the result lands, the interval
  ℓ₅ is drawn as an error bar against the line ρ_C.

### 3.3 The prediction board

- **Rows.** One per registered prediction, quoted from its registration. Examples:
  - "e_cut ≈ 5.6·10⁻⁶ at g² = 0.004, Mq = 10";
  - "both M8 shares ≥ 0.9 at the controls";
  - "order-14 shares agree with the seen order-12 values to 10⁻³".
- **Columns:** the predicted value or condition, the outcome, and a mark (met, missed or not yet run).
- **A missed prediction stays in view, in amber.** It is not a failure of the run. It is information about the
  hypothesis.
- **Source.** The predictions are read from a small `predictions.json` per suite, hand-written at registration time and
  committed with the registration. The viewer does not parse prose.

### 3.4 History

A strip under the board lists amendments in time order (§5.7, §5.8, M8 A4, …). Each one links to the tiles it changed,
and to the superseded tiles it left struck through.

## 4. M8 geometry panels (inside the M8 tile detail)

- **Where the energy lives.** Stacked bars of the positive shares π_H, π_C, π_S, π_R for each channel. These sum to 1,
  so stacking is honest. A rule marks ½ on the channel's own block. A toggle overlays the coarse and fine orders, so the
  seam can be seen.
- **The raw block matrix** D_XY/D, as a signed 4×4 grid titled "raw, signed: not a distribution". It is never stacked.
- **Shape against level.** A dumbbell per channel joins the bare-shape price P_X to the level Δ_X. The segment is the
  dressing discount. A caption reads "decided by shape" or "decided by dressing".
- **The slow function f = ψ/ψ₀** on a selectable two-dimensional slice of normal coordinates, over contours of ψ₀².
  Helike's pair has a phase view, where her charge-2 winding appears as two turns. The viewer computes this from the
  exported coefficients, so its chip says **DisplayGrade**. Either the viewer depends on `helike-quantum` for the
  Hermite basis and chart, or those two functions move to `helike-core`, which is preferred.

## 5. Live progress (a small runner change)

For running tiles to move, each control runner appends one JSON line per stage to `progress.jsonl` in its output
directory:

  {"suite": "M8", "g2": 0.004, "Mq": 10, "stage": "order 14: helike level 2", "t": "…", "commit": "…"}

The stages are coarse: the export, then each order and state, then the done line with its status. The viewer tails the
file. A runner that writes no progress file still works; its tiles simply show queued, then finished. This is the only
change the scene asks of the runners. It adds no computation, and it changes no receipt.

## 6. Later: the grid stage (sketched, not specified)

When the registered grid has run, a case view will show the M8 carrier along s, using the same rules:
- the sector's level lines with uncertainty bands;
- verified and unresolved crossing markers;
- the inter-sector "gap hands over" line;
- the own-share strip with its ½ line;
- a HELD / HANDED / DISSOLVED / OPEN band;
- selection_changed ticks.

That view is specified when the grid's receipt format is fixed.

## 7. Controls of the scene

- the scene picker: "Controls";
- a suite filter, and clicking a tile for its detail;
- a live toggle, which tails the progress files;
- a coarse/fine toggle for M8, and the slice plane and phase view;
- `--shot controls` in `parse_shot`, for a headless screenshot, as for the other scenes.

## 8. Acceptance (before the scene is used to show anything)

1. **Numbers match receipts.** For each suite, a unit test loads its report through the adapter and checks every value
   the scene would draw against the report, bit for bit.
2. **OPEN is never green.** A report with an OPEN check renders the hatch. A FAIL renders the red outline.
3. **Reported-only values never colour a tile.** The domain gate's ℓ/r column, and the heat signal's T3, are drawn
   grey whatever their values.
4. **Superseded tiles are kept.** The M8 pre-amendment-4 Mq = 10 tiles are present, struck through, and linked to A4.
5. **A missing receipt is visible.** If a suite's report is missing, or from a different commit than its test
   receipts, its row says so in amber.
6. **The slice is labelled DisplayGrade**, checked in a screenshot test.

## 9. Scope

The scene is a reading aid for the registered controls. It shows what the receipts say, and how each run compares with
what was predicted before it ran. It adds no claim, and no picture it draws is evidence of anything a receipt does not
already say.

## 10. Build status (26 November)

Built in `helike-view` as the fourth scene, "Controls" (`--shot controls`; `--shot controls-lower` for the detail,
predictions and history). Modules: `controls.rs` (adapters and model), `view_controls.rs` (drawing), `theme.rs`
(the shared look, now applied to every scene).

- **Suites drawn from receipts:** domain gate (§5.7), gluing (§5.5/§5.8), heat signal. M8 and S_C R2 have no
  controls report on this branch, so their rows show the missing receipt in amber, with the expected path.
- **Predictions:** a `predictions.json` beside each suite, quoting its registration. A machine-readable condition
  (equals, an inequality, or agreement to the quoted significant figures) is compared with the report value. A
  condition that is not machine-readable stays "not yet", and its outcome is written by hand.
- **History:** `research/controls-history.json`. The two first-stage gluing tiles that §5.6 and §5.7 replaced are
  read from the archived report, and drawn struck through under their columns.
- **Progress:** `scripts/helike_progress.py` appends the §5 lines. The heat-signal and gluing control runners call it.
- **Acceptance:**
  - **1:** every drawn value is compared bit for bit with the report, through its JSON pointer.
  - **2 and 3:** OPEN, FAIL and reported-only checks are tested.
  - **4:** the mechanism is tested on the gluing tiles that §5.6 and §5.7 replaced.
  - **5:** a missing receipt is tested.
  - **6:** not done. The M8 geometry panels (§4), including the DisplayGrade slice, wait for an M8 report and its
    coefficient export.
- **Not drawn because the receipt lacks it:** the domain gate's tail² and normalisation-width values. Its report
  records only whether each budget passed, and the detail table says so.
- **Not yet built:** the R2 interval result, drawn as an error bar against ρ_C. It waits for the R2 result file
  format.

## 11. The Geometry scene (26 November, built)

A fifth scene, "Geometry of the controls", draws the data as it runs through the controls. It has two tabs.

**Spectrum and signals.** The data comes from `research/geometry-controls-20261126/geometry.json`, written by
`scripts/export_helike_geometry.py` from the committed cone-heat receipts. The export uses the heat-signal module's own
functions, and its plateaus and missing weights equal the heat-signal report's.
- **Ladder:** every Ritz level, g² 0.002 to 0.004 and Mq 6 to 10. The axis is broken into one zoomed band per cluster of
  levels. A line joins each level from one rung to the next.
- **Roses:** what each level is made of, at the fine and the coarse quadrature order (the S2 seam).
- **Octave circle:** the registered tones, with their offsets in cents.
- **Corridor:** log C_S(τ) for every source.

**The state.** f = ψ/ψ₀ and |ψ|² on a slice of normal coordinates, drawn over the contours of ψ₀². The inputs are the
`helike cone-levels` and `cone-export` files in `research/geometry-controls-20261126/states/`, with their CLI receipts
beside them. The Hermite basis comes from `helike-quantum` (`hermite::normalized`), which the viewer now depends on.
This is DisplayGrade.

**Finding about §4's expectation.** In the h1 × h2 plane, the chiral Helike state winds **once** (winding −1.000 at
g² 0.004, Mq 10). The export's charge compression on the pair is ∓2.000000. This plane is the plane of her one-quantum
doublet, so f ∝ z₁ ± i z₂. The charge operator acts on this plane as twice a rotation. The picture shows one turn of
phase, not the two turns §4 expected.

**Layout audit.** Every scene was captured at 1440×900 and at the minimum window size, 960×640. Long captions are wrapped
to their panel, and each plot reserves the measured height of its caption. The Controls board keeps a minimum width
and scrolls sideways below it. The Geometry scene stacks its panels when narrow. The audit uses `--size WxH`.

