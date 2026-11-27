import H8Interwell

#print axioms H8.metric_pairing
#print axioms H8.dissipation
#print axioms H8.path_derivative
#print axioms H8.rectangle_boundaries
#print axioms H8.interior_descent
#print axioms H8.equilibria
#print axioms H8.saddle_height
#print axioms H8.unstable_root
#print axioms H8.unstable_direction
#print axioms H8.rectangle_ceiling
#print axioms H8.decay_direction
#print axioms H8.weighted_cauchy
#print axioms H8.straight_excess
#print axioms H8.coupling_scaling

-- Without y < 1 the strict ceiling need not hold.
example : ¬ (max (1 / 2 : ℝ) (1 * 2 ^ 2) < 1) := by norm_num

-- The ordinary coordinate gradient doubles the repeated-coordinate velocity.
example : 2 * H8.fy 1 (1 / 2) (1 / 2) ≠ H8.fy 1 (1 / 2) (1 / 2) := by
  norm_num [H8.fy]

-- The straight-line excess vanishes at the round sphere, and not at u = 2.
example : ((1 : ℝ) ^ 2 - 1) ^ 2 / 18 = 0 := by norm_num
example : ((2 : ℝ) ^ 2 - 1) ^ 2 / 18 = 1 / 2 := by norm_num
