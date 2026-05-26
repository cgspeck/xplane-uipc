## Context

The `uipc-debug` tool evaluates FSUIPC mappings offline against a static CSV state file (`HashMap<String, f64>`). The `ipc_host` crate provides the Win32 IPC window, shared-memory protocol parsing, and value table infrastructure already used by the `xplane_uipc` plugin. The `ipc_host` crate is a workspace member but is not currently a dependency of `uipc-debug`.

The `xplane_uipc` plugin demonstrates the pattern: it spawns an IPC thread running `create_ipc_window_and_run()`, populates the value table from evaluated mappings in a flight-loop callback, and sends `IpcCommands::Shutdown` on disable. `uipc-debug` has a TUI event loop instead of a flight loop, so periodic table population must happen in the event loop.

The tool's primary purpose is to evaluate mappings and serve those values — the IPC host is the outward-facing interface to FSUIPC clients. The TUI is a debugging/monitoring display on top of the same evaluation pipeline.

## Goals / Non-Goals

**Goals:**
- Add `ipc_host` as a dependency of `uipc-debug`
- IPC mode is enabled by default: run the IPC host so clients can read evaluated offsets
- Spawn IPC thread running `create_ipc_window_and_run()`, periodically populate the value table from `EvalEngine` results, and cleanly shut down on quit
- Value table entries use the same type-conversion logic as `xplane_uipc::PluginState::update()`
- Clean shutdown: send `IpcCommands::Shutdown` on quit, join thread, handle errors
- TUI status indicator showing IPC vs. offline mode
- `--no-ipc` flag to disable the IPC host for offline-only CSV debugging
- No breaking changes to existing offline functionality

**Non-Goals:**
- Writing to FSUIPC offsets from the TUI (no write-back from keyboard input)
- Supporting the FSUIPC write protocol to clients (the IPC host only serves reads; writes come from the IPC shared memory protocol)
- Multiple simultaneous IPC connections
- Reading values from the IPC table for TUI display (TUI reads from eval engine directly, same as offline mode)

## Decisions

### 1. Architecture: single eval pipeline, two outputs

```
                          ┌──────────────────────┐
                          │   EvalEngine          │
                          │   (state: HashMap)    │
                          └──────┬───────────────┘
                                 │ evaluate_all()
                                 │
                          ┌──────▼───────────────┐
                          │   MappingResult[]     │
                          └──┬───────────────┬───┘
                             │               │
                    ┌────────▼───┐   ┌──────▼────────┐
                    │ TUI Table  │   │ value_table    │
                    │ (display)  │   │ (IPC shared    │
                    │            │   │  memory server)│
                    └────────────┘   └──────┬─────────┘
                                            │
                                    ┌───────▼─────────┐
                                    │ FSUIPC clients  │
                                    │ (SLC, etc.)     │
                                    └─────────────────┘
```

**Rationale:** The eval engine produces `MappingResult[]`. These results drive both the TUI display (existing behavior) and the IPC value table (new behavior). The TUI never reads from the value table — it reads from its own cached results, exactly as in offline mode. This keeps the two outputs decoupled and avoids circular dependencies.

### 2. IPC thread lifecycle

The IPC thread is spawned immediately after loading mappings (by default, unless `--no-ipc` is given). The tool stores the `JoinHandle` and the command channel `Sender<IpcCommands>` in `Option` fields of `App` for access from the TUI event loop.

**Rationale:** Following the `xplane_uipc` pattern exactly. The command channel is the only way to signal shutdown.

### 3. Periodic table population

In the TUI event loop, a `Duration::from_millis(50)` elapsed counter tracks when to re-evaluate mappings and repopulate the value table. On each tick:
1. Call `EvalEngine::evaluate_all()` to get fresh values (this is already called on state changes in offline mode)
2. Convert results to `ipc_host::value_table::Entry` values
3. Call `set_value_table()` with a new table built from evaluated entries

**Rationale:** 50ms (~20Hz) matches the `xplane_uipc` plugin's update rate. This is a polling approach — the eval engine is stateless and cheap enough to re-run on every tick.

### 4. --no-ipc flag semantics

- IPC mode is the default: the tool acts as an FSUIPC server, making evaluated offsets available to clients via the `ipc_host` shared memory
- `--no-ipc` disables the IPC thread entirely, reverting to the original offline behavior (evaluate against CSV, display results in TUI only)
- If IPC initialization fails (e.g., window class registration), the tool logs the error and falls back to offline mode gracefully

**Rationale:** Making IPC the default means the tool serves its primary purpose (providing FSUIPC values to clients) without extra flags. The `--no-ipc` flag preserves the original CSV-debugging use case.

### 5. TUI mode indicator

When IPC mode is active, the table title shows ` Mappings (IPC) ` in green. When `--no-ipc` is used, the title shows ` Mappings (Offline) ` in default styling.

**Rationale:** Clear visual distinction between modes. Minimal TUI changes — just the title bar.

### 6. Clean shutdown

On `q` press in the TUI event loop:
1. Send `IpcCommands::Shutdown` via the stored channel sender
2. Join the IPC thread (with timeout to avoid hanging)
3. Exit normally

**Rationale:** Following the `xplane_uipc` pattern exactly. The `Shutdown` command causes `create_ipc_window_and_run()` to call `DestroyWindow` and exit its message loop.

## Risks / Trade-offs

- **[Error handling]** If the IPC thread panics (e.g., cannot register window class), the TUI should fall back gracefully to offline mode. **Mitigation:** The thread uses `anyhow::Result` and will log the error before the panic propagates. The TUI detects the failure via a shared error flag and transitions to offline mode.
- **[Performance]** Populating the value table on every tick could be expensive for large mappings files. **Mitigation:** Only active/writable vectors are rebuilt; individual entries are inserted by index. For typical mapping files (~100-500 entries), the cost is negligible.
- **[Windows console]** `create_ipc_window_and_run` uses Win32 window messaging which requires a compatible thread context. **Mitigation:** The IPC thread is a dedicated thread, separate from the TUI main thread, matching the `xplane_uipc` pattern.
