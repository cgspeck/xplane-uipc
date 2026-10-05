use uipc_mapping::FsuipcType;

#[derive(Clone, Debug)]
pub enum Value {
    UnsignedInteger8(u8),
    Integer8(i8),
    UnsignedInteger16(u16),
    Integer16(i16),
    UnsignedInteger32(u32),
    Integer32(i32),
    UnsignedInteger64(u64),
    Integer64(i64),
    Float32(f32),
    Float64(f64),
    Bool(bool),
    String(Vec<u8>),
}

impl Value {
    /// Convert a mapping's f64 result to the declared FSUIPC type.
    ///
    /// Integers round to nearest (half away from zero); types up to 32 bits wrap
    /// two's complement when out of range, matching FSUIPC's raw-memory semantics.
    /// Returns `None` for non-finite input and for `String`.
    pub fn from_f64(value: f64, ty: FsuipcType) -> Option<Value> {
        if !value.is_finite() {
            return None;
        }
        let r = value.round();
        let wrapped = r as i64;
        Some(match ty {
            FsuipcType::U8 => Value::UnsignedInteger8(wrapped as u8),
            FsuipcType::I8 => Value::Integer8(wrapped as i8),
            FsuipcType::U16 => Value::UnsignedInteger16(wrapped as u16),
            FsuipcType::I16 => Value::Integer16(wrapped as i16),
            FsuipcType::U32 => Value::UnsignedInteger32(wrapped as u32),
            FsuipcType::I32 => Value::Integer32(wrapped as i32),
            FsuipcType::U64 if r < 0.0 => Value::UnsignedInteger64(wrapped as u64),
            FsuipcType::U64 => Value::UnsignedInteger64(r as u64),
            FsuipcType::I64 => Value::Integer64(wrapped),
            FsuipcType::F32 => Value::Float32(value as f32),
            FsuipcType::F64 => Value::Float64(value),
            FsuipcType::String => return None,
        })
    }

    /// Little-endian encoding of the value as it appears in FSUIPC memory.
    pub fn to_le_bytes(&self) -> Vec<u8> {
        match self {
            Value::UnsignedInteger8(v) => v.to_le_bytes().to_vec(),
            Value::Integer8(v) => v.to_le_bytes().to_vec(),
            Value::UnsignedInteger16(v) => v.to_le_bytes().to_vec(),
            Value::Integer16(v) => v.to_le_bytes().to_vec(),
            Value::UnsignedInteger32(v) => v.to_le_bytes().to_vec(),
            Value::Integer32(v) => v.to_le_bytes().to_vec(),
            Value::UnsignedInteger64(v) => v.to_le_bytes().to_vec(),
            Value::Integer64(v) => v.to_le_bytes().to_vec(),
            Value::Float32(v) => v.to_le_bytes().to_vec(),
            Value::Float64(v) => v.to_le_bytes().to_vec(),
            Value::Bool(v) => vec![*v as u8],
            Value::String(bytes) => bytes.clone(),
        }
    }

    /// Decode a client write payload using this value's type. The payload must
    /// be exactly the type's width. Strings and bools are not writable.
    pub fn decode_le(&self, bytes: &[u8]) -> Option<f64> {
        fn arr<const N: usize>(b: &[u8]) -> Option<[u8; N]> {
            b.try_into().ok()
        }
        Some(match self {
            Value::UnsignedInteger8(_) => u8::from_le_bytes(arr(bytes)?) as f64,
            Value::Integer8(_) => i8::from_le_bytes(arr(bytes)?) as f64,
            Value::UnsignedInteger16(_) => u16::from_le_bytes(arr(bytes)?) as f64,
            Value::Integer16(_) => i16::from_le_bytes(arr(bytes)?) as f64,
            Value::UnsignedInteger32(_) => u32::from_le_bytes(arr(bytes)?) as f64,
            Value::Integer32(_) => i32::from_le_bytes(arr(bytes)?) as f64,
            Value::UnsignedInteger64(_) => u64::from_le_bytes(arr(bytes)?) as f64,
            Value::Integer64(_) => i64::from_le_bytes(arr(bytes)?) as f64,
            Value::Float32(_) => f32::from_le_bytes(arr(bytes)?) as f64,
            Value::Float64(_) => f64::from_le_bytes(arr(bytes)?),
            Value::Bool(_) | Value::String(_) => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub value: Value,
    pub source: u16,
    pub destination: u16,
    pub writable: bool,
}

#[derive(Debug)]
pub struct Table {
    pub entries: Box<[Option<Entry>; 65536]>,
    /// Offsets inserted since the last `clear_active_and_writable()`, in insertion order.
    pub active: Vec<u16>,
    /// Writable offsets inserted since the last `clear_active_and_writable()`.
    pub writable: Vec<u16>,
    // O(1) membership flags mirroring `active` / `writable`.
    active_mask: Box<[bool]>,
    writable_mask: Box<[bool]>,
}

impl Table {
    pub fn new() -> Self {
        let entries: Box<[Option<Entry>; 65536]> =
            vec![None; 65536].into_boxed_slice().try_into().unwrap();
        Table {
            entries,
            active: Vec::new(),
            writable: Vec::new(),
            active_mask: vec![false; 65536].into_boxed_slice(),
            writable_mask: vec![false; 65536].into_boxed_slice(),
        }
    }

    pub fn insert(&mut self, index: u16, entry: Entry) {
        let i = index as usize;
        if !self.active_mask[i] {
            self.active_mask[i] = true;
            self.active.push(index);
        }
        if entry.writable && !self.writable_mask[i] {
            self.writable_mask[i] = true;
            self.writable.push(index);
        }
        self.entries[i] = Some(entry);
    }

    pub fn clear_active_and_writable(&mut self) {
        for &i in &self.active {
            self.active_mask[i as usize] = false;
        }
        for &i in &self.writable {
            self.writable_mask[i as usize] = false;
        }
        self.active.clear();
        self.writable.clear();
    }

    /// Start an update cycle: returns the previous cycle's active offsets and
    /// clears the bookkeeping. Pass the result to `end_update`.
    pub fn begin_update(&mut self) -> Vec<u16> {
        let previous = self.active.clone();
        self.clear_active_and_writable();
        previous
    }

    /// Finish an update cycle: drop entries that were active last cycle but were
    /// neither inserted nor kept this cycle, so stale values are not served.
    pub fn end_update(&mut self, previous: Vec<u16>) {
        for i in previous {
            if !self.active_mask[i as usize] {
                self.entries[i as usize] = None;
            }
        }
    }

    /// Re-activate an existing entry without changing its value. No-op if absent.
    pub fn keep(&mut self, index: u16) {
        if let Some(entry) = self.entries[index as usize].take() {
            self.insert(index, entry);
        }
    }

    pub fn is_active(&self, index: u16) -> bool {
        self.active_mask[index as usize]
    }

    pub fn is_writable(&self, index: u16) -> bool {
        self.writable_mask[index as usize]
    }

    pub fn get(&self, index: u16) -> Option<&Entry> {
        self.entries[index as usize].as_ref()
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

use std::sync::{Arc, LazyLock, RwLock};

pub static VALUE_TABLE: LazyLock<RwLock<Arc<RwLock<Table>>>> =
    LazyLock::new(|| RwLock::new(Arc::new(RwLock::new(Table::new()))));

pub fn create_table_with_entries(entries: &[(u16, Entry)]) -> Arc<RwLock<Table>> {
    let mut table = Table::new();
    for &(index, ref entry) in entries {
        table.insert(index, entry.clone());
    }
    Arc::new(RwLock::new(table))
}

pub fn set_value_table(table: Arc<RwLock<Table>>) {
    *VALUE_TABLE.write().unwrap() = table;
}

pub fn get_value_table() -> Arc<RwLock<Table>> {
    VALUE_TABLE.read().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_active_and_writable_then_repopulate() {
        let mut table = Table::new();
        table.insert(
            10,
            Entry {
                value: Value::UnsignedInteger32(100),
                source: 0,
                destination: 0,
                writable: true,
            },
        );
        table.insert(
            20,
            Entry {
                value: Value::UnsignedInteger32(200),
                source: 0,
                destination: 0,
                writable: false,
            },
        );
        assert_eq!(table.active.len(), 2);
        assert_eq!(table.writable.len(), 1);

        table.clear_active_and_writable();
        assert!(table.active.is_empty());
        assert!(table.writable.is_empty());

        // Entries still exist in array but vectors are empty
        table.insert(
            30,
            Entry {
                value: Value::UnsignedInteger32(300),
                source: 0,
                destination: 0,
                writable: true,
            },
        );
        table.insert(
            40,
            Entry {
                value: Value::UnsignedInteger32(400),
                source: 0,
                destination: 0,
                writable: false,
            },
        );
        assert_eq!(table.active.len(), 2);
        assert_eq!(table.writable.len(), 1);
    }

    /// Regression: re-inserting the same offsets after a clear must repopulate
    /// `active`/`writable`. Previously only empty slots were tracked, so every
    /// update cycle after the first left both vectors empty and all writes failed.
    #[test]
    fn test_reinsert_same_offsets_after_clear() {
        let mut table = Table::new();
        for _ in 0..3 {
            table.clear_active_and_writable();
            table.insert(
                10,
                Entry {
                    value: Value::UnsignedInteger32(1),
                    source: 0,
                    destination: 0,
                    writable: true,
                },
            );
            table.insert(20, entry(Value::UnsignedInteger32(2)));
            table.insert(20, entry(Value::UnsignedInteger32(3)));
            assert_eq!(table.active, vec![10, 20]);
            assert_eq!(table.writable, vec![10]);
            assert!(table.is_writable(10));
            assert!(!table.is_writable(20));
            assert!(table.is_active(20));
        }
        table.clear_active_and_writable();
        assert!(!table.is_active(10));
        assert!(!table.is_writable(10));
    }

    #[test]
    fn test_from_f64_rounds_instead_of_truncating() {
        assert!(matches!(
            Value::from_f64(1023.9999, FsuipcType::I32),
            Some(Value::Integer32(1024))
        ));
        assert!(matches!(
            Value::from_f64(-2.5, FsuipcType::I16),
            Some(Value::Integer16(-3))
        ));
        assert!(matches!(
            Value::from_f64(0.4999, FsuipcType::U8),
            Some(Value::UnsignedInteger8(0))
        ));
    }

    #[test]
    fn test_from_f64_wraps_out_of_range() {
        assert!(matches!(
            Value::from_f64(-1.0, FsuipcType::U16),
            Some(Value::UnsignedInteger16(0xFFFF))
        ));
        assert!(matches!(
            Value::from_f64(-1.0, FsuipcType::U32),
            Some(Value::UnsignedInteger32(u32::MAX))
        ));
        assert!(matches!(
            Value::from_f64(256.0, FsuipcType::U8),
            Some(Value::UnsignedInteger8(0))
        ));
        assert!(matches!(
            Value::from_f64(40000.0, FsuipcType::I16),
            Some(Value::Integer16(-25536))
        ));
        assert!(matches!(
            Value::from_f64(-1.0, FsuipcType::U64),
            Some(Value::UnsignedInteger64(u64::MAX))
        ));
        assert!(matches!(
            Value::from_f64(1.8e19, FsuipcType::U64),
            Some(Value::UnsignedInteger64(18_000_000_000_000_000_000))
        ));
    }

    #[test]
    fn test_from_f64_rejects_non_finite_and_string() {
        assert!(Value::from_f64(f64::NAN, FsuipcType::I32).is_none());
        assert!(Value::from_f64(f64::INFINITY, FsuipcType::F64).is_none());
        assert!(Value::from_f64(1.0, FsuipcType::String).is_none());
    }

    #[test]
    fn test_from_f64_floats_unrounded() {
        assert!(matches!(
            Value::from_f64(1.25, FsuipcType::F32),
            Some(Value::Float32(v)) if v == 1.25
        ));
        assert!(matches!(
            Value::from_f64(1.25, FsuipcType::F64),
            Some(Value::Float64(v)) if v == 1.25
        ));
    }

    #[test]
    fn test_decode_le_uses_type() {
        let v = Value::Integer16(0);
        assert_eq!(v.decode_le(&[0xFF, 0xFF]), Some(-1.0));
        let v = Value::UnsignedInteger16(0);
        assert_eq!(v.decode_le(&[0xFF, 0xFF]), Some(65535.0));
        let v = Value::Float32(0.0);
        assert_eq!(v.decode_le(&1.5f32.to_le_bytes()), Some(1.5));
        let v = Value::Integer64(0);
        assert_eq!(v.decode_le(&(-5i64).to_le_bytes()), Some(-5.0));
        let v = Value::Integer32(0);
        assert_eq!(v.decode_le(&(-100_000i32).to_le_bytes()), Some(-100_000.0));
    }

    #[test]
    fn test_decode_le_rejects_wrong_size_and_unwritable() {
        assert_eq!(Value::UnsignedInteger32(0).decode_le(&[1, 2]), None);
        assert_eq!(Value::Float64(0.0).decode_le(&[0; 4]), None);
        assert_eq!(Value::String(vec![0]).decode_le(&[0]), None);
        assert_eq!(Value::Bool(false).decode_le(&[1]), None);
    }

    #[test]
    fn test_to_le_bytes() {
        assert_eq!(
            Value::UnsignedInteger32(0x12345678).to_le_bytes(),
            vec![0x78, 0x56, 0x34, 0x12]
        );
        assert_eq!(Value::Integer16(-1).to_le_bytes(), vec![0xFF, 0xFF]);
        assert_eq!(Value::Bool(true).to_le_bytes(), vec![1]);
    }

    fn entry(value: Value) -> Entry {
        Entry {
            value,
            source: 0,
            destination: 0,
            writable: false,
        }
    }

    /// Verify that every Value variant round-trips through the table correctly.
    #[test]
    fn test_value_variants_round_trip() {
        let mut table = Table::new();

        let cases: Vec<(u16, Value)> = vec![
            // Unsigned integers
            (0, Value::UnsignedInteger8(0)),
            (1, Value::UnsignedInteger8(127)),
            (2, Value::UnsignedInteger8(255)),
            (3, Value::UnsignedInteger16(0)),
            (4, Value::UnsignedInteger16(32767)),
            (5, Value::UnsignedInteger16(65535)),
            (6, Value::UnsignedInteger32(0)),
            (7, Value::UnsignedInteger32(2_147_483_647)),
            (8, Value::UnsignedInteger32(4_294_967_295)),
            (9, Value::UnsignedInteger64(0)),
            (10, Value::UnsignedInteger64(u64::MAX)),
            // Signed integers
            (20, Value::Integer8(0)),
            (21, Value::Integer8(127)),
            (22, Value::Integer8(-1)),
            (23, Value::Integer8(-128)),
            (24, Value::Integer16(0)),
            (25, Value::Integer16(32767)),
            (26, Value::Integer16(-1)),
            (27, Value::Integer16(-10)),
            (28, Value::Integer16(-32768)),
            (29, Value::Integer32(0)),
            (30, Value::Integer32(2_147_483_647)),
            (31, Value::Integer32(-1)),
            (32, Value::Integer32(-2_147_483_648)),
            (33, Value::Integer64(0)),
            (34, Value::Integer64(i64::MAX)),
            (35, Value::Integer64(-1)),
            (36, Value::Integer64(i64::MIN)),
            // Floats
            (40, Value::Float32(0.0)),
            (41, Value::Float32(3.14)),
            (42, Value::Float32(-273.15)),
            (43, Value::Float64(0.0)),
            (44, Value::Float64(3.14159265358979)),
            (45, Value::Float64(-273.15)),
            // Bool
            (50, Value::Bool(true)),
            (51, Value::Bool(false)),
        ];

        for (offset, value) in &cases {
            table.insert(*offset, entry(value.clone()));
        }

        for (offset, expected) in &cases {
            let stored = &table.get(*offset).unwrap().value;
            match (stored, expected) {
                (Value::UnsignedInteger8(a), Value::UnsignedInteger8(b)) => assert_eq!(a, b),
                (Value::Integer8(a), Value::Integer8(b)) => assert_eq!(a, b),
                (Value::UnsignedInteger16(a), Value::UnsignedInteger16(b)) => assert_eq!(a, b),
                (Value::Integer16(a), Value::Integer16(b)) => assert_eq!(a, b),
                (Value::UnsignedInteger32(a), Value::UnsignedInteger32(b)) => assert_eq!(a, b),
                (Value::Integer32(a), Value::Integer32(b)) => assert_eq!(a, b),
                (Value::UnsignedInteger64(a), Value::UnsignedInteger64(b)) => assert_eq!(a, b),
                (Value::Integer64(a), Value::Integer64(b)) => assert_eq!(a, b),
                (Value::Float32(a), Value::Float32(b)) => assert_eq!(a, b),
                (Value::Float64(a), Value::Float64(b)) => assert_eq!(a, b),
                (Value::Bool(a), Value::Bool(b)) => assert_eq!(a, b),
                (Value::String(a), Value::String(b)) => assert_eq!(a, b),
                _ => panic!(
                    "variant mismatch at offset {}: stored {:?}, expected {:?}",
                    offset, stored, expected
                ),
            }
        }
    }

    /// The specific bug that triggered this fix: i16 with value -10 must not become 0.
    #[test]
    fn test_signed_int16_negative_value() {
        let mut table = Table::new();
        table.insert(0x0246, entry(Value::Integer16(-10)));
        match &table.get(0x0246).unwrap().value {
            Value::Integer16(v) => assert_eq!(*v, -10),
            other => panic!("expected SignedInt16, got {:?}", other),
        }
    }
}
