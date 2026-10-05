## 1. Core Implementation

- [ ] 1.1 Add `ACTIVITY_COUNTER_OFFSET: u16 = 0x337E` and `BUILTIN_OFFSETS: &[u16]` constants in `xplane_uipc/src/plugin_state.rs`
- [ ] 1.2 Add an `activity_counter: u16` field to `PluginState`, initialised to `0` in `PluginState::new()`
- [ ] 1.3 In `PluginState::update()`, after the mapping loop and before `end_update()`, do `wrapping_add(1)` on the counter and insert it at `0x337E` as `Value::UnsignedInteger16` with `writable: false`
- [ ] 1.4 In `find_load_and_resolve_mappings()` (`lib.rs`), filter out mappings whose offset is in `BUILTIN_OFFSETS`, logging `tracing::warn!` with the offset, before building `ResolvedMapping`s
- [ ] 1.5 Confirm the reload path still only replaces `state.mappings`, so the counter persists

## 2. Testing

- [ ] 2.1 Unit test in `plugin_state.rs`: `PluginState::new(vec![])` with `update()` called twice serves `0x337E` as `UnsignedInteger16(1)` then `(2)`, active and not writable
- [ ] 2.2 Unit test: with the counter set to `0xFFFF`, the next `update()` serves `0`
- [ ] 2.3 Unit test for the collision filter: a mapping on `0x337E` is dropped and the others are kept (extract the filter into a testable function)
- [ ] 2.4 `ipc_host` test: a 2-byte write to a non-writable `u16` entry at `0x337E` is not forwarded (likely already covered by existing non-writable tests; add one if not)
- [ ] 2.5 Run `make` (or `cargo test` on Windows) and check that everything passes

## 3. Docs

- [ ] 3.1 Document built-in offsets (`0x337E` activity counter, read-only, overrides mappings) in the README's mapping section
