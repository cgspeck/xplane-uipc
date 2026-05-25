## 1. Define Table Types

- [x] 1.1 Add Value enum and Entry struct to lib.rs or a shared module
- [x] 1.2 Add Table struct with entries: Box<[Option<Entry>; 65536]> and active: Vec<u16>
- [x] 1.3 Implement Table::new() constructor

## 2. Initialize Table in lib.rs

- [x] 2.1 Create global/static table wrapped in Arc<RwLock<Table>>
- [x] 2.2 Populate table entries in XPluginEnable for offsets 0x3304, 0x3308, 0x3124, 0x320c
- [x] 2.3 Add necessary imports (Arc, RwLock)

## 3. Update uipc_host.rs to Use Table

- [x] 3.1 Add table as parameter or global to wnd_proc()
- [x] 3.2 Replace HashMap contains_key + get with table.entries[index].is_some() check
- [x] 3.3 Replace HashMap value extraction with table.entries[index].clone()
- [x] 3.4 Remove HashMap and ValueType usage from uipc_host.rs

## 4. Verify and Test

- [x] 4.1 Run cargo check to verify compilation
- [x] 4.2 Run cargo test if tests exist
- [ ] 4.3 Verify plugin loads in X-Plane (if possible)