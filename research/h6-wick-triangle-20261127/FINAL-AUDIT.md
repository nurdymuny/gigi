# Final bounded audit

27 November 2026. Incoming source: `36eb3b4f9e240f2fa3d226dc11f2111dcf3a8c6d`.

- Math gate: **10 checks pass** (`release-02`). Four broken engine variants
  produce assertion failures. Three broken Lean variants produce proof errors.
- Formal boundary: five matrix declarations accepted; only the three allowed
  foundational Mathlib axioms appear. The Maxwell/Wick dictionary is not formalized
  end to end. The proof note distinguishes written and formal verification.
- Publication: **11 preservation/format checks, 26 focused review checks and
  9 doublet/artifact checks pass**. These are the three `final-*` receipts.
- All LF source hashes in those four release receipts were recomputed from the
  delivered worktree and matched. All PDF hashes recorded by the final document
  checks matched the delivered `statement.pdf`.
- An attempted reuse of `release-02` returned FileExistsError and a nonzero exit
  code. The existing receipt's SHA256 was unchanged. The rejection is retained in
  `existing-output-refusal.log`; it was not counted as another passing math run.
- Final PDF: 27 pages, stable references, no LaTeX warnings or overfull/underfull
  boxes in `pdf-build-04.log`. Rendered pages 19-27 were visually checked. Pages
  20-21 have the mathematical correction and retain CONDITIONAL/CONJECTURE labels;
  page 23 has provenance; pages 24-27 have the intact tables and references.
  Pages 20-21 were byte-identical as rendered before and after the availability
  prose was condensed. There is no nearly empty overflow page in the final PDF.
- `git diff --check` passed, excluding preserved compiler logs.

No GIGI core changed. No full engine suite, GPU job, simulation, or large spectral
matrix calculation was run or claimed. One documented finite prediction was
refuted; its documented four-point alternative was verified. The seven top-level
hypotheses remain open.
