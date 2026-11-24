# Pre-registration: the field of the lowest excitation on SU(2) ladders

23 November 2026. Written and committed before any of the measurements below were computed. Companion to
*Yang-Mills Requirements as Geometric Objects* (`docs/research/yang-mills-geometric-identities-2026-11-23.md`,
Section 14) and to the lattice code `scripts/ks_su2_lattice.py`.

## Why

With Helike the number came first and was not the discovery. The twist was the lowest eigenvalue; what made it
Helike was reading the eigen*field*: what it is, where it lives, what organises it. When the data posed a problem I
did not stop at the number. I tried geometric readings one after another, kept each only if a written test survived,
and the break came when I found the right physical picture: the magnets. I am bringing the same method here.

This is Yang-Mills, and I am not going to close a door because of that. I am also not going to claim more than a
test supports. Both of those are kept by writing the readings and the tests down first.

## What is already known (not blind)

On open ladders of p = 1..5 plaquettes the gap is, at g² = 2: 3.42629, 3.35458, 3.31810, 3.30219, 3.29425; at
g² = 1: 3.26910, 3.06250, 2.95483, 2.90391, (2.89187, not converged in the spin cutoff). The decrements roughly halve.
A nearest-neighbour hopping band confined to p plaquettes, E₁(p) = ε − 2t·cos(π/(p+1)) with ε and t fixed by p = 1
and 2 (2t = 0.14341), fits p = 2 exactly and predicts gaps 0.00679, 0.00808, 0.00784 too high at p = 3, 4, 5
(g² = 2). Because I have seen these
numbers, nothing below predicts a gap value.

## The object

The lowest excited state ψ₁ of Kogut–Susskind SU(2) (lattice units), measured on:
- open ladders p = 2, 3, 4, 5 at g² = 2 (primary) and g² = 1 (secondary, with its cutoff caveat), at the largest
  cutoff used in Section 12 (link spins ≤ 5/2 for p ≤ 4, ≤ 2 for p = 5);
- periodic ladders (rings) p = 3, 4, 5 at g² = 2, at link spins ≤ 3/2 and ≤ 2 where the basis stays under 200,000
  states; a ring whose basis exceeds that at spin 3/2 is dropped, and the drop is reported. A ring has no ends: it
  is the control that separates an edge from a band.

## The measurements (all gauge invariant; none depends on intertwiner phase conventions)

- **M1, multiplicity** of the lowest excited level (eigenvalues equal to a relative 10⁻⁸).
- **M2, transition profile** A_k = ⟨ψ₁| tr U_k |ψ₀⟩, k = 1..p: how strongly each plaquette's loop operator creates the
  excitation from the vacuum. If the level is degenerate, the profile is computed from the spectral projector
  (Σ over the level of |A_k|²), never from one eigenvector.
- **M3, energy profile** e_k = ⟨ψ₁|h_k|ψ₁⟩ − ⟨ψ₀|h_k|ψ₀⟩, with h_k the plaquette's magnetic term plus the electric
  energy of its links, a shared rung split equally between its two plaquettes. Σ_k e_k equals the gap identically.
- **M4, reflection parity** of the profile under k ↦ p + 1 − k (even, odd, or neither).
- **M5, spin content**: the weight of ψ₁ by the number of excited links and by the largest link spin.

## Three readings and what each predicts

- **Band.** One plaquette-loop excitation hopping between neighbours, confined by the ends. Profile even and
  node-free, close to sin(πk/(p+1)); energy spread like the profile; on a ring the profile is uniform and the ring gap
  lies *below* the open gap at the same p (the band bottom ε − 2t versus ε − 2t·cos(π/(p+1))).
- **Edge.** The excitation is bound at the ends. Profile peaked at k = 1 and k = p, falling into the middle; the open
  gap lies *below* the ring gap (the ends bind it).
- **Band with distinct ends.** A band whose end plaquettes have their own energy, because an end plaquette has one
  neighbour and an unshared rung. Profile node-free but with end weight off the sine's; the ring gap below the open
  gap, by a different amount than the plain band predicts.

I do not have a favourite. The ring comparison is the sharpest single test: it separates Edge from both band
readings by its sign.

## Scoring (fixed now)

For each open ladder p = 3, 4, 5 at g² = 2, with S_k = sin(πk/(p+1)):
- C = cosine similarity of |A| with S; W = (A₁² + A_p²)/Σ A²; W_S the same for S.
- **Band** if C ≥ 0.99 and |W − W_S| ≤ 0.05, and every ring measured has gap below the open gap at the same p.
- **Edge** if W ≥ W_S + 0.15 and every ring measured has gap above the open gap.
- **Band with distinct ends** if the profile is node-free (all A_k of one sign) and the ring gaps are below the open
  ones, but Band's profile criteria fail.
- **None of these** otherwise, reported as found.

A reading is reported as holding only if it holds at every p scored; g² = 1 is reported beside it, not scored.

## Gates (planted failures the analysis must catch before its verdicts count)

- **G1, planted band**: a free tight-binding chain of p sites (ε, t) analysed by the same code must score Band.
- **G2, planted edge**: the same chain with an attractive end potential strong enough to bind a state must score
  Edge.
- **G3, mode fidelity**: if any measured level is degenerate, the projector-based profile must be unchanged by a
  random rotation inside the level, while single-eigenvector profiles change.
- **G4, bookkeeping**: Σ_k e_k = gap to 10⁻¹⁰ on every lattice.
- **G5, cutoff**: profiles at the two largest cutoffs agree (cosine ≥ 0.999); a lattice that fails is reported as not
  converged and is not scored.

## What happens next, whatever the verdict

- If **Edge** or **Band with distinct ends** holds, the next question is what the end does geometrically, an unshared
  rung, a missing neighbour, a boundary condition, and I will test those readings one at a time, each written down
  first, as with Helike.
- If **Band** holds, the excitation is a particle on the strip, and the next measurement is its effective mass and
  how it depends on g².
- If **none** holds, the profiles are reported as they are, and that is the problem to work on.

Amendments after any data is seen will be listed here with what had been seen when they were made. These runs wait
until Test G (`research/edge-theorem-20261122-g/`) finishes, so the two do not compete for the machine.

## Amendment 1 (23 November 2026, after the adversarial review; before any measurement)

**What had been seen when this was written.** None of the measurements above. Known in addition to the original
registration: the adversarial review's electric-only control on the three-plaquette ring at link spins ≤ 1/2 (16
physical basis states, 8 in each winding sector; no plaquette matrix element between sectors; least odd-sector
electric energy 9g²/8 against the least nonzero even-sector energy 3g²/2). Those are exact Casimir sums for the
represented states, not finite-coupling ring gaps. The original registration above is kept unchanged.

**A1.1 Winding sectors (review C1).** A ring has a noncontractible cycle. Cutting it through one top and one bottom
horizontal link defines z(s) = (−1)^(2 j_top + 2 j_bottom), which every plaquette operator preserves. A ring state in
the other sector cannot be created from the vacuum by any plaquette operator, so its transition amplitudes all vanish,
and an open-versus-ring gap ordering alone would not separate an edge from a band. Therefore:
- every ring state is labelled by its winding sector (and, where computed, its reflection sector);
- the **absolute gap** (lowest excited state of the whole ring) and the **probe-accessible gap** (lowest excited state
  in the vacuum's sector with nonzero plaquette transition strength) are reported separately;
- the ring comparison of the Scoring section uses the probe-accessible gap, sector-matched to the open ladder;
- a verdict whose definition depends on rings is scored only if at least one valid, sector-matched ring comparison
  was made; "every ring measured" is not satisfied by measuring none.

**A1.2 The transition kernel replaces diagonal intensities (review C2).** For the excited level with projector P (rank
r) and O_k = tr U_k, the measured object is the kernel C_kl = ⟨O_k Ω, P O_l Ω⟩. Its diagonal I_k = C_kk is the
intensity. In the Scoring section, |A_k| is replaced by the envelope √I_k, and W by (I₁ + I_p)/Σ I_k (for a three-site
sine amplitude this is 1/2; squaring intensities again would give 1/3). Signs and parity are read from the kernel, not
from the diagonal: for a rank-one real level, the relative sign of plaquettes k and l is the sign of C_kl; "node-free"
means every C_1k has one sign. For a degenerate level, parity is read from the action of the reflection on the excited
subspace. The energy profile of a level of rank r is e_k = Tr(P h_k)/r − ⟨Ω, h_k Ω⟩; for a numerically grouped but
split cluster this is the cluster's mean excitation energy, reported with the cluster's energy spread.

**A1.3 What the profile is (review C3).** ψ₁/ψ₀ lives on the gauge configuration space. Because ψ₁ is orthogonal to
the positive ψ₀, the ratio has zero weighted mean and must change sign. The plaquette profile is a spatial response
extracted from it, not the whole field; a node-free envelope is a different object from a node-free ratio, and the
results will be described that way. The comparison with Helike (Section 14, item 2 of the note) needs the same
geometric space: the meshes discretise S³/2I, not S³, and a uniform ladder Hamiltonian carries no Berger metric. That
experiment needs its own specification (quotient or cover, metric weights, gauge sector, normalisation, and a map from
classical tangent fields to quantum observables) before any comparison of charge or shape.

**A1.4 Added gates (all must be caught before any verdict counts).**
- **G6, inaccessible winding state**: a planted ring spectrum with a lower state in the odd winding sector must be
  reported as "probe does not see this level", not classified as an edge or a band.
- **G7, zero overlap**: a level with zero transition strength returns "probe does not see this level", never a
  normalised profile.
- **G8, kernel formulas**: the three-site sine gives W = 1/2; the profiles (1, 0, 1) and (1, 0, −1), identical in
  intensity, are resolved as even and odd from the kernel.
- **G9, planted doublet**: a degenerate pair is rotated by a random unitary inside its span; the kernel, the energy
  profile and the parity verdict are unchanged, while single-eigenvector profiles change. This gate runs even if the
  real spectra have no degeneracy.
- **G10, gaps converged too**: G5's cutoff check applies to the gaps compared (open, ring, absolute and
  probe-accessible), not only to profiles; a stable profile can coexist with a poorly converged energy.
