# XPlane UIPC

FSUIPC-like interface for X-Plane. Uses shared memory (IPC) to expose X-Plane datarefs as FSUIPC-compatible offsets, driven by a TOML mapping file.

Goals:

1. management of offsets in a config file;
2. stability -- should not crash when changing aircraft or liveries;
3. should work with Self Loading Cargo + ZiboMod 737

Non-goals:

1. supporting in-process access;
2. supporting WideFS UDP protocol;

## X-Plane dependencies

Check the `mapping.toml`.

Livetraffic plugin required for date fields.

## Project Structure

| Crate | Type | Purpose |
|-------|------|---------|
| `xplane_uipc/` | X-Plane plugin (`cdylib`)| Loads as a plugin into X-Plane, reads `mappings.toml`, evaluates dataref expressions, and writes results to shared memory |
| `ipc_host/` | Library (`cdylib` + `rlib`) | Windows IPC/shared-memory mechanism; communication layer between plugin and FSUIPC clients |
| `uipc-expr/` | Library | RPN-style expression parser and evaluator for offset value formulas |
| `uipc-mapping/` | Library | Loads and validates `mappings.toml` into offset definitions |
| `uipc-debug/` | Binary (TUI) | Offline and live debug tool for evaluating mappings against static data or a running IPC host |
| `expr-calculator/` | Binary (REPL) | Standalone REPL for interactively testing RPN expressions |
| `fsuipc-test-client/` | .NET Console App | Reads FSUIPC offsets from any compatible host (interactive TUI or batch JSON mode) |
| `xtask/` | Build tool | Build system tasks -- `dist`, `deploy` |

## Development

### Linux (cross-compile checks)

Most development can be done on Linux. The `Makefile` drives formatting, linting, cross-compilation, and testing in one command:

```shell
make          # runs: fmt, clippy, check, test (everything that can run on Linux)
make test     # run tests only (portable crates)
make check    # type-check Windows crates via cross-compilation
make clippy   # lint all crates (portable natively, Windows via cross-compilation)
make fmt      # check formatting
```

`make clippy` and `make check` cross-compile the Windows-only crates (`xplane_uipc`, `ipc_host`, `uipc-debug`) using `--target x86_64-pc-windows-msvc`. This verifies Rust type-checking and borrow-checking without needing a Windows linker. Tests for these crates can only run on Windows.

**Prerequisites:**

```shell
rustup target add x86_64-pc-windows-msvc

# Arch Linux
sudo pacman -S mingw-w64-headers    # provides windows.h for bindgen

# Debian/Ubuntu
sudo apt install mingw-w64-x86-64-dev
```

### Windows (full build + deploy)

Full builds, linking, and plugin deployment require Windows with LLVM installed (`winget install LLVM.LLVM`).

```shell
cargo xtask dist      # release build + package to dist/xplane-uipc/
cargo xtask deploy    # dist + copy to C:\X-Plane 12\Resources\plugins\xplane-uipc
cargo test            # run all tests including ipc_host
```

## Tools

### RPN Expression REPL (`expr-calculator`)

A standalone REPL for interactively testing the RPN expression language used in `mappings.toml`.

```shell
cargo run -p expr-calculator
```

Supports tab completion, basic arithmetic, logical operators, and functions (`abs`, `round`).

### Mapping Debugger (`uipc-debug`)

TUI tool for testing offset mappings against static state data or a live IPC host.

```shell
# Offline mode -- evaluate mappings against a static CSV
cargo run -p uipc-debug -- -m xplane_uipc/mappings.toml -s uipc-debug/state.csv --no-ipc

# Live mode -- start IPC host for FSUIPC client testing
cargo run -p uipc-debug -- -m xplane_uipc/mappings.toml -s uipc-debug/state.csv
```

See `uipc-debug/README.md` for full keybindings and options.

### FSUIPC Test Client (`fsuipc-test-client`)

.NET console application (Windows only, requires .NET 10 SDK + FSUIPCClientDLL) that connects to any FSUIPC-compatible host and reads offsets in real time or batch mode.

```shell
dotnet run --project fsuipc-test-client -- fsuipc-test-client/sample-offsets.txt
dotnet run --project fsuipc-test-client -- fsuipc-test-client/sample-offsets.txt --batch > output.json
```

See `fsuipc-test-client/README.md` for input format, keybindings, and build instructions.

## License

GNU LGPLv3.

FSUIPC is a trademark of Pete Dowson. This plugin is an independent implementation and is not affiliated with or endorsed by the FSUIPC project.

X-Plane SDK License [viewable here](./vendor/x-plane-sdk/4.3.0/SDK/license.txt).
