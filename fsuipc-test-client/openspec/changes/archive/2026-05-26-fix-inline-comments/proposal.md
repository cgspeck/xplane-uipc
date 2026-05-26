## Why

The offset parser rejects lines with comments after valid definitions — `0x02BC,i32      # Indicated airspeed` fails with "unknown type" because the `#` and everything after is parsed as part of the type field.

## What Changes

- **OffsetParser**: strip inline `#` comments from each line during parsing
- **Sample file**: already uses inline comments, will now work as-is
- Not a breaking change — existing files without inline comments continue to work

## Capabilities

### New Capabilities

None — this is a bug fix to an existing capability.

### Modified Capabilities

- `offset-input-file`: The parse function will now strip trailing `#` comments from lines before parsing

## Impact

- `OffsetParser.cs` — one-line fix in the line processing loop
- No new dependencies
