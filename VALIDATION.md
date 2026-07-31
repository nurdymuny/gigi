# VALIDATION — the numeric validation artifacts, indexed

This file surfaces the captured validation runs that otherwise live several directories deep:
the places where a GIGI computation is checked against an independent analytic or textbook
ground truth, with the numbers quoted verbatim from the artifact files and a stated way to
falsify each claim. The ML head-to-head numbers (geometric vs flat, with losses) live in the
README section [Measured: geometric vs flat](README.md#measured-geometric-vs-flat).

## Headline: Wilson-loop holonomy vs the analytic closed form (bilayer graphene)

On AB-stacked bilayer graphene, the numerically integrated Wilson-loop holonomy |γ| matches
the analytic closed form at all seven gap ratios Δ/ε ∈ {0, 0.25, 0.5, 1, 2, 5, 100} with max
relative error 0.00029 (2.9e-4), while the integrality deviation is 0.0000 and 0.0002 at the
endpoints (integer Chern −1 and 0) but rises to 0.4472 at Δ=ε — a Dirac-string
non-integer-Chern signature that a flat-bundle negative control (constant deviation 0.2500)
cannot mimic. Falsification: rerun
[`theory/kahler_upgrade/validation/validation_tests_v5.py`](theory/kahler_upgrade/validation/validation_tests_v5.py)
(or the cross-test `tests/kahler_l71_integrality_smoke.rs` in dgp-core, the DGP repo) — any
rel_err > 0.00029 or a mid-Δ deviation ≠ 0.4472 falsifies it.

Source artifact: [`theory/kahler_upgrade/validation/results_v5.txt`](theory/kahler_upgrade/validation/results_v5.txt)

**Test 15: BLG prequantization integrality (catalog L7.1 / §2.1)** — loop radius = 5.00e+07 m⁻¹, kinetic energy ε = 2.7085 meV, BLG effective mass m* = 3.2040e-32 kg (verbatim from `theory/kahler_upgrade/validation/results_v5.txt`, lines 14–22)

| Δ/ε | analytic \|γ\| | Wilson \|γ\| | rel_err | \|γ\|/(2π) | int_dev |
|-------:|---------:|---------:|--------:|--------:|-------:|
| 0.00   | 6.283185 | 6.283185 | 0.00000 | 1.00000 | 0.0000 |
| 0.25   | 5.503852 | 5.503740 | 0.00002 | 0.87597 | 0.1240 |
| 0.50   | 4.759289 | 4.759079 | 0.00004 | 0.75746 | 0.2425 |
| 1.00   | 3.473259 | 3.472931 | 0.00009 | 0.55279 | 0.4472 |
| 2.00   | 1.840302 | 1.839978 | 0.00018 | 0.29289 | 0.2929 |
| 5.00   | 0.449394 | 0.449277 | 0.00026 | 0.07152 | 0.0715 |
| 100.00 | 0.001256 | 0.001256 | 0.00029 | 0.00020 | 0.0002 |

Assertions in the file: (1) PASS max rel_err = 0.00029 < 5% across all Δ; (2) PASS endpoint
integrality dev 0.0000 (Chern −1) and 0.0002 (Chern 0); (3) PASS Δ=ε dev = 0.4472 (Dirac
string, non-integer Chern); (4) PASS BLG mid-Δ deviation 0.4472 sits in the Wu-Yang
toy-monopole regime 0.33–0.40 (catalog test_7). Tests 16–17 add monotonicity in Δ and a
flat-bundle negative control (constant γ = π/2 gives dev 0.2500, cannot mimic the
endpoints-~0 / middle-~0.45 signature).

## Other captured artifacts

- [`theory/kahler_upgrade/validation/results_v2.txt`](theory/kahler_upgrade/validation/results_v2.txt) —
  Wu-Yang prequantization integrality (Test 7): five integer-Chern charges (q = 0.5..3.0,
  Chern 1/2/3/4/6) give N-vs-S holonomy deviation-from-integer 0.00e+00 exactly, while four
  non-integer charges correctly FAIL with deviations 0.3333–0.4000 (the Dirac obstruction);
  plus Test 8 WDVV associativity on `QH*(CP^2)` with max associator 0.00e+00 over 27 triples
  and an so(3) negative control (max associator 1.0000).
- [`theory/kahler_upgrade/validation/results_v3.txt`](theory/kahler_upgrade/validation/results_v3.txt) —
  Berezin-Toeplitz semiclassical (Test 10): normalized deviation matches the theoretical
  |hbar − 2 sin(hbar/2)| to every printed digit at four hbar values (4.1149e-02 down to
  8.1364e-05), with cubic-scaling ratios dev/hbar³ = 0.04115–0.04166 converging on the
  predicted 1/24 = 0.0417; plus Riemann-Roch on T² exact at levels n = 1–5 and discrete Hodge
  cohomology recovering Betti numbers (1, 2, 1) with commutator sanity ||d1 d0|| = 0.
- [`theory/poincare_to_sharding/validation/`](theory/poincare_to_sharding/validation/)
  (`run_all.py` + `README.md` + t1–t10, tfp\*, tfh\*) — a TDD gate suite where every test
  computes ground truth by an independent textbook method (SymPy chain homology, closed-form
  curvature, independent eigendecomposition) with explicit circular-logic guards and
  non-triviality witnesses; the README documents three honest red-first failures (T2
  extreme-value cap, T5 expander counterexample forcing an honest non-universal-bound
  disclosure, T6 spurious precondition) fixed before green; ~10 s total wall-clock.
- [`theory/encryption/validation/results_v0_3.txt`](theory/encryption/validation/results_v0_3.txt) —
  GIGI Encrypt v0.3 math validation: 25/25 PASS across Sprints I–M plus composition and
  golden-vector suites, including secp256k1 field arithmetic, HMAC-SHA256 / HKDF-SHA256 /
  SHA-256 Merkle golden vectors, chi-square pseudorandomness of the curvature MAC, Shamir
  k−1 information-theoretic security, and ratchet one-wayness.
