# Reply to GGOG team — protocol audit, G1 finding

**To:** GGOG / app team
**From:** GIGI fiber team (Bee + Claude)
**Date:** 2026-06-07
**Re:** Your `LETTER_TO_GIGI_TEAM_protocol_audit_2026-06-03.md` — triage + first finding
**Working evidence:** this letter, at `gigi/theory/ggog/LETTER_TO_GGOG_2026-06-07_G1_REPLY.md`

## TL;DR

Read your audit letter end-to-end. Triaged the 8 cross-repo items by responsible repo. Did a read-only audit of one substantive claim from your ask #2 (signed projections) and have a load-bearing finding to share before any of us touches code.

**Headline: the signed-projection invariant you're asking us to prove ALREADY HOLDS end-to-end on the wire.** No format change needed. The strict-verify discipline your audit wants is well-founded against the current `gigi-wasm`. You can roll out `opened.projection === '<expected>'` assertions on the existing projection types today without waiting on us.

Details, proof trace, and a small follow-up test we'd like you to apply, below.

## Triage by repo

We mapped your 8 items to who owns what code:

| # | Item | Owner |
|---|---|---|
| 1 | Register challenge → Ed25519 sig | `ggog-core` + `ggog-app` (you) |
| 2 | Signed projections | `gigi-wasm` (you) + `ggog-app` (you) |
| 3 | Signed `psi_transfer` | `gigi-wasm` (you) + `ggog-core` (you) |
| 4 | `SocketAddr` → `IpAddr` | `ggog-core` (you) |
| 6 | Multi-VM startup assertion | `ggog-core` (you) |
| 7 | Bounded mpsc + broadcast cap + idle reaping | `ggog-core` (you) |
| 9 | DM AAD binding | `gigi-wasm` (you) |
| S9 | Redaction reach for offline peers | `ggog-core` (you) |

Every item lands in code your team owns — `ggog-core`, `ggog-app`, or `gigi-wasm`. Our role here is **substrate-side audit + answers**, not commits. We'll send a reply letter per substantive item; you apply the code on your side.

(We had a brief moment of trying to land a patch directly in your `gigi-wasm` tree before realizing the boundary. Reverted. Going forward we treat anything under `~/Documents/ggog/` as off-limits to direct edits.)

## G1 finding: cross-projection replay resistance is ALREADY guaranteed

You asked, in item #2, that the substrate make it impossible for an attacker to take a signed `Reaction` bundle and replay it as a signed `Comment` (or any other projection). We traced the question through 5 hops of `gigi-wasm` source.

### The 5-hop proof trace

**Hop 1.** `create_reaction` (lib.rs:1363) sets `projection: bundle::Projection::Reaction` as a field on the `Fiber` struct, then calls `sign_bundle(&mut b, &state.identity)`. Same pattern in `create_redaction`, `create_collectible`, `create_follow`, `create_unfollow`, `create_voice_note`, `create_video`, `create_profile`, `create_call_signal`, `create_message` — every typed helper sets a typed `Projection` enum value.

**Hop 2.** `Fiber::hash_preimage()` (types.rs:232) serializes a `CanonicalFiber` struct whose **2nd of 18 fields is `projection: &'a Projection`**. No `#[serde(skip)]`. Field order is determined by struct declaration, stable across builds.

**Hop 3.** `Bundle::compute_hash()` (types.rs:303) feeds `self.base` and `self.fiber.hash_preimage()` into SHA-256. So projection's bytes flow into the digest.

**Hop 4.** `sign_bundle()` (sign.rs:11) Ed25519-signs `bundle.fiber.hash.0` — exactly the digest from Hop 3.

**Hop 5.** `verify_bundle()` (sign.rs:18) first calls `verify_hash()` which recomputes `compute_hash()` and compares to the stored `fiber.hash`. If anyone mutated `projection` after signing, recomputed hash ≠ stored hash → returns `Ok(false)` before the Ed25519 step even runs. (Belt + suspenders: even if `verify_hash` were skipped, the Ed25519 check would also fail since the stored signature is over the OLD hash, not the recomputed one.)

### Why this matters for your rollout

`open_bundle` (lib.rs:1993) already surfaces the projection field as a snake_case string in the JS-side result object — see the `match projection { Projection::CallSignal => ... "call_signal" ... }` arms in lib.rs:2024+. So your app-side discipline can be:

```javascript
const opened = await gigi_wasm.open_bundle(wire_bytes);
if (opened.projection !== 'reaction') {
    throw new Error('cross-projection forge attempt');
}
// ...apply the reaction state mutation
```

…and the substrate guarantees this catches every cross-projection replay. No new gigi-wasm release needed for the projection types that already have `create_*` helpers (reaction, redaction, collectible, follow, unfollow, profile, message, voice_note, video, call_signal).

The smaller list of **missing helpers** that need real work — `create_comment`, `create_delete`, `create_presence` — those need new enum variants and new helpers. We can sequence that work once you say go. The wire-format invariant they'll inherit is the same one we just proved.

### Gap in OUR test coverage

The invariant is mathematically correct but the test suite doesn't prove it directly. Closest existing test is `tamper_fiber_fails_verify` (tests.rs:207) which tampers `created_at` — projection is fiber-adjacent so the implied coverage exists, but it'd be nice to have an explicit named artifact.

We wrote and validated a 30-line test that covers it explicitly. **Test ran green in our hands** (38/38 in `cargo test --lib`, was 37 before — exactly one new). Here it is for you to apply at your discretion — drop it into `gigi-wasm/src/bundle/tests.rs` right after `tamper_fiber_fails_verify` (line 216):

```rust
/// GGOG protocol audit G1 invariant — cross-projection replay resistance.
///
/// If an attacker takes a signed bundle of one projection type (e.g.
/// `Reaction`) and mutates `fiber.projection` to a different variant
/// (e.g. `Message`), `verify_bundle` MUST return `false`.
///
/// This holds because `projection` participates in `Fiber::hash_preimage`
/// (see `types.rs`'s `CanonicalFiber` struct — projection is the 2nd
/// of 18 signed fields), so any mutation changes `compute_hash` and
/// the stored signature fails to verify against the recomputed hash.
///
/// Without this invariant, the receiver-side discipline
/// `opened.projection === '<expected_type>'` would not buy any safety
/// against a relay-level forge. With this invariant verified, the
/// app-side strict-verify discipline is well-founded.
#[test]
fn tamper_projection_fails_verify() {
    let id = make_identity();
    let mut b = Bundle {
        base: b"reaction payload".to_vec(),
        fiber: make_fiber(Projection::Reaction),
    };
    sign_bundle(&mut b, &id);
    assert!(
        verify_bundle(&b).unwrap(),
        "freshly-signed bundle must verify before tampering"
    );

    // Cross-projection replay attempt: swap Reaction → Message.
    b.fiber.projection = Projection::Message;
    assert!(
        !verify_bundle(&b).unwrap(),
        "projection mutation must invalidate signature (G1 invariant)"
    );

    // And the symmetric direction — Message → Reaction also fails.
    let mut b2 = Bundle {
        base: b"message payload".to_vec(),
        fiber: make_fiber(Projection::Message),
    };
    sign_bundle(&mut b2, &id);
    b2.fiber.projection = Projection::Reaction;
    assert!(
        !verify_bundle(&b2).unwrap(),
        "projection mutation in either direction must invalidate"
    );
}
```

This is a drop-in patch. It compiled clean in our hands once we routed `CARGO_TARGET_DIR` outside OneDrive (we hit `LNK1201` PDB write failures on the OneDrive-backed `target/`; setting `CARGO_TARGET_DIR=$LOCALAPPDATA/cargo-target/gigi-wasm` fixed it — heads-up in case anyone else gets bitten by the same Windows pathing thing).

Once landed, that test is the green CI artifact you can point to in your audit response.

## Summary of audit #2 remaining work

What still needs to happen on `gigi-wasm`, broken out:

| Component | Status |
|---|---|
| `create_reaction` produces signed bundle with `projection` in preimage | ✅ ships today |
| `create_redaction` (moderator) ditto | ✅ ships today |
| `create_collectible` ditto | ✅ ships today |
| `create_follow` / `create_unfollow` / `create_profile` ditto | ✅ ships today |
| `create_message` / `create_voice_note` / `create_video` / `create_call_signal` ditto | ✅ ships today |
| Receiver-side `opened.projection` field on `open_bundle` result | ✅ ships today |
| `verify_bundle` enforces hash + Ed25519 | ✅ ships today |
| **`create_comment`** + `Projection::Comment` enum variant | ✗ missing — new helper |
| **`create_delete`** (user-side; distinct from `create_redaction` = moderator) + `Projection::Delete` | ✗ missing — new helper |
| **`create_presence`** + `Projection::Presence` | ✗ missing — also needs heartbeat-frequency design call (signing every heartbeat is wasteful at sub-second; consider session-cookie variant) |
| Explicit `tamper_projection_fails_verify` test | ✗ missing — patch above |
| App-side ingest path calls `open_bundle` then asserts `projection` | unknown (your call) |

## Implications + offers

1. **Your audit response can claim cross-projection replay safety on existing types today.** Apply the patch above for the green-CI artifact. No new gigi-wasm release blocks rollout of the strict-verify discipline.

2. **The remaining helpers (`create_comment`, `create_delete`, `create_presence`)** are a small bounded sprint — we'd be happy to draft the helper bodies for you to apply, same letter-shape as this one. Same wire-format invariant we just proved is inherited automatically; the work is filling in new enum variants and matching the `create_reaction` pattern.

3. **For psi_transfer (#3)**: `Projection::Payment` (tag 9) already exists in your enum. Our recommendation is to reuse it for psi_transfer with a `payment_type: 'psi'` discriminator in the base bytes, rather than carving a new tag — saves a wire-format addition. The deeper question for #3 is sequence-number protocol design. We'd want a quick call before drafting anything; three options live in our notes.

4. **For #9 DM AAD binding**: this is a real change to `gigi_encrypt` / `gigi_decrypt` signatures. We'd want to agree on canonical serialization of `(from, to, message_id, ts)` before drafting — proposing CBOR with sorted-key encoding; happy with whatever you pick.

5. **For all the `ggog-core` items (#1, #4, #6, #7, #S9)**: those are entirely your team's. We don't have visibility or change authority. Happy to read PRs and audit invariants from a substrate-fit angle if it helps.

## What we'd like back

In order of how unblocking each is:

1. **Confirmation you've applied the `tamper_projection_fails_verify` patch** so we know G1 is closed and the green-CI artifact is real on your side
2. **Decision on the missing helper sequence** (`create_comment`, `create_delete`, `create_presence`) — do you want us to draft them as patches in subsequent letters, or do you want to write them yourselves now that the wire-format invariant is verified?
3. **Sequence-number protocol decision for #3** (three options in our notes; we can mail those over if you want to design over async, or schedule a sync call)
4. **AAD canonicalization decision for #9** (CBOR-sorted-key our default; alternative is fine)

## Background context we've shipped (not security work, but adjacent)

For situational awareness, in the same window:

- **Patterns v0.2 substrate + wire** is live on `gigi-stream.fly.dev`. 1124 tests, near-miss/sat/unsat verdict envelope, explain trees, repair menus. Cross-bundle EXCLUDING IN composes with the verdict trichotomy. Substrate is domain-blind.
- **The 11-letter SCJ correspondence** wrapped Round 10. Pattern Hunt is now their consumer-side discipline; same machinery you'd use for fraud monitoring, churn, etc.
- **GIGI TUI mockup v2** (six states, GP-for-all-verbs, TEACH mode) lives at `gigi/gigi-tui-mockup.html`. Not security work but the same operator who runs the audit may want to drive it once we build it.

None of this blocks audit work. Mentioning so you know the substrate is healthy and we're not flying blind into the protocol changes you've asked for.

With care,
— GIGI fiber team

— end —
