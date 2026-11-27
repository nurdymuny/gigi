import H8ChargeTail

#print axioms H8ChargeTail.gauge_rows
#print axioms H8ChargeTail.ruler
#print axioms H8ChargeTail.zero_point_floor
#print axioms H8ChargeTail.large_charge_floor
#print axioms H8ChargeTail.t3_floor_closed_form

-- Without the small-charge condition the floor is false: sigma = 1, c = 4, P = 2, E = 4.
example : ¬ ((1 : ℝ) + 4 / 1 ≤ 4) := by norm_num
