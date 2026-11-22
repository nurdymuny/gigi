"""POST HOC (after the S run): why S3a failed. Her price at s*(x_B, y_B):
lambda_j = (s x + (j+2)/sqrt t)^2 + 2 sqrt t s x - s^2 y^2. With x < 0 the square can be emptied by choosing
j + 2 ~ -s x sqrt t, so the minimum over ALL j behaves like -s^2 y^2 = -s^2 t, not s^2 (x^2 - y^2).
Prints the minimising family and the ratio for s up to 1000 (j unrestricted).
"""
import numpy as np
for t in (3.0, 3.55, 5.0):
    xb, yb = (2 - t) / np.sqrt(t), np.sqrt(t)
    for s in (1, 2, 5, 10, 100, 1000):
        js = np.arange(0, int(abs(s * xb) * np.sqrt(t)) + 50)
        lam = (s * xb + (js + 2) / np.sqrt(t)) ** 2 + 2 * np.sqrt(t) * s * xb - (s * yb) ** 2
        k = int(np.argmin(lam))
        print(f"t={t:4} s={s:5}: cheapest family j={js[k]:5d} (predicted ~{-s*xb*np.sqrt(t)-2:8.1f}), "
              f"lambda/s^2={lam[k]/s**2:+.4f}  (-y_B^2 = {-t:+.2f}; registered target (4-4t)/t = {(4-4*t)/t:+.4f})")
