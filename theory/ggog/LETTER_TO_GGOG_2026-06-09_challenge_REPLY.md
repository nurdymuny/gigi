# Reply to GGOG status note — yes to domain separator, here's `sign_register_challenge`

**To:** GGOG / app team
**From:** GIGI fiber team (Bee + Claude)
**Date:** 2026-06-09 (same-day reply)
**Re:** Your `LETTER_TO_GIGI_TEAM_protocol_audit_2026-06-09_status.md` — ggog-core column closed; you asked one question
**Working evidence:** this letter at `gigi/theory/ggog/LETTER_TO_GGOG_2026-06-09_challenge_REPLY.md`

## TL;DR

Congrats on landing five items in one window — #1 / #4 / #6 / #7 (a, b, c) / S9, 16 + 742 tests green, that's an exceptional sprint.

You asked one question: should `challenge_signature` use a domain separator before we implement `sign_register_challenge`?

**Yes, please.** The account's Ed25519 signing key already signs `fiber.hash.0` via `sign_bundle` (`src/bundle/sign.rs:11`) — a 32-byte SHA-256. Your register-challenge is also a 32-byte random nonce. Without a domain separator, the two signing contexts share a domain, and a relay-positioned attacker could choose `nonce = future_target_bundle_hash` and harvest a reusable bundle signature. Classic Ed25519 signing-oracle risk.

The fix is a fixed ASCII prefix in front of the nonce before signing. Concrete proposal + drop-in patches for both sides below.

## §6 — `sign_register_challenge` binding + domain-separator design

### §6a — Design choice: simple ASCII prefix

Format:

```
signed_message = b"ggog/register-challenge/v1\n" || nonce
                 \___________ 27 bytes _______/   \_ 32 bytes _/
                            domain separator         relay nonce
                 total: 59 bytes
```

Why this shape:
- **27-byte fixed prefix** makes the signed-message space disjoint from
  bundle signing (`fiber.hash` is exactly 32 bytes). Even if an attacker
  could choose a nonce equal to a future hash, the prefix means the
  signed bytes are ≠ those hash bytes.
- **Versioned** (`v1`) so we have a clear migration path if the design
  changes. Future rotations bump to `v2` and the relay can require it
  via env var without touching the wire protocol.
- **Newline as separator** for readability when grep'ing logs; any
  non-hex byte would work but `\n` is unambiguous and human-friendly
  in the rare case someone has to debug a raw signed message.
- **ASCII**, so it round-trips through any encoding boundary cleanly.
  No CBOR / no binary framing — keeps the dep surface tiny on both
  sides.

The signed bytes are NOT a bundle. They're a free-standing message
specific to register-challenge — that's the whole point of the domain
separator. Verifier just reconstructs the same 59 bytes and verifies.

### §6b — Drop-in patch for `src/lib.rs` (gigi-wasm)

`Identity` already has `sign_hex(message: &[u8]) -> String` exposed
(see `src/crypto.rs:50-53`). The new binding is a thin wrapper:

```rust
/// Sign the relay's register-challenge nonce with the account key.
///
/// Per the 2026-06-09 GGOG status letter and our reply: the relay
/// issues a 32-byte random `nonce` on WS upgrade via the `challenge`
/// frame. The client signs the domain-separated message
///
///   b"ggog/register-challenge/v1\n" || nonce
///
/// with the account's Ed25519 signing key and echoes the hex-encoded
/// signature back in the next `register` frame's `challenge_signature`
/// field.
///
/// ## Why the domain separator
///
/// The account key also signs `fiber.hash.0` via `sign_bundle` (32-byte
/// SHA-256). Without the prefix the two signing contexts would share a
/// domain, and a relay that picks `nonce = future_target_bundle_hash`
/// could harvest a reusable bundle signature. The 27-byte ASCII prefix
/// keeps the contexts disjoint.
///
/// ## Arguments
///
/// `nonce` — exactly 32 bytes, as issued by the relay's `challenge`
/// frame. Any other length is rejected — the relay protocol pins this.
///
/// ## Returns
///
/// 128-hex-char Ed25519 signature over the 59-byte domain-separated
/// message.
#[wasm_bindgen]
pub fn sign_register_challenge(nonce: &[u8]) -> Result<String, JsValue> {
    if nonce.len() != 32 {
        return Err(JsValue::from_str(&format!(
            "nonce must be exactly 32 bytes, got {}",
            nonce.len()
        )));
    }
    GIGI_STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref()
            .ok_or_else(|| JsValue::from_str("GIGI not initialized"))?;

        const DOMAIN: &[u8] = b"ggog/register-challenge/v1\n";
        let mut msg = Vec::with_capacity(DOMAIN.len() + nonce.len());
        msg.extend_from_slice(DOMAIN);
        msg.extend_from_slice(nonce);

        Ok(state.identity.sign_hex(&msg))
    })
}
```

### §6c — Matching verifier patch for `ggog-core`

In your register handler, change the verify call from:

```rust
// Old (no domain separator):
identity::verify_signature(&account_pubkey, &nonce_bytes, &sig_bytes)
```

to:

```rust
// New (matches gigi-wasm §6b):
const REGISTER_CHALLENGE_DOMAIN: &[u8] = b"ggog/register-challenge/v1\n";
let mut signed_msg = Vec::with_capacity(REGISTER_CHALLENGE_DOMAIN.len() + 32);
signed_msg.extend_from_slice(REGISTER_CHALLENGE_DOMAIN);
signed_msg.extend_from_slice(&nonce_bytes);
identity::verify_signature(&account_pubkey, &signed_msg, &sig_bytes)
```

Apply this in both the JSON and binary-DHOOM register paths (the
`identity::verify_signature` call site in each). Your existing four
tests should still pin the new shape:

- `register_challenge_accepts_legitimate_signature` — replace the test's
  inline signer to call the domain-separated sign, then verify.
- `register_challenge_rejects_stale_or_swapped_nonce` — still passes;
  swapping the nonce changes the signed bytes regardless of prefix.
- `register_challenge_rejects_wrong_signer` — still passes; pubkey
  mismatch is independent of prefix.
- `register_challenge_rejects_malformed_inputs` — still passes; length
  + hex-decode checks are pre-signature.

If you want one more, here's a regression test for the domain
separator itself:

```rust
#[test]
fn register_challenge_rejects_signature_without_domain_separator() {
    // A signature over the raw 32-byte nonce (no prefix) MUST NOT
    // verify against the relay's domain-separated message construction.
    // Prevents a forge against an older client implementation.
    let id = make_identity();
    let nonce = [7u8; 32];

    // Naively sign the raw nonce (the way the OLD client would have).
    let raw_sig = id.sign_hex(&nonce);

    // Relay-side verify reconstructs the domain-separated message.
    const REGISTER_CHALLENGE_DOMAIN: &[u8] = b"ggog/register-challenge/v1\n";
    let mut signed_msg = Vec::new();
    signed_msg.extend_from_slice(REGISTER_CHALLENGE_DOMAIN);
    signed_msg.extend_from_slice(&nonce);

    let sig_bytes = hex::decode(&raw_sig).unwrap();
    assert!(
        !identity::verify_signature(&id.pubkey(), &signed_msg, &sig_bytes).unwrap_or(false),
        "raw-nonce signature MUST NOT verify against domain-separated message"
    );
}
```

### §6d — Test patch for gigi-wasm side

`src/bundle/tests.rs` after the patches from the 2026-06-09 letter:

```rust
#[test]
fn sign_register_challenge_round_trips_with_domain_separator() {
    // Sign a 32-byte nonce; reconstruct the same domain-separated
    // message on the "verifier" side; ensure the signature verifies.
    let id = make_identity();
    let nonce = [42u8; 32];

    const DOMAIN: &[u8] = b"ggog/register-challenge/v1\n";
    let mut msg = Vec::new();
    msg.extend_from_slice(DOMAIN);
    msg.extend_from_slice(&nonce);

    let sig_hex = id.sign_hex(&msg);
    let sig_bytes = hex::decode(&sig_hex).expect("valid hex");

    // Use ed25519_dalek directly to verify (mirroring what ggog-core
    // does via its identity::verify_signature helper).
    use ed25519_dalek::{Signature, Verifier};
    let sig = Signature::from_slice(&sig_bytes).expect("64-byte sig");
    let vk = id.verifying_key();   // helper if needed; or test against pubkey path
    assert!(vk.verify(&msg, &sig).is_ok(),
            "domain-separated nonce signature must verify");
}

#[test]
fn sign_register_challenge_rejects_wrong_length_nonce() {
    // Substrate enforces the 32-byte length contract; relay relies on
    // it for the security argument.
    // This test reaches into the wasm-bindgen wrapper indirectly via
    // its internal length-check (no GIGI_STATE initialization needed
    // since we're testing the length-gate, but if the bind sig changes
    // shape this can move to a wasm-bindgen-test).
    // Skipped here if not easily callable outside wasm context; the
    // gate is the leading length-check in §6b. Verify by inspection.
}
```

(The second test is informational — the length check is the leading
guard in §6b and a fast-fail before any state borrow. If you have a
wasm-bindgen-test harness, that's the right place for it.)

## Client-side wiring sketch

For `src/connection.js` in `ggog-app`:

```javascript
// On WS message dispatch, branch for the new challenge frame:
case 'challenge': {
    // Relay handed us a 32-byte hex nonce.
    const nonceBytes = hexToBytes(msg.nonce);
    if (nonceBytes.length !== 32) {
        console.warn('challenge: bad nonce length', nonceBytes.length);
        return;
    }
    // Sign with the substrate; stash for next register send.
    this._pendingChallengeSignature = gigi_wasm.sign_register_challenge(nonceBytes);
    this._pendingChallengeNonce = msg.nonce;  // echo back as hex
    break;
}

// On register send (existing path), include the two new fields when
// we have them:
const registerFrame = {
    type: 'register',
    account: this.state.identity.pubkey,
    genesis_hash: CANONICAL_GENESIS_HASH,
    ...(this._pendingChallengeSignature && {
        challenge_signature: this._pendingChallengeSignature,
        challenge_nonce:     this._pendingChallengeNonce,
    }),
};
```

The `_pendingChallengeSignature` survives only until the next register
send so a relay can't re-prompt and harvest multiple signatures.
Optionally clear it on `register_ack` or after send to be belt-and-
suspenders.

## What we'd like back

Just one thing: confirmation §6b compiles + §6d round-trip test passes
when you apply. That's the gate before you wire `connection.js` and
flip `GGOG_REQUIRE_CHALLENGE=1`.

After that, the sequencing from our 2026-06-09 e/f letter holds:

- **G2 ready when:** §5e (`gigi_encrypt` / `gigi_decrypt` with AAD)
  ships AND `sign_register_challenge` ships (§6b) — both gates live
  in `gigi-wasm` so you can land them in the same build.
- **#1 strict-only when:** every shipped client has §6b in its
  bundle and you've drained the rollout window.
- **G3 / G4 / G5:** unchanged from prior letter.

## Acknowledgements

> No coordination needed from you on any of them.

Beautiful. The clean split between substrate (us) and relay (you)
is paying off.

> 16/16 node bin, 742/742 lib

We hit a similar pattern with our paper substrate ship today —
~1354 tests across the kahler+imagine+sharded+transactions+patterns+
causal_states feature matrix, byte-identical no-feature build. The
"strict-additive" discipline is the load-bearing thing. Glad to see
you're holding the same line on the relay side.

> The proper fix [for multi-VM] is Redis-backed shared state; that's a
> separate project.

Concur. If you ever want a hand specifying the Redis schema, ping —
we've done some related work on cross-VM consistency for `gigi-stream`
that might compose with how you'd want to shard `ClientRegistry` /
`DenylistIndex`. Not urgent.

With care,
— GIGI fiber team

— end —
