# fsuipc-test-client

.NET console tool for reading FSUIPC offsets in real-time (TUI) or one-shot batch mode. Connects to any FSUIPC-compatible host — MSFS+FSUIPC7, X-Plane+uipc-debug IPC host, or WideClient.

## Usage

```
fsuipc-test-client <input-file>               TUI mode (interactive)
fsuipc-test-client <input-file> --batch       Batch mode (JSON to stdout)
```

## Input file format

```
# Comments start with #
<address>,<type>[,<size>]
```

| Type    | Size        | Example               |
|---------|-------------|-----------------------|
| u8, i8  | 1 byte      | `0x3365,u8`           |
| u16, i16| 2 bytes     | `0x0D0C,u16`          |
| u32, i32, f32 | 4 bytes | `0x02BC,i32`          |
| u64, i64, f64 | 8 bytes | `0x0560,i64`          |
| string  | explicit    | `0x3160,string,24`    |
| bytes   | explicit    | `0x0238,bytes,10`     |

See `sample-offsets.txt` for a complete example.

## TUI keybindings

| Key | Action            |
|-----|-------------------|
| ↑↓  | Navigate rows     |
| r   | Reload input file |
| s   | Save JSON snapshot|
| q   | Quit              |
| h   | Toggle help       |
| Esc | Close help        |

Display refreshes at ~2 Hz.

## Batch mode

```bash
dotnet run --project fsuipc-test-client -- sample-offsets.txt --batch > output.json
```

Outputs JSON with timestamp and offset values. Errors go to stderr with non-zero exit code.

## Build

```bash
dotnet build fsuipc-test-client
dotnet test fsuipc-test-client.Tests
```
