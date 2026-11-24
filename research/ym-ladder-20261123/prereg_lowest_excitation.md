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
