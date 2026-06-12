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

* [ZIBO mod B737-800x](https://forums.x-plane.org/forums/topic/138974-b737-800x-zibo-mod-info-installation-download-links/)
* [FlyWithLUA NG+](https://forums.x-plane.org/files/file/82888-flywithlua-ng-next-generation-plus-edition-for-x-plane-12-win-lin-mac/) to run accompanying script to inject dates and utc offsets
* [BetterPushback](https://forums.x-plane.org/files/file/90556-better-pushback-for-x-plane-1112/)

## Installation

1. copy `lua\Xplane_Uipc_Helper_Set_Date_Datarefs.lua` to `C:\X-Plane 12\Resources\plugins\FlyWithLua\Scripts`
2. extract plugin zip to `C:\X-Plane 12\Resources\plugins`

## Logs and Debugging

Logs are in:

* `C:\X-Plane 12\Log.txt` (plugin load & startup, menu actions)
* `C:\X-Plane 12\uipc.log` (requested missing offsets, read offsets that don't exist, trace logging etc)

You can modify the trace logging level in `C:\X-Plane 12\Resources\plugins\xplane-uipc\config.toml`. Setting it to `debug` or `trace` will generate a large `uipc.log` file very quickly.

You can also start/stop capture UIPC messages by clicking  "X-Plane UIPC" ->  "Start Capture" / "Stop Capture". Binary dumps of the shared memory IPC messages will be written to `C:\X-Plane 12\Resources\plugins\xplane-uipc\capture`.

## List of supported Offsets

See:

* [SLC-OFFSETS.md](docs\OFFSETS.md)
* [mappings.toml](xplane_uipc\mappings.toml)

## Extending mappings.toml / adding your own offsets

See [SLC-OFFSETS.md](docs\SLC-OFFSETS.md) for links and references.

Make your changes to `C:\X-Plane 12\Resources\plugins\xplane-uipc\mappings.toml`, then click "X-Plane UIPC" ->  "Reload Mappings".

## Project Structure

| Crate | Type | Purpose |
| ------- | ------ | --------- |
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

Moved to [a dedicated repository](https://github.com/cgspeck/fsuipc-test-client).

## License

GNU LGPLv3.

FSUIPC is a trademark of Pete Dowson. This plugin is an independent implementation and is not affiliated with or endorsed by the FSUIPC project.

X-Plane SDK License [viewable here](./vendor/x-plane-sdk/4.3.0/SDK/license.txt).
