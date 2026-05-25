#[derive(Clone, Debug)]
pub enum Value {
    UnsignedInt8(u8),
    UnsignedInt16(u16),
    UnsignedInteger32(u32),
    Integer64(i64),
    Float64(f64),
    Bool(bool),
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
    pub active: Vec<u16>,
    pub writable: Vec<u16>,
}

impl Table {
    pub fn new() -> Self {
        let entries: Box<[Option<Entry>; 65536]> =
            vec![None; 65536].into_boxed_slice().try_into().unwrap();
        Table {
            entries,
            active: Vec::new(),
            writable: Vec::new(),
        }
    }

    pub fn insert(&mut self, index: u16, entry: Entry) {
        if self.entries[index as usize].is_none() {
            self.active.push(index);
        }
        let writable = entry.writable;
        self.entries[index as usize] = Some(entry);
        if writable {
            self.writable.push(index);
        }
    }

    pub fn clear_active_and_writable(&mut self) {
        self.active.clear();
        self.writable.clear();
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
}
