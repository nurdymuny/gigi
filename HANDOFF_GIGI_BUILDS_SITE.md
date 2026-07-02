# HANDOFF — GIGI Builds companion site + tetmesh chapter

**From:** remote (cloud) Claude Code session, 2026-07-01
**To:** local desktop Claude Code session on Bee's machine
**Branch with all work:** `claude/spec-review-generalize-13d0c2` (pushed to `nurdymuny/gigi`)

## Context in one paragraph

Bee is responding to Connor Castillo (UC Davis) who asked about tetrahedral-mesh
classification. Strategy: the tetmesh spec is framed as a chapter of her book
*GIGI Builds* that didn't fit in print ("The Mesh That Audits Itself"), published
on a new companion site, so the reply to Connor is: "you're in luck — it was a
GIGI chapter, here's the page with a live demo; questions → contact me."

## What is DONE (all on the branch above)

1. **`GIGI_TETMESH_SPEC_v0.6.md`** — the spec, generalized (no Connor references),
   framed as a *GIGI Builds* chapter draft, corrected against the real engine
   (real GQL, flat fiber schema, `unit_cube_384`), with a validation-receipt
   appendix. Every number reproduced by the harness.
2. **`examples/tetmesh_fiber_harness.py`** — generates the Kuhn mesh, validates
   all the math (PSD rank-3 Gram, facet closure, one classifier cell mod S4,
   Regge deficits ~0, Maubach period-3 class cycle), loads 5,760 fibers +
   5,376 refine events into a live gigi-stream, runs the audit queries.
3. **Engine fixes** in `src/aggregation.rs` (`group_by_measures`,
   `integrate_measures`), `src/parser.rs`, `src/bin/gigi_stream.rs`:
   INTEGRATE multi-measure aliasing, `count(*)`/text-field count returning
   empty, global no-OVER INTEGRATE returning empty. Unit tests added; verified
   live.
4. **`examples/tetmesh_visual.html`** — self-contained three.js demo (three.js
   r160 + OrbitControls bundled inline via esbuild; works offline by
   double-click). Kuhn cube under adaptive longest-edge bisection, colored by
   classifier cell, level scrubber, explode/cutaway, tooltips, telemetry panel.
5. **`site/gigi-builds/`** — the companion site:
   - `index.html` — self-contained static page (no build step): hero with CSS
     cover mock, receipts strip, full 7-part/18-chapter catalog, the missing
     chapter feature, database promo + quickstart, author section, footer.
   - `the-mesh-that-audits-itself.html` — copy of the visual, linked from the
     page.
   Built from the real book (`gigi_builds_main.pdf`, found in Google Drive):
   real TOC, ISBN 9798181715820, "1373 passed" receipt, preface quotes.

## What is PENDING (needs Bee's local files — do this in the local session)

1. **Real cover art.** Cover screenshot lives at:
   `C:\Users\nurdm\OneDrive\Pictures\Screenshots 1\Screenshot 2026-06-15 135215.png`
   In `site/gigi-builds/index.html`, replace the inner markup of `.cover-art`
   with `<img src="cover.png" alt="GIGI Builds book cover">` (comment in the
   CSS marks the spot; keep the tilt/shadow frame). Bee likes the cover's
   "photo embedded with depth" treatment — preserve that feel.
2. **Author photos.** Bee + Bird photos are in `C:\Users\nurdm\Downloads\`
   (FoxyAI_Image_*.jpg files + Image_00694ad9-*.jpg). Pick 3 (Bee's choice),
   copy into `site/gigi-builds/` as `bee-1.jpg`, `bee-2.jpg`, `bee-3.jpg`,
   and swap the placeholder SVGs inside the three `.pol .ph` divs for
   `<img>` tags (comments mark the spots). Resize/crop square-ish, keep files
   reasonably small (<400KB each).
3. **"Get the book" link.** Currently `#book` placeholder — set the real
   purchase URL when Bee provides it.
4. **Deploy.** Copy `site/gigi-builds/` (both HTML files + images) into the
   website repo: `C:\Users\nurdm\OneDrive\Documents\math-website-main\gigi\`
   as `gigi-builds/` so it serves at `davisgeometric.com/gigi/gigi-builds`.
   The main gigi site's style is being redone — do NOT restyle this page to
   match it; this page's look is intentional (warm paper + night sky).
5. **Optional polish ideas** (not started): swap the missing-chapter preview
   SVG for a real screenshot of the demo (`mesh-preview.png` slot is noted in
   a comment); OG/social meta tags + og-image; favicon.

## UPDATE 2026-07-01 (later): exercise pages are DONE

All 126 exercises (18 chapters x 7) are published at
`site/gigi-builds/exercises/` — a hub (`index.html`) plus `ch01.html` ..
`ch18.html`, generated from the book's LaTeX source (Bee shared the
`gigi_builds` source folder via Google Drive) by
`scripts/build_exercise_pages.py`. The main page links them (nav item,
"126 exercises are online" in the Builds lede, and every chapter card's
"exercises →" tag). Faithful conversion: Build/Receipt/Bonus segments,
KaTeX (CDN) for inline math, code spans keep literal `--` so commands
copy-paste correctly, Čech/verb/ref handled. The book .tex sources are
NOT committed — only the generated exercise HTML is. To regenerate:
`python3 scripts/build_exercise_pages.py <chapters_dir> site/gigi-builds/exercises`.
Deploy step 4 above now includes the `exercises/` subfolder.

## UPDATE 2026-07-02: exercise pages are now INTERACTIVE

Per Bee's original ask ("interactive like the mesh demo"), the exercise
pages now carry a working layer, all verified headless end-to-end:
- **Workbook**: every one of the 126 exercises has a done-checkbox and a
  paste-your-receipt field, persisted in localStorage (`gb-ex` key);
  chapter pages show an n/7 progress bar and the hub shows per-chapter
  badges. Done cards get an aqua edge.
- **Live GQL console** on every chapter page: endpoint field (defaults
  to the public instance, which serves `access-control-allow-origin: *`;
  or a local engine started with `GIGI_CORS_ORIGIN=*`), quick-fill
  chips, POST to `/v1/gql`, pretty-printed response.
- **Click-to-copy** on every code chip, with a toast.
- **Chapter 1 playground**: the curvature odometer — the engine's
  Welford + per-record kappa math ported to JS. Reproduces the book's
  BLD-CH1-WORKED-KAPPA receipts in-browser: kappa(s151) = 4.7652 vs book
  4.7653, typical-reading kappa = 0.03177 vs book 0.031728; targets
  flip to green checkmarks when hit.
- Pattern for more playgrounds: add an entry to `PLAYGROUNDS` in
  `scripts/build_exercise_pages.py` (keyed by chapter number) and
  regenerate. Natural next candidates: ch3 (GIGI hash / birthday-bound
  collision demo, port `src/hash.rs`), ch9 (spectral-gap toy graph).

## UPDATE 2026-07-02 (later): every chapter page has a visual instrument

Bee asked for visualizations on all the exercise pages, each different,
building on one another *within each Part* (resetting at Part boundaries).

- All 18 live in ONE new file: `site/gigi-builds/exercises/viz.js`.
  Each chapter page carries exactly one added line, just before `</body>`:
  `<script defer src="viz.js" data-ch="N"></script>`. That's the entire
  footprint in the HTML — the panel builds its own DOM, injects its own
  CSS (`.gbviz*` classes only), and mounts itself before the first
  `article.ex`. Hand-edit the pages freely; nothing else references it.
- Do NOT regenerate pages from `build_exercise_pages.py` without
  re-adding the script line (the generator doesn't know about it yet;
  porting it into the generator is a nice-to-have).
- Part arcs: I readings→stalks→addresses (ch1–3), II one record through
  the machine (ch4–7), III signals→spectrum→wire (ch8–10), IV
  holonomy→torsion (ch11–12), V shard equality→honest error bars
  (ch13–14), VI gauge→verifier→ratchet (ch15–17), VII keeper's console
  (ch18). Each panel says "Builds on the Chapter N instrument" when it
  continues an arc.
- All math is real where the engine has math: Welford/κ (R=100, stats
  before the record), Jacobi eigensolver for λ₁ + Fiedler coloring,
  blocked errors at 2τ (the WITH JACKKNIFE story, φ=0.8 → τ=4.5),
  gauge-invariant distance tuples at the 1e-10 grain. Deterministic
  seeded PRNG everywhere; no Date.now/Math.random.
- Only network touch in all 18: ch18's click-gated "Ping the real
  engine" button → POST /v1/public/gql `HEALTH tetmesh_demo;` (response
  is a flat JSON object; the panel prints its keys).
- Headless-verified (Playwright, network blocked, all 18: panel mounts,
  canvas paints, first button clicks clean, zero console errors).

## Facts the next session will want

- Book: *GIGI Builds — The Fiber-Bundle Database: A Working Engineer's Guide
  from Schema to Shard*, Bee Rosa Davis, 2026, ISBN 9798181715820, PolyForm
  Noncommercial 1.0.0. 7 parts, 18 chapters, appendices A–D.
- The site page and demo are fully self-contained; no npm/build anywhere.
- Public GIGI instance: https://gigi-stream.fly.dev (health endpoint works).
- Contact on the page: bee_davis@alumni.brown.edu.
- Validation receipts and engine-caveat history: appendix of
  `GIGI_TETMESH_SPEC_v0.6.md`.
- A PR has NOT been opened for the branch; Bee hasn't asked for one.
