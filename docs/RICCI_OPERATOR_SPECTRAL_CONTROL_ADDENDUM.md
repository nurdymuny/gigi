# RF2D spectral accounting addendum

Date: 2026-11-17. Written after the primary diagnostic, before this additional
measurement. It is post-diagnostic localization, not a new acceptance rule.

Use only the saved coarse round-control M and K, and its actual edge cochain
a. Solve K v_j = lambda_j M v_j with M-orthonormal columns v_j. Lambda is
pronounced lamb-duh. Let c_j=v_j^T M a. Then exactly, up to disclosed numerical
errors,

    a^T M a = sum c_j^2,
    ||(M^{-1}K-4I)a||_M^2 = sum (lambda_j-4)^2 c_j^2.

Report the spectral sum and a direct mass solve independently. Partition at
lambda=8, twice the planted continuum eigenvalue 4, fixed before this extra
measurement. Report BOTH the field-mass fraction and residual-mass fraction
above that partition. Do not choose a cut after seeing the spectrum, and do
not remove any modes or change the actual error being assessed.

TDD: a diagonal generalized eigenproblem with known coefficients must agree;
dropping the eigenvalue-squared amplification must fail that test. Hash the
coarse matrices, cochain, source, and result. This is a finite mechanism check,
not a statement about all modes of the nonlinear Berger Jacobian and not a
claim that an HHE polynomial filter has repaired BW05.
