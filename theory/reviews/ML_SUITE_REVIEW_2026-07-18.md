# ML Suite Review — batch `035ebad..4af484f`

**Scope:** the 8 hand-rolled, pure-Rust ML endpoints added to `src/bin/gigi_stream.rs`
(+3008 insertions / 0 deletions). Review-only: read the actual math, not the commit messages.
No source was modified, nothing committed.

**Batch:** 23 commits, all authored by Bee Davis. Merged to `origin/main` at merge commit `4af484f`.

---

## 1. Verdict

**Ship-with-one-fix.** The ML suite is, on the whole, real and correct work: eight endpoints of
hand-rolled linear algebra (Gaussian elimination, Gauss-Jordan inverse+logdet, power-iteration PCA,
GMM EM with log-sum-exp, k-means++, split-conformal GP intervals, Funk-SVD, a pooled two-sample-t
changepoint detector) with **no `ndarray`/`linfa` dependency**, each backed by a passing
ground-truth cargo test. An independent read of the math against known properties found **no
SHIPPED-BUT-WRONG defect** — no concrete input was shown to produce a wrong numeric result or a
panic. The integration surface is clean: strictly additive, full-feature build compiles, the 927-test
lib suite and the named Halcyon/holonomy fences stay green, and no locked gauge/spectral/curvature
math was touched. The one thing standing between this and "production-ready as-is" is a **test-coverage
gap**: `/scan/fit` — the supervised logistic-regression lens-weight learner — is the single endpoint
with zero cargo-test coverage, and its algorithm is inlined in the async handler in a way that makes
it structurally hard to test. It ships a learner with published headline metrics and no regression
gate. That is a should-fix, not a blocker: the code reads correctly and mirrors its Python reference;
it is simply unguarded.

---

## 2. Findings

### CONFIRMED

**[SHOULD-FIX] `/scan/fit` supervised learner has zero cargo-test coverage and is structurally untestable**
- **File:** `src/bin/gigi_stream.rs:9887` (handler `bundle_scan_fit`; learner closures `train` 9937–9952, `ap` 9954–9965)
- **Failing input → wrong output:** No runtime crash. The defect is a missing gate. Mutate the
  class-weight sign/term (`wpos`, line 9946), the `lr/m` scaling (9947–9948), the `z`-clamp (9945),
  or the average-precision integral step `if r > pr` (9962), and the **entire cargo-test suite still
  passes green**. A future refactor that breaks `bundle_scan_fit`'s wiring — e.g. a change to
  `scan_compute_lenses`' returned norm/id alignment (9927) — ships silently, along with wrong
  `pr_auc_supervised_heldout` vs `pr_auc_unsupervised` numbers.
- **Why it's uniquely exposed:** the three siblings it's modeled on — `cluster_records` (10156),
  `predict_field` (10650), `factorize_matrix` (11180) — are *free functions*, which is exactly why
  each has direct tests (20049–20507). `/scan/fit`'s learner lives as two closures inside the async
  handler, unreachable from a `#[test]`. `ml_all_endpoints_regression_smoke` (20470) drives every
  other endpoint but silently omits `/scan/fit` (and `/factorize`, though factorize is covered
  separately). `scripts/scan_fit.py` only `sys.exit()`s on missing/single-class labels — it asserts
  nothing.
- **Suggested fix:** extract the logistic-GD + average-precision core to a free `fn scan_fit_weights(...)`
  (mirroring `cluster_records`/`predict_field`), then add one ground-truth test asserting learned
  weights recover a planted linear separator and that supervised PR-AUC ≥ the unsupervised baseline
  on a synthetic labeled fixture. Add the fit path to `ml_all_endpoints_regression_smoke`.

### PLAUSIBLE / UNVERIFIED (note-level — honestly labeled, no failing input)

These are diagnostic-accuracy or heuristic nits. None is a crash or a wrong-output-on-concrete-input;
each is surfaced for completeness, not as a defect to gate the ship.

1. **Spectral diagnostic string can misreport component count** — `src/bin/gigi_stream.rs:10485`.
   The `notes` string hard-codes `"(1 connected component)"`, but the else/spectral-cut branch runs
   for **any** `ncomp < k` (e.g. 2 components with `k=3`). Labels and eigenvalues are correct; only
   the human-readable note lies. Fix: interpolate the real `ncomp`.

2. **Funk-SVD cold-start on test-only entities** — `src/bin/gigi_stream.rs:11276`. User/item indices
   are assigned from *all* observed rows, but the CV model trains only on the 80% split, so an entity
   appearing solely in the 20% held-out split keeps random-init factors. Indices stay in bounds (no
   crash); held-out RMSE is only ever biased **upward**, so the baseline comparison stays valid. This
   is honest cold-start behavior. Optional fix: skip test rows whose user/item is unseen in train, or
   fall back to `mu + b_u + b_i`.

3. **Changepoint time-axis auto-detect matches loose substrings** — `src/bin/gigi_stream.rs:11387`.
   Name-substring matching (`order`/`seq`/`index`/`step`) can pick a value column like `order_amount`
   as the time axis and sort the series by transaction size. Only fires when no explicit `time` param
   is passed; caller can override. Acknowledged heuristic. Optional fix: prefer monotonic/parseable
   columns, or require an explicit `time` when detection is ambiguous.

4. **GP "guaranteed coverage" comment overstates** — `src/bin/gigi_stream.rs:10732`. The code note
   says coverage is "guaranteed (exchangeability)", but the conformity scores come from k-fold CV
   held-out predictions (cross-conformal), so the finite-sample guarantee is heuristic rather than a
   clean single split-conformal holdout. Empirically calibrated and tested (diabetes coverage 0.919).
   Comment-only.

5. **"fixes high-dim overfit" claim (commit 32c7ad2) has no gate and unflattering real-data evidence**
   — `src/bin/gigi_stream.rs:10599`. The ridge is genuinely present and correct (added to slope
   diagonals only, intercept unpenalized). But the only `local_linear` test uses 2 features, and on
   the one real high-dim dataset in the suite (diabetes, 10 features, `scripts/sweep_results.json:160`)
   `local_linear` scores R²=0.366, *losing* to the flat KNN baseline at 0.449. Not a bug; the framing
   just outruns the evidence and nothing gates the high-dim regime.

---

## 3. Structural note

**~3008 lines of ML now live inline in `src/bin/gigi_stream.rs`**, a file that was already very large.
The contiguous handler+helper+struct region is `9286–11553` (~2270 lines); inline `#[cfg(test)]`
tests add ~730 lines at ~19790–20520; 8 router lines; plus 5 Python/JSON script files under
`scripts/`. No new `src/` module was created — everything is hand-rolled on `std + serde + axum`
reading through the existing `gigi::engine::Engine` API.

- **Module-extraction candidate.** The ML block is self-contained (shared helpers `scan_solve`,
  `mat_inv_logdet`, `kmeans_lloyd`, `local_linear_at/scaled`, `scan_trigrams/jaccard`,
  `scan_compute_lenses` are all internal to it) and touches no feature-gated symbols. It is a clean
  candidate to lift into `src/ml/` (e.g. `ml/scan.rs`, `ml/cluster.rs`, `ml/predict.rs`,
  `ml/reduce.rs`, `ml/factorize.rs`, `ml/changepoints.rs`) with the handlers left thin in the binary.
  Extraction would also fix the `/scan/fit` testability problem as a side effect.
- **Test-coverage picture.** The ground-truth gates are the **inline cargo tests** — ~24 ML test fns
  with real synthetic-ground-truth assertions (blobs→distinct clusters, calibrated 90% conformal
  coverage, low-rank PCA recovery, regime recovery, factorize-beats-mean). These run in default CI
  (`default = []`, verified `91 passed; 0 failed`). The **Python benches are print-only** —
  `bench_kaggle.py`, `bench_movielens.py`, `full_sweep.py` measure genuine ground truth against
  sklearn baselines and honestly report GIGI *losses*, but contain no `assert`/`raise`;
  `scan_fit.py` only `sys.exit()`s. `scripts/sweep_results.json` is an honest snapshot, **not a
  re-runnable gate** (needs a live release server + sklearn + manual copy). So: 7 of 8 endpoints have
  a real cargo gate; `/scan/fit` is the hole.

---

## 4. Policy flag — 22 `Co-Authored-By` footers on the ML batch

The batch `035ebad..4af484f` carries **22 `Co-Authored-By` footers** on `origin/main`. Per the
standing rule (no AI co-author on commits authored on Bee's behalf; author = Bee only), these are a
policy slip — the same class as the prior footer-slip.

**This is Bee's call, and I have changed nothing.** The commits are already on `origin/main`, so
removing the footers means a **history rewrite + force-push** of published history — the same
trade-off as last time. Flagging for awareness; not stripping, not recommending an automatic strip.
If Bee wants them gone, it's a deliberate `git rebase`/filter + `--force-with-lease` on a branch
others may have pulled, weighed against just letting the slip stand on already-published history.

---

## 5. What's clean / genuinely good

Credit where due — a hand-rolled pure-Rust ML suite with **no ML dependencies** is real work, and the
load-bearing math holds up under independent scrutiny:

- **`scan_solve` (9303)** — Gaussian elimination with partial pivoting + `|piv|<1e-12` singularity
  guard (returns `None`). Correct.
- **`mat_inv_logdet` (10070)** — Gauss-Jordan on `[A|I]`, `logdet = Σ ln|pivot|`,
  `inv[i][j] = a[i][n+j]/a[i][i]`. Verified this yields the true inverse and log|det| despite not
  normalizing pivot rows. Singular → identity fallback in GMM.
- **`kmeans_lloyd` (10101)** — genuine D²-weighted k-means++ seeding, multi-restart keeping min
  inertia. Correct.
- **GMM EM (10229–10335)** — log-sum-exp E-step, responsibilities provably sum to 1, `reg=1e-6` ridge
  on Σ, singular-cov guarded, monotone-LL convergence break; full/diagonal/spherical all correct.
- **Spectral clustering (10384–10488)** — symmetric k-NN adjacency, union-find components with exact
  fast-path when `ncomp≥k`, shifted power iteration + Gram-Schmidt deflation for bottom-k Laplacian
  eigenmaps, correct `L=D−A` and NJW `L_sym`. Correct (modulo the note-string in §2).
- **DBSCAN (10337–10382)** — correct core/border/noise semantics, auto-ε = 75th pct of minPts-NN
  distance.
- **LOF density lens (9744–9759)** — `lrd`, reachability, LOF ratio, sign all correct.
- **Completion lens (9702–9743)** — leave-one-out neighbor covariance, ridge for invertibility,
  inverse-power iteration to the smallest-eigenvalue/normal direction. Correct.
- **PCA `/reduce` (11000–11093)** — correlation-matrix power iteration + deflation, descending
  eigenvalues, reconstruction SSE computed **before** whitening so whitening can't corrupt the RMSE.
  Correct.
- **`local_linear` ridge (10584–10641)** — ridge on the normal-equations slope diagonals (intercept
  unpenalized) = `X'WX+λI`. Correct.
- **GP conformal `/infer` (10729–10792)** — split-conformal finite-sample quantile `⌈(m+1)·0.90⌉`,
  coverage measured on a held-out validation third, no leakage into Q. Genuinely calibrated.
- **SVM Pegasos (10862–10888)** & **kNN vote (10856–10860)** — correct hinge sub-gradient with
  `η=1/(λt)`; distance-weighted argmax; CV excludes the held-out fold from the neighbor pool.
- **`factorize` SGD (11235–11296)** — textbook biased regularized Funk-SVD, deterministic Fisher-Yates
  shuffle, 80/20 held-out vs global-mean baseline.
- **`changepoints` (11431–11445)** — correctly-formed pooled two-sample t + non-max suppression.
- **`scan_fit` logistic regression (9937–9952)** — the math itself reads correctly (class-weighted GD,
  z-clamp, standard AP integral); the issue is coverage, not correctness.
- **Integration** — strictly additive (3008/0, three pure-insertion hunks), router appends 8 routes
  without disturbing existing wiring, full-feature build compiles (exit 0, only pre-existing dead-code
  warnings), no locked gauge/spectral/curvature/holonomy file touched, merge `4af484f` is a clean
  two-parent with zero conflict artifacts, 927-lib-test suite + 14 named Halcyon fences green.
- **Cost/DoS guards** — cluster/predict cap at 8000 records, factorize at 500k ratings, all returning
  422 UNPROCESSABLE_ENTITY rather than hanging; determinism via fixed-seed LCGs throughout.
- **Bench honesty** — the Python harnesses run real sklearn head-to-heads and openly report GIGI
  losses (digits kmeans/gmm, wine, iris svm, diabetes local_linear). Not cherry-picked.

---

## 6. Recommended next actions (for Bee — not executed)

1. **Close the `/scan/fit` coverage gap** (the one should-fix). Extract the logistic-GD + AP core to a
   free `fn`, add a ground-truth test (planted separator; supervised PR-AUC ≥ unsupervised baseline),
   and add the fit path to `ml_all_endpoints_regression_smoke`.
2. **Fix the spectral note string** (10485) to interpolate the real component count. One-line.
3. **Consider extracting the ML block to `src/ml/`** — `gigi_stream.rs` is already huge; the block is
   self-contained and extraction also solves #1's testability problem.
4. **Soften two overstated claims** to match evidence: the GP "guaranteed coverage" comment (10732 →
   "empirically calibrated, cross-conformal") and the "fixes high-dim overfit" framing (add a high-dim
   test or temper the claim, given diabetes R²=0.366 loses to KNN).
5. **Decide the footer question** (§4) — leave the 22 `Co-Authored-By` footers as a published slip, or
   deliberately rewrite+force-push. Your call; I changed nothing.
6. **Optional:** guard `/factorize` cold-start (skip test-only entities) and make changepoint
   time-axis detection prefer monotonic columns / require explicit `time` on ambiguity.
