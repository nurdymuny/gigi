# Group transport: formal coverage

This companion checks conditional algebra for the
[larger-colour-space proof](../../docs/research/h8-group-transport-proof-2026-11-27.md).
It reuses the project's pinned Lean 4.36.0 and Mathlib revision without
changing H7 or H8 theorem sources.

## What the nine targets say

| Declaration in H8Group | Kernel-checked conclusion |
| --- | --- |
| gradient_intertwines | A linear map preserving the specified bracket maps the component vector field exactly, without a speed factor |
| cubic_transfer | Bracket and pairing preservation give the factor I in the cubic term |
| superpotential_transfer | All three quadratic terms and the cubic scale by I |
| plane_pairing | The repeated planar coordinate retains its factor two and its common factor I |
| plane_dissipation | Scaled superpotential decreases by I times the original squared gradient |
| scaled_saddle_height | The endpoint difference is Iu/2 |
| action_density_scale | Two square-root metric factors multiply to I |
| distance_squeeze | Given a global lower bound and arbitrarily close upper bounds, the distance equals Iu/2 |
| coupling_action | Conversion to original coordinates yields Iu/(2g^2) |

The pairing arguments B and C are explicit functions. The algebraic
identities require the stated preservation identities, but do not assert
that these functions are positive invariant inner products. That property
and the gradient interpretation are proved in the written note. Similarly,
brackets are explicit operations: this library does not classify Lie
algebras or construct root subalgebras. The final squeeze assumes both
distance estimates; it does not prove them by renaming the conclusion.

There are no custom axioms or admissions. The permitted foundational
dependencies are propext, Classical.choice and Quot.sound. Global orbit
existence, chain-rule integrals over ambient paths, endpoint connectors and
the existence theorem for embeddings are not end-to-end formalized.

## Gates and reproduction

From the repository root:

```text
python scripts/check_h8_group_transport.py <fresh-output-directory>
```

The checker runs exact matrix examples, builds the library, logs all
signatures and dependencies, compiles four exact verification examples,
and rejects four deliberately corrupted sources:

- bracket preservation replaced by True;
- I omitted from superpotential transport;
- the repeated coordinate's factor two removed;
- the endpoint action divided by four instead of two.

It also rejects poisoned dependency reports. Existing output directories
are refused: a failed attempt cannot overwrite a previous receipt. New
receipts start with passed=false and become true only after every gate.

Standalone compiler commands, from this directory:

```text
lake build
lake env lean H8GroupDeclarations.lean
lake env lean H8GroupVerification.lean
```

The necessary imports already belong to the H8 dependency cache. Neither
dependency revisions nor proof statements are relaxed to make tests pass.
The complete written proof and exact matrix checks have separate scopes;
a successful compiler run is not advertised as a formalization of all
compact simple groups or the Yang--Mills mass gap.
