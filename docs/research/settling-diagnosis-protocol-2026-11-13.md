# Settling disagreement diagnosis: frozen controls

Investigate rc1, rc3 and rc4 first switches; retain rc2 as the successful
control and report all six traces. No engine, prior threshold, prior report or
component value changes. Use the frozen CSV bytes and verify their SHA256s.

1. Recheck original model against its elementary exponential oracle. Then use
   measured input with the same nominal R/C and the same initial measured output.
   No later output is used to initialize, fit or shift a prediction.
2. Compute three explicit input reconstructions: left-hold, right-hold and linear
   interpolation between the measured input samples. They are alternative
   assumptions about unobserved intersample input, not additional measurements.
   Check the linear solution against a separately formed augmented-matrix
   exponential. Replay left-hold on the unchanged native CONDUCT endpoint in
   <=64-phase blocks, carrying only predicted state between blocks.
3. At the original comparison timestamps and original settling thresholds,
   decompose error exactly: observed-minus-original = measured-input-model-minus-
   original + observed-minus-measured-input-model. Report signed contributions
   at the original worst departure, and residual RMSE/max after the old bound.
   A smaller residual is diagnostic evidence, not proof of a physical cause.
4. Independently calculate a conservative interval enclosure with tau in
   [0.95*0.90,1.05*1.10]*R*C and input bounded by the two adjacent measured samples.
   Positive scalar dynamics permits propagation of endpoint extrema. This even
   allows tau to vary within that range between intervals, so an observation
   outside the enclosure excludes nominal-component tolerances alone under
   these input/initial-state assumptions. It does NOT exclude unknown voltage
   calibration error or unobserved input overshoot. No fitted parameter is used.
5. Repeat intervals with time origin shifted, to ensure units/order matter but
   absolute clock origin does not. Test known constant/ramp input analytically,
   signed decomposition and bounds before running real traces. Deliberately
   remove source forcing and tolerance widening; require behavioral failures.
6. Preserve identifiability limits: voltage obeys tau*v'=u-v with tau=R*C.
   (R,C) and (a*R,C/a) are indistinguishable from these observations. Additional
   resistance, capacitance, probe effects, calibration and channel interpretation
   cannot be uniquely assigned without independent hardware evidence.

Evidence labels: engine arithmetic checked for these circuits; measured-input
counterfactual effect quantified; remaining mismatch quantified; physical cause
not identifiable where independent evidence is absent. Do not call any
post-hoc diagnostic a held-out hardware validation. Choose the smallest proposed
fix only after these controls. Page examples must distinguish measured results
from planned domain studies; no deployment is authorized.
