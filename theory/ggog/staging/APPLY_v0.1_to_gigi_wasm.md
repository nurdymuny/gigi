# APPLY v0.1 — gigi-wasm GGOG protocol audit ship

**Target:** `gigi-wasm` working copy (whatever path you operate from).
**Source letters:**
- `theory/ggog/LETTER_TO_GGOG_2026-06-07_G1_REPLY.md` (case A confirmation + `tamper_projection_fails_verify`)
- `theory/ggog/LETTER_TO_GGOG_2026-06-09_e_f_REPLY.md` (§5a–h: helpers + AAD + tests)
- `theory/ggog/LETTER_TO_GGOG_2026-06-09_challenge_REPLY.md` (§6b: `sign_register_challenge`)

**Tracks:**
- GGOG sprint items #2 (signed projections finish), #3 (psi_transfer), #9 (DM AAD)
- Their #1 register-challenge counterpart (§6b is our half of the pair)

**Output artifact:** new wasm bundle GGOG pulls; once shipped + verify-only metrics drain, they flip `GGOG_REQUIRE_CHALLENGE=1`.

---

## Pre-flight

1. **Verify clean baseline.** From your gigi-wasm working copy:
   ```bash
   cargo test --lib  # should be 37/37 before this patch set
   ```
   Note: a previous baseline letter said "38/38 with the G1 `tamper_projection_fails_verify` applied" — that test is NOT yet in the tree (confirmed by grep this session). After Step 8 below it should hit 38, then climb as the §5h tests land.

2. **OneDrive PDB workaround if you're on Windows.** MSVC's `LNK1201` writes failed on OneDrive-backed `target/` last time. Route the target dir off OneDrive:
   ```bash
   export CARGO_TARGET_DIR="$LOCALAPPDATA/cargo-target/gigi-wasm"
   ```
   (PowerShell equivalent: `$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\cargo-target\gigi-wasm"`)

3. **Apply order matters.** §5a must land before §5b/§5c/§5d/§5f (they reference its new variants). §5e is BREAKING — apply last so intermediate test runs aren't blocked by call-site fallout. Order below is dependency-correct.

---

## Step 1 — §6b: `sign_register_challenge` (low-risk, isolated)

**File:** `src/lib.rs`
**Where:** anywhere after `pub fn gigi_decrypt` (around line 301). Suggest right after `gigi_decrypt` ends so the crypto-adjacent surface is grouped.
**What:** add new `#[wasm_bindgen]` function.

**Append:**

```rust
/// Sign the relay's register-challenge nonce with the account key.
///
/// Per the 2026-06-09 GGOG protocol-audit letter exchange: the relay
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

**Sanity check after this step:**
```bash
cargo build --lib   # should compile clean
```

---

## Step 2 — §5a: `Projection` enum + tag table extension

**File:** `src/bundle/types.rs`

### 2a. Add three variants at end of `enum Projection` (line 49-70)

**Locate:**
```rust
pub enum Projection {
    Video,
    Message,
    // ... existing 17 variants ...
    Redaction,
}
```

**Replace with:**
```rust
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
    /// Operator-authority redaction (tag 17). Bundle's signer MUST
    /// be the network operator pubkey for clients/relays to honor it.
    /// See ggog-core for the full design; this enum tracks parity.
    Redaction,
    /// User-side comment on a parent post (tag 18). Distinct from
    /// `Reaction` — carries free-form text in the base bytes.
    /// GGOG protocol audit #2 finish — landed 2026-06-09.
    Comment,
    /// User-side soft delete of own content (tag 19). Distinct from
    /// `Redaction` which is operator-authority; receivers honor
    /// `Delete` only when `fiber.creator == subject_creator`.
    Delete,
    /// Event-driven presence announcement (tag 20). No heartbeat —
    /// signed once per (WS open / profile edit / re-announce handshake).
    /// GGOG protocol audit #2 finish — landed 2026-06-09.
    Presence,
}
```

### 2b. Update `to_tag` (line 73 area)

**Locate the existing `match self {` body in `to_tag`** and append three arms before the closing brace:

```rust
            Projection::Redaction => 17,
            Projection::Comment => 18,     // NEW
            Projection::Delete => 19,      // NEW
            Projection::Presence => 20,    // NEW
        }
    }
```

### 2c. Update `from_tag` (line 95 area)

Append three arms before the catch-all `_ => None`:

```rust
            17 => Some(Projection::Redaction),
            18 => Some(Projection::Comment),     // NEW
            19 => Some(Projection::Delete),      // NEW
            20 => Some(Projection::Presence),    // NEW
            _ => None,
        }
    }
```

**Sanity check after this step:**
```bash
cargo build --lib   # should compile clean; existing match arms warn-not-error if exhaustive
```

If you hit "non-exhaustive match" errors anywhere else in the codebase, add the three new arms wherever the compiler points (they shouldn't exist in the wild but the strict-additive variant addition is the safe baseline).

---

## Step 3 — §5f: `open_bundle` match arms for the new variants

**File:** `src/lib.rs`
**Where:** `open_bundle` function (line 1993+). Find the big `match projection { ... }` block (around line 2024+) and append three new arms before the closing brace.

### 3a. Append after the existing `Projection::Redaction => { ... }` arm

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

**Sanity check after this step:**
```bash
cargo build --lib   # compile clean
```

---

## Step 4 — §5b: `create_comment` helper

**File:** `src/lib.rs`
**Where:** right after `create_reaction` ends (line 1428).

**Append:**

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

---

## Step 5 — §5c: `create_delete` helper

**File:** `src/lib.rs`
**Where:** right after `create_comment`.

**Append:**

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

---

## Step 6 — §5d: `create_presence` helper (event-driven)

**File:** `src/lib.rs`
**Where:** right after `create_delete`.

**Append:**

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

---

## Step 7 — §5g: `create_psi_transfer` (reuses Payment tag 9)

**File:** `src/lib.rs`
**Where:** right after `create_presence`.

**Append:**

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

**Sanity check after Steps 4–7:**
```bash
cargo build --lib   # all four helpers compile against the new Projection variants
```

---

## Step 8 — §5h tests: bundle round-trip + projection tamper

**File:** `src/bundle/tests.rs`
**Where:** inside `mod tests`, after `tamper_fiber_fails_verify` (currently line 207-ish). The existing `make_identity`, `make_fiber` helpers are reused.

### 8a. Add `make_fiber_with_parent` helper if not already present

**Locate `fn make_fiber(projection: Projection) -> Fiber` (around line 336)** and add a sibling helper right after it:

```rust
    fn make_fiber_with_parent(projection: Projection, parent: ContentHash) -> Fiber {
        Fiber {
            parent: Some(parent),
            ..make_fiber(projection)
        }
    }
```

### 8b. Add the G1 invariant test (from 2026-06-07 letter)

**Insert after `tamper_fiber_fails_verify`:**

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
    #[test]
    fn tamper_projection_fails_verify() {
        let id = make_identity();
        let mut b = Bundle {
            base: b"reaction payload".to_vec(),
            fiber: make_fiber(Projection::Reaction),
        };
        sign_bundle(&mut b, &id);
        assert!(verify_bundle(&b).unwrap(),
                "freshly-signed bundle must verify before tampering");

        b.fiber.projection = Projection::Message;
        assert!(!verify_bundle(&b).unwrap(),
                "projection mutation must invalidate signature (G1 invariant)");

        let mut b2 = Bundle {
            base: b"message payload".to_vec(),
            fiber: make_fiber(Projection::Message),
        };
        sign_bundle(&mut b2, &id);
        b2.fiber.projection = Projection::Reaction;
        assert!(!verify_bundle(&b2).unwrap(),
                "projection mutation in either direction must invalidate");
    }
```

### 8c. Add the four new-helper round-trip tests

**Append after `tamper_projection_fails_verify`:**

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
        let subject_hash = "0".repeat(64);
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
```

### 8d. Add the §6b round-trip test

**Append after `psi_transfer_seq_in_signed_payload`:**

```rust
    #[test]
    fn sign_register_challenge_round_trips_with_domain_separator() {
        // The §6b shape from the 2026-06-09 challenge-reply letter:
        // domain-separated nonce signing for GGOG register challenge.
        // Verifier-side mirror lives in ggog-core (§6c, landed
        // 2026-06-09, 17/17 node-bin tests green).
        use ed25519_dalek::{Signature, Verifier};

        let id = make_identity();
        let nonce = [42u8; 32];

        const DOMAIN: &[u8] = b"ggog/register-challenge/v1\n";
        let mut msg = Vec::new();
        msg.extend_from_slice(DOMAIN);
        msg.extend_from_slice(&nonce);

        let sig_hex = id.sign_hex(&msg);
        let sig_bytes = hex::decode(&sig_hex).expect("valid hex");
        let sig = Signature::from_slice(&sig_bytes).expect("64-byte sig");

        let vk = id.verifying_key();   // assumes Identity exposes this; if
                                       // not, derive from id.pubkey_base58().
        assert!(vk.verify(&msg, &sig).is_ok(),
                "domain-separated nonce signature must verify");
    }
```

> **If `Identity::verifying_key()` doesn't exist:** add a small accessor in `src/crypto.rs`:
> ```rust
> pub fn verifying_key(&self) -> ed25519_dalek::VerifyingKey {
>     self.signing_key.verifying_key()
> }
> ```
> Apply as a sibling to the existing `pub fn sign_hex(&self, ...) -> String` (line 50-53).

**Sanity check after Step 8:**
```bash
cargo test --lib 2>&1 | tail -20
```
Expect: ~43 tests passing (37 baseline + 6 new = `tamper_projection_fails_verify` + 4 round-trip + sign_register_challenge).

---

## Step 9 — §5e: `gigi_encrypt` / `gigi_decrypt` AAD param (BREAKING)

This is the breaking-API change. Apply last because every existing call site must update together.

### 9a. Update `src/crypto.rs` — `encrypt_for` / `decrypt_from`

**Locate `pub fn encrypt_for` (around line 56)** and add an `aad: &[u8]` parameter. The final ChaCha20-Poly1305 call switches from `cipher.encrypt(nonce, plaintext)` to the `Payload` form:

```rust
    pub fn encrypt_for(
        &self,
        plaintext: &[u8],
        recipient_dm_pubkey: &str,
        aad: &[u8],                              // NEW PARAM
    ) -> Result<(Vec<u8>, [u8; 12], String), String> {
        // ... existing ECDH / HKDF / nonce gen unchanged through line ~88 ...

        // Encrypt with ChaCha20-Poly1305, AAD-bound
        let cipher = ChaCha20Poly1305::new(&key);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, chacha20poly1305::aead::Payload {
                msg: plaintext,
                aad,
            })
            .map_err(|e| format!("Encrypt failed: {}", e))?;

        // ephemeral_public stringification unchanged
        let ephemeral_pubkey = bs58::encode(ephemeral_public.as_bytes()).into_string();
        Ok((ciphertext, nonce_bytes, ephemeral_pubkey))
    }
```

And `decrypt_from`:

```rust
    pub fn decrypt_from(
        &self,
        ciphertext: &[u8],
        nonce: &[u8],
        sender_ephemeral_pubkey: &str,
        aad: &[u8],                              // NEW PARAM
    ) -> Result<Vec<u8>, String> {
        // ... existing ECDH / HKDF / nonce parse unchanged through line ~130 ...

        cipher
            .decrypt(nonce, chacha20poly1305::aead::Payload {
                msg: ciphertext,
                aad,
            })
            .map_err(|_| "Decryption failed (invalid ciphertext, wrong key, or AAD mismatch)".to_string())
    }
```

### 9b. Update `src/lib.rs` — `gigi_encrypt` / `gigi_decrypt`

**Replace** `pub fn gigi_encrypt` (line 266) and `pub fn gigi_decrypt` (line 287):

```rust
/// Encrypt a message with AAD binding the ciphertext to
/// (from, to, message_id, ts).
///
/// Canonical AAD construction (caller responsibility):
///   aad_bytes = b"from=" || hex(from) || b"|to=" || hex(to)
///            || b"|msg=" || message_id || b"|ts=" || to_string(ts)
///
/// See §5e of LETTER_TO_GGOG_2026-06-09_e_f_REPLY.md for the security
/// argument and the call-site shape in messenger.js.
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

### 9c. AAD round-trip test

**Add to `src/bundle/tests.rs` (or wherever `crypto::tests` lives):**

```rust
    #[test]
    fn aad_mismatch_fails_decrypt() {
        // Encrypt with one AAD, attempt decrypt with a different AAD —
        // auth tag MUST fail. The §5e gate.
        let (alice, _seed_a) = generate_new_identity().unwrap();
        let (bob, _seed_b) = generate_new_identity().unwrap();
        let plaintext = b"hello world";
        let aad_good = b"from=A|to=B|msg=1|ts=1000";
        let aad_bad  = b"from=A|to=B|msg=1|ts=1001"; // ts differs

        let (ct, nonce, eph) = alice
            .encrypt_for(plaintext, &bob.dm_pubkey_base58(), aad_good)
            .unwrap();

        // Good AAD → success.
        let pt = bob.decrypt_from(&ct, &nonce, &eph, aad_good).unwrap();
        assert_eq!(pt, plaintext);

        // Bad AAD → fail.
        assert!(bob.decrypt_from(&ct, &nonce, &eph, aad_bad).is_err(),
                "AAD mismatch must fail decrypt");
    }
```

> If `dm_pubkey_base58()` is named differently in your crypto.rs, swap to the actual accessor. The test is otherwise self-contained.

**Sanity check after Step 9:**
```bash
cargo test --lib 2>&1 | tail -20
```
Expect: 44 tests (43 + `aad_mismatch_fails_decrypt`).

If you have call sites in `gigi-wasm` itself that call `gigi_encrypt` / `gigi_decrypt` from internal Rust code (rare for a wasm-bindgen entry point), update them to pass an `aad: &[]` empty slice. Most call sites are JS-side and don't compile-break — they'll RuntimeError until the messenger.js wiring lands (see §5e in the e/f letter for that snippet).

---

## Final gate

```bash
# All lib tests
cargo test --lib 2>&1 | tail -5

# Build the wasm bundle
wasm-pack build --target web --release   # or whatever your existing build command is
```

**Test floor:** baseline 37 + 7 new = **44 tests minimum** post-apply.

**Wasm artifact:** ship the `pkg/` output wherever GGOG pulls from (their `connection.js` waits for `sign_register_challenge` to exist on the wasm module; their `messenger.js` will wire AAD after).

---

## What goes back to GGOG when shipped

A short status letter at `theory/ggog/LETTER_TO_GGOG_<date>_shipped.md` saying:

- gigi-wasm now carries §5a–§5h + §6b
- Test count + green confirmation
- New wasm bundle hash / version / publication path
- Cue them to: drop the `connection.js` snippet, smoke-test, drain verify-only window, flip `GGOG_REQUIRE_CHALLENGE=1`

That closes the loop on G2 + half of #1; the rest of G3/G4/G5 follows as the rollout window drains.

---

## Notes / known unknowns

- The `Identity::verifying_key()` accessor in §8d is conditional — if it doesn't exist, the 3-line addition in §6b's note slots it in.
- The `dm_pubkey_base58()` name in §9c is a guess — adjust to whatever the actual accessor is in your `src/crypto.rs`.
- Whether `connection.js` wiring lands in the same gigi-wasm artifact PR or a follow-up — GGOG team noted they're holding the wiring until your build ships, so the order is yours.
- The §5e API break: if you want a `gigi_encrypt_v2` co-existing instead (per the GGOG team's backwards-compat envelope discipline), let me know and I'll restructure §5e to add a v2 helper instead of breaking the v1 sig.
