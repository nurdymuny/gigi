# GGOG × GIGI Chat Projection Contract — v1

**Date:** 2026-04-14  
**Owners:** GGOG Copilot + GIGI DB Team  
**Status:** Proposed contract v1 — NOT current GGOG implementation. Section 4 explicitly separates GGOG's live wire format from the proposed v1 target.

---

## 1. Scope and Principle

This document defines the shared bundle schema contract for chat-related projections carried over the GIGI/DHOOM storage and transport layer.

**GIGI DB is not modified.** The engine stores, queries, and streams any bundle regardless of domain. This document specifies only what field names and types GGOG and Just Gigi clients agree to use when creating chat bundles — the same way any application defines its own table schema on top of a general-purpose database.

The bundle is the source of truth. Transport routing is handled by a thin wire frame (Section 4). No application semantics live in the transport.

---

## 2. Value Type Reference

All field values use GIGI's `Value` enum from `src/types.rs`. Permitted types for chat fields:

| GIGI Type         | Rust variant         | Use in chat                                    |
|-------------------|----------------------|------------------------------------------------|
| `Integer(i64)`    | `Value::Integer`     | sequence numbers, type IDs                    |
| `Text(String)`    | `Value::Text`        | IDs, message body, status enums               |
| `Timestamp(i64)`  | `Value::Timestamp`   | nanosecond Unix epoch timestamps               |
| `Bool(bool)`      | `Value::Bool`        | `encrypted`, `edited`, `removed` flags        |
| `Binary(Vec<u8>)` | `Value::Binary`      | encrypted payload bytes, voice note blobs     |
| `Null`            | `Value::Null`        | absent optional fields                        |

`Value::Binary` is a first-class runtime variant as of commit `e486b55`. It is WAL-serialised with tag `0x07` and length-prefixed raw bytes. `FieldType::Binary` in the schema is the declaration; `Value::Binary(Vec<u8>)` is the storage value.

**Bool constraint:** `Value::Bool` is a first-class variant in GIGI's value system. All boolean fields in this contract store `Value::Bool(true/false)`. DHOOM's wire encoding coerces `T`/`F` tokens to `Value::Bool` automatically. `FieldType` has no dedicated Boolean variant — declare boolean fields as `FieldType::Categorical` with `default: Value::Bool(false)`. Do not store booleans as `Text("T")` or `Text("F")`.

`Vector` and `Numeric` types are available but not used in the base chat contract. GGOG may use `Vector` for semantic embedding on message content in an extended schema.

---

## 2.1 Binary Field Boundary Convention

This section is a **protocol rule**, not an implementation detail. Both sides must treat it as binding.

### Representation at JSON API edges

When a `Value::Binary` field crosses a JSON boundary (HTTP request/response, NDJSON stream), it is encoded as a JSON string with the prefix `b64:` followed by standard base64 (RFC 4648, no line breaks):

```
"media_bytes": "b64:AAEC/w=="
```

The `b64:` prefix is the sole discriminator. The receiving side decodes any string with this prefix as `Value::Binary` unconditionally.

### Collision policy for plain strings

The prefix is **authoritative regardless of schema**. This is not a soft reservation — it is an unconditional decode rule:

> **Rule:** Any value in any field in any family that begins with `b64:` is decoded as `Value::Binary` at every JSON boundary, regardless of the field's declared `FieldType`. There are no exceptions and no schema-based override. User-controlled text that might begin with `b64:` — in a message body, a display name, a status string, anywhere — MUST be escaped at the application layer before writing to any GIGI field. The engine does not enforce this. A missed escape is silent data corruption.

This constraint applies globally, not only to fields declared `FieldType::Binary`.

### Storage and WAL

Inside the GIGI engine — WAL, mmap snapshot, in-memory BundleStore — binary fields are raw `Vec<u8>`. The `b64:` prefix never appears at rest. It is applied only at serialisation time by `value_to_json` and stripped only by `json_to_value`.

### DHOOM transport

DHOOM encodes binary fields as raw bytes in the fiber section. The `b64:` prefix is **not used** in DHOOM payloads. Peers communicating via `application/dhoom` carry binary at full fidelity with no encoding overhead.

> `b64:` is an API-edge bridge for JSON surfaces. It is not the conceptual model for peer-to-peer transport.

### Size guidance (provisional)

| Payload type             | Recommended max per field | Strategy above limit          |
|--------------------------|--------------------------|-------------------------------|
| Encrypted message body   | 64 KB                    | Reject at ingest with 413     |
| Voice note (inline)      | 256 KB                   | Use `media_ref` + out-of-band blob store instead |
| Any binary field         | 1 MB absolute cap        | Gigi ingest returns 413       |

Exact limits are implementation policy, not schema — the engine does not enforce them today. These are the recommended values pending §8.8 discussion.

---

## 3. Field Structure

### 3.1 Universal Fields (required on every chat bundle)

These three fields appear on every event family without exception. They are `base_fields` in the `BundleSchema`.

| Field             | FieldType     | Value type       | Description                                  |
|-------------------|---------------|------------------|----------------------------------------------|
| `projection_type` | `Categorical` | `Text`           | Event family name from §3.3 namespace        |
| `sender_id`       | `Categorical` | `Text`           | Sender peer identity (opaque, client-defined)|
| `timestamp_ns`    | `Timestamp`   | `Timestamp(i64)` | Send time, nanosecond Unix epoch             |

### 3.2 Common Routing Fields (per-family, not universal)

These fields are required by most families but not all. Each family's table in §3.3 explicitly lists which are required vs optional for that family.

| Field            | FieldType     | Value type | Description                               |
|------------------|---------------|------------|-------------------------------------------|
| `message_id`     | `Categorical` | `Text`     | Unique message identifier (UUID or hash)  |
| `recipient_id`   | `Categorical` | `Text`     | Recipient peer or group identity          |
| `conversation_id`| `Categorical` | `Text`     | Thread or channel scoping key             |

### 3.3 Event Families

#### `chat/dm` — Direct Message

| Field            | Required | Value type | Description                                     |
|------------------|----------|------------|-------------------------------------------------|
| `message_id`     | yes      | `Text`     | Unique message ID                               |
| `recipient_id`   | yes      | `Text`     | Recipient identity                              |
| `conversation_id`| yes      | `Text`     | Thread scoping key                              |
| `body`           | yes      | `Text` or `Binary` | Message text, or encrypted ciphertext when `encrypted=true` |
| `encrypted`      | yes      | `Bool`     | True if `body` carries encrypted bytes          |

**Binary body convention:** `body` has two mutually exclusive runtime types depending on `encrypted`. Clients MUST implement both branches explicitly:

1. Read `encrypted` first.
2. If `encrypted = false`: `body` is `Value::Text`. Render directly.
3. If `encrypted = true`: `body` is `Value::Binary` (raw ciphertext). Pass to the decryption layer. Do NOT attempt to render as text.

Treating `body` as always-renderable text is a defect — it will produce garbage or crash on any encrypted record. At JSON API boundaries, an encrypted body arrives as `"body": "b64:..."` per §2.1; the receiving client decodes the base64 and then decrypts.
| `media_ref`      | no       | `Text`     | Reference key for attached media (not inline)   |
| `reply_to`       | no       | `Text`     | `message_id` of the message being replied to    |
| `edited`         | no       | `Bool`     | True if this is an edit of a prior message      |

#### `chat/signal` — Call Signal

| Field          | Required | Value type | Description                                          |
|----------------|----------|------------|------------------------------------------------------|
| `recipient_id` | yes      | `Text`     | Call recipient                                       |
| `call_id`      | yes      | `Text`     | Stable ID for the call session                       |
| `signal_type`  | yes      | `Text`     | `"offer"`, `"answer"`, `"ice"`, `"reject"`, `"end"`, `"busy"` |
| `sdp`          | no       | `Text`     | SDP payload for offer/answer                         |
| `ice_candidate`| no       | `Text`     | ICE candidate string for `"ice"` signals             |
| `media_type`   | no       | `Text`     | `"audio"`, `"video"`, `"screen"` — absent = audio    |

`message_id` is not used on signal events. Call identity is tracked by `call_id`.

#### `chat/reaction` — Reaction Event

| Field            | Required | Value type | Description                                      |
|------------------|----------|------------|--------------------------------------------------|
| `target_id`      | yes      | `Text`     | `message_id` of the message being reacted to     |
| `emoji`          | yes      | `Text`     | Unicode string, e.g. `"👍"`                      |
| `action`         | yes      | `Text`     | `"add"` or `"remove"`                            |
| `conversation_id`| no       | `Text`     | Optional thread context                          |

#### `chat/ack` — Delivery / Read Acknowledgement

| Field            | Required | Value type | Description                                      |
|------------------|----------|------------|--------------------------------------------------|
| `target_id`      | yes      | `Text`     | `message_id` being acknowledged                  |
| `ack_type`       | yes      | `Text`     | `"delivered"` or `"read"`                        |
| `recipient_id`   | yes      | `Text`     | Original sender of the acknowledged message      |
| `conversation_id`| no       | `Text`     | Optional thread context                          |

#### `chat/typing` — Ephemeral Typing Indicator

| Field            | Required | Value type | Description                                      |
|------------------|----------|------------|--------------------------------------------------|
| `recipient_id`   | yes      | `Text`     | Recipient seeing the indicator                   |
| `state`          | yes      | `Text`     | `"start"` or `"stop"`                           |
| `conversation_id`| no       | `Text`     | Optional thread context                          |

`message_id` is not applicable. Typing events are not persisted (see §5).

#### `chat/voice_note` — Voice Note

| Field            | Required | Value type  | Description                                     |
|------------------|----------|-------------|-------------------------------------------------|
| `message_id`     | yes      | `Text`      | Unique message ID                               |
| `recipient_id`   | yes      | `Text`      | Recipient identity                              |
| `conversation_id`| yes      | `Text`      | Thread scoping key                              |
| `media_ref`      | yes*     | `Text`      | Reference key to out-of-band voice blob         |
| `media_bytes`    | yes*     | `Binary`    | Inline voice blob bytes (small payloads only)   |
| `duration_ms`    | yes      | `Integer`   | Duration in **milliseconds**                    |
| `encrypted`      | yes      | `Bool`      | True if blob or reference is encrypted          |
| `waveform`       | no       | `Text`      | Serialized waveform hint for UI rendering       |

\* Exactly one of `media_ref` or `media_bytes` must be present. `media_bytes` MUST NOT exceed 256 KB per §2.1 size guidance. Senders SHOULD prefer `media_ref` + out-of-band blob storage for production payloads. `media_bytes` is provided for small/test payloads and interop fixtures.

---

## 4. Wire Frame

### 4.1 GGOG Current Wire Format (live today)

GGOG currently ships a lightweight binary frame:

```
[tag: 1 byte = 0x01][type_len: 1 byte][type_bytes: N][to_len: 1 byte][to_bytes: M][payload_bytes]
```

- `0x00` tag = JSON fallback frame
- `0x01` tag = DHOOM binary frame
- `type` = projection type string (UTF-8)
- `to` = recipient identifier string (UTF-8)
- `payload` = DHOOM bundle bytes

Relay nodes decode `type` and `to` for routing. Sender identity is currently injected into the payload by relays (the mutation GGOG wants to eliminate).

### 4.2 Proposed v1 GIGI Wire Frame (not yet implemented)

The v1 frame moves sender identity into the header and adds an Ed25519 signature so relays can verify provenance without payload access. This is the target, not the current state.

```
┌───────────────────────────────────────────────────┐
│  MAGIC       4 bytes   0x47494749 ("GIGI")         │
│  VERSION     1 byte    0x01                        │
│  MSG_TYPE    1 byte    see §4.3                    │
│  FRAME_LEN   4 bytes   total frame length (u32 BE) │
│  SENDER      32 bytes  sender Ed25519 public key   │
│  RECIPIENT   32 bytes  SHA-256(recipient pubkey)   │
│  SIG         64 bytes  Ed25519 signature (see §4.4)│
│  PAYLOAD     N bytes   DHOOM bundle bytes          │
└───────────────────────────────────────────────────┘
```

Total header overhead: **138 bytes**.

### 4.3 MSG_TYPE Values (u8)

| Value  | Name         | Description                               |
|--------|--------------|-------------------------------------------|
| `0x01` | `DM`         | Direct message bundle                     |
| `0x02` | `SIGNAL`     | Call signal bundle                        |
| `0x03` | `REACTION`   | Reaction bundle                           |
| `0x04` | `ACK`        | Delivery/read ack bundle                  |
| `0x05` | `TYPING`     | Ephemeral typing indicator bundle         |
| `0x06` | `VOICE_NOTE` | Voice note bundle                         |
| `0xFF` | `RELAY`      | Relay-only routing packet (no app bundle) |

The `MSG_TYPE` byte is the canonical compact type discriminator on hot relay paths. Clients decode `projection_type` from the bundle payload on receipt — relays use the byte.

### 4.4 Signature Model

`SENDER` = the sender's 32-byte Ed25519 public key (not a hash — the key itself is needed for verification).

`SIG` = Ed25519 signature over the 106 bytes `[MAGIC..RECIPIENT]` (all header fields excluding `SIG` itself), signed with the sender's Ed25519 private key.

Any relay or recipient holding the sender's public key can verify the signature without a shared secret. This eliminates relay-side payload mutation: relays route on `RECIPIENT`, forward `PAYLOAD` untouched, and optionally verify `SIG` against `SENDER` without parsing the bundle.

### 4.5 Content-Type

- Modern peers (GGOG, Just Gigi desktop): `application/dhoom`
- Fallback for clients that cannot decode DHOOM: `application/x-ndjson` via `POST /v1/bundles/{name}/query-stream`
- JSON is not used on peer-to-peer paths.

---

## 5. Ephemeral Event Policy

`chat/typing` events:
- **Not persisted** to the GIGI WAL or mmap store.
- Coalesced at the relay: if two `start` events arrive for the same `(sender_id, recipient_id)` within 2 seconds, only the first is forwarded.
- TTL: 5 seconds. Relay discards if not delivered within TTL.

`chat/ack`:
- `ack_type = "delivered"` is persisted.
- `ack_type = "read"` is persisted with a shorter retention window (policy TBD by GGOG — open item §8.4).

---

## 6. Field Migration Map — GGOG Current → v1 Contract

This table exists because GGOG's live field names differ from the v1 contract in several places. Neither side should assume the other already uses v1 names.

| GGOG Current Field  | v1 Contract Field    | Family          | Change                                           |
|---------------------|----------------------|-----------------|--------------------------------------------------|
| `text`              | `body`               | chat/dm         | Rename                                           |
| `gigi_envelope`     | *(removed)*          | all             | Bundle IS the envelope — no nested wrapper       |
| `dm_pubkey`         | Frame `SENDER` (§4.2)| all             | **v1 target only:** GGOG currently injects this into the payload. In v1 it moves to the wire frame `SENDER` field (32-byte Ed25519 pubkey) so relays never touch the bundle. |
| `reply_to`          | `reply_to`           | chat/dm         | No change                                        |
| `reply_to_text`     | *(removed)*          | chat/dm         | Denormalized; look up via `reply_to` message_id  |
| `candidate`         | `ice_candidate`      | chat/signal     | Rename                                           |
| `duration_secs`     | `duration_ms`        | chat/voice_note | Unit change: multiply by 1000                    |
| `mime_type`         | *(not in base contract)* | chat/voice_note | Not included because `projection_type = "chat/voice_note"` already implies audio media. If GGOG needs to distinguish codec formats (e.g. `opus` vs `aac`), add `codec` as an optional fiber field — do not re-add `mime_type` without confirming GGOG still sends it today. |
| `is_typing: bool`   | `state: "start"/"stop"` | chat/typing | Type change: bool → enum string                  |
| `read_receipt` type | `chat/ack` + `ack_type="read"` | -   | Merged into ack family                           |
| `call_offer/answer/ice/reject/end/busy` | `chat/signal` + `signal_type` | - | All call signals unified under one family |

---

## 7. GIGI BundleSchema Definitions (Rust)

No changes to the GIGI engine are required. These schemas use the existing `BundleSchema` / `FieldDef` API.

```rust
use gigi::types::{BundleSchema, FieldDef, Value};

pub fn chat_dm_schema() -> BundleSchema {
    BundleSchema::new("chat/dm")
        // Universal base fields
        .base(FieldDef::categorical("projection_type"))
        .base(FieldDef::categorical("sender_id"))
        .base(FieldDef::timestamp("timestamp_ns", 1e9))
        // Per-family base fields
        .base(FieldDef::categorical("message_id"))
        .base(FieldDef::categorical("recipient_id"))
        .base(FieldDef::categorical("conversation_id"))
        // Fiber fields
        .fiber(FieldDef::categorical("body"))
        .fiber(FieldDef::categorical("encrypted").with_default(Value::Bool(false)))
        .fiber(FieldDef::categorical("media_ref").with_default(Value::Null))
        .fiber(FieldDef::categorical("reply_to").with_default(Value::Null))
        .fiber(FieldDef::categorical("edited").with_default(Value::Bool(false)))
        .index("timestamp_ns")
        .index("conversation_id")
}

pub fn chat_signal_schema() -> BundleSchema {
    BundleSchema::new("chat/signal")
        .base(FieldDef::categorical("projection_type"))
        .base(FieldDef::categorical("sender_id"))
        .base(FieldDef::timestamp("timestamp_ns", 1e9))
        .base(FieldDef::categorical("recipient_id"))
        .base(FieldDef::categorical("call_id"))
        .fiber(FieldDef::categorical("signal_type"))
        .fiber(FieldDef::categorical("sdp").with_default(Value::Null))
        .fiber(FieldDef::categorical("ice_candidate").with_default(Value::Null))
        .fiber(FieldDef::categorical("media_type").with_default(Value::Null))
        .index("timestamp_ns")
        .index("call_id")
}

pub fn chat_ack_schema() -> BundleSchema {
    BundleSchema::new("chat/ack")
        .base(FieldDef::categorical("projection_type"))
        .base(FieldDef::categorical("sender_id"))
        .base(FieldDef::timestamp("timestamp_ns", 1e9))
        .base(FieldDef::categorical("recipient_id"))
        .fiber(FieldDef::categorical("target_id"))
        .fiber(FieldDef::categorical("ack_type"))
        .fiber(FieldDef::categorical("conversation_id").with_default(Value::Null))
        .index("timestamp_ns")
        .index("target_id")
}

pub fn chat_typing_schema() -> BundleSchema {
    BundleSchema::new("chat/typing")
        .base(FieldDef::categorical("projection_type"))
        .base(FieldDef::categorical("sender_id"))
        .base(FieldDef::timestamp("timestamp_ns", 1e9))
        .base(FieldDef::categorical("recipient_id"))
        .fiber(FieldDef::categorical("state"))
        .fiber(FieldDef::categorical("conversation_id").with_default(Value::Null))
    // No WAL index — typing events are not persisted
}

// reaction and voice_note follow the same pattern.
// See §3.3 for field definitions.
```

---

## 8. Open Items

| # | Item                                           | Owner |
|---|------------------------------------------------|-------|
| 1 | Confirm `recipient_id` encoding for group chats (single ID vs list) | GGOG |
| 2 | Confirm encryption scheme for `body` field (key exchange model)     | GGOG |
| 3 | Define `media_ref` resolution protocol (CID? URL? relay-local?)     | Joint |
| 4 | Confirm `ack` read-receipt retention window                         | GGOG |
| 5 | Agree interop test fixture format (DHOOM or JSON?) — see §8.5 proposal | Joint |
| 6 | Confirm `call_id` generation scheme (who mints it: caller or relay?)   | GGOG  |
| 7 | Define binary-first fallback negotiation error codes                   | Joint |
| 8 | Confirm max binary payload sizes (§2.1 provisionals — 64 KB / 256 KB) | Joint |
| 9 | Confirm `b64:` prefix escape policy for user-generated text in GGOG    | GGOG  |
| 10 | Define binary field list per family (which families MAY carry Binary fields) | Joint |

### §8.5 Interop Fixture Proposal

The first concrete end-to-end fixture should be a binary voice note ingest and replay. The fixture uses the JSON API edge path (`application/x-ndjson`) where `b64:` encoding is correct and expected. DHOOM is exercised in the re-export step, where the binary is raw bytes with no prefix.

**Step 1 — JSON ingest (b64: at the API edge, as designed)**

```
POST /v1/bundles/chat_voice_note/ingest
Content-Type: application/x-ndjson

{"projection_type":"chat/voice_note","sender_id":"alice","recipient_id":"bob","timestamp_ns":1710000000000000000,"message_id":"msg-vn-001","conversation_id":"conv-xyz","media_bytes":"b64:AAEC/w==","duration_ms":4200,"encrypted":true}
```

The string `"b64:AAEC/w=="` decodes to `Value::Binary([0x00, 0x01, 0x02, 0xFF])` at the JSON boundary per §2.1.

**Step 2 — DHOOM re-export (raw bytes, no b64: prefix)**

```
GET /v1/bundles/chat_voice_note/dhoom
```

The exported DHOOM fiber carries `media_bytes` as raw bytes (`00 01 02 FF`). There is no `b64:` in the DHOOM payload. A second client ingesting this DHOOM export must arrive at identical `Value::Binary([0x00, 0x01, 0x02, 0xFF])` without any base64 decode step.

Pass criteria:
1. Ingest (Step 1) returns `{"status": "ingested", "count": 1}` with `curvature > 0`
2. Point-query by `message_id = "msg-vn-001"` returns the record
3. `media_bytes` field in storage is `Value::Binary([0x00, 0x01, 0x02, 0xFF])` — exact bytes, no prefix
4. DHOOM re-export (Step 2) completes without error
5. A second client ingesting the DHOOM export produces identical bytes for `media_bytes` — confirming raw-byte fidelity end-to-end

This is a joint deliverable. GIGI provides the endpoint; GGOG provides the client-side decode verification for criteria 3 and 5.

---

## 9. Next Steps

1. GGOG sends current message family schema map → GIGI reviews against the migration table in §6
2. ~~GIGI cuts v1 release of `application/dhoom` content-type path on `/v1/bundles/{name}/ingest`~~ ✅ Done — `POST /v1/bundles/{name}/ingest` live as of commit `67aa7ef`
3. ~~`Value::Binary` storage gap closed~~ ✅ Done — `e486b55`
4. GGOG and GIGI jointly run interop fixture §8.5 — binary voice note ingest, replay, decode
5. GGOG answers §8 open items 1, 2, 6, 8, 9 → GIGI finalises BundleSchema for all six families
6. Joint: CI fixture coverage for all six event families (§3.3), including at least one Binary field per relevant family
7. Lock pass/fail invariants before any client ships against this contract
