# Ricci / Davis-Wilson research checkpoint

Branch: `codex/feat-ricci-correspondence-checkpoint`.
Base commit: `0a64e2f848693221bf25f330ea4fb19041249106`.
Date: 2026-11-17. **Progress preservation, not a release or convergence claim.**

## Start here

[Handoff](../../docs/RICCI_HANDOFF_2026_11_17.md) contains the
current question, exact results, constraints, code/data locations and next
steps. [Latest boundary result](../../docs/RICCI_BOUNDARY_EXTENSION_RESULTS.md)
explains the verified incompatibility witness. [Historical handoff](../../docs/RICCI_HANDOFF.md)
records the earlier attempts and why they should not be repeated.

## What this branch preserves

- Native `MONODROMY_GROUP_ORDER`, `RICCI_FLOW`, `RICCI_WILSON_AUDIT`, their
  parser/HTTP wiring, focused contracts, native lattice/operator probes.
- Independent numerical oracles, mesh/refinement studies, consistency audits,
  closed-curvature recovery, local push attribution and shared-face checks.
- Specs, results, RED/GREEN receipts, mechanism removals, adversarial evidence
  checks, raw arrays, query captures, diagnostic logs and scientific figures.
- The successful six-cell shared-curvature construction and the subsequent
  failed fixed-exterior boundary lift, both with their actual conclusions.

Only relevant native integration hunks were staged. Unrelated changes to
statistics, storage/WAL, analytic objects, HHE, licensing and other work remain
in the original local worktree; this branch does not promote those changes.

## Scientific status

The declared homogeneous SU(2) continuum result and local constructions have
their documented validation. The full lattice correspondence is NOT established.
The local common quadratic preserves19 edge integrals and18 face fluxes, with
internal agreement around1e-14, but has a3.738 boundary mismatch to unchanged
outside fields. Those exterior traces conflict with each other: the strongest
same-edge witness forces error>=0.08387887804869132, versus1e-9 acceptance.

Latest focused gate:15 tests, six caught mechanism removals, seven rejected
evidence attacks. Evidence verification is true; boundary-extension acceptance
is false. BW05 remains OPEN. Thresholds, measured inputs and mapping unchanged.
No full suite or new native build was run for this packaging commit. Historical
native receipts describe their recorded source/binary, not a newly built
binary from the curated branch. No deployment or merge to main is requested.

## Evidence archives

`MANIFEST.json` lists every archive and entry with SHA256 and byte length.
There are30 archives containing1015 entries, totaling336.32MiB compressed.
Archives are split below GitHub's single-file limit. They preserve exact bytes,
including unsuccessful experiments, not merely selected passing summaries.

The per-run archives recreate paths under `output/`. The `source-snapshot.zip`
archive preserves current source versions referenced by receipts, plus shared
integration files. These are exact mixed-worktree provenance snapshots and can
contain unrelated changes in shared files. They are NOT applied to active
branch code. Older RED receipts may name still older versions; the snapshot
does not pretend to recover historical source bytes that were unavailable.

Compiled executables/bytecode are excluded and listed with their original
hashes in the manifest. No scientific data, arrays or receipt logs were excluded
for being a failed result. Rebuild binaries rather than trusting captured
executables. A credential-signature scan found no matches in the scoped
evidence JSON/log/Python files; that is not a universal secret-audit claim.

Verify this package without rerunning research:

```powershell
python research/ricci-checkpoint-20261117/package_checkpoint.py verify
```

On a separate fresh checkout, evidence archives may be extracted into that
checkout to restore their `output/` paths. Inspect the manifest first. Do not
overwrite an existing experiment, and do not extract `source-snapshot.zip`
over an active worktree: inspect it in a separate empty directory.

## Historical replay caveat

Receipts are deliberately unchanged. Several record absolute Windows paths,
exact mixed-worktree source hashes, and the PRE-CHECKPOINT base HEAD. After this
commit, naive replay can reject the changed HEAD, curated shared source files,
or relocated paths even when the numerical evidence itself is intact. This
is a provenance mismatch, not a new mathematical failure. The original local
arrays and source files remain available on Bee's machine.

Do not rewrite old receipts or disable their integrity checks to manufacture a
PASS. Use the archived source versions and declared original revision/context
for historical inspection, or create a separately named, transparently rebased
replay with its own provenance. The package verifier above is independent of
the old absolute paths and checks the committed retained bytes directly.

## Next work

Claude should assess the fixed-exterior obstruction first. A new proposal
must address compatible neighboring reconstructions, the new outer boundary,
and the complete energy derivative in the ORIGINAL Whitney test norm. A larger
polynomial inside the same patch cannot reconcile conflicting fixed traces.
Keep attempts bounded, spec first, strict focused TDD. No gate is closed by
this checkpoint commit.
