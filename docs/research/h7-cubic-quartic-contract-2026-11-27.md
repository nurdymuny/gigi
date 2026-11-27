# H7 cubic-quartic closure: bounded proof contract

27 November 2026. Baseline: `1ba75baa762aeab4ca580c6eabe1354a86cb6af3`.

## Target and scope

Close only Sub-conjecture A_g in `closed-set-20261127/H7-every-group.md`:
for each fixed compact simple real Lie algebra, with a fixed positive
Ad-invariant inner product, prove |C| <= c_g Q^(3/4) for all triples.
C = <X1,[X2,X3]> and Q is the sum of the three unordered squared bracket norms.
The desired consequence is coercivity and compact resolvent for the existing
homogeneous reduced Hamiltonian, at fixed g > 0 in the existing positive-kappa
window. No simulation, eigenvalue calculation, model change or threshold change.

The proof dependency is Austing--Wheater, *Adding a Myers Term to the IIB
Matrix Model*, JHEP 11 (2003) 009, arXiv:hep-th/0310170v2, equation (2.2).
Use its global algebraic bound, not a claim about convergence of a D=3 matrix
integral. The scaling corollary does not require that integral to converge.

## Required proof steps

1. State compactness, positive invariant norm, fixed group, and the trace
   convention. Translate anti-Hermitian Lie-algebra matrices to Hermitian
   matrices; retain the factor of i, trace scale and unordered-pair convention.
2. State the cited global bound with constants independent of the triple.
   Do not attempt to infer uniformity from ordinary polynomial degrees or
   compactness of Q = 1: that level set is not compact.
3. Use homogeneity with r = Q^(-1/4) only when Q > 0. Treat Q = 0 separately.
4. Prove the potential lower bound with an exact nonnegative factorization.
   State that the algebraic constant is not claimed optimal for this group.
5. Specify the natural full-space realization and its invariant sector
   restrictions. Explain compactness of the form embedding. Do not choose
   new boundary conditions at quotient singularities.
6. Update the register before strengthening the narrative paper. H7 itself,
   the fold, the carrier ordering and continuum hypotheses stay open.

## Focused checks, written before the proof edit

- Exact SU(3) matrix fixtures check the trace convention, cyclic pairing,
  Hermitian sign, unordered pairs and the one-half in the epsilon definition.
- Symbolic identities check cubic/quartic homogeneity, rescaling, norm
  rescaling, the coercivity factorization and the unchanged kappa window.
- A non-regular commuting limit is checked symbolically as an illustration,
  not used as evidence for a universal bound.
- Negative controls remove a bracket, reverse the trace sign, change the
  exponent, halve the coercivity constant, and promote the whole H7 claim.
  Each must be rejected by the corresponding check.
- Documentation checks must initially fail for the missing proof and stale
  pending-lemma text. They check presentation and scope, not proof validity.
- Receipts go to new directories; existing results cannot be overwritten.
  A failing check records failure and returns a nonzero exit code.
- Existing focused algebra, preservation and doublet checks run after the
  edit. Preservation allows only the named new statement, one exact H7
  evidence-text replacement and the exact new bibliography entry.
- Rebuild until references stabilize and inspect the PDF, especially the new
  proof, status table and bibliography. No full engine test suite is needed
  for this mathematical/editorial change, and none is claimed to have run.

## Acceptance

The written argument, with its cited analytic dependency, closes the
sub-conjecture. Python checks only the finite algebra and document contract;
it does not prove Austing--Wheater's inequality or compact-resolvent theory.
Record red and final receipts, deliberate bad-fixture rejections, source
hashes and the rebuilt PDF hash. Retain previous receipts as history.
