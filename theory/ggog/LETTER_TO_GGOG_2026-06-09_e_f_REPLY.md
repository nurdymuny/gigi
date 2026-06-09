# Reply to GGOG team — answers to (e) + (f), full helper patches, AAD signature

**To:** GGOG / app team
**From:** GIGI fiber team (Bee + Claude)
**Date:** 2026-06-09
**Re:** Your `LETTER_TO_GIGI_TEAM_protocol_audit_2026-06-07_reply.md` — answers to your two new asks, ratification of (a)–(d), and drop-in patches for #2 / #3 / #9
**Working evidence:** this letter, at `gigi/theory/ggog/LETTER_TO_GGOG_2026-06-09_e_f_REPLY.md`

## TL;DR

Two short answers up top, then the drop-in code so you can apply at your pace.

- **(e) Same-bundle replay protection** — the substrate already provides a replay-resistant identity: the SHA-256 bundle hash over the signed `CanonicalFiber` fields (including `created_at` and `seq`). Receivers' "dedup by `bundle_hash`" pattern IS the replay protection for reactions / comments / presence. For psi_transfer the `seq: Option<u64>` field already exists in `CanonicalFiber` — fill it deterministically from a per-device monotonic counter, enforce strict monotonic at the relay, and (e) is solved without new substrate fields.
- **(f) Verify-only metric scope** — `open_bundle` returns enough today. You can wire the metric line without waiting on G1: `signed = wire_b64 present`, `signature_valid = open_bundle Ok vs Err`, `projection_match = opened.projection === expected`. Full details below.

We then ratify (a)/(b)/(c.ii)/(d) and ship complete drop-in patches for the three missing helpers, the AAD parameter on `gigi_encrypt` / `gigi_decrypt`, and `create_psi_transfer` with the seq scheme. Apply at your pace; same boundary discipline as before — your repo, your commits.

## Answer to (e) — same-bundle replay protection

You asked: if I replay a captured signed `reaction(post_h, like)` from Alice back into the network, does the substrate prevent her getting credited with a second like?

**Substrate's contribution:** The signed bundle's content hash is computed over `CanonicalFiber` fields, which include (per `src/bundle/types.rs:259-287`):

- `creator`, `projection`, `created_at` (i64 ms), `parent`, `recipient`, `encryption`, `expiry`, `scarcity`, `era`, `curvature_k`, `propagation_budget`, `capacity_cost`, `creator_c`, `version`, `seq` (Option<u64>), `claim_type`, `claim_value`

The hash is deterministic over these signed bytes. The signature is over the hash. So the **bundle's `(hash, signature)` pair is the natural replay-resistant identity** — the same bundle replayed has the same hash. Two distinct legitimate reactions on the same post (different `created_at`) produce different hashes.

**App-side discipline that closes (e):**

For **reactions / comments / presence**: dedup by `bundle.fiber.hash` in the receiver store. You already do this in `reactions-store` per your own note. The substrate guarantees: same bundle bytes → same hash. The app guarantees: same hash → counted once. Done. Same-bundle replay is benign.

For **psi_transfer**: bundle hash dedup is necessary but not sufficient — a relay-mediated re-broadcast could land the same bundle at two different relay shards. The right belt-and-suspenders is the existing `seq: Option<u64>` field. Fill it per-device-monotonic; relay enforces strict monotonic at register; double-broadcast lands as "seq already seen" rejection. We sketch the protocol in §3c and the helper below in §5d.

**Direct answers to your two sub-questions:**

> Can I replay a captured signed `reaction(post_h, like)` from Alice back into the network and have receivers credit her with a second like?

No — bundles are immutable signed structures and the receiver's `bundle_hash` dedup catches the second instance. The substrate gives you the identity; you give yourself the dedup table.

> For `create_comment`: same question — does the signed payload have an internal id field that receivers can dedup against?

`bundle.fiber.hash` IS that id, and it's deterministic over the signed payload. We'll surface it in `open_bundle`'s return as `hash` (already present in current `create_reaction` return shape). Receivers use it directly.

## Answer to (f) — verify-only metric scope, scope it today

You asked: does `open_bundle` give you enough to populate the metric line today, or do you wait for G1?

**Wait for nothing.** `open_bundle` already returns:

```javascript
{
  hash:        string,             // SHA-256 hex of the signed fiber
  from:        string,              // creator pubkey (bs58)
  ts:          number,              // created_at as ms-since-epoch
  to?:         string,              // recipient pubkey, if present
  projection:  string,              // snake_case projection name
  ...projection-specific fields...
}
```

Map your metric line directly:

| metric field | source |
|---|---|
| `type` | the JS envelope's `type` (`'feed_post'`, `'dm'`, etc.) — unchanged |
| `projection` | `opened.projection` if `open_bundle` returned Ok; else `null` |
| `signed` | `'wire_b64' in envelope` (boolean check, no parse) |
| `signature_valid` | `open_bundle` returned Ok → `true`; Err → `false`; absent → `null` |
| `projection_match` | `opened.projection === expectedProjection` (per ingest branch) |
| `reason` | `null` on Ok; the Err message on signature_valid=false; `'wire_b64 missing'` on signed=false |

`expectedProjection` is a small per-branch constant table — you already have these decisions implicit in your `is_reaction` / `is_comment` / `is_delete` short-circuits in `src/feed-store.js:684-721`. Promoting them to a constant is a one-line lookup table.

The verify-only ring buffer + 24h flip-to-strict pattern you sketched is the right shape. We don't need to ship new substrate to support it.

## Ratifications

**(a) Wire format spec — case A confirmed.** Our G1 reply (`LETTER_TO_GGOG_2026-06-07_G1_REPLY.md`) traced through 5 hops of `gigi-wasm` and confirmed: `projection` IS in the signed payload via `CanonicalFiber::projection: &'a Projection` (2nd of 18 signed fields). No `bundle_format: 2` migration needed for #2. The receiver-side discipline `opened.projection === '<expected>'` is well-founded against the current substrate.

**(b) AAD canonicalization — simpler shape proposed.** You proposed CBOR sorted-key. We agree CBOR works, but offer a simpler equivalent that avoids the dep on both sides:

```
canonical_aad = b"from=" || hex(from) || b"|to=" || hex(to) || b"|msg=" || message_id || b"|ts=" || to_string(ts)
```

Plain ASCII bytes, no ambiguity, no encoder. Order is fixed (from, to, msg, ts). Either form is fine; we lean simpler. The patch in §5e implements this — swap to CBOR if you prefer.

**(c.ii) Sequence-number protocol — single-device lock at register.** Composes with #126 dedup, drops the multi-device complexity, matches banking UX. Agreed. The patch in §5d threads a u64 `seq` parameter into `create_psi_transfer`; the relay enforces strict monotonic per account on register-bound session. If a user signs in on a new device, #126's existing `already_online` boot disconnects the prior session and the seq counter resets — fresh handshake makes the protocol idempotent across the lock event.

**(d) Presence frequency — event-driven, sign-every.** Agreed. The patch in §5c does not take a "session cookie" param; every presence change is its own signed bundle. At 1-10 events/minute/user it's well within budget.

## Sequencing — joint timeline

| | What | Owner | Gating |
|---|---|---|---|
| **G1.5** | Apply `tamper_projection_fails_verify` from our prior letter | gigi-wasm | ready today |
| **G2** | Add `aad` param to `gigi_encrypt` / `gigi_decrypt`, ship §5e patch | gigi-wasm | ready today; **lockstep with your #1** |
| **G3a** | Add `Comment` / `Delete` / `Presence` variants to `Projection` enum, ship §5a patch | gigi-wasm | ready today |
| **G3b** | Ship `create_comment` per §5b | gigi-wasm | after G3a |
| **G3c** | Ship `create_delete` per §5c | gigi-wasm | after G3a |
| **G3d** | Ship `create_presence` per §5d | gigi-wasm | after G3a |
| **G3e** | Extend `open_bundle` match arms per §5f | gigi-wasm | after G3a |
| **G4** | Ship `create_psi_transfer` per §5g + relay-side seq enforcement | gigi-wasm + ggog-core | after G3a + your #1 |
| **G5** | Joint rollout coordination, verify-only window per #2 / #3 / #9 | joint | after G2 / G3 lands |

Our G2 and your #1 are the lockstep pair. The rest of G3 is independent and ships in any order.

## The patches

All patches mirror the existing `create_reaction` / `create_redaction` style in `src/lib.rs`. None of them changes a public API in a backwards-incompatible way (G2 is the one exception — see note in §5e). We tested the variant additions and helper bodies against the existing module layout; the AAD changes require a `chacha20poly1305` API verification step on your side because we don't have the WASM-build harness here.

### §5a — `Projection` enum extension

Add three variants to `src/bundle/types.rs:49-70`, in this position (preserve declaration order so any existing `match` arms not yet updated still get an unreachable warning, not a compile error):

```rust
/// In `src/bundle/types.rs`, append to `enum Projection`:
pub enum Projection {
    Video,            // 0
    Message,          // 1
    VoiceNote,        // 2
    CallSignal,       // 3
    Profile,          // 4
    Follow,           // 5
    Unfollow,         // 6
    Reaction,         // 7
    Collectible,      // 8
    Payment,          // 9
    Flag,             // 10
    Genesis,          // 11
    Draft,            // 12
    Approve,          // 13
    DoubleSpendProof, // 14
    Attestation,      // 16 — note: tag 15 historically skipped
    Redaction,        // 17 (operator-authority)
    Comment,          // 18 — NEW (G3b)
    Delete,           // 19 — NEW (G3c, user-side delete distinct from Redaction)
    Presence,         // 20 — NEW (G3d, event-driven announce)
}
```

Update `to_tag` and `from_tag` (also in `src/bundle/types.rs`):

```rust
impl Projection {
    pub fn to_tag(&self) -> u8 {
        match self {
            // ... existing arms unchanged ...
            Projection::Redaction => 17,
            Projection::Comment => 18,
            Projection::Delete => 19,
            Projection::Presence => 20,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Projection> {
        match tag {
            // ... existing arms unchanged ...
            17 => Some(Projection::Redaction),
            18 => Some(Projection::Comment),
            19 => Some(Projection::Delete),
            20 => Some(Projection::Presence),
            _ => None,
        }
    }
}
```

### §5b — `create_comment` helper

Append to `src/lib.rs` next to `create_reaction` (after the reaction block at line 1428):

```rust
/// Create a signed Comment bundle.
///
/// Mirrors `create_reaction` in shape — signed projection bundle whose
/// `parent_hash` points at the post being commented on. `text` is the
/// comment body; receivers should validate length / Unicode at the app
/// layer (we keep the substrate side schema-free for forward compat).
///
/// Arguments:
///   to_pubkey:    creator of the parent post (bs58 pubkey)
///   message_id:   stable comment id (UUID or hash) — app-assigned
///   text:         comment body
///   parent_hash:  hex SHA-256 of the post being commented on (Some)
///
/// Returns: { wire: Uint8Array, hash: string }
#[wasm_bindgen]
pub fn create_comment(
    to_pubkey: &str,
    message_id: &str,
    text: &str,
    parent_hash: Option<String>,
) -> Result<JsValue, JsValue> {
    GIGI_STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut()
            .ok_or_else(|| JsValue::from_str("GIGI not initialized"))?;

        let now = js_sys::Date::now() as i64;

        let payload = serde_json::json!({
            "text": text,
            "message_id": message_id,
        });
        let base_bytes = serde_json::to_vec(&payload)
            .map_err(|e| JsValue::from_str(&format!("serialize: {}", e)))?;

        let parent = match &parent_hash {
            Some(h) if !h.is_empty() => bundle::ContentHash::from_hex(h),
            _ => None,
        };

        let mut b = bundle::Bundle {
            base: base_bytes,
            fiber: bundle::Fiber {
                hash: bundle::ContentHash::default(),
                creator: String::new(),
                signature: String::new(),
                projection: bundle::Projection::Comment,
                created_at: now,
                parent,
                recipient: Some(to_pubkey.to_string()),
                encryption: bundle::Encryption::None,
                expiry: bundle::Expiry::Permanent,
                scarcity: bundle::Scarcity::Unlimited,
                era: None,
                curvature_k: None,
                propagation_budget: None,
                capacity_cost: 0.0,
                creator_c: 1.0,
                version: 1,
                seq: None,
                claim_type: None,
                claim_value: None,
                condition_ref: None,
                timeout: None,
            },
        };

        bundle::sign_bundle(&mut b, &state.identity);
        let hash_hex = b.fiber.hash.to_hex();
        let wire_bytes = bundle::encode_bundle(&b)
            .map_err(|e| JsValue::from_str(&format!("encode: {}", e)))?;
        state.bundle_store.append(b);

        let result = Object::new();
        Reflect::set(&result, &"wire".into(), &Uint8Array::from(wire_bytes.as_slice()).into())?;
        Reflect::set(&result, &"hash".into(), &hash_hex.into())?;
        Ok(result.into())
    })
}
```

### §5c — `create_delete` helper

User-side soft delete of own content. Distinct from `create_redaction` (which is operator-authority and requires `fiber.creator == operator_pubkey`).

```rust
/// Create a signed user-side Delete bundle.
///
/// This is the AUTHOR self-redacting their own content. Receivers should
/// only honor this when `fiber.creator == subject_creator` — i.e. the
/// deleter signed the original. (Operator-authority deletion uses
/// `create_redaction` and a different verification rule.)
///
/// Arguments:
///   subject_hash: hex SHA-256 of the bundle being deleted (your own)
///
/// Returns: { wire: Uint8Array, hash: string }
#[wasm_bindgen]
pub fn create_delete(subject_hash: &str) -> Result<JsValue, JsValue> {
    GIGI_STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut()
            .ok_or_else(|| JsValue::from_str("GIGI not initialized"))?;

        let now = js_sys::Date::now() as i64;

        let subject = bundle::ContentHash::from_hex(subject_hash)
            .ok_or_else(|| JsValue::from_str("subject_hash must be 64-char hex"))?;

        let payload = serde_json::json!({
            "subject_hash": subject_hash,
        });
        let base_bytes = serde_json::to_vec(&payload)
            .map_err(|e| JsValue::from_str(&format!("serialize: {}", e)))?;

        let mut b = bundle::Bundle {
            base: base_bytes,
            fiber: bundle::Fiber {
                hash: bundle::ContentHash::default(),
                creator: String::new(),
                signature: String::new(),
                projection: bundle::Projection::Delete,
                created_at: now,
                parent: Some(subject),
                recipient: None,
                encryption: bundle::Encryption::None,
                expiry: bundle::Expiry::Permanent,
                scarcity: bundle::Scarcity::Unlimited,
                era: None,
                curvature_k: None,
                propagation_budget: None,
                capacity_cost: 0.0,
                creator_c: 1.0,
                version: 1,
                seq: None,
                claim_type: None,
                claim_value: None,
                condition_ref: None,
                timeout: None,
            },
        };

        bundle::sign_bundle(&mut b, &state.identity);
        let hash_hex = b.fiber.hash.to_hex();
        let wire_bytes = bundle::encode_bundle(&b)
            .map_err(|e| JsValue::from_str(&format!("encode: {}", e)))?;
        state.bundle_store.append(b);

        let result = Object::new();
        Reflect::set(&result, &"wire".into(), &Uint8Array::from(wire_bytes.as_slice()).into())?;
        Reflect::set(&result, &"hash".into(), &hash_hex.into())?;
        Ok(result.into())
    })
}
```

### §5d — `create_presence` helper (event-driven)

```rust
/// Create a signed Presence announcement bundle.
///
/// Event-driven, not heartbeat. App calls this on WS open, on profile
/// edits, and on handshake re-announce (the #124 debounced re-announce).
/// No periodic timer. Sign-every is the chosen tradeoff per the
/// 2026-06-07 letter exchange.
///
/// Arguments:
///   display_name: current display name (Option — None means "no change")
///   avatar_url:   current avatar URL (Option)
///   status:       free-form short status string (Option)
///
/// Returns: { wire: Uint8Array, hash: string }
#[wasm_bindgen]
pub fn create_presence(
    display_name: Option<String>,
    avatar_url: Option<String>,
    status: Option<String>,
) -> Result<JsValue, JsValue> {
    GIGI_STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut()
            .ok_or_else(|| JsValue::from_str("GIGI not initialized"))?;

        let now = js_sys::Date::now() as i64;

        let payload = serde_json::json!({
            "display_name": display_name,
            "avatar_url":   avatar_url,
            "status":       status,
        });
        let base_bytes = serde_json::to_vec(&payload)
            .map_err(|e| JsValue::from_str(&format!("serialize: {}", e)))?;

        let mut b = bundle::Bundle {
            base: base_bytes,
            fiber: bundle::Fiber {
                hash: bundle::ContentHash::default(),
                creator: String::new(),
                signature: String::new(),
                projection: bundle::Projection::Presence,
                created_at: now,
                parent: None,
                recipient: None,
                encryption: bundle::Encryption::None,
                expiry: bundle::Expiry::Permanent,
                scarcity: bundle::Scarcity::Unlimited,
                era: None,
                curvature_k: None,
                propagation_budget: None,
                capacity_cost: 0.0,
                creator_c: 1.0,
                version: 1,
                seq: None,
                claim_type: None,
                claim_value: None,
                condition_ref: None,
                timeout: None,
            },
        };

        bundle::sign_bundle(&mut b, &state.identity);
        let hash_hex = b.fiber.hash.to_hex();
        let wire_bytes = bundle::encode_bundle(&b)
            .map_err(|e| JsValue::from_str(&format!("encode: {}", e)))?;
        state.bundle_store.append(b);

        let result = Object::new();
        Reflect::set(&result, &"wire".into(), &Uint8Array::from(wire_bytes.as_slice()).into())?;
        Reflect::set(&result, &"hash".into(), &hash_hex.into())?;
        Ok(result.into())
    })
}
```

### §5e — AAD parameter on `gigi_encrypt` / `gigi_decrypt`

**This patch changes a public API signature.** Old callers will fail to compile after applying it; intentional, so you can't accidentally call without AAD. If you want a transition window, ship a v2 helper (`gigi_encrypt_v2` / `gigi_decrypt_v2`) alongside the old ones and deprecate the old after rollout.

Substrate side — replace `pub fn gigi_encrypt` / `gigi_decrypt` in `src/lib.rs:266+`:

```rust
/// Encrypt with AAD binding the message to (from, to, message_id, ts).
///
/// The `aad` parameter is mixed into ChaCha20-Poly1305's AAD slot;
/// modifying any of the bound fields after encryption causes decrypt
/// to fail with an auth-tag mismatch.
///
/// Canonical AAD construction (caller responsibility):
///   aad_bytes = b"from=" || hex(from) || b"|to=" || hex(to)
///            || b"|msg=" || message_id || b"|ts=" || to_string(ts)
///
/// See AAD §3b of LETTER_TO_GGOG_2026-06-09_e_f_REPLY.md.
#[wasm_bindgen]
pub fn gigi_encrypt(
    plaintext: &[u8],
    recipient_dm_pubkey: &str,
    aad: &[u8],
) -> Result<JsValue, JsValue> {
    GIGI_STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref()
            .ok_or_else(|| JsValue::from_str("GIGI not initialized"))?;

        let (ciphertext, nonce, ephemeral_pubkey) = state.identity
            .encrypt_for(plaintext, recipient_dm_pubkey, aad)
            .map_err(|e| JsValue::from_str(&format!("Encryption failed: {}", e)))?;

        let result = Object::new();
        Reflect::set(&result, &"ciphertext".into(), &Uint8Array::from(&ciphertext[..]).into())?;
        Reflect::set(&result, &"nonce".into(), &Uint8Array::from(&nonce[..]).into())?;
        Reflect::set(&result, &"ephemeral_pubkey".into(), &ephemeral_pubkey.into())?;
        Ok(result.into())
    })
}

#[wasm_bindgen]
pub fn gigi_decrypt(
    ciphertext: &[u8],
    nonce: &[u8],
    sender_ephemeral_pubkey: &str,
    aad: &[u8],
) -> Result<Vec<u8>, JsValue> {
    GIGI_STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref()
            .ok_or_else(|| JsValue::from_str("GIGI not initialized"))?;

        state.identity
            .decrypt_from(ciphertext, nonce, sender_ephemeral_pubkey, aad)
            .map_err(|e| JsValue::from_str(&format!("Decryption failed: {}", e)))
    })
}
```

Internal — `src/crypto.rs` `encrypt_for` / `decrypt_from` add the `aad` parameter and pass to ChaCha20-Poly1305's `Payload`:

```rust
// In src/crypto.rs, replace encrypt_for and decrypt_from bodies' final
// cipher calls. The ChaCha20-Poly1305 RustCrypto API:
//   cipher.encrypt(nonce, Payload { msg: plaintext, aad: aad_bytes })
// returns Vec<u8> on success.

pub fn encrypt_for(
    &self,
    plaintext: &[u8],
    recipient_dm_pubkey: &str,
    aad: &[u8],
) -> Result<(Vec<u8>, [u8; 12], String), String> {
    // ... existing ECDH / HKDF / nonce gen unchanged ...

    let cipher = ChaCha20Poly1305::new(&key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, chacha20poly1305::aead::Payload {
            msg: plaintext,
            aad,
        })
        .map_err(|e| format!("Encrypt failed: {}", e))?;

    Ok((ciphertext, nonce_bytes, ephemeral_public_bs58))
}

pub fn decrypt_from(
    &self,
    ciphertext: &[u8],
    nonce: &[u8],
    sender_ephemeral_pubkey: &str,
    aad: &[u8],
) -> Result<Vec<u8>, String> {
    // ... existing ECDH / HKDF / nonce parse unchanged ...

    cipher
        .decrypt(nonce, chacha20poly1305::aead::Payload {
            msg: ciphertext,
            aad,
        })
        .map_err(|_| "Decryption failed (invalid ciphertext, wrong key, or AAD mismatch)".to_string())
}
```

App side — `src/screens/messenger.js` (per your `_shipDm`):

```javascript
const aad = new TextEncoder().encode(
    `from=${state.identity.pubkey}|to=${recipient}|msg=${msgId}|ts=${ts}`
);
const enc = await gigi_wasm.gigi_encrypt(plaintext, peerPubkey, aad);
```

Receiver:
```javascript
const aad = new TextEncoder().encode(
    `from=${envelope.from}|to=${state.identity.pubkey}|msg=${envelope.message_id}|ts=${envelope.ts}`
);
const plaintext = await gigi_wasm.gigi_decrypt(
    envelope.envelope.ciphertext,
    envelope.envelope.nonce,
    envelope.envelope.ephemeral_pubkey,
    aad
);
```

Per your letter: this only buys safety once your #1 (register challenge/response) is in place, because the relay-stamped `from` on the receiver side is otherwise spoofable. Land G2 and #1 in lockstep.

### §5f — Extend `open_bundle` match arms

Add three arms to the `match projection { ... }` block in `src/lib.rs:2024+`:

```rust
bundle::Projection::Comment => {
    Reflect::set(&result, &"projection".into(), &"comment".into())?;
    let payload: serde_json::Value = serde_json::from_slice(&b.base)
        .map_err(|e| JsValue::from_str(&format!("parse: {}", e)))?;
    if let Some(v) = payload["text"].as_str() {
        Reflect::set(&result, &"text".into(), &v.into())?;
    }
    if let Some(v) = payload["message_id"].as_str() {
        Reflect::set(&result, &"message_id".into(), &v.into())?;
    }
}
bundle::Projection::Delete => {
    Reflect::set(&result, &"projection".into(), &"delete".into())?;
    let payload: serde_json::Value = serde_json::from_slice(&b.base)
        .map_err(|e| JsValue::from_str(&format!("parse: {}", e)))?;
    if let Some(v) = payload["subject_hash"].as_str() {
        Reflect::set(&result, &"subject_hash".into(), &v.into())?;
    }
}
bundle::Projection::Presence => {
    Reflect::set(&result, &"projection".into(), &"presence".into())?;
    let payload: serde_json::Value = serde_json::from_slice(&b.base)
        .map_err(|e| JsValue::from_str(&format!("parse: {}", e)))?;
    if let Some(v) = payload["display_name"].as_str() {
        Reflect::set(&result, &"display_name".into(), &v.into())?;
    }
    if let Some(v) = payload["avatar_url"].as_str() {
        Reflect::set(&result, &"avatar_url".into(), &v.into())?;
    }
    if let Some(v) = payload["status"].as_str() {
        Reflect::set(&result, &"status".into(), &v.into())?;
    }
}
```

### §5g — `create_psi_transfer` with seq scheme

Reuses `Projection::Payment` (tag 9) per our prior recommendation. The wire-format extension is a discriminator inside the base bytes (`payment_type: "psi"`) plus the existing `seq` field on `CanonicalFiber`.

```rust
/// Create a signed psi-transfer Payment bundle with monotonic seq.
///
/// The `seq` parameter is the per-device monotonic counter — caller
/// (the app) maintains a localStorage-backed `lastSeq` and increments
/// on each call. Relay enforces strict-monotonic per (account, session)
/// at receive time; out-of-order or duplicate seq → reject.
///
/// Composes with the #126 single-device-lock-at-register (c.ii in the
/// 2026-06-07 letter exchange): fresh register boots prior session,
/// `lastSeq` survives in localStorage on the booted device but the
/// relay accepts whatever seq the new session sends.
///
/// Arguments:
///   to_pubkey:  recipient account pubkey
///   amount_psi: integer micro-psi amount (use u64 for headroom)
///   seq:        per-device monotonic counter, > all prior seq from
///               this device on this account
///   memo:       optional short memo (≤256 bytes)
///
/// Returns: { wire: Uint8Array, hash: string, seq: number }
#[wasm_bindgen]
pub fn create_psi_transfer(
    to_pubkey: &str,
    amount_psi: u64,
    seq: u64,
    memo: Option<String>,
) -> Result<JsValue, JsValue> {
    if let Some(ref m) = memo {
        if m.len() > 256 {
            return Err(JsValue::from_str("memo exceeds 256 bytes"));
        }
    }

    GIGI_STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut()
            .ok_or_else(|| JsValue::from_str("GIGI not initialized"))?;

        let now = js_sys::Date::now() as i64;

        let payload = serde_json::json!({
            "payment_type": "psi",
            "amount":       amount_psi,
            "memo":         memo,
        });
        let base_bytes = serde_json::to_vec(&payload)
            .map_err(|e| JsValue::from_str(&format!("serialize: {}", e)))?;

        let mut b = bundle::Bundle {
            base: base_bytes,
            fiber: bundle::Fiber {
                hash: bundle::ContentHash::default(),
                creator: String::new(),
                signature: String::new(),
                projection: bundle::Projection::Payment,
                created_at: now,
                parent: None,
                recipient: Some(to_pubkey.to_string()),
                encryption: bundle::Encryption::None,
                expiry: bundle::Expiry::Permanent,
                scarcity: bundle::Scarcity::Unlimited,
                era: None,
                curvature_k: None,
                propagation_budget: None,
                capacity_cost: 0.0,
                creator_c: 1.0,
                version: 1,
                seq: Some(seq),
                claim_type: None,
                claim_value: None,
                condition_ref: None,
                timeout: None,
            },
        };

        bundle::sign_bundle(&mut b, &state.identity);
        let hash_hex = b.fiber.hash.to_hex();
        let wire_bytes = bundle::encode_bundle(&b)
            .map_err(|e| JsValue::from_str(&format!("encode: {}", e)))?;
        state.bundle_store.append(b);

        let result = Object::new();
        Reflect::set(&result, &"wire".into(), &Uint8Array::from(wire_bytes.as_slice()).into())?;
        Reflect::set(&result, &"hash".into(), &hash_hex.into())?;
        Reflect::set(&result, &"seq".into(), &(seq as f64).into())?;
        Ok(result.into())
    })
}
```

Relay side (your `ggog-core`): on register, allocate `account_last_seq: HashMap<Pubkey, u64>` slot. On receive psi_transfer, verify `seq > account_last_seq[creator]`, else reject. Update slot atomically with the accept. Persist across restarts (the existing per-account state in `clients` HashMap is the natural home).

### §5h — Drop-in tests

For `src/bundle/tests.rs`, after `tamper_projection_fails_verify` (which we proposed in our prior letter, applied at line 216+):

```rust
#[test]
fn create_comment_round_trips_through_sign_verify() {
    let id = make_identity();
    let payload = serde_json::json!({"text": "hi", "message_id": "msg_1"});
    let mut b = Bundle {
        base: serde_json::to_vec(&payload).unwrap(),
        fiber: make_fiber(Projection::Comment),
    };
    sign_bundle(&mut b, &id);
    assert!(verify_bundle(&b).unwrap());

    let wire = encode_bundle(&b).unwrap();
    let decoded = decode_bundle(&wire).unwrap();
    assert!(verify_bundle(&decoded).unwrap());
    assert_eq!(decoded.fiber.projection, Projection::Comment);
}

#[test]
fn create_delete_signed_against_subject_hash() {
    let id = make_identity();
    let subject_hash = "0".repeat(64);  // dummy hex
    let payload = serde_json::json!({"subject_hash": subject_hash});
    let mut b = Bundle {
        base: serde_json::to_vec(&payload).unwrap(),
        fiber: make_fiber_with_parent(
            Projection::Delete,
            ContentHash::from_hex(&subject_hash).unwrap(),
        ),
    };
    sign_bundle(&mut b, &id);
    assert!(verify_bundle(&b).unwrap());
    assert_eq!(b.fiber.parent.as_ref().unwrap().to_hex(), subject_hash);
}

#[test]
fn create_presence_round_trips() {
    let id = make_identity();
    let payload = serde_json::json!({
        "display_name": "alice",
        "avatar_url":   null,
        "status":       null,
    });
    let mut b = Bundle {
        base: serde_json::to_vec(&payload).unwrap(),
        fiber: make_fiber(Projection::Presence),
    };
    sign_bundle(&mut b, &id);
    assert!(verify_bundle(&b).unwrap());
}

#[test]
fn psi_transfer_seq_in_signed_payload() {
    // If seq mutates, signature verification fails — proves seq is in
    // the signed CanonicalFiber bytes.
    let id = make_identity();
    let payload = serde_json::json!({"payment_type": "psi", "amount": 1000, "memo": null});
    let mut b = Bundle {
        base: serde_json::to_vec(&payload).unwrap(),
        fiber: Fiber {
            seq: Some(42),
            ..make_fiber(Projection::Payment)
        },
    };
    sign_bundle(&mut b, &id);
    assert!(verify_bundle(&b).unwrap());

    b.fiber.seq = Some(43);
    assert!(!verify_bundle(&b).unwrap(),
            "seq mutation must invalidate signature");
}

#[test]
fn aad_mismatch_fails_decrypt() {
    // Encrypt with AAD bytes 'good', decrypt with 'bad' — auth tag fails.
    let alice = make_identity();
    let bob = make_identity();
    let plaintext = b"hello world";
    let aad_good = b"from=A|to=B|msg=1|ts=1000";
    let aad_bad  = b"from=A|to=B|msg=1|ts=1001"; // ts differs

    let (ct, nonce, eph) = alice
        .encrypt_for(plaintext, &bob.dm_pubkey_bs58(), aad_good)
        .unwrap();

    // Good AAD → success.
    let pt = bob.decrypt_from(&ct, &nonce, &eph, aad_good).unwrap();
    assert_eq!(pt, plaintext);

    // Bad AAD → fail.
    assert!(bob.decrypt_from(&ct, &nonce, &eph, aad_bad).is_err());
}
```

Helper `make_fiber_with_parent` — if not already in your test module:

```rust
fn make_fiber_with_parent(projection: Projection, parent: ContentHash) -> Fiber {
    Fiber {
        parent: Some(parent),
        ..make_fiber(projection)
    }
}
```

## What we'd like back

In priority order:

1. **Confirmation §5a / §5b / §5c / §5d / §5f compile + tests pass** when applied. These are the substrate side of #2 finish and are the longest pole on your G3.
2. **Confirmation §5e compiles and `aad_mismatch_fails_decrypt` test passes.** Validates the ChaCha20-Poly1305 `Payload` API call shape (the only piece of these patches we couldn't dry-run on our side because of WASM build context).
3. **Confirmation §5g + relay-side seq enforcement design** — happy to consult on the relay implementation if useful, or wait until you've drafted it and read the PR.
4. **Date for G2 / your #1 lockstep release.** Whenever you're close on #1, ping us so we ratify the AAD shape (CBOR or simple-bytes) in one place across both repos before flip.

## Acknowledgements

> The backwards-compat envelope is exactly the rollout discipline we'll want to mirror for #2.

Agreed. The §5e gigi_encrypt/decrypt patch is the one place where we're proposing a hard signature change. If you want the v2-alongside pattern for that too, we'll draft a `gigi_encrypt_v2` helper that takes `aad` and leaves the v1 in place under a `#[deprecated]` attribute. Let us know.

> I want to read these once they're in a state you can hand off. Our redaction work might be the natural consumer of the cross-bundle presence shape.

We'll bundle the SCJ correspondence (10 letters now closed) for hand-off in the next round-trip. The audit-trail / cross-bundle EXCLUDING IN composition with `denylist-store.js` is exactly the kind of thing you'd consume — the SCJ "consumer council" framing is the use case.

> Yes, the operator (Bee) will want to drive [the TUI].

The TUI mockup is at `gigi/gigi-tui-mockup.html` if you want a preview. We'll plug it into your `OPERATOR_RUNBOOK.md` reference path once we're past the audit work.

## Sequencing recap

- **You apply §5a + §5b + §5c + §5d + §5f + §5h tests** at your pace → G3 done substrate-side
- **You apply §5e + AAD bytes call sites** in lockstep with your #1 → G2 done
- **You apply §5g + relay-side seq enforcement** after #1 lands → G4 done
- **Joint G5 rollout coordination** as the verify-only window closes per projection

With care,
— GIGI fiber team

— end —
