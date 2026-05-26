## 1. Crate setup

- [x] 1.1 Create `uipc-debug/` directory with `Cargo.toml` adding `uipc-mapping`, `uipc-expr`, `ratatui`, `crossterm`, `clap` (derive), and `tracing` as dependencies
- [x] 1.2 Add `uipc-debug` to workspace `members` in root `Cargo.toml`
- [x] 1.3 Create `src/main.rs` with `clap` derive CLI: `--mapping <path>`, `--state [path]`, `--log-level [LEVEL]`, `--log-file [PATH]`, `--no-log-file`

## 2. Evaluation engine

- [x] 2.1 Create `src/eval.rs` with a struct that owns parsed mappings + a `HashMap<String, f64>` for state
- [x] 2.2 Implement `fn evaluate_all(&self) -> Vec<MappingResult>` that iterates mappings, resolves dataref values from state, and computes FSUIPC values per the forward-pass logic (simple: `v*s+oa`, expr: `eval(rpn)`, static: direct)
- [x] 2.3 Implement `fn missing_keys(&self) -> Vec<String>` to report dataref paths referenced by mappings but absent from state
- [x] 2.4 Unit tests for evaluate_all with simple, expr, and static mappings, including missing-key and edge cases

## 3. CSV state I/O

- [x] 3.1 Create `src/state.rs` with `fn load_state(path) -> Result<HashMap<String, f64>>` parsing headerless CSV
- [x] 3.2 Implement `fn write_state(path, state, all_keys)` that writes all keys (filling missing with 0.0)
- [x] 3.3 Implement `fn write_fsuipc_output(path, results)` writing `offset,type,value,writable` CSV
- [x] 3.4 Unit tests for CSV round-trip and missing-key filling

## 4. Trace logging setup

- [x] 4.1 Create `src/trace.rs` with a shared ring buffer (`VecDeque<String>`, configurable capacity, default 1000)
- [x] 4.2 Implement a custom `tracing_subscriber::Layer` that writes to the ring buffer
- [x] 4.3 File logging: default writes to `uipc-debug.log`, overridable via `--log-file <PATH>`, disable via `--no-log-file`
- [x] 4.4 Configurable log level via `--log-level <LEVEL>` (default TRACE), applied via `tracing_subscriber::filter`

## 5. TUI layout and widgets

- [x] 5.1 Create `src/tui.rs` with `ratatui` main loop: terminal setup (raw mode, alternate screen), event polling
- [x] 5.2 Layout: vertical split with offset table pane (top) and trace log pane (bottom, ~30%). Table expands to fill terminal when log pane is hidden
- [x] 5.3 Offset table widget showing columns: Offset, Type, W, Inputs, FSUIPC Value, Source
- [x] 5.4 Trace log widget: scrollable list with PgUp/PgDn navigation, auto-scroll, pause-on-scroll-up, End-to-resume
- [x] 5.5 Expression detail popup widget (rendered as a centered overlay with expression string + variable table)

## 6. Keybindings and actions

- [x] 6.1 Implement navigation: `↑`/`↓` for table row selection, `Tab` to cycle pane focus
- [x] 6.2 Implement `Enter` on expression row → open detail popup, `Esc` → close
- [x] 6.3 Implement `r` → prompt for mapping path → reload mappings via `uipc-mapping::load_mappings()`
- [x] 6.4 Implement `s` → prompt for state path → load state CSV
- [x] 6.5 Implement `l` → toggle trace log pane visibility (table expands when hidden)
- [x] 6.6 Implement `w` → prompt for output path → write state CSV with 0-fill
- [x] 6.7 Implement `c` → prompt for output path → write computed FSUIPC values CSV
- [x] 6.8 Implement `?` → toggle help overlay (keybinding reference)
- [x] 6.9 Implement `q` → clean shutdown

## 7. Integration and polish

- [x] 7.1 Wire everything together in `main.rs`: parse CLI args → load mapping → optionally load state → enter TUI loop
- [x] 7.2 Ensure clean error handling: invalid mapping path, parse errors, state load failures all reported in trace pane (not panics)
- [x] 7.3 Test with real `mappings.toml` and a hand-crafted state CSV for a few offsets
- [x] 7.4 Run `cargo fmt` and `cargo test` on the workspace
