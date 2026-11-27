import H8OriginSplitting

#print axioms H8Origin.e2_closed_form
#print axioms H8Origin.e2_negative
#print axioms H8Origin.charge_two_annihilated
#print axioms H8Origin.charge_two_cross_annihilated
#print axioms H8Origin.charge_zero_image
#print axioms H8Origin.ordering_of_quasimodes

-- A doubled factor is false at kappa = 1, t = 1.
example : ¬ ((-4 : ℝ) * 1 * (1 + 1 / 1) = -2 * 1 * (1 + 1 / 1)) := by norm_num
