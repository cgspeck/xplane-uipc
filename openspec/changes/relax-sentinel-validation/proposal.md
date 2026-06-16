## Why

`iterate_records` in `ipc_host::mapped_view` rejects records whose 4th field
doesn't equal `0x5061756C` ("luaP"). The FSInterrogate utility writes a
memory-pointer value instead (e.g. `0x0105FFF8`), causing all its requests to
be discarded as "bad sentinel" — even though the actual request parameters
(reqID, dwOffset, nBytes) are perfectly valid.

We need to serve FSUIPC clients that don't set the sentinel to "luaP".

## What Changes

- Relax the sentinel check so that **any non-zero value** in the 4th field is
  accepted as a valid record, not just `0x5061756C`.
- Keep the sentinel as a diagnostic field (`sentinel_ok` continues to indicate
  `== luaP`), but **don't gate record processing on it**.
- Remove the unbounded `LOGGED_SENTINEL_VALUES` HashSet (prevents memory leak
  from per-record unique FSInterrogate pointer values).
- Downgrade the per-record debug log of non-"luaP" records from `debug!` to
  `trace!` to prevent INFO-level flooding.
- For read requests, write the response into the inline payload area
  (offset+16, same as now) regardless of whether the sentinel matched.
- Preserve the existing error-counting and recovery-scanning logic for
  truly broken data (zero reqID, etc.).

## Capabilities

### New Capabilities
- `flexible-record-validation`: Accept records from any FSUIPC-compatible
  client regardless of the 4th-field sentinel value.

### Modified Capabilities
- `capture-inspect`: Updated to reflect new sentinel semantics.

## Impact

- **`ipc_host::mapped_view`**: `iterate_records` no longer treats
  non-"luaP" sentinels as hard errors. Unknown values are logged once
  at trace level but the record is still processed.
- **`ipc_host::examples::capture-inspect`**: The tool will show FSInterrogate
  records as valid rather than "BAD SENTINEL".
- **`ipc_host::tests`**: New fixture-based integration tests for both
  SLC and FSInterrogate capture files.
