import sys

root = sys.argv[1]


def make_rep(state):
    def rep(a, b):
        assert state["s"].count(a) == 1, a[:80]
        state["s"] = state["s"].replace(a, b)
    return rep


# ---------------------------------------------------------------- spec.MD
st = {"s": open(f"{root}/spec.MD").read()}
rep = make_rep(st)
s = st["s"]
a = s.index("**Algorithm:**\n\n1. **DBSCAN clustering per pathogen**")
b = s.index("**Output:** `/data/tessera_results.json` (excerpt from the final run)")
st["s"] = s[:a] + '''All detection logic lives in `src/detection.py` (pure functions, runnable locally). `detect_outbreaks(mode=...)` is a thin Modal wrapper.

**Clustering (both modes):**

1. **DBSCAN per pathogen** on Euclidean distances, `min_samples=3`, adaptive radius over nonzero pairwise distances:
   ```python
   eps = max(mean - 1.5 * std, percentile(dists, 5))
   if "Salmonella" in pathogen:
       eps = percentile(dists, 3)   # serovars share >95% of core genome
   ```
   Each cluster's medoid is recorded as its center.
2. **Hierarchical sub-clustering**: any *Salmonella* cluster with more than 50 members is split with Ward linkage into `max(3, size // 10)` subclusters (min size 3; each records `parent_cluster`).
3. **Cluster quality** (`cluster_quality`): weighted serovar purity, and per pathogen **ARI / AMI against NCBI SNP clusters** (`pds_acc`) when present. Unclustered isolates count as singletons.

**Temporal analysis, `mode="robust"` (default, the baseline fixes):**

4. **Day-precision dates only.** Year- and month-only dates are excluded (inferred from `YYYY-01-01` / `YYYY-MM-01` patterns in older data, or read from `date_precision`).
5. **Calendar weeks** (Monday start). A week is tested only with at least `MIN_SAMPLES_PER_WEEK = 3` isolates.
6. **Strictly prior baseline**: the isolates collected in the `NULL_WINDOW_WEEKS = 26` calendar weeks *before* the tested week (at least 10). The centroid is computed from the baseline only, so there's no look-ahead.
7. **Rank (PIT-style) test**: one-sided Mann-Whitney U of the week's distances-to-centroid against the baseline isolates' distances (alternative: the week is closer). Also reported: the PIT value `pit_u` of the week's median distance under the baseline's empirical CDF.
8. **Benjamini-Hochberg FDR** at `FDR_ALPHA = 0.05` across all tests in the run. Each test reports `p_value`, `q_value` and `alert`.
9. **Lead time**:
   - `alert_lead_times`: days from the first FDR alert in [announcement − 365 days, announcement], among weeks where a majority of isolates are outbreak-target isolates, to the announcement.
   - `lead_times` (cluster-based): now requires a day-precision earliest member and a serovar match.

**Temporal analysis, `mode="legacy"`:** the original algorithm, kept bit-for-bit. Weekly bins by `%Y-W%W` on all dates, `D_anchor` against the all-time pathogen centroid, rolling Z over `min(26, n_weeks − 1)` rows, and an alert when Z < −2. It reproduces the Dec-2025 output exactly (tested). Its flaws, which motivated robust mode:
- weeks of 1–2 isolates could alert;
- about 60% of "dates" were year- or month-only placeholders piling into fake weeks;
- the window counted rows rather than weeks, and included the current week;
- the centroid used future data;
- about 13 false alerts were expected by chance with no multiple-testing control.

''' + s[b:]

rep('''**Output:** `/data/tessera_results.json` (excerpt from the final run)''', '''**Output:** `/data/tessera_results.json`. New runs add `detection_mode`, `config`, `temporal_coverage.date_precision`, `cluster_quality`, and in robust mode `time_traveler.{coverage, n_tests, naive_p_lt_0.05, expected_false_at_0.05, alert_lead_times}`, with per-test `p_value`/`q_value`/`pit_u`/`members`. Excerpt from the Dec-2025 legacy run:''')
rep('''| `ENCODER_TYPE` | `nucleotide_transformer` | Encoder used by Stages 3–4 |''', '''| `ENCODER_TYPE` | `nucleotide_transformer` | Encoder used by Stages 3–4 |
| `GENOME_WINDOW_BP` | 10,240 | Window size for genome sampling (= one NT chunk) |
| `GENOME_N_WINDOWS` | 48 | Windows per genome |
| `DETECTION_MODE` | `robust` | `robust` (baseline fixes) or `legacy` |
| `MIN_SAMPLES_PER_WEEK` | 3 | Robust: isolates a week needs to be tested |
| `FDR_ALPHA` | 0.05 | Robust: Benjamini-Hochberg false-discovery rate |''')
rep('''| `Z_THRESHOLD` | 2.0 | Alert when Z < −2.0 |
| `NULL_WINDOW_WEEKS` | 26 | Rolling baseline window (weeks) |''', '''| `Z_THRESHOLD` | 2.0 | Legacy: alert when Z < −2.0 |
| `NULL_WINDOW_WEEKS` | 26 | Baseline window (robust: calendar weeks strictly before; legacy: rows) |''')
rep('''| `WEEKLY_BIN_FORMAT` | `%Y-W%W` | Week label (Monday-start, `W00` = days before the first Monday) |''', '''| `WEEKLY_BIN_FORMAT` | `%Y-W%W` | Legacy week label (robust uses ISO `%G-W%V`) |''')
rep('''Hard-coded in `detect_outbreaks`: DBSCAN `min_samples=3`, `LARGE_CLUSTER_THRESHOLD = 50`, `MIN_SUBCLUSTER_SIZE = 3`.''', '''Other knobs live in `DetectionConfig` (`src/detection.py`): DBSCAN `min_samples=3`, `large_cluster_threshold=50`, `min_subcluster_size=3`, `min_baseline_isolates=10`, `lead_window_days=365`.''')
rep('''### Run Individual Stages''', '''### Choose Data Source / Detection Mode

```bash
modal run modal_pipeline.py                          # outbreak-window cohort, robust detection (default)
modal run modal_pipeline.py --source random          # original random sample
modal run modal_pipeline.py --detection-mode legacy  # original detection algorithm
```

Results are saved locally to `output/tessera_results_{source}_{mode}.json`.

### Run Locally (no Modal, no GPU)

```bash
python -m src.detection output/embeddings.npy output/embeddings_metadata.csv --mode robust
python -m src.cohort --out cohort.csv
python tests/test_detection.py && python tests/test_genome.py
```

### Run Individual Stages''')
rep('''# Download only
modal run modal_pipeline.py::download_ncbi_pathogen_data''', '''# Outbreak-window cohort (default source)
modal run modal_pipeline.py::download_outbreak_cohort
modal run modal_pipeline.py::extract_core_genome --source cohort

# Random-sample source: download only
modal run modal_pipeline.py::download_ncbi_pathogen_data''')
rep('''| tessera-data | `/data/{Pathogen_name}/` | Downloaded NCBI genomes |''', '''| tessera-data | `/data/{Pathogen_name}/` | Downloaded NCBI genomes (random source) |
| tessera-data | `/data/cohort/cohort.csv` | Outbreak-window cohort with labels |
| tessera-data | `/data/cohort/ncbi_dataset/data/` | Cohort assemblies |''')
rep('''The 2021 Onion Oranienburg outbreak couldn't be tested, because the sample contains only one Oranienburg genome.

### Earlier Run''', '''The 2021 Onion Oranienburg outbreak couldn't be tested, because the sample contains only one Oranienburg genome.

### Robust Re-analysis of the Same Embeddings

Re-running detection with `--mode robust` on `output/embeddings.npy` (local, no new embeddings):

| Metric | Value |
|--------|-------|
| Date precision (1,500 isolates) | day 482, month 167, year 660, missing 191 |
| Testable weeks (≥ 3 isolates, ≥ 10 in prior 26 weeks) | 14 (*Listeria* 13, *E. coli* 1, *Salmonella* 0) |
| Tests with p < 0.05 | 2 (0.7 expected by chance) |
| FDR alerts | **0** (best q = 0.38) |

So the 3 legacy alerts were artifacts of single-isolate weeks, and this random, century-wide sample can't support weekly surveillance. That motivated the outbreak-window cohort.

### Outbreak-Window Cohort (selected 2026-11-04, not yet embedded)

| | 2021 Onion *Salmonella* | 2018 Romaine *E. coli* |
|---|---|---|
| Window isolates (USA, assembled) | 57,204 | 13,442 |
| Target isolates | 1,907 → 300 sampled (292 labelled Oranienburg) | 388 → 300 sampled (44 labelled O157 + 256 via 22 SNP clusters) |
| Background sampled | 300 | 300 |
| Weeks with ≥ 3 cohort isolates | 56 | 54 |
| Target arrivals by month | 4–16 per month until Jul 2021, then **30 / 69 / 77** in Aug / Sep / Oct | 6–47 per month, no clear surge |

86% of cohort isolates carry an NCBI SNP-cluster label. The *E. coli* target is "any O157 SNP cluster" rather than specifically the Romaine strain, so it's a weaker test than Oranienburg.

### Earlier Run''')
s = st["s"]
a = s.index("## Future Enhancements")
b = s.index("## Using a Pre-trained Encoder")
st["s"] = s[:a] + '''## Future Enhancements

1. ~~**Temporal Analysis**~~ ✅ collection dates, weekly binning, lead-time calculations
2. ~~**Neural encoder**~~ ✅ Nucleotide Transformer 100M (ESM-2 and HyenaDNA available but unused)
3. ~~**True PIT normalization**~~ ✅ rank test against a strictly prior empirical baseline, plus BH-FDR
4. ~~**Per-week sample floor**~~ ✅ `MIN_SAMPLES_PER_WEEK`
5. ~~**Whole-genome input**~~ ✅ windows across all contigs
6. ~~**Remove random projection**~~ ✅ raw NT-100M features
7. ~~**Targeted outbreak data**~~ ✅ outbreak-window cohort with SNP-cluster labels
8. **Trained projection head**: contrastive training so the metric approximates the Davis-regularized geometry
9. **Plasmid/chromosome separation**: prerequisite for mosaic shear and HGT alerts
10. **Resistance Detection**: use the `AMR_genotypes` already in Pathogen Detection metadata for (r, θ)
11. **GIGI-backed analysis**: local-group anomaly scan, spectral clustering, changepoint tests
12. **Geographic Clustering**: location-based outbreak boundaries
13. **Real-time Streaming**: continuous monitoring of new submissions

---

''' + s[b:]
open(f"{root}/spec.MD", "w").write(st["s"])
print("spec ok")

# ---------------------------------------------------------------- README.md
st = {"s": open(f"{root}/README.md").read()}
rep = make_rep(st)
rep('''**Interpretation.**

- **Species level:**''', '''### Robust re-analysis of the same embeddings

Re-running detection with the baseline fixes (`python -m src.detection ... --mode robust`) on the saved embeddings shows the original temporal result was an artifact:

- **Dates:** only 482 of 1,500 isolates have day-precision dates; 660 are year-only and 167 month-only (previously padded to `YYYY-01-01` / `YYYY-MM-01`).
- **Testable weeks:** only 14 have at least 3 isolates and a 10-isolate prior baseline (*Listeria* 13, *E. coli* 1, *Salmonella* 0).
- **Alerts:** **0** FDR alerts (best q = 0.38); 2 tests have p < 0.05, against 0.7 expected by chance. The 3 legacy alerts came from single-isolate weeks.

A random, century-wide sample can't test weekly surveillance. The pipeline now defaults to an **outbreak-window cohort** from NCBI Pathogen Detection, selected 2026-11-04 and not yet embedded:
- 1,200 genomes: 300 target + 300 background per outbreak;
- 54–56 weeks per outbreak with at least 3 isolates;
- 86% carry an NCBI SNP-cluster label for scoring.

Oranienburg target arrivals jump from 4–16 a month to **30 / 69 / 77** in Aug / Sep / Oct 2021, around the 22 Sept announcement.

**Interpretation of the Dec-2025 run.**

- **Species level:**''')
open(f"{root}/README.md", "w").write(st["s"])
print("README ok")
