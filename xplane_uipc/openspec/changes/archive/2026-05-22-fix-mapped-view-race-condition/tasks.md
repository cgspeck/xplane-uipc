## 1. WarnedSet: Add clear_key method

- [x] 1.1 Add `clear_key(key: u16, category: WarnCategory)` method to `WarnedSet` in `ipc_host/src/warning.rs` that clears the bit for a specific offset+category using `fetch_and` with the inverted bit mask
- [x] 1.2 Add unit tests for `clear_key`: cleared key can warn again, clearing one key doesn't affect others, clearing one category doesn't affect others
- [x] 1.3 Run `cargo test -p ipc_host` to verify warning module tests pass

## 2. Table: Add clear_active_and_writable method

- [x] 2.1 Add `clear_active_and_writable(&mut self)` method to `Table` in `ipc_host/src/value_table.rs` that empties the `active` and `writable` vectors
- [x] 2.2 Add a unit test verifying vectors are cleared and `insert()` repopulates them correctly after clearing

## 3. PluginState: Add populate_table method

- [x] 3.1 Add `populate_table(&mut self)` method to `PluginState` in `xplane_uipc/src/plugin_state.rs` that iterates all mappings, calls `read_xplane()`, and uses `table.insert()` for each successful value
- [x] 3.2 Ensure `populate_table()` handles the case where `read_xplane()` returns `None` (unresolved datarefs) by skipping those entries
- [x] 3.3 Export `populate_table` as a public method so it can be called from `lib.rs`

## 4. lib.rs: Call populate_table before IPC thread spawn

- [x] 4.1 In `XPluginEnable()`, call `state.populate_table()` after `load_mappings_and_init()` and before `thread::spawn()` for the IPC thread
- [x] 4.2 Add tracing info log indicating table population is complete with count of populated entries

## 5. PluginState: Fix update() to use insert() and rebuild vectors

- [x] 5.1 Modify `update()` in `xplane_uipc/src/plugin_state.rs` to call `table.clear_active_and_writable()` at the start of each update cycle
- [x] 5.2 Replace direct `table.entries[m.offset as usize] = Some(...)` assignment with `table.insert(m.offset, entry)` for all mappings
- [x] 5.3 Verify the `writable` field on `Entry` is set correctly from `m.writable`

## 6. Clear warnings after table population

- [x] 6.1 In `mapped_view.rs` or a shared location, after table population completes, clear `ReadNotExist` warnings for all newly-added offsets by calling `warned_set.clear_key(offset, ReadNotExist)`
- [x] 6.2 Ensure the `WarnedSet` reference is accessible where table population occurs (may require passing it through or using a shared reference)

## 7. Integration testing and verification

- [x] 7.1 Run `cargo test` for all crates to verify no regressions
- [x] 7.2 Build the plugin with `cargo build --release` and verify compilation succeeds
- [x] 7.3 Verify the spec requirements are met: table-readiness and warning-lifecycle scenarios
