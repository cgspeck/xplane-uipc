## Context

"FSUIPC for Programmers" lists `0x66C0`, 64 bytes: "Free for general use, for example in button or keys programming." Real FSUIPC is one block of memory, so these bytes behave like any other offset: a write changes them, and a read returns them.

In xplane-uipc, reads and writes go through the value table, which holds one typed value per offset and is filled each cycle from `mappings.toml` by the flight loop. Nothing in the table models raw memory a client can write. Since the protocol change, a read the table can't answer is zero-filled and warned about once per offset. A write to an offset that isn't mapped is ignored and warned about once.

The IPC window procedure runs on the IPC thread and owns the per-window state; today that's a `WarnedSet` in `GWLP_USERDATA`. The planned `log-lua-requests` change replaces it with an `IpcState` struct.

## Goals / Non-Goals

**Goals:**
- Make `0x66C0`–`0x66FF` behave like FSUIPC memory: stored writes, faithful reads, shared by all clients.
- Keep it entirely on the IPC thread, with no flight loop or mapping involvement.

**Non-Goals:**
- Exposing the area to X-Plane (a dataref or Lua bridge). That's a follow-up if the Lua-request logs show clients need FSUIPC Lua scripts.
- Keeping the area across plugin restarts. FSUIPC starts with it cleared, too.
- Making other offsets act as raw memory.

## Decisions

1. **The buffer lives in `IpcState`.** `user_area: [u8; 64]`, zeroed when the window is created. The window procedure is the only thing that uses it, on one thread, so it needs no locking. It survives "Reload Mappings" (which only touches the flight loop's state) and "Reset Warnings" (which only clears `warned`).

2. **Byte-level semantics, clipped to the area.** The range is `USER_AREA = 0x66C0..=0x66FF`, a constant exported from `ipc_host`.
   - **Reads** that start inside the area copy the stored bytes up to the end of the area, and zero-fill whatever part of the payload extends past it.
   - **Writes** that start inside the area store the bytes that fall inside it. Anything past the end is dropped, and a debug message says so.
   - Records that start before the area and run into it keep going through the value table, so mapped offsets just below `0x66C0` behave as before. Clients use the area at its own offsets, so this edge case doesn't need more.
   - Neither path consults the value table, and neither produces "not in table" or unmapped-write warnings. Writes are never forwarded to the flight loop and never count as rejected.

3. **Logging.** Each write is logged at `debug` with its offset, size and bytes as hex. Reads aren't logged beyond the existing `trace` output. A configurable level isn't needed: nothing about these writes is unusual.

4. **The area takes precedence over mappings.** At load, mappings with an offset in `USER_AREA` are dropped with a warning, next to `drop_builtin_offsets`. A mapping would never be served anyway, since the area is checked first; dropping it makes that visible.
   - *Alternative:* add the area to `BUILTIN_OFFSETS`. Rejected: the `builtin-offsets` spec defines built-ins as read-only and plugin-generated, and this area is the opposite.

5. **This change and `log-lua-requests` share `IpcState`.** Whichever is implemented first adds the struct and moves `WarnedSet` into it. The second adds its field.

## Risks / Trade-offs

- [Low] **A client could read another client's data.** That's what the area is for in FSUIPC.
- [Low] **A record overlapping the start of the area isn't handled byte by byte** (see decision 2). If a client is seen doing that, it can be extended.

## Open Questions

- Does the client reading `0x66F8` expect a FSUIPC Lua script (requested via `0x0D70`) to fill it? The `log-lua-requests` change will show which script it asks for, and whether a dataref bridge is worth building.
