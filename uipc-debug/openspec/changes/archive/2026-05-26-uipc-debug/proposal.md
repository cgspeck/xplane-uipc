## Why

Debugging FSUIPC offset mappings currently requires running X-Plane, observing client behavior (e.g. Self Loading Cargo), and inferring what went wrong from log files. There is no way to test mapping formulas (scaling, expressions) in isolation, inspect what FSUIPC value a given set of dataref inputs produces, or hand-craft test cases without the simulator running.

## What Changes

- Add a new workspace crate `uipc-debug` — a standalone TUI console tool for offline analysis and testing of FSUIPC mappings
- The tool loads `mappings.toml` and a hand-editable state CSV (dataref values), then computes and displays the resulting FSUIPC offset values per mapping
- Dual-pane TUI: mapping state table (with raw inputs + computed outputs) and a trace log
- Interactive keybindings for reloading, loading/writing state files, and writing computed values
- Expression detail popup showing the RPN and per-variable breakdown
- No reverse dependencies — other workspace crates do not depend on `uipc-debug`

## Capabilities

### New Capabilities
- `mapping-evaluation`: Load mappings and state, compute FSUIPC values from dataref inputs, display results in a TUI table with expression detail popup
- `state-file-io`: Load and write CSV state files (dataref key-value pairs); auto-populate missing keys with 0 on write
- `fsuipc-output-export`: Write computed FSUIPC offset values to CSV
- `trace-logging`: Per-session tracing captured in a TUI pane and optionally written to a log file

### Modified Capabilities

- (none)

## Impact

- New crate `uipc-debug` in the workspace with dependencies on `uipc-mapping` + `uipc-expr` (already in workspace), plus `ratatui`, `clap`, and `tracing`
- No changes to existing crates; no breaking changes
- `Cargo.toml` workspace members list updated
