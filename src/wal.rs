//! Write-Ahead Log (WAL) for crash-safe persistence.
//!
//! Implements the durability guarantee: every mutation is logged
//! before being applied to the in-memory store.
//!
//! WAL format (per entry):
//!   [4 bytes: length] [1 byte: op_type] [payload] [4 bytes: CRC32]
//!
//! Op types:
//!   0x01 = INSERT
//!   0x02 = CREATE_BUNDLE (schema)
//!   0xFF = CHECKPOINT — two distinct jobs share this marker. The
//!   compaction path writes one after a successful snapshot (there
//!   it really does mean "all prior entries are in the data file");
//!   the auto-checkpoint cadence (`Engine::maybe_checkpoint`, every
//!   `checkpoint_interval` ops) writes one with an fsync and NO
//!   snapshot. Recovery therefore anchors on the FIRST checkpoint —
//!   the one whose snapshot was actually loaded — not the last.

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::types::{BundleSchema, FieldDef, FieldType, Record, Value};

const OP_INSERT: u8 = 0x01;
const OP_CREATE_BUNDLE: u8 = 0x02;
const OP_UPDATE: u8 = 0x03;
const OP_DELETE: u8 = 0x04;
const OP_DROP_BUNDLE: u8 = 0x05;
const OP_MEASUREMENT_OVERRIDE: u8 = 0x06;
const OP_CREATE_TRIGGER: u8 = 0x07;
const OP_DROP_TRIGGER: u8 = 0x08;
// TDD-HAL-II.4b: lattice + gauge-field durability ops. Gated on the
// `gauge` feature; the no-default-features build never emits or reads
// these bytes (the writer methods don't exist; the reader's match arm
// returns `Unknown WAL op` if a stray gauge-built WAL is opened by a
// no-default-features binary, which is the right loud-failure behavior).
#[cfg(feature = "gauge")]
const OP_LATTICE_DECLARE: u8 = 0x09;
#[cfg(feature = "gauge")]
const OP_GAUGE_FIELD_DECLARE: u8 = 0x0A;
// TDD-HAL-V.1: durable post-thermalization buffer snapshot for an
// already-declared GAUGE_FIELD. Spec — HALCYON_PART_V_SNAPSHOT_GATES.md
// §3 (P0) + Bee's locked decision D-V-A (explicit little-endian).
//
// Payload (all multi-byte fields little-endian, per D-V-A):
//   [u32 name_len][name_bytes]
//   [u8  group_tag]   SU2=0x01, SU3=0x02, U1=0x03, ZN=0x04(+u32 n)
//   [32 sha256_bytes] SHA-256 over the LE-encoded buffer bytes only;
//                     the same hash is the citation handle returned to
//                     the caller and the WAL-write receipt (D-V-C).
//   [u32 buf_len]     # of f64 entries (NOT byte count)
//   [buf_len * 8 buffer_bytes]   each f64 written via f64::to_le_bytes
//
// For SU(2) on the buckyball (90 edges × 4 floats/edge) the buffer
// portion alone is exactly 90*4*8 = 2880 bytes (matches spec §0). The
// total entry adds the 4-byte name-length prefix, name UTF-8, 1 group
// tag, 32 SHA bytes, and the 4-byte buffer length — about 41 bytes of
// framing for a single-character field name like "U".
//
// Replay (P1, follow-up gate) re-derives SHA-256 from the LE buffer
// bytes and compares against the payload's `sha256` field — that's the
// canonical citation handle and the integrity check both. The same
// hash lands in the WAL entry AND the Rows envelope returned to the
// caller (per locked decision D-V-C).
#[cfg(feature = "gauge")]
const OP_GAUGE_FIELD_SNAPSHOT: u8 = 0x0B;
// AURORA Phase 2: durable HAMILTONIAN_DECLARE record. Metadata-only
// (name, kind_tag, group_tag, registered_at). Emitted by
// `gauge::hamiltonian_registry::register()` when a `WalWriter` is
// supplied. Replay handling is OUT OF SCOPE for the Phase 2 workflow —
// the entry is persisted as audit / introspection, and host binaries
// must explicitly re-register at startup per the AURORA Q5 contract.
#[cfg(feature = "gauge")]
const OP_HAMILTONIAN_DECLARE: u8 = 0x0C;
// AURORA Phase 3: audit trail for SYMPLECTIC_FLOW integrator dispatch.
// Emitted ONCE per flow invocation (not per step) — the dispatch is
// invariant across steps within one invocation. Payload is a flat
// (path, factory_name, handle_name) triple; replay tools attribute
// energy/Casimir drift to the integrator family that produced it.
// Forward-stable: `path` is a `String` discriminant (e.g.
// `"bracket_step"`, `"stormer_verlet_kdk"`) so new integrator
// families append without breaking existing replay.
#[cfg(feature = "gauge")]
const OP_INTEGRATOR_CHOICE: u8 = 0x0D;
// IMAGINE coherence Phase 2: durable audit when the tame-metric
// fallback engages on a high-K bundle. Emitted by the
// `bundle_imagine_coherence` HTTP handler when
// `bundle.curvature_stats().mean()` exceeds the Phase 2 threshold and
// the caller did NOT pass an explicit `metric_curvature` override.
//
// Payload (all multi-byte fields little-endian):
//   [u32 bundle_name_len][bundle_name UTF-8 bytes]
//   [f64 original_k]
//   [f64 substituted_k]
//   [u64 timestamp_ms]
//
// Forward-compat: replay reads back into
// `WalEntry::ImagineFallback { bundle, original_k, substituted_k,
// timestamp_ms }`. Older binaries that don't recognize 0x0E hit the
// trailing `Unknown WAL op` match arm — but the loud failure is the
// right behavior for an unknown opcode (matches the gauge ops policy).
#[cfg(feature = "imagine")]
const OP_IMAGINE_FALLBACK: u8 = 0x0E;
const OP_CHECKPOINT: u8 = 0xFF;

/// TDD-HAL-V.1: payload of a `OP_GAUGE_FIELD_SNAPSHOT` (0x0B) WAL
/// entry. Captures the post-thermalization link buffer of an already-
/// declared `GAUGE_FIELD` so the field survives a `gigi-stream`
/// restart (HALCYON_PART_V_SNAPSHOT_GATES.md §3, P0).
///
/// Encoding is explicit little-endian (Bee's locked decision D-V-A)
/// so the WAL is portable across architectures (x86_64 / aarch64).
/// `sha256` is computed over the LE-encoded `buffer` bytes — that is
/// the citation handle the Solves Vol. 4 chapter cites (D-V-C).
#[cfg(feature = "gauge")]
#[derive(Debug, Clone, PartialEq)]
pub struct GaugeFieldSnapshotPayload {
    /// The declared field's name. Must match a previously-declared
    /// `GAUGE_FIELD` at replay time (orphan-snapshot rejection lands
    /// in V.2; this struct is the wire format only).
    pub name: String,
    /// The group tag carried for validation. Replay (P1) errors loudly
    /// if it disagrees with the declared field's group — catches the
    /// snapshot-against-wrong-field bug.
    pub group: crate::gauge::Group,
    /// Row-major `(n_edges, repr_dim)` f64 buffer. For SU(2) on the
    /// buckyball this is 90 × 4 = 360 entries → 2880 bytes after
    /// LE encoding (spec §0).
    pub buffer: Vec<f64>,
    /// SHA-256 of the LE-encoded buffer bytes. Computed at encode
    /// time, re-verified at decode time. Same hash returned to the
    /// caller as the citation handle (D-V-C).
    pub sha256: [u8; 32],
}

#[cfg(feature = "gauge")]
impl GaugeFieldSnapshotPayload {
    /// Construct from a buffer; computes the SHA-256 over LE bytes.
    pub fn from_buffer(name: String, group: crate::gauge::Group, buffer: Vec<f64>) -> Self {
        let sha256 = Self::compute_buffer_sha256(&buffer);
        Self {
            name,
            group,
            buffer,
            sha256,
        }
    }

    /// SHA-256 over the canonical LE encoding of the buffer (each f64
    /// written via `f64::to_le_bytes`). This is the citation handle.
    pub fn compute_buffer_sha256(buffer: &[f64]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        for v in buffer {
            hasher.update(v.to_le_bytes());
        }
        hasher.finalize().into()
    }

    /// Serialize to little-endian bytes per locked decision D-V-A.
    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            4 + self.name.len() + 1 + 4 /* possible zn n */ + 32 + 4 + self.buffer.len() * 8,
        );
        // name (length-prefixed)
        out.extend_from_slice(&(self.name.len() as u32).to_le_bytes());
        out.extend_from_slice(self.name.as_bytes());
        // group tag (matches OP_GAUGE_FIELD_DECLARE encoding for symmetry)
        match self.group {
            crate::gauge::Group::SU2 => out.push(0x01),
            crate::gauge::Group::SU3 => out.push(0x02),
            crate::gauge::Group::U1 => out.push(0x03),
            crate::gauge::Group::ZN { n } => {
                out.push(0x04);
                out.extend_from_slice(&n.to_le_bytes());
            }
        }
        // sha256 (raw 32 bytes; not length-prefixed — fixed width)
        out.extend_from_slice(&self.sha256);
        // buffer (length-prefixed entry count, then LE f64 bytes)
        out.extend_from_slice(&(self.buffer.len() as u32).to_le_bytes());
        for v in &self.buffer {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    /// Parse from little-endian bytes (mirror of `to_le_bytes`).
    /// Does not verify the SHA-256 against the buffer — that's the
    /// replay path's job (V.2). This decoder accepts the bytes as
    /// written and surfaces any structural error as `InvalidData`.
    pub fn from_le_bytes(data: &[u8]) -> io::Result<Self> {
        let mut offset = 0usize;
        let name = read_string(data, &mut offset)?;
        if offset >= data.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "group tag"));
        }
        let group_tag = data[offset];
        offset += 1;
        let group = match group_tag {
            0x01 => crate::gauge::Group::SU2,
            0x02 => crate::gauge::Group::SU3,
            0x03 => crate::gauge::Group::U1,
            0x04 => {
                if offset + 4 > data.len() {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "ZN modulus"));
                }
                let n = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
                offset += 4;
                crate::gauge::Group::ZN { n }
            }
            bad => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown gauge group tag: {bad:#x}"),
                ));
            }
        };
        if offset + 32 > data.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "sha256"));
        }
        let mut sha256 = [0u8; 32];
        sha256.copy_from_slice(&data[offset..offset + 32]);
        offset += 32;
        if offset + 4 > data.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "buffer len"));
        }
        let buf_len =
            u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + buf_len * 8 > data.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "buffer bytes"));
        }
        let mut buffer = Vec::with_capacity(buf_len);
        for _ in 0..buf_len {
            let v = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            buffer.push(v);
        }
        let _ = offset; // silence trailing-offset lint
        Ok(Self {
            name,
            group,
            buffer,
            sha256,
        })
    }
}

/// TDD-HAL-V.3: typed errors for the WAL replay path. Lifted out of the
/// `io::Error::new(InvalidData, …)` string soup so the snapshot-restore
/// gate has a structurally-checkable surface: integration tests match on
/// the variant, not on a substring of the Display impl.
///
/// Three variants land here for V.3:
///
/// - `OrphanedSnapshot(name)` — a `OP_GAUGE_FIELD_SNAPSHOT` entry for
///   `name` was seen during replay but no preceding
///   `OP_GAUGE_FIELD_DECLARE` for the same name reached the registry.
///   The replay aborts loudly rather than installing a buffer on top of
///   an undeclared field (where would the lattice come from?).
/// - `SnapshotGroupMismatch { name, expected, found }` — the snapshot
///   payload's group tag disagrees with the declared field's group. This
///   catches "snapshot against wrong field" corruption — e.g. a buffer
///   captured against a U(1) field somehow landing on an SU(2) field.
/// - `SnapshotChecksumMismatch { name }` — the SHA-256 the replay re-
///   derives from the LE-encoded buffer bytes disagrees with the
///   payload's stored `sha256`. The same hash is the citation handle
///   (Bee's locked decision D-V-C), so a mismatch means either the
///   buffer bytes or the sha256 field corrupted in flight; replay
///   refuses to install a buffer the citation no longer addresses.
///
/// The `From<WalError> for io::Error` impl below preserves the existing
/// `replay_gauge_substrate -> io::Result<()>` surface so the engine's
/// open path doesn't need a typed-error refactor for V.3.
#[cfg(feature = "gauge")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalError {
    /// `OP_GAUGE_FIELD_SNAPSHOT` for an unknown field — no preceding
    /// `OP_GAUGE_FIELD_DECLARE` for the same name reached the registry.
    OrphanedSnapshot(String),
    /// Snapshot payload's group tag disagrees with the declared field's
    /// group. Catches snapshot-against-wrong-field corruption.
    SnapshotGroupMismatch {
        /// Name of the field at replay time.
        name: String,
        /// Group of the declared field (from the registry handle).
        expected: crate::gauge::Group,
        /// Group tag carried in the snapshot payload (from the WAL).
        found: crate::gauge::Group,
    },
    /// Re-derived SHA-256 disagrees with the payload's stored hash.
    SnapshotChecksumMismatch {
        /// Name of the field whose snapshot failed checksum verification.
        name: String,
    },
}

#[cfg(feature = "gauge")]
impl std::fmt::Display for WalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalError::OrphanedSnapshot(name) => write!(
                f,
                "WAL OP_GAUGE_FIELD_SNAPSHOT for field '{name}' has no preceding \
                 OP_GAUGE_FIELD_DECLARE — orphan snapshot, replay refuses to install"
            ),
            WalError::SnapshotGroupMismatch {
                name,
                expected,
                found,
            } => write!(
                f,
                "WAL OP_GAUGE_FIELD_SNAPSHOT for field '{name}' carries group {} \
                 but the declared field is {} — snapshot-against-wrong-field",
                found.label(),
                expected.label(),
            ),
            WalError::SnapshotChecksumMismatch { name } => write!(
                f,
                "WAL OP_GAUGE_FIELD_SNAPSHOT for field '{name}' fails SHA-256 \
                 re-derivation against payload.sha256 — buffer or hash \
                 corrupted in flight"
            ),
        }
    }
}

#[cfg(feature = "gauge")]
impl std::error::Error for WalError {}

#[cfg(feature = "gauge")]
impl From<WalError> for io::Error {
    fn from(e: WalError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, e.to_string())
    }
}

/// CRC32 (Castagnoli) — simple polynomial checksum for integrity.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x82F6_3B78;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// WAL writer — appends entries to the log file.
pub struct WalWriter {
    writer: BufWriter<File>,
    entry_count: u64,
}

impl WalWriter {
    /// Open (or create) a WAL file at the given path.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let entry_count = 0;
        Ok(Self {
            writer: BufWriter::new(file),
            entry_count,
        })
    }

    /// Log a CREATE_BUNDLE operation.
    pub fn log_create_bundle(&mut self, schema: &BundleSchema) -> io::Result<()> {
        let payload = encode_schema(schema);
        self.write_entry(OP_CREATE_BUNDLE, &payload)
    }

    /// Log an INSERT operation.
    pub fn log_insert(&mut self, bundle_name: &str, record: &Record) -> io::Result<()> {
        let payload = encode_insert(bundle_name, record);
        self.write_entry(OP_INSERT, &payload)
    }

    /// Log an UPDATE operation (partial field update).
    pub fn log_update(
        &mut self,
        bundle_name: &str,
        key: &Record,
        patches: &Record,
    ) -> io::Result<()> {
        let payload = encode_update(bundle_name, key, patches);
        self.write_entry(OP_UPDATE, &payload)
    }

    /// Log a DELETE operation.
    pub fn log_delete(&mut self, bundle_name: &str, key: &Record) -> io::Result<()> {
        let payload = encode_insert(bundle_name, key); // reuse insert encoding for key
        self.write_entry(OP_DELETE, &payload)
    }

    /// Log a DROP_BUNDLE operation.
    pub fn log_drop_bundle(&mut self, bundle_name: &str) -> io::Result<()> {
        let mut payload = Vec::new();
        write_string(&mut payload, bundle_name);
        self.write_entry(OP_DROP_BUNDLE, &payload)
    }

    /// Log a MeasurementOverride — measured data supersedes a completion.
    pub fn log_measurement_override(
        &mut self,
        bundle_name: &str,
        field: &str,
        key: &Record,
        old_completed_value: f64,
        old_confidence: f64,
        new_measured_value: f64,
        timestamp: u64,
    ) -> io::Result<()> {
        let mut payload = Vec::new();
        write_string(&mut payload, bundle_name);
        write_string(&mut payload, field);
        encode_record_into(&mut payload, key);
        payload.extend_from_slice(&old_completed_value.to_le_bytes());
        payload.extend_from_slice(&old_confidence.to_le_bytes());
        payload.extend_from_slice(&new_measured_value.to_le_bytes());
        payload.extend_from_slice(&timestamp.to_le_bytes());
        self.write_entry(OP_MEASUREMENT_OVERRIDE, &payload)
    }

    /// Log a checkpoint marker.
    pub fn log_checkpoint(&mut self) -> io::Result<()> {
        self.write_entry(OP_CHECKPOINT, &[])
    }

    /// Log a CREATE TRIGGER operation (Feature #9).
    pub fn log_create_trigger(
        &mut self,
        name: &str,
        bundle: &str,
        channel: &str,
        operation: &str,
        filter_str: Option<&str>,
    ) -> io::Result<()> {
        let mut payload = Vec::new();
        write_string(&mut payload, name);
        write_string(&mut payload, bundle);
        write_string(&mut payload, channel);
        write_string(&mut payload, operation);
        // Optional filter
        if let Some(f) = filter_str {
            payload.push(1);
            write_string(&mut payload, f);
        } else {
            payload.push(0);
        }
        self.write_entry(OP_CREATE_TRIGGER, &payload)
    }

    /// Log a DROP TRIGGER operation (Feature #9).
    pub fn log_drop_trigger(&mut self, name: &str) -> io::Result<()> {
        let mut payload = Vec::new();
        write_string(&mut payload, name);
        self.write_entry(OP_DROP_TRIGGER, &payload)
    }

    /// TDD-HAL-II.4b: log a LATTICE durable declaration. Payload is
    /// the canonical GQL re-emit form of the Lattice (the same string
    /// `Lattice::to_gql` produces). Replay uses `Lattice::from_gql` to
    /// reconstruct, so the WAL round-trip is bit-identical to the
    /// declaration the user wrote.
    #[cfg(feature = "gauge")]
    pub fn log_lattice_declare(&mut self, gql: &str) -> io::Result<()> {
        let mut payload = Vec::new();
        write_string(&mut payload, gql);
        self.write_entry(OP_LATTICE_DECLARE, &payload)
    }

    /// TDD-HAL-II.4b: log a GAUGE_FIELD durable declaration. Variant
    /// is metadata-only per Bee's locked decision 1: re-running the
    /// init recipe at replay produces a byte-identical buffer. Mutated
    /// buffers (post-HEATBATH_SWEEP) are a P1 follow-up via a
    /// separate `GAUGE_FIELD_CHECKPOINT` op; II.4b targets declaration
    /// durability only.
    ///
    /// Payload format:
    ///   - name (string)
    ///   - lattice_name (string)
    ///   - group_tag (u8) — 0x01 = SU2, 0x02 = SU3, 0x03 = U1, 0x04 = ZN
    ///   - if group_tag == ZN: modulus n (u32 LE)
    ///   - init_tag (u8) — 0x01 = Identity, 0x02 = HaarRandom, 0x03 = FromField
    ///   - if init_tag == HaarRandom: seed_present (u8) + optional u64 LE
    ///   - if init_tag == FromField: source name (string)
    ///   - if init_tag == Identity: nothing (seed_present = 0 always)
    #[cfg(feature = "gauge")]
    pub fn log_gauge_field_declare(
        &mut self,
        name: &str,
        lattice_name: &str,
        group: crate::gauge::Group,
        init_kind: &crate::gauge::GaugeFieldInit,
        init_seed: Option<u64>,
    ) -> io::Result<()> {
        let mut payload = Vec::new();
        write_string(&mut payload, name);
        write_string(&mut payload, lattice_name);
        match group {
            crate::gauge::Group::SU2 => payload.push(0x01),
            crate::gauge::Group::SU3 => payload.push(0x02),
            crate::gauge::Group::U1 => payload.push(0x03),
            crate::gauge::Group::ZN { n } => {
                payload.push(0x04);
                payload.extend_from_slice(&n.to_le_bytes());
            }
        }
        match init_kind {
            crate::gauge::GaugeFieldInit::Identity => {
                payload.push(0x01);
            }
            crate::gauge::GaugeFieldInit::HaarRandom => {
                payload.push(0x02);
                match init_seed {
                    Some(s) => {
                        payload.push(1);
                        payload.extend_from_slice(&s.to_le_bytes());
                    }
                    None => payload.push(0),
                }
            }
            crate::gauge::GaugeFieldInit::FromField(src) => {
                payload.push(0x03);
                write_string(&mut payload, src);
            }
            // INIT FLUX (2026-07-16): U(1)-only bundle materialization —
            // PERSIST is rejected at declaration, so a flux init can
            // never legitimately reach the WAL declaration encoder. No
            // byte tag is allocated (the WAL format is unchanged);
            // surface a typed refusal instead of writing a corrupt entry.
            crate::gauge::GaugeFieldInit::FluxRandom
            | crate::gauge::GaugeFieldInit::FluxUniform => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "INIT FLUX gauge fields are not WAL-persistable (the \
                     materialized theta bundle is the durable artifact)",
                ));
            }
        }
        self.write_entry(OP_GAUGE_FIELD_DECLARE, &payload)
    }

    /// TDD-HAL-V.1: log a post-thermalization GAUGE_FIELD buffer
    /// snapshot. Payload is the explicit little-endian encoding of
    /// `GaugeFieldSnapshotPayload` per locked decision D-V-A; the
    /// payload's `sha256` field is computed over the LE buffer bytes
    /// at construction time and is the citation handle the replay
    /// path re-verifies against (D-V-C).
    #[cfg(feature = "gauge")]
    pub fn log_gauge_field_snapshot(
        &mut self,
        payload: &GaugeFieldSnapshotPayload,
    ) -> io::Result<()> {
        let bytes = payload.to_le_bytes();
        self.write_entry(OP_GAUGE_FIELD_SNAPSHOT, &bytes)
    }

    /// AURORA Phase 2: log a HAMILTONIAN_DECLARE record. Metadata-only
    /// — `(name, kind_tag, group_tag, registered_at)`. Replay handling
    /// is deferred to a follow-up workflow; this is currently audit /
    /// introspection only.
    ///
    /// Payload format (all multi-byte fields little-endian):
    ///   [u32 name_len][name_bytes]
    ///   [u32 kind_tag_len][kind_tag_bytes]
    ///   [u32 group_tag_len][group_tag_bytes]
    ///   [u64 registered_at]
    #[cfg(feature = "gauge")]
    pub fn log_hamiltonian_declare(
        &mut self,
        name: &str,
        kind_tag: &str,
        group_tag: &str,
        registered_at: u64,
    ) -> io::Result<()> {
        let mut payload = Vec::new();
        write_string(&mut payload, name);
        write_string(&mut payload, kind_tag);
        write_string(&mut payload, group_tag);
        payload.extend_from_slice(&registered_at.to_le_bytes());
        self.write_entry(OP_HAMILTONIAN_DECLARE, &payload)
    }

    /// AURORA Phase 3: log an `INTEGRATOR_CHOICE` record. Emitted once
    /// per `SYMPLECTIC_FLOW` invocation, after capability dispatch
    /// resolves and before the integrator loop opens. Diagnostic only —
    /// replay does not re-execute (per the same forward-compat policy
    /// as `HamiltonianDeclare`).
    ///
    /// Payload format (all multi-byte fields little-endian):
    ///   [u32 path_len][path_bytes]
    ///   [u32 factory_name_len][factory_name_bytes]
    ///   [u32 handle_name_len][handle_name_bytes]
    #[cfg(feature = "gauge")]
    pub fn log_integrator_choice(
        &mut self,
        path: &str,
        factory_name: &str,
        handle_name: &str,
    ) -> io::Result<()> {
        let mut payload = Vec::new();
        write_string(&mut payload, path);
        write_string(&mut payload, factory_name);
        write_string(&mut payload, handle_name);
        self.write_entry(OP_INTEGRATOR_CHOICE, &payload)
    }

    /// IMAGINE coherence Phase 2: log an `IMAGINE_FALLBACK` record.
    /// Emitted by the `bundle_imagine_coherence` HTTP handler when the
    /// tame-metric fallback engages (bundle K mean exceeds the
    /// configured threshold and the caller did not pass an explicit
    /// `metric_curvature` override). Diagnostic only — replay does not
    /// re-execute the fallback decision; the audit is for downstream
    /// consumers (Marcella's confidence routing, operator dashboards)
    /// to know which trajectories were integrated on a substituted
    /// geometry instead of the literal substrate metric.
    ///
    /// Payload format (all multi-byte fields little-endian):
    ///   [u32 bundle_name_len][bundle_name_bytes]
    ///   [f64 original_k]
    ///   [f64 substituted_k]
    ///   [u64 timestamp_ms]
    #[cfg(feature = "imagine")]
    pub fn log_imagine_fallback(
        &mut self,
        bundle: &str,
        original_k: f64,
        substituted_k: f64,
        timestamp_ms: u64,
    ) -> io::Result<()> {
        let mut payload = Vec::new();
        write_string(&mut payload, bundle);
        payload.extend_from_slice(&original_k.to_le_bytes());
        payload.extend_from_slice(&substituted_k.to_le_bytes());
        payload.extend_from_slice(&timestamp_ms.to_le_bytes());
        self.write_entry(OP_IMAGINE_FALLBACK, &payload)
    }

    /// Sync the WAL to disk (fsync).
    pub fn sync(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        self.writer.get_ref().sync_all()
    }

    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }

    fn write_entry(&mut self, op: u8, payload: &[u8]) -> io::Result<()> {
        let total_len = 1 + payload.len(); // op + payload
        let len_bytes = (total_len as u32).to_le_bytes();

        // Build entry for CRC: op + payload
        let mut entry = Vec::with_capacity(total_len);
        entry.push(op);
        entry.extend_from_slice(payload);
        let checksum = crc32(&entry);

        self.writer.write_all(&len_bytes)?;
        self.writer.write_all(&entry)?;
        self.writer.write_all(&checksum.to_le_bytes())?;
        self.entry_count += 1;
        Ok(())
    }
}

/// WAL reader — replays entries from a log file.
pub struct WalReader {
    file: File,
}

/// A single WAL entry.
#[derive(Debug, Clone)]
pub enum WalEntry {
    CreateBundle(BundleSchema),
    Insert {
        bundle_name: String,
        record: Record,
    },
    Update {
        bundle_name: String,
        key: Record,
        patches: Record,
    },
    Delete {
        bundle_name: String,
        key: Record,
    },
    DropBundle(String),
    /// A measured value overrides a previously completed (predicted) value.
    MeasurementOverride {
        bundle_name: String,
        field: String,
        key: Record,
        old_completed_value: f64,
        old_confidence: f64,
        new_measured_value: f64,
        timestamp: u64,
    },
    Checkpoint,
    /// Feature #9: Trigger definition persisted to WAL.
    CreateTrigger {
        name: String,
        bundle: String,
        channel: String,
        operation: String,
        filter_str: Option<String>,
    },
    /// Feature #9: Drop a trigger by name.
    DropTrigger(String),
    /// TDD-HAL-II.4b: durable LATTICE declaration. Payload is the
    /// canonical GQL re-emit form; replay parses it via
    /// `Lattice::from_gql` and installs the result in
    /// `lattice::registry`.
    #[cfg(feature = "gauge")]
    LatticeDeclare {
        /// Canonical GQL re-emit (whitespace-stable, see
        /// `Lattice::to_gql`).
        gql: String,
    },
    /// TDD-HAL-II.4b: durable GAUGE_FIELD declaration. Metadata-only
    /// (Bee's locked decision 1) — buffer is re-materialized via
    /// `SU2GaugeField::new(name, lattice, init_kind, init_seed)` at
    /// replay time, which produces a byte-identical buffer for the
    /// SU(2)+Haar+seed path and an identity buffer for the Identity
    /// path.
    #[cfg(feature = "gauge")]
    GaugeFieldDeclare {
        name: String,
        lattice_name: String,
        group: crate::gauge::Group,
        init_kind: crate::gauge::GaugeFieldInit,
        init_seed: Option<u64>,
    },
    /// TDD-HAL-V.1: durable post-thermalization buffer snapshot for an
    /// already-declared `GAUGE_FIELD`. Payload is the LE-encoded
    /// `GaugeFieldSnapshotPayload`; replay (P1, follow-up gate) installs
    /// `payload.buffer` into the live handle after re-deriving SHA-256
    /// from the LE bytes and comparing against `payload.sha256`.
    #[cfg(feature = "gauge")]
    GaugeFieldSnapshot(GaugeFieldSnapshotPayload),
    /// AURORA Phase 2: durable HAMILTONIAN_DECLARE record. Metadata-
    /// only — declares that a `HamiltonianFactory` named `name`
    /// (with `kind_tag` / `group_tag`) was registered at
    /// `registered_at`. Replay handling is deferred to a follow-up
    /// workflow; the host binary must explicitly re-register at
    /// startup per the AURORA Q5 contract.
    #[cfg(feature = "gauge")]
    HamiltonianDeclare {
        name: String,
        kind_tag: String,
        group_tag: String,
        registered_at: u64,
    },
    /// AURORA Phase 3: which integrator path `SYMPLECTIC_FLOW`
    /// selected for one invocation. Emitted once at flow entry,
    /// after capability dispatch and before the integrator loop.
    /// `path` is a discriminant string (`"bracket_step"` |
    /// `"stormer_verlet_kdk"`); new families append without breaking
    /// existing replay (forward-compat skip-unknown applies on
    /// older binaries).
    #[cfg(feature = "gauge")]
    IntegratorChoice {
        path: String,
        factory_name: String,
        handle_name: String,
    },
    /// IMAGINE coherence Phase 2: durable audit when the tame-metric
    /// fallback engages. Records the bundle name, the original K mean
    /// that triggered the fallback, the substituted (tame) K actually
    /// fed to the integrator, and a unix millisecond timestamp.
    /// Diagnostic only — replay does not re-execute the decision.
    #[cfg(feature = "imagine")]
    ImagineFallback {
        bundle: String,
        original_k: f64,
        substituted_k: f64,
        timestamp_ms: u64,
    },
}

impl WalReader {
    /// Open an existing WAL file for reading.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self { file })
    }

    /// Read all valid entries from the WAL.
    pub fn read_all(&mut self) -> io::Result<Vec<WalEntry>> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut entries = Vec::new();
        loop {
            match self.read_one() {
                Ok(Some(entry)) => entries.push(entry),
                Ok(None) => break,
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
        }
        Ok(entries)
    }

    /// Streaming replay — calls `f` for each entry without buffering the entire WAL.
    /// Stops at EOF or an UnexpectedEof error (truncated final entry is silently ignored).
    /// Returns Err on CRC failures or other I/O errors.
    pub fn replay<F>(&mut self, mut f: F) -> io::Result<()>
    where
        F: FnMut(WalEntry) -> io::Result<()>,
    {
        self.file.seek(SeekFrom::Start(0))?;
        loop {
            match self.read_one() {
                Ok(Some(entry)) => f(entry)?,
                Ok(None) => break,
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn read_one(&mut self) -> io::Result<Option<WalEntry>> {
        // Read length
        let mut len_buf = [0u8; 4];
        match self.file.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
        let total_len = u32::from_le_bytes(len_buf) as usize;

        // Read entry (op + payload)
        let mut entry = vec![0u8; total_len];
        self.file.read_exact(&mut entry)?;

        // Read CRC
        let mut crc_buf = [0u8; 4];
        self.file.read_exact(&mut crc_buf)?;
        let stored_crc = u32::from_le_bytes(crc_buf);
        let computed_crc = crc32(&entry);

        if stored_crc != computed_crc {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("WAL CRC mismatch: stored={stored_crc:#x}, computed={computed_crc:#x}"),
            ));
        }

        let op = entry[0];
        let payload = &entry[1..];

        match op {
            OP_CREATE_BUNDLE => {
                let schema = decode_schema(payload)?;
                Ok(Some(WalEntry::CreateBundle(schema)))
            }
            OP_INSERT => {
                let (bundle_name, record) = decode_insert(payload)?;
                Ok(Some(WalEntry::Insert {
                    bundle_name,
                    record,
                }))
            }
            OP_UPDATE => {
                let (bundle_name, key, patches) = decode_update(payload)?;
                Ok(Some(WalEntry::Update {
                    bundle_name,
                    key,
                    patches,
                }))
            }
            OP_DELETE => {
                let (bundle_name, key) = decode_insert(payload)?;
                Ok(Some(WalEntry::Delete { bundle_name, key }))
            }
            OP_DROP_BUNDLE => {
                let mut offset = 0;
                let bundle_name = read_string(payload, &mut offset)?;
                Ok(Some(WalEntry::DropBundle(bundle_name)))
            }
            OP_MEASUREMENT_OVERRIDE => {
                let mut offset = 0;
                let bundle_name = read_string(payload, &mut offset)?;
                let field = read_string(payload, &mut offset)?;
                let key = decode_record(payload, &mut offset)?;
                let old_completed_value =
                    f64::from_le_bytes(payload[offset..offset + 8].try_into().unwrap());
                offset += 8;
                let old_confidence =
                    f64::from_le_bytes(payload[offset..offset + 8].try_into().unwrap());
                offset += 8;
                let new_measured_value =
                    f64::from_le_bytes(payload[offset..offset + 8].try_into().unwrap());
                offset += 8;
                let timestamp = u64::from_le_bytes(payload[offset..offset + 8].try_into().unwrap());
                let _ = timestamp; // consume
                Ok(Some(WalEntry::MeasurementOverride {
                    bundle_name,
                    field,
                    key,
                    old_completed_value,
                    old_confidence,
                    new_measured_value,
                    timestamp,
                }))
            }
            OP_CHECKPOINT => Ok(Some(WalEntry::Checkpoint)),
            OP_CREATE_TRIGGER => {
                let mut offset = 0;
                let name = read_string(payload, &mut offset)?;
                let bundle = read_string(payload, &mut offset)?;
                let channel = read_string(payload, &mut offset)?;
                let operation = read_string(payload, &mut offset)?;
                let has_filter = if offset < payload.len() { payload[offset] } else { 0 };
                offset += 1;
                let filter_str = if has_filter == 1 {
                    Some(read_string(payload, &mut offset)?)
                } else {
                    None
                };
                Ok(Some(WalEntry::CreateTrigger {
                    name,
                    bundle,
                    channel,
                    operation,
                    filter_str,
                }))
            }
            OP_DROP_TRIGGER => {
                let mut offset = 0;
                let name = read_string(payload, &mut offset)?;
                Ok(Some(WalEntry::DropTrigger(name)))
            }
            #[cfg(feature = "gauge")]
            OP_LATTICE_DECLARE => {
                let mut offset = 0;
                let gql = read_string(payload, &mut offset)?;
                Ok(Some(WalEntry::LatticeDeclare { gql }))
            }
            #[cfg(feature = "gauge")]
            OP_GAUGE_FIELD_DECLARE => {
                let mut offset = 0;
                let name = read_string(payload, &mut offset)?;
                let lattice_name = read_string(payload, &mut offset)?;
                let group_tag = payload[offset];
                offset += 1;
                let group = match group_tag {
                    0x01 => crate::gauge::Group::SU2,
                    0x02 => crate::gauge::Group::SU3,
                    0x03 => crate::gauge::Group::U1,
                    0x04 => {
                        let n = u32::from_le_bytes(
                            payload[offset..offset + 4].try_into().unwrap(),
                        );
                        offset += 4;
                        crate::gauge::Group::ZN { n }
                    }
                    bad => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("unknown gauge group tag: {bad:#x}"),
                        ))
                    }
                };
                let init_tag = payload[offset];
                offset += 1;
                let (init_kind, init_seed) = match init_tag {
                    0x01 => (crate::gauge::GaugeFieldInit::Identity, None),
                    0x02 => {
                        let seed_present = payload[offset];
                        offset += 1;
                        let seed = if seed_present == 1 {
                            let s = u64::from_le_bytes(
                                payload[offset..offset + 8].try_into().unwrap(),
                            );
                            offset += 8;
                            Some(s)
                        } else {
                            None
                        };
                        let _ = offset; // silence trailing-offset lint
                        (crate::gauge::GaugeFieldInit::HaarRandom, seed)
                    }
                    0x03 => {
                        let src = read_string(payload, &mut offset)?;
                        (crate::gauge::GaugeFieldInit::FromField(src), None)
                    }
                    bad => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("unknown gauge init tag: {bad:#x}"),
                        ))
                    }
                };
                Ok(Some(WalEntry::GaugeFieldDeclare {
                    name,
                    lattice_name,
                    group,
                    init_kind,
                    init_seed,
                }))
            }
            #[cfg(feature = "gauge")]
            OP_GAUGE_FIELD_SNAPSHOT => {
                let payload = GaugeFieldSnapshotPayload::from_le_bytes(payload)?;
                Ok(Some(WalEntry::GaugeFieldSnapshot(payload)))
            }
            #[cfg(feature = "gauge")]
            OP_HAMILTONIAN_DECLARE => {
                let mut offset = 0;
                let name = read_string(payload, &mut offset)?;
                let kind_tag = read_string(payload, &mut offset)?;
                let group_tag = read_string(payload, &mut offset)?;
                if offset + 8 > payload.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "HAMILTONIAN_DECLARE registered_at",
                    ));
                }
                let registered_at = u64::from_le_bytes(
                    payload[offset..offset + 8].try_into().unwrap(),
                );
                Ok(Some(WalEntry::HamiltonianDeclare {
                    name,
                    kind_tag,
                    group_tag,
                    registered_at,
                }))
            }
            #[cfg(feature = "gauge")]
            OP_INTEGRATOR_CHOICE => {
                let mut offset = 0;
                let path = read_string(payload, &mut offset)?;
                let factory_name = read_string(payload, &mut offset)?;
                let handle_name = read_string(payload, &mut offset)?;
                Ok(Some(WalEntry::IntegratorChoice {
                    path,
                    factory_name,
                    handle_name,
                }))
            }
            #[cfg(feature = "imagine")]
            OP_IMAGINE_FALLBACK => {
                let mut offset = 0;
                let bundle = read_string(payload, &mut offset)?;
                if offset + 24 > payload.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "IMAGINE_FALLBACK payload truncated",
                    ));
                }
                let original_k = f64::from_le_bytes(
                    payload[offset..offset + 8].try_into().unwrap(),
                );
                offset += 8;
                let substituted_k = f64::from_le_bytes(
                    payload[offset..offset + 8].try_into().unwrap(),
                );
                offset += 8;
                let timestamp_ms = u64::from_le_bytes(
                    payload[offset..offset + 8].try_into().unwrap(),
                );
                Ok(Some(WalEntry::ImagineFallback {
                    bundle,
                    original_k,
                    substituted_k,
                    timestamp_ms,
                }))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown WAL op: {op:#x}"),
            )),
        }
    }
}

// ── Encoding helpers ──

fn write_string(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
}

fn read_string(data: &[u8], offset: &mut usize) -> io::Result<String> {
    if *offset + 4 > data.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "string length",
        ));
    }
    let len = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
    *offset += 4;
    if *offset + len > data.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "string data"));
    }
    let s = String::from_utf8(data[*offset..*offset + len].to_vec())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    *offset += len;
    Ok(s)
}

fn encode_value(v: &Value) -> Vec<u8> {
    let mut buf = Vec::new();
    match v {
        Value::Integer(i) => {
            buf.push(0x01);
            buf.extend_from_slice(&i.to_le_bytes());
        }
        Value::Float(f) => {
            buf.push(0x02);
            buf.extend_from_slice(&f.to_le_bytes());
        }
        Value::Text(s) => {
            buf.push(0x03);
            write_string(&mut buf, s);
        }
        Value::Bool(b) => {
            buf.push(0x04);
            buf.push(*b as u8);
        }
        Value::Timestamp(t) => {
            buf.push(0x05);
            buf.extend_from_slice(&t.to_le_bytes());
        }
        Value::Null => {
            buf.push(0x00);
        }
        Value::Vector(v) => {
            buf.push(0x06);
            buf.extend_from_slice(&(v.len() as u32).to_le_bytes());
            for &x in v {
                buf.extend_from_slice(&x.to_le_bytes());
            }
        }
        Value::Binary(b) => {
            buf.push(0x07);
            buf.extend_from_slice(&(b.len() as u32).to_le_bytes());
            buf.extend_from_slice(b);
        }
    }
    buf
}

fn decode_value(data: &[u8], offset: &mut usize) -> io::Result<Value> {
    if *offset >= data.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "value tag"));
    }
    let tag = data[*offset];
    *offset += 1;
    match tag {
        0x00 => Ok(Value::Null),
        0x01 => {
            let v = i64::from_le_bytes(data[*offset..*offset + 8].try_into().unwrap());
            *offset += 8;
            Ok(Value::Integer(v))
        }
        0x02 => {
            let v = f64::from_le_bytes(data[*offset..*offset + 8].try_into().unwrap());
            *offset += 8;
            Ok(Value::Float(v))
        }
        0x03 => {
            let s = read_string(data, offset)?;
            Ok(Value::Text(s))
        }
        0x04 => {
            let b = data[*offset] != 0;
            *offset += 1;
            Ok(Value::Bool(b))
        }
        0x05 => {
            let v = i64::from_le_bytes(data[*offset..*offset + 8].try_into().unwrap());
            *offset += 8;
            Ok(Value::Timestamp(v))
        }
        0x06 => {
            let len = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
            *offset += 4;
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                let x = f64::from_le_bytes(data[*offset..*offset + 8].try_into().unwrap());
                *offset += 8;
                v.push(x);
            }
            Ok(Value::Vector(v))
        }
        0x07 => {
            let len = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
            *offset += 4;
            let b = data[*offset..*offset + len].to_vec();
            *offset += len;
            Ok(Value::Binary(b))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown value tag: {tag:#x}"),
        )),
    }
}

fn encode_field_type(ft: &FieldType) -> Vec<u8> {
    let mut buf = Vec::new();
    match ft {
        FieldType::Numeric => buf.push(0x01),
        FieldType::Categorical => buf.push(0x02),
        FieldType::OrderedCat { order } => {
            buf.push(0x03);
            buf.extend_from_slice(&(order.len() as u32).to_le_bytes());
            for s in order {
                write_string(&mut buf, s);
            }
        }
        FieldType::Timestamp => buf.push(0x04),
        FieldType::Binary => buf.push(0x05),
        FieldType::Vector { dims } => {
            buf.push(0x06);
            buf.extend_from_slice(&(*dims as u32).to_le_bytes());
        }
    }
    buf
}

fn decode_field_type(data: &[u8], offset: &mut usize) -> io::Result<FieldType> {
    let tag = data[*offset];
    *offset += 1;
    match tag {
        0x01 => Ok(FieldType::Numeric),
        0x02 => Ok(FieldType::Categorical),
        0x03 => {
            let count = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
            *offset += 4;
            let mut order = Vec::with_capacity(count);
            for _ in 0..count {
                order.push(read_string(data, offset)?);
            }
            Ok(FieldType::OrderedCat { order })
        }
        0x04 => Ok(FieldType::Timestamp),
        0x05 => Ok(FieldType::Binary),
        0x06 => {
            let dims = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
            *offset += 4;
            Ok(FieldType::Vector { dims })
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown field type tag: {tag:#x}"),
        )),
    }
}

fn encode_field_def(fd: &FieldDef) -> Vec<u8> {
    let mut buf = Vec::new();
    write_string(&mut buf, &fd.name);
    buf.extend_from_slice(&encode_field_type(&fd.field_type));
    buf.extend_from_slice(&encode_value(&fd.default));
    // range: Option<f64> as tag + f64
    match fd.range {
        Some(r) => {
            buf.push(0x01);
            buf.extend_from_slice(&r.to_le_bytes());
        }
        None => {
            buf.push(0x00);
        }
    }
    buf.extend_from_slice(&fd.weight.to_le_bytes());
    buf
}

fn decode_field_def(data: &[u8], offset: &mut usize) -> io::Result<FieldDef> {
    let name = read_string(data, offset)?;
    let field_type = decode_field_type(data, offset)?;
    let default = decode_value(data, offset)?;
    let range_tag = data[*offset];
    *offset += 1;
    let range = if range_tag == 0x01 {
        let r = f64::from_le_bytes(data[*offset..*offset + 8].try_into().unwrap());
        *offset += 8;
        Some(r)
    } else {
        None
    };
    let weight = f64::from_le_bytes(data[*offset..*offset + 8].try_into().unwrap());
    *offset += 8;
    Ok(FieldDef {
        name,
        field_type,
        default,
        range,
        weight,
        // WAL records were written before the v0.2 encryption-mode field
        // existed; on load we default to None (plaintext). Bundles created
        // pre-v0.2 honor the bundle-level gauge_key path independently of
        // this field, so backwards-compat is preserved.
        encryption: crate::types::EncryptionMode::None,
        encryption_group: None,
    })
}

fn encode_schema(schema: &BundleSchema) -> Vec<u8> {
    let mut buf = Vec::new();
    write_string(&mut buf, &schema.name);
    buf.extend_from_slice(&(schema.base_fields.len() as u32).to_le_bytes());
    for f in &schema.base_fields {
        buf.extend_from_slice(&encode_field_def(f));
    }
    buf.extend_from_slice(&(schema.fiber_fields.len() as u32).to_le_bytes());
    for f in &schema.fiber_fields {
        buf.extend_from_slice(&encode_field_def(f));
    }
    buf.extend_from_slice(&(schema.indexed_fields.len() as u32).to_le_bytes());
    for idx in &schema.indexed_fields {
        write_string(&mut buf, idx);
    }
    // ── Gauge key (encryption) ──
    // Marker byte:
    //   0  = no key
    //   1  = v0.1 legacy: each transform is [f64 scale | f64 offset] (Affine only)
    //   2  = v0.2 tagged: each transform has a u8 variant tag followed by
    //        variant-specific payload (Affine: 16 B scale+offset; Opaque/
    //        Indexed: 32 B key)
    if let Some(ref gk) = schema.gauge_key {
        buf.push(2u8);
        buf.extend_from_slice(&(gk.transforms.len() as u32).to_le_bytes());
        for t in &gk.transforms {
            match t {
                crate::crypto::FieldTransform::Identity => {
                    buf.push(0x00);
                }
                crate::crypto::FieldTransform::Affine { scale, offset } => {
                    buf.push(0x01);
                    buf.extend_from_slice(&scale.to_le_bytes());
                    buf.extend_from_slice(&offset.to_le_bytes());
                }
                crate::crypto::FieldTransform::Opaque { key } => {
                    buf.push(0x02);
                    buf.extend_from_slice(key);
                }
                crate::crypto::FieldTransform::Indexed { key } => {
                    buf.push(0x03);
                    buf.extend_from_slice(key);
                }
                crate::crypto::FieldTransform::Probabilistic {
                    scale,
                    offset,
                    sigma,
                    bucket_key,
                } => {
                    // Tag 0x04: [f64 scale | f64 offset | f64 sigma | 32B bucket_key]
                    buf.push(0x04);
                    buf.extend_from_slice(&scale.to_le_bytes());
                    buf.extend_from_slice(&offset.to_le_bytes());
                    buf.extend_from_slice(&sigma.to_le_bytes());
                    buf.extend_from_slice(bucket_key);
                }
                crate::crypto::FieldTransform::Isometric {
                    group_id,
                    matrix,
                    offset_vec,
                    member_index,
                } => {
                    // Tag 0x05:
                    //   [u32 member_index]
                    //   [u32 group_id_len][group_id_bytes]
                    //   [u32 k][k*k * f64 matrix row-major][k * f64 offset]
                    buf.push(0x05);
                    buf.extend_from_slice(&(*member_index as u32).to_le_bytes());
                    let gid_bytes = group_id.as_bytes();
                    buf.extend_from_slice(&(gid_bytes.len() as u32).to_le_bytes());
                    buf.extend_from_slice(gid_bytes);
                    let k = matrix.len();
                    buf.extend_from_slice(&(k as u32).to_le_bytes());
                    for row in matrix {
                        for v in row {
                            buf.extend_from_slice(&v.to_le_bytes());
                        }
                    }
                    for v in offset_vec {
                        buf.extend_from_slice(&v.to_le_bytes());
                    }
                }
            }
        }
    } else {
        buf.push(0u8);
    }
    // ── Invariant constraints ──
    buf.extend_from_slice(&(schema.invariants.len() as u32).to_le_bytes());
    for inv in &schema.invariants {
        write_string(&mut buf, &inv.expr_field);
        buf.extend_from_slice(&inv.expected.to_le_bytes());
        buf.extend_from_slice(&inv.tol.to_le_bytes());
    }
    buf
}

fn decode_schema(data: &[u8]) -> io::Result<BundleSchema> {
    let mut offset = 0usize;
    let name = read_string(data, &mut offset)?;
    let mut schema = BundleSchema::new(&name);

    let base_count = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    for _ in 0..base_count {
        schema
            .base_fields
            .push(decode_field_def(data, &mut offset)?);
    }

    let fiber_count = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    for _ in 0..fiber_count {
        schema
            .fiber_fields
            .push(decode_field_def(data, &mut offset)?);
    }

    let idx_count = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    for _ in 0..idx_count {
        schema.indexed_fields.push(read_string(data, &mut offset)?);
    }

    // ── Gauge key (encryption) — may be absent in old WAL entries ──
    // Marker byte: 0 = no key, 1 = v0.1 legacy (scale+offset only),
    // 2 = v0.2 tagged (per-transform variant byte + payload).
    if offset < data.len() {
        let marker = data[offset];
        offset += 1;
        if marker == 1 {
            // v0.1 backwards-compat: every transform is Affine
            let n = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            let mut transforms = Vec::with_capacity(n);
            for _ in 0..n {
                let scale = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                offset += 8;
                let off_val = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                offset += 8;
                transforms.push(crate::crypto::FieldTransform::Affine {
                    scale,
                    offset: off_val,
                });
            }
            schema.gauge_key = Some(crate::crypto::GaugeKey { transforms });
        } else if marker == 2 {
            // v0.2: tagged per-transform variants
            let n = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            let mut transforms = Vec::with_capacity(n);
            for _ in 0..n {
                let tag = data[offset];
                offset += 1;
                match tag {
                    0x00 => {
                        transforms.push(crate::crypto::FieldTransform::Identity);
                    }
                    0x01 => {
                        let scale = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                        offset += 8;
                        let off_val = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                        offset += 8;
                        transforms.push(crate::crypto::FieldTransform::Affine {
                            scale,
                            offset: off_val,
                        });
                    }
                    0x02 => {
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&data[offset..offset + 32]);
                        offset += 32;
                        transforms.push(crate::crypto::FieldTransform::Opaque { key });
                    }
                    0x03 => {
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&data[offset..offset + 32]);
                        offset += 32;
                        transforms.push(crate::crypto::FieldTransform::Indexed { key });
                    }
                    0x04 => {
                        // Probabilistic: scale + offset + sigma + bucket_key
                        let scale = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                        offset += 8;
                        let off_val = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                        offset += 8;
                        let sigma = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                        offset += 8;
                        let mut bucket_key = [0u8; 32];
                        bucket_key.copy_from_slice(&data[offset..offset + 32]);
                        offset += 32;
                        transforms.push(crate::crypto::FieldTransform::Probabilistic {
                            scale,
                            offset: off_val,
                            sigma,
                            bucket_key,
                        });
                    }
                    0x05 => {
                        // Isometric: member_index + group_id + matrix + offset_vec
                        let member_index = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                        offset += 4;
                        let gid_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                        offset += 4;
                        let group_id = String::from_utf8_lossy(&data[offset..offset + gid_len]).into_owned();
                        offset += gid_len;
                        let k = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                        offset += 4;
                        let mut matrix = Vec::with_capacity(k);
                        for _ in 0..k {
                            let mut row = Vec::with_capacity(k);
                            for _ in 0..k {
                                row.push(f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap()));
                                offset += 8;
                            }
                            matrix.push(row);
                        }
                        let mut offset_vec = Vec::with_capacity(k);
                        for _ in 0..k {
                            offset_vec.push(f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap()));
                            offset += 8;
                        }
                        transforms.push(crate::crypto::FieldTransform::Isometric {
                            group_id,
                            matrix,
                            offset_vec,
                            member_index,
                        });
                    }
                    other => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unknown FieldTransform tag: {other:#x}"),
                        ));
                    }
                }
            }
            schema.gauge_key = Some(crate::crypto::GaugeKey { transforms });
        }
        // marker == 0 → no key, fall through
    }

    // ── Invariant constraints — may be absent in old WAL entries ──
    if offset + 4 <= data.len() {
        let inv_count = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        for _ in 0..inv_count {
            let field = read_string(data, &mut offset)?;
            let expected = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            let tol = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            schema.invariants.push(crate::types::InvariantDef { expr_field: field, expected, tol });
        }
    }

    Ok(schema)
}

fn encode_insert(bundle_name: &str, record: &Record) -> Vec<u8> {
    let mut buf = Vec::new();
    write_string(&mut buf, bundle_name);
    encode_record_into(&mut buf, record);
    buf
}

/// Encode a record (field count + sorted key-value pairs) into a buffer.
fn encode_record_into(buf: &mut Vec<u8>, record: &Record) {
    buf.extend_from_slice(&(record.len() as u32).to_le_bytes());
    let mut keys: Vec<&String> = record.keys().collect();
    keys.sort();
    for key in keys {
        write_string(buf, key);
        buf.extend_from_slice(&encode_value(&record[key]));
    }
}

/// Decode a record (field count + key-value pairs) from a buffer at offset.
fn decode_record(data: &[u8], offset: &mut usize) -> io::Result<Record> {
    let field_count = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
    *offset += 4;
    let mut record = Record::new();
    for _ in 0..field_count {
        let key = read_string(data, offset)?;
        let value = decode_value(data, offset)?;
        record.insert(key, value);
    }
    Ok(record)
}

fn decode_insert(data: &[u8]) -> io::Result<(String, Record)> {
    let mut offset = 0usize;
    let bundle_name = read_string(data, &mut offset)?;
    let record = decode_record(data, &mut offset)?;
    Ok((bundle_name, record))
}

fn encode_update(bundle_name: &str, key: &Record, patches: &Record) -> Vec<u8> {
    let mut buf = Vec::new();
    write_string(&mut buf, bundle_name);
    encode_record_into(&mut buf, key);
    encode_record_into(&mut buf, patches);
    buf
}

fn decode_update(data: &[u8]) -> io::Result<(String, Record, Record)> {
    let mut offset = 0usize;
    let bundle_name = read_string(data, &mut offset)?;
    let key = decode_record(data, &mut offset)?;
    let patches = decode_record(data, &mut offset)?;
    Ok((bundle_name, key, patches))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_schema() -> BundleSchema {
        BundleSchema::new("users")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"))
            .fiber(FieldDef::numeric("salary").with_range(100_000.0))
            .index("name")
    }

    fn test_record() -> Record {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(42));
        r.insert("name".into(), Value::Text("Alice".into()));
        r.insert("salary".into(), Value::Float(75000.0));
        r
    }

    /// Schema round-trip through encode/decode.
    #[test]
    fn schema_roundtrip() {
        let schema = test_schema();
        let encoded = encode_schema(&schema);
        let decoded = decode_schema(&encoded).unwrap();
        assert_eq!(decoded.name, schema.name);
        assert_eq!(decoded.base_fields.len(), schema.base_fields.len());
        assert_eq!(decoded.fiber_fields.len(), schema.fiber_fields.len());
        assert_eq!(decoded.indexed_fields, schema.indexed_fields);
        assert_eq!(decoded.base_fields[0].name, "id");
        assert_eq!(decoded.fiber_fields[0].name, "name");
        assert_eq!(decoded.fiber_fields[1].range, Some(100_000.0));
    }

    /// WAL-1: gauge_key survives encode → decode (was silently dropped before fix).
    #[test]
    fn schema_roundtrip_with_gauge_key() {
        use crate::types::EncryptionMode;
        // Test schema with mixed-mode fiber fields so the WAL roundtrip
        // exercises every FieldTransform variant tag.
        let mut schema = BundleSchema::new("users")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name").with_encryption(EncryptionMode::Opaque))
            .fiber(FieldDef::categorical("kind").with_encryption(EncryptionMode::Indexed))
            .fiber(
                FieldDef::numeric("salary")
                    .with_range(100_000.0)
                    .with_encryption(EncryptionMode::Affine),
            )
            .index("name");
        let seed = crate::crypto::GaugeKey::random_seed();
        schema.gauge_key = Some(crate::crypto::GaugeKey::derive(&seed, &schema.fiber_fields));
        let n_transforms = schema.gauge_key.as_ref().unwrap().transforms.len();
        let encoded = encode_schema(&schema);
        let decoded = decode_schema(&encoded).unwrap();
        let gk = decoded.gauge_key.expect("gauge_key must survive WAL roundtrip");
        assert_eq!(gk.transforms.len(), n_transforms);

        // Each transform variant must roundtrip with its associated material
        // (scale/offset for Affine, key bytes for Opaque/Indexed).
        let orig = &schema.gauge_key.as_ref().unwrap().transforms;
        for (a, b) in orig.iter().zip(gk.transforms.iter()) {
            match (a, b) {
                (
                    crate::crypto::FieldTransform::Affine { scale: s1, offset: o1 },
                    crate::crypto::FieldTransform::Affine { scale: s2, offset: o2 },
                ) => {
                    assert!((s1 - s2).abs() < 1e-15);
                    assert!((o1 - o2).abs() < 1e-15);
                }
                (
                    crate::crypto::FieldTransform::Opaque { key: k1 },
                    crate::crypto::FieldTransform::Opaque { key: k2 },
                ) => assert_eq!(k1, k2),
                (
                    crate::crypto::FieldTransform::Indexed { key: k1 },
                    crate::crypto::FieldTransform::Indexed { key: k2 },
                ) => assert_eq!(k1, k2),
                (
                    crate::crypto::FieldTransform::Identity,
                    crate::crypto::FieldTransform::Identity,
                ) => {}
                (left, right) => panic!(
                    "WAL roundtrip changed FieldTransform variant: {:?} → {:?}",
                    left, right
                ),
            }
        }
    }

    /// WAL-2: invariants survive encode → decode.
    #[test]
    fn schema_roundtrip_with_invariants() {
        let schema = BundleSchema::new("quat")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("q0"))
            .fiber(FieldDef::numeric("q1"))
            .fiber(FieldDef::numeric("q2"))
            .fiber(FieldDef::numeric("q3"))
            .with_invariant(crate::types::InvariantDef {
                expr_field: "q0".to_string(),
                expected: 1.0,
                tol: 1e-9,
            });
        let encoded = encode_schema(&schema);
        let decoded = decode_schema(&encoded).unwrap();
        assert_eq!(decoded.invariants.len(), 1);
        assert_eq!(decoded.invariants[0].expr_field, "q0");
        assert!((decoded.invariants[0].expected - 1.0).abs() < 1e-15);
        assert!((decoded.invariants[0].tol - 1e-9).abs() < 1e-30);
    }

    /// Insert record round-trip.
    #[test]
    fn insert_roundtrip() {
        let rec = test_record();
        let encoded = encode_insert("users", &rec);
        let (name, decoded) = decode_insert(&encoded).unwrap();
        assert_eq!(name, "users");
        assert_eq!(decoded, rec);
    }

    /// CRC32 detects corruption.
    #[test]
    fn crc_integrity() {
        let data = b"Hello, GIGI!";
        let c1 = crc32(data);
        let c2 = crc32(data);
        assert_eq!(c1, c2);

        let mut corrupted = data.to_vec();
        corrupted[5] ^= 0xFF;
        assert_ne!(crc32(&corrupted), c1);
    }

    /// Full WAL write + replay cycle.
    #[test]
    fn wal_write_read_cycle() {
        let dir = std::env::temp_dir().join("gigi_wal_test");
        let _ = fs::create_dir_all(&dir);
        let wal_path = dir.join("test.wal");
        let _ = fs::remove_file(&wal_path);

        // Write
        {
            let mut wal = WalWriter::open(&wal_path).unwrap();
            wal.log_create_bundle(&test_schema()).unwrap();
            wal.log_insert("users", &test_record()).unwrap();

            let mut r2 = Record::new();
            r2.insert("id".into(), Value::Integer(99));
            r2.insert("name".into(), Value::Text("Bob".into()));
            r2.insert("salary".into(), Value::Float(90000.0));
            wal.log_insert("users", &r2).unwrap();

            wal.log_checkpoint().unwrap();
            wal.sync().unwrap();
            assert_eq!(wal.entry_count(), 4);
        }

        // Read
        {
            let mut reader = WalReader::open(&wal_path).unwrap();
            let entries = reader.read_all().unwrap();
            assert_eq!(entries.len(), 4);

            match &entries[0] {
                WalEntry::CreateBundle(s) => assert_eq!(s.name, "users"),
                _ => panic!("Expected CreateBundle"),
            }
            match &entries[1] {
                WalEntry::Insert {
                    bundle_name,
                    record,
                } => {
                    assert_eq!(bundle_name, "users");
                    assert_eq!(record.get("id"), Some(&Value::Integer(42)));
                }
                _ => panic!("Expected Insert"),
            }
            match &entries[3] {
                WalEntry::Checkpoint => {}
                _ => panic!("Expected Checkpoint"),
            }
        }

        let _ = fs::remove_file(&wal_path);
        let _ = fs::remove_dir(dir);
    }

    /// Value encoding covers all types.
    #[test]
    fn value_all_types() {
        let values = vec![
            Value::Integer(i64::MIN),
            Value::Integer(i64::MAX),
            Value::Float(std::f64::consts::PI),
            Value::Float(-0.0),
            Value::Text(String::new()),
            Value::Text("Hello 🌍".into()),
            Value::Bool(true),
            Value::Bool(false),
            Value::Timestamp(1710000000000),
            Value::Timestamp(-1),
            Value::Null,
        ];
        for v in &values {
            let encoded = encode_value(v);
            let mut offset = 0;
            let decoded = decode_value(&encoded, &mut offset).unwrap();
            assert_eq!(&decoded, v, "round-trip failed for {v:?}");
            assert_eq!(offset, encoded.len());
        }
    }

    /// Update WAL entry round-trip.
    #[test]
    fn update_roundtrip() {
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(42));
        let mut patches = Record::new();
        patches.insert("name".into(), Value::Text("Alice_v2".into()));
        patches.insert("salary".into(), Value::Float(95000.0));

        let encoded = encode_update("users", &key, &patches);
        let (name, dec_key, dec_patches) = decode_update(&encoded).unwrap();
        assert_eq!(name, "users");
        assert_eq!(dec_key, key);
        assert_eq!(dec_patches, patches);
    }

    /// WAL write+replay cycle with update and delete.
    #[test]
    fn wal_update_delete_cycle() {
        let dir = std::env::temp_dir().join("gigi_wal_test_ud");
        let _ = fs::create_dir_all(&dir);
        let wal_path = dir.join("test_ud.wal");
        let _ = fs::remove_file(&wal_path);

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let mut patches = Record::new();
        patches.insert("name".into(), Value::Text("Updated".into()));

        {
            let mut wal = WalWriter::open(&wal_path).unwrap();
            wal.log_create_bundle(&test_schema()).unwrap();
            wal.log_insert("users", &test_record()).unwrap();
            wal.log_update("users", &key, &patches).unwrap();
            wal.log_delete("users", &key).unwrap();
            wal.log_checkpoint().unwrap();
            wal.sync().unwrap();
            assert_eq!(wal.entry_count(), 5);
        }

        {
            let mut reader = WalReader::open(&wal_path).unwrap();
            let entries = reader.read_all().unwrap();
            assert_eq!(entries.len(), 5);

            match &entries[2] {
                WalEntry::Update {
                    bundle_name,
                    key: k,
                    patches: p,
                } => {
                    assert_eq!(bundle_name, "users");
                    assert_eq!(k, &key);
                    assert_eq!(p, &patches);
                }
                _ => panic!("Expected Update"),
            }
            match &entries[3] {
                WalEntry::Delete {
                    bundle_name,
                    key: k,
                } => {
                    assert_eq!(bundle_name, "users");
                    assert_eq!(k, &key);
                }
                _ => panic!("Expected Delete"),
            }
        }

        let _ = fs::remove_file(&wal_path);
        let _ = fs::remove_dir(dir);
    }

    /// MeasurementOverride WAL entry round-trip.
    #[test]
    fn measurement_override_roundtrip() {
        let dir = std::env::temp_dir().join("gigi_wal_test_mo");
        let _ = fs::create_dir_all(&dir);
        let wal_path = dir.join("test_mo.wal");
        let _ = fs::remove_file(&wal_path);

        let mut key = Record::new();
        key.insert("patient_id".into(), Value::Integer(7));

        // Write
        {
            let mut writer = WalWriter::open(&wal_path).unwrap();
            writer
                .log_measurement_override(
                    "vitals",
                    "heart_rate",
                    &key,
                    72.5,          // old_completed_value
                    0.85,          // old_confidence
                    78.0,          // new_measured_value
                    1710000000000, // timestamp
                )
                .unwrap();
            writer.sync().unwrap();
        }

        // Read back
        {
            let mut reader = WalReader::open(&wal_path).unwrap();
            let entries = reader.read_all().unwrap();
            assert_eq!(entries.len(), 1);
            match &entries[0] {
                WalEntry::MeasurementOverride {
                    bundle_name,
                    field,
                    key: k,
                    old_completed_value,
                    old_confidence,
                    new_measured_value,
                    timestamp,
                } => {
                    assert_eq!(bundle_name, "vitals");
                    assert_eq!(field, "heart_rate");
                    assert_eq!(k, &key);
                    assert!((old_completed_value - 72.5).abs() < f64::EPSILON);
                    assert!((old_confidence - 0.85).abs() < f64::EPSILON);
                    assert!((new_measured_value - 78.0).abs() < f64::EPSILON);
                    assert_eq!(*timestamp, 1710000000000);
                }
                _ => panic!("Expected MeasurementOverride"),
            }
        }

        let _ = fs::remove_file(&wal_path);
        let _ = fs::remove_dir(dir);
    }

    /// TDD-HAL-V.1: GaugeFieldSnapshotPayload round-trips byte-identically
    /// through explicit little-endian encode/decode (Bee's locked decision
    /// D-V-A). Buffer is a 90-edge × 4-component SU(2) buckyball-sized
    /// identity field (q0=1, q1=q2=q3=0 per edge); SHA-256 is initialized
    /// to zeros for this structural-only check (the SHA-recompute gate is
    /// `tdd_hal_v_1_snapshot_payload_sha256_recomputed`).
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_1_snapshot_payload_le_roundtrip() {
        // 90 edges × 4 floats per edge = 360 entries (SU(2) identity).
        let mut buffer = Vec::with_capacity(90 * 4);
        for _ in 0..90 {
            buffer.push(1.0);
            buffer.push(0.0);
            buffer.push(0.0);
            buffer.push(0.0);
        }
        let payload = GaugeFieldSnapshotPayload {
            name: "U".to_string(),
            group: crate::gauge::Group::SU2,
            buffer: buffer.clone(),
            sha256: [0u8; 32],
        };
        let bytes = payload.to_le_bytes();
        let decoded = GaugeFieldSnapshotPayload::from_le_bytes(&bytes).unwrap();
        // Strict equality on every field — Vec<f64> equality is exact
        // here because the buffer is constructed from bit-identical
        // f64 literals (1.0, 0.0) and the LE encoding round-trip
        // preserves all 64 bits.
        assert_eq!(decoded.name, "U");
        assert_eq!(decoded.group, crate::gauge::Group::SU2);
        assert_eq!(decoded.sha256, [0u8; 32]);
        assert_eq!(decoded.buffer.len(), 360);
        assert_eq!(decoded.buffer, buffer);
        // And the whole struct (PartialEq is bit-exact on f64).
        assert_eq!(decoded, payload);
    }

    /// TDD-HAL-V.1: serialized payload size for the SU(2) buckyball
    /// snapshot matches the spec's 2880-byte buffer plus the documented
    /// framing. Buffer-only contribution is 90 × 4 × 8 = 2880 bytes
    /// (spec §0). The total entry framing is name_len_prefix(4) +
    /// name_bytes(1 for "U") + group_tag(1) + sha256(32) +
    /// buffer_len_prefix(4) = 42 framing bytes, total 2922.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_1_snapshot_payload_size_buckyball() {
        let buffer = vec![0.0f64; 90 * 4]; // SU(2) on the buckyball
        let payload = GaugeFieldSnapshotPayload {
            name: "U".to_string(),
            group: crate::gauge::Group::SU2,
            buffer,
            sha256: [0u8; 32],
        };
        let bytes = payload.to_le_bytes();
        // Buffer-only contribution: 360 entries × 8 bytes/entry = 2880.
        // This is the spec's headline number — the rest is framing.
        let buffer_bytes = 90 * 4 * 8;
        assert_eq!(buffer_bytes, 2880);
        // Framing: 4 (name len) + 1 ("U") + 1 (group tag SU2) + 32 (sha256)
        //         + 4 (buffer length count) = 42 bytes.
        let framing = 4 + "U".len() + 1 + 32 + 4;
        assert_eq!(framing, 42);
        assert_eq!(bytes.len(), framing + buffer_bytes);
        assert_eq!(bytes.len(), 2922);
    }

    /// TDD-HAL-V.1: SHA-256 of the buffer bytes is recomputable at
    /// decode time and must agree with the payload's `sha256` field
    /// when the buffer is intact (Bee's locked decision D-V-C: the
    /// same SHA-256 is the citation handle in the WAL entry AND the
    /// Rows envelope returned to the caller). This gate catches a bug
    /// where the payload's sha256 disagrees with the actual buffer.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_1_snapshot_payload_sha256_recomputed() {
        // Realistic thermalized-ish buffer: 360 distinct f64 values.
        let buffer: Vec<f64> = (0..(90 * 4))
            .map(|i| (i as f64) * 0.001 + 0.123_456_789)
            .collect();
        // Construct via the canonical helper so sha256 is the SHA over
        // the LE-encoded buffer.
        let payload =
            GaugeFieldSnapshotPayload::from_buffer("U".to_string(), crate::gauge::Group::SU2, buffer.clone());
        // Sanity: the stored sha256 is not the zero hash.
        assert_ne!(payload.sha256, [0u8; 32]);

        // Round-trip via LE bytes.
        let bytes = payload.to_le_bytes();
        let decoded = GaugeFieldSnapshotPayload::from_le_bytes(&bytes).unwrap();
        assert_eq!(decoded.buffer, buffer);
        assert_eq!(decoded.sha256, payload.sha256);

        // Re-derive SHA-256 from the decoded buffer's LE bytes; this is
        // exactly what the replay path will do. Must match the stored
        // SHA byte-for-byte.
        let recomputed = GaugeFieldSnapshotPayload::compute_buffer_sha256(&decoded.buffer);
        assert_eq!(
            recomputed, decoded.sha256,
            "SHA-256 recomputed at decode time must match payload.sha256"
        );

        // The hash is also stable across two independent computations on
        // the same buffer (sanity check the helper is deterministic).
        let again = GaugeFieldSnapshotPayload::compute_buffer_sha256(&buffer);
        assert_eq!(again, payload.sha256);
    }
}
