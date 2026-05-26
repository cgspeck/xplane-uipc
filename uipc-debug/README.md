# uipc-debug

Offline and live debug tool for FSUIPC offset mappings.

## Usage

```text
Usage: uipc-debug --mapping <MAPPING> [OPTIONS]

Options:
  -m, --mapping <MAPPING>    Path to mappings.toml
  -s, --state <STATE>        Path to state CSV (dataref key-value pairs)
      --log-level <LEVEL>    Log level [default: trace] [possible values: trace, debug, info, warn, error]
      --log-file <PATH>      Log file path [default: uipc-debug.log]
      --no-log-file          Disable file logging
      --no-ipc               Disable IPC host mode (offline CSV debugging)
  -h, --help                 Print help
```

## Modes

**IPC mode (default):** Starts the IPC host via `ipc_host`, making evaluated offset values available to FSUIPC clients (e.g., Self Loading Cargo) over shared memory. The value table is updated at ~20Hz from the eval engine.

**Offline mode (`--no-ipc`):** Evaluate mappings against a static state CSV file. No IPC server is started. Useful for testing formulas without a running X-Plane or FSUIPC client.

## Input Files

### Mappings (`mappings.toml`)

Standard FSUIPC offset mapping file used by the `xplane_uipc` plugin. Defines offset addresses, data types, and source datarefs with optional scaling and expressions.

### State CSV (`state.csv`)

Headerless CSV with `dataref_path,value` pairs. Example:

```csv
sim/flightmodel/position/indicated_airspeed,250.0
sim/cockpit2/controls/parking_brake_ratio,0.5
```

Missing keys are reported in the trace log and show `—` in the FSUIPC value column.

## TUI Controls

| Key     | Action                           |
|---------|----------------------------------|
| `q`     | Quit (cleanly shuts down IPC)    |
| `?`     | Toggle help overlay              |
| `Tab`   | Cycle focus (Table ↻ Log)        |
| `↑/↓`  | Navigate table rows              |
| `PgUp`/`PgDn` | Scroll trace log          |
| `End`   | Resume log auto-scroll           |
| `Enter` | Open expression detail popup     |
| `Esc`   | Close popup                      |
| `r`     | Reload mapping file              |
| `s`     | Load state CSV                   |
| `l`     | Toggle log pane                  |
| `w`     | Write state CSV (0-fill missing) |
| `c`     | Write computed FSUIPC values CSV |

## Building

From the workspace root:

```shell
cargo build -p uipc-debug
```
