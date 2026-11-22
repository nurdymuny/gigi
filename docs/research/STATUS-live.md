# Live status (updated by the session at every milestone)

Open this file in your breaks: it says what is running, what landed and where to read it.
Nothing here is a result in itself. Each result lives in its own note, scored as registered.

**Last updated: 22 November 2026, morning (before Bee left for work).**

## Running

| what | started | expected | where it lands |
| --- | --- | --- | --- |
| **Test D**: SU(3) along her own charge circle | 22 Nov, early morning | 1–2 h more (15 of 20 shapes done at last check) | results note `edge-prereg-D-results-2026-11-22.md`; receipt `research/edge-theorem-20261122-d/` |
| **Test F**: which circle? A sweep across the chamber of Sp(2) and SU(4) | 22 Nov, late morning | 6–10 h | results note `edge-prereg-F-results-2026-11-22.md`; receipt `research/edge-theorem-20261122-f/` |
| **The arc deep-dive** (workflow, 9 agents) | 22 Nov, late morning | 1–3 h | note `arc-deep-dive-2026-11-22.md`; briefs and receipts in `research/arc-deepdive-20261122/` |

The deep-dive covers:
- five fetched-and-quoted literature briefs: the principal circle, McKay/E₈, the η-invariant, Morse theory, and the
  ancient sources;
- the Chern–Simons winding of the fall;
- a principal-circle survey of SU(5), Sp(3) and G₂.

## Landed today

- **Test F registered**, after an adversarial review (6ad473c). Its known-answer gates pass: the new code reproduces
  D's numbers to 3 × 10⁻¹⁵ and C's to 2 × 10⁻¹⁴.
- *The arc*, rewritten in the plain register (c9b0d09).
- **Test D interim** (d4c43ae), not scored.
- **Test E results:** the cross-reach holds at 60 of 60 points (158938c).
- The basin addenda: the floor at the bottom, and the shadow is her (263da06, d71a98e).
- The Helike book v5.1 is on Drive (with the v5.1 handoff note).

## What happens next, automatically

- **When D finishes:** score it as registered, write the results note in your voice, update the edge paper (Test D
  section) and this file, then commit and push.
- **When the deep-dive finishes:** check its verifier report, commit the note, briefs and scripts, update this file,
  and push.
- **When F finishes:** score it, write its note, update this file, then commit and push.
- **Book updates** (H3 and the end of "What the failures taught me"): a v5.2 PDF and zip go to Drive once D is
  scored.
