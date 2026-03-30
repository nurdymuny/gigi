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
//!   0xFF = CHECKPOINT (marks that all prior entries are flushed to data file)

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
const OP_CHECKPOINT: u8 = 0xFF;

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
}
