## Tasks

- [ ] 1. Add `uipc-mapping` dependency to `ipc_host` (`ipc_host/Cargo.toml`)
- [ ] 2. Add `Value::from_f64(f64, FsuipcType) -> Option<Value>` with rounding/wrapping/non-finite rules, plus tests (`ipc_host/src/value_table.rs`)
- [ ] 3. Add `Value::to_le_bytes()` and `Value::decode_le(&[u8]) -> Option<f64>`, plus tests (`ipc_host/src/value_table.rs`)
- [ ] 4. Add `Table::begin_update()`, `Table::end_update(prev)` and `Table::keep(index)`, plus tests for expiry and retention (`ipc_host/src/value_table.rs`)
- [ ] 5. Bound numeric reads to `nBytes` using `to_le_bytes`, plus a test (`ipc_host/src/mapped_view.rs`)
- [ ] 6. Decode writes via the entry's `decode_le`; reject size mismatches without forwarding, plus tests (`ipc_host/src/mapped_view.rs`)
- [ ] 7. Replace `f64_to_value` with `Value::from_f64` in `xplane_uipc/src/plugin_state.rs` and `uipc-debug/src/tui.rs`
- [ ] 8. Introduce `Reading::{Value, Retain, Missing}` and use begin/keep/end in `PluginState::update()` (`xplane_uipc/src/plugin_state.rs`)
- [ ] 9. Round integer and integer-array dataref writes (`xplane_uipc/src/plugin_state.rs`)
- [ ] 10. Fix `read_bytes` NUL/size bounding and empty-string handling (`xplane_uipc/src/plugin_state.rs`)
- [ ] 11. Correct the `read_xplane` doc comment (missing expression variables default to 0.0)
- [ ] 12. Run `cargo fmt`, `cargo clippy`, `cargo test --workspace`
