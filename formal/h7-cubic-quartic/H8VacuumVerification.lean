import H8VacuumFloor

#print axioms H8Vacuum.factorization_jet
#print axioms H8Vacuum.squares_zero
#print axioms H8Vacuum.cone_tail_identity
#print axioms H8Vacuum.cone_tail_bound
#print axioms H8Vacuum.strict_floor_of_attained
#print axioms H8Vacuum.sector_floor
#print axioms H8Vacuum.gaussian_jet

-- Gaussian equality is incompatible with a universal strict-floor claim.
example : ¬ (3 : ℝ) < 3 := by norm_num

-- At the exact cone threshold, a factor of one instead of one-half is false.
example : ¬ ((1 : ℝ)*2^2 - 1*1*2^3 ≤ -(1*1*2^3)) := by norm_num

-- Reapply the conditional theorem with every analytic input explicit.
example {S : Type*} (admissible kernel : S → Prop) (energy : S → ℝ)
    (floor bottom : ℝ)
    (hlo : ∀ x, admissible x → floor ≤ energy x)
    (hatt : ∃ x, admissible x ∧ energy x = bottom)
    (heq : ∀ x, admissible x → energy x = floor → kernel x)
    (hno : ∀ x, admissible x → ¬ kernel x) : floor < bottom :=
  H8Vacuum.strict_floor_of_attained admissible kernel energy floor bottom hlo hatt heq hno
