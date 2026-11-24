# Literature check: what is classical, what is mine, and what I must answer

24 November 2026. **A search-based check (about a dozen targeted searches, abstracts and arXiv pages), not a full
reading.** Every citation below must be read and its conventions matched before it appears in a paper. Items marked
*verify* were not confirmed in the search.

## Classical, not mine

- **The decomposition.** On a three-manifold, grad CS = ∗F_A and Hess CS = ∗d_A, the twisted odd-signature operator.
  These are the basic set-up of instanton Floer theory: Floer, CMP 118 (1988); Taubes, J. Diff. Geom. 31 (1990);
  Donaldson, *Floer Homology Groups in Yang–Mills Theory* (CUP 2002), ch. 2; Himpel–Kirk–Lesch, math/0302234. The
  second variation d²YM(b) = |d_A b|² + ⟨F_A, [b∧b]⟩ is Bourguignon–Lawson, CMP 79 (1981). So "Q = |curl_A b|² + flux"
  is a three-dimensional repackaging of known facts. **My λ = μ² − B is an application of it to Helike, not a new
  decomposition.** I must check the factor 2 against my bracket convention.
- **The curl spectrum on Berger spheres.** Capoferri–Vassiliev, arXiv 2309.02015 (J. LMS 2026), Appendix D,
  Theorem D.1, crediting Gibbons. There are four families, reducing to ±n with multiplicity n² − 1 on the round
  sphere. arXiv 2607.03273 relabels the families. My homogeneous helicities ω_k and the twist's μ must be matched
  against this. The Hodge Laplacian on 1-forms of homogeneous 3-spheres is arXiv 2605.05406, with joint work with
  E. A. Lauret (*verify* authors). Curl on round spheres: Bär, arXiv 1702.02044.
- **The Cauchy–Riemann identity.** The scalar version, the CR structure and Kohn Laplacian on S³ (Landau levels of the
  Hopf fibration), is classical. My curl/divergence identity along θ̂¹ + iθ̂² is an adaptation to adjoint-valued
  1-forms. Present it as such.
- **The quantum superpotential.** The field-theory statement V = ½|grad CS|² and the exact non-normalisable state
  exp(±CS): Witten, gr-qc/0306083. Reduced Chern–Simons theories on S³ as supersymmetric QM: Bayram–Tekin,
  arXiv 2405.09473 and 2407.17943 (Chern–Simons, not Yang–Mills).
- **Colour–space locking.** "Spin from isospin" (Jackiw–Rebbi; Hasenfratz–'t Hooft 1976) and van Baal's O(3) × O(3)
  constant-mode analysis are the precedents. Hess W = I + L_c·L_s at the winding vacuum should be framed as a
  computation in that setting, not as a discovery.
- **The round-sphere matrix model and the two-family potential.** Balachandran–Vaidya–de Queiroz (1412.7900);
  Acharyya–Balachandran et al. (1606.08711, 1702.06430); Errasti Díez–Pandey–Vaidya (2001.10524, the N = 1 version);
  van Baal and Hari Dass, NPB 385 (1992), whose eqs. (54), (88), (92) and (101) my Phase 1–2 reproduce; Koller–van Baal.

## What I must answer: a prior 0⁺/2⁺ crossing

**Van den Heuvel, hep-lat/9604017** (with van Baal; see also hep-lat/9509019, 9608018 and the 1996 Leiden thesis)
reports a scalar–tensor (0⁺/2⁺) glueball crossing on S³ for f = g²(R)/2π² > 0.3, with m(2⁺)/m(0⁺) ≈ 1.5 near
f ≈ 0.25. They read it as a breakdown of the one-loop effective Hamiltonian, not physics. Before any claim about the
crossing near g² ≈ 1.76 in this model, I must:
1. map their f and their effective Hamiltonian to my g and my single-grain truncation;
2. decide whether my crossing is the same phenomenon, the same artefact, or neither. Mine is in a different
   truncation, at a different order, and with residual-controlled spectra, but that does not settle it.

## What looks open (not found in the search)

- An independent computation of the Yang–Mills Hessian at the Berger (Levi-Civita) field. The only hit was my own
  Zenodo record 24412603. Classical stability work is for round spheres and symmetric spaces (Bourguignon–Lawson;
  Laquer, Michigan Math. J. 31 (1984)). Recent work: Han–Wen 2601.17649; 2602.15666; 2609.24045 (stable Yang–Mills on
  round S³ are flat). The citation "kot1986" in *By Enough* could not be confirmed: *verify*.
- A glueball or low-lying-spectrum study on a squashed or Berger S³. Squashed-sphere gauge theory in the search was
  supersymmetric localisation (e.g. 1102.4716), not this. **The Berger probe experiment appears to be open.**
- The explicit statement that the twist is an exact eigenvector with the helicity² − flux eigenvalue, the flux
  theorem (Theorem 1 of the flux analysis), the top-rung uniqueness, and the floor sufficient condition (Theorem 3)
  were not found. They appear to be mine, built on classical ingredients.

## Read first

1. Capoferri–Vassiliev, 2309.02015, App. D.
2. Van den Heuvel, hep-lat/9604017, and the 1996 thesis.
3. Laquer (1984); Bourguignon–Lawson (1981).
4. Witten, gr-qc/0306083.
5. 2605.05406, and the Lauret joint work.
6. Donaldson (2002), ch. 2.

## A housekeeping note

The search found that Zenodo record 24412603 lists the affiliation as Brown University. My affiliation for this work
is Davis Geometric. I will fix it in the next version of that record.

## Second pass (later on 24 November): the Helike Ladder Theorem

A fuller check for *By Enough* v2 §4. Each item says what I read myself and what I took from a search.
- **Capoferri–Vassiliev, J. London Math. Soc. 113 (2026) e70431 (arXiv:2309.02015), Theorem D.1.** I read
  Appendices C–D (v6). Their Berger parameter is a = √t, and their frame is oppositely oriented to mine. With that
  dictionary my curl at the trivial connection reproduces all four series exactly, multiplicities included: every
  eigenvalue with |λ| < 5 at t = 0.6, 3, 7 (receipt `research/helike-ladder-20261124/ladder.json`). Helike's
  helicity at x = 0 is their Series I with n = j + 2. The first computation is Gibbons, Ann. Phys. 125 (1980) 98–116.
  The Bracchi–Capoferri–Vassiliev relabelling (arXiv:2607.03273) is equivalent.
- **Kumar–Lechtenfeld–Picanço Costa, NPB 973 (2021) 115583, eq. (3.28).** I read it. This is the closest precedent.
  On the round sphere, for A_a = ½(1+ψ)T_a, the extremal sector v = j+2 has
  ω² = 2(2j+3)ψ + 2(2j²+6j+5) = (2j+3+ψ)² − (ψ²−1). That equals my λ = h² − B at t = 1, x = y = 1+ψ, with my j twice
  theirs (exact symbolic identity, receipt). So the closed form on the round isotropic slice is theirs, in Casimir
  language. They have no helicity or flux reading, no squashing, and no floor question.
- **van den Heuvel, PLB 386 (1996), eq. (6).** Read by the search agent: the flat Hessian as a perfect square.
  Bourguignon–Lawson gives the curl² + flux form in one line. I found no statement of it at non-flat fields.
- **Kobayashi–Ohnita–Takeuchi, Math. Z. 193 (1986) 165–189, doi:10.1007/BF01174329.** The bibliographic data is
  confirmed; I have not read the content.
- **Folland, Trans. AMS 171 (1972) 83–133.** This is the standard citation for the tangential CR complex on spheres,
  the scalar analogue of the ladder's Cauchy–Riemann identity. Abstract only.
- **Not found:** the flux spectrum with B ≠ Θ, the leak = gauge identity, and the sufficient floor condition.
- **Still open:** a forward-citation search from KLC 2021 and from van Baal–Hari Dass 1992 (MathSciNet or Google
  Scholar, best done in a browser), and full texts of Bourguignon–Lawson, Taubes 1983 and Laquer 1984.
