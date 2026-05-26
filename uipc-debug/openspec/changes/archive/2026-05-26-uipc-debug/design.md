## Context

The workspace already has `uipc-mapping` (parses `mappings.toml` into `DatarefMapping`/`MappingSource`) and `uipc-expr` (RPN expression evaluator). The plugin's `PluginState` does the actual evaluation via `ResolvedMapping::read_xplane()`, but that requires X-Plane's SDK to resolve dataref handles — not available offline.

A new crate `uipc-debug` will reuse the mapping loading and expression evaluation from these crates, replacing the X-Plane-specific dataref resolution with a static key-value lookup (the state CSV).

## Goals / Non-Goals

**Goals:**
- Standalone TUI that loads mappings and state, computes FSUIPC values offline
- Reuse `uipc-mapping` parsing and `uipc-expr` evaluation without modification
- Dual-pane layout: mapping state table + toggleable trace log pane
- Expression detail popup with per-variable values
- Keybindings for reload, load/write state, write computed output, log pane toggle, help, quit
- TRACE-level logging by default, written to file and in-memory ring buffer
- File logging path and log level configurable via CLI; file logging can be disabled
- No reverse dependencies from other workspace crates

**Non-Goals:**
- Live connection to X-Plane or the IPC shared memory
- Writing offsets back to any running system
- Inverting expressions (many-to-one cannot be reversed)
- Supporting the FSUIPC write protocol
- Modifying existing crates or the plugin

## Decisions

### 1. Architecture: evaluation layer separate from TUI

```
┌──────────────────────────────────────────────────────────┐
│  uipc-debug                                              │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  App (state machine)                               │  │
│  │  ┌───────────┐  ┌───────────┐  ┌────────────────┐  │  │
│  │  │ Mapping   │  │ State     │  │ Eval Engine    │  │  │
│  │  │ Store     │  │ Store     │  │ (forward pass)  │  │  │
│  │  │ (parsed)  │  │ (HashMap) │  │ Simple: v*s+oa  │  │  │
│  │  │           │  │           │  │ Expr: eval(rpn) │  │  │
│  │  │           │  │           │  │ Static: return  │  │  │
│  │  └───────────┘  └───────────┘  └────────────────┘  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────────┐│
│  │  TUI (ratatui + crossterm)                          ││
│  │  ┌────────────────────┐                             ││
│  │  │ Offset Table       │  ◀─ expands when log       ││
│  │  │ Pane               │      pane is hidden        ││
│  │  └────────────────────┘                             ││
│  │  ┌────────────────────┐  (togglable with `l`)      ││
│  │  │ Trace Log          ├── ring buffer ← tracing    ││
│  │  │ Pane               │   layer                    ││
│  │  └────────────────────┘                             ││
│  │  ┌──────────────┐  ┌──────────────┐                ││
│  │  │ Help Overlay │  │ Expr Detail  │                ││
│  │  │ (modal)      │  │ (popup)      │                ││
│  │  └──────────────┘  └──────────────┘                ││
│  └──────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────┘
```

**Rationale:** Keeping evaluation logic separate from rendering makes the tool testable and allows the eval engine to be reused if a non-TUI mode is added later (e.g., batch evaluation).

### 2. Eval Engine: forward pass from dataref inputs to FSUIPC values

The state CSV maps `dataref_path → value`. For each mapping:

- **Simple**: look up the `dataref_path` in state → compute `fsuipc = value * scale + offset_add`
- **Expr**: for each `(name, path)` in `datarefs`, look up `path` in state → collect values by `name` → call `expr.eval(&vars)`
- **Static**: return the static value directly

**Rationale:** This is exactly what `ResolvedMapping::read_xplane()` does, minus the X-Plane SDK. The state CSV replaces live dataref reads.

### 3. CSV state file format

```csv
sim/flightmodel/position/indicated_airspeed,250.0
sim/cockpit2/controls/parking_brake_ratio,0.5
```

Simple headerless CSV, one key-value per line. Keys are always full dataref paths (from `dataref` or the values in `datarefs` maps). No quoting needed for typical values.

**Rationale:** Maximally hand-editable. No special parsing needed — split on `,`, trim whitespace, parse second as f64.

### 4. Expression detail popup

When a row with an expression source is selected and `Enter` is pressed, a popup shows:
- Full RPN expression string
- Table of `variable → dataref_path → value → contribution` for each input

The "contribution" column is just the value; it's not an attempt to decompose the expression (which would require symbolic analysis of RPN).

**Rationale:** Honest about what we can compute. The useful information is "what values went into this expression" — the user can reason about the RPN themselves.

### 5. TUI framework: ratatui + crossterm

Standard Rust TUI stack. `ratatui` for widgets (table, paragraph, popup), `crossterm` for terminal backend and raw-mode input handling on Windows.

**Rationale:** Mature, well-maintained, no platform issues on Windows (the only target for this project).

### 6. Tracing setup

The tool has its own `tracing_subscriber` with two sinks:
- **TUI pane**: in-memory ring buffer (`VecDeque<String>`, configurable capacity, default 1000)
- **File**: written to `uipc-debug.log` in the current directory by default

Both sinks receive all events simultaneously. The ring buffer is read by the TUI trace pane widget.

**Log level:** Defaults to `TRACE` (the most verbose level). Override with `--log-level <LEVEL>` accepting `trace`, `debug`, `info`, `warn`, `error`.

**File logging:** Enabled by default, writing to `uipc-debug.log`. Override the path with `--log-file <PATH>`. Disable file logging entirely with `--no-log-file`.

**Layout response to toggle:** When the log pane is hidden, the offset table pane expands to fill the full terminal height. The toggle is instantaneous — no re-initialization of the terminal or tracing layer needed (the ring buffer keeps accepting entries regardless).

**Rationale:** TRACE by default because the tool's purpose is debugging — verbose output is the point. File logging on by default so nothing is lost if the terminal scrolls or the pane is hidden. A dedicated flag to disable file logging avoids accidental log accumulation in scripted use.

### 7. Keybinding design

| Key | Scope | Action |
|-----|-------|--------|
| `q` | Global | Quit |
| `?` | Global | Toggle help overlay |
| `Tab` | Global | Cycle pane focus |
| `l` | Global | Toggle trace log pane visibility |
| `r` | Global | Prompt mapping path → reload |
| `s` | Global | Prompt state path → load state CSV |
| `w` | Global | Write state CSV (with 0-fill) |
| `c` | Global | Write computed FSUIPC output CSV |
| `↑`/`↓` | Table pane | Navigate rows |
| `PgUp`/`PgDn` | Log pane | Scroll trace log up/down |
| `Enter` | Table pane | Open expression detail popup (if expr source) |
| `Esc` | Popup | Close popup |

## Risks / Trade-offs

- **[Complexity]** `ratatui` adds a non-trivial dependency and build time. **Mitigation:** It's a dev tool, not the plugin; build time impact on the plugin is zero.
- **[CSV parsing]** Hand-edited CSV has no schema validation. A typo in a dataref path silently yields a missing-key warning. **Mitigation:** The trace pane logs all missing keys; keys not found in any mapping are flagged.
- **[Large mappings]** A mapping file with hundreds of entries could overwhelm a terminal. **Mitigation:** The table pane scrolls; a search/filter could be added later.
- **[Windows console]** Windows terminal can be quirky with raw-mode TUI. **Mitigation:** `crossterm` handles platform differences; testing on Windows Terminal and ConEmu.
