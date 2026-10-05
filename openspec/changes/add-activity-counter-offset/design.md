## Context

Every value-table entry currently comes from a `ResolvedMapping` built from `mappings.toml`. `PluginState::update()` runs once per flight loop callback (`flight_loop_callback` in `lib.rs`, at `update_rate_hz`, 20 Hz by default). It brackets the inserts with `begin_update()`/`end_update()`, so any offset not inserted or kept in a cycle is expired.

On "Reload Mappings", the existing `PluginState` is kept and only `state.mappings` is replaced. Fields on `PluginState` other than `mappings` therefore persist across reloads.

## Goals / Non-Goals

**Goals:**
- Serve `0x337E` as a `u16` that increments once per update cycle and wraps at `0xFFFF`.
- Keep it read-only and protect it from being shadowed by user mappings.
- Keep the implementation small, and make the next built-in offset easy to add.

**Non-Goals:**
- A general plugin-side offset framework, or config to enable or disable built-ins.
- Pausing the counter while X-Plane is paused. Flight loop callbacks keep running while paused, as FSUIPC's sim callbacks do, and clients use the counter for liveness rather than sim-time.
- Counting IPC requests from clients.

## Decisions

1. **One activity = one flight loop update cycle.** FSUIPC counts calls from the simulator, and the flight loop callback is X-Plane's equivalent. Counting client IPC requests would let a client's own polling move the counter, which defeats a liveness check.
   - *Alternative:* count every X-Plane callback regardless of `update_rate_hz`. Rejected because the plugin has only one callback and its rate already is the update rate.

2. **The counter lives on `PluginState` and is written in `update()`, after the mapping loop.** It runs on the main thread with the table write lock already held, so it adds no new synchronization. It uses `wrapping_add(1)` on a `u16` field. Each cycle it is inserted as `Value::UnsignedInteger16` with `writable: false`, so `end_update()` never expires it.

3. **Built-ins take precedence over user mappings.** When mappings are resolved, any mapping whose offset is a built-in offset is removed and a warning is logged. Removing it, rather than letting the built-in overwrite it in the table, also keeps `write_offset()` from forwarding a client write to that mapping's dataref.
   - *Alternative:* fail the load. Rejected because one bad entry shouldn't disable every other mapping. Other per-entry problems already go to `load_errors`/warnings.

4. **Built-in offsets are listed in a small const slice (`BUILTIN_OFFSETS`) next to `PluginState`,** so the collision filter doesn't hardcode `0x337E`. No trait or registry is added until a second built-in exists.

5. **The counter is not reset on reload.** A reload is invisible to clients, and a counter that jumps back to 0 could look like a restart. It starts at 0 when the plugin starts.

## Risks / Trade-offs

- [Low] **Rate differs from FSUIPC.** FSUIPC increments at the sim's message rate, which is often higher than 20 Hz. Clients use the counter for "is it changing", not for a rate, so this is acceptable.
- [Low] **Counter stops while the flight loop doesn't run** (for example during scenery loads). That matches FSUIPC behaviour when the sim stops calling it.
- [Low] **Existing user mappings for `0x337E` stop being served.** A grep of the shipped `mappings.toml` shows none, and the warning makes the change visible.
