# Rounding in the final combination of the family block certificate (finding S6-5)

22 November 2026. Scope: `scripts/ricci_su2_family_block_cert.py` (`certify_point`, `certify_mv`) and its receipt
`research/su2-floor-family-20261119/family_block_cert_j80.json`. I did not rerun the 64-CPU-hour certificate.
Script: `scripts/ricci_su2_family_cert_rounding_bound.py`. Receipt: `rounding_bound.json` (this directory).

## Verdict

**The certificate's conclusion stands rigorously, given the bound below.** In the worst case the round-to-nearest
error of the non-directed final operations is at most **1.64e-15**. The smallest certified margin in the receipt is
**1.2221e-6** (at j = 1). That is 7.5e8 times the error bound. The finding is right that "outward rounding" overstates
what the code does. The fix is one sentence in the paper, citing this bound, or a recoded combination. The theorem
does not change.

## 1. Which operations are not directed

All the rest is enclosed: the entries (interval lambdify), `mid_rad`, `rho`, the a posteriori residual, `eps`, and the
derivative norms `dblock_upper`. The reviewer checked this and I re-read it. The non-directed steps are exactly these,
all in mpmath at `prec = 64`, so the unit roundoff is u = 2^-64 = 5.42e-20:

| # | where | operation | roundings |
|---|---|---|---|
| 1 | `certify_point` | `mu = mpf(eta) - eps` (eta a double, eps an exact mpf) | 1 |
| 2 | `certify_mv` | `hw_x = mpf(x1 - x0)/2`, `hw_y = mpf(y1 - y0)/2` | 0 (see below) |
| 3 | `certify_mv` | `hw_s = max(sh - sc, sc - sl)` | 1 (rounding is monotone, so max of rounded = rounded max) |
| 4 | `certify_mv` | `dblock_upper(...) * hw[i]`, i = 0, 1, 2 | 1 each |
| 5 | `certify_mv` | `var = fsum(...)` of three non-negative terms | <= 2 (mpmath `fsum` calls `mpf_sum`, which rounds once; I allow two) |
| 6 | `certify_mv` | `lower = mu - var`, then the test `lower > 0` | 1 |

**Step 2 is exact.** Every box endpoint is an endpoint of D = [-3,-1.5] x [0.5,3] x [3,10] plus k w / 2^n, where
w in {1.5, 2.5, 7} and n <= 20 (the depth limit is 20). Each such number is an integer over 2^21 with magnitude below
16, so it has at most 25 significant bits. The bisection `0.5*(a+b)` and the double difference `x1 - x0` are therefore
exact, and so is the division by 2.

**The centre lies inside the s-enclosure.** `sc = 0.5*(s0+s1)` uses double square roots, but the s-width of a box is at
least 7·2^-20 / (2√10) ≈ 1.05e-6, which is far larger than the ≈ 1e-15 by which s0 or s1 can miss √t. So `sc` is
inside [sl, sh], and the mean-value segment stays inside the box on which `dblock_upper` is a supremum.

## 2. Error bound

Use exact reals η (the double), e (the mpf `eps`), μ_t = η − e, h_x, h_y, h_s, and D_i (the `dblock_upper` values,
which are exact mpf), with var_t = Σ D_i h_i. The computed values are:

- μ̂ = μ_t(1+δ₁), |δ₁| <= u;
- var̂ = Σ D_i h_i (1+θ_i), |θ_i| <= γ₄ = 4u/(1−4u). This counts the h_s rounding, the product and two additions;
- the test `lower > 0` is equivalent to μ̂ > var̂. Round-to-nearest is monotone and preserves sign, and mpmath has no
  underflow.

Suppose μ̂ > var̂ >= 0. Then μ_t > 0, and var_t <= var̂/(1−γ₄) < μ̂/(1−γ₄) <= μ_t(1+u)/(1−γ₄). Hence

  μ_t − var_t >= (μ̂ − var̂) − κ μ_t,  where κ = u + γ₄(1+u)/(1−γ₄) = 2.7105e-19,

and μ̂ − var̂ = lower̂/(1+δ₂) >= lower̂/(1+u). So the exact mean-value inequality μ_t − var_t > 0 holds whenever
lower̂ > κ(1+u) μ_t.

**A bound on μ_t.** The directed part of `certify_point` proves λ_min(B̃(c)) >= η − e = μ_t exactly, where
B̃ = B_{j,q} − λ_j I. So μ_t <= λ_min(B̃(c)) <= min_k B̃_kk(c). This gives

  M* = max_{j<80, q non-twist} min_k sup_D B̃_kk,

which I computed by outward interval evaluation of the certificate's own `dshift` diagonal expressions over the whole
domain (prec 64; this is rigorous): **M* = 6025.62**, attained at j = 79. M* grows with j, from 87.8 at j = 0 to
6025.6 at j = 79; all values are in the receipt.

**Comparison.**

| quantity | value |
|---|---|
| error bound κ(1+u)M* | **1.633e-15** |
| smallest certified margin in the receipt (float of lower̂) | 1.2220726003215297e-06 (j = 1) |
| same, minus half an ulp of a double (lower̂ >= this) | 1.2220726e-06 |
| ratio | **7.48e8** |
| certified boxes covered | 1,640,935 (all 80 sectors, `all_certified: true`) |

I used the global minimum margin against the global maximum M*. Every box therefore has lower̂ >= 1.222e-6 >
1.64e-15 >= κ(1+u)μ_t, so for every certified box the exact inequality holds. **The conclusion of the j < 80
certificate is rigorous with this bound.** (Status: PROVED, given the a priori error analysis above and the interval
evaluation of M*.)

## 3. A directed re-check from stored data is not possible

The receipt stores only `min_margin` per sector (the float of lower̂, minimised over boxes). It does not store μ, var or
the boxes. A directed recomputation of the final combination for the receipt's own boxes would mean rerunning the
certificate. I did not do that.

What I did instead is a demonstration, not a re-check. I wrote a directed version of `certify_mv`
(`certify_mv_directed`). It computes hw, var and μ − var in mpmath interval arithmetic, with `eps` from the same
directed path, and tests the lower endpoint. I ran it beside the code as written on 60 random dyadic sub-boxes (random
j < 80, random non-twist q, depth 6–14, seed 20261122). Results (COMPUTED; `rounding_bound.json`,
`sample_directed_vs_nearest`):

- the verdicts agree on all 60 boxes (45 certified, 15 not);
- the directed lower endpoint is never above the round-to-nearest value;
- the largest absolute difference is 2.2e-16 (μ up to 1769), and the largest relative difference is 9.0e-18;
- the largest |difference|/(u μ) is 8.2. This is consistent with the bound: the directed endpoint carries its own
  outward widening of about 4uμ, and the round-to-nearest value carries at most κμ ≈ 5uμ.

## 4. Suggested wording for the paper (Sec. 6, proof (c))

"...bounded below by an interval LDL^T certificate with outward rounding and an a posteriori residual. The last three
scalar operations (centre bound minus residual, the mean-value sum, and their difference) are done in 64-bit
round-to-nearest. Their total error is at most κ M* = 1.6e-15 (κ = 2.7e-19, with M* = 6025.6 an interval bound on the
centre value over D), against a minimum certified margin of 1.2e-6."

Alternatively, recode the three operations in `iv` (as in `certify_mv_directed`) for any future run.
