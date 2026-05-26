# Agents

## Verification

Before declaring work complete, always run formatter, tests, build and dist, e.g.

```bash
cargo fmt
cargo test
cargo build
cargo xtask dist
```

## .NET projects

The `fsuipc-test-client/` directory is a .NET 10 console application with Windows dependencies (FSUIPCClientDLL).

```bash
# Build
dotnet build fsuipc-test-client

# Test
dotnet test fsuipc-test-client.Tests

# Run (TUI mode)
dotnet run --project fsuipc-test-client -- sample-offsets.txt

# Run (batch mode)
dotnet run --project fsuipc-test-client -- sample-offsets.txt --batch > output.json
```