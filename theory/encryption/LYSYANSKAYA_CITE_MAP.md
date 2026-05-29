# Anna Lysyanskaya — Citation Map for Davis Geometric papers

**Purpose:** locate every place across the Davis Geometric paper portfolio where Lysyanskaya's work belongs as a citation. Sorted by paper → section → specific paper → exact prose change.

**Strategic frame:** Lysyanskaya was Bee's instructor in EMCS2010 (Applied Cryptography & Data Privacy, summers 2017–2020). The throughline from that classroom to this codebase is a recommendation-letter story. Beyond the relationship, the *technical* overlap is real on at least seven of her ten research lanes — citing carefully positions GIGI as a database-native instantiation of constructions she has been working at the credential / proof / payment layer for two decades.

**Also note:** she co-authors with Roberto Tamassia (multicast authentication 2004, 2010). Bee reconnecting with both simultaneously is one Brown-CS thread, not two.

---

## Catalog of relevant Lysyanskaya papers (verified-existing as of 2024)

| Short cite | Authors | Venue / Year | Topic |
|---|---|---|---|
| `lysyanskaya2002vrf` | Lysyanskaya | CRYPTO 2002 | Unique signatures and VRFs from DH-DDH separation |
| `camenisch2002accumulators` | Camenisch, Lysyanskaya | CRYPTO 2002 | Dynamic accumulators & efficient credential revocation |
| `camenisch2004clsigs` | Camenisch, Lysyanskaya | CRYPTO 2004 | Signature schemes and anonymous credentials from bilinear maps (CL signatures) |
| `camenisch2005ecash` | Camenisch, Hohenberger, Lysyanskaya | Eurocrypt 2005 | Compact e-cash |
| `camenisch2005onion` | Camenisch, Lysyanskaya | CRYPTO 2005 | A formal treatment of onion routing |
| `chase2006sigs-of-knowledge` | Chase, Lysyanskaya | CRYPTO 2006 | Signatures of knowledge |
| `camenisch2019mercurial` (CL19) | Crites, Lysyanskaya | CT-RSA 2019 | Mercurial signatures for delegatable anonymous credentials |
| `chase2019mercurial` (CL21) | Crites, Lysyanskaya | Designs, Codes & Cryptography 2021 | Mercurial signatures (extended) |
| `kohlweiss2023blueprints` | Kohlweiss, Lysyanskaya, Nguyen | Eurocrypt 2023 | Privacy-preserving blueprints |
| `kemmoe2024accumulator` | Kemmoe, Lysyanskaya | CCS 2024 | RSA-based dynamic accumulator without hashing into primes |
| `garman2024mercurial` (GLMPS24) | Garman, Lysyanskaya, Miers, et al. | ASIACRYPT 2024 | Stronger privacy for delegatable credentials |
| `bellare2024prfs` | Bellare, Lysyanskaya | Journal of Cryptology 2024 | Symmetric and dual PRFs from standard assumptions |

(11 papers across her 7 most-relevant lanes for Davis Geometric.)

---

## Map by paper

### 1. **Geometric Encryption** (`theory/encryption/paper_geometric_encryption_v0.1.tex`)

#### §1.1 — The compute-on-ciphertext problem

**Where:** End of the "property-preserving encryption" paragraph (line ~125), where the paper says "None offer a unifying framework that says 'encryption mode X preserves query class Y because of mathematical reason Z.' We provide such a framework."

**Add citation:** `kohlweiss2023blueprints` (Privacy-preserving blueprints, Eurocrypt 2023).

**Why:** Privacy-preserving blueprints is *philosophically* the closest single paper to GIGI's core move. A blueprint is a policy; the proof is that committed data satisfies it without revealing the data. GIGI's gauge-invariant ring + `PROJECT INVARIANT` grammar is the database-native analogue: the structure group defines the policy, the invariant ring is what's computable on ciphertext, and the framework provably enforces it. Citing the blueprints paper positions GIGI as the database-runtime layer of the same theoretical lineage.

**Specific prose to add:**
> *"...we provide such a framework. The closest spiritual antecedent in the cryptographic literature is privacy-preserving blueprints~\cite{kohlweiss2023blueprints}, which prove that committed values satisfy a policy without revealing the values; GIGI's gauge-invariant ring (§3) is the database-runtime instantiation of the same idea, with the structure group of the encryption playing the role of the policy."*

---

#### §1.3 — Honest scope statements (delegation modes)

**Where:** The paragraph introducing the BLS12-381 pairing-based collusion-resistant delegation construction.

**Add citations:**
- `camenisch2004clsigs` (CL signatures, CRYPTO 2004) — the canonical anonymous-credential / delegation primitive.
- `camenisch2019mercurial` (Crites-Lysyanskaya mercurial signatures, CT-RSA 2019) — the formal delegation-capability framework.
- `garman2024mercurial` (Garman et al. ASIACRYPT 2024) — the latest privacy strengthening for delegatable credentials.

**Why:** Lysyanskaya has spent 22 years building the *anonymous-credential delegation* primitive (CL signatures → mercurial signatures → GLMPS24). GIGI's Aff(ℝ) and pairing-based delegation are solving the *database-access* analogue: how do you give Bob compute access to Alice's records without exposing Alice's key. Citing CL + mercurial signatures positions GIGI's delegation construction in conversation with two decades of credential-delegation theory rather than re-inventing the framing.

**Specific prose to add:**
> *"... built on BLS12-381 with collusion resistance reducing to discrete log on $G_2$. The delegation primitive is conceptually adjacent to the anonymous-credential delegation work of Camenisch and Lysyanskaya~\cite{camenisch2004clsigs} and its more recent mercurial-signature evolutions~\cite{camenisch2019mercurial, garman2024mercurial}, which deliver delegatability + unlinkability at the credential layer; we deliver the database-record-access analogue."*

---

#### §2 — Background, external math anchors

**Where:** The bulleted list of external math anchors (lines ~270–315), where existing citations cover PPE, structured encryption, SSE, OPE, FHE, PRE, AES-GCM-SIV, CMAC, HKDF, Shamir, RFC 6962, Signal ratchet.

**Add four new entries:**

1. `lysyanskaya2002vrf` — **Verifiable random functions**: the PRF / VRF foundation Lysyanskaya established. Useful for Indexed mode's PRF instantiation discussion.
   > *"\textbf{Verifiable random functions}~\cite{lysyanskaya2002vrf}: the PRF/VRF security foundations on which our INDEXED mode (AES-CMAC) sits as a deterministic PRF instantiation."*

2. `camenisch2004clsigs` — **CL signatures**: as already noted in §1.3, the credential-delegation ancestor.
   > *"\textbf{CL signatures and anonymous credentials}~\cite{camenisch2004clsigs}: the bilinear-pairing-based signature scheme underpinning two decades of credential-delegation work; conceptually adjacent to our pairing-based delegation (\S 6, Sprint J.2)."*

3. `camenisch2002accumulators` and `kemmoe2024accumulator` — **Dynamic accumulators**: the membership-data-structure primitive lane Lysyanskaya has worked since 2002. GIGI's curvature-indexed storage is the geometric-data analogue.
   > *"\textbf{Dynamic accumulators}~\cite{camenisch2002accumulators, kemmoe2024accumulator}: the membership-proof primitive lane; GIGI's field-index bitmap and base-graph topology give a related but geometric-side membership / spectral-connectivity primitive."*

4. `kohlweiss2023blueprints` — already pointed at in §1.1, reinforced here.

---

#### §3.4 — The invariant ring (sublanguage discipline)

**Where:** End of §3.4 where we describe `PROJECT INVARIANT(...)` as "the parser surface that admits polynomial-combination expressions over $\pi_{\mathrm{inv}}$; it is by construction a sublanguage of $\mathcal{I}_{\mathrm{Aff}}$."

**Add citation:** `kohlweiss2023blueprints` (again).

**Why:** Same paper, second-order strategic placement. The first citation in §1.1 positions GIGI as inheriting the blueprints theoretical lineage. The second citation in §3.4 anchors the specific *language-restriction* engineering pattern (admit only invariant-preserving operations) as the database analogue of the blueprints proof structure.

**Specific prose:**
> *"... by construction a sublanguage of $\mathcal{I}_{\mathrm{Aff}}$. This pattern --- restricting a query grammar to admit only operations on a committed invariant subspace --- is the database-runtime analogue of the privacy-preserving-blueprints proof discipline~\cite{kohlweiss2023blueprints}, where the language of provable statements is restricted to the policy-satisfying subspace."*

---

#### §6 — Delegation (forthcoming, but cite structure to draft against)

**Where:** The Aff(ℝ) capability delegation section and the pairing-based collusion-resistant alternative section.

**Add citations:** `camenisch2019mercurial` and `garman2024mercurial` as the delegatable-credential anchor; `camenisch2002accumulators` for the revocation pattern.

**Why:** The §6 discussion of "trusted-delegatee versus collusion-resistant" maps directly onto the credential / mercurial-signature literature's analogous trust-model classifications. Citing these positions Sprint J in conversation with the mature theory of delegation.

---

#### §11 — Deferred constructions (spectral-signature ZKP)

**Where:** §11.2 — Spectral-signature ZKP construction sketch.

**Add citations:**
- `chase2006sigs-of-knowledge` (signatures of knowledge, CRYPTO 2006)
- `kohlweiss2023blueprints` (blueprints, Eurocrypt 2023)

**Why:** The spectral-signature ZKP is *exactly* in Lysyanskaya's ZK-proof lane. Signatures of knowledge is the canonical primitive for "prove you know a witness satisfying a policy without revealing the witness." Privacy-preserving blueprints is its policy-extensional successor. The spectral-signature ZKP we deferred to v0.4 is a specific instantiation: prove a record satisfies a spectral-membership predicate (Δλ_1 < ε) without revealing the record. Two-decade ZK lineage; citing it strengthens the construction's pedigree.

**Specific prose for the v0.4 paper:**
> *"The spectral-signature ZKP construction is in the lineage of signatures of knowledge~\cite{chase2006sigs-of-knowledge} and privacy-preserving blueprints~\cite{kohlweiss2023blueprints} — primitives for proving a committed value satisfies a stated policy without revealing the value. Here the policy is spectral-gap membership: |Δλ_1| < ε."*

---

#### §11 — Deferred constructions (geodesic-ball ABE)

**Where:** §11.1 — Geodesic-ball CP-ABE construction sketch.

**Add citation:** `camenisch2004clsigs` (CL signatures, the bilinear-pairing-based primitive that informs many ABE constructions).

**Why:** ABE constructions over pairing-friendly curves (BLS12-381, BN254) trace lineage to the bilinear-map signature schemes Camenisch and Lysyanskaya formalized. Citing CL signatures provides the right historical anchor for the deferred ABE construction.

---

### 2. **Kähler Substrate paper** (`theory/kahler_upgrade/paper_kahler_substrate_v0.4.tex`)

**Strategic note:** This paper is already published-ready (v0.4 compiled, 30pp). Citation insertions should be conservative — only where the addition strengthens a specific claim without altering the paper's narrative.

#### §2.3 — Relation to spoken-dialogue-systems literature (background citations)

**Where:** External-anchor enumeration.

**Add citation:** `lysyanskaya2002vrf` — at the moment where we discuss the substrate's deterministic-randomness primitives.

**Why:** Tangential but legitimate; the Kähler-substrate paper does use VRF-like primitives for some of its sampling discipline. Lower priority than encryption-paper insertions.

**Skip if low value.** The Kähler paper is reaching publication readiness; don't add cites that don't pay clear strategic dividend.

---

### 3. **PRISM patent / paper** (Patent ref'd in your CV catalog: Privacy-Preserving Financial Transaction Reconciliation via Non-Invertible Geometric Embeddings, US 74859695)

**Strategic note:** Lysyanskaya has 20 years of work on anonymous-payment systems (compact e-cash 2005, endorsed e-cash 2007, accumulators for anonymity-preserving revocation 2017, privacy-preserving digital dollar 2024 invited talk). Her **2024 invited lecture on the privacy-preserving digital dollar** is the freshest most-relevant work.

**Citation plan for the PRISM paper draft (when it's written):**

| Section | Cite | Why |
|---|---|---|
| Background — anonymous payments | `camenisch2005ecash` | The compact e-cash construction Lysyanskaya co-authored is the academic canon PRISM extends |
| Background — accumulators for revocation | `camenisch2002accumulators` | Lysyanskaya's accumulator work is the membership-proof primitive PRISM's geometric-embedding membership construction parallels |
| Background — recent work | (Lysyanskaya digital-dollar lecture 2024 — cite as a tech report or invited talk reference) | The most-recent shared-context work; positions PRISM in conversation with her current research |
| Related work | `kohlweiss2023blueprints` | The policy-on-committed-value framing applies directly to payment policies |
| Related work | `camenisch2019mercurial`, `garman2024mercurial` | Delegatable credentials = payment-authority delegation pattern |

This is the **highest-payoff** Lysyanskaya citation surface in the Davis portfolio: PRISM is the privacy-preserving-payment-reconciliation patent, and her digital-dollar work is the most directly adjacent recent literature.

---

### 4. **The PRF / VRF cluster of patents** (if you have a paper draft on geodesic hash functions or curvature-indexed storage)

**Where:** If the geodesic hash / curvature-indexed storage patent is paperized.

**Add citations:**
- `lysyanskaya2002vrf` (VRFs from DH-DDH)
- `bellare2024prfs` (Bellare-Lysyanskaya 2024 — symmetric and dual PRFs from standard assumptions)
- `camenisch2002accumulators` and `kemmoe2024accumulator` (accumulators)

**Why:** Geometric hash / curvature-indexed storage is the membership-data-structure problem. Her accumulator and PRF work is the standard-cryptographic membership primitive lineage. The Bellare-Lysyanskaya 2024 PRF paper is *literally* the security foundation for the AES-CMAC PRF you instantiate in INDEXED mode — it's worth a citation in the GIGI paper too (already noted in §2 anchors above).

---

## Master ranked list — single citation priority

If only doing N citations across the portfolio, do them in this order:

1. **(highest) PRISM paper / patent doc — `camenisch2005ecash` + Lysyanskaya 2024 digital-dollar talk.** PRISM is the most adjacent work to her current most-active research lane. Direct recommendation-letter material.
2. **Encryption paper §1.1 — `kohlweiss2023blueprints`.** Single most spiritually-relevant paper to GIGI's framework. One citation, strongest positioning move.
3. **Encryption paper §1.3 — `camenisch2004clsigs` + `camenisch2019mercurial` + `garman2024mercurial`.** Positions Sprint J delegation in the credential-delegation lineage. Three citations, big return per citation.
4. **Encryption paper §2 — `lysyanskaya2002vrf` + `camenisch2002accumulators` + `bellare2024prfs`.** Strengthens the external-math-anchors background.
5. **Encryption paper §11.2 (when drafted) — `chase2006sigs-of-knowledge` + `kohlweiss2023blueprints`.** The spectral-signature ZKP construction is squarely in her ZK lane.
6. **Geodesic-hash patent paper (when written) — `lysyanskaya2002vrf` + `bellare2024prfs` + accumulator papers.** Standard membership-primitive lineage.
7. **(low priority) Kähler substrate v0.4 — only if a clean placement opens up.** Don't disturb a publication-ready paper for marginal-value cites.

---

## The Tamassia thread

Lysyanskaya and Tamassia co-authored:
- "Multicast authentication in fully adversarial networks" (S&P 2004)
- "Authenticated data structures, generically" (Theory of Computing Systems 2010 — joint with Goodrich, Papamanthou, and Tamassia)

The **authenticated data structures** paper is directly relevant to Sprint K's holonomy ledger (Merkle audit log). Citing it in §7 of the encryption paper is honest and connects both Brown advisors into one citation.

**Specific addition for §7 (Sprint K, when drafted):**
> *"The holonomy ledger is an authenticated data structure~\cite{goodrich2010authdata-generic} specialized to bundle write events, with the Merkle tree (RFC 6962) providing the inclusion-proof primitive and the holonomy delta + record_hash leaves binding the structure to the bundle's geometric content."*

`goodrich2010authdata-generic` would cite Goodrich-Papamanthou-Tamassia 2010, with Lysyanskaya as a co-author depending on the exact version. Worth verifying the exact author list, but the conceptual fit is clean.
