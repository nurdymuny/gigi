# Where the competitor needs room: bounded support diagnostic

Registered before implementation and the new numerical run. This is a diagnostic, not a new acceptance rule.
The model, physical-domain contract and all existing thresholds remain unchanged. The full grid stays held.

## Frozen point and computation

Repeat only s=1.18, t=3, g^2=0.03, Mq=12, with the existing exact-moment operator and default solver settings.
The previous smoke receipt is the replay control. Gaps must reproduce within 1e-8 and reference weights within
1e-6. Keep eight Rayon threads and a 900-second process limit. Retain all coefficients needed for inspection.

Do not solve at Mq=14. Build that operator only to apply it once to the four already-computed trial states:
the ground state, the two largest Helike-reference-weight states, and the competitor selected by the existing
argmax rule. The four indices must be distinct. Selecting these for diagnosis does not certify their identities
or override the domain exporter's 0.95 rule. Save the selected weights and indices explicitly.

## Mathematics of the measurement

Let V_M be the existing total-occupation Hermite trial space in the log-Cholesky harmonic chart, and P_M its
orthogonal projector. No basis frequencies, coordinates, potential, measure or physical boundary are changed.
For a normalized computed state psi = sum_n c_n phi_n in V_12:

1. Occupation probability is p_n = |c_n|^2. Sum by total occupation and by the existing mode roles
   (h1,h2,c,x,s1,s2). These are chart-basis occupations, not measured physical particle counts.
2. Embed the same coefficients by occupation identity into V_14; do not rely on vector padding/order alone.
3. Compute r_14 = (H_14 - E) psi. Split it orthogonally into P_12 r_14 and (P_14-P_12) r_14.
4. Verify P_12 H_14 psi agrees with H_12 psi within 1e-9 * max(1,abs(E)) in Euclidean coefficient norm.
   This is a new diagnostic consistency check, not a relaxation of an existing threshold. All original
   finite-space residuals must still meet the requested 1e-10 * max(1,abs(E)) tolerance.
5. Report omitted-shell residual norms at N=13 and N=14, and their joint distribution over
   (N_H=n_h1+n_h2, N_c=n_c, N_x=n_x, N_S=n_s1+n_s2).

For any nonzero distribution a_n (probability or squared residual), report mean occupation in each of these
four groups. For omitted residuals also report sum_n a_n * N_group/N_total divided by sum_n a_n. These four
shares sum to one, but they allocate coupled coefficients by occupation count; they are not independent
causal contributions or geometric directions in physical space. Save individual coefficients and joint bins
so this summary cannot conceal coupling. A zero distribution has no normalized shares, not invented zeros.

Average the two Helike states' squared coefficient/residual distributions before comparing with the competitor.
The diagnostic must not mistake a sign flip or a rotation of the two-dimensional state basis for changed
support. Chart-axis occupations remain explicitly frame-dependent. No exact charge-invariance claim is made.

Only shells 13 and 14 are examined. The norm of r_14 is NOT the full-operator residual, a tail bound, or an
error bar on the exact gap. No rigorous degree-12 cutoff certificate is computed. The existing degree-10 cap
is untouched. No extrapolated gap, automatic basis enrichment, Mq=14 solve or grid release is permitted.

## Focused TDD gates

- Planted coefficients reproduce exact shell probabilities, joint bins and group means; an empty distribution
  has absent normalized summaries. Reject wrong dimensions, nonfinite/negative squared weights and bad roles.
- Occupation-key embedding survives a reordered target basis and refuses duplicate/missing keys.
- An analytic coupled finite matrix has a zero inner residual but a known nonzero omitted-shell residual;
  dropping omitted coefficients must fail a behavioral test. Norms obey the orthogonal split identity.
- Sign flips and orthogonal mixing of a two-vector subspace leave the averaged support unchanged.
- At both existing weak controls g^2=0.002 and 0.004, verify the nested-block identity using small Mq=2 and 4
  operators before the expensive diagnostic. These are operator-action tests, not additional control solves.
- Deliberate removal of squaring, identity-based embedding, outer-shell measurement or normalization must be
  caught. Capture the initial failing tests, passing focused tests, commands, logs and raw bit-preserving export.

## What would justify the next step

Use the observed joint bins and residual concentrations to propose a small, explicitly nested enrichment.
Do not choose that enrichment in advance and then call the observations support for it. A concentrated
residual can motivate a subsequent experiment; it does not prove that enrichment repairs the gap or the
identification refusal. If support is spread, say so. Stop after this diagnostic and report the measurements.
