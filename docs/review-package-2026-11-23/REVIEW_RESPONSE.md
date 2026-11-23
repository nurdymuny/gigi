# Response to the two reviews of 23 November 2026

Two adversarial reviews of this package came back on 23 November:
- **the first review:** a text file in the Drive package folder;
- **the second review:** a Google Doc in the same folder, with an evidence archive. It includes an independent build of
  the small sectors from the displayed quadratic form.

Neither found a counterexample to the Floor Theorem (Theorem 6.7) or to the domain theorem (Theorem 7.3 of the Floor
paper, Theorem 6.1 of *By Enough*). Both asked for corrections of scope and wording, and a few of reproducibility. This
file lists every point and what I did. The versions to post are:
- *The Floor Theorem* **3.1** (tag `floor-theorem-v3.1`);
- *By Enough, But Not Too Much* **1.1** (tag `edge-paper-v1.1`; its Lean stays at `edge-paper-v1-lean`);
- *Three Ways Across a Turn* **2.1**.

The reviewed snapshot stays tagged `floor-theorem-v3` and `edge-paper-v1`.

## The Floor Theorem (3.1)

| point | review | what I did |
| --- | --- | --- |
| Opening overstates the general-family edge (x < 0, gauge slice, one interval, P₂) | both | The abstract and the introduction now lead with what is proved. The twist is the floor of every sector on the box D. For H+GG\* an explicit Rayleigh-quotient bound rules out an all-sector floor at x > 0. The bounded interval of floor values in x is numerical, with changing competitors. P₂ gives the edge only where B₄,₀ is the competitor. |
| §7: "for y ≳ 1 the competitor is B₄,₀" too broad | both | Now: where B₄,₀ is the active competitor the computed edge is a root of P₂. Other blocks control other ranges, including 4 of 38 held-out points with y ≳ 1 (from the companion study). |
| §7: the x > 0 argument is about H+GG\*, and x = 0 is open | both | Stated: the test vector is not gauge-orthogonal, the gauge-orthogonal statement is numerical, and the argument does not settle x = 0. |
| §7: "the crossings are exact" | both | Now "the computed crossings agree with algebraic roots", with their precisions kept. |
| §7: "x below an edge" needs its window | first | Now "within the sampled window", with the lower bound referenced. |
| Meron: "one unstable direction in every sector" | both | Corrected in the discussion: one unstable direction in the left-invariant sector and none in the higher sectors examined (1 ≤ j ≤ 12), at every stretch sampled. The awkward sentence in "Resting fields" is also rewritten. |
| 168: name the metric | first | "the first nonzero scalar eigenvalue of the **round** unit Poincaré sphere", in the abstract, §2, the corollary and the discussion. |
| 56 against the 18.25 empty interval; neither is a mass gap | first | One sentence added to the floor-gap corollary. |
| Round-sphere check has no receipt | both | New `scripts/ricci_su2_round_sphere_check.py`, receipt `research/su2-left-invariant-sector-20261119/round_sphere_check.json`. It gives −2, 1, 6, 7, 10 to 10⁻¹⁴, with index one, and is cited in §8 and the code README. |
| Tag alignment | first | The paper cites `floor-theorem-v3.1`, which holds the §7 scripts and the new receipt. |
| Shorter abstract; main theorem and proof roadmap earlier | second | **Not done in 3.1.** The corrections are in; restructuring the opening is a larger rewrite I want to do myself. |

All changes are tracked in the redline (`paper_v3_draft_redline.pdf` shows v2 → 3.1).

## By Enough, But Not Too Much (1.1)

| point | review | what I did |
| --- | --- | --- |
| §8's opening says every amendment preceded every scored quantity | second | Rewritten to distinguish four kinds: after a code gate and before scored data (most); Test F Amendment 1, after three printed points (not blind on those); Test F Amendment 2, after a gate stop and before further data; and its implementation correction, after all 36 points and the scores (mechanical, nine wall points recomputed, no verdict changed). |
| Decimal slab endpoints entered as doubles | both | Appendix B.2 now states the exact endpoints. The certified boxes are those with the binary64 endpoints. Nine of the twelve upper x-endpoints lie above their decimal, so those certified boxes contain the boxes as written. Three lie below (−0.274, −0.229, −0.02, by 2.1×10⁻¹⁷, 9.1×10⁻¹⁸ and 4.2×10⁻¹⁹). For those the theorem holds on the box with the exact dyadic endpoint, which is listed, and the sliver is not certified. Theorem 5.7 points to these endpoints. D and [2.75, 10] are dyadic and unaffected. |
| Test D "not truncation-sensitive" | both | Now: "survives the two additional sectors tested, (5,0) and (6,0), which does not rule out an obstruction from a still higher sector". |
| Test B: neutral competitors' low slope is not universal | second | Added: below the threshold in most sampled cases, but 5 of 61 exceed it. |
| Abstract consistency (operator, x = 0) | both | Checked. The abstract already names H+GG\* and marks the gauge-orthogonal statement as computed. Unchanged. |

## Three Ways Across a Turn (2.1)

| point | review | what I did |
| --- | --- | --- |
| §11 infers a physical energy floor from the ticket rule | both | §11 is retitled "(speculative)" and split. The exact part is the curvature of the group of turns and Wilson's 1 − cos θ, now with the remark that it is continuous and arbitrarily small, so curvature alone gives no floor. The motivating picture is labelled speculative and states that indivisibility is an **added assumption**, not a consequence of the geometry. The Status paragraph says what the Clay problem concerns and that the ticket game does not supply it. The summary at the start and the plain-language box are adjusted to match. |
| SU(3) is not a round sphere | second | Corrected in §11 and the plain box: SU(3) is curved but has flat planes spanned by commuting directions. |
| Figure 8(b) still showed 2.5%–36% | both | Regenerated with 4.7%–36%. The English figure only; the Russian edition is not being reposted. |
| Half-angle statement needs its domain | first | Stated for 0 < θ < 2π, with the chord undefined in direction at θ = 2π, and not by itself a double-cover construction. |
| Missing raw mesh inputs for the face-angle figure | both | The meshes (about 105 MB) stay out of git. Their per-face angles are now a committed receipt (`face_angles_per_face.json`, checked against Table 2's `face_angles.json`), so the figure rebuilds from the repository alone. The README says exactly which step needs the meshes. |
| Missing receipt for the arc-in-recovery force numbers (0.090%, 0.0084%, 0.0021%) | first | **Still open.** The script `scripts/ricci_su2_force_log_vs_defect.py` exists, but its output was not saved, and rerunning it needs the meshes. Disclosed here; to be fixed when I next have the meshes mounted. |

## The package

| point | review | what I did |
| --- | --- | --- |
| Source zips do not build standalone (`../dgstyle.tex` missing) | second | The new zips include `docs/papers/dgstyle.tex` and a `BUILD.md` with clean and redline commands. |
| Some scripts print a failure but exit 0 | second | New helper `scripts/_receipt_status.py`. Eleven proof and certificate scripts now exit 1 when their receipt records a failed check (any pass flag false, or a FAIL / MISMATCH status). It is not applied to the scripts of pre-registered predictions, where a failed prediction is a result, not an error. |
| One authoritative manifest | first | `DERIVATIONS_README.md` is marked authoritative for these versions; its historical fix notes are labelled historical. |
| Research summary overstated the floor region; the determinant omitted the s² scaling | first | Both corrected in `RESEARCH_SUMMARY.md`. |
| "her" and "price" in theorem statements and application-facing summaries | first | Theorem statements already use "the twist" and "eigenvalue". The narrative voice in the body is kept on purpose. |
| Leaf certificates and a standalone verifier for the 64 CPU-hour run | second | **Not done.** Disclosed as a limit: the receipt keeps counts and minimum margins, not every leaf. |
| An external specialist read of the main proof | second | **Agreed, and not something I can do from here.** |
