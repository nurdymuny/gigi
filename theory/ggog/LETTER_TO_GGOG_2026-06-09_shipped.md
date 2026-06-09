# Status to GGOG team — gigi-wasm §5a–h + §6b SHIPPED

**To:** GGOG / app team
**From:** GIGI fiber team (Bee + Claude)
**Date:** 2026-06-09 (same-day on §6c ack)
**Re:** Your `LETTER_TO_GIGI_TEAM_protocol_audit_2026-06-09_6c_applied.md` — the ask "next ping is shipped" — this is it
**Working evidence:** this letter at `gigi/theory/ggog/LETTER_TO_GGOG_2026-06-09_shipped.md`

## TL;DR

All 10 patches from `theory/ggog/staging/APPLY_v0.1_to_gigi_wasm.md` applied to `gigi-wasm` working tree. Test floor moved from **37 → 44** green. Build clean. Ready for you to ship the new wasm bundle through your publication path, then drop the `connection.js` snippet and start draining the verify-only window.

## What landed in gigi-wasm

Applied in dependency order per the staging doc:

| Step | What | Tests added |
|---|---|---|
| **§6b** | `sign_register_challenge(nonce: &[u8]) -> Result<String, JsValue>` — domain-separated nonce signing with 27-byte ASCII prefix `b"ggog/register-challenge/v1\n"`. | `sign_register_challenge_round_trips_with_domain_separator` |
| **§5a** | `Projection` enum extension: `Comment` (tag 18), `Delete` (tag 19), `Presence` (tag 20). `to_tag` / `from_tag` / `from_str_name` updated. Existing `projection_tag_roundtrip` and `projection_str_roundtrip` tests extended to cover the new variants. | (extends 2 existing) |
| **§5f** | `open_bundle` match arms for the 3 new projection types — return `text` + `message_id` for comment, `subject_hash` for delete, `display_name` / `avatar_url` / `status` for presence. | (used by §5h round-trip tests) |
| **§5b** | `create_comment(to_pubkey, message_id, text, parent_hash)` | `create_comment_round_trips_through_sign_verify` |
| **§5c** | `create_delete(subject_hash)` | `create_delete_signed_against_subject_hash` |
| **§5d** | `create_presence(display_name, avatar_url, status)` | `create_presence_round_trips` |
| **§5g** | `create_psi_transfer(to_pubkey, amount_psi, seq, memo)` — reuses `Projection::Payment` (tag 9) with `seq: Some(seq)` in the signed fiber. | `psi_transfer_seq_in_signed_payload` |
| **§5h** | G1 invariant `tamper_projection_fails_verify` test from the 2026-06-07 letter. | (1 named above) |
| **§5e** | `gigi_encrypt(plaintext, recipient_dm_pubkey, aad: &[u8])` + `gigi_decrypt(ciphertext, nonce, sender_ephemeral_pubkey, aad: &[u8])` — ChaCha20-Poly1305 `Payload { msg, aad }` form. **API BREAKING.** | `aad_mismatch_fails_decrypt` |

## Test gate

```
running 44 tests
... (all existing 37 + 7 new) ...

test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
```

Including the new gates:

- `sign_register_challenge_round_trips_with_domain_separator` — pins the §6b domain-separator pair with your `verify_register_challenge` reconstruction
- `aad_mismatch_fails_decrypt` — pins the §5e auth-tag check (modify ts in AAD → decrypt fails)
- `psi_transfer_seq_in_signed_payload` — pins that mutating `seq` after sign invalidates verify
- `tamper_projection_fails_verify` — G1 cross-projection replay invariant
- 4 round-trip tests pinning the 4 new helper bundles encode + sign + decode + verify cleanly

## API-break note: §5e

`gigi_encrypt` / `gigi_decrypt` now take a `aad: &[u8]` parameter. Both internal helper call sites in `gigi-wasm` (the DHOOM-encoded create_message path and the `create_dm_v2` path) currently pass `b""` (empty AAD) — preserves pre-§5e semantics for those internal helpers. They're flagged in code comments as "§5e follow-up will thread real AAD."

**JS side**: every external `gigi_encrypt` / `gigi_decrypt` call site MUST supply an AAD bytes argument. Empty `new Uint8Array()` works for transitional callers; the production messenger.js path should build the canonical bytes:

```javascript
const aad = new TextEncoder().encode(
    `from=${state.identity.pubkey}|to=${recipient}|msg=${msgId}|ts=${ts}`
);
```

Per our §5e spec the canonical is `b"from=" || hex(from) || b"|to=" || hex(to) || b"|msg=" || msgId || b"|ts=" || ts`. We held off changing internal `gigi-wasm` call sites because (a) keeps the §5e ship minimal, and (b) per your sequencing note, you'll wire AAD into `messenger.js` after `aad_mismatch_fails_decrypt` smokes against the live build anyway.

## What you do next

Per your sequencing letter, after this ship:

1. **Smoke `aad_mismatch_fails_decrypt`** against the new wasm build (you can mirror the test in JS against the public binding to be doubly sure). This is the §5e gate before you wire `messenger.js`.
2. **Drop the `connection.js` snippet** from our 2026-06-09 challenge-reply letter:
   - On `challenge` frame → `gigi_wasm.sign_register_challenge(nonceBytes)` and stash
   - On next `register` send → include `challenge_signature` + `challenge_nonce` fields
3. **Wire AAD construction in `messenger.js`** for both send and receive paths.
4. **Wire receiver-side `opened.projection === expected` asserts** in `feed-store.js:684-721` and `json-router.js:317-352` for the 3 new projections. The verify-only metric buffer at `/admin/verify-metrics` already exists per your status letter; the moment the new wasm lands, you'll see signed/unsigned ratios live.
5. **Drain the verify-only window.** Once unsigned-register ratio → 0 and the signed-projection ratio is at parity, flip `GGOG_REQUIRE_CHALLENGE=1` on the relay and (separately) flip strict-only projection asserts in `feed-store.js`.

## Sequencing recap

- **G2** ✅ — §5e is live in gigi-wasm; pair with your #1 register challenge as planned
- **G3a** ✅ — §5a Projection enum + tag table extended
- **G3b/c/d** ✅ — `create_comment` / `create_delete` / `create_presence` live
- **G3e** ✅ — `open_bundle` arms for the 3 new types
- **G4** ✅ — `create_psi_transfer` substrate side; gated on your relay-side seq enforcement at register
- **G5** — joint rollout coordination; starts when verify-only metrics drain

## Repo / boundary discipline note

Per the 2026-06-09 letter exchange, this apply happened directly in the
`~/Documents/ggog/gigi-wasm/` working tree (Bee's one-time call to override
the stream-crossing constraint for this apply, since gigi-wasm is our code).
Future round-trips revert to letter-only at `theory/ggog/` unless you say
otherwise.

## Acknowledgements

> The 27-byte ASCII prefix `b"ggog/register-challenge/v1\n"` makes the two
> domains structurally non-overlapping: a `fiber.hash.0` SHA-256 output
> cannot accidentally start with those bytes, and the v1 suffix gives a
> clean rotation path if we ever need to re-domain.

Sharp framing of the security argument. Our §6b implementation uses the
same prefix verbatim. The two implementations land in lockstep — no
ambiguity at the wire boundary.

With care,
— GIGI fiber team

— end —
