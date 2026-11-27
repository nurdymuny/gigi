import H7CubicQuartic

set_option autoImplicit false

#check H7.zero_of_scaled_bound
#check H7.cubic_quartic_bound
#check H7.positive_constant_bound
#check H7.coercivity_factorization
#check H7.scaled_coercivity
#check H7.potential_lower_bound
#check H7.kappa_pos
#check H7.quadratic_coefficients

#print axioms H7.zero_of_scaled_bound
#print axioms H7.cubic_quartic_bound
#print axioms H7.positive_constant_bound
#print axioms H7.coercivity_factorization
#print axioms H7.scaled_coercivity
#print axioms H7.potential_lower_bound
#print axioms H7.kappa_pos
#print axioms H7.quadratic_coefficients

-- A halved coercivity constant fails at the exact scalar minimizer.
example : (3 / 2 : ℝ) ^ 4 / 2 - (3 / 2 : ℝ) ^ 3 + 27 / 64 < 0 := by
  norm_num

-- The stated constant is attained in the scalar problem, not necessarily by a triple.
example : (3 / 2 : ℝ) ^ 4 / 2 - (3 / 2 : ℝ) ^ 3 + 27 / 32 = 0 := by
  norm_num

-- The upper source-window boundary cannot be included in strict positivity.
example : H7.kappa 3 (5 / 4) = 0 := by
  norm_num [H7.kappa]

-- Pointwise domination alone, without homogeneity, does not give the conclusion.
example : (100 : ℝ) ≤ 1 * 0 + 100 ∧
    ¬ ((100 : ℝ) ≤ (1 + 100) * (0 : ℝ) ^ (3 / 4 : ℝ)) := by
  norm_num
