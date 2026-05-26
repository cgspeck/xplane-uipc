## Why

Need a .NET test client that can inspect FSUIPC offset values in real-time and batch mode, for testing the uipc-debug IPC host, X-Plane, and MSFS+FSUIPC7. The existing Rust `uipc-debug` tool evaluates mappings from dataref expressions — this tool reads raw FSUIPC offsets directly from the client side, providing the complementary perspective.

## What Changes

- Create a new .NET console application in `fsuipc-test-client/`
- Input file format: `<offset>,<type>[,<size>]` — fixed-size types need no explicit size
- **TUI mode** using Spectre.Console: live offset table with configurable refresh rate (~1-2 Hz)
- **Batch mode**: one-shot query, outputs JSON to stdout
- Keybindings mirroring the existing `uipc-debug` tool (`q`, `?`, `↑/↓`, `r`, `s`, `Tab`, etc.)
- Auto-connects to FSUIPC host via FSUIPCClientDLL (`UIPCMAIN` window discovery — no targeting)

## Capabilities

### New Capabilities
- `offset-input-file`: Reading and parsing FSUIPC offset definitions from a text file
- `tui-inspector`: Real-time TUI display of offset values with file reload, save, and row navigation
- `batch-query`: One-shot batch query mode with JSON output

### Modified Capabilities
<!-- None — this is a new tool, not modifying existing capabilities -->

## Impact

- New directory `fsuipc-test-client/` in the workspace
- New dependencies: `FSUIPCClientDLL` NuGet package, `Spectre.Console` NuGet package
- .NET project targeting net8.0-windows (or net9.0-windows)
- No changes to existing Rust code
