import H8GroupTransport

#print axioms H8Group.gradient_intertwines
#print axioms H8Group.cubic_transfer
#print axioms H8Group.superpotential_transfer
#print axioms H8Group.plane_pairing
#print axioms H8Group.plane_dissipation
#print axioms H8Group.scaled_saddle_height
#print axioms H8Group.action_density_scale
#print axioms H8Group.distance_squeeze
#print axioms H8Group.coupling_action

example : (4 : ℝ) * (1 / 2) = 2 := by norm_num
example : ¬ (4 : ℝ) * (1 / 2) = 1 / 2 := by norm_num
example : ¬ (4 : ℝ) * (1 / 2) = Real.sqrt 4 * (1 / 2) := by
  intro h
  nlinarith [Real.sq_sqrt (show (0 : ℝ) ≤ 4 by norm_num)]

-- Explicitly reapply the conditional squeeze without pretending to prove its hypotheses.
example (d I u : ℝ) (hlo : I * u / 2 ≤ d)
    (hhi : ∀ ε : ℝ, 0 < ε → d ≤ I * u / 2 + ε) : d = I * u / 2 :=
  H8Group.distance_squeeze d I u hlo hhi
