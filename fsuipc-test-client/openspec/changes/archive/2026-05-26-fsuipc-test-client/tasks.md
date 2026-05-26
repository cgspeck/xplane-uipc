## 1. Project scaffold

- [x] 1.1 Create .NET project (`dotnet new console`) in `fsuipc-test-client/`
- [x] 1.2 Add NuGet packages: `FSUIPCClientDLL`, `Spectre.Console`
- [x] 1.3 Set target framework to `net8.0-windows` (or `net9.0-windows`)
- [x] 1.4 Add workspace entry to `cargo.toml`? (no — .NET project lives independently)
- [x] 1.5 Add AGENTS.md instructions for build/test of .NET project

## 2. Offset input file parser

- [x] 2.1 Implement line parser: split on `,`, trim whitespace, handle `#` comments and blank lines
- [x] 2.2 Implement type→size mapping: `u8`/`i8`→1, `u16`/`i16`→2, `u32`/`i32`/`f32`→4, `u64`/`i64`/`f64`→8, `string`/`bytes`→explicit
- [x] 2.3 Validate address is valid hex in 0x0000..0xFFFF range
- [x] 2.4 Return structured list of offset definitions with error reporting (line number + message)
- [x] 2.5 Write unit tests for parser

## 3. FSUIPC connection and offset registration

- [x] 3.1 Implement `FSUIPCConnection.Open()` with error handling
- [x] 3.2 Implement offset factory: create `Offset<T>` objects from parsed definitions
- [x] 3.3 Implement batch read via `FSUIPCConnection.Process()`
- [x] 3.4 Handle typed values: extract `.Value` from each offset and format for display
- [x] 3.5 Implement `Close()` and cleanup on exit

## 4. Batch mode

- [x] 4.1 Add CLI flags: `--batch`/`-b`, input file path (positional or `--input`)
- [x] 4.2 In batch mode: parse file → open connection → Process() → serialize JSON → close → exit
- [x] 4.3 Handle connection failure: JSON error to stderr, non-zero exit code
- [x] 4.4 Implement JSON serialization matching the design schema

## 5. TUI mode

- [x] 5.1 Build App state struct: list of offset definitions, list of current values, selected row index
- [x] 5.2 Implement main event loop with `LiveDisplay` at ~500ms interval
- [x] 5.3 Render table: columns for address, type, value; highlight selected row
- [x] 5.4 Implement keyboard handling: `↑`/`↓` navigation, `q` quit, `?` help
- [x] 5.5 Implement `r` reload: reparse file, recreate offsets, update display
- [x] 5.6 Implement `s` save: serialize current state to JSON file (file dialog or fixed name)
- [x] 5.7 Implement help overlay panel
- [x] 5.8 Handle `FSUIPCConnection.Process()` exceptions gracefully (show error in TUI)

## 6. Polish and verification

- [x] 6.1 Create sample input file with known offsets
- [ ] 6.2 Test against uipc-debug IPC host (start uipc-debug, launch .NET TUI)
- [ ] 6.3 Test batch mode piping: `fsuipc-test-client offsets.txt --batch > output.json`
- [x] 6.4 Add error messages for common failures (no FSUIPC host, bad file path, parse errors)
- [x] 6.5 Write README documenting usage, input format, keybindings
- [x] 6.6 Run `cargo fmt`, `cargo build` from workspace root to confirm no breakage
