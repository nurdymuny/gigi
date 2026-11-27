import H6WickTriangle

#print axioms H6Wick.odd_product
#print axioms H6Wick.odd_conjugation
#print axioms H6Wick.trace_conjugation
#print axioms H6Wick.odd_trace_zero
#print axioms H6Wick.triangle_zero

-- The scalar connected triangle control is nonzero.
example : (8 : ℝ) * (1/4) * (1/16) * (1/36) = 1/288 := by norm_num
example : (676 : ℝ) / 27 > 0 := by norm_num
