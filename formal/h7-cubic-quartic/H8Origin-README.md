# H8 origin splitting: Lean scope

`H8OriginSplitting.lean` checks six declarations under the pinned toolchain and Mathlib revision of this project,
with no added axioms and no admitted steps:

| Declaration | What it checks |
| --- | --- |
| `e2_closed_form` | −(2κ/ω₁)(1/u + 1/ω₁) = −2κ(1 + 1/t) for ω₁ = √t, u = 1/√t, t > 0 |
| `e2_negative` | −2κ(1 + 1/t) < 0 for κ, t > 0 |
| `charge_two_annihilated` | 2⟨X₂,[X₃,X₁]⟩ − 2⟨X₃,[X₁,X₂]⟩ = 0, for any invariant symmetric form and bracket |
| `charge_two_cross_annihilated` | ⟨[X₃,X₁],X₃⟩ + ⟨X₂,[X₁,X₂]⟩ = 0, also using antisymmetry |
| `charge_zero_image` | −(2⟨X₂,[X₃,X₁]⟩ + 2⟨X₃,[X₁,X₂]⟩) = −4⟨X₁,[X₂,X₃]⟩ |
| `ordering_of_quasimodes` | two energies within K g³ of c + e₂g² and of c are ordered when K g < −e₂/2 |

The identities are stated for an arbitrary type with a bracket and a real bilinear form satisfying invariance,
symmetry and (for the cross term) antisymmetry. They are the algebra behind §3 and §4 of the written proof. The
gradient formulas for 𝒞 that turn them into ∇𝒞·∇ statements are not formalized.

**Outside the formal boundary:** polynomial differentiation and Laplacians, Gaussian moments and the Hermite
projection that produce e₂, Schur's lemma, non-degeneracy at 2u, the cutoff quasimode estimate, and the spectral
localization (Simon; Helffer–Sjöstrand). These are written in
[the proof note](../../docs/research/h8-origin-splitting-proof-2026-11-27.md) and exercised exactly by
`scripts/check_h8_origin_splitting.py`.

The checker rejects four broken variants:

- invariance removed from the cross identity;
- a doubled e₂ factor;
- the charge-0 quasimode bound removed;
- the window K g < −2e₂ in place of K g < −e₂/2.

Each must fail to compile for a mathematical reason, not a missing import.
