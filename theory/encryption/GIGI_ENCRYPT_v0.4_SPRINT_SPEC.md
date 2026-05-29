# GIGI Geometric Encryption — v0.4 Sprint Spec
## Sprints N, O, P, Q

**Status:** Draft — v0.1  
**Author:** Bee Rosa Davis  
**Date:** May 28, 2026  
**Depends on:** GIGI_ENCRYPT_v0.2 (Sprints A–H), v0.3.1 (integrity tag fix)  
**Validation:** All four sprints validated in `theory/encryption/validation/rigorous_v2.py`

---

## Background and Motivation

v0.2 shipped five encryption modes (Affine, Opaque, Indexed, Probabilistic,
Isometric) with six higher-level constructions (Curvature-MAC, Aff(ℝ)
delegation, holonomy ledger, Čech threshold sharing, RG-flow ratchet,
pairing-based collusion-resistant delegation). v0.4 extends the surface in
four directions motivated by the Lysyanskaya cipher-text computability
literature and by findings from the v2 rigorous validation suite:

1. **Sprint N** — Invariant consistency verification: public deterministic
   check that a reported invariant tuple was computed honestly from the
   encrypted bundle. Future direction: formal ZK protocol.
2. **Sprint O** — Credential-gated invariant query authorization: access
   control that gates query execution on invariant-ring membership and
   credential validity. Gauge rerandomization validated; CL unlinkability
   deferred.
3. **Sprint P** — Geodesic-ball approximate membership index: a geometric
   membership filter that indexes bundle members as a centroid + radius
   structure in fiber space.
4. **Sprint Q** — K-preserving transformation characterization / PQ roadmap:
   identifies the diagonal affine group as the exact K-preserving subgroup
   and separates gauge geometry from lattice-based hiding.

### New finding from v2 validation (applies to all sprints)

K alone is gameable: an adversary can construct a different distribution with
the same K. The full invariant tuple **(K, λ₁, Hol, τ)** is required as a
consistency fingerprint. Note: the invariant tuple is a forensic fingerprint,
not automatically a security parameter. Collision resistance for πinv over the
admissible bundle class has not been formally proven. All sprint claims
reference it as an invariant fingerprint or consistency signature, not a
security basis.

---

## Sprint N — Invariant Consistency Verification

### Functionality

A verifier holding only the encrypted bundle can compute K without the gauge
key and without decryption. A prover who submits a false K value is caught
with probability 1. An honest prover succeeds with probability 1.

This is public deterministic verification, not a proof of knowledge. The
verifier independently recomputes K from the ciphertext — no witness is
extracted. The future ZK direction (a formal Sigma protocol proving knowledge
of the gauge witness) is defined below as an open problem.

### Mathematical foundation

**Proposition (Sprint N):** K is a valid statement for public invariant
consistency verification because K(Encg(σ)) = K(σ) for all g ∈ Aff(ℝ)ᵏ
(gauge invariance, Theorem 3.5 of the encryption paper). Therefore:

- The verifier independently computes K from the encrypted bundle.
- Soundness: if prover claims K' ≠ K_true, verifier's computation catches
  the discrepancy with probability 1 in exact arithmetic, and with
  probability ≥ 1 − 2^{−40} after 10^{−10} quantization.
- This is an integrity claim, not a confidentiality claim. The encrypted
  bundle leaks the distribution shape, rank/order structure, and all
  affine-invariant statistics. The verifier's view is not "only K."

**Completeness (validated):** 1.0000 across 1,000 random trials.  
**Soundness (validated):** 5/5 adversarial K claims caught.

**Leakage scope:** The verifier learns πinv(B) = (K, λ₁, Hol, τ, β₀, β₁)
plus the distribution shape of the encrypted bundle. This is
leakage-scoped invariant disclosure, not zero knowledge.

**Future ZK direction (open):** A formal Sigma protocol would prove
knowledge of witness w = (g, σ) satisfying the relation:

    R = { (C, y ; w) : C = Encg(σ), y = πinv(σ), w = (g, σ) }

**Sigma-protocol target (three properties):**
1. **Completeness.** Honest prover holding w convinces verifier with
   probability 1.
2. **Special soundness.** An extractor recovers w from any two
   accepting transcripts that share the first message and differ on
   the challenge (knowledge soundness with extractor advantage 1).
3. **Special honest-verifier zero-knowledge.** A simulator, given
   only the statement (C, y) and a challenge, produces accepting
   transcripts whose distribution is indistinguishable from real
   prover transcripts.

Realizing all three for R simultaneously is the open work; the
encryption Enc_g is algebraic-affine so a Schnorr-style proof of
knowledge of the additive offset b is straightforward, but proving
knowledge of the multiplicative scale a (an element of ℝ*, not a
finite-field group element) requires a different protocol — either
range-bounded a via Pedersen commitments, or a fresh treatment over
the reals. That is PhD-level proof work; the current sprint does not
claim it.

**Open (hard proof):** Soundness of invariant reporting is clear by
construction. The open integrity question is: can two distinct bundles
B ≠ B' satisfy πinv(B) = πinv(B')? Formally:

    Pr[B' ≠ B ∧ πinv(B') = πinv(B)]

This collision experiment defines when the fingerprint is sufficient for
audit purposes.

### Implementation

```
src/invariant_verify.rs
  - InvariantStatement { bundle_id, claimed_K, claimed_lambda1, claimed_Hol, claimed_tau }
  - verify_invariant_statement(encrypted_bundle, statement) -> VerifyResult
    1. Compute K_verifier = K(encrypted_bundle) independently
    2. Check |K_verifier - statement.claimed_K| < 1e-10
    3. Repeat for λ₁, Hol, τ
    4. Return Verified | Rejected(field, delta)
  - Note: verifier NEVER receives gauge key. If it does, the test is invalid.
  - Note: this is public deterministic verification, not zero knowledge.

src/bin/gigi_stream.rs
  - v0.4 ships: POST /v1/bundles/{name}/verify_invariant
      Body: { "bundle_id": <prover's claim>, "claimed": {full tuple},
              "tolerances": Option<{per-field f64 overrides}> }
      Resp: { "verdict": "verified" | "bundle_mismatch" | "rejected", ... }
    The URL path {name} is the verifier's claim about which bundle the
    HTTP server holds — passed as `store_bundle_id` so the bundle-id
    binding (Gap 1 fix) is enforced at the wire layer.
    Not feature-gated; applies to any v0.2+ bundle.
  - Deferred to v0.5: GQL surface `VERIFY INVARIANT(K, lambda1, Hol,
    tau) ON <bundle>` — the HTTP endpoint covers the auditor-facing
    use case for v0.4; GQL parser extension is a separate scope.
```

### TDD

```rust
// test_invariant_verify.rs

// N-1: Verifier computes K from ciphertext — no key passed in
#[test]
fn test_verifier_no_key_required() {
    let plaintext = generate_bundle(1000);
    let (a, b) = (2.5_f64, 100.0_f64);
    let encrypted = apply_affine_gauge(&plaintext, a, b);
    let K_plain = compute_K(&plaintext);
    let K_verifier = compute_K(&encrypted); // key NOT passed
    assert!((K_plain - K_verifier).abs() < 1e-10);
}

// N-2: Soundness — prover submits wrong K, caught by verifier recomputation
#[test]
fn test_soundness_wrong_K_caught() {
    let bundle = generate_bundle(500);
    let K_true = compute_K(&bundle);
    let wrong_claims = vec![K_true + 0.001, K_true * 2.0, 0.0, 1.0];
    for wrong_K in wrong_claims {
        let result = verify_invariant_statement(&bundle, wrong_K, /* ... */);
        assert!(matches!(result, VerifyResult::Rejected { .. }));
    }
}

// N-3: Completeness — honest prover succeeds across 1000 random bundles
#[test]
fn test_completeness_random_bundles() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut successes = 0;
    for _ in 0..1000 {
        let pt = random_bundle(&mut rng);
        let a = rng.gen_range(0.01..100.0) * if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
        let b = rng.gen_range(-10000.0..10000.0);
        let enc = apply_affine_gauge(&pt, a, b);
        let K_honest = compute_K(&pt);
        if verify_invariant_statement(&enc, K_honest, /* ... */).is_verified() {
            successes += 1;
        }
    }
    assert!(successes as f64 / 1000.0 > 0.999);
}

// N-4: Full tuple — K alone is not sufficient (fingerprint collision test)
#[test]
fn test_full_tuple_required() {
    // Construct two bundles with same K (= Var/range²) but different
    // sorted-difference spectral-gap proxy λ₁. K depends only on the
    // 2nd moment and the (max − min) span, so a uniform-on-[0,1]
    // bundle and a 2-cluster bundle on [0,1] can be tuned to agree
    // on K while differing in λ₁ (uniform has constant sorted diffs;
    // 2-cluster has bimodal sorted diffs).
    //
    // b1: 100 i.i.d. uniform draws on [0, 1] (rng seed 7).
    // b2: 50 points clustered at q1 + 50 at q3, with q1, q3 chosen so
    //     Var(b2) = Var(b1) and range(b2) = range(b1). Concretely,
    //     scale a tight bimodal pattern to match both moments — the
    //     test helper does this by solving for cluster offsets
    //     analytically:
    //       Var(b2) = ((q3 − q1) / 2)² + δ²   (δ = within-cluster jitter)
    //       range(b2) = (q3 + δ) − (q1 − δ) = (q3 − q1) + 2δ
    //     given target (Var_b1, range_b1), choose δ small (≈ 1e-3),
    //     then q3 − q1 = sqrt(4·(Var_b1 − δ²)), and shift so range
    //     matches b1.
    let mut rng = StdRng::seed_from_u64(7);
    let b1: Vec<f64> = (0..100).map(|_| rng.gen_range(0.0..1.0)).collect();
    let var_b1 = sample_variance(&b1);
    let range_b1 = b1.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - b1.iter().cloned().fold(f64::INFINITY, f64::min);
    let delta = 1e-3_f64;
    let half_sep = (var_b1 - delta * delta).max(1e-12).sqrt();
    let q1 = 0.5 - half_sep;
    let q3 = 0.5 + half_sep;
    let b2: Vec<f64> = (0..50)
        .map(|i| q1 + (i as f64) * (2.0 * delta / 49.0))
        .chain((0..50).map(|i| q3 + (i as f64) * (2.0 * delta / 49.0)))
        .collect();
    // Rescale b2 so range matches b1 exactly (K is scale-invariant
    // under range-and-variance proportional rescaling).
    let range_b2_raw = b2.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - b2.iter().cloned().fold(f64::INFINITY, f64::min);
    let s = range_b1 / range_b2_raw;
    let b2: Vec<f64> = b2.iter().map(|x| x * s).collect();

    let k1 = compute_K(&b1);
    let k2 = compute_K(&b2);
    assert!((k1 - k2).abs() / k1.abs() < 0.05, "K should agree: {} vs {}", k1, k2);

    let l1 = compute_lambda1_proxy(&b1);
    let l2 = compute_lambda1_proxy(&b2);
    // Uniform vs bimodal sorted-diff variance differs by ≥ 10×.
    assert!((l1 - l2).abs() / l1.min(l2) > 0.5, "λ₁ should differ: {} vs {}", l1, l2);

    // Verify that full tuple (K, λ₁, Hol, τ) rejects b2 when b1 is reference.
    let stmt = InvariantStatement::from_bundle(&b1);
    assert!(matches!(
        verify_full_tuple(&b2, &stmt),
        VerifyResult::Rejected { field: "lambda1", .. }
    ));
}
```

---

## Sprint O — Credential-Gated Invariant Query Authorization

### Functionality

A user proves they hold a valid credential for a query class (e.g., `K`,
`lambda1`, `K+lambda1`) without revealing their identity. GIGI verifies
the query class is in the invariant ring IAff and executes the query.

Gauge rerandomization is validated: the same bundle encrypted under
different random gauges produces distinct ciphertexts with identical
invariant results. Full credential unlinkability (in the CL anonymous
credential sense) is deferred until CL-style randomized presentations
are implemented and proved under the CL security model.

This is adjacent to Lysyanskaya's CL anonymous credential system at the
architectural level. The analogy: CL credentials prove authorization to
access a service without revealing identity; credential-gated invariant
queries prove authorization to execute a query class without revealing
identity. The database-layer and credential-layer problems are coupled
but the formal security proof for the credential layer is deferred.

### Mathematical foundation

**Invariant ring IAff** (from §3.4 of encryption paper):

    IAff = { f ∈ Map(E, ℝ) | f ∘ Encg = f for all g ∈ Aff(ℝ)ᵏ }

IAff contains: K, λ₁, Hol, τ, β₀, β₁, and all polynomial combinations.
IAff does NOT contain: mean, sum, std, range, max, min, raw_value.

**Invariance test (mathematical falsification harness):** A query f fails
the IAff test if:

    |f(Encg(v)) − f(v)| / (|f(v)| + ε) ≥ 1e-6

for any tested gauge g ∈ Aff(ℝ)ᵏ. This finite test falsifies non-invariant
queries but does NOT prove membership in IAff by itself. Proof of membership
comes from the parser: the grammar admits only polynomial combinations of
IAff generators, so any query that parses is in IAff by construction. The
randomized gauge test is a TDD falsification harness, not the mathematical
definition of IAff.

**Generating set for IAff (parser-admitted vocabulary):** the grammar
admits polynomial combinations of the generating set

    G_IAff = { K, λ₁, ⟨Hol⟩, τ, β₀, β₁ }

These six scalars are simultaneously fixed under the field-wise
Aff(ℝ)ᵏ action (per-field affine rescaling preserves variance-over-
range, and graph-topology invariants do not see fiber values at all).
That G_IAff generates IAff (rather than merely lies inside it) is the
"completeness of the generator set" question for the affine invariant
ring of bundle-shaped data. We claim containment (Theorem 4.X in the
encryption paper) but not generation — the open question is whether
every f ∈ C(E,ℝ) satisfying f ∘ Enc_g = f for all g ∈ Aff(ℝ)ᵏ admits
a polynomial expression in G_IAff. Classical invariant theory of the
diagonal affine group on ℝᵏ suggests this is true for the value-
dependent part (Schur-Weyl style argument on the moment generating
function), with the topology-dependent part contributed by the
bitmap-graph Hodge complex. A formal generation proof is a v0.5
target.

**Adversary disguise attack (validated):** K_fake = mean(v)/std(v)² fails
the falsification test with relative change 0.5862 under gauge (3.7, 100).
The grammar catches this before execution.

**Gauge rerandomization (validated):** 5 re-randomized encryptions of the
same bundle produce 5 distinct ciphertexts, all with identical K. This
demonstrates that ciphertext presentations differ while the invariant result
is preserved. This is gauge-level rerandomization, not full CL credential
unlinkability.

**Open:** Replace HMAC credential binding with CL signatures on
committed user_id. Formal credential unlinkability proof under CL
security assumptions.

**CL flavor pinned (recommendation):** the v0.5 upgrade path targets
**BBS+ signatures** (Boneh-Boyen-Shacham 2004 + Au-Susilo-Mu 2006;
ZKAttest-style randomized presentation) rather than the original
CL04 bilinear-map construction. Rationale: BBS+ is the modern
production deployment of CL anonymous credentials (W3C VC Data
Model 2.0, Hyperledger AnonCreds 2.0), and its proof of knowledge
of a signed commitment is the cleanest fit for our "prove
authorization without revealing identity" requirement. If a path
to lattice analogs is required (post-quantum credentials),
BBS+-style signatures admit lattice variants (Beullens-Dobson-Katsumata
2023 lattice-BBS); CL04 does not without losing the unlinkability
proof. The fallback is CL04 if BBS+ tooling is unavailable in the
target deployment.

### Implementation

```
src/credentials.rs
  - QueryCredential { user_commitment: Commitment, query_class: QueryClass,
                      bundle_id: BundleId, signature: CLSignature }
  - CredentialIssuer::issue(user_id_committed, query_class, bundle_id) -> QueryCredential
  - verify_credential(credential, query_class, bundle_id) -> bool
    (verifier NEVER sees user_id — only the commitment)

src/invariant_ring.rs  (new module)
  - fn is_in_IAff(query_fn: &dyn Fn(&[f64]) -> f64, test_bundle: &Bundle) -> bool
    Falsification harness: runs query on plaintext and on 5 random gauges.
    Returns false if any |f(enc) - f(plain)| / |f(plain)| >= 1e-6.
    Returns true if all gauges pass — necessary but not sufficient for IAff
    membership. The parser provides the proof by construction.
  - fn parse_invariant_query(gql: &str) -> Result<QueryFn, GrammarError>
    Parser admits only polynomial combinations of IAff generators.
    Rejects at parse time — not runtime.

src/bin/gigi_stream.rs
  - Add GQL: CREDENTIAL QUERY <query_class> ON <bundle_id> WITH <credential>
  - Verifies credential, checks query_class ∈ IAff, executes, returns result.
  - Never logs user_id.
```

### TDD

```rust
// test_credentials.rs

// O-1: Mathematical invariance — not set lookup
#[test]
fn test_invariance_mathematical_not_lookup() {
    let bundle = generate_bundle(1000);
    let gauges = vec![(2.5,100.0), (-1.3,7.0), (0.001,-500.0), (1000.0,0.0)];
    
    // Should be invariant
    for f in [compute_K, compute_tau, |v| compute_K(v) + compute_K(v).powi(2)] {
        assert!(is_in_IAff(f, &bundle, &gauges));
    }
    // Should NOT be invariant  
    for f in [compute_mean, compute_sum, compute_std] {
        assert!(!is_in_IAff(f, &bundle, &gauges));
    }
}

// O-2: Adversarial fake-K caught by grammar
#[test]
fn test_adversarial_fake_K_caught() {
    let bundle = generate_bundle(1000);
    let K_fake = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64
                             / (variance(v) + 1e-10);
    // K_fake looks like dispersion but uses mean — NOT invariant
    assert!(!is_in_IAff(K_fake, &bundle, &[(3.7, 100.0)]));
    // Real K passes
    assert!(is_in_IAff(compute_K, &bundle, &[(3.7, 100.0)]));
}

// O-3: Gauge rerandomization — same bundle, different encryptions, same K result
// Note: this validates gauge-level rerandomization, NOT CL credential unlinkability.
// Full credential unlinkability is deferred to CL implementation.
#[test]
fn test_gauge_rerandomization() {
    let bundle = generate_bundle(500);
    let K_true = compute_K(&bundle);
    let mut ciphertexts = vec![];
    let mut K_values = vec![];
    for _ in 0..5 {
        let (a, b) = random_nonzero_gauge();
        let enc = apply_affine_gauge(&bundle, a, b);
        ciphertexts.push(enc[..5].to_vec());
        K_values.push(compute_K(&enc));
    }
    // All ciphertexts distinct
    let unique: HashSet<_> = ciphertexts.iter().collect();
    assert_eq!(unique.len(), 5);
    // All K values identical
    let K_max = K_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let K_min = K_values.iter().cloned().fold(f64::INFINITY, f64::min);
    assert!(K_max - K_min < 1e-8);
}

// O-4: Credential binding — wrong bundle_id rejected
#[test]
fn test_credential_binding() {
    let cred = issue_credential(COMMITTED_USER, "K", "bundle_001");
    assert!(verify_credential(&cred, "K", "bundle_001"));
    assert!(!verify_credential(&cred, "K", "bundle_002"));      // wrong bundle
    assert!(!verify_credential(&cred, "mean", "bundle_001"));   // non-invariant query
}
```

---

## Sprint P — Geodesic-Ball Approximate Membership Index

### Functionality

A new GIGI index type that indexes bundle members as a geodesic ball
(centroid + radius) in fiber space. Membership queries run without
decryption for scalar isotropic gauge. For field-wise affine gauge,
an ellipsoidal condition is required (see math below).

This is adjacent to Lysyanskaya's dynamic accumulator work (CL 2002,
Kemmoe-Lysyanskaya CCS 2024) in the data-structural sense: both support
membership queries without revealing the full set. They differ in security
model — the RSA accumulator has collision resistance under strong RSA;
the geodesic-ball index is a geometric approximate membership filter
without a formal cryptographic collision assumption. Do not call this
a "cryptographic accumulator."

**Leakage scope (explicit):** the index is a struct
`(centroid, covariance, chi2_threshold, member_count, gauge_type)`.
Anyone with read access to the index learns:

- the **centroid** of accumulated members in fiber space (μ ∈ ℝᵏ)
- the **covariance** Σ (or its diagonal/scalar approximation)
- the **count** of accumulated members (τ_index)
- the gauge type (scalar isotropic vs field-wise)

The index does NOT reveal individual members. However, an adversary
with read access plus arbitrary query power can probe any v ∈ ℝᵏ and
observe its membership verdict; the χ²-quantile boundary surface
defines a level-set that leaks roughly k(k+3)/2 + 1 real parameters
about the member distribution per index. **This is not a hiding
primitive.** It is a structured-data index whose security property is
correctness-of-membership-classification, not confidentiality of the
underlying member values. For confidentiality, layer OPAQUE
(AES-256-GCM-SIV) on the member-encoding before accumulation; the
index then operates on AEAD ciphertexts, and membership is a coarse
proximity claim on the ciphertext space (useful for sanctions
filtering at scale, not for hiding member identities).

### Mathematical foundation

**Accumulation:**

    centroid(S) = (1/|S|) Σᵢ sᵢ  (geometric mean in ℝᵏ)

**Membership threshold:**

    Use chi-square/Mahalanobis threshold rather than fixed 3σ:
    (v − centroid)ᵀ Σ⁻¹ (v − centroid) ≤ χ²(k, 1−α)

    where Σ = Cov(members), k = fiber dimension, α = false-reject rate.
    This is dimension-aware: correct for k=1,2,3,...,high-dimensional fibers.
    The 3σ threshold is a special case for k=1.

**Gauge consistency — two cases:**

Case 1 (scalar isotropic gauge, g(v) = av + b, same a for all fields):

    centroid(g(S)) = a·centroid(S) + b
    Σ_enc = a²·Σ
    Euclidean ball membership is preserved: v ∈ ball(S) ⟺ g(v) ∈ ball(g(S))

Case 2 (field-wise affine gauge, g(v) = Dv + b, D = diag(a₁,...,aₖ)):

    A Euclidean ball maps to an ellipsoid. Membership must use:
    (v_enc − c_enc)ᵀ (D⁻ᵀ D⁻¹) (v_enc − c_enc) ≤ r²
    i.e., the Mahalanobis distance under the induced metric.

The implementation must handle both cases. Caller specifies gauge type.

**K-consistency secondary gate:**

K-consistency is a diagnostic secondary feature, not a security gate.
The Python validation showed the adversarial boundary point passed both
the primary distance gate and the K-shift gate. K-consistency provides
a weak heuristic check; boundary-adversary resistance remains open until
a formal membership witness is defined.

**Dynamic centroid drift:**

Single deletion: ‖μ_{S\{j}} − μ_S‖ = ‖xⱼ − μ‖ / (n−1) = O(1/n)
  (valid when removed point is distance-bounded)

Batch deletion of R members:
  ‖μ_{S\R} − μ_S‖ ≤ |R| / (n − |R|) · max_{x∈R} ‖x − μ_R‖

The spec previously claimed batch removal is O(1/n) — this is incorrect.
Batch removal is O(|R|/n) and the test removes 20% of members, so the
O(1/n) asymptotic does not apply to that test.

### Implementation

```
src/membership_index.rs  (renamed from accumulator.rs)
  - GeodesicBallIndex {
        centroid: Vec<f64>,
        covariance: Matrix,    // for Mahalanobis threshold
        chi2_threshold: f64,   // χ²(k, 1−α), dimension-aware
        member_count: u64,
        gauge_type: GaugeType, // Scalar | FieldWise
    }
  - GeodesicBallIndex::new(members: &[Vec<f64>], alpha: f64) -> Self
    threshold = chi2_quantile(k, 1.0 - alpha)
    NOT 3σ fixed — dimension-aware threshold.
  - fn membership_check(v: &[f64]) -> MembershipResult
    Primary: Mahalanobis distance ≤ chi2_threshold
    K-consistency: diagnostic only, not a security gate.
  - fn encrypted_membership_scalar(v_enc: &[f64], a: f64, b: f64) -> MembershipResult
    Valid for scalar isotropic gauge: g(v) = av + b (same a all fields).
  - fn encrypted_membership_fieldwise(v_enc: &[f64], D: &DiagMatrix, b: &[f64]) -> MembershipResult
    Required for field-wise affine gauge: uses ellipsoidal condition
    (v_enc − c_enc)ᵀ (D⁻ᵀ D⁻¹) (v_enc − c_enc) ≤ r²

GQL extension:
  CREATE MEMBERSHIP INDEX ON <bundle> FIELDS (f1, f2, ...) ALPHA 0.05
  INSERT <value> INTO <index_id>
  MEMBERSHIP <value> IN <index_id>  -- returns bool, no decryption
```

### TDD

```rust
// test_membership_index.rs

// P-1: Threshold is dimension-aware (chi-square), not fixed 3σ
#[test]
fn test_threshold_dimension_aware() {
    let members_k1 = generate_bundle_k(50, 1);
    let members_k4 = generate_bundle_k(50, 4);
    let idx_k1 = GeodesicBallIndex::new(&members_k1, 0.05);
    let idx_k4 = GeodesicBallIndex::new(&members_k4, 0.05);
    // Thresholds should differ by dimension — not be the same 3σ value
    assert!((idx_k1.chi2_threshold - idx_k4.chi2_threshold).abs() > 1e-6,
            "Threshold must be dimension-aware");
    // k=1: χ²(1, 0.95) ≈ 3.84; k=4: χ²(4, 0.95) ≈ 9.49
    assert!((idx_k1.chi2_threshold - 3.84).abs() < 0.1);
    assert!((idx_k4.chi2_threshold - 9.49).abs() < 0.1);
}

// P-2: True positive rate matches the 1−α tail bound
// (statistical statement: TPR converges to 1−α as n→∞, with sampling
// deviation O(1/√n). For n=50 and α=0.05 the 95% CI on TPR is
// [0.84, 0.99]; the lower bound is the assertion.)
#[test]
fn test_true_positive_rate_matches_tail_bound() {
    let members = generate_bundle_k(50, 3);
    let alpha = 0.05_f64;
    let idx = GeodesicBallIndex::new(&members, alpha);
    let tpr = members.iter()
        .filter(|m| idx.membership_check(m).is_member())
        .count() as f64 / 50.0;
    let expected_tpr = 1.0 - alpha;
    let n = members.len() as f64;
    let sampling_sd = (expected_tpr * alpha / n).sqrt();
    // Within 3 sampling standard deviations of the χ² tail bound.
    assert!(
        (tpr - expected_tpr).abs() < 3.0 * sampling_sd,
        "TPR = {} (expected ≈ {} ± {})",
        tpr, expected_tpr, 3.0 * sampling_sd
    );
}

// P-3: Adversarial boundary — false admit rate documented, not hidden
#[test]
fn test_adversarial_boundary_documented() {
    let members = generate_bundle_k(50, 3);
    let idx = GeodesicBallIndex::new(&members, 0.05);
    let mut false_admits = 0;
    for _ in 0..1000 {
        let direction = random_unit_vector(3);
        // Mahalanobis boundary: point at 0.99 * threshold in adversarial direction
        let adversary = mahalanobis_boundary_point(&idx.centroid, &idx.covariance,
                                                    &direction, 0.99);
        if idx.membership_check(&adversary).is_member() {
            false_admits += 1;
        }
    }
    let far = false_admits as f64 / 1000.0;
    println!("Boundary adversary false admit rate: {:.4}", far);
    // Document — K-consistency is diagnostic only, not a security gate
    assert!(far < 0.50, "False admit rate unexpectedly high: {}", far);
    // TODO: formal membership witness needed to close this.
}

// P-4: Encrypted membership — scalar gauge (ball preserved) vs field-wise (ellipsoid)
#[test]
fn test_encrypted_membership_scalar_gauge() {
    // Scalar isotropic gauge: g(v) = av + b (same a for all fields)
    // Ball membership is preserved
    let members = generate_bundle_k(50, 3);
    let idx = GeodesicBallIndex::new(&members, 0.05);
    let test_member = members[0].clone();
    let (a_scalar, b) = (3.7_f64, 1000.0_f64);
    let plain_result = idx.membership_check(&test_member);
    let enc_result = idx.encrypted_membership_scalar(
        &apply_affine_isotropic(&test_member, a_scalar, b), a_scalar, b
    );
    assert_eq!(plain_result.is_member(), enc_result.is_member(),
               "Scalar gauge must preserve ball membership");
}

#[test]
fn test_encrypted_membership_fieldwise_requires_ellipsoid() {
    // Field-wise gauge: D = diag(a1, a2, a3) with different a_i
    // Euclidean ball maps to ellipsoid — must use Mahalanobis condition
    let members = generate_bundle_k(50, 3);
    let idx = GeodesicBallIndex::new(&members, 0.05);
    let test_member = members[0].clone();
    let D = DiagMatrix::new(vec![2.0, 5.0, 0.3]); // different scale per field
    let b = vec![100.0, 200.0, 50.0];
    let enc = apply_affine_fieldwise(&test_member, &D, &b);
    let plain_result = idx.membership_check(&test_member);
    // Must use ellipsoidal check, not Euclidean
    let enc_result = idx.encrypted_membership_fieldwise(&enc, &D, &b);
    assert_eq!(plain_result.is_member(), enc_result.is_member(),
               "Field-wise gauge requires ellipsoidal membership check");
}

// P-5: Dynamic centroid — single deletion O(1/n), batch is O(|R|/n)
#[test]
fn test_dynamic_centroid_drift() {
    let members = generate_bundle_k(50, 3);
    let idx_full    = GeodesicBallIndex::new(&members, 0.05);
    let idx_minus1  = GeodesicBallIndex::new(&members[..49], 0.05);   // single deletion
    let idx_minus10 = GeodesicBallIndex::new(&members[..40], 0.05);   // batch deletion

    let drift_single = euclidean_distance(&idx_full.centroid, &idx_minus1.centroid);
    let drift_batch  = euclidean_distance(&idx_full.centroid, &idx_minus10.centroid);

    // Single deletion: O(1/n)
    assert!(drift_single < 0.5, "Single deletion drift too large: {}", drift_single);
    // Batch deletion: O(|R|/n) — larger, not O(1/n)
    assert!(drift_batch > drift_single,
            "Batch deletion drift should exceed single deletion");
    println!("Drift single={:.4}, batch={:.4} (ratio={:.2}x)",
             drift_single, drift_batch, drift_batch / drift_single);
}
```

---

## Sprint Q — K-Preserving Transformation Characterization / PQ Roadmap

### Functionality

Characterizes exactly which linear and affine maps preserve the Davis
dispersion K, and separates that geometric question from lattice-based
hiding. Lays the groundwork for a post-quantum extension by identifying
the mathematical constraint any PQ replacement must satisfy.

This is not a shipped post-quantum encryption mode. It is a roadmap
document with validated mathematical findings.

### Mathematical foundation

**Corrected result: K-preserving affine group**

For per-coordinate Davis dispersion Kᵢ = Var(xᵢ)/range(xᵢ)², the
diagonal affine group preserves each Kᵢ:

    GAff_K = (ℝ*)ᵏ ⋉ ℝᵏ

i.e., independent per-field scalings aᵢ ≠ 0 and translations bᵢ preserve
each Kᵢ. This is the field-wise affine group Aff(ℝ)ᵏ acting diagonally.

Note: translations b ∈ ℝᵏ are not elements of GL(ℝᵏ); they belong to the
affine group Aff(ℝᵏ). The K-preserving group lives in the affine group,
not in the linear group GL(ℝᵏ).

**Two sub-cases:**

(a) For per-field K vector (K₁,...,Kₖ) preservation:
    Admissible group = diagonal affine (ℝ*)ᵏ ⋉ ℝᵏ — independent aᵢ per field.

(b) For Euclidean ball membership preservation (Sprint P Case 1):
    Further restriction required: aᵢ = a for all i (isotropic scalar).
    The admissible linear part contracts to scalar conformal maps aI.

These two requirements are distinct. The encryption paper (Affine mode) uses
case (a). Sprint P scalar gauge uses case (b). Both are correct within their
stated scope.

**O(k) rotation:**

O(k) rotation preserves tr(Cov(v)) (rotation-invariant). It does NOT
preserve per-field Kᵢ (rotation mixes fields). It also does NOT preserve
tr(Cov(v))/range² because range = max−min is not rotation-invariant.

A rotation-invariant version of trace-K must use a rotation-invariant
denominator:

    K_trace = tr(Cov(v)) / diam(v)²,   diam(v) = max_{p,q} ‖vₚ − v_q‖

or simply use tr(Cov(v)) directly as the Isometric mode invariant.
The (max−min)² denominator used in earlier drafts is not rotation-invariant
and should not appear in Isometric mode claims.

**PQ roadmap:**

PQ-Scalar (as previously specified with modular arithmetic) is not a
post-quantum encryption mode. Scalar affine mod q is easy algebra. LWE can
provide a computational hiding layer, but the scalar affine transform itself
provides no post-quantum security.

The correct framing: LWE is a hiding primitive (separate from the gauge).
The gauge preserves K; LWE hides the plaintext. These are separate layers.

Correct modular requirement: for the scalar gauge g(v) = av + b mod q to
preserve K, all encoded values must satisfy 0 ≤ av + b < q (no wraparound).
The range-check mitigation "reject if a·range > q/2" is necessary but not
sufficient; a centered representative interval with full no-wrap proof is
required.

**LWE separability (validated):**

K(As + e mod q) ≈ K(random), not K(s). This illustrates that LWE behaves
as a hiding layer rather than a gauge action. It is not a security proof;
LWE pseudorandomness is a distributional/computational assumption.

**Research invitation (open problem statement):**

The diagonal affine group (ℝ*)ᵏ ⋉ ℝᵏ is the exact K-preserving
subgroup of Aff(ℝ)ᵏ acting on ℝᵏ. A post-quantum mode requires a
*hiding layer* (computational, LWE-based) whose induced action on
the plaintext fiber commutes with this gauge — i.e., the hiding map
H_LWE and any g ∈ (ℝ*)ᵏ ⋉ ℝᵏ should satisfy

    g ∘ H_LWE  =  H_LWE ∘ g'   for some g' ∈ G_AffK

so that the verifier can still compute K(H_LWE(g(σ))) without
decrypting. We do not know of prior work that constructs such an
H_LWE; the closest analogs (Kirshanova 2014 lattice PRE,
Aono-Hayashi 2017) hide plaintext under lattice hardness but do not
preserve a geometric invariant under encryption. We'd value any read
on whether the gauge-equivariant lattice-hiding question is
well-formed and whether existing techniques (homomorphic LWE
encodings, structured lattices over ℝᵏ, ring-LWE with diagonal
multiplier algebras) suggest a route. We'd be honored to collaborate
on this if the question turns out to be tractable.

### Implementation

```
src/crypto.rs  (additions — roadmap only, not a shipped mode)
  - fn is_K_preserving_affine(D: &DiagMatrix, b: &[f64], bundle: &Bundle) -> bool
    Check: does diagonal affine (D, b) preserve per-field K?
    Returns true iff all |Kᵢ(D*v + b) - Kᵢ(v)| / Kᵢ(v) < 1e-6.

  - fn is_K_preserving_scalar(a: f64, b: &[f64], bundle: &Bundle) -> bool
    Check: does scalar isotropic gauge (aI, b) preserve K?
    Required subset for Euclidean ball membership preservation.

  - fn characterize_K_preserving_group(k: usize) -> SubgroupDescription
    Returns:
      "For per-field K: diagonal affine (R*)^k ⋉ R^k"
      "For Euclidean ball membership: scalar conformal aI + b, a ∈ R*"
      "Linear subgroup (no translation): restrict to (R*)^k or {aI}"

  // PQ-Scalar: NOT a shipped mode. Roadmap only.
  // Requires: (a) correct modular no-wrap guarantee, (b) LWE hiding layer
  // on top, (c) formal security reduction. None of these are done.
  // Do not ship as EncryptionMode until these are complete.
```

### TDD

```rust
// test_K_preserving.rs

// Q-1: General GL shear breaks per-field K — expected and confirmed
#[test]
fn test_general_GL_shear_breaks_K() {
    let bundle = generate_bundle(500);
    let mut shear = Matrix::identity(3);
    shear[(0, 1)] = 2.0;
    assert!(!is_K_preserving_affine(&DiagMatrix::from_matrix(&shear), &[], &bundle),
            "Shear matrix must NOT preserve per-field K");
}

// Q-2: Diagonal affine (R*)^k ⋉ R^k preserves per-field K
#[test]
fn test_diagonal_affine_preserves_per_field_K() {
    let bundle_k = generate_bundle_k(500, 3);
    // Independent a_i per field — all ≠ 0
    let D = DiagMatrix::new(vec![2.5_f64, -1.3, 0.7]);
    let b = vec![100.0, -50.0, 300.0];
    for i in 0..3 {
        let K_orig = compute_K(&bundle_k.col(i));
        let K_enc  = compute_K(&apply_affine_field(&bundle_k.col(i), D[i], b[i]));
        assert!((K_orig - K_enc).abs() < 1e-10,
                "Diagonal affine must preserve K per field, field {}", i);
    }
}

// Q-3: O(k) rotation — tr(Cov) is invariant; (max-min)^2 denominator is NOT
#[test]
fn test_rotation_trCov_invariant_range_not() {
    let bundle = generate_bundle_k(500, 3);
    let R = rotation_matrix_2d_embedded(3, std::f64::consts::PI / 5.0);
    let rotated = apply_matrix_gauge(&bundle, &R);

    // tr(Cov) should be rotation-invariant
    let trCov_orig = bundle.covariance().trace();
    let trCov_rot  = rotated.covariance().trace();
    assert!((trCov_orig - trCov_rot).abs() < 1e-8,
            "tr(Cov) must be rotation-invariant");

    // (max-min)^2 denominator is NOT rotation-invariant — document this
    let range_sq_orig = (bundle.max() - bundle.min()).powi(2);
    let range_sq_rot  = (rotated.max() - rotated.min()).powi(2);
    // These will differ — that's expected
    println!("range² original={:.6}, rotated={:.6} (rotation-variant, as expected)",
             range_sq_orig, range_sq_rot);

    // Rotation-invariant trace-K must use diameter²
    let diam_orig = bundle.diameter();
    let diam_rot  = rotated.diameter();
    let trK_diam_orig = trCov_orig / diam_orig.powi(2);
    let trK_diam_rot  = trCov_rot  / diam_rot.powi(2);
    assert!((trK_diam_orig - trK_diam_rot).abs() < 1e-8,
            "tr(Cov)/diam² must be rotation-invariant");
}

// Q-4: LWE is hiding — K(As+e) ≈ K(random), this illustrates the concept
// Note: this is not a security proof; LWE security is a computational assumption.
#[test]
fn test_LWE_hiding_illustration() {
    let s = random_binary_secret(256);
    let (A, e) = generate_LWE_instance(500, 256, 3.0);
    let lwe_samples = lwe_encrypt(&A, &s, &e, 65537);
    let random_samples = random_uniform(500, 65537);
    let K_s    = compute_K(&s);
    let K_lwe  = compute_K(&lwe_samples);
    let K_rand = compute_K(&random_samples);
    // LWE samples should look more like random than like secret in K-space
    assert!((K_lwe - K_rand).abs() < (K_lwe - K_s).abs(),
            "LWE illustration: K(As+e) closer to K(random) than K(s)");
}

// Q-5: K-preserving group characterization — diagonal affine, not scalar-only
#[test]
fn test_K_preserving_group_is_diagonal_affine() {
    let bundle = generate_bundle_k(500, 3);
    // Diagonal affine (independent a_i) preserves per-field K
    let D_diag = DiagMatrix::new(vec![2.5_f64, -1.3, 0.7]);
    assert!(is_K_preserving_affine(&D_diag, &[100.0,-50.0,300.0], &bundle));
    // Scalar (special case of diagonal) also preserves K
    let D_scalar = DiagMatrix::new(vec![2.5_f64, 2.5, 2.5]);
    assert!(is_K_preserving_affine(&D_scalar, &[0.0,0.0,0.0], &bundle));
    // Shear (off-diagonal) breaks K
    assert!(!is_K_preserving_affine(&DiagMatrix::from_shear(2.0, 3), &[], &bundle));
    // Rotation breaks per-field K
    let R = rotation_matrix_2d_embedded(3, 0.5);
    assert!(!is_K_preserving_affine(&DiagMatrix::from_matrix(&R), &[], &bundle));
}
```

---

## Cross-Sprint Requirements

### Full invariant tuple enforcement (all sprints)

Per the v2 validation finding: K alone is gameable (adversary can match K
with a different distribution; λ₁ catches it). All sprint security claims
must reference the full tuple:

    πinv(B) = (K, λ₁, Hol, τ, β₀, β₁)

Not K alone. Where a sprint's spec references K as a security parameter,
replace with πinv or explicitly justify why K suffices for that specific
claim.

### Sprint ordering

N and O are independent. P depends on N (formal membership witness needed
to close boundary adversary). Q is independent.

Recommended build order: **N → O → Q → P**

### Open problems table

| Sprint | Open problem | Difficulty | Path forward |
|--------|-------------|------------|--------------|
| N | Formal Sigma protocol proving knowledge of gauge witness | Hard (PhD) | Define relation R = {(C,y;w): C=Encg(σ), y=πinv(σ), w=(g,σ)} |
| O | Replace HMAC with CL signatures on committed user_id | Medium | Camenisch-Lysyanskaya 2004; formal unlinkability proof |
| P | Boundary adversary false admit; formal membership witness | Medium | Formal membership witness or ZK proof from Sprint N |
| Q | Lattice hiding layer with computational hardness over diagonal affine group | Hard (PhD) | Closest prior work: Kirshanova 2014 (lattice PRE), Aono-Hayashi 2017 (lattice PRE); neither preserves a geometric invariant like K under encryption. Combining gauge invariance with lattice-based hiding is the open construction. |

---

## Note for Lysyanskaya Email

*To be added to the email to Dr. Lysyanskaya:*

PRISM — a separate Davis Geometric product built on GIGI — is already
live at useprism.sh. It implements geometric privacy (PRISM Vault) for
financial reconciliation: transaction fields are embedded as geometric
objects inside the bank's secure perimeter; only the shapes leave. PRISM
achieves 99.97% F1 on 1M transactions in 22 seconds, with S + d² = 1 as
the conservation law.

The architectural lineage is direct. **Compact E-Cash**
(Camenisch-Lysyanskaya CRYPTO 2005) established a primitive that's still
the cleanest statement of what privacy-preserving payments need: *a
transaction is verifiable without revealing the transactable*. The
encoded coin is committed; the spend reveals only enough algebraic
information to prove validity. PRISM's *"shapes leave the perimeter,
values don't"* is the database-runtime echo of the same idea. We've
moved the locus from a single coin's serial number to a continuous
fiber over a bundle (so the "coin" becomes a transaction record with
many fields, and the "serial" becomes the bundle's invariant
fingerprint π_inv), but the architectural commitment — verifiability
without revelation — is the same. The 2024 invited talk on the
privacy-preserving digital dollar extends that lineage to a national-
scale CBDC; PRISM is the deployed industrial-scale instance of the
same architectural principle, validated on real bank reconciliation
data today.

---

*End of GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md*
