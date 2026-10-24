# Research relationship registry: decisions backed by stored evidence

This is a Python application over GIGI's ordinary record APIs. It keeps a claim,
its sources, its evidence level (called *altitude*), and reviewer decisions
separate. A shared word does not establish a shared mechanism. Following links
does not prove that the linked ideas compose.

## Reproduce the checks

Requirements: Python 3.11 or later, the repository's Rust toolchain, and a native
build environment for GIGI. The Python checks use only the standard library.
Run these commands from the repository root:

```powershell
python -m unittest discover -s e2e/probes/registry -p test_registry.py -v
python e2e/probes/registry/verify_registry_mutations.py
cargo test
cargo build --bin gigi-stream
python e2e/probes/registry/verify_registry_live.py --engine target/debug/gigi-stream.exe
```

On Linux/macOS, use `target/debug/gigi-stream` for the last argument. A release
build is also supported: build with `cargo build --release --bin gigi-stream`
and pass the resulting executable. Build the binary explicitly: `cargo test`
alone does not establish that the server binary is current. If an existing
engine holds that executable open on Windows, use another Cargo target directory
instead of terminating someone else's server.

The test layers answer different questions:

1. **27 deterministic tests** exercise actual decisions and stored state using
   an in-memory HTTP substitute, including write rejection and success responses
   that perform no write. They do not establish engine persistence.
2. **12 removed-safeguard controls** temporarily replace a safeguard with broken
   behavior and require its regression test to fail an assertion. Test crashes
   do not count as detecting a defect. No source files are modified.
3. **The live harness** starts its own engine on a free `127.0.0.1` port with a
   fresh data directory. It runs the 21 acceptance gates, plants missing-source
   and blank-locator cases, checks failed and no-op writes, and verifies symbolic
   comparisons. Two separate Python processes with different hash seeds replace
   the same path ruling. It then kills only its child engine, restarts against
   the same data, and compares all eight bundles exactly. Every operation after
   restart is read-only, including checking the promotion, replacement ruling,
   and retraction warnings.

The harness prints its evidence directory under ignored
`outputs/registry-verification-*/`. It retains before/after JSON snapshots,
engine logs, and a receipt containing the executable's SHA-256, modification
time, Python version, endpoint, and final pass status. It stops its engine on
success or failure. A failed run leaves evidence for diagnosis; running again
creates a fresh directory. These are synthetic fixtures, not approved research
records. The GitHub CI SDK job runs these checks and retains their receipts.

For manual work, `GIGI_REGISTRY_URL` selects the server (default
`http://127.0.0.1:3143`). `registry_bootstrap.py` seeds the fixture;
`registry_gates.py` mutates it. Use a disposable instance. Gates expect fresh
state and must not be rerun as a restart test. In particular, the supported
relation starts at `NONE`, and promotion must actually change it to `RESONANCE`.

## What the fixes enforce

**Promotion and writes.** Supporting evidence must name an existing source with
recognized availability and a nonblank locator. `RESONANCE` needs qualifying
evidence on both sides. Missing/retracted sources and blank locators cannot earn
it. `unavailable` describes access, so it does not invalidate evidence.
`STRUCTURAL` additionally requires scope, qualifying source evidence, and a
stored passing check whose negative control ran and failed as required. This
checks recorded evidence, not the truth of an arbitrary claimed proof.
Both promotions and reviewer rulings check the HTTP response and then read back
every written field. A refusal or a success response with no stored update
cannot be reported as success. An unacknowledged write can still have reached
storage; callers should inspect state before retrying after transport failure.

**Retraction during retrieval.** Direct views and path edges carry source
availability, locators, warnings, and a structured `needs_review` flag. Paths
flag whether any edge needs review without inventing an aggregate altitude.
Missing evidence is explicitly represented. Stored altitude is retained and
marked as no longer earned where appropriate; automatic recomputation remains
unimplemented. A saved composition ruling remains the reviewer's stored ruling,
with evidence warnings alongside it.

**Composition rulings.** Only `compatible_under` and `incompatible` are accepted.
A ruling requires a nonblank reason and reviewer name; `compatible_under` also
requires explicit conditions. The ordered relation IDs must exist, be distinct,
and form a contiguous path respecting directed/symmetric edges. Caller-supplied
reviewer identity and agent status are declarations, not authentication.

The record ID is `PR-` followed by the SHA-256 of the ordered path signature.
A successful write explicitly **replaces the current ruling** at that key;
there is no review history or compare-and-swap concurrency protection. This is
stable across Python processes. For older random-ID records, a single valid
legacy ruling can still be read. Multiple legacy records cause an explicit
ambiguity error. A newly recorded canonical ruling supersedes those legacy rows
without deleting them. The code never guesses chronology from query order.
Changing the relations themselves does not version or invalidate a ruling
automatically; re-review changed paths before relying on an old decision.

**Bounded contradiction detection.** Four coordinates must match exactly:
quantity, object, domain, and conditions. `value_kind="numeric"` denotes an
**exact asserted number**, compared as a finite decimal (`1` equals `1.0`). It
does not represent an estimate, confidence interval, or tolerance. Units must be
consistent in the quantity definition. `value_kind="invariance"` supports only
the explicit mutually exclusive values `invariant` and `changes`. This identifies
incompatible assertions; it does not decide which source is correct.

Expressions such as `r` and `1*r`, unknown/missing value kinds, invalid numeric
values, and different contexts go to `possible_conflicts()` for review. They are
never declared mathematical contradictions merely because strings differ. The
original `r` versus `1` fixture is now a review item. The numeric positive
control compares the exact assertions `2` versus `1` at `r=2`; the rotation
property fixture supplies a separate positive control.

## Existing datasets and remaining boundaries

The claim schema now includes `value_kind`. The bootstrap creates a fresh schema;
it is not a production migration. Existing untyped claims remain readable and
are conservatively routed to review until explicitly typed. Back up and migrate
existing datasets deliberately; do not bootstrap over them.

The restart check also exposed a snapshot encoder defect: numeric-looking text
such as `"1.0"` reloaded as a JSON number. The accompanying DHOOM encoder fix uses
the existing escaped-string representation to preserve exact text, including
numeric spellings, boolean/null-like strings, and literal escape sentinels. Two
Rust regressions exercise direct encoding plus heap/mmap snapshot reload; both
failed before the fix. This protects newly written snapshots. It cannot recover
spelling or precision already lost by an old snapshot; non-text claim values are
routed to review, not silently converted back and called exact.

Queries refuse malformed success responses and results that reach their limit
(1,000 by default), because completeness cannot then be established. This is
bounded fixture tooling, not a paginated large-registry service. Evidence reads
and writes are not one transaction, and concurrent changes need application-level
coordination. Locators are checked for presence, not fetched or authenticated.
Restricted-source metadata is identified but per-request access control is not
implemented. Executable maps, mathematical theorem proving, automatic altitude
recomputation, and semantic equivalence of symbolic expressions remain outside
this implementation.
