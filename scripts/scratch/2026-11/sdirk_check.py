"""Scratch check of the SDIRK2 stage formulas on the scalar test y' = lam y (vector-space form)."""
import numpy as np
g = 1 - 1/np.sqrt(2)
for lam in (-1.0, -1e6, 1j):
    for h in (0.1, 0.05):
        # stage 1: O1 = g h lam (1 + O1); stage 2: O2 = (1-g)/g O1 + g h lam (1 + O2)  (linear, y = 1 + O)
        O1 = g*h*lam/(1 - g*h*lam)
        O2 = ((1-g)/g*O1 + g*h*lam)/(1 - g*h*lam)
        R = 1 + O2
        print(lam, h, abs(R - np.exp(lam*h)) / h**3 if abs(lam) < 10 else abs(R))
