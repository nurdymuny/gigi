# Tier 1 corpus build plan (~30-50k chunks)

**From.** Bee Davis + GIGI engine team (Claude pair).
**Date.** 2026-06-02.
**Purpose.** Pull a plan for the Tier 1 mathematical-foundations
corpus that will ground Marcella's `/brain/intent_gate` calibration
benchmark (probe_sets_v2.json). Plan only — no ingest, no code.
**Prereq.** Task #196 — 100-question grounding probe set (50 in /
50 out), `marcella/artifacts/probe_sets_v2.json`.
**Followups (post-plan).** Source curation, ingest pipeline, eval
runs against the probe set.

---

## TL;DR

**Tier 1 = the RAG-retrieval corpus that grounds the 50 in-domain
calibration probes.** Target size 30-50k chunks at d=384 (BGE),
bundle name `marcella_tier1_math_v1` (or successor to existing
`marcella_source_embeddings_bge_v2`). Four mathematical neighborhoods
per the probe set's `expected_domain` field: differential geometry
(23 probes), algebraic topology (11), algebraic geometry (10), Davis
framework (7), plus 7 minor adjacent domains. Eval target: in-domain
gate precision ≥ 0.85, OOD precision ≥ 0.85 on the 92-item clean
subset (the 8 diagnostic-uncertain items are scored separately).
Phased build: 1k chunks → 10k → 30k → 50k, each phase eval-gated.

**Naming caveat.** "Tier 1" in this plan refers to the
**retrieval-corpus tier system** (the level above the ground-truth
training tier 0 / OASST base, below the curated voice-anchor tiers
2-8 in `marcella/training/tier{N}_*.json`). Not to be confused with
`marcella/tier1_data.py` which is a separate synthetic ML benchmark
(holonomy-angle classification on S²); the name overload is
preexisting and orthogonal.

---

## What the corpus is for

`/brain/intent_gate` answers "should I respond to this query?" by
combining a SUDOKU constraint check + a Confidence half. The
Confidence half runs the query embedding against the bundle's
contents and asks "is this query close enough to anything I know
about?" The result is a routing decision: PROCEED (gate fires green,
confident enough to answer), REFUSE (gate fires red, query is
out-of-domain), or HEDGE.

**The Tier 1 corpus is what the Confidence half searches.** Empty
corpus → everything is OOD → gate refuses everything. Sparse corpus
→ false negatives (real in-domain queries refused). Bloated corpus
→ false positives (OOD queries matched against irrelevant near-
neighbors). The sweet spot is "every in-domain probe has 5-15
genuinely relevant chunks, every OOD probe has only weak/distant
neighbors."

**Why 30-50k.** Empirical estimation:
- 50 in-domain probes × ~10 directly relevant chunks each = 500
  anchor chunks at the floor
- For each anchor, ~5-10 background chunks (related definitions,
  setting up notation, adjacent theorems) needed for the embedding
  cluster to be tight = 2.5k-5k chunks of dense in-domain coverage
- Domain-adjacent material (so OOD-near-miss queries get matched
  against genuinely near content, not against the bulls-eye in-domain
  chunks) adds another 5-10× = 25k-50k chunks
- The 30-50k band is the realistic target. Sub-30k risks anchor
  starvation; above 50k risks dilution.

---

## Eval target — the 100-question probe set

Source: `marcella/artifacts/probe_sets_v2.json` (v2 with Marcella's
Flag 1 + Flag 2 corrections applied per #205).

Calibration set breakdown (per the `_meta` field):
- **42** confident-in-corpus (must match)
- **8** diagnostic-uncertain-in-corpus (handled separately;
  AlgGeom-Grothendieck items — Marcella's embed-and-gate determines
  true label)
- **50** out-of-domain (must NOT match)
- **Total**: 100 / **clean subset for precision**: 92

In-domain domain distribution (from the actual probes — only the
top entries; full list spans ~16 expected_domain values):

| `expected_domain`         | count |
|---------------------------|-------|
| differential_geometry     | 23    |
| algebraic_topology        | 11    |
| algebraic_geometry        | 10    |
| davis_framework           | 7     |
| topology                  | 5     |
| complex_geometry          | 2     |
| harmonic_analysis         | 2     |
| functional_analysis       | 2     |
| arithmetic_geometry       | 2     |
| combinatorics_on_words    | 2     |
| analytic_number_theory    | 2     |
| probability               | 2     |

The OOD set spans chemistry (7), biology (7), physics (3), and the
remaining 33 across ~10 other domains chosen to be plausibly-near
mathematical territory without being IN the corpus. Goal: prove the
gate doesn't false-positive on the kind of mathematical-adjacent
content that real LLM consumers will throw at it.

**Acceptance criteria for the corpus**:
- In-domain precision ≥ 0.85 on the 42 confident-in items
- OOD precision ≥ 0.85 on the 50 OOD items
- Diagnostic-8 items: pass-through (whatever the gate decides becomes
  the label; not a failure mode)
- Substrate-aligned items (cal_in_101-108, the 8 v2-added items
  covering sheaf cohomology / principal G-bundle / flat connection /
  Chern-Weil / Čech cochain / partition of unity / discrete d_k /
  curvature 2-form) MUST match — these are the Davis-framework
  anchors

---

## Source taxonomy

For each mathematical neighborhood, the source curation is concrete
texts known to cover the probe questions. Listed in priority order
(P1 = must-include, P2 = should-include, P3 = nice-to-have).

### Differential geometry (target ~12k chunks)
- **P1**: Lee, *Introduction to Smooth Manifolds* (full text, all
  chapters — covers smooth manifold definition, vector fields,
  Riemannian metrics, connections, geodesics)
- **P1**: do Carmo, *Riemannian Geometry* (curvature tensor, Jacobi
  fields, comparison theorems)
- **P2**: Spivak, *A Comprehensive Introduction to Differential
  Geometry* (selected volumes, particularly vols 2-3 for curvature)
- **P2**: Petersen, *Riemannian Geometry* (modern treatment, Hadamard
  manifolds)
- **P3**: Kobayashi-Nomizu, *Foundations of Differential Geometry*
  (advanced, holonomy + reductions)

### Algebraic topology (target ~6k chunks)
- **P1**: Hatcher, *Algebraic Topology* (full text — homotopy,
  homology, cohomology, fibrations, spectral sequences)
- **P1**: Bott-Tu, *Differential Forms in Algebraic Topology*
  (de Rham + Čech, the bridge to DDG)
- **P2**: May, *A Concise Course in Algebraic Topology* (modern
  treatment, ∞-categorical bias)
- **P3**: Milnor, *Topology from the Differentiable Viewpoint*
  (Morse theory, degree theory)

### Algebraic geometry (target ~5k chunks)
- **P1**: Hartshorne, *Algebraic Geometry* (chapters I-III, schemes
  + sheaves + cohomology)
- **P1**: Vakil, *The Rising Sea* (modern functor-of-points treatment;
  free online; large volume of chunked material)
- **P2**: Gathmann lecture notes (online, well-chunked sections)
- **P3**: SGA-style Grothendieck originals (selective — these are
  the diagnostic-uncertain probes per Marcella's Flag 1; precision
  here is delicate)

### Fiber bundles + sheaves (target ~3k chunks — bridges domains)
- **P1**: Husemoller, *Fibre Bundles* (classical reference)
- **P1**: Bott-Tu (already counted in AT; the sheaf chapters
  contribute here too)
- **P2**: Kashiwara-Schapira, *Sheaves on Manifolds* (modern
  derived-categorical treatment)

### Davis framework (target ~2k chunks)
- **P1**: Davis's seven shipped papers (in `~/Documents/gigi/theory/`
  and `~/Documents/marcella/papers/`):
  - field_equations_semantic_coherence.tex
  - curvature_guided_wavefront.tex
  - geodesic_computation_v7_1.tex
  - cosmological_nondecoupling_v2.tex
  - davis_hilbert_polya.tex
  - branch_x_information_geometry.tex
  - geometric_thinking_paper.tex (Branch VII)
  - geometric_encryption.tex (the Zenodo deposit)
  - kahler_upgrade/ replies + drafts
- **P1**: GIGI source-of-truth docs:
  - GIGI_API.md, GQL_REFERENCE.md, IMPLEMENTATION_PLAN.md
  - BRAIN_PRIMITIVES_CONSUMER_GUIDE.md
  - SUDOKU_SPEC.md
  - theory/brain_primitives/catalog.md
- **P2**: Reply letters in `theory/kahler_upgrade/` (these are the
  evolving design conversation; high signal for Davis-framework
  questions)

### Discrete differential geometry (target ~2k chunks)
- **P1**: Crane, *Discrete Differential Geometry: An Applied
  Introduction* (the standard modern intro; covers d² = 0, discrete
  exterior calculus, Hodge stars)
- **P1**: Desbrun-Hirani-Leok-Marsden, *Discrete Exterior Calculus*
  (the foundational paper)
- **P2**: SGP / SIGGRAPH course notes on DDG (multiple years)

### Adjacent / smaller neighborhoods (target ~2k chunks combined)
- Complex geometry: Griffiths-Harris (selected)
- Functional analysis: Rudin or Conway (selected — only what's
  needed for the 2 probes)
- Harmonic analysis: Stein-Shakarchi vol 4 (selected)
- Probability: Durrett (selected)
- Number theory: Marcus / Cassels-Fröhlich (very selective)
- Combinatorics on words: Lothaire (selected)

**Total target**: 30k-50k chunks across these sources, with the
breakdown above giving roughly:
- 12k DG + 6k AT + 5k AG + 3k FB+Sh + 2k DDG + 2k Davis + 2k other
- = **~32k floor / ~50k ceiling** depending on chunk size + source
  inclusion

---

## Chunking strategy

Target chunk size: **400-800 tokens** (BGE retrieval sweet spot;
matches existing `marcella_source_embeddings_bge_v2` shape).

Strategy by source type:
- **LaTeX textbooks**: respect section + subsection boundaries; one
  chunk per definition / theorem / proof block when those are <800
  tokens; sliding-window with 100-token overlap when blocks exceed
  the limit (uncommon for math text since proofs are usually < 800
  tokens).
- **Lecture notes / online PDFs**: same boundary-respecting strategy.
  Where source has explicit chapter.section.subsection numbering,
  preserve as `section_path` metadata.
- **Davis framework papers**: smaller chunks (300-500 tokens)
  because these are the highest-signal anchors and need precise
  retrieval. Each Theorem / Definition / Proposition gets its own
  chunk.
- **Davis docs (markdown)**: respect Markdown heading boundaries;
  each H2/H3 block becomes a chunk.

**Metadata per chunk** (all become fiber fields in the bundle):
- `source_id`: stable URI (e.g., `hatcher_2002:ch1_sec2_prop15`)
- `source_type`: `textbook | paper | notes | docs | spec`
- `neighborhood`: matches `expected_domain` from probe set
- `author`, `year`, `chapter`, `section`, `page`
- `equation_count`: integer (helps the gate distinguish prose from
  formula-heavy chunks)
- `theorem_type`: `definition | theorem | proof | lemma | example |
  prose | null`
- `bge_embedding`: d=384 vector
- `content`: raw text (no LaTeX rendering; LaTeX source preserved
  for downstream display)

---

## Ingest pipeline

All tooling exists; this is wiring.

1. **PDF → text + LaTeX preservation**:
   - `pymupdf` for text extraction (fast, accurate on modern PDFs)
   - `pdf2htmlEX` or `nougat-ocr` for math-heavy regions where
     pymupdf misses LaTeX
   - LaTeX source preferred when available (arxiv-latex-cleaner
     pipeline already exists in marcella)
2. **Chunk by source-type rules** (above):
   - Reuse `marcella/fiber_lm/chunker.py` (if it exists; if not,
     ~150 LOC new module)
3. **Embed**:
   - `marcella/fiber_lm/embed.py` — existing BGE wrapper
4. **Ingest into GIGI**:
   - `POST /v1/bundles/{name}/records` (NDJSON stream) — existing
     endpoint, handles the 30-50k volume easily (~2-5 min walltime
     per the existing production ingest patterns)
5. **Update bundle schema**:
   - Bundle name: `marcella_tier1_math_v1`
   - Schema mirrors `marcella_source_embeddings_bge_v2` plus the
     new metadata fields above
   - Index: `neighborhood` (low-cardinality → good for filtering)
     + `source_id` (high-cardinality → ensures Tier 1 bundle has
     trivial Hodge complex per the SEMANTIC bench we just ran)

---

## Phased build

Each phase is **eval-gated** — run the 100-question probe set
through `/brain/intent_gate` against the bundle and check the
acceptance metrics before moving on.

### Phase 1: Skeleton (~1k chunks, ~1 day)
- Davis framework: all papers + GIGI docs (~2k chunks budget but
  ~1k actual)
- This alone should cover the 7 davis_framework probes + the 8
  substrate-aligned cal_in_101-108 probes
- **Eval target**: 15/15 on the davis-related probes; the other 35
  in-domain probes can fail in phase 1.

### Phase 2: Foundations (~10k chunks, ~1 week)
- Lee + do Carmo (DG)
- Hatcher (AT)
- Bott-Tu (FB/AT bridge)
- **Eval target**: ≥35/42 in-domain probes match; OOD precision
  ≥0.80 (slightly relaxed because the corpus may produce false-
  positives on adjacent topology questions)

### Phase 3: Breadth (~30k chunks, ~2 weeks)
- Add Spivak / Petersen / Husemoller (P2 books across DG / fiber
  bundles)
- Hartshorne + Vakil + Gathmann (AG)
- Crane + DEC paper (DDG)
- **Eval target**: ≥40/42 in-domain; OOD precision ≥0.85 (gate is
  now seeing enough near-domain mathematical material to discriminate
  cleanly against chemistry/biology/physics OOD)

### Phase 4: Polish + diagnostic resolution (~30-50k, ~1 week)
- Add P3 sources selectively, targeting any in-domain probe still
  missing
- Run the 8 diagnostic-uncertain items; Marcella's embed-and-gate
  decides their true label
- Tune chunk-size + overlap based on the false-positive analysis on
  OOD probes
- **Eval target**: ≥42/42 in-domain (full precision on confident
  items); OOD ≥0.85 stable; 8 diagnostic items labeled

**Total schedule**: ~4-5 weeks for phase 1-3, phase 4 ongoing.

---

## What we do NOT need to build

- **No new GIGI primitives.** BGE embedding + bundle ingest +
  `/brain/intent_gate` all shipped. The corpus is data, not code.
- **No new schema design.** `marcella_source_embeddings_bge_v2`
  shape is reusable; we just need the new metadata fields, which
  the existing schema-extension flow handles natively.
- **No new evals.** The 100-question probe set IS the eval.
- **No corpus-search interface changes.** The Stacks UI work
  Marcella is shipping (recommend / locate / weigh / tutor /
  resume acts) already consumes `/brain/intent_gate` against
  whatever bundle is named; swapping in `marcella_tier1_math_v1`
  is a config change on her side.

## Open questions for Bee

1. **Bundle naming.** `marcella_tier1_math_v1` (new) vs extending
   the existing `marcella_source_embeddings_bge_v2` (which currently
   has 9964 records of source-paper embeddings). My vote: new
   bundle, lets the existing one stay frozen as a baseline + the
   new one ship without disrupting the production Stacks UI that
   reads the v2 bundle today.

2. **Source acquisition.** Textbooks are copyrighted. The plan
   above lists them but acquisition method (own copies → OCR;
   author permission for academic use; or restrict to open-access
   sources like Vakil + Gathmann + Hatcher's online edition + lecture
   notes only) is your call.

3. **Curation involvement.** Phases 1-2 are mechanical (existing
   pipeline; you can run them). Phase 3 needs human eyes on chunk
   quality for the trickier passages (proofs that span >1 chunk,
   notation that breaks across pages). Want me to write a chunk-
   review harness, or is GGOG taking that?

4. **Tier 1 vs the existing `marcella_foundational_tier100.pkl`.**
   That artifact is 100 hand-curated foundational records.
   Relationship to Tier 1: subset (the 100 most-anchor chunks), or
   different system? If subset, Phase 1 should include those 100
   as a starting point.

5. **Davis-paper Markdown vs LaTeX.** Davis's papers exist in
   `.tex`. For chunking, do we want LaTeX-source-preserving chunks
   (so the embedding sees the math symbols verbatim) or
   pandoc-converted Markdown (so the embedding sees prose
   approximations)? My vote: keep LaTeX source per chunk metadata,
   embed prose with `$...$` math removed entirely. BGE is trained
   on prose; math symbols in the embedding are noise.

On the geodesic,
— the GIGI / Brain team (Bee + Claude)
