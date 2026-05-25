# Extended Sentinel Scan

## Summary

The resilient parsing added in `ipc-capture` recovers from bad sentinels by scanning 16
bytes forward for the next valid "luaP" record. This window is too small when the junk
data is larger — e.g., `:FSD` followed by ASCII string data and up to 156 bytes of zeros.
The parser breaks, losing all subsequent records in the view.

This change extends the recovery scan to cover the rest of the mapped view, logs
unrecognised sentinel values once each, and specifically logs any `:FSD`-prefixed text
that follows them.

## Motivation

- `:FSD` junk data appears in production buffers from X-Plane FSD clients
- Current 16-byte scan window misses valid records that follow the junk
- Zero diagnostic output — operators have no way to know what junk was received
- The byte capture from `ipc-capture` is the only clue, requiring offline analysis

## Scope

- **In**: `ipc_host/src/mapped_view.rs` (`iterate_records`, `process_mapped_view`,
  `find_next_record`), callers in `lib.rs` and `capture-inspect.rs`, tests
- **Out**: No changes to X-Plane plugin, menu system, or capture logic
